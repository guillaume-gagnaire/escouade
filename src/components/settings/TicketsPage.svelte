<script lang="ts">
  import { COLUMNS } from '../../lib/board';
  import { t } from '../../lib/i18n';
  import { linkSource, pickLabel, SERVICE_IDS, SERVICES, setColumnState, toggleComment } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Column, Container, ExternalState, Project, Service } from '../../lib/types';
  import Group from './Group.svelte';
  import ServiceBadge from './ServiceBadge.svelte';
  import Switch from './Switch.svelte';
  import { tip } from '../../lib/tip';

  // « Tickets importés », a page of a project (design/settings-2-2/PROPOSITION.md §2, page 5): the sources it links,
  // one per service whose account is connected, and what moving its tickets does there (its draft: the modal's
  // project). The accounts themselves, the sync and the automatic import are Escouade's: « Services connectés ».
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);
  const integ = $derived(draft.integrations);
  const connected = $derived(SERVICE_IDS.filter((id) => app.accounts.find((a) => a.service === id)?.connected));
  /** The links whose account is connected, in the services' order. */
  const linked = $derived(SERVICE_IDS.filter((id) => connected.includes(id) && integ.links.some((l) => l.service === id)));

  const linkOf = (id: Service) => integ.links.find((l) => l.service === id);

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

  /** The containers by their place in their service (a ClickUp list's « Space › Folder »), in their order. */
  function groups(list: Container[]): { name: string; items: Container[] }[] {
    const out: { name: string; items: Container[] }[] = [];
    for (const c of list) {
      const name = c.group ?? '';
      const last = out.at(-1);
      if (last && last.name === name) last.items.push(c);
      else out.push({ name, items: [c] });
    }
    return out;
  }

  const stateLabel = (st: ExternalState) =>
    st.id.startsWith('label:') ? t('integrations.autoImport.labelState', { name: st.name }) : st.name;
</script>

<Group title={t('integrations.sources.title', { project: draft.name.trim() || project.name })}>
  {#if !connected.length}
    <div class="empty">
      <p>{t('integrations.sources.connectFirst')}</p>
      <button class="btn" onclick={() => (settingsForm.page = 'services')}>{t('integrations.sources.openServices')}</button>
    </div>
  {/if}
  {#each connected as id (id)}
    {@const list = containers[containersKey(id)] ?? []}
    {@const current = linkOf(id)}
    {@const why = failed[containersKey(id)]}
    <div class="src">
      <div class="who"><ServiceBadge {id} small /><span class="l">{SERVICES[id].name}</span></div>
      <div class="pick">
        <span class="k">{SERVICES[id].container}</span>
        <select
          class="field select"
          aria-label={pickLabel(id)}
          value={linking[containersKey(id)] ?? current?.container ?? ''}
          disabled={!!linking[containersKey(id)]}
          onchange={(e) => pick(id, e.currentTarget.value)}
        >
          <option value="">{t('integrations.sources.none')}</option>
          {#if current && !list.some((c) => c.id === current.container)}<option value={current.container}>{current.name}</option>{/if}
          {#each groups(list) as g, i (i)}
            {#if g.name}
              <optgroup label={g.name}>
                {#each g.items as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
              </optgroup>
            {:else}
              {#each g.items as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
            {/if}
          {/each}
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
        {#each linked as id (id)}<span class="svc"><ServiceBadge {id} small />{SERVICES[id].name}</span>{/each}
        <span class="cm">{t('integrations.mapping.comment')}</span>
      </div>
      {#each COLUMNS as c (c.id)}
        {@const column = t(`board.columns.${c.id}`)}
        <div class="mrow">
          <span class="col"><span class="dot" style:background={c.tone}></span>{column}</span>
          {#each linked as id (id)}
            {@const link = linkOf(id)}
            {@const why = link ? failed[statesKey(id, link.container)] : undefined}
            <span class="svc">
              <select
                class="field select"
                aria-label={t('integrations.mapping.cell', { service: SERVICES[id].name, column })}
                use:tip={why}
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

<style>
  .l {
    font-weight: 600;
  }
  .error {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--del);
    overflow-wrap: anywhere;
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin: 8px 0 0;
  }
  .empty p {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .src {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 12px 0;
  }
  .src + .src {
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
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
    color: var(--muted);
  }
  .select {
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
    padding: 9px 0;
  }
  .mrow:not(:first-child) {
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
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
