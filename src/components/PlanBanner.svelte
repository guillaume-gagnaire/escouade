<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { t } from '../lib/i18n';
  import { menu } from '../lib/menu.svelte';
  import { modelLabel } from '../lib/models';
  import { planView, type PlanRow, type PlanRowState } from '../lib/plan';
  import { readPref, removePref, writePref } from '../lib/prefs';
  import { clamp } from '../lib/resize';
  import { app } from '../lib/state.svelte';
  import type { Agent } from '../lib/types';
  import Icon from './Icon.svelte';
  import Splitter from './Splitter.svelte';
  import { tip } from '../lib/tip';

  // The plan an agent follows, between its header and its messages: a line (the plan, how far it is), a bar with a
  // segment for each task, and the list of the tasks. What the stream says is worked out in `lib/plan.ts`.
  // `folded`: held folded, the reader's choice aside (the « Workspace » at mid-height); its button calls `onunfold`.
  let { agent, folded = false, onunfold }: { agent: Agent; folded?: boolean; onunfold?: () => void } = $props();
  const uid = $props.id();
  const listId = `plan-list-${uid}`;

  /** A window this tall opens the list of a plan whose banner the reader has not folded or unfolded: the preview's small
   *  window (1028 × 779) has it open. */
  const TALL = 760;
  /** Dots of subagents drawn in the stack; the count says the rest, and the menu of the stack lists them all. */
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
  const open = $derived(!folded && (app.planOpen[agent.id] ?? tall));
  const toggle = () => (folded ? onunfold?.() : (app.planOpen[agent.id] = !open));

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
  /** The share of its segment a task fills: whole once done, none to do, as far as it is under way (whole but faint when
   *  nothing measures it). */
  const segShare = (row: PlanRow) => (row.state === 'done' ? 1 : row.state === 'todo' ? 0 : (row.percent ?? 100) / 100);
  const titleTip = $derived(view ? (view.file ? `${view.title}\n${t('plan.source', { file: view.file })}` : view.title) : '');

  /**
   * The stack of dots opens the conversation of the one subagent that runs; of several, it opens a menu of them, whose entry
   * opens that one. The menu gives the focus back to the stack when it closes, but not in a web view that gives none to a button
   * a click presses; and `openSubagent` notes the element that has the focus as the place to come back to (the stack, by
   * `data-open-from`: `Conversation` finds it again, drawn anew). So the entry waits for the menu to be gone, and the stack takes
   * the focus.
   */
  function openFromStack(stack: HTMLElement) {
    const dots = view?.subDots ?? [];
    if (dots.length === 1) return app.openSubagent(agent.id, dots[0].id);
    if (!dots.length) return;
    menu.showAt(
      stack,
      dots.map((dot) => ({
        label: dot.name,
        ...(dot.model ? { hint: modelLabel(dot.model, app.models) } : {}),
        onClick: async () => {
          await tick();
          stack.focus();
          app.openSubagent(agent.id, dot.id);
        },
      })),
    );
  }
</script>

<svelte:window onresize={() => (tall = window.innerHeight >= TALL)} />

<!-- The subagents running: a dot for each, breathing, in one button, and their count. The button opens the conversation of
     the subagent in place of the agent's, or a menu of them to pick one when several run. The dots are drawings. -->
{#snippet subs()}
  {#if view && view.subsRunning > 0}
    {@const dots = view.subDots}
    {@const only = dots.length === 1 ? dots[0] : null}
    <span class="subs">
      <button
        type="button"
        class="stack"
        data-open-sub={only?.id}
        data-open-from="dots"
        aria-haspopup={only ? undefined : 'menu'}
        aria-label={only ? t('subagents.openRow', { name: only.name }) : t('subagents.openAny')}
        use:tip={only ? only.name : t('subagents.openAny')}
        onclick={(e) => openFromStack(e.currentTarget)}
        >{#each dots.slice(0, DOTS) as dot (dot.id)}<span class="dot" aria-hidden="true"></span>{/each}</button
      >{t('plan.subsRunning', { count: view.subsRunning })}
    </span>
  {/if}
{/snippet}

{#if view}
  <div class="plan" bind:this={root}>
    <div class="inner">
      {#if hasTasks}
        <!-- The subagents are beside the button that folds the banner, not in it (no button in a button): the button
             spans the row, they sit above it, where they stood inside it. -->
        <div class="head tasks">
          <button
            type="button"
            class="fold"
            aria-expanded={open}
            aria-controls={open ? listId : undefined}
            aria-label={t('plan.toggle', { title: view.title })}
            onclick={toggle}
          >
            <span class="badge">{t('plan.badge')}</span>
            <span class="title" use:tip={titleTip}>{view.title}</span>
            <span class="count">{t('plan.tasks', { done: view.done, count: view.total })}</span>
            <span class="pct">{t('plan.percent', { percent: view.percent })}</span>
            <span class="chev" class:closed={!open}><Icon name="down" size={14} /></span>
          </button>
          {@render subs()}
        </div>
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
              ><span class="fill" class:unmeasured={row.percent === null && row.state !== 'todo'} style:--w={segShare(row)}></span></span
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
                <span class="n">{row.n}</span>
                <span class="ico" use:tip={stateWord(row.state)}>
                  <span class="sr">{stateWord(row.state)}</span>
                  {#if row.state === 'done'}
                    <span class="check" aria-hidden="true"><Icon name="check-c" size={14} /></span>
                  {:else if row.state === 'run'}
                    <span class="spin" aria-hidden="true"></span>
                  {:else if row.state === 'block'}
                    <span class="hold" aria-hidden="true"></span>
                  {:else}
                    <span class="ring" aria-hidden="true"></span>
                  {/if}
                </span>
                <span class="what">
                  <span class="rt" use:tip={{ text: row.title, cut: true }}>{row.title}</span>
                  <span class="meta">
                    {#if row.sub}
                      {@const sub = row.sub}
                      <!-- Its conversation opens in place of the agent's. -->
                      <button
                        type="button"
                        class="subtag"
                        data-open-sub={sub.id}
                        data-open-from="task"
                        use:tip={t('subagents.openRow', { name: sub.name })}
                        onclick={() => app.openSubagent(agent.id, sub.id)}
                        ><Icon name="sub" size={12} /><span class="sname">{sub.name}</span>{#if sub.model}<span class="model"
                            >{modelLabel(sub.model, app.models)}</span
                          >{/if}</button
                      >
                    {/if}
                    {#if row.step}<span class="step" use:tip={{ text: row.step, cut: true }}>{row.step}</span>{/if}
                    {#if row.state === 'block'}<span class="wait">{t('plan.waiting')}</span>{/if}
                    {#if row.after}<span class="after">{t('plan.after', { list: row.after })}</span>{/if}
                  </span>
                </span>
                <span class="rbar" aria-hidden="true"
                  ><span class="rfill" class:unmeasured={row.percent === null} style:--w={(row.percent ?? 100) / 100}></span></span
                >
                <span class="rpct" use:tip={row.steps ? t('plan.stepsOf', { done: row.steps[0], count: row.steps[1] }) : undefined}
                  >{progress(row)}</span
                >
              </li>
            {/each}
          </ul>
        {/if}
      {:else}
        <!-- Subagents work, but no task list: who works is all there is to tell, and each one opens. -->
        <div class="head static">
          <span class="badge">{t('plan.badgeSubs')}</span>
          <span class="title" use:tip={{ text: view.title, cut: true }}>{view.title}</span>
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
  /* The banner of the 2.0 design (design/app-redesign/index.html, `.plan-box`): under the header, across the
     conversation, its rows in the thread's column of 780 px. */
  .plan {
    position: relative;
    flex: 0 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-bottom: 1px solid var(--line);
    background: rgba(var(--fg-rgb), 0.012);
  }
  /* A column that gives up the list's height first when the window is short. */
  .inner {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(780px, 100% - 64px);
    min-height: 0;
    margin: 0 auto;
    padding: 12px 0 10px;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    font-size: 12.5px;
    white-space: nowrap;
  }
  /* With tasks: a grid, so that the subagents sit right after the title while the button that folds the banner (the
     whole row, in the subgrid below) holds everything else; the count, the share and the chevron keep to the right. */
  .head.tasks {
    display: grid;
    grid-template-columns:
      [badge] auto [title] minmax(0, max-content) [subs] auto [room] minmax(0, 1fr)
      [count] auto [pct] auto [chev] auto;
    column-gap: 10px;
  }
  .fold {
    grid-column: 1 / -1;
    grid-row: 1;
    display: grid;
    grid-template-columns: subgrid;
    align-items: center;
    min-width: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .fold > .badge {
    grid-column: badge;
  }
  .fold > .title {
    grid-column: title;
  }
  .fold > .count {
    grid-column: count;
  }
  .fold > .pct {
    grid-column: pct;
  }
  .fold > .chev {
    grid-column: chev;
  }
  .head.tasks > .subs {
    grid-column: subs;
    grid-row: 1;
  }
  .fold:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 3px;
    border-radius: 4px;
  }
  .badge {
    flex: none;
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 7px;
    border-radius: 6px;
    background: color-mix(in oklch, var(--p) 18%, transparent);
    color: var(--p-ink);
    font-size: 11.5px;
    font-weight: 650;
  }
  .title {
    min-width: 0;
    overflow: hidden;
    color: var(--text);
    font-weight: 600;
    text-overflow: ellipsis;
  }
  .subs {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  /* One button for all the dots: it opens the conversation of the subagent, or a menu of them. Its target is 24 px
     square at least (WCAG 2.5.8); its margins give back the room its padding takes, so the dots keep their place. */
  .stack {
    flex: none;
    display: inline-flex;
    align-items: center;
    min-width: 24px;
    min-height: 24px;
    margin: -8px;
    padding: 8px;
    border: none;
    border-radius: 12px;
    background: transparent;
    cursor: pointer;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--p-ink);
    box-shadow: 0 0 0 2px var(--panel);
    animation: breathe 1.8s ease-in-out infinite;
  }
  .dot + .dot {
    margin-left: -2px;
  }
  .stack:hover .dot,
  .stack:focus-visible .dot {
    background: var(--text);
  }
  .stack:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -4px;
  }
  @keyframes breathe {
    50% {
      opacity: 0.45;
    }
  }
  .count {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .pct {
    min-width: 34px;
    color: var(--soft);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .chev {
    color: var(--muted);
    transition: transform 0.25s var(--ease);
  }
  .chev.closed {
    transform: rotate(-90deg);
  }

  /* The bar: one segment for each task. */
  .segs {
    flex: none;
    display: flex;
    gap: 3px;
  }
  .seg {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.09);
    overflow: hidden;
  }
  /* A fill as far as the task is (`--w`, from 0 to 1): grown by a transform, which costs no layout. */
  .fill,
  .rfill {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--st);
    transform: scaleX(var(--w, 0));
    transform-origin: left;
    transition: transform 1s linear;
  }
  /* The colour of a state, for its segment, its bar and its figure. */
  .done {
    --st: var(--soft);
  }
  .run {
    --st: var(--p-ink);
  }
  .block {
    --st: var(--wait);
  }
  .todo {
    --st: transparent;
  }
  /* A task under way that nothing measures is whole but faint: no figure is made up. */
  .unmeasured {
    opacity: 0.35;
  }

  /* The list. */
  .list {
    position: relative;
    min-height: 0;
    max-height: 232px;
    margin: 2px -8px 0;
    padding: 0;
    list-style: none;
    display: grid;
    align-content: start;
    gap: 1px;
    overflow: auto;
    overscroll-behavior: contain;
    scroll-behavior: smooth;
  }
  .list.sized {
    max-height: none;
  }
  .row {
    display: grid;
    grid-template-columns: 20px 18px minmax(0, 1fr) 56px 68px;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 0 8px;
    border-radius: 7px;
    font-size: 12.5px;
  }
  .row.focus {
    background: rgba(var(--fg-rgb), 0.045);
  }
  .n {
    color: var(--dim);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .ico {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
  }
  .check {
    display: grid;
    color: var(--add);
  }
  .ring,
  .hold {
    width: 11px;
    height: 11px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--ring);
  }
  /* Blocked: an amber ring and its middle, the agent's question waits. */
  .hold {
    background: radial-gradient(circle, var(--wait) 0 2px, transparent 2.5px);
    box-shadow: inset 0 0 0 1.5px var(--wait);
  }
  .spin {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1.75px solid color-mix(in oklch, var(--p) 30%, transparent);
    border-top-color: var(--p-ink);
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .what {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    white-space: nowrap;
  }
  .rt {
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    text-overflow: ellipsis;
  }
  .row.done .rt {
    color: var(--muted);
  }
  .row.run .rt,
  .row.block .rt {
    color: var(--text);
    font-weight: 550;
  }
  .meta {
    display: inline-flex;
    flex: 0 1 auto;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .meta:empty {
    display: none;
  }
  .subtag {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 6px;
    max-width: 260px;
    height: 20px;
    padding: 0 7px;
    border: none;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--soft);
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
  }
  .subtag :global(.icon) {
    margin-right: -3px;
  }
  .sname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .subtag:hover {
    background: rgba(var(--fg-rgb), 0.09);
    color: var(--text);
  }
  .subtag:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .model {
    flex: none;
    color: var(--muted);
  }
  .step {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .wait {
    color: var(--wait);
  }
  .after {
    color: var(--dim);
    font-size: 11.5px;
  }
  .rbar {
    height: 3px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.08);
    overflow: hidden;
  }
  .row.todo .rbar {
    visibility: hidden;
  }
  .rpct {
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    text-align: right;
    white-space: nowrap;
  }
  .row.done .rpct {
    color: var(--dim);
  }
  .row.block .rpct {
    color: var(--wait);
  }
  /* Read by screen readers only. */
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  /* Narrower, as the conversation's header (its container `conv`): the subagents' count, then the step, then the tag of the
     subagent, the bars of the rows and the count of tasks give their room to the titles. */
  @container conv (max-width: 1000px) {
    .head.tasks .subs {
      display: none;
    }
  }
  @container conv (max-width: 760px) {
    .meta .step {
      display: none;
    }
  }
  @container conv (max-width: 580px) {
    .subtag,
    .rbar,
    .count {
      display: none;
    }
    .row {
      grid-template-columns: 20px 18px minmax(0, 1fr) 68px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
      border-color: var(--p-ink);
    }
    .dot {
      animation: none;
    }
    .fill,
    .rfill,
    .chev {
      transition: none;
    }
    .list {
      scroll-behavior: auto;
    }
  }
</style>
