<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../lib/i18n';
  import { modelLabel } from '../lib/models';
  import { planView, type PlanRow, type PlanRowState } from '../lib/plan';
  import { readPref, removePref, writePref } from '../lib/prefs';
  import { clamp } from '../lib/resize';
  import { app } from '../lib/state.svelte';
  import type { Agent } from '../lib/types';
  import Splitter from './Splitter.svelte';

  // The plan an agent follows, between its header and its messages: a line (the plan, how far it is), a bar with a
  // segment for each task, and the list of the tasks. What the stream says is worked out in `lib/plan.ts`.
  let { agent }: { agent: Agent } = $props();
  const uid = $props.id();
  const listId = `plan-list-${uid}`;

  /** A window this tall opens the list of a plan whose banner the reader has not folded or unfolded. */
  const TALL = 800;
  /** Dots of subagents drawn in the stack; the count says the rest. */
  const DOTS = 6;
  /** After the reader scrolled the list, it is left alone this long. */
  const QUIET_MS = 6000;
  /** The scroll events that follow the list's own move (it scrolls smoothly) are not the reader's. */
  const OWN_SCROLL_MS = 700;
  /** The least the list can be, and the least the messages keep. */
  const MIN_LIST = 60;
  const MIN_MESSAGES = 160;
  /** Without the messages to measure (the banner alone), what the window keeps besides the list: title bar, header, messages, message field. */
  const OTHER_ROOM = 486;
  /** What a screen reader is told changes at most this often. */
  const ANNOUNCE_MS = 2000;

  const view = $derived(planView(agent, app.ticketOf(agent.id)));
  const hasTasks = $derived((view?.total ?? 0) > 0);
  const focusId = $derived(view?.focusId ?? null);
  const rowCount = $derived(view?.rows.length ?? 0);

  // Open or folded: the reader's choice for this agent, else by the window's height (told again when it changes).
  let tall = $state(window.innerHeight >= TALL);
  const open = $derived(app.planOpen[agent.id] ?? tall);
  const toggle = () => (app.planOpen[agent.id] = !open);

  /** The height the reader gave the list, kept for every plan; none: as tall as the tasks need, up to its share of the window. */
  function keptHeight(): number | null {
    const raw = readPref('planHeight');
    return raw !== null && /^\d+$/.test(raw) && Number(raw) > 0 ? Number(raw) : null;
  }
  let height = $state<number | null>(keptHeight());
  const resetHeight = () => {
    height = null;
    removePref('planHeight');
  };

  let root = $state<HTMLElement>();
  let listEl = $state<HTMLElement>();
  /** How tall the list is, and the tallest it can be: the messages that follow the banner keep 160 px. */
  let listH = $state(0);
  let maxH = $state(MIN_LIST);

  function measure() {
    listH = listEl?.clientHeight ?? 0;
    const next = root?.nextElementSibling;
    maxH = Math.max(MIN_LIST, next instanceof HTMLElement ? next.clientHeight + listH - MIN_MESSAGES : window.innerHeight - OTHER_ROOM);
  }
  $effect(() => {
    if (!root) return;
    // Measured again when what sizes the list changes, and when the banner or the messages are resized (the window).
    void [listEl, open, height, rowCount, hasTasks];
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(root);
    const next = root.nextElementSibling;
    if (next) ro.observe(next);
    return () => ro.disconnect();
  });

  // The row to look at is put in the middle of the list, by the banner, unless the reader scrolled it lately; for
  // another agent it always is. A move of the banner's own is no scrolling of the reader's.
  let userAt = 0;
  let ownAt = 0;
  let centredFor: string | null = null;
  let centredIn: HTMLElement | undefined;
  function centre(force: boolean) {
    if (!listEl) return;
    if (!force && Date.now() - userAt < QUIET_MS) return;
    const row = listEl.querySelector<HTMLElement>('.row.focus');
    if (!row) return;
    ownAt = Date.now();
    listEl.scrollTop = Math.max(0, row.offsetTop - (listEl.clientHeight - row.offsetHeight) / 2);
  }
  function onScroll() {
    if (Date.now() - ownAt > OWN_SCROLL_MS) userAt = Date.now();
  }
  $effect(() => {
    const id = agent.id;
    void [open, listEl, focusId, rowCount];
    const another = id !== centredFor;
    centredFor = id;
    // A list that was not there before (unfolded again, or the next agent's) was scrolled by nobody: what the reader did
    // to the one before says nothing of it.
    if (another || listEl !== centredIn) userAt = 0;
    centredIn = listEl;
    centre(another);
  });

  // What is said to a screen reader: the tasks done (else the subagents running), not more than every 2 seconds, the
  // latest one when the time is up.
  const summary = $derived(
    view ? (view.total ? t('plan.tasks', { done: view.done, count: view.total }) : t('plan.subsRunning', { count: view.subsRunning })) : '',
  );
  let announced = $state('');
  let announcedAt = 0;
  let latest = '';
  let announceTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    latest = summary;
    untrack(() => {
      clearTimeout(announceTimer);
      const say = () => {
        announced = latest;
        announcedAt = Date.now();
      };
      const wait = announcedAt + ANNOUNCE_MS - Date.now();
      if (wait <= 0) say();
      else announceTimer = setTimeout(say, wait);
    });
  });
  $effect(() => () => clearTimeout(announceTimer));

  /** The state of a task in words, read when shown (the language can change). */
  const stateWord = (state: PlanRowState) =>
    t(
      state === 'done'
        ? 'plan.state.done'
        : state === 'run'
          ? 'plan.state.inProgress'
          : state === 'todo'
            ? 'plan.state.todo'
            : 'plan.state.blocked',
    );
  /** How far a task is, or the word for a task under way that nothing measures. */
  const progress = (row: PlanRow) => (row.percent === null ? stateWord(row.state) : t('plan.percent', { percent: row.percent }));
  /** The segment of a task: whole for a task done, to do, or not measured (faint); as far as it is otherwise. */
  const segWidth = (row: PlanRow) => (row.state === 'run' || row.state === 'block' ? (row.percent ?? 100) : 100);
  const titleTip = $derived(view ? (view.file ? `${view.title}\n${t('plan.source', { file: view.file })}` : view.title) : '');
</script>

<svelte:window onresize={() => (tall = window.innerHeight >= TALL)} />

{#snippet subs()}
  {#if view && view.subsRunning > 0}
    <span class="subs">
      <span class="dots" aria-hidden="true"
        >{#each view.subDots.slice(0, DOTS) as dot, i (i)}<span class="dot" title={dot.name}></span>{/each}</span
      >{t('plan.subsRunning', { count: view.subsRunning })}
    </span>
  {/if}
{/snippet}

{#if view}
  <div class="plan" bind:this={root}>
    <div class="inner">
      {#if hasTasks}
        <button
          type="button"
          class="head"
          aria-expanded={open}
          aria-controls={open ? listId : undefined}
          aria-label={t('plan.toggle', { title: view.title })}
          onclick={toggle}
        >
          <span class="badge">{t('plan.badge')}</span>
          <span class="title" title={titleTip}>{view.title}</span>
          {@render subs()}
          <span class="count mono">{t('plan.tasks', { done: view.done, count: view.total })}</span>
          <span class="pct mono">{t('plan.percent', { percent: view.percent })}</span>
          <span class="chev" aria-hidden="true">{open ? '▾' : '▸'}</span>
        </button>
        <div
          class="segs"
          role="progressbar"
          aria-label={t('plan.badge')}
          aria-valuemin={0}
          aria-valuemax={view.total}
          aria-valuenow={view.done}
          aria-valuetext={t('plan.tasks', { done: view.done, count: view.total })}
        >
          {#each view.rows as row (row.n)}
            <span class="seg {row.state}" aria-hidden="true"
              ><span
                class="fill"
                class:faint={row.state === 'todo'}
                class:unmeasured={row.percent === null && row.state !== 'todo'}
                style:width="{segWidth(row)}%"
              ></span></span
            >
          {/each}
        </div>
        {#if open}
          <!-- The role is said again: a list with no bullets is a list no more for the web views of macOS. -->
          <!-- svelte-ignore a11y_no_redundant_roles -->
          <ul
            role="list"
            class="list"
            class:sized={height !== null}
            id={listId}
            aria-label={t('plan.list')}
            bind:this={listEl}
            onscroll={onScroll}
            style:height={height ? `${height}px` : undefined}
          >
            {#each view.rows as row (row.n)}
              <li class="row {row.state}" class:focus={row.focus} data-plan-task={row.n}>
                <span class="n mono">{row.n}</span>
                <span class="ico" title={stateWord(row.state)}>
                  <span class="sr">{stateWord(row.state)}</span>
                  {#if row.state === 'done'}
                    <span class="check" aria-hidden="true">✓</span>
                  {:else if row.state === 'run'}
                    <span class="spin" aria-hidden="true"></span>
                  {:else if row.state === 'block'}
                    <span class="bang" aria-hidden="true">!</span>
                  {:else}
                    <span class="dash" aria-hidden="true"></span>
                  {/if}
                </span>
                <span class="what">
                  <span class="rt" title={row.title}>{row.title}</span>
                  <span class="meta">
                    {#if row.sub}
                      <span class="subtag mono"
                        >{t('plan.subOn', { name: row.sub.name })}{#if row.sub.model}<span class="model"
                            >{modelLabel(row.sub.model, app.models)}</span
                          >{/if}</span
                      >
                    {/if}
                    {#if row.step}<span class="step" title={row.step}>{row.step}</span>{/if}
                    {#if row.state === 'block'}<span class="wait">{t('plan.waiting')}</span>{/if}
                    {#if row.after}<span class="after mono">{t('plan.after', { list: row.after })}</span>{/if}
                  </span>
                </span>
                <span class="rbar" aria-hidden="true"
                  ><span class="rfill" class:unmeasured={row.percent === null} style:width="{row.percent ?? 100}%"></span></span
                >
                <span class="rpct mono" title={row.steps ? t('plan.stepsOf', { done: row.steps[0], count: row.steps[1] }) : undefined}
                  >{progress(row)}</span
                >
              </li>
            {/each}
          </ul>
        {/if}
      {:else}
        <!-- Subagents work, but no task list: who works is all there is to tell. -->
        <div class="head static">
          <span class="badge">{t('plan.badgeSubs')}</span>
          <span class="title" title={view.title}>{view.title}</span>
          {@render subs()}
        </div>
      {/if}
    </div>
    {#if hasTasks && open}
      <Splitter
        axis="y"
        label={t('plan.gripLabel')}
        title={t('plan.grip')}
        value={clamp(listH || height || MIN_LIST, MIN_LIST, maxH)}
        min={MIN_LIST}
        max={maxH}
        onresize={(h) => (height = h)}
        oncommit={(h) => writePref('planHeight', String(h))}
        onreset={resetHeight}
      />
    {/if}
    <span class="sr" role="status" aria-live="polite" aria-atomic="true">{announced}</span>
  </div>
{/if}

<style>
  .plan {
    position: relative;
    flex: 0 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  .inner {
    /* The rows ask for less room when the conversation is narrow (half the window, in the split layout). */
    container-type: inline-size;
    width: 100%;
    max-width: 780px;
    min-height: 0;
    margin: 0 auto;
    padding: 12px 28px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .head.static {
    cursor: default;
  }
  .head:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
    border-radius: 3px;
  }
  .badge {
    flex: none;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 3px 7px;
    border-radius: 3px;
    background: var(--accent);
    color: var(--accent-ink);
  }
  .title {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subs {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
  }
  .dots {
    display: flex;
  }
  .dot {
    display: inline-block;
    width: 16px;
    height: 16px;
    margin-left: -4px;
    border-radius: 50%;
    border: 2px solid var(--panel);
    background: var(--accent-soft);
  }
  .count {
    flex: none;
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
  }
  .pct {
    flex: none;
    min-width: 46px;
    text-align: right;
    font-size: 14px;
    font-weight: 600;
  }
  .chev {
    flex: none;
    font-size: 10px;
    color: var(--dim);
  }

  /* The bar: one segment for each task. */
  .segs {
    display: flex;
    gap: 3px;
    height: 6px;
  }
  .seg {
    flex: 1;
    border-radius: 2px;
    background: var(--elev2);
    overflow: hidden;
  }
  .fill,
  .rfill {
    display: block;
    height: 100%;
    background: var(--st);
    transition: width 1s linear;
  }
  /* The colour of a state, for its segment, its bar and its figure. */
  .done {
    --st: var(--ok);
  }
  .run {
    --st: var(--accent);
  }
  .block {
    --st: var(--wait);
  }
  .todo {
    --st: var(--dim);
  }
  /* A task under way that nothing measures is whole but faint, a task to do fainter: no figure is made up. */
  .unmeasured {
    opacity: 0.35;
  }
  .faint {
    opacity: 0.25;
  }

  /* The list. */
  .list {
    position: relative;
    min-height: 0;
    max-height: min(300px, 28vh);
    margin: 2px -10px 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    overscroll-behavior: contain;
    scroll-behavior: smooth;
  }
  .list.sized {
    max-height: none;
  }
  .row {
    flex: none;
    display: grid;
    grid-template-columns: 22px 20px minmax(0, 1fr) 150px 56px;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: var(--r-sm);
  }
  .row.focus {
    background: color-mix(in oklch, var(--accent) 10%, transparent);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .row:hover {
    background: var(--elev);
  }
  .n {
    font-size: 10.5px;
    /* The rank is what “après 03” refers to: legible, where the design's dimmer colour is not. */
    color: var(--muted);
  }
  .ico {
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .check,
  .bang {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 800;
  }
  .check {
    background: var(--ok);
    color: var(--bg);
  }
  .bang {
    background: var(--wait);
    color: #2a1f05;
  }
  .spin {
    display: inline-block;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid var(--accent-soft);
    border-top-color: var(--accent);
    animation: ccSpin 0.9s linear infinite;
  }
  .dash {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1.5px dashed var(--dim);
  }
  .what {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .rt {
    font-size: 13px;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.todo .rt {
    color: var(--muted);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 11px;
    color: var(--muted);
  }
  .meta:empty {
    display: none;
  }
  .subtag {
    display: flex;
    align-items: center;
    gap: 5px;
    flex: none;
    max-width: 60%;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--elev2);
    color: var(--text);
    font-size: 10.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .model {
    color: var(--muted);
  }
  .step {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wait {
    color: var(--wait);
  }
  .after {
    font-size: 10.5px;
    color: var(--muted);
    white-space: nowrap;
  }
  .rbar {
    height: 4px;
    border-radius: 2px;
    background: var(--elev2);
    overflow: hidden;
  }
  .rpct {
    font-size: 11px;
    text-align: right;
    color: var(--st);
    white-space: nowrap;
  }
  .row.todo .rpct {
    color: var(--muted);
  }
  /* Read by screen readers only. */
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  @container (max-width: 560px) {
    .row {
      grid-template-columns: 22px 20px minmax(0, 1fr) 56px;
    }
    .rbar,
    .head:not(.static) .subs {
      display: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
      border-color: var(--accent);
    }
    .fill,
    .rfill {
      transition: none;
    }
    .list {
      scroll-behavior: auto;
    }
  }
</style>
