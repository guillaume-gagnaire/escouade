<script lang="ts">
  import { settingsAlerts } from '../../lib/settings-alerts.svelte';
  import Icon from '../Icon.svelte';
  import { tick } from 'svelte';
  import { openUrl } from '../../lib/opener';
  import { fList } from '../../lib/format';
  import { forgeBadge } from '../../lib/forges';
  import { t } from '../../lib/i18n';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { ForgeAccountView, ForgeForm, ForgesView } from '../../lib/types';
  import Group from './Group.svelte';

  // The code hosts' accounts, one per instance: how each stands (checked when the tab opens), the
  // projects whose remote is on it, and the hosts of the projects' remotes without one.
  let view = $state<ForgesView | null>(null);
  let checking = $state(false);
  let form = $state<ForgeForm | null>(null);
  let sending = $state(false);
  let guessing = $state(false);
  let refused = $state('');

  const provider = (kind: string) => view?.providers.find((p) => p.kind === kind);
  const chosen = $derived(form ? provider(form.kind) : undefined);

  function take(v: ForgesView | null | undefined) {
    if (!v) return;
    view = v;
    settingsAlerts.takeForges(v);
    if (v.issues?.length) app.accounts = v.issues;
  }

  /** GitHub Issues' account is github.com's: connected or gone there, the list follows. */
  const issuesAccount = $derived(app.accounts.find((a) => a.service === 'github' && a.connected)?.label ?? '');

  $effect(() => {
    void issuesAccount;
    api.forges().then(take, () => {});
    checking = true;
    api
      .forgeCheck()
      .then(take, () => {})
      .finally(() => (checking = false));
  });

  async function recheck() {
    checking = true;
    take(await app.run(api.forgeCheck()));
    checking = false;
  }

  function open(kind = view?.providers[0]?.kind ?? 'github', url = '') {
    refused = '';
    form = { kind, url, email: '', token: '' };
  }

  /** The form of a code host of `kind`, opened from elsewhere (GitLab Issues' « Connecter GitLab… », Azure Boards' « Connecter Azure DevOps… »): shown, its address field focused. */
  export async function connectKind(kind: string) {
    if (!form) open(kind);
    else form.kind = kind;
    await tick();
    const at = document.querySelector<HTMLFormElement>('[data-anchor="forges"] form.connect');
    at?.scrollIntoView({ block: 'nearest' });
    at?.querySelectorAll<HTMLInputElement>('input')[0]?.focus();
  }

  /** « Connecter <hôte> »: its kind, by its name, else by asking it. */
  async function openFor(url: string, kind: string | null) {
    open(kind ?? view?.providers[0]?.kind ?? 'github', url);
    if (kind) return;
    guessing = true;
    try {
      const guessed = await api.forgeGuess(url);
      if (form && guessed) form.kind = guessed;
      else if (form) refused = t('integrations.forges.unknownKind');
    } catch (e) {
      refused = String(e);
    } finally {
      guessing = false;
    }
  }

  async function connect() {
    if (!form || sending) return;
    sending = true;
    refused = '';
    try {
      take(await api.forgeConnect($state.snapshot(form)));
      form = null;
    } catch (e) {
      refused = String(e);
    } finally {
      sending = false;
    }
  }

  async function disconnect(a: ForgeAccountView) {
    take(await app.run(api.forgeDisconnect(a.host)));
  }

  async function tokenPage() {
    if (!form) return;
    const url = await app.run(api.forgeTokenPage(form.kind, form.url));
    if (url) app.run(openUrl(url));
  }

  /** Where it is: its host, with the organization or collection its address names (Azure DevOps). */
  const where = (a: ForgeAccountView) => a.url.replace(/^https?:\/\//, '').replace(/\/+$/, '') || a.host;

  /** What its last check found, in a sentence. */
  function standing(a: ForgeAccountView): { text: string; tone: 'on' | 'warn' | 'bad' | '' } {
    const label = a.label;
    switch (a.status.health) {
      case 'connected':
        return { text: t('integrations.forges.connected', { label }), tone: 'on' };
      case 'missingScopes':
        return { text: t('integrations.forges.missing', { scopes: a.status.missing.join(', ') }), tone: 'warn' };
      case 'expired':
        return { text: t('integrations.forges.expired', { label }), tone: 'bad' };
      case 'unreachable':
        return { text: t('integrations.forges.unreachable', { label }), tone: 'bad' };
      case 'unread':
        return { text: t('integrations.forges.unread'), tone: 'warn' };
      default:
        return { text: checking ? t('integrations.forges.checking') : t('integrations.forges.unchecked', { label }), tone: '' };
    }
  }
</script>

{#snippet badge(kind: string | null)}
  {@const b = forgeBadge(kind)}
  <span class="badge" aria-hidden="true">{b.letter}</span>
{/snippet}

<Group title={t('integrations.forges.title')} note={t('integrations.forges.note')} anchor="forges">
  {#snippet action()}
    {#if view?.accounts.length}
      <button class="btn" disabled={checking || !!form} onclick={recheck}
        >{checking ? t('integrations.forges.checking') : t('integrations.forges.recheck')}</button
      >
    {/if}
    <button class="btn" disabled={!!form} onclick={() => open()}
      ><Icon name="plus" size={14} />{t('integrations.forges.connectEllipsis')}</button
    >
  {/snippet}
  {#if view && !view.accounts.length && !view.unknown.length && !form}
    <p class="empty">{t('integrations.forges.empty')}</p>
  {/if}
  {#each view?.accounts ?? [] as a (a.host)}
    {@const st = standing(a)}
    {@const served = a.plugin ? null : provider(a.kind)?.servedBy}
    <div class="acct" role="group" aria-label={a.host}>
      {@render badge(a.kind)}
      <div class="txt">
        <span class="l">{a.name} <span class="host">· {where(a)}</span></span>
        <span class="d {st.tone}">{st.text}</span>
        {#if a.status.health === 'missingScopes'}<span class="d">{a.label}</span>{/if}
        {#if a.status.error && a.status.health !== 'connected'}<span class="d why">{a.status.error}</span>{/if}
        {#if a.inFile}<span class="d warn">{t('integrations.accounts.keychainUnavailable')}</span>{/if}
        {#if a.viaCli}<span class="d">{t('integrations.forges.viaCli', { cli: provider(a.kind)?.cli ?? 'gh' })}</span>{/if}
        {#if a.plugin}<span class="d">{t('integrations.accounts.servedByPlugin', { plugin: a.plugin })}</span>{/if}
        {#if served && !served.on}
          <span class="d warn" data-plugin-off={served.plugin}>{t('integrations.forges.pluginOff', { plugin: served.name })}</span>
        {:else if served?.stopped}
          <span class="d warn" data-plugin-stopped={served.plugin}
            >{t('integrations.forges.pluginStopped', { plugin: served.name, why: served.stopped })}</span
          >
        {/if}
        {#if a.issues}
          <span class="d"
            >{t('integrations.forges.issues', {
              source: a.kind === 'gitlab' ? 'GitLab Issues' : a.kind === 'azure' ? 'Azure Boards' : 'GitHub Issues',
            })}</span
          >
        {:else}
          <span class="d">{t('integrations.forges.noTickets')}</span>
        {/if}
        {#if a.projects.length}
          <span class="d">{t('integrations.forges.projects', { count: a.projects.length, list: fList(a.projects) })}</span>
        {/if}
      </div>
      <button class="btn" aria-label={t('integrations.forges.disconnectHost', { host: a.host })} onclick={() => disconnect(a)}
        >{t('integrations.forges.disconnect')}</button
      >
    </div>
  {/each}
  {#each view?.unknown ?? [] as u (u.host)}
    <div class="acct" role="group" aria-label={u.host}>
      {@render badge(u.kind)}
      <div class="txt">
        <span class="l">{u.kind ? (provider(u.kind)?.name ?? u.host) : u.host} <span class="host">· {u.host}</span></span>
        <span class="d">{t('integrations.forges.noAccount', { count: u.projects.length, list: fList(u.projects) })}</span>
      </div>
      <button class="btn" onclick={() => openFor(u.url, u.kind)}>{t('integrations.forges.connectHost', { host: u.host })}</button>
    </div>
  {/each}
  {#if form}
    <form
      class="connect"
      aria-label={t('integrations.forges.formLabel')}
      onsubmit={(e) => {
        e.preventDefault();
        connect();
      }}
    >
      <label
        ><span>{t('integrations.forges.kind')}</span><select
          class="field"
          bind:value={form.kind}
          disabled={guessing}
          onchange={() => form && delete form.plugin}
        >
          {#each view?.providers ?? [] as p (p.kind)}<option value={p.kind}>{p.name}</option>{/each}
        </select></label
      >
      {#if chosen?.servers && chosen.servers.length > 1}
        <label
          ><span>{t('integrations.accounts.servedBy')}</span><select class="field" bind:value={form.plugin}>
            {#each chosen.servers as s (s.plugin)}<option value={s.firstHand ? '' : s.plugin}>{s.name}</option>{/each}
          </select></label
        >
      {/if}
      <label
        ><span>{t('integrations.forges.address')}</span><input
          class="field mono"
          bind:value={form.url}
          placeholder={chosen?.addressHint ?? chosen?.defaultHost ?? 'git.example.com'}
          spellcheck="false"
        /></label
      >
      {#if chosen?.needsEmail}
        <label
          ><span>{t('integrations.accounts.email')}</span><input
            class="field"
            bind:value={form.email}
            placeholder="ada@atlas.dev"
            spellcheck="false"
          /></label
        >
      {/if}
      <label
        ><span
          >{#if chosen?.cli}<Rich k="integrations.forges.token"
              >{#snippet hint()}<em>{t('integrations.forges.tokenHint', { cli: chosen.cli ?? '' })}</em>{/snippet}</Rich
            >{:else}{t('integrations.accounts.token')}{/if}</span
        ><input
          class="field mono"
          type="password"
          bind:value={form.token}
          placeholder={chosen?.cli ? (chosen.cliLogin ?? `${chosen.cli} auth login`) : ''}
          autocomplete="off"
        /></label
      >
      {#if chosen}<p class="d">{t('integrations.forges.scopes', { scopes: chosen.scopes.join(', ') })}</p>{/if}
      <button type="button" class="link" onclick={tokenPage}>{t('integrations.forges.createToken')}</button>
      {#if guessing}<p class="d">{t('integrations.forges.guessing')}</p>{/if}
      {#if refused}<p class="error" role="alert">{refused}</p>{/if}
      <div class="actions">
        <button type="button" class="btn small ghost" onclick={() => (form = null)}>{t('common.cancel')}</button>
        <button type="submit" class="btn small primary" disabled={sending || guessing}
          >{sending ? t('integrations.accounts.checking') : t('integrations.accounts.connect')}</button
        >
      </div>
    </form>
  {/if}
</Group>

<style>
  .badge {
    min-width: 24px;
    height: 24px;
    padding: 0;
    flex: none;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.07);
    color: var(--text);
    font-size: 11px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  /* An instance a line (`.forge` of the 2.0 design): green connected, amber missing rights, red expired. */
  .acct {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 0;
  }
  :global(.note) + .acct {
    margin-top: 6px;
  }
  .acct + .acct,
  .connect {
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .l {
    font-weight: 600;
  }
  .host {
    font-family: var(--mono);
    font-size: 11.5px;
    font-weight: 400;
    color: var(--muted);
  }
  .d {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .d.on {
    color: var(--add);
  }
  .d.warn {
    color: var(--warn-ink);
  }
  .d.bad {
    color: var(--del);
  }
  .d.why {
    font-family: var(--mono);
    font-size: 11px;
  }
  .small {
    height: 30px;
    font-size: 12.5px;
    padding: 0 14px;
  }
  .empty {
    margin: 8px 0 0;
    padding: 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .connect {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 0 14px 36px;
  }
  .connect label {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12.5px;
  }
  .connect label > span {
    width: 110px;
    flex: none;
    color: var(--muted);
  }
  .connect em {
    font-style: normal;
    color: var(--muted);
  }
  .connect .field {
    flex: 1;
    min-width: 0;
  }
  .link {
    align-self: flex-start;
    padding: 0;
    border: none;
    background: none;
    color: var(--p-ink);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .error {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--del);
    overflow-wrap: anywhere;
  }
</style>
