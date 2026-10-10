<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import { accountName, backToAccounts, PRINCIPAL, SHARED_FILES } from '../../lib/accounts';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { IS_MAC } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import { disposeTerminal, mountTerminal, openAccountLogin } from '../../lib/terminals';
  import type { Account, ShareMode } from '../../lib/types';
  import Modal from './Modal.svelte';

  // « Ajouter un compte… »: a new Claude account, made with what it shares of Principal's folder, then signed in.
  // « Se connecter… » (`accountId`): straight to the sign-in. The sign-in runs the account's own `claude` in a terminal
  // here; Escouade looks for it every 2 s, then closes the terminal and reads the account's quota. Closed before, the
  // account stays, not signed in.
  let { accountId }: { accountId?: string } = $props();

  /** How often the sign-in is looked for. */
  const POLL_MS = 2000;
  /** How long « Connecté » is seen above its terminal before the terminal goes. */
  const SEEN_MS = 1500;

  /** The account signing in; none while it is still to make. */
  let account = $state<Account | null>(untrack(() => app.settings.accounts.find((a) => a.id === accountId) ?? null));
  let name = $state('');
  /** What Principal has to share; null until known. */
  let shareable = $state<string[] | null>(null);
  let checked = $state<Record<string, boolean>>({});
  let mode = $state<ShareMode>('link');
  let busy = $state(false);
  let error = $state<string | null>(null);
  /** The sign-in's terminal, while it runs. */
  let termId = $state<string | null>(null);
  let box = $state<HTMLDivElement>();
  /** Signed in: with which email. */
  let signedIn = $state<{ email: string | null } | null>(null);
  let poll: ReturnType<typeof setInterval> | undefined;
  let seen: ReturnType<typeof setTimeout> | undefined;
  /** Signed in without its email yet: looked at once more for it. */
  let waitedForEmail = false;
  let closed = false;
  /**
   * Which sign-in the account had before its terminal opened (its status' stamp), none when it had none or could not be
   * read. A sign-in out of date is « connected » too: the user is signed in just now only once it is another one.
   * Undefined until read, once for the window (a terminal started again keeps it).
   */
  let before: string | null | undefined;
  /** A look at the sign-in is under way: the next tick waits for it. */
  let looking = false;

  const principal = $derived(accountName(app.settings.accounts.find((a) => a.id === PRINCIPAL) ?? { id: PRINCIPAL, name: '' }));
  const files = $derived((shareable ?? []).filter((n) => SHARED_FILES.includes(n)));
  const folders = $derived((shareable ?? []).filter((n) => !SHARED_FILES.includes(n)));
  const exited = $derived(termId !== null && app.exitedTerms[termId] !== undefined);
  const title = $derived(account ? t('accounts.modal.signInTitle', { name: accountName(account) }) : t('accounts.modal.addTitle'));

  untrack(() => {
    if (account) return void start();
    api
      .accountShareable()
      .then((list) => {
        shareable = list;
        checked = Object.fromEntries(list.map((n) => [n, true]));
      })
      .catch(() => (shareable = []));
  });

  // The terminal shown in the window while it runs.
  $effect(() => {
    const id = termId;
    const el = box;
    if (!id || !el) return;
    mountTerminal(id, el);
    return () => mountTerminal(id, null);
  });

  async function create() {
    if (!name.trim() || busy) return;
    busy = true;
    error = null;
    try {
      const share = (shareable ?? []).filter((n) => checked[n]);
      const made = await api.accountCreate(name.trim(), share, mode);
      app.settings.accounts = [...app.settings.accounts, made];
      account = made;
      await start();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  /** The account's `claude` in a terminal, the sign-in looked for. */
  async function start() {
    if (!account || closed) return;
    error = null;
    try {
      if (before === undefined) {
        before = await api.accountStatus(account.id).then(
          (s) => s.stamp,
          () => null,
        );
        if (closed) return;
      }
      const info = await openAccountLogin(account.id);
      if (closed) return disposeTerminal(info.id);
      termId = info.id;
      clearInterval(poll);
      poll = setInterval(check, POLL_MS);
    } catch (e) {
      error = String(e);
    }
  }

  async function check() {
    // The last look may be waiting still (the keychain asking its user): not another meanwhile.
    if (!account || signedIn || looking) return;
    looking = true;
    let s;
    try {
      s = await api.accountStatus(account.id);
    } catch {
      return;
    } finally {
      looking = false;
    }
    if (closed || signedIn || !s.connected || s.stamp === before) return;
    // Claude Code writes the email beside the sign-in: a moment more for it.
    if (!s.email && !waitedForEmail) {
      waitedForEmail = true;
      return;
    }
    signedIn = { email: s.email };
    clearInterval(poll);
    seen = setTimeout(finish, SEEN_MS);
  }

  function dropTerminal() {
    if (!termId) return;
    disposeTerminal(termId);
    // Killed here: its exit, which comes after, is nobody's to read.
    app.dropExit(termId);
    termId = null;
  }

  /** Signed in: the terminal goes, and the account's quota is read at once. */
  function finish() {
    seen = undefined;
    dropTerminal();
    void app.run(api.refreshUsage());
  }

  /** « Relancer »: its `claude` stopped before the sign-in. */
  async function restart() {
    dropTerminal();
    await start();
  }

  function stop() {
    if (closed) return;
    closed = true;
    clearInterval(poll);
    if (seen === undefined) return dropTerminal();
    // Signed in, closed before its terminal went: its quota is read all the same.
    clearTimeout(seen);
    finish();
  }

  function close() {
    stop();
    backToAccounts();
  }

  onDestroy(stop);
</script>

{#snippet item(n: string)}
  <label class="item"><input type="checkbox" name={n} bind:checked={checked[n]} /><span class="mono">{n}</span></label>
{/snippet}

{#snippet actions()}
  {#if !account}
    <button class="btn ghost" onclick={close}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={busy || !name.trim()} onclick={create}>{t('accounts.modal.create')}</button>
  {:else}
    <button class="btn primary" onclick={close}>{t('accounts.modal.done')}</button>
  {/if}
{/snippet}

<!-- Signing in, only its × closes it: the account stays, not signed in. -->
<Modal {title} width={account ? 820 : 600} onclose={close} footer={!account || signedIn ? actions : undefined}>
  {#if !account}
    <div class="field-row">
      <label class="k" for="account-name">{t('accounts.modal.name')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        id="account-name"
        class="field name"
        spellcheck="false"
        autofocus
        placeholder={t('accounts.modal.namePlaceholder')}
        bind:value={name}
        onkeydown={(e) => e.key === 'Enter' && create()}
      />
    </div>
    {#if shareable?.length}
      <fieldset class="share">
        <legend>{t('accounts.modal.share', { name: principal })}</legend>
        {#if files.length}
          <div class="line">
            <span class="k">{t('accounts.modal.files')}</span>
            {#each files as n (n)}{@render item(n)}{/each}
            {#if !IS_MAC && mode === 'link'}<span class="hint">{t('accounts.modal.filesCopied')}</span>{/if}
          </div>
        {/if}
        {#if folders.length}
          <div class="line">
            <span class="k">{t('accounts.modal.folders')}</span>
            {#each folders as n (n)}{@render item(n)}{/each}
          </div>
        {/if}
        <div class="line modes" role="radiogroup" aria-label={t('accounts.modal.mode')}>
          <label class="item"><input type="radio" name="share-mode" value="link" bind:group={mode} />{t('accounts.modal.link')}</label>
          <label class="item"><input type="radio" name="share-mode" value="copy" bind:group={mode} />{t('accounts.modal.copy')}</label>
        </div>
      </fieldset>
    {:else if shareable}
      <p class="note">{t('accounts.modal.nothingToShare', { name: principal })}</p>
    {/if}
    <p class="note">{t('accounts.modal.neverShared')}</p>
  {:else}
    <p class="said" class:ok={signedIn} role="status">
      {#if signedIn}
        {signedIn.email ? t('accounts.modal.connected', { email: signedIn.email }) : t('accounts.modal.connectedNoEmail')}
      {:else}
        {t('accounts.modal.signInHint')}
      {/if}
    </p>
    {#if termId}<div class="term" bind:this={box}></div>{/if}
    {#if exited && !signedIn}
      <div class="ended">
        <span>{t('accounts.modal.ended')}</span>
        <button class="btn" onclick={restart}>{t('accounts.modal.restart')}</button>
      </div>
    {/if}
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</Modal>

<style>
  .field-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .k {
    flex: none;
    min-width: 70px;
    font-size: 12px;
    color: var(--dim);
  }
  .name {
    flex: 1;
    height: 32px;
    padding: 0 10px;
    font-size: 13px;
  }
  .share {
    margin: 0;
    padding: 12px 14px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border: 1px solid var(--line);
    border-radius: var(--r);
  }
  legend {
    padding: 0 6px;
    font-size: 12.5px;
    font-weight: 600;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
  }
  .modes {
    padding-top: 4px;
    border-top: 1px solid var(--line);
  }
  .item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    cursor: pointer;
  }
  .hint {
    font-size: 11.5px;
    color: var(--muted);
  }
  .note {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--dim);
  }
  .said {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--muted);
  }
  .said.ok {
    color: var(--add);
    font-weight: 600;
  }
  .term {
    height: 380px;
    padding: 10px 6px 6px 12px;
    border-radius: var(--r);
    background: var(--term);
    overflow: hidden;
  }
  .term :global(.xterm) {
    height: 100%;
  }
  .term :global(.xterm-viewport) {
    background: transparent !important;
  }
  .ended {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--del);
  }
</style>
