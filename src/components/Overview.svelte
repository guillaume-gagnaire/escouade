<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import { ticketTag } from '../lib/board';
  import { fPct, fSince, fWhen } from '../lib/format';
  import { t } from '../lib/i18n';
  import Rich from '../lib/i18n/Rich.svelte';
  import { api } from '../lib/ipc';
  import { menu } from '../lib/menu.svelte';
  import { modeLabel, modelLabel, planResume } from '../lib/models';
  import { keyLabel } from '../lib/platform';
  import { contextUse, estimateHint, fSpentUsd, spent } from '../lib/spend';
  import { can, showsCost, speakerOf } from '../lib/engines';
  import { app } from '../lib/state.svelte';
  import { LIST_EVERY_MS, NEEDS, ageOf, chipsOf, ciOf, overviewPrs, rowTone, type OverviewPr, type PrNeed } from '../lib/prlist';
  import { revealHidden } from '../lib/recipe';
  import { SUMMED_UP, toolLabel } from '../lib/tools';
  import type { Agent, OverviewPrs, PendingRequest, Project } from '../lib/types';
  import EngineMark from './EngineMark.svelte';
  import Icon from './Icon.svelte';
  import StatusDot from './StatusDot.svelte';
  import { pendingFor, pendingLabel, pendingText } from '../lib/later';
  import { tip } from '../lib/tip';

  // « Vue d’ensemble » (Ctrl+Shift+A): every agent of every project in one list, the ones waiting for an answer first,
  // whose permissions are answered on the spot; a line opens its agent.

  // What a turn over says in a muted grey; a failure in red.
  const SC: Record<string, string> = {
    idle: 'var(--muted)',
    done: 'var(--muted)',
    error: 'var(--del)',
  };
  const MOVES = new Set(['ArrowDown', 'ArrowUp', 'Home', 'End']);
  /** How long the buttons of a request that just took a row's place wait: a click meant for the one before misses. */
  const HOLD_MS = 500;
  /** The lines of a permission's argument shown: one longer is answered in the conversation, where it is whole. */
  const ARG_LINES = 4;

  /** A group of lines: the agents waiting for an answer (no project: they come from all of them), or a project's. */
  interface Group {
    key: string;
    project: Project | null;
    agents: Agent[];
  }

  const live = $derived(Object.values(app.agents).filter((a) => !a.archived));
  /** The agents waiting for an answer, the longest waiting first (as Ctrl+J goes through them). */
  const waiting = $derived(live.filter((a) => a.status === 'waiting').sort((a, b) => a.lastActivity - b.lastActivity));
  /**
   * The agents waiting first, then each project's others in the order of its tabs and of its sidebar. An agent is on
   * one line only: the arrows meet it once.
   */
  const groups = $derived.by(() => {
    const out: Group[] = [{ key: 'waiting', project: null, agents: waiting }];
    for (const p of app.projects) {
      const agents = live.filter((a) => a.projectId === p.id && a.status !== 'waiting').sort((a, b) => a.createdAt - b.createdAt);
      if (agents.length) out.push({ key: `p-${p.id}`, project: p, agents });
    }
    return out;
  });
  /** The projects' lists of pull requests, as the backend last gave them (it reads them at most once a minute). */
  let lists = $state<OverviewPrs[]>([]);
  /** The pull requests that wait for the user, between the agents that wait and the projects. */
  const prs = $derived(
    overviewPrs(
      lists,
      live,
      Object.values(app.tickets),
      app.projects.map((p) => p.id),
    ),
  );
  /** Its parts, the most pressing first, those with a line only. */
  const needs = $derived(NEEDS.map((need) => ({ need, lines: prs.filter((o) => o.need === need) })).filter((n) => n.lines.length));
  /** « Merge requests » when GitLab says them all. */
  const prsTitle = $derived(t(prs.every((o) => o.word === 'MR') ? 'stats.overview.prs.titleMr' : 'stats.overview.prs.title'));
  const NEED_TITLE: Record<PrNeed, 'stats.overview.prs.fix' | 'stats.overview.prs.merge' | 'stats.overview.prs.review'> = {
    fix: 'stats.overview.prs.fix',
    merge: 'stats.overview.prs.merge',
    review: 'stats.overview.prs.review',
  };
  /** The lines in the order the arrows go through them: an agent's by its id, a pull request's by its key. */
  const order = $derived(
    groups.flatMap((g) =>
      g.project ? g.agents.map((a) => a.id) : [...g.agents.map((a) => a.id), ...needs.flatMap((n) => n.lines.map((o) => o.key))],
    ),
  );
  const sub = $derived(
    t('stats.overview.sub', {
      projects: t('common.count.projects', { count: app.projects.length }),
      agents: t('common.count.agents', { count: live.length }),
      running: live.filter((a) => a.status === 'running').length,
    }),
  );

  let root = $state<HTMLElement>();
  /** The line Tab comes back to, which the arrows move: the first one until another gets the focus. */
  let chosen = $state<string | null>(null);
  const current = $derived(chosen && order.includes(chosen) ? chosen : (order[0] ?? null));
  /** Answers on their way, by agent: its buttons wait for the backend. */
  let busy = $state<Record<string, boolean>>({});
  /** Rows whose place or request just changed, by agent: their buttons wait `HOLD_MS`. */
  let held = $state<Record<string, boolean>>({});
  const holdTimers = new Map<string, ReturnType<typeof setTimeout>>();
  /** What each waiting row shows, by agent, as last drawn: its place in the list and its request (null before the first drawing). */
  let shown: Record<string, string> | null = null;
  /** Arguments taller than their four lines, by request: shown cut, they are answered in the conversation. */
  let overflows = $state<Record<string, boolean>>({});

  // A row that changes under the pointer holds its buttons a moment: the click meant for what was there must not answer
  // what is there now. It changes when its request does (the next one after an answer, the same one asked again with
  // other words), and when it moves (a row above answered, or a new one above it: every row below slides). Not the rows
  // drawn when the view opens.
  $effect.pre(() => {
    const now = Object.fromEntries(
      waiting.map((a, i) => {
        const r = requestOf(a);
        return [a.id, JSON.stringify([i, r?.id, r?.arg, r?.description, r?.reason])];
      }),
    );
    const before = shown;
    shown = now;
    if (!before) return;
    untrack(() => {
      for (const [agentId, what] of Object.entries(now)) if (before[agentId] !== what) hold(agentId);
    });
  });
  $effect(() => () => holdTimers.forEach(clearTimeout));

  function hold(agentId: string) {
    held[agentId] = true;
    clearTimeout(holdTimers.get(agentId));
    holdTimers.set(
      agentId,
      setTimeout(() => {
        delete held[agentId];
        holdTimers.delete(agentId);
      }, HOLD_MS),
    );
  }

  /**
   * Why a permission is not answered on the spot, null when it is: only what is read whole here is. A tool whose
   * summary says part of what it asks at most (an MCP tool's, a subagent's), or says nothing, is read in the
   * conversation; so is an argument cut by the backend, longer than its four lines, or wrapping past them (`shown`: the
   * argument as drawn, its hidden characters spelled out); and so is a request Claude Code would refuse by default.
   */
  function notHere(req: PendingRequest, shown: string): string | null {
    if (req.defaultNo || !SUMMED_UP.has(req.tool) || !shown.trim()) return t('stats.overview.readInConversation');
    if (req.cut || shown.split('\n').length > ARG_LINES || overflows[req.id]) {
      return t('stats.overview.tooLong');
    }
    return null;
  }

  /**
   * Tells whether the argument of a request runs past its four lines: once drawn, and again when its size changes, when
   * its text changes (the same request asked again with other words), and when the app's fonts have loaded (the
   * fallback's letters are not as wide).
   */
  function measure(node: HTMLElement, arg: { id: string; text: string }) {
    let id = arg.id;
    // A measure asked for just before the row went is not taken.
    let gone = false;
    const check = () => {
      if (gone) return;
      const over = node.scrollHeight > node.clientHeight + 1;
      if (!!overflows[id] !== over) overflows[id] = over;
    };
    check();
    const ro = new ResizeObserver(check);
    ro.observe(node);
    document.fonts?.addEventListener('loadingdone', check);
    return {
      update(next: { id: string; text: string }) {
        if (next.id !== id) delete overflows[id];
        id = next.id;
        // Measured once the new text is in.
        tick().then(check);
      },
      destroy() {
        gone = true;
        ro.disconnect();
        document.fonts?.removeEventListener('loadingdone', check);
        delete overflows[id];
      },
    };
  }
  /** The agent whose line takes the focus back once its answered request, and the button that held the focus, are gone. */
  let refocus = $state<string | null>(null);

  // The keys work as soon as the view is open.
  onMount(() => {
    if (current) lineOf(current)?.focus();
  });

  async function readPrs() {
    try {
      lists = await api.overviewPrs();
    } catch {
      // The group shows what the tickets and the agents have; the view « Pull requests » says why.
    }
  }

  // As the view « Pull requests » reads them: at once, then every minute while it is shown and the window visible.
  onMount(() => {
    void readPrs();
    const timer = setInterval(() => {
      if (document.visibilityState === 'visible') void readPrs();
    }, LIST_EVERY_MS);
    return () => clearInterval(timer);
  });

  // Answered, an agent leaves « Attend ta réponse » for its project's group, its buttons with it: its line gets the
  // focus they held, not the page.
  $effect(() => {
    void groups;
    const id = refocus;
    if (!id) return;
    const active = document.activeElement;
    if (active && active !== document.body) return;
    refocus = null;
    focusLine(id);
  });

  function lineOf(id: string): HTMLElement | null {
    for (const row of root?.querySelectorAll<HTMLElement>('li[data-line]') ?? []) {
      if (row.dataset.line === id) return row.querySelector<HTMLElement>('.line');
    }
    return null;
  }

  function focusLine(id: string | undefined) {
    if (!id) return;
    chosen = id;
    lineOf(id)?.focus();
  }

  function onFocusIn(e: FocusEvent) {
    // Tab and the mouse move the current line too.
    const row = e.target instanceof Element ? e.target.closest<HTMLElement>('li[data-line]') : null;
    if (row?.dataset.line) chosen = row.dataset.line;
    // The focus went somewhere else on its own: it is not taken back.
    if (refocus && row?.dataset.line !== refocus) refocus = null;
  }

  /** Shows the agent in its project: its conversation, not the editor (as Ctrl+J does). */
  function open(a: Agent) {
    app.selectAgent(a.id);
    app.closeEditor(a.projectId);
  }

  /** The arrows go from line to line, from anywhere in a row (its buttons included). */
  function rowKey(e: KeyboardEvent, id: string) {
    if (!MOVES.has(e.key) || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    e.preventDefault();
    const i = order.indexOf(id);
    const last = order.length - 1;
    const to = e.key === 'ArrowDown' ? Math.min(i + 1, last) : e.key === 'ArrowUp' ? Math.max(i - 1, 0) : e.key === 'Home' ? 0 : last;
    focusLine(order[to]);
  }

  /**
   * Enter and Space open the agent of the line that has the focus (its row's buttons keep their keys). Not with a
   * modifier: Ctrl+Enter answers a request in the conversation only, here it does nothing.
   */
  function lineKey(e: KeyboardEvent, go: () => void) {
    if ((e.key !== 'Enter' && e.key !== ' ') || e.ctrlKey || e.metaKey || e.altKey || e.shiftKey) return;
    // Space would scroll the list.
    e.preventDefault();
    go();
  }

  /** Shows what a pull request's line is about: its agent, else its ticket on the board, else the project's pull requests. */
  function openPr(o: OverviewPr) {
    if (o.agent) open(o.agent);
    else if (o.ticket) app.openBoard(o.projectId);
    else app.openPrs(o.projectId);
  }

  // Escape goes back to the view « Vue d’ensemble » covers. Seen before the others (capture): a dialog or a menu open
  // closes first, alone.
  function onWindowKey(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.defaultPrevented || e.isComposing || app.modal || menu.open) return;
    app.closeOverview();
  }

  /**
   * The request the agent asked first, as its view sums it up: the backend keeps them in the order they came, so
   * another one coming does not take its place.
   */
  function requestOf(a: Agent): PendingRequest | undefined {
    return a.requests?.[0];
  }

  /** Answers the request the row shows, by its id: the one the user read, whatever came since. */
  async function decide(e: Event, a: Agent, req: PendingRequest, decision: 'allow' | 'deny') {
    // The row opens its agent on a click: its buttons only answer.
    e.stopPropagation();
    if (busy[a.id] || held[a.id]) return;
    busy[a.id] = true;
    if (root?.contains(document.activeElement)) refocus = a.id;
    await app.run(api.answerPermission(a.id, req.id, decision));
    delete busy[a.id];
  }

  function answer(e: Event, a: Agent) {
    e.stopPropagation();
    open(a);
  }
</script>

<svelte:window onkeydowncapture={onWindowKey} />

{#snippet prRow(o: OverviewPr)}
  {@const p = app.projects.find((x) => x.id === o.projectId)}
  {@const ci = ciOf(o.pr)}
  {@const tag = o.ticket?.key ?? o.agent?.name}
  <!-- As an agent's line: the keyboard opens from the line (role button), the mouse from anywhere in the row. -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <li
    class="row pr"
    class:cur={o.key === current}
    data-line={o.key}
    data-pr={o.pr.number}
    aria-labelledby={`ov-${o.key}`}
    onclick={() => openPr(o)}
    onkeydown={(e) => rowKey(e, o.key)}
  >
    <div class="line" role="button" tabindex={o.key === current ? 0 : -1} onkeydown={(e) => lineKey(e, () => openPr(o))}>
      <span class="agent">
        <span class="glyph {rowTone(o.pr)}"><Icon name={ci.icon} size={13} /></span>
        <span class="name" id={`ov-${o.key}`}>{t('stats.overview.prs.number', { word: o.word, number: o.pr.number })}</span>
        {#if p}<span class="proj"><span class="swatch" style:background={p.color}></span>{p.name}</span>{/if}
        {#if tag}<span class="tag">{tag}</span>{/if}
      </span>
      <span class="act"><span class="txt" use:tip={{ text: o.pr.title, cut: true }}>{o.pr.title}</span></span>
      <span class="chips">
        {#each chipsOf(o.pr, o.me).slice(0, 2) as c (c.key)}<span class="chip t-{c.tone}"><Icon name={c.icon} size={12} />{c.text}</span
          >{/each}
        {#if !chipsOf(o.pr, o.me).length}<span class="chip t-{ci.tone}">{ci.text}</span>{/if}
      </span>
      <span class="num since">{ageOf(o.pr, app.now)}</span>
    </div>
  </li>
{/snippet}

{#snippet row(a: Agent, asks: boolean)}
  {@const used = spent(a)}
  {@const ctx = contextUse(a)}
  {@const tag = ticketTag(app.ticketOf(a.id))}
  {@const p = asks ? app.projects.find((x) => x.id === a.projectId) : undefined}
  {@const req = asks ? requestOf(a) : undefined}
  {@const asked = a.status === 'waiting' ? requestOf(a) : undefined}
  {@const pending = pendingFor(a, app.agents, app.tickets, app.now, can(a, 'backgroundTasks'))}
  <!-- The keyboard opens the agent from its line (role button); the mouse, from anywhere in the row. -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <li
    class="row"
    class:waiting={asks}
    class:cur={a.id === current}
    data-agent={a.id}
    data-line={a.id}
    aria-labelledby={`ov-a-${a.id}`}
    onclick={() => open(a)}
    onkeydown={(e) => rowKey(e, a.id)}
  >
    <div class="line" role="button" tabindex={a.id === current ? 0 : -1} onkeydown={(e) => lineKey(e, () => open(a))}>
      <span class="agent">
        <StatusDot status={a.status} awaits={!!pending} />
        <span class="name" id={`ov-a-${a.id}`} use:tip={{ text: a.name, cut: true }}>{a.name}</span>
        <EngineMark engine={a.engine} />
        {#if p}<span class="proj"><span class="swatch" style:background={p.color}></span>{p.name}</span>{/if}
        {#if tag}<span class="tag">{tag}</span>{/if}
      </span>
      <span class="act">
        {#if a.status === 'waiting'}
          <span class="txt" style:color="var(--wait)"
            >{asked?.kind === 'permission' && asked.tool !== 'ExitPlanMode'
              ? t('stats.overview.asksPermission')
              : t('stats.overview.status.waiting')}</span
          >
        {:else if a.resumeAt}
          <span class="txt" style:color="var(--wait)">{t('stats.overview.resumes', { when: fWhen(a.resumeAt, app.now) })}</span>
        {:else if pending}
          <!-- Its turn is over, not its work (choice 25): what goes on in its tooltip. -->
          <span class="txt" style:color="var(--muted)" use:tip={pendingText(pending, app.now)}>{pendingLabel(pending, app.now)}</span>
        {:else if a.setup}
          <span class="dots" aria-hidden="true"><span></span><span></span><span></span></span><span class="txt" use:tip={a.setup}
            >{t('stats.overview.settingUp', { step: a.setup })}</span
          >
        {:else if a.status === 'running'}
          <span class="dots" aria-hidden="true"><span></span><span></span><span></span></span><span class="txt"
            >{a.activity ?? t('stats.overview.thinking')}</span
          >
        {:else}
          <span class="txt" style:color={SC[a.status]}>{t(`stats.overview.status.${a.status}`)}</span>
        {/if}
      </span>
      <span class="model"><span class="sr">{t('common.model')}{' '}</span>{modelLabel(a.model, app.models)}</span>
      <span class="num" use:tip={used.estimated ? estimateHint(used) : undefined}
        ><span class="sr">{t('common.cost')}{' '}</span>{showsCost(a) ? fSpentUsd(used) : ''}</span
      >
      <span class="num ctx" class:full={ctx?.full} use:tip={ctx?.title ?? t('stats.overview.contextUnknown')}
        ><span class="sr">{t('stats.overview.context')}{' '}</span>{ctx ? fPct(ctx.pct) : '—'}</span
      >
      <span class="num since" use:tip={t('stats.overview.lastChange', { when: fWhen(a.lastActivity, app.now) })}
        ><span class="sr">{t('stats.overview.since')}{' '}</span>{fSince(a.lastActivity, app.now)}</span
      >
    </div>
    {#if a.progressLine}
      <!-- What the agent reported through Escouade's MCP server, until its next report. -->
      <div class="said" use:tip={{ text: a.progressLine, cut: true }}>
        <span class="sr">{t('mcp.progress.label')}{' '}</span>{a.progressLine}
      </div>
    {/if}
    {#if asks}
      <!-- Drawn anew for each request: its argument is measured for itself. -->
      {#key req?.id}
        <div class="req">
          {#if req?.kind === 'permission' && req.tool !== 'ExitPlanMode'}
            <!-- What is read is what runs: an invisible or direction character is spelled out, not drawn. -->
            {@const arg = revealHidden(req.arg)}
            {@const note = notHere(req, arg)}
            <div class="what">
              <div class="cmd">
                <span class="badge">{toolLabel(req.tool)}</span>
                <span class="arg mono" use:measure={{ id: req.id, text: arg }}>{arg}</span>
              </div>
              {#if req.description}<span class="why">{revealHidden(req.description)}</span>{/if}
              {#if req.reason}<span class="why">{revealHidden(req.reason)}</span>{/if}
              {#if note}<span class="why more">{note}</span>{/if}
            </div>
            <!-- Allowed on the spot only what is read whole here. -->
            <span class="acts">
              {#if !note}
                <button class="btn primary" disabled={busy[a.id] || held[a.id]} onclick={(e) => decide(e, a, req, 'allow')}
                  >{t('stats.overview.allow')}</button
                >
                <button class="btn" disabled={busy[a.id] || held[a.id]} onclick={(e) => decide(e, a, req, 'deny')}
                  >{t('stats.overview.deny')}</button
                >
              {:else}
                <button class="btn primary" onclick={(e) => answer(e, a)}>{t('common.reply')}</button>
              {/if}
            </span>
          {:else}
            <!-- A question, a plan to read, or a request not told yet: answered in the conversation. -->
            {@const text =
              req?.kind === 'question'
                ? req.questions.map((q) => q.question).join(' · ')
                : req
                  ? t('stats.overview.proposesPlanThen', { engine: speakerOf(a.engine), mode: modeLabel(planResume(a)) })
                  : a.terminal
                    ? t('stats.overview.waitsInTerminal')
                    : t('stats.overview.waitsForAnswer', { engine: speakerOf(a.engine) })}
            <span class="ask" use:tip={{ text: text, cut: true }}>{text}</span>
            <span class="acts"><button class="btn primary" onclick={(e) => answer(e, a)}>{t('common.reply')}</button></span>
          {/if}
        </div>
      {/key}
    {/if}
  </li>
{/snippet}

<main class="overview" bind:this={root} onfocusin={onFocusIn} aria-labelledby="ov-title">
  <header class="head">
    <h1 id="ov-title">{t('common.overview')}</h1>
    <span class="sub" use:tip={{ text: sub, cut: true }}>{sub}</span>
    <span class="keys" aria-hidden="true">
      {#if order.length}
        <Rich k="stats.overview.hintChoose"
          >{#snippet keys()}<kbd><Icon name="up" size={11} /></kbd><kbd><Icon name="down" size={11} /></kbd>{/snippet}</Rich
        >
        <span class="sep">·</span>
        <Rich k="stats.overview.hintOpen">{#snippet key()}<kbd>{keyLabel('Enter')}</kbd>{/snippet}</Rich>
        <span class="sep">·</span>
      {/if}
      <Rich k="stats.overview.hintBack">{#snippet key()}<kbd>{keyLabel('Esc')}</kbd>{/snippet}</Rich>
    </span>
  </header>
  <div class="scroll">
    {#if groups.length > 1 || waiting.length || prs.length}
      <div class="list ov">
        <!-- Each value says what it is to assistive technologies (`.sr`): the headings are for the eyes. -->
        <div class="cols" aria-hidden="true">
          <span>{t('common.agent')}</span><span>{t('stats.overview.activity')}</span><span class="model">{t('common.model')}</span><span
            class="num">{t('common.cost')}</span
          ><span class="num ctx">{t('stats.overview.context')}</span><span class="num since">{t('stats.overview.since')}</span>
        </div>
        {#each groups as g (g.key)}
          {#if g.project || g.agents.length}
            <section class="group" class:waiting={!g.project} aria-labelledby={`ov-${g.key}`}>
              <h2 class="gh">
                {#if g.project}
                  <span class="swatch" style:background={g.project.color}></span>
                {:else}
                  <StatusDot status="waiting" />
                {/if}
                <span id={`ov-${g.key}`}>{g.project ? g.project.name : t('stats.overview.waitingGroup')}</span>
                <span class="n">{g.agents.length}</span>
              </h2>
              <ul>
                {#each g.agents as a (a.id)}
                  {@render row(a, !g.project)}
                {/each}
              </ul>
            </section>
          {/if}
          {#if !g.project && needs.length}
            <section class="group" aria-labelledby="ov-prs">
              <h2 class="gh">
                <Icon name="pr" size={13} />
                <span id="ov-prs">{prsTitle}</span>
                <span class="n">{prs.length}</span>
              </h2>
              {#each needs as n (n.need)}
                <h3 class="need" id={`ov-need-${n.need}`}>{t(NEED_TITLE[n.need])}<span class="n">{n.lines.length}</span></h3>
                <ul aria-labelledby={`ov-need-${n.need}`}>
                  {#each n.lines as o (o.key)}
                    {@render prRow(o)}
                  {/each}
                </ul>
              {/each}
            </section>
          {/if}
        {/each}
      </div>
    {:else}
      <div class="empty">{t('stats.overview.empty')}</div>
    {/if}
  </div>
</main>

<style>
  /* Full frame, as the preview's `#overview`: its columns fold by its own width, not the window's. */
  .overview {
    container: ov / inline-size;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    min-height: 64px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
    padding: 0 24px 0 28px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  h1 {
    flex: none;
    margin: 0;
    font-size: 17px;
    font-weight: 640;
    letter-spacing: -0.02em;
  }
  .sub {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted);
    font-size: 12.5px;
  }
  .keys {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
  }
  .keys kbd {
    min-width: 20px;
    justify-content: center;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .ov {
    --cols: minmax(0, 1.25fr) minmax(0, 2fr) 104px 74px 74px 70px;
    padding: 0 20px 24px;
  }
  /* The same columns on every line, whatever their group: values are compared down the page. */
  .cols,
  .line {
    display: grid;
    grid-template-columns: var(--cols);
    align-items: center;
    gap: 16px;
  }
  .cols {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 36px;
    padding: 0 10px;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .gh {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 18px 10px 6px;
    font-size: 12.5px;
    font-weight: 620;
  }
  .group.waiting .gh {
    color: var(--wait);
  }
  /* The parts of the group « Pull requests »: quieter than a group's heading, on the same left edge. */
  .need {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    padding: 10px 10px 4px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
  }
  .glyph {
    flex: none;
    display: inline-flex;
    color: var(--dim);
  }
  .glyph.ok {
    color: var(--run);
  }
  .glyph.wait {
    color: var(--wait);
  }
  .glyph.fail {
    color: var(--del);
  }
  /* The chips take the columns of the model, the cost and the context: a pull request has none of them. */
  .chips {
    grid-column: 3 / -2;
    display: flex;
    justify-content: flex-end;
    gap: 5px;
    min-width: 0;
    overflow: hidden;
  }
  .chip {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 7px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 11.5px;
    font-weight: 520;
    white-space: nowrap;
  }
  .chip.t-ok {
    color: var(--run);
  }
  .chip.t-wait {
    color: var(--wait);
  }
  .chip.t-fail {
    color: var(--del);
  }
  .chip.t-muted,
  .chip.t-dim {
    color: var(--muted);
  }
  .chip.t-ask {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--wait);
  }
  .swatch {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 3px;
  }
  .n {
    color: var(--dim);
    font-weight: 500;
  }
  ul {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    border-radius: 9px;
    cursor: pointer;
  }
  .row:hover {
    background: var(--hover);
  }
  /* The line the arrows are on. */
  .row.cur {
    background: rgba(var(--fg-rgb), 0.055);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  /* A request waiting, in the colours of its card in the conversation. */
  .row.waiting {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .row.waiting:hover {
    box-shadow: inset 0 0 0 1px var(--wait);
  }
  .line {
    min-height: 40px;
    padding: 0 10px;
    font-size: 12.5px;
    outline: none;
  }
  /* The focus is on the line, the ring around its whole row: a request waiting is part of it. */
  .row:has(.line:focus-visible) {
    outline: 2px solid color-mix(in oklch, var(--p) 70%, transparent);
    outline-offset: -1px;
  }
  .agent {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
    white-space: nowrap;
  }
  /* A row's dot says its state without its ring: the ring is the group's, « Attend ta réponse ». */
  .agent :global(.state-dot::after) {
    display: none;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 590;
  }
  .proj {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: 12px;
  }
  .proj .swatch {
    width: 7px;
    height: 7px;
    border-radius: 2px;
  }
  .tag {
    flex: none;
    color: var(--p-ink);
    font-size: 12px;
    font-weight: 550;
  }
  .act {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--soft);
    white-space: nowrap;
  }
  .txt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dots {
    display: inline-flex;
    flex: none;
    gap: 3px;
  }
  .dots span {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--run);
    animation: breathe 1.2s ease-in-out infinite;
  }
  .dots span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots span:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes breathe {
    50% {
      opacity: 0.45;
    }
  }
  .model {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
  }
  .num {
    text-align: right;
    white-space: nowrap;
  }
  .line .num {
    color: var(--soft);
    font-variant-numeric: tabular-nums;
  }
  .line .ctx.full {
    color: var(--wait);
  }
  /* What it reported, under its name (past its dot), quoted. */
  .said {
    margin: -6px 0 0;
    padding: 0 10px 10px 27px;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .said::before {
    content: '« ';
  }
  .said::after {
    content: ' »';
  }
  /* The request, in a well under the agent's name. */
  .req {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
    margin: 0 10px 10px 27px;
    padding: 10px 10px 10px 12px;
    border-radius: 8px;
    background: rgba(0, 0, 0, calc(0.2 * var(--shade)));
  }
  .what {
    display: grid;
    flex: 1;
    gap: 4px;
    min-width: 0;
  }
  .cmd {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
  }
  .badge {
    flex: none;
    height: 20px;
    padding: 0 6px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.08);
    font: 600 11.5px/20px var(--mono);
  }
  /*
   * The command as it runs, its lines kept, over four lines at most (one taller is answered in the conversation), in
   * the order the shell reads it: right-to-left letters must not reorder a « ; » or a « | » (as the test launch's).
   */
  .arg {
    flex: 1;
    min-width: 0;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    overflow: hidden;
    color: var(--text);
    font: 12.5px/20px var(--mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    unicode-bidi: bidi-override;
    direction: ltr;
  }
  .why {
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
    text-wrap: pretty;
  }
  .why.more {
    color: var(--wait);
  }
  .ask {
    flex: 1;
    min-width: 0;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    color: var(--text);
    font-size: 13px;
    line-height: 1.45;
    text-wrap: pretty;
  }
  /* Its buttons at the right, the argument beside them wrapping the same with « Répondre » as with the two others. */
  .acts {
    display: flex;
    flex: none;
    justify-content: flex-end;
    gap: 6px;
  }
  .acts .btn {
    height: 28px;
    font-size: 12.5px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .empty {
    padding: 90px 0 0;
    color: var(--muted);
    font-size: 13px;
    text-align: center;
  }
  /* Narrower, as the preview under 1 240 px of window: the model goes, then the context and since when. */
  @container ov (max-width: 1180px) {
    .ov {
      --cols: minmax(0, 1.2fr) minmax(0, 2fr) 74px 64px 60px;
    }
    .model {
      display: none;
    }
  }
  @container ov (max-width: 720px) {
    .ov {
      --cols: minmax(0, 1fr) minmax(0, 1fr) 64px;
    }
    .ctx,
    .since {
      display: none;
    }
    .row.pr .chips {
      grid-column: 3;
    }
    .row.pr .chip:not(:first-child) {
      display: none;
    }
    .keys {
      display: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .dots span {
      animation: none;
    }
  }
</style>
