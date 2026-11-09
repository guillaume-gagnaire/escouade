<script lang="ts">
  import { untrack } from 'svelte';
  import { startAgentOnPr } from '../../lib/branch-actions';
  import { fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import {
    FILTERS,
    LIST_EVERY_MS,
    ageOf,
    chipsOf,
    ciOf,
    filterCounts,
    filterPrs,
    forkBranch,
    forkOwner,
    moveIn,
    ownerOf,
    rowTone,
    saveFilter,
    savedFilter,
    type PrFilter,
  } from '../../lib/prlist';
  import { openPr, prKey } from '../../lib/prs';
  import { reviewMark } from '../../lib/prreview';
  import { failureTitle } from '../../lib/prview';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { PrFailure, PrRead, Project, ProjectPrs, PullRequest } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import PrReviewView from './PrReviewView.svelte';

  // The project's « Pull requests » view (ESC-65, direction A of design/esc-65): beside the Kanban, its open pull requests
  // in a dense list (CI, reviews, conflict, author, age), three filters, and on the row chosen « Lancer un agent » (Entrée)
  // and the host's page. Read when shown, then every minute while the window is visible; nothing is written on the host.
  let { project }: { project: Project } = $props();

  let read = $state<PrRead<ProjectPrs> | null>(null);
  let readAt = $state<number | null>(null);
  let reading = $state(false);
  let filter = $state<PrFilter>(savedFilter());
  /** The row chosen (its key), the one Entrée acts on. */
  let chosen = $state<string | null>(null);
  /** The pull request an agent is being started on. */
  let starting = $state<number | null>(null);

  const data = $derived(read?.data ?? null);
  const word = $derived(data?.word ?? app.prWords[project.id] ?? 'PR');
  const me = $derived(data?.me ?? '');
  const all = $derived(data?.prs ?? []);
  const shown = $derived(filterPrs(all, filter, me));
  const counts = $derived(filterCounts(all, me));
  const agents = $derived(Object.values(app.agents).filter((a) => a.projectId === project.id));
  const at = $derived(shown.findIndex((p) => prKey(p) === chosen));
  const reviews = $derived(app.prReviews[project.id] ?? {});
  /** The review that fills the view, in place of the list (ESC-66). */
  const reviewing = $derived.by(() => {
    const n = app.prReviewOpen[project.id];
    return n == null ? null : (reviews[n] ?? null);
  });

  async function load() {
    const id = project.id;
    reading = true;
    let next: PrRead<ProjectPrs>;
    try {
      next = await api.projectPrs(id);
    } catch (e) {
      next = { failure: { kind: 'other', message: String(e) } as PrFailure };
    }
    if (project.id !== id) return;
    reading = false;
    // A read that fails keeps what was shown, and says why above it.
    if (next.failure && read?.data) next = { data: read.data, failure: next.failure };
    read = next;
    if (next.data && !next.failure) {
      readAt = Date.now();
      app.prWords[id] = next.data.word;
    }
  }

  $effect(() => {
    void project.id;
    untrack(() => {
      read = null;
      readAt = null;
      chosen = null;
      void load();
      app.loadPrReviews(project.id).catch(() => {});
    });
    const timer = setInterval(() => {
      if (document.visibilityState === 'visible' && !reading) void load();
    }, LIST_EVERY_MS);
    return () => clearInterval(timer);
  });

  function setFilter(f: PrFilter) {
    filter = f;
    saveFilter(f);
  }

  const rows: HTMLButtonElement[] = [];
  function choose(i: number, focus = true) {
    const p = shown[i];
    if (!p) return;
    chosen = prKey(p);
    if (focus) rows[i]?.focus();
  }

  async function act(p: PullRequest) {
    const owner = ownerOf(agents, p, word);
    if (owner) return app.selectAgent(owner.id);
    if (starting !== null) return;
    starting = p.number;
    try {
      await startAgentOnPr(project.id, p.number, p.fork ? forkBranch(word, p.number) : p.head);
    } finally {
      starting = null;
    }
  }

  function onKey(e: KeyboardEvent, i: number) {
    if (e.key === 'Enter') {
      e.preventDefault();
      void act(shown[i]);
    } else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) {
      e.preventDefault();
      choose(moveIn(shown.length, i, e.key));
    }
  }
</script>

{#snippet failed(f: PrFailure, banner: boolean)}
  <div class="err" class:banner role="alert">
    <Icon name="info" size={15} />
    <div class="err-t">
      <b>{failureTitle(f)}</b>
      {#if f.kind === 'refused' || f.kind === 'rateLimit' || f.kind === 'unreachable'}<span class="why">{f.message}</span>{/if}
    </div>
    {#if f.kind === 'refused' || f.settings}
      <button class="btn" onclick={() => (app.modal = { kind: 'settings', page: 'services' })}>{t('prs.view.noPr.settings')}</button>
    {/if}
    {#if f.kind !== 'unavailable'}
      <button class="btn" onclick={() => void load()}><Icon name="restart" size={14} />{t('prs.view.retry')}</button>
    {/if}
  </div>
{/snippet}

{#if reviewing}
  <PrReviewView {project} review={reviewing} {me} />
{:else}
  <main class="prs" aria-busy={reading}>
    <header class="head">
      <div class="h-title">
        <h1>{word === 'MR' ? t('prs.list.titleMr') : t('prs.list.title')}</h1>
        <div class="crumbs">
          <span class="proj-name">{project.name}</span>
          {#if data}<span aria-hidden="true">·</span><span class="num">{t('prs.list.open', { count: all.length })}</span>{/if}
        </div>
      </div>
      <div class="segmented filters" role="tablist" aria-label={t('prs.list.filterLabel', { word })}>
        {#each FILTERS as f (f)}
          <button role="tab" aria-selected={filter === f} onclick={() => setFilter(f)}
            >{t(`prs.list.filter.${f}`)}{#if data}<span class="count num">{counts[f]}</span>{/if}</button
          >
        {/each}
      </div>
      {#if readAt}<span class="read">{t('prs.list.readAt', { time: fTime(readAt) })}</span>{/if}
      <button
        class="icon-btn"
        class:spin={reading}
        aria-label={t('prs.list.refresh')}
        use:tip={t('prs.list.refresh')}
        onclick={() => void load()}><Icon name="restart" size={15} /></button
      >
    </header>

    {#if !read}
      <div class="state" role="status">
        <span class="spinner" aria-hidden="true"></span>
        <p>{t('prs.view.project.loading', { word })}</p>
      </div>
    {:else if !data && read.failure}
      <div class="state">{@render failed(read.failure, false)}</div>
    {:else if data}
      {#if read.failure}{@render failed(read.failure, true)}{/if}
      {#if !me && filter !== 'all'}<p class="note">{t('prs.list.noMe')}</p>{/if}
      {#if !shown.length}
        <div class="state">
          <span class="empty-icon"><Icon name="pr" size={22} /></span>
          <p class="empty-t">{t(`prs.list.empty.${filter}`, { word })}</p>
          <p>{t('prs.list.emptyHint', { word })}</p>
          {#if filter !== 'all'}
            <button class="btn ghost" onclick={() => setFilter('all')}><Icon name="pr" size={14} />{t('prs.list.showAll', { word })}</button
            >
          {/if}
        </div>
      {:else}
        <ul class="list" aria-label={t('prs.list.filterLabel', { word })}>
          {#each shown as p, i (prKey(p))}
            {@const owner = ownerOf(agents, p, word)}
            {@const ci = ciOf(p)}
            {@const fork = forkOwner(p)}
            {@const branch = p.fork ? forkBranch(word, p.number) : p.head}
            {@const review = reviews[p.number]}
            <li class="row" class:on={i === at} class:draft={p.draft}>
              <button
                class="main"
                bind:this={rows[i]}
                aria-label={t('prs.list.row', { word, number: p.number, title: p.title })}
                aria-current={i === at ? 'true' : undefined}
                onclick={() => choose(i)}
                ondblclick={() => void act(p)}
                onkeydown={(e) => onKey(e, i)}
              >
                <span class="st {rowTone(p)}"><Icon name={p.draft ? 'pr' : ci.icon} size={14} /></span>
                <span class="n num">#{p.number}</span>
                <span class="title">
                  <span class="t">{p.title}</span>
                  {#if p.draft}<span class="tag">{t('prs.list.draft')}</span>{/if}
                  {#if fork}<span class="tag fork" use:tip={t('prs.list.forkHint', { repo: p.headRepo ?? '?', branch: p.head })}
                      ><Icon name="branch" size={11} />{t('prs.list.fork', { owner: fork })}</span
                    >{/if}
                  {#if owner}<span class="tag agent" use:tip={t('prs.list.agentHint', { agent: owner.name })}
                      ><Icon name="agents" size={11} />{owner.name}</span
                    >{/if}
                  {#if review}{@const mark = reviewMark(review)}<span class="tag review {mark.tone}">{mark.text}</span>{/if}
                </span>
                <span class="chips">
                  {#each chipsOf(p, me) as c (c.key)}<span class="chip {c.tone}"><Icon name={c.icon} size={12} />{c.text}</span>{/each}
                </span>
                <span class="ci {ci.tone}">{ci.text}</span>
                <span class="by">{p.author ?? ''}</span>
                <span class="age num">{ageOf(p, app.now)}</span>
              </button>
              <span class="acts">
                {#if review}
                  <button
                    class="btn small"
                    use:tip={t('prs.review.showHint', { word })}
                    onclick={() => app.showPrReview(project.id, p.number)}><Icon name="read" size={13} />{t('prs.review.show')}</button
                  >
                {:else}
                  <button
                    class="btn small"
                    use:tip={t('prs.review.askHint', { word })}
                    onclick={() => void app.startPrReview(project.id, p.number)}><Icon name="read" size={13} />{t('prs.review.ask')}</button
                  >
                {/if}
                {#if owner}
                  <button
                    class="btn small"
                    onclick={() => app.selectAgent(owner.id)}
                    use:tip={t('prs.list.openAgentHint', { agent: owner.name, word })}
                    ><Icon name="agents" size={13} />{t('prs.list.openAgent')}</button
                  >
                {:else}
                  <button
                    class="btn small"
                    disabled={starting !== null}
                    onclick={() => void act(p)}
                    use:tip={t('prs.list.launchHint', { word, branch })}
                    ><Icon name="agents" size={13} />{starting === p.number ? t('prs.list.launching') : t('prs.list.launch')}</button
                  >
                {/if}
                <button
                  class="icon-btn"
                  aria-label={t('prs.view.openInBrowser')}
                  use:tip={t('prs.view.openInBrowser')}
                  onclick={() => openPr(p)}><Icon name="ext" size={14} /></button
                >
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </main>
{/if}

<style>
  .prs {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  /* The Kanban's header: the title and its line, then the filters and the refresh. */
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 60px;
    padding: 0 14px 0 22px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .h-title {
    display: grid;
    gap: 2px;
    min-width: 0;
    margin-right: auto;
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }
  .crumbs {
    display: flex;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
  }
  .proj-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .filters button {
    gap: 6px;
  }
  .count {
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 500;
  }
  .filters button[aria-selected='true'] .count {
    color: var(--soft);
  }
  .read {
    color: var(--dim);
    font-size: 12px;
  }
  .spin :global(.icon) {
    animation: turn 0.9s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin :global(.icon) {
      animation: none;
    }
  }
  @media (max-width: 1100px) {
    .read {
      display: none;
    }
  }

  /* The states: reading, nothing to show, a failure. */
  .state {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 8px;
    padding: 40px;
    color: var(--muted);
    text-align: center;
  }
  .state p {
    margin: 0;
    max-width: 46ch;
  }
  .empty-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin-bottom: 4px;
    border-radius: 12px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .empty-t {
    color: var(--text);
    font-size: 14px;
    font-weight: 560;
  }
  .state .btn {
    margin-top: 8px;
  }
  .spinner {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--line-2);
    border-top-color: var(--soft);
    animation: turn 0.9s linear infinite;
  }
  .err {
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: 600px;
    padding: 14px 16px;
    border-radius: 10px;
    color: var(--del);
    text-align: left;
    background: color-mix(in oklch, var(--del) 7%, var(--panel));
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 35%, transparent);
  }
  .err.banner {
    max-width: none;
    margin: 10px 16px 0;
    padding: 9px 12px;
  }
  .err-t {
    flex: 1;
    display: grid;
    gap: 3px;
  }
  .err b {
    color: var(--text);
    font-weight: 600;
  }
  .why {
    color: var(--soft);
  }
  .err .btn {
    flex: none;
  }
  .note {
    margin: 10px 22px 0;
    color: var(--muted);
    font-size: 12.5px;
  }

  /* A row of 34 px per pull request, its columns aligned; the chosen one shows its actions over its end. */
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    margin: 0;
    padding: 6px 10px 12px;
    list-style: none;
  }
  .row {
    position: relative;
    border-radius: 7px;
  }
  .row + .row {
    box-shadow: 0 -1px 0 var(--line);
  }
  .row:hover,
  .row.on {
    background: var(--hover);
  }
  .row.on {
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .row.on + .row,
  .row:hover + .row {
    box-shadow: none;
  }
  .main {
    display: grid;
    grid-template-columns: 18px 46px minmax(0, 1fr) auto 92px minmax(0, 88px) 44px;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: default;
  }
  .main:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 60%, transparent);
  }
  .st {
    display: grid;
    place-items: center;
  }
  .ok {
    color: var(--run);
  }
  .wait {
    color: var(--wait);
  }
  .fail {
    color: var(--del);
  }
  .dim {
    color: var(--muted);
  }
  .st.dim {
    color: var(--idle);
  }
  .n,
  .by,
  .age,
  .ci {
    font-size: 12px;
    white-space: nowrap;
  }
  .n,
  .by,
  .age {
    color: var(--muted);
  }
  .by {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .age {
    text-align: right;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }
  .t {
    overflow: hidden;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .draft .t {
    color: var(--muted);
  }
  .tag {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 18px;
    padding: 0 6px;
    border-radius: 5px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 520;
    white-space: nowrap;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .tag.fork {
    color: var(--info);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--info) 35%, transparent);
  }
  .tag.review.ask {
    color: var(--wait);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .tag.review.fail {
    color: var(--del);
  }
  .tag.agent {
    color: var(--p-ink);
    background: var(--p-soft);
    box-shadow: none;
  }
  .chips {
    display: flex;
    justify-content: flex-end;
    gap: 5px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 7px;
    border-radius: 6px;
    font-size: 11.5px;
    font-weight: 520;
    white-space: nowrap;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .chip.muted {
    color: var(--muted);
  }
  .chip.ask {
    color: var(--wait);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .acts {
    position: absolute;
    top: 3px;
    right: 6px;
    bottom: 3px;
    display: none;
    align-items: center;
    gap: 4px;
    padding-left: 24px;
    background: linear-gradient(90deg, transparent, var(--panel) 22px);
  }
  .row:hover .acts,
  .row.on .acts,
  .row:focus-within .acts {
    display: flex;
  }
  .btn.small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  /* Narrow: the author and the CI's words give their room to the title. */
  @media (max-width: 1150px) {
    .main {
      grid-template-columns: 18px 46px minmax(0, 1fr) auto 44px;
    }
    .ci,
    .by {
      display: none;
    }
  }
</style>
