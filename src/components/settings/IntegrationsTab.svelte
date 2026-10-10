<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { COLUMNS } from '../../lib/board';
  import { t } from '../../lib/i18n';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { linkSource, SERVICE_IDS, SERVICES, setColumnState, toggleComment, trelloTokenPage } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { AccountForm, Column, Container, ExternalState, Project, Service } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // The external ticket systems: the accounts (connected at once, apart from the draft), the
  // project's sources and what moving its tickets does there (its draft: the modal's project), and
  // the sync and automatic import of every project (the app's settings).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);
  const integ = $derived(draft.integrations);
  const s = $derived(settingsForm.settings.integrations);
  const connected = $derived(SERVICE_IDS.filter((id) => app.accounts.find((a) => a.service === id)?.connected));
  /** The links whose account is connected, in the services' order. */
  const linked = $derived(SERVICE_IDS.filter((id) => connected.includes(id) && integ.links.some((l) => l.service === id)));

  const account = (id: Service) => app.accounts.find((a) => a.service === id);
  const linkOf = (id: Service) => integ.links.find((l) => l.service === id);

  /** « Connecter… »: the open forms, the one being checked, and what each service refused. */
  let forms = $state<Partial<Record<Service, AccountForm>>>({});
  let checking = $state<Service | null>(null);
  let refused = $state<Partial<Record<Service, string>>>({});

  /** Containers by service and project, states by service and container, as fetched (or why not). */
  let containers = $state<Record<string, Container[]>>({});
  let states = $state<Record<string, ExternalState[]>>({});
  let failed = $state<Record<string, string>>({});
  /** The container whose states are on their way, by service and project. */
  let linking = $state<Record<string, string>>({});

  const containersKey = (id: Service) => `${id}|${project.id}`;
  const statesKey = (id: Service, container: string) => `${id}|${container}`;

  $effect(() => {
    for (const id of connected) {
      const key = containersKey(id);
      if (containers[key] || failed[key]) continue;
      containers[key] = [];
      api
        .integrationContainers(id, project.id)
        .then((list) => {
          containers[key] = list;
        })
        .catch((e) => {
          failed[key] = String(e);
        });
    }
  });

  $effect(() => {
    for (const id of linked) {
      const container = linkOf(id)?.container;
      if (!container) continue;
      const key = statesKey(id, container);
      if (states[key] || failed[key]) continue;
      states[key] = [];
      api
        .integrationStates(id, container)
        .then((v) => {
          states[key] = v.states;
        })
        .catch((e) => {
          failed[key] = String(e);
        });
    }
  });

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
      // Its containers are fetched again for the new account.
      delete containers[containersKey(id)];
      delete failed[containersKey(id)];
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

  async function pick(id: Service, containerId: string) {
    // This project's draft, even if another one is chosen meanwhile.
    const target = draft;
    const key = containersKey(id);
    if (!containerId) {
      target.integrations = linkSource(target.integrations, id, null, {});
      return;
    }
    const c = containers[key]?.find((x) => x.id === containerId) ?? { id: containerId, name: containerId };
    linking[key] = containerId;
    try {
      const v = await api.integrationStates(id, containerId);
      states[statesKey(id, containerId)] = v.states;
      target.integrations = linkSource(target.integrations, id, c, v.defaults);
    } catch (e) {
      // Linked all the same, without states: the states say why they are missing.
      failed[statesKey(id, containerId)] = String(e);
      target.integrations = linkSource(target.integrations, id, c, {});
    } finally {
      delete linking[key];
    }
  }

  function setState(id: Service, column: Column, stateId: string) {
    const link = linkOf(id);
    if (!link) return;
    const known = states[statesKey(id, link.container)] ?? [];
    const st = known.find((x) => x.id === stateId) ?? (stateId ? { id: stateId, name: stateId } : null);
    draft.integrations = setColumnState(integ, id, column, st);
  }

  /** The states a column's menu offers: those fetched, and the one chosen even if it is gone. */
  function options(id: Service, column: Column): ExternalState[] {
    const link = linkOf(id);
    if (!link) return [];
    const known = states[statesKey(id, link.container)] ?? [];
    const chosen = link.states[column];
    return chosen && !known.some((x) => x.id === chosen.id) ? [...known, chosen] : known;
  }

  const stateLabel = (st: ExternalState) => (st.id.startsWith('label:') ? `◆ ${st.name}` : st.name);

  const open = (url: string) => app.run(openUrl(url));

  const every = $derived([5, 15, 60].map((n) => ({ value: n, label: t('integrations.autoImport.minutes', { n }) })));
  const blank = (): AccountForm => ({ site: '', email: '', key: '', token: '' });
</script>

{#snippet badge(id: Service, small = false)}
  <span class="badge" class:small style:background={SERVICES[id].color} style:color={SERVICES[id].ink} aria-hidden="true"
    >{SERVICES[id].letter}</span
  >
{/snippet}

<Group title={t('integrations.accounts.title')} note={t('integrations.accounts.note')}>
  {#each SERVICE_IDS as id (id)}
    {@const a = account(id)}
    {@const form = forms[id]}
    <div class="acct">
      {@render badge(id)}
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
      </div>
      {#if a?.connected}
        <button class="btn small ghost" onclick={() => disconnect(id)}>{t('integrations.accounts.disconnect')}</button>
      {:else if !form}
        <button class="btn small" onclick={() => (forms[id] = blank())}>{t('integrations.accounts.connectEllipsis')}</button>
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

<Group title={t('integrations.sources.title', { project: draft.name.trim() || project.name })}>
  {#if !connected.length}
    <p class="empty">{t('integrations.sources.connectFirst')}</p>
  {/if}
  {#each connected as id (id)}
    {@const list = containers[containersKey(id)] ?? []}
    {@const current = linkOf(id)}
    {@const why = failed[containersKey(id)]}
    <div class="src">
      <div class="who">{@render badge(id, true)}<span class="l">{SERVICES[id].name}</span></div>
      <div class="pick">
        <span class="k">{SERVICES[id].container}</span>
        <select
          class="field select"
          aria-label={t(`integrations.sources.pick.${id}`)}
          value={linking[containersKey(id)] ?? current?.container ?? ''}
          disabled={!!linking[containersKey(id)]}
          onchange={(e) => pick(id, e.currentTarget.value)}
        >
          <option value="">{t('integrations.sources.none')}</option>
          {#if current && !list.some((c) => c.id === current.container)}<option value={current.container}>{current.name}</option>{/if}
          {#each list as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
        {#if why}<p class="error">{why}</p>{/if}
      </div>
    </div>
  {/each}
</Group>

{#if linked.length}
  <Group title={t('integrations.mapping.title')} note={t('integrations.mapping.note')}>
    <div class="map" style:--n={linked.length}>
      <div class="mrow head">
        <span class="col">Kanban</span>
        {#each linked as id (id)}<span class="svc">{@render badge(id, true)}{SERVICES[id].name}</span>{/each}
        <span class="cm">{t('integrations.mapping.comment')}</span>
      </div>
      {#each COLUMNS as c (c.id)}
        {@const column = t(`board.columns.${c.id}`)}
        <div class="mrow">
          <span class="col"><span class="dot" style:background={c.color}></span>{column}</span>
          {#each linked as id (id)}
            {@const link = linkOf(id)}
            {@const why = link ? failed[statesKey(id, link.container)] : undefined}
            <span class="svc">
              <select
                class="field select"
                aria-label={t('integrations.mapping.cell', { service: SERVICES[id].name, column })}
                title={why}
                value={link?.states[c.id]?.id ?? ''}
                onchange={(e) => setState(id, c.id, e.currentTarget.value)}
              >
                <option value="">{t('integrations.mapping.unchanged')}</option>
                {#each options(id, c.id) as st (st.id)}<option value={st.id}>{stateLabel(st)}</option>{/each}
              </select>
            </span>
          {/each}
          <span class="cm">
            <Switch
              label={t('integrations.mapping.commentOnArrival', { column })}
              bind:on={() => integ.comments.includes(c.id), () => (draft.integrations = toggleComment(integ, c.id))}
            />
          </span>
        </div>
      {/each}
    </div>
  </Group>
{/if}

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
  .badge {
    min-width: 28px;
    height: 28px;
    padding: 0 3px;
    flex: none;
    border-radius: 7px;
    font-size: 12px;
    font-weight: 800;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .badge.small {
    min-width: 20px;
    height: 20px;
    border-radius: 5px;
    font-size: 9px;
  }
  .acct {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
  }
  .acct:not(:first-child),
  .src:not(:first-child) {
    border-top: 1px solid var(--line);
  }
  .txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .l {
    font-size: 13px;
    font-weight: 600;
  }
  .d {
    font-size: 11.5px;
    color: var(--dim);
    overflow-wrap: anywhere;
  }
  .d.on {
    color: var(--muted);
  }
  .d.warn {
    color: var(--wait);
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
    padding: 4px 16px 14px 58px;
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
    color: var(--dim);
  }
  .connect .field {
    flex: 1;
    min-width: 0;
    height: 32px;
    padding: 0 10px;
    background: var(--panel);
    font-size: 12.5px;
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
    color: var(--accent);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .link:hover:not(:disabled) {
    color: var(--text);
  }
  .link:disabled {
    color: var(--dim);
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
  .empty {
    margin: 0;
    padding: 14px 16px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .src {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 12px 16px;
  }
  .who {
    width: 150px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 9px;
    padding-top: 22px;
  }
  .pick {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .k {
    font-size: 11px;
    color: var(--dim);
  }
  .select {
    height: 30px;
    padding: 0 8px;
    background: var(--panel);
    font-size: 12px;
  }
  .map {
    display: flex;
    flex-direction: column;
  }
  .mrow {
    display: grid;
    grid-template-columns: 130px repeat(var(--n), minmax(0, 1fr)) 80px;
    align-items: center;
    gap: 12px;
    padding: 9px 16px;
  }
  .mrow:not(:first-child) {
    border-top: 1px solid var(--line);
  }
  .mrow.head {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
  }
  .col {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    font-weight: 600;
  }
  .head .col {
    font-size: 11.5px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .svc {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .svc .select {
    width: 100%;
    min-width: 0;
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .cm {
    display: flex;
    justify-content: center;
  }
</style>
