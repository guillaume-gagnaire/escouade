<script lang="ts">
  import { openUrl } from '../../lib/opener';
  import { t } from '../../lib/i18n';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { BY_FORGE, SERVICE_IDS, SERVICES, trelloTokenPage } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { AccountForm, Service } from '../../lib/types';
  import Chips from './Chips.svelte';
  import ForgesSection from './ForgesSection.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import ServiceBadge from './ServiceBadge.svelte';
  import Switch from './Switch.svelte';

  // « Services connectés », a page of Escouade (design/settings-2-2/PROPOSITION.md §2, page 4): the code hosts, the
  // accounts of the ticket sources (connected at once, apart from the draft), and the sync and automatic import of every
  // project (the app's settings). What a project links is on its page « Tickets importés ».
  const s = $derived(settingsForm.settings.integrations);

  const account = (id: Service) => app.accounts.find((a) => a.service === id);

  /** « Connecter… »: the open forms, the one being checked, and what each service refused. */
  let forms = $state<Partial<Record<Service, AccountForm>>>({});
  let checking = $state<Service | null>(null);
  let refused = $state<Partial<Record<Service, string>>>({});

  async function connect(id: Service) {
    const form = forms[id];
    if (!form || checking) return;
    checking = id;
    delete refused[id];
    try {
      const view = await api.integrationConnect(id, $state.snapshot(form));
      app.accounts = app.accounts.some((a) => a.service === id)
        ? app.accounts.map((a) => (a.service === id ? view : a))
        : [...app.accounts, view];
      delete forms[id];
    } catch (e) {
      refused[id] = String(e);
    } finally {
      checking = null;
    }
  }

  async function disconnect(id: Service) {
    const views = await app.run(api.integrationDisconnect(id));
    if (views) app.accounts = views;
  }

  const open = (url: string) => app.run(openUrl(url));

  const every = $derived([5, 15, 60].map((n) => ({ value: n, label: t('integrations.autoImport.minutes', { n }) })));
  const blank = (): AccountForm => ({ site: '', email: '', key: '', token: '' });
  /** The code host a source without an account of its own is connected as. */
  const forgeName = (id: Service) => (id === 'azure' ? 'Azure DevOps' : 'GitLab');

  /** The code hosts' part: GitLab Issues' « Connecter GitLab… » opens its form there (one account for both uses). */
  let forges = $state<ReturnType<typeof ForgesSection>>();
</script>

<!-- Two parts, named apart (ESC-165): the code hosts first (choice 27 of the 2.0 design), they serve every agent's
     pull requests; then the sources of the tickets, their accounts, their sync and their import. -->
<header class="part" data-anchor="hosts">
  <h2>{t('integrations.parts.forges.title')}</h2>
  <p>{t('integrations.parts.forges.note')}</p>
</header>
<ForgesSection bind:this={forges} />

<header class="part" data-anchor="sources">
  <h2>{t('integrations.parts.sources.title')}</h2>
  <p>{t('integrations.parts.sources.note')}</p>
</header>

<Group title={t('integrations.accounts.title')} note={t('integrations.accounts.note')}>
  {#each SERVICE_IDS as id (id)}
    {@const a = account(id)}
    {@const form = forms[id]}
    <div class="acct">
      <ServiceBadge {id} />
      <div class="txt">
        <span class="l">{SERVICES[id].name}</span>
        <span class="d" class:on={a?.connected}
          >{a?.connected ? t('integrations.accounts.connected', { label: a.label }) : t('integrations.accounts.notConnected')}</span
        >
        {#if a?.connected && a.inFile}
          <span class="d warn">{t('integrations.accounts.keychainUnavailable')}</span>
        {/if}
        {#if a?.connected && a.unread}
          <span class="d warn">{t('integrations.accounts.keychainUnreadable')}</span>
        {/if}
        {#if a?.connected && a.plugin}
          <span class="d">{t('integrations.accounts.servedByPlugin', { plugin: a.plugin })}</span>
        {/if}
        {#if BY_FORGE[id]}<span class="d">{t('integrations.accounts.byForge', { forge: forgeName(id) })}</span>{/if}
        {#if id === 'azure'}<span class="d">{t('integrations.accounts.workItems')}</span>{/if}
      </div>
      {#if BY_FORGE[id]}
        {#if !a?.connected}
          <button class="btn" onclick={() => forges?.connectKind(BY_FORGE[id]!)}
            >{t('integrations.accounts.connectForge', { forge: forgeName(id) })}</button
          >
        {/if}
      {:else if a?.connected}
        <button class="btn" onclick={() => disconnect(id)}>{t('integrations.accounts.disconnect')}</button>
      {:else if !form}
        <button class="btn" onclick={() => (forms[id] = blank())}>{t('integrations.accounts.connectEllipsis')}</button>
      {/if}
    </div>
    {#if form && !a?.connected}
      <form
        class="connect"
        aria-label={t('integrations.accounts.formLabel', { service: SERVICES[id].name })}
        onsubmit={(e) => {
          e.preventDefault();
          connect(id);
        }}
      >
        {#if a?.servers && a.servers.length > 1}
          <label
            ><span>{t('integrations.accounts.servedBy')}</span><select class="field" bind:value={form.plugin}>
              {#each a.servers as s (s.plugin)}<option value={s.firstHand ? '' : s.plugin}>{s.name}</option>{/each}
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
          <label
            ><span>{t('integrations.accounts.apiKey')}</span><input class="field mono" bind:value={form.key} spellcheck="false" /></label
          >
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
            <button type="button" class="link" disabled={!form.key.trim()} onclick={() => open(trelloTokenPage(form.key))}
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
        {#if refused[id]}<p class="error" role="alert">{refused[id]}</p>{/if}
        <div class="actions">
          <button type="button" class="btn small ghost" onclick={() => delete forms[id]}>{t('common.cancel')}</button>
          <button type="submit" class="btn small primary" disabled={checking === id}
            >{checking === id ? t('integrations.accounts.checking') : t('integrations.accounts.connect')}</button
          >
        </div>
      </form>
    {/if}
  {/each}
</Group>

<Group title={t('integrations.sync.title')}>
  <Row label={t('integrations.sync.updateStatus')} desc={t('integrations.sync.updateStatusDesc')}>
    <Switch label={t('integrations.sync.updateStatus')} bind:on={s.syncStates} />
  </Row>
  <Row label={t('integrations.sync.loopSummary')} desc={t('integrations.sync.loopSummaryDesc')}>
    <Switch label={t('integrations.sync.loopSummary')} bind:on={s.loopComments} />
  </Row>
  <Row label={t('integrations.sync.extractCriteria')} desc={t('integrations.sync.extractCriteriaDesc')}>
    <Switch label={t('integrations.sync.extractCriteria')} bind:on={s.extractCriteria} />
  </Row>
</Group>

<Group title={t('integrations.autoImport.title')}>
  <Row label={t('integrations.autoImport.enable')} desc={t('integrations.autoImport.enableDesc')}>
    <Switch label={t('integrations.autoImport.enable')} bind:on={s.autoImport} />
  </Row>
  <Row label={t('integrations.autoImport.label')} hint={t('integrations.autoImport.labelHint')}>
    <input
      class="field mono input"
      aria-label={t('integrations.autoImport.label')}
      spellcheck="false"
      placeholder="claude-ready"
      bind:value={s.importLabel}
    />
  </Row>
  <Row label={t('integrations.autoImport.every')}>
    <Chips label={t('integrations.autoImport.every')} options={every} mono bind:value={s.importEvery} />
  </Row>
</Group>

<style>
  .part {
    padding: 26px 0 2px;
  }
  .part:first-child {
    padding-top: 6px;
  }
  :global(.sg) + .part {
    margin-top: 14px;
    border-top: 1px solid var(--line);
  }
  .part h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }
  .part p {
    max-width: 64ch;
    margin: 4px 0 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .part + :global(.sg) {
    border-top: none;
  }
  .acct {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 0;
  }
  :global(.note) + .acct {
    margin-top: 6px;
  }
  .acct + .acct,
  .connect + .acct {
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
