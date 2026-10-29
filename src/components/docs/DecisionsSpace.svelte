<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import {
    DECISION_FILTERS,
    decisionChangedLine,
    decisionChanges,
    decisionCounts,
    decisionDay,
    decisionDayWords,
    decisionNo,
    decisionOrigin,
    filterDecisions,
    replacementsOf,
    type DecisionFilter,
  } from '../../lib/decisions';
  import { docsStore } from '../../lib/docs.svelte';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { DocsDecision, Project } from '../../lib/types';
  import DecisionForm from './DecisionForm.svelte';
  import SpaceHead from './SpaceHead.svelte';

  // « Décisions » (ESC-259, docs/FACTORY.md « Les décisions : un registre, pas un document », design/factory/4-decisions.png):
  // the register, numbered and dated, filtered by state, and the detail of the decision chosen. An agent proposes, a human
  // accepts (« Accepter », a click in the register); an accepted decision is never rewritten: « La remplacer » proposes a
  // new one, which supersedes it once accepted. The decisions a ticket's agent left are accepted when the ticket is.
  let { project }: { project: Project } = $props();

  const all = $derived(docsStore.decisions[project.id] ?? []);
  const tree = $derived(docsStore.tree[project.id]);
  const counts = $derived(decisionCounts(all));

  let filter = $state<DecisionFilter>('all');
  let filterOpen = $state(false);
  const rows = $derived(filterDecisions(all, filter));

  /** The decision read on the right: the one chosen while the filter keeps it, else the first row. */
  let chosen = $state<number | null>(null);
  const shown = $derived(rows.find((d) => d.number === chosen) ?? rows[0] ?? null);

  let accepting = $state<number | null>(null);
  async function accept(d: DocsDecision) {
    accepting = d.number;
    const done = await app.run(api.docsDecisionAccept(project.id, d.number));
    accepting = null;
    if (!done) return;
    app.toast(t('docs.decisions.acceptedToast', { n: decisionNo(d.number) }));
    await docsStore.load(project.id);
  }

  /** « Proposer une décision » open, by hand (null) or to replace an accepted one (its number). */
  let proposing = $state<{ replaces: number | null } | null>(null);

  function pick(f: DecisionFilter) {
    filter = f;
    filterOpen = false;
  }
  /** Shows a decision named from another one (« Remplace », « Remplacée par »), the filter let go if it hides it. */
  function show(n: number) {
    if (!rows.some((d) => d.number === n)) filter = 'all';
    chosen = n;
  }
  function headOf(d: DocsDecision): string {
    const n = decisionNo(d.number);
    if (d.state === 'superseded')
      return t('docs.decisions.head.superseded', { n, by: d.supersededBy != null ? decisionNo(d.supersededBy) : '?' });
    if (d.state === 'proposed') return t('docs.decisions.head.proposed', { n, when: decisionDayWords(d.date, app.now) });
    const when = decisionDayWords(d.acceptedAt || d.date, app.now);
    return d.acceptedBy
      ? t('docs.decisions.head.accepted', { n, when, by: d.acceptedBy })
      : t('docs.decisions.head.acceptedAlone', { n, when });
  }
</script>

<svelte:window
  onclick={(e) => {
    if (filterOpen && !(e.target as Element).closest?.('.filter')) filterOpen = false;
  }}
/>

<SpaceHead title={t('docs.spaces.decisions')}>
  {#snippet line()}
    {#if counts.all}
      <span data-counts
        >{[
          t('docs.decisions.count', { count: counts.all }),
          t('docs.decisions.accepted', { count: counts.accepted }),
          t('docs.decisions.proposed', { count: counts.proposed }),
          t('docs.decisions.superseded', { count: counts.superseded }),
        ].join(' · ')}</span
      >
    {/if}
  {/snippet}
  {#snippet actions()}
    <div class="filter">
      <button
        class="btn"
        aria-haspopup="menu"
        aria-expanded={filterOpen}
        aria-label={t('docs.decisions.filterLabel')}
        onclick={() => (filterOpen = !filterOpen)}>{t(`docs.decisions.filters.${filter}`)}<Icon name="down" size={12} /></button
      >
      {#if filterOpen}
        <div class="menu" role="menu">
          {#each DECISION_FILTERS as f (f)}
            <button role="menuitemradio" aria-checked={filter === f} onclick={() => pick(f)}>
              <span>{t(`docs.decisions.filters.${f}`)}</span>
              <span class="n">{counts[f]}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
    <button class="btn primary" onclick={() => (proposing = { replaces: null })}
      ><Icon name="plus" size={14} />{t('docs.decisions.propose')}</button
    >
  {/snippet}
</SpaceHead>

{#if !all.length}
  <div class="body"><p class="empty">{t('docs.decisions.empty')}</p></div>
{:else}
  <div class="split">
    <div class="reg">
      {#if !rows.length}
        <p class="empty">{t('docs.decisions.emptyFilter')}</p>
      {:else}
        <table aria-label={t('docs.decisions.listLabel')}>
          <thead>
            <tr>
              <th class="c-num">{t('docs.decisions.columns.number')}</th>
              <th>{t('docs.decisions.columns.decision')}</th>
              <th>{t('docs.decisions.columns.state')}</th>
              <th>{t('docs.decisions.columns.changed')}</th>
              <th class="c-date">{t('docs.decisions.columns.date')}</th>
            </tr>
          </thead>
          <tbody>
            {#each rows as d (d.number)}
              {@const origin = decisionOrigin(d)}
              <tr class:on={shown?.number === d.number} data-decision={d.number} data-state={d.state} onclick={() => (chosen = d.number)}>
                <td class="c-num">{decisionNo(d.number)}</td>
                <td class="c-title">
                  <button class="pick" aria-current={shown?.number === d.number ? 'true' : undefined} onclick={() => (chosen = d.number)}
                    >{d.title}</button
                  >
                  {#if origin}<span class="origin">{origin}</span>{/if}
                </td>
                <td class="c-state">
                  <span class="dstate {d.state}">{t(`docs.decisions.states.${d.state}`)}</span>
                  {#if d.state === 'proposed'}
                    <button
                      class="btn accept"
                      aria-label={t('docs.decisions.acceptLabel', { n: decisionNo(d.number) })}
                      disabled={accepting === d.number}
                      use:tip={t('docs.decisions.acceptTip')}
                      onclick={(e) => {
                        e.stopPropagation();
                        void accept(d);
                      }}>{t('docs.decisions.accept')}</button
                    >
                  {/if}
                </td>
                <td class="c-changed">{decisionChangedLine(d, tree)}</td>
                <td class="c-date">{decisionDay(d.date, app.now)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <aside class="detail" aria-label={shown ? shown.title : t('docs.decisions.none')}>
      {#if !shown}
        <p class="empty">{t('docs.decisions.none')}</p>
      {:else}
        {@const d = shown}
        {@const changes = decisionChanges(d, tree)}
        {@const origin = decisionOrigin(d)}
        {@const replacing = replacementsOf(d, all)}
        <div class="k" data-head>{headOf(d)}</div>
        <h2>{d.title}</h2>
        {#if d.state === 'proposed'}
          <div class="acts">
            <button class="btn primary" disabled={accepting === d.number} onclick={() => accept(d)}
              ><Icon name="check" size={14} />{t('docs.decisions.accept')}</button
            >
            {#if d.origin === 'ticket' && d.originRef}<span class="hint">{t('docs.decisions.fromTicket', { key: d.originRef })}</span>{/if}
          </div>
        {:else if d.state === 'accepted'}
          <div class="acts">
            <button class="btn" use:tip={t('docs.decisions.replaceTip')} onclick={() => (proposing = { replaces: d.number })}
              >{t('docs.decisions.replace')}</button
            >
          </div>
        {/if}
        {#if d.supersedes.length || d.supersededBy != null || replacing.length}
          <div class="chips" data-lineage>
            {#each d.supersedes as n (n)}
              <button class="chip" aria-label={t('docs.decisions.openDecision', { n: decisionNo(n) })} onclick={() => show(n)}
                >{t('docs.decisions.replaces')} {decisionNo(n)}</button
              >
            {/each}
            {#if d.supersededBy != null}
              {@const by = d.supersededBy}
              <button class="chip" aria-label={t('docs.decisions.openDecision', { n: decisionNo(by) })} onclick={() => show(by)}
                >{t('docs.decisions.replacedBy')} {decisionNo(by)}</button
              >
            {/if}
            {#each replacing as r (r.number)}
              <button
                class="chip wait"
                aria-label={t('docs.decisions.openDecision', { n: decisionNo(r.number) })}
                onclick={() => show(r.number)}>{t('docs.decisions.replacementProposed')} · {decisionNo(r.number)}</button
              >
            {/each}
          </div>
        {/if}
        {#if d.context}<div class="k">{t('docs.decisions.context')}</div>
          <p>{d.context}</p>{/if}
        {#if d.decision}<div class="k">{t('docs.decisions.decision')}</div>
          <p>{d.decision}</p>{/if}
        {#if d.consequences}<div class="k">{t('docs.decisions.consequences')}</div>
          <p>{d.consequences}</p>{/if}
        <div class="k">{t('docs.decisions.changed')}</div>
        {#if changes.length}
          <div class="chips" data-changes>
            {#each changes as c (c.path)}
              {@const label = c.added == null ? c.title : `${c.title} +${c.added} −${c.removed}`}
              {#if c.known}
                <button
                  class="chip"
                  aria-label={t('docs.decisions.openPage', { name: c.title })}
                  onclick={() => app.openDocs(project.id, 'spec', c.path)}>{label}</button
                >
              {:else}
                <span class="chip">{label}</span>
              {/if}
            {/each}
          </div>
        {:else}
          <p class="dim">{t('docs.decisions.nothingChanged')}</p>
        {/if}
        {#if d.tickets.length || origin || d.documents.length}
          <div class="k">{t('docs.decisions.links')}</div>
          <div class="chips" data-links>
            {#each d.tickets as k (k)}<span class="chip">{k}</span>{/each}
            {#if origin && !(d.origin === 'ticket' && d.tickets.includes(d.originRef))}<span class="chip">{origin}</span>{/if}
            {#each d.documents as doc (doc)}
              <button class="chip" onclick={() => app.openDocs(project.id, 'documents')}>{doc.split('/').pop()}</button>
            {/each}
          </div>
        {/if}
      {/if}
    </aside>
  </div>
{/if}

{#if proposing}
  <DecisionForm
    {project}
    replaces={proposing.replaces}
    onclose={() => (proposing = null)}
    ondone={(d) => {
      proposing = null;
      filter = 'all';
      chosen = d.number;
    }}
  />
{/if}

<style>
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 18px 22px;
  }
  .empty {
    margin: 0;
    color: var(--muted);
    line-height: 1.5;
  }
  .filter {
    position: relative;
  }
  .filter .btn {
    gap: 6px;
  }
  .menu {
    position: absolute;
    z-index: 20;
    top: calc(100% + 4px);
    right: 0;
    min-width: 180px;
    display: grid;
    padding: 4px;
    border-radius: 11px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 18px 48px -10px rgba(0, 0, 0, calc(0.8 * var(--shade)));
  }
  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 30px;
    padding: 0 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--soft);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .menu button:hover,
  .menu button:focus-visible {
    background: var(--hover);
    color: var(--text);
  }
  .menu button[aria-checked='true'] {
    color: var(--text);
    font-weight: 560;
  }
  .menu .n {
    margin-left: auto;
    color: var(--dim);
    font-size: 12px;
  }

  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(280px, 1fr);
  }
  .reg {
    min-height: 0;
    overflow: auto;
    padding: 6px 18px 18px;
  }
  .reg > .empty {
    padding: 14px 4px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 12px 10px 8px;
    border-bottom: 1px solid var(--line);
    background: var(--frame);
    color: var(--muted);
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }
  td {
    padding: 10px;
    border-bottom: 1px solid var(--line);
    vertical-align: top;
  }
  tbody tr {
    cursor: pointer;
  }
  tbody tr:hover {
    background: var(--hover);
  }
  tbody tr.on {
    background: var(--raise-2);
  }
  .c-num {
    width: 44px;
    color: var(--soft);
  }
  .c-date {
    width: 56px;
    color: var(--soft);
    white-space: nowrap;
  }
  .c-title {
    min-width: 0;
  }
  .pick {
    display: block;
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-weight: 620;
    text-align: left;
    cursor: pointer;
  }
  .origin {
    display: block;
    margin-top: 2px;
    color: var(--soft);
  }
  .c-state {
    white-space: nowrap;
  }
  .c-state .accept {
    display: flex;
    height: 24px;
    margin-top: 6px;
    padding: 0 9px;
    font-size: 12px;
  }
  .c-changed {
    color: var(--soft);
  }
  .dstate {
    display: inline-block;
    padding: 2px 9px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 560;
    background: oklch(from var(--run) l c h / 0.12);
    color: var(--run);
  }
  .dstate.proposed {
    background: oklch(from var(--wait) l c h / 0.14);
    color: var(--wait);
  }
  .dstate.superseded {
    background: var(--hover);
    color: var(--muted);
  }

  .detail {
    min-height: 0;
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 10px;
    padding: 18px 22px 24px;
    border-left: 1px solid var(--line);
  }
  .detail .k {
    margin-top: 4px;
    color: var(--muted);
    font-size: 11.5px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }
  .detail .k[data-head] {
    margin-top: 0;
  }
  .detail h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 640;
    letter-spacing: -0.01em;
  }
  .detail p {
    margin: 0;
    color: var(--soft);
    line-height: 1.5;
    white-space: pre-line;
  }
  .detail p.dim {
    color: var(--dim);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .hint {
    color: var(--dim);
    font-size: 12px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    padding: 3px 10px;
    border: 1px solid var(--line-2);
    border-radius: 999px;
    background: var(--raise);
    color: var(--soft);
    font: inherit;
    font-size: 12.5px;
  }
  button.chip {
    cursor: pointer;
  }
  button.chip:hover {
    color: var(--text);
    border-color: var(--line-3);
  }
  .chip.wait {
    color: var(--wait);
  }
</style>
