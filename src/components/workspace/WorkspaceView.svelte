<script lang="ts">
  // The « Workspace » (spec 2026-11-25-vue-workspace-design.md, « La toile »): the agents of a project, or of every
  // project from « Vue d'ensemble » (a column each), stacked on a dotted canvas one moves and zooms: each its compass,
  // its sides in piles below 60 %. The canvas is one world, transformed by the camera; the controls float above it.
  // The project's workflows are here since ESC-689: the step on each agent's card, « Workflows » in the floating bar,
  // and a workflow's diagram enlarged over the canvas.
  import { tick, untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { menu } from '../../lib/menu.svelte';
  import { shownAgents, sidebarItems } from '../../lib/sidebar';
  import { app } from '../../lib/state.svelte';
  import type { Agent, Project } from '../../lib/types';
  import {
    altitude,
    DOT,
    fitBest,
    HUD_H,
    keepAnchor,
    MARGIN,
    nearest,
    pan,
    ZOOM_STEP,
    zoomAt,
    type Altitude,
    type Box,
    type Camera,
  } from '../../lib/workspace-camera';
  import { followEnvironments, prDetails } from '../../lib/workspace-ci.svelte';
  import { landingOf } from '../../lib/workspace-landing';
  import { frameOf, launchedOf, layoutColumns, type Level, type Side } from '../../lib/workspace-layout';
  import { followers } from '../../lib/workflow-view';
  import { workflows } from '../../lib/workflows.svelte';
  import { projectTracks } from '../../lib/workspace-workflow';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';
  import Compass from './Compass.svelte';
  import Links from './Links.svelte';
  import WorkflowBig from './WorkflowBig.svelte';
  import WorkflowsPanel from './WorkflowsPanel.svelte';

  /** None: every project's, over « Vue d'ensemble ». */
  let { project }: { project?: Project } = $props();

  /** « Masquer les inactifs » of the floating bar: the agents ready or done left out, those that work, wait or failed kept. */
  let hideIdle = $state(false);
  const inactive = (a: Agent) => a.status === 'idle' || a.status === 'done';
  // As « Vue d'ensemble »: the projects in the order of the rail, those without an agent left out; each one's agents in
  // the order of its sidebar, archived ones aside.
  const columns = $derived(
    (project ? [project] : app.projects).flatMap((p) => {
      const agents = shownAgents(
        sidebarItems(
          p,
          Object.values(app.agents).filter((a) => a.projectId === p.id && !a.archived),
        ),
      ).filter((a) => !hideIdle || !inactive(a));
      return project || agents.length ? [{ project: p, agents, launched: launchedOf(agents, app.agents, app.tickets) }] : [];
    }),
  );
  const agents = $derived(columns.flatMap((c) => c.agents));
  /** Who launched each agent launched, by its name: the dotted link's word, said in text. */
  const launchedBy = $derived(new Map(columns.flatMap((c) => c.launched.map(([from, to]) => [to, app.agents[from]?.name ?? '']))));
  const projectOf = $derived(new Map(columns.map((c) => [c.project.id, c.project])));
  /** Its pile of ended subagents unfolded, by agent; when it was unfolded last (a failure since stays out of the pile). */
  let teamOpen = $state<Record<string, boolean>>({});
  let seenAt = $state<Record<string, number>>({});
  /** The agents whose tools are all shown, their pile unfolded. */
  let toolsOpen = $state<Record<string, boolean>>({});
  /** The height measured of each card, by agent then by card (`centre`: the agent's): a card that grows moves the others. */
  let heights = $state<Record<string, Record<string, number>>>({});
  /** The sides unfolded at mid-height, by agent: they stay so while the canvas is open. */
  let unfolded = $state<Record<string, Side[]>>({});
  const frameAt = (a: Agent, level: Level) =>
    frameOf(a, {
      level,
      unfolded: unfolded[a.id],
      landing: landing.get(a.id),
      teamOpen: teamOpen[a.id],
      toolsOpen: toolsOpen[a.id],
      seenAt: seenAt[a.id],
      // Its banner open or folded as in its conversation (PlanBanner: the reader's choice, else a window of 760 px).
      planOpen: app.planOpen[a.id] ?? window.innerHeight >= 760,
      centreH: heights[a.id]?.centre || undefined,
      teamH: heights[a.id],
      testCommand: projectOf.get(a.projectId)?.board.testCommand,
      southH: heights[a.id]?.south || undefined,
      northH: heights[a.id]?.north || undefined,
    });
  function sized(agentId: string, id: string, h: number) {
    const mine = heights[agentId];
    if (h) {
      if (mine?.[id] !== h) heights[agentId] = { ...mine, [id]: h };
      return;
    }
    // 0: not laid out (yet), or gone: its size by default, and nothing kept of a card no longer there.
    if (!mine || !(id in mine)) return;
    const { [id]: _, ...rest } = mine;
    if (Object.keys(rest).length) heights[agentId] = rest;
    else delete heights[agentId];
  }
  /** What lands in each agent's zone: its pages, its remote CI, its deployments. */
  const landing = $derived(
    new Map(agents.map((a) => [a.id, landingOf(a, { pr: prDetails[a.id] ?? null, environments: app.environments[a.projectId] ?? null })])),
  );
  // The canvas follows the deployments of its projects while it is open, as the view « Environnements ».
  const followed = $derived(columns.map((c) => c.project.id).join(' '));
  $effect(() => {
    const stops = followed.split(' ').filter(Boolean).map(followEnvironments);
    return () => stops.forEach((stop) => stop());
  });
  // The workflows of its projects and their agents' tracks, as the Workflow tab read them: read again when the
  // corrections of a project change (Réglages › Projets › Kanban › Workflow).
  const fixes = $derived(JSON.stringify(columns.map((c) => [c.project.id, c.project.workflowFixes ?? []])));
  $effect(() => {
    void fixes;
    const ids = columns.map((c) => c.project.id);
    untrack(() => ids.forEach((id) => void workflows.follow(id)));
  });
  /** The agents that follow a workflow of the project, « Tickets » too: the count of « Workflows ». */
  const following = $derived(
    project
      ? followers(
          projectTracks(
            project.id,
            workflows.list(project.id),
            Object.values(app.tickets).filter((tk) => tk.projectId === project.id),
            app.agents,
            0,
          ),
        )
      : 0,
  );
  /** « Workflows » of the floating bar open. */
  let wfOpen = $state(false);
  let wfButton = $state<HTMLButtonElement>();
  function closeWorkflows() {
    wfOpen = false;
    wfButton?.focus();
  }
  /** The diagram open over this canvas: of one of its projects. */
  const diagram = $derived(app.workflowDiagram && projectOf.has(app.workflowDiagram.projectId) ? app.workflowDiagram : null);
  // Gone with the canvas: it does not come back with the next one.
  $effect(() => () => untrack(() => (app.workflowDiagram = null)));

  const laid = (level: Level) =>
    layoutColumns(
      columns.map((c) => ({ projectId: c.project.id, frames: c.agents.map((a) => frameAt(a, level)), launched: c.launched })),
      !project,
    );
  const mid = $derived(laid('mid'));
  const near = $derived(laid('near'));
  const questions = $derived(agents.reduce((n, a) => n + (a.requests?.length ?? 0), 0));

  let view: HTMLDivElement | undefined = $state();
  // At 100 % on opening, the first agent (or the first column) top left, under the floating bar.
  let cam = $state<Camera>({ x: MARGIN, y: HUD_H + MARGIN, zoom: 1 });
  const alt = $derived(altitude(cam.zoom));
  /** The compasses on screen: their sides in piles at mid-height (but an agent that waits), unfolded near. */
  const shown = $derived(alt === 'mid' ? mid : near);
  const compassAt = $derived(new Map(shown.items.map((f) => [f.agentId, f])));
  let drag: { x: number; y: number } | null = $state(null);

  const size = () => ({ w: view?.clientWidth ?? 0, h: view?.clientHeight ?? 0 });
  /** The part of the canvas under the floating bar: where the cards are fitted, and whose middle the zoom keeps. */
  const below = () => {
    const s = size();
    return { w: s.w, h: Math.max(0, s.h - HUD_H) };
  };
  const centre = () => {
    const b = below();
    return [b.w / 2, HUD_H + b.h / 2] as const;
  };

  // « Ajuster » (and 0): below the floating bar, a fitted card is never under it.
  function fitAll() {
    const c = fitBest({ mid, near }, below());
    cam = { ...c, y: c.y + HUD_H };
  }

  /** Where each agent's card is at an altitude (the card does not move on screen). */
  const anchors = (a: Altitude): (Box & { agentId: string })[] =>
    (a === 'mid' ? mid : near).items.map((f) => ({
      agentId: f.agentId,
      x: f.x + f.centre.x,
      y: f.y + f.centre.y,
      w: f.centre.w,
      h: f.centre.h,
    }));

  /** Zooms by `factor` around (px, py); across 60 %, the agent nearest the middle stays where it was on screen. */
  function zoomAround(factor: number, px: number, py: number) {
    let c = cam;
    const to = altitude(zoomAt(c, factor, px, py).zoom);
    if (to !== alt) {
      const from = nearest(anchors(alt), { ...c, y: c.y - HUD_H }, below());
      const there = from && anchors(to).find((b) => b.agentId === from.agentId);
      if (from && there) c = keepAnchor(c, from, there);
    }
    cam = zoomAt(c, factor, px, py);
  }

  const zoomBy = (factor: number) => {
    const [px, py] = centre();
    zoomAround(factor, px, py);
  };

  /** A box under the pointer that scrolls and can that way (a plan's list): the wheel is its own, as in AskInset. */
  function scrolls(e: WheelEvent): boolean {
    for (let el = e.target as HTMLElement | null; el && el !== view; el = el.parentElement) {
      if (!['auto', 'scroll'].includes(getComputedStyle(el).overflowY)) continue;
      if (e.deltaY < 0 ? el.scrollTop > 0 : e.deltaY > 0 && el.scrollTop + el.clientHeight < el.scrollHeight - 1) return true;
    }
    return false;
  }

  function wheel(e: WheelEvent) {
    if (!e.ctrlKey && !e.metaKey && scrolls(e)) return;
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      const r = view!.getBoundingClientRect();
      zoomAround(Math.exp(-e.deltaY * 0.01), e.clientX - r.left, e.clientY - r.top);
    } else cam = pan(cam, -e.deltaX, -e.deltaY);
  }

  function down(e: PointerEvent) {
    // The main button alone: a right click keeps its menu, a middle click its own use.
    // Nor on a card or in its field: one selects text there.
    if (e.button > 0 || (e.target as Element).closest('button, [role="button"], input, textarea, article, [data-side]')) return;
    drag = { x: e.clientX, y: e.clientY };
    view?.setPointerCapture?.(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    cam = pan(cam, e.clientX - drag.x, e.clientY - drag.y);
    drag = { x: e.clientX, y: e.clientY };
  }
  const up = () => (drag = null);

  /** A pile clicked at mid-height: its side unfolded, the zoom kept, the agent's card where it was on screen. */
  function unfold(agentId: string, side: Side) {
    const at = () => anchors(alt).find((b) => b.agentId === agentId);
    const from = at();
    unfolded[agentId] = [...(unfolded[agentId] ?? []), side];
    const to = at();
    if (from && to) cam = keepAnchor(cam, from, to);
    // The pile is gone with the focus: it goes to the first of what came out of it.
    void tick().then(() => {
      const items = [...(view?.querySelectorAll<HTMLElement>(`[data-agent="${agentId}"] [data-side="${side}"]`) ?? [])];
      const first = items.flatMap((el) => [el, ...el.querySelectorAll<HTMLElement>('*')]).find((el) => el.matches(FOCUSABLE)) ?? items[0];
      if (!first) return;
      if (!first.matches(FOCUSABLE)) first.setAttribute('tabindex', '-1');
      first.focus();
    });
  }

  function toggleTeam(agentId: string) {
    teamOpen[agentId] = !teamOpen[agentId];
    if (teamOpen[agentId]) seenAt[agentId] = Date.now();
  }

  /** Back to the project's view, or to the list of « Vue d'ensemble ». */
  const leave = () => (project ? app.closeWorkspace(project.id) : app.closeOverviewCanvas());

  const FOCUSABLE = 'button:not([disabled]), a[href], input, textarea, select, [tabindex]:not([tabindex="-1"])';

  // Escape leaves the canvas, unless the focus is in a field, or a menu, a dialog or a popover is open (it is theirs).
  const OVER = '[aria-modal="true"], [role="dialog"], [role="listbox"], [role="menu"]';
  const typing = (el: Element | null) =>
    !!el && (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || (el as HTMLElement).isContentEditable);
  function key(e: KeyboardEvent) {
    if (e.metaKey || e.ctrlKey || e.altKey || e.defaultPrevented) return;
    if (typing(document.activeElement) || menu.open || document.querySelector(OVER)) return;
    // A workflow's diagram over the canvas has the keys: Escape closes it, back to the canvas; then « Workflows ».
    if (diagram) {
      if (e.key === 'Escape') (e.preventDefault(), (app.workflowDiagram = null));
      return;
    }
    if (wfOpen && e.key === 'Escape') return (e.preventDefault(), closeWorkflows());
    if (e.key === 'Escape') leave();
    else if (e.key === '+' || e.key === '=') zoomBy(ZOOM_STEP);
    else if (e.key === '-') zoomBy(1 / ZOOM_STEP);
    else if (e.key === '0') fitAll();
    else return;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={key} />

<div
  class="ws"
  class:pan={!!drag}
  bind:this={view}
  role="region"
  aria-label={t('workspace.label')}
  style:background-size="{DOT * cam.zoom}px {DOT * cam.zoom}px"
  style:background-position="{cam.x}px {cam.y}px"
  onwheel={wheel}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
>
  <!-- First in the keyboard's order: one leaves without going through every card. -->
  <button class="hud exit btn" onclick={leave}>
    <Icon name="left" size={14} /><span>{t('workspace.exit')}</span>
  </button>
  <div class="pad"></div>
  {#if agents.length}
    <div class="world" style:transform="translate({cam.x}px, {cam.y}px) scale({cam.zoom})">
      {#if shown.items.length}<Links links={shown.links} w={shown.w} h={shown.h} />{/if}
      {#each shown.heads as h (h.projectId)}
        {@const p = projectOf.get(h.projectId)}
        {#if p}
          <h2 class="head" style="left:{h.x}px;top:{h.y}px;width:{h.w}px;height:{h.h}px">
            <span class="swatch" style:background={p.color}></span><span class="name">{p.name}</span>
          </h2>
        {/if}
      {/each}
      <!-- One piece per agent, in the canvas's order: the keyboard goes from one to the next as the eye does. -->
      {#each agents as a (a.id)}
        {@const f = compassAt.get(a.id)}
        {#if f}
          <Compass
            frame={f}
            agent={a}
            landing={landing.get(a.id)}
            launchedBy={launchedBy.get(a.id)}
            ontoggle={toggleTeam}
            ontoggletools={(id) => (toolsOpen[id] = !toolsOpen[id])}
            onsize={(id, h) => sized(a.id, id, h)}
            onunfold={unfold}
            zoom={cam.zoom}
          />
        {/if}
      {/each}
    </div>
  {:else}
    <p class="empty">{t(project ? 'workspace.empty' : 'workspace.emptyAll')}</p>
  {/if}

  <div class="hud bar">
    <button aria-label={t('workspace.zoomOut')} onclick={() => zoomBy(1 / ZOOM_STEP)}>−</button>
    <span class="pct">{t('workspace.zoom', { percent: Math.round(cam.zoom * 100) })}</span>
    <button aria-label={t('workspace.zoomIn')} onclick={() => zoomBy(ZOOM_STEP)}>+</button>
    <i></i>
    <button onclick={fitAll}>{t('workspace.fit')}</button>
    <i></i>
    <button aria-pressed={hideIdle} onclick={() => (hideIdle = !hideIdle)}>{t('workspace.hideIdle')}</button>
    <i></i>
    <span class="info">{t('workspace.count', { count: agents.length })}</span>
    {#if questions}<span class="info q">{t('workspace.questions', { count: questions })}</span>{/if}
  </div>
  <!-- « Workflows », a pill of its own at the right, over the panel it opens: the bar keeps its width at 1024 px. -->
  {#if project}
    <button
      class="hud wf-entry"
      bind:this={wfButton}
      aria-expanded={wfOpen}
      aria-controls={wfOpen ? 'ws-workflows' : undefined}
      use:tip={t('workspace.wf.entryTip', { project: project.name })}
      onclick={() => (wfOpen ? closeWorkflows() : (wfOpen = true))}
      ><Icon name="flow" size={14} /><span>{t('workspace.wf.entry')}</span>{#if following}<span class="n">{following}</span>{/if}</button
    >
  {/if}
  {#if project && wfOpen}<WorkflowsPanel {project} onclose={closeWorkflows} />{/if}
  {#if diagram}
    {#key diagram}<WorkflowBig projectId={diagram.projectId} workflow={diagram.workflow} agentId={diagram.agentId} />{/key}
  {/if}
</div>

<style>
  /* The work area's own surface (App.svelte, `.work`), with discreet dots. */
  .ws {
    /* The surface of its cards and piles: the sidebar's selected card (rgba of --fg-rgb at 6 %), opaque over the links. */
    --ws-card: color-mix(in oklch, var(--text) 6%, var(--panel));
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    cursor: grab;
    background-color: var(--panel);
    background-image: radial-gradient(var(--line-2) 1px, transparent 1px);
  }
  /* The background under the cards: a finger there pans the canvas; on a card, an inset or a list scrolls (a touch-action
     is the intersection of its ancestors'). */
  .pad {
    position: absolute;
    inset: 0;
    touch-action: none;
  }
  .ws.pan {
    cursor: grabbing;
    user-select: none;
  }
  .world {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }
  /* A project's column on the canvas of every project: its name over its agents, in its colour. */
  .head {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    padding-bottom: 12px;
    border-bottom: 2px solid var(--line-2);
    color: var(--text);
    font-size: 17px;
    font-weight: 650;
    white-space: nowrap;
  }
  .head .swatch {
    flex: none;
    width: 12px;
    height: 12px;
    border-radius: 4px;
  }
  .head .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--muted);
  }
  /* Above the canvas, with the shadow of the app's floating surfaces. */
  .hud {
    position: absolute;
    top: 14px;
    z-index: 1;
    /* Its own layer: WebKit otherwise paints the cards of the scaled world over it. */
    will-change: transform;
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 8px 24px -6px rgba(0, 0, 0, calc(0.5 * var(--shade)));
  }
  .exit {
    left: 16px;
    background: var(--raise);
  }
  /* A pill of the app: its buttons are those of the composer's chips. */
  .bar {
    left: 50%;
    display: flex;
    align-items: center;
    gap: 2px;
    height: 32px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--raise);
    color: var(--soft);
    font-size: 12.5px;
    white-space: nowrap;
    transform: translateX(-50%);
  }
  .bar button {
    height: 24px;
    padding: 0 10px;
    border: none;
    border-radius: 999px;
    background: none;
    color: var(--text);
    font: inherit;
    font-weight: 550;
    cursor: pointer;
  }
  .bar button:hover,
  .bar button[aria-pressed='true'] {
    background: var(--hover);
  }
  .bar i {
    width: 1px;
    height: 14px;
    margin: 0 4px;
    background: var(--line-2);
  }
  .pct {
    min-width: 44px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .info {
    padding: 0 6px;
    color: var(--muted);
  }
  .info.q {
    color: var(--wait);
  }
  /* « Workflows »: a pill as the bar, at the right; its icon in the project's ink, its count as the side panel's tabs
     had it. Pressed while its panel is open. */
  .wf-entry {
    right: 16px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 13px 0 11px;
    border: none;
    border-radius: 999px;
    background: var(--raise);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    font-weight: 550;
    white-space: nowrap;
    cursor: pointer;
  }
  .wf-entry:hover,
  .wf-entry[aria-expanded='true'] {
    background: color-mix(in oklch, var(--text) 7%, var(--raise));
  }
  .wf-entry:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 2px;
  }
  .wf-entry :global(.icon) {
    color: var(--p-ink);
  }
  .wf-entry .n {
    color: var(--muted);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
</style>
