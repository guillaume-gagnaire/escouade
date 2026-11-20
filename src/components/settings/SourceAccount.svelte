<script lang="ts">
  import { openUrl } from '../../lib/opener';
  import { t } from '../../lib/i18n';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { accountsOf, BY_FORGE, HOSTED, SERVICES, trelloTokenPage } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { AccountForm, Service } from '../../lib/types';
  import ServiceBadge from './ServiceBadge.svelte';

  // One source of tickets in « Services connectés » (design/settings-2-2/PROPOSITION.md §2, page 4): its accounts, how
  // each stands, and « Connecter… » with its form, connected at once (its secrets in the keychain, ESC-499 without one).
  // The same block in the first launch's « Ce que tu veux brancher » (ESC-512): one way to connect a source, never two.
  // A source served by its code host's account (GitLab Issues, Azure Boards) is connected by the host's form: `onforge`.
  let {
    id,
    onforge,
    open: opened = false,
  }: {
    id: Service;
    onforge?: (kind: string) => void;
    /** Its form open from the start (the first launch: the source was just chosen). */
    open?: boolean;
  } = $props();

  const account = $derived(app.accounts.find((a) => a.service === id));
  /** The source's accounts, several for Jira, Trello, Linear and ClickUp (ESC-397); one at most for the others. */
  const list = $derived.by(() => {
    const many = accountsOf(app.accounts, id);
    if (many.length) return many;
    return account?.connected && !BY_FORGE[id] ? [account] : [];
  });
  /** A source that takes another account beside those connected. */
  const several = $derived(!HOSTED.includes(id) && !BY_FORGE[id]);
  const one = $derived(list.length === 1 ? list[0] : undefined);

  const blank = (): AccountForm => ({ site: '', email: '', key: '', token: '' });
  let form = $state<AccountForm | null>(null);
  let checking = $state(false);
  let refused = $state('');
  // Asked open (the first launch): the form of a source with no account yet.
  $effect(() => {
    if (opened && !form && !list.length && !BY_FORGE[id]) form = blank();
  });

  async function connect() {
    if (!form || checking) return;
    checking = true;
    refused = '';
    try {
      const view = await api.integrationConnect(id, $state.snapshot(form));
      // The same account again is put right; another one comes after those of its service (ESC-397). The line « Non
      // connecté » of a source without any goes.
      const i = app.accounts.findIndex((a) => a.service === id && a.connected && (a.id === view.id || !several));
      app.accounts =
        i >= 0 ? app.accounts.map((a, k) => (k === i ? view : a)) : [...app.accounts.filter((a) => a.service !== id || a.connected), view];
      form = null;
    } catch (e) {
      refused = String(e);
    } finally {
      checking = false;
    }
  }

  async function disconnect(acc?: string) {
    const views = await app.run(acc ? api.integrationDisconnect(id, acc) : api.integrationDisconnect(id));
    if (views) app.accounts = views;
  }

  const open = (url: string) => app.run(openUrl(url));
  /** The code host a source without an account of its own is connected as. */
  const forgeName = $derived(id === 'azure' ? 'Azure DevOps' : 'GitLab');
</script>

<div class="src" data-source={id}>
  <div class="acct">
    <ServiceBadge {id} />
    <div class="txt">
      <span class="l">{SERVICES[id].name}</span>
      {#if BY_FORGE[id] || !list.length}
        <span class="d" class:on={account?.connected}
          >{account?.connected
            ? t('integrations.accounts.connected', { label: account.label })
            : t('integrations.accounts.notConnected')}</span
        >
      {:else if one}
        {@render state(one)}
      {/if}
      {#if BY_FORGE[id]}<span class="d">{t('integrations.accounts.byForge', { forge: forgeName })}</span>{/if}
      {#if id === 'azure'}<span class="d">{t('integrations.accounts.workItems')}</span>{/if}
    </div>
    {#if BY_FORGE[id]}
      {#if !account?.connected && onforge}
        <button class="btn" onclick={() => onforge(BY_FORGE[id]!)}>{t('integrations.accounts.connectForge', { forge: forgeName })}</button>
      {/if}
    {:else}
      {#if list.length && several && !form}
        <button class="btn" onclick={() => (form = blank())}>{t('integrations.accounts.connectAnother')}</button>
      {/if}
      {#if one}
        <button class="btn" onclick={() => disconnect(one.id)}>{t('integrations.accounts.disconnect')}</button>
      {:else if !list.length && !form}
        <button class="btn" onclick={() => (form = blank())}>{t('integrations.accounts.connectEllipsis')}</button>
      {/if}
    {/if}
  </div>
  {#if list.length > 1}
    {#each list as acc (acc.id)}
      <div class="acct sub">
        <div class="txt">{@render state(acc)}</div>
        <button
          class="btn small"
          aria-label={t('integrations.accounts.disconnectOne', { label: acc.label })}
          onclick={() => disconnect(acc.id)}>{t('integrations.accounts.disconnect')}</button
        >
      </div>
    {/each}
  {/if}
  {#if form && (several || !list.length)}
    <form
      class="connect"
      aria-label={t('integrations.accounts.formLabel', { service: SERVICES[id].name })}
      onsubmit={(e) => {
        e.preventDefault();
        connect();
      }}
    >
      {#if account?.servers && account.servers.length > 1}
        <label
          ><span>{t('integrations.accounts.servedBy')}</span><select class="field" bind:value={form.plugin}>
            {#each account.servers as s (s.plugin)}<option value={s.firstHand ? '' : s.plugin}>{s.name}</option>{/each}
          </select></label
        >
      {/if}
      {#if id === 'jira'}
        <label
          ><span>{t('integrations.accounts.site')}</span><input
            class="field mono"
            bind:value={form.site}
            placeholder="atlas.atlassian.net"
            spellcheck="false"
          /></label
        >
        <label
          ><span>{t('integrations.accounts.email')}</span><input
            class="field"
            bind:value={form.email}
            placeholder="ada@atlas.dev"
            spellcheck="false"
          /></label
        >
        <label
          ><span>{t('integrations.accounts.apiToken')}</span><input
            class="field mono"
            type="password"
            bind:value={form.token}
            autocomplete="off"
          /></label
        >
        <button type="button" class="link" onclick={() => open('https://id.atlassian.com/manage-profile/security/api-tokens')}
          >{t('integrations.accounts.createAtlassianToken')}</button
        >
      {:else if id === 'trello'}
        <label><span>{t('integrations.accounts.apiKey')}</span><input class="field mono" bind:value={form.key} spellcheck="false" /></label>
        <label
          ><span>{t('integrations.accounts.token')}</span><input
            class="field mono"
            type="password"
            bind:value={form.token}
            autocomplete="off"
          /></label
        >
        <div class="links">
          <button type="button" class="link" onclick={() => open('https://trello.com/power-ups/admin')}
            >{t('integrations.accounts.getApiKey')}</button
          >
          <button type="button" class="link" disabled={!form.key.trim()} onclick={() => form && open(trelloTokenPage(form.key))}
            >{t('integrations.accounts.getTokenForKey')}</button
          >
        </div>
      {:else if id === 'linear'}
        <label
          ><span>{t('integrations.accounts.apiKey')}</span><input
            class="field mono"
            type="password"
            bind:value={form.token}
            autocomplete="off"
          /></label
        >
        <button type="button" class="link" onclick={() => open('https://linear.app/settings/account/security')}
          >{t('integrations.accounts.createLinearKey')}</button
        >
      {:else if id === 'clickup'}
        <label
          ><span>{t('integrations.accounts.apiKey')}</span><input
            class="field mono"
            type="password"
            bind:value={form.token}
            placeholder={t('integrations.accounts.clickupKeyHint')}
            autocomplete="off"
          /></label
        >
        <button type="button" class="link" onclick={() => open('https://app.clickup.com/settings/apps')}
          >{t('integrations.accounts.createClickupKey')}</button
        >
      {:else}
        <label
          ><span
            ><Rich k="integrations.accounts.githubToken"
              >{#snippet hint()}<em>{t('integrations.accounts.githubTokenHint')}</em>{/snippet}</Rich
            ></span
          ><input class="field mono" type="password" bind:value={form.token} placeholder="gh auth login" autocomplete="off" /></label
        >
        <button type="button" class="link" onclick={() => open('https://github.com/settings/tokens')}
          >{t('integrations.accounts.createGithubToken')}</button
        >
      {/if}
      {#if refused}<p class="error" role="alert">{refused}</p>{/if}
      <div class="actions">
        <button type="button" class="btn small ghost" onclick={() => (form = null)}>{t('common.cancel')}</button>
        <button type="submit" class="btn small primary" disabled={checking}
          >{checking ? t('integrations.accounts.checking') : t('integrations.accounts.connect')}</button
        >
      </div>
    </form>
  {/if}
</div>

{#snippet state(acc: (typeof app.accounts)[number])}
  <span class="d on">{t('integrations.accounts.connected', { label: acc.label })}</span>
  {#if acc.inFile}<span class="d warn">{t('integrations.accounts.keychainUnavailable')}</span>{/if}
  {#if acc.unread}<span class="d warn">{t('integrations.accounts.keychainUnreadable')}</span>{/if}
  {#if acc.plugin}<span class="d">{t('integrations.accounts.servedByPlugin', { plugin: acc.plugin })}</span>{/if}
{/snippet}

<style>
  :global(.src) + .src {
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .acct {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 0;
  }
  .acct.sub {
    padding: 7px 0 7px 36px;
  }
  .acct + .acct {
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
  .d {
    font-size: 12px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .d.on {
    color: var(--soft);
  }
  .d.warn {
    color: var(--warn-ink);
  }
  .small {
    height: 30px;
    font-size: 12.5px;
    padding: 0 14px;
  }
  .connect {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 4px 0 14px 36px;
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
  .links {
    display: flex;
    gap: 16px;
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
  .link:hover:not(:disabled) {
    color: var(--text);
  }
  .link:disabled {
    color: var(--muted);
    cursor: default;
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
