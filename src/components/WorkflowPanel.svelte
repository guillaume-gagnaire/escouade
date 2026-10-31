<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { engineName } from '../lib/engines';
  import { fDur, fSince, fTime, fUsd } from '../lib/format';
  import { t } from '../lib/i18n';
  import Rich from '../lib/i18n/Rich.svelte';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { Agent, Project, WorkflowPassage, WorkflowTrack } from '../lib/types';
  import {
    backArcs,
    checkedOf,
    currentPhase,
    doorsOf,
    DOORS_SHOWN,
    doneSteps,
    historyRows,
    labelIn,
    nextOf,
    nextSays,
    phaseOf,
    pickWorkflow,
    pipsOf,
    plainLine,
    stepDetail,
    stepsOf,
    stripOf,
    TICKETS,
    ticketsWorkflow,
    ticketTracks,
    tracksIn,
    turnsGrid,
    withTickets,
    workflowTitle,
  } from '../lib/workflow-view';
  import { diagramOf, progressOf, toMermaid, type DgAgent } from '../lib/workflow-diagram';
  import { workflows } from '../lib/workflows.svelte';
  import Icon from './Icon.svelte';

  // The Diagram view (ESC-245), loaded the first time it shows.
  let diagramModule: Promise<typeof import('./WorkflowDiagram.svelte')> | null = null;
  const loadDiagram = () => (diagramModule ??= import('./WorkflowDiagram.svelte'));

  // The Workflow tab of the side panel (ESC-194, choices 141 to 154; `#loops*` of the preview): the rail of the
  // project's workflow, its ways back in the gutter, the agents on its steps; the agents in it, then one agent's history.
  // A loop of one prompt shows its turns and its checklist; a project with none says what was looked for.
  let { project, agent }: { project: Project; agent: Agent | null } = $props();

  const pid = $derived(project.id);
  // The project's workflows as it has them, read again when its corrections change (Réglages › Projets › Workflow).
  const fixes = $derived(JSON.stringify(project.workflowFixes ?? []));
  $effect(() => {
    const id = pid;
    void fixes;
    untrack(() => {
      void workflows.loadTracks(id);
      void workflows.loadScan(id);
    });
  });

  const read = $derived(workflows.scans[pid] ?? null);
  // « Tickets », Escouade's own (ESC-197, choice 159): beside those read, as long as the Kanban has tickets.
  const tickets = $derived(Object.values(app.tickets).filter((tk) => tk.projectId === pid));
  const ticketsWf = $derived(ticketsWorkflow(tickets));
  const scan = $derived(read ? { ...read, workflows: withTickets(read.workflows, ticketsWf) } : null);
  const tracks = $derived([
    ...workflows.list(pid).filter((tr) => app.agents[tr.agentId]),
    ...ticketTracks(pid, tickets, app.agents, app.now),
  ]);
  const wf = $derived(pickWorkflow(scan, tracks, agent?.id ?? null, workflows.chosen[pid] ?? null));
  const isTickets = $derived(wf?.id === TICKETS);
  const steps = $derived(wf ? stepsOf(wf) : []);
  const inWf = $derived(wf ? tracksIn(wf, tracks) : []);
  // Folded is the column's: the diagram shows the agents and the history under it.
  const folded = $derived(workflows.isFolded(pid) && workflows.viewOf() === 'column');
  const phase = $derived(wf ? phaseOf(wf) : null);
  const mine = $derived(inWf.find((tr) => tr.agentId === agent?.id) ?? null);
  const doors = $derived(wf ? doorsOf(wf, inWf) : []);
  let allDoors = $state(false);
  let allHistory = $state(false);
  let menuOpen = $state(false);

  /** The step an agent is shown on: its own, or the one it left to do something else. */
  function railStep(tr: WorkflowTrack): string | null {
    if (!tr.outside) return tr.step;
    return [...tr.passages].reverse().find((p) => p.workflow === tr.workflow && p.step)?.step ?? null;
  }
  const on = (step: string) => inWf.filter((tr) => railStep(tr) === step);
  const nameOf = (id: string) => app.agents[id]?.name ?? id;
  const waits = (id: string) => app.agents[id]?.status === 'waiting';
  const running = $derived(inWf.filter((tr) => !tr.outside).length);
  const gone = $derived(inWf.length - running);

  /** The agent whose history is read: the one chosen in the list, else the open agent, else the first. */
  const reader = $derived(inWf.find((tr) => tr.agentId === workflows.reader[pid]) ?? mine ?? inWf[0] ?? null);
  // The rail says where that agent is in its pass (ESC-244): the steps done ticked, its own lit, the next underlined.
  const hereStep = $derived(reader && !reader.outside ? reader.step : null);
  const doneSet = $derived(new Set(wf && reader ? doneSteps(wf, reader) : []));
  const next = $derived(wf && reader ? nextOf(wf, reader) : null);
  /** The conditions of the ways back of a next, for the tooltip of its line. */
  const nextTip = (n: ReturnType<typeof nextOf>) =>
    (n?.backs ?? [])
      .map((b) => b.condition)
      .filter(Boolean)
      .join('\n') || null;
  const readerPassages = $derived(wf && reader ? reader.passages.filter((p) => p.workflow === wf.id) : []);
  const phasePassages = $derived(wf && reader ? currentPhase(wf, reader.passages) : []);
  const shown = $derived(allHistory ? readerPassages : phasePassages);
  const costKnown = (tr: WorkflowTrack) => !tr.support.missing.includes('cost');
  const rows = $derived(wf && reader ? historyRows(wf, shown, app.now, costKnown(reader)) : []);
  const strip = $derived(wf ? stripOf(wf, shown, app.now) : []);
  const sum = (ps: WorkflowPassage[], k: 'ms' | 'cost') =>
    ps.reduce((n, p) => n + (k === 'ms' ? (p.outcome === 'running' ? Math.max(p.ms, app.now - p.at) : p.ms) : p.cost), 0);

  const behind = $derived(inWf.filter((tr) => tr.behind));

  // ---------- the ways back, drawn from the nodes' real places ----------
  let graph = $state<HTMLElement>();
  let arcs = $state<{ d: string; outer: boolean }[]>([]);
  const loopSteps = $derived(wf?.promptLoop?.steps ?? []);
  const railNames = $derived(wf?.kind === 'prompt' ? loopSteps.map((s) => String(s.n)) : steps.map((s) => s.name));
  const backs = $derived(wf ? backArcs(wf.kind === 'prompt' ? { ...wf, transitions: wf.promptLoop?.returns ?? [] } : wf, railNames) : []);

  function draw() {
    const g = graph;
    if (!g) return void (arcs = []);
    const gr = g.getBoundingClientRect();
    const mid = (el: Element) => {
      const r = el.getBoundingClientRect();
      return r.top + r.height / 2 - gr.top;
    };
    const nodes = [...g.querySelectorAll('.wf-step .wf-node')];
    const arc = (y1: number, y2: number, x: number) =>
      `M23 ${y1} H${x + 6} Q${x} ${y1} ${x} ${y1 - 6} V${y2 + 6} Q${x} ${y2} ${x + 6} ${y2} H22 M19 ${y2 - 3} L22 ${y2} L19 ${y2 + 3}`;
    const out: { d: string; outer: boolean }[] = [];
    backs.forEach(([from, to], i) => {
      if (nodes[from] && nodes[to]) out.push({ d: arc(mid(nodes[from]), mid(nodes[to]), Math.max(13 - 4 * i, 9)), outer: false });
    });
    const end = g.querySelector('.wf-end .icon');
    if (end && nodes[0]) out.push({ d: arc(mid(end), mid(nodes[0]), 5), outer: true });
    arcs = out;
  }

  $effect(() => {
    void [backs, folded, inWf, steps, graph, app.lang];
    void tick().then(draw);
  });
  $effect(() => {
    const g = graph;
    if (!g) return;
    const ro = new ResizeObserver(() => draw());
    ro.observe(g);
    return () => ro.disconnect();
  });

  function putDoor(command: string) {
    if (agent) app.composerPut = { agentId: agent.id, text: command };
  }
  function openSettings() {
    app.modal = { kind: 'settings', tab: 'projects', projectId: pid };
  }
  function choose(id: string) {
    workflows.chosen[pid] = id;
    menuOpen = false;
  }

  /** What an agent's row says of where it is. */
  function whereOf(tr: WorkflowTrack) {
    const running = [...tr.passages].reverse().find((p) => p.outcome === 'running' && p.step);
    const ticket = app.agents[tr.agentId]?.ticketId ? app.tickets[app.agents[tr.agentId].ticketId!] : null;
    const bits: { text: string; mono?: boolean }[] = [];
    for (const s of tr.subagents) bits.push({ text: s, mono: true });
    if (running && running.pass > 1) bits.push({ text: t('workflow.pass', { n: running.pass }) });
    if (tr.ticketLoop) bits.push({ text: t('workflow.ticketLoop', { n: tr.ticketLoop[0], max: tr.ticketLoop[1] }) });
    return { bits, ticket };
  }
  function outsideSince(tr: WorkflowTrack) {
    const p = [...tr.passages].reverse().find((x) => x.step === null);
    return p ? fSince(p.at, app.now) : '';
  }
  const partialSays = (tr: WorkflowTrack) =>
    t('workflow.partialSays', {
      harness: engineName(app.agents[tr.agentId]?.engine),
      what: tr.support.missing
        .map((m) => (m === 'cost' || m === 'timing' || m === 'subagents' ? t(`workflow.missing.${m}`) : m))
        .join(', '),
    });
  // ---------- the Diagram view (ESC-245, choices 196 to 212) ----------
  const view = $derived(workflows.viewOf());
  let big = $state(false);
  const model = $derived(wf ? diagramOf(wf, Math.max(0, ...tickets.map((tk) => tk.maxLoops))) : null);
  const progress = $derived(wf && model ? progressOf(wf, model, reader) : null);
  /** Who is on each step of the diagram: the agent read first. */
  const dgAgents = $derived.by(() => {
    const out: Record<string, DgAgent[]> = {};
    for (const tr of inWf) {
      const step = railStep(tr);
      if (!step || tr.outside) continue;
      const a = { id: tr.agentId, name: nameOf(tr.agentId), cur: tr.agentId === reader?.agentId, wait: waits(tr.agentId) };
      (out[step] ??= [])[a.cur ? 'unshift' : 'push'](a);
    }
    return out;
  });
  /** « Copier en Mermaid »: the workflow without its agents, for a doc or a PR (choice 209). */
  async function copyMermaid() {
    if (!model) return;
    const code = toMermaid(model);
    try {
      await navigator.clipboard.writeText(code);
      app.toast(t('workflow.dg.copied', { count: code.split('\n').length }), 'ok');
    } catch {
      app.toast(t('workflow.dg.cantCopy'), 'error');
    }
  }
  $effect(() => {
    // Back to the column, or another workflow: the enlarged view closes.
    void [view, wf?.id];
    untrack(() => (big = false));
  });

  const turnNow = $derived(reader?.turns.length ? reader.turns[reader.turns.length - 1] : null);
  const checked = $derived(wf ? checkedOf(wf, reader) : null);
</script>

{#snippet bits(list: { text: string; mono?: boolean }[])}
  {#each list as b, i (i)}{#if i > 0}{' · '}{/if}{#if b.mono}<span class="mono">{b.text}</span>{:else}{b.text}{/if}{/each}
{/snippet}

{#snippet diagram()}
  {#if model && progress}
    {#await loadDiagram() then { default: Diagram }}
      <Diagram {model} {progress} agents={dgAgents} {big} onclose={() => (big = false)} oncopy={copyMermaid} />
    {/await}
  {/if}
{/snippet}

{#snippet who(tr: WorkflowTrack)}
  <button
    class="who"
    class:off={!!tr.outside}
    class:wait={waits(tr.agentId)}
    aria-current={tr.agentId === agent?.id ? 'true' : undefined}
    use:tip={t('workflow.openAgent', { name: nameOf(tr.agentId) })}
    onclick={() => app.selectAgent(tr.agentId)}><i></i><span>{nameOf(tr.agentId)}</span></button
  >
{/snippet}

<div class="wf" data-testid="workflow">
  {#if !scan}
    <div class="caption wf-cap wf-wait">{t('workflow.reading')}</div>
  {:else if !wf && scan.off}
    <!-- « Pas de workflow pour ce projet » (ESC-309): what it does, and where it is undone. -->
    <div class="wf-empty" data-testid="workflow-off">
      <span class="ic"><Icon name="flow" size={14} /></span>
      <h3>{t('workflow.offTitle')}</h3>
      <p>{t('workflow.offBody', { project: project.name })}</p>
      <div class="btns">
        <button class="btn" onclick={openSettings}>{t('workflow.offUndo')}</button>
      </div>
    </div>
  {:else if !wf}
    <div class="wf-empty">
      <span class="ic"><Icon name="flow" size={14} /></span>
      <h3>{t('workflow.emptyTitle', { project: project.name })}</h3>
      <p>{t('workflow.emptyBody', { project: project.name })}</p>
      <div class="wf-looked">
        {#each scan.searched.slice(0, 6) as dir (dir)}
          <div><span class="mono">{dir}</span><span>{t('workflow.looked')}</span></div>
        {/each}
        {#each scan.alone.slice(0, 3) as a (a.path)}
          <div><span class="mono">{a.command}</span><span>{t('workflow.alone')}</span></div>
        {/each}
        {#each scan.untraced as u (u.name)}
          <div><span class="mono">{u.name}</span><span>{t('workflow.untraced')}</span></div>
        {/each}
      </div>
      <div class="btns">
        <button class="btn" onclick={openSettings}>{t('workflow.describe')}</button>
        <button class="btn" onclick={openSettings}><Icon name="sparkle" size={14} />{t('workflow.analyze')}</button>
      </div>
      <p>{t('workflow.tickets', { project: project.name })}</p>
    </div>
  {:else}
    <div class="wf-top">
      <div class="menu-at">
        <button
          class="select"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          aria-label={t('workflow.menu')}
          onclick={() => (menuOpen = !menuOpen)}
          onkeydown={(e) => e.key === 'Escape' && (menuOpen = false)}
          ><Icon name="flow" size={14} /><b>{workflowTitle(wf)}</b><Icon name="down" size={14} /></button
        >
        {#if menuOpen}
          <div class="wf-menu" role="menu">
            {#each scan.workflows as w (w.id)}
              <button role="menuitemradio" aria-checked={w.id === wf.id} onclick={() => choose(w.id)}>{workflowTitle(w)}</button>
            {/each}
          </div>
        {/if}
      </div>
      <span class="grow"></span>
      <div class="wf-view" role="tablist" aria-label={t('workflow.dg.viewTabs')}>
        <button
          role="tab"
          aria-selected={view === 'column'}
          aria-label={t('workflow.dg.column')}
          use:tip={t('workflow.dg.columnTip')}
          onclick={() => workflows.setView('column')}><Icon name="rail" size={14} /></button
        >
        <button
          role="tab"
          aria-selected={view === 'diagram'}
          aria-label={t('workflow.dg.diagram')}
          use:tip={t('workflow.dg.diagramTip')}
          onclick={() => workflows.setView('diagram')}><Icon name="diagram" size={14} /></button
        >
      </div>
      {#if view === 'diagram'}
        <button class="icon-btn" aria-label={t('workflow.dg.expand')} use:tip={t('workflow.dg.expand')} onclick={() => (big = true)}
          ><Icon name="expand" size={14} /></button
        >
      {/if}
      <button class="icon-btn" aria-label={t('workflow.dg.copy')} use:tip={t('workflow.dg.copyTip')} onclick={copyMermaid}
        ><Icon name="copy" size={14} /></button
      >
      {#if wf.kind === 'cycle' && view === 'column'}
        <button
          class="icon-btn"
          aria-label={folded ? t('workflow.unfold') : t('workflow.fold')}
          aria-pressed={folded}
          use:tip={folded ? t('workflow.unfold') : t('workflow.fold')}
          onclick={() => workflows.setFolded(pid, !folded)}
          ><span class="turn" class:up={!folded}><Icon name="down" size={14} /></span></button
        >
      {/if}
      <button class="icon-btn" aria-label={t('workflow.settings')} use:tip={t('workflow.settings')} onclick={openSettings}
        ><Icon name="sliders" size={14} /></button
      >
    </div>
    <div class="caption wf-cap">
      {#if wf.kind === 'prompt'}
        {t('workflow.promptCaption')}
      {:else if isTickets}
        {t('workflow.ticketsEscouade')}
      {:else}
        {t('workflow.steps', { count: steps.length })}{#if phase}{' · '}<Rich
            k="workflow.phase"
            done={String(phase.done)}
            total={String(phase.total)}>{#snippet file()}<span class="mono">{phase.file}</span>{/snippet}</Rich
          >{/if}
      {/if}{#if scan.readAt}{' · '}{t('workflow.readAt', { time: fTime(scan.readAt) })}{/if}
    </div>

    {#if behind.length}
      <div class="wf-band info" role="status">
        <Icon name="info" size={14} />
        <span
          >{t('workflow.changed', { time: fTime(scan.readAt), names: behind.map((tr) => nameOf(tr.agentId)).join(', ') })}
          <button class="link" onclick={openSettings}>{t('workflow.review')}</button></span
        >
      </div>
    {/if}

    {#if !folded && doors.length}
      <div class="wf-gates">
        <span class="lbl">{t('workflow.doors')}</span>
        {#each allDoors ? doors : doors.slice(0, DOORS_SHOWN) as d (d.command)}
          <button
            class="gate"
            disabled={!agent}
            use:tip={agent
              ? t('workflow.doorTip', { command: d.command, agent: agent.name })
              : t('workflow.doorNoAgent', { command: d.command })}
            onclick={() => putDoor(d.command)}>{d.command}</button
          >
        {/each}
        {#if doors.length > DOORS_SHOWN}
          <button class="gate more" aria-expanded={allDoors} onclick={() => (allDoors = !allDoors)}
            >{allDoors ? t('workflow.lessDoors') : t('workflow.moreDoors', { n: doors.length - DOORS_SHOWN })}</button
          >
        {/if}
      </div>
    {/if}

    {#if wf.kind === 'prompt' && wf.promptLoop}
      {@const loop = wf.promptLoop}
      {@const grid = turnsGrid(reader, loop.maxTurns)}
      <div class="wf-ring">
        <svg
          class="ring"
          viewBox="0 0 58 58"
          role="img"
          aria-label={checked ? t('workflow.checkedOf', { done: String(checked.done), total: String(checked.total) }) : ''}
        >
          <circle class="bg" cx="29" cy="29" r="25" />
          {#if checked && checked.total}
            <circle
              class="fg"
              cx="29"
              cy="29"
              r="25"
              stroke-dasharray="{(157 * checked.done) / checked.total} 157"
              transform="rotate(-90 29 29)"
            />
          {/if}
          <text x="29" y="30" text-anchor="middle">{checked ? `${checked.done}/${checked.total}` : '–'}</text>
          <text class="sub" x="29" y="42" text-anchor="middle">{t('workflow.checked')}</text>
        </svg>
        <div class="txt">
          {#if turnNow}
            <b>{turnNow.outcome === 'running' ? t('workflow.turnNow', { n: turnNow.n }) : t('workflow.turnLast', { n: turnNow.n })}</b>
          {/if}
          {#if wf.exit}<span>{t('workflow.exit', { exit: plainLine(wf.exit) })}</span>{:else if loop.maxTurns}<span
              >{t('workflow.exitTurns', { count: loop.maxTurns })}</span
            >{/if}
          {#if grid.find((u) => u.next)}<span class="nx">{t('workflow.nextTurn', { n: grid.find((u) => u.next)!.n })}</span>{/if}
        </div>
      </div>
      {#if grid.length}
        <div class="wf-turns" role="img" aria-label={t('workflow.turns')}>
          {#each grid as u (u.n)}<i class={u.cls} class:next={u.next}>{u.n}</i>{/each}
        </div>
      {/if}
      {#if view === 'diagram'}
        {@render diagram()}
      {:else if loopSteps.length}
        <div class="caption wf-cap sub">
          <Rich k="workflow.oneTurn">{#snippet file()}<span class="mono">{loop.prompt}</span>{/snippet}</Rich>
        </div>
        <div class="wf-graph" bind:this={graph}>
          <svg class="wf-edges" aria-hidden="true">
            {#each arcs as a, i (i)}<path class:outer={a.outer} d={a.d} />{/each}
          </svg>
          {#each loopSteps as s (s.n)}
            {@const back = loop.returns.find((r) => r.from === String(s.n) && r.to !== String(s.n))}
            <div class="wf-step">
              <i class="wf-node"></i>
              <div class="wf-main">
                <div class="wf-line"><span class="wf-name wrap">{s.text}</span></div>
                {#if back}
                  {@const to = loopSteps.find((x) => String(x.n) === back.to)}
                  <div class="wf-detail">
                    <span class="back"><Icon name="restart" size={12} />{t('workflow.backTo', { step: to?.text ?? back.to })}</span>
                    {#if back.max}<span>{t('workflow.upTo', { count: back.max })}</span>{/if}
                  </div>
                {/if}
              </div>
              <div class="wf-who"></div>
            </div>
          {/each}
        </div>
      {/if}
      {#if reader}
        <div class="wf-h">
          {t('workflow.historyOf', { name: nameOf(reader.agentId) })}
          <span class="n">{t('workflow.turnsOf', { count: reader.turns.length })} · {fDur(reader.ms)} · {fUsd(reader.cost)}</span>
        </div>
        <div class="wf-hist">
          {#each [...reader.turns].reverse() as u (u.n)}
            {@const cls = u.outcome === 'running' ? 'on' : u.ticked === false || u.outcome === 'failed' ? 'bad' : ''}
            <div class="ev {cls}">
              <span class="at">{fTime(u.at)}</span><i class="dot"></i>
              <span class="what"
                ><b
                  >{u.outcome === 'running'
                    ? t('workflow.turnRunning', { n: u.n })
                    : u.ticked === false
                      ? t('workflow.turnUnticked', { n: u.n })
                      : t('workflow.turnTicked', { n: u.n })}</b
                >{#if u.checked}<span>{t('workflow.checkedOf', { done: String(u.checked.done), total: String(u.checked.total) })}</span
                  >{/if}</span
              >
              <span class="num">{fDur(u.outcome === 'running' ? Math.max(u.ms, app.now - u.at) : u.ms)} · {fUsd(u.cost)}</span>
            </div>
          {/each}
        </div>
      {/if}
    {:else}
      {#if view === 'diagram'}
        {@render diagram()}
      {:else}
        <div class="wf-graph" class:folded bind:this={graph}>
          <svg class="wf-edges" aria-hidden="true">
            {#each arcs as a, i (i)}<path class:outer={a.outer} d={a.d} />{/each}
          </svg>
          {#each steps as s (s.name)}
            {@const d = stepDetail(wf, s)}
            {@const here = on(s.name)}
            {@const isDone = doneSet.has(s.name)}
            {@const back = next?.backs.find((b) => b.to === s.name)}
            <div
              class="wf-step"
              class:busy={here.length > 0}
              class:here={hereStep === s.name}
              class:done={isDone}
              class:next={next?.to === s.name}
              class:alt={!!back}
              class:guess={s.guessed}
              data-step={s.name}
            >
              <i class="wf-node"
                >{#if isDone}<Icon name="check" size={10} />{/if}</i
              >
              <div class="wf-main">
                <div class="wf-line">
                  <span class="wf-name">{labelIn(wf, s.name)}</span>{#if isDone}<span class="sr">{' '}({t('workflow.doneStep')})</span
                    >{/if}{#if next?.to === s.name}<span class="wf-next">{t('workflow.nextTag')}</span>{:else if back}<span
                      class="wf-next alt"
                      use:tip={back.condition ?? back.say ?? ''}>{t('workflow.orBackTag')}</span
                    >{/if}{#if s.command}<span class="gate" class:plain={isTickets}>{s.command}</span>{/if}
                </div>
                {#if s.guessed}
                  <div class="wf-detail">
                    <span
                      ><Rich k="workflow.guessed">{#snippet file()}<span class="fl">{s.path.split('/').pop()}</span>{/snippet}</Rich></span
                    >
                  </div>
                {:else if d.agents.length || d.writes.length || d.back.length || s.note}
                  <div class="wf-detail">
                    {#each d.agents as a, i (a)}
                      {#if i > 0}<Icon name={i === d.agents.length - 1 && d.rounds ? 'swap' : 'right'} size={12} />{/if}<span class="sa"
                        >{a}</span
                      >
                    {/each}
                    {#if d.rounds}<span>{t('workflow.upTo', { count: d.rounds })}</span>{/if}
                    {#if s.note}<span>{s.note}</span>{/if}
                    {#if d.writes.length}
                      <span><Rich k="workflow.writes">{#snippet files()}<span class="fl">{d.writes.join(', ')}</span>{/snippet}</Rich></span
                      >
                    {/if}
                    {#each d.back as b (b.to)}
                      <span class="back" use:tip={b.condition ?? ''}
                        ><Icon name="restart" size={12} />{b.say ?? t('workflow.backTo', { step: labelIn(wf, b.to) })}</span
                      >
                    {/each}
                  </div>
                {/if}
              </div>
              <div class="wf-who">
                {#each here as tr (tr.agentId)}{@render who(tr)}{/each}
              </div>
            </div>
          {/each}
          {#if wf.exit}
            <div class="wf-end" class:next={!!next?.exit}>
              <Icon name="restart" size={12} /><span
                >{#if next?.exit}<span class="wf-next">{t('workflow.nextTag')}</span>{/if}{plainLine(wf.exit)}</span
              >
            </div>
          {/if}
        </div>
      {/if}

      {#if !folded && inWf.length}
        <div class="wf-h">
          {t('workflow.agentsIn', { name: wf.name })}
          <span class="n"
            >{t('workflow.running', { count: running })}{#if gone}{' · '}{t('workflow.gone', { count: gone })}{/if}</span
          >
        </div>
        <div class="wf-list">
          {#each inWf as tr (tr.agentId)}
            {@const w = whereOf(tr)}
            {@const a = app.agents[tr.agentId]}
            {@const nx = nextOf(wf, tr)}
            <button
              class="wf-ag"
              class:off={!!tr.outside}
              class:wait={waits(tr.agentId)}
              aria-current={reader?.agentId === tr.agentId ? 'true' : undefined}
              onclick={() => (workflows.reader[pid] = tr.agentId)}
            >
              <i></i>
              <span class="nm"
                ><span>{a.name}</span>{#if w.ticket}<span class="ticket-chip">{w.ticket.key}</span>{/if}{#if tr.support.partial}<span
                    class="emark">{engineName(a.engine).split(' ')[0]}</span
                  ><span class="smark">{t('workflow.partial')}</span>{/if}</span
              >
              <span class="nums"
                >{fDur(tr.ms)}{#if costKnown(tr)}{' · '}{fUsd(tr.cost)}{/if}</span
              >
              <span class="where">
                <span class="pips"
                  >{#each pipsOf(wf, tr) as p, i (i)}<i class={p}></i>{/each}</span
                >
                <span class="t">
                  {#if tr.outside}
                    <b>{t('workflow.outside')}</b> {t('workflow.outsideFor', { since: outsideSince(tr) })} · {tr.outside}
                  {:else if tr.step}
                    <b>{labelIn(wf, tr.step)}</b>{#if w.bits.length}{' · '}{@render bits(w.bits)}{/if}
                  {:else}
                    {t('workflow.between')}
                  {/if}
                </span>
              </span>
              {#if nx}
                <span class="nx" use:tip={nextTip(nx)}>{nextSays(wf, nx)}</span>
              {/if}
              {#if tr.support.partial && tr.support.missing.length}
                <span class="wf-partial"><Icon name="info" size={12} /><span>{partialSays(tr)}</span></span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}

      <!-- « Tickets » has no history of its own: the Kanban and the agent's conversation tell it. -->
      {#if reader && !isTickets}
        <div class="wf-h">
          {t('workflow.historyOf', { name: nameOf(reader.agentId) })}
          <span class="n"
            >{#if phase}{t('workflow.phaseShort', { n: phase.done })}{' · '}{/if}{fDur(
              sum(shown, 'ms'),
            )}{#if costKnown(reader)}{' · '}{fUsd(sum(shown, 'cost'))}{/if}</span
          >
          <span class="grow"></span>
          {#if readerPassages.length > phasePassages.length}
            <button class="link" onclick={() => (allHistory = !allHistory)}
              >{allHistory ? t('workflow.seeLess') : t('workflow.seeAll')}</button
            >
          {/if}
        </div>
        {#if rows.length}
          <div class="wf-strip" role="img" aria-label={t('workflow.strip')}>
            {#each strip as s, i (i)}<i class={s.cls} style:flex={s.flex}></i>{/each}
          </div>
          <div class="wf-hist">
            {#each rows as r (r.key)}
              <div class="ev {r.cls}">
                <span class="at">{fTime(r.at)}</span>
                {#if r.backLine}<Icon name="restart" size={14} />{:else}<i class="dot"></i>{/if}
                <span class="what"
                  ><b>{r.what}</b>{#if r.sub.length}<span>{@render bits(r.sub)}</span>{/if}</span
                >
                <span class="num"
                  >{#if r.ms !== null}{fDur(r.ms)}{/if}{#if r.cost !== null && r.ms !== null}{' · '}{fUsd(r.cost)}{/if}</span
                >
              </div>
            {/each}
          </div>
        {:else}
          <p class="caption wf-cap">{t('workflow.nothingYet')}</p>
        {/if}
      {/if}
    {/if}
  {/if}
</div>

<style>
  /* The rail fits a column of 330 px: the classes and the measures of the preview's `.wf` (design/app-redesign). */
  .wf {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
    scrollbar-width: thin;
  }
  .wf > * {
    flex: none;
  }
  .mono {
    font-family: var(--mono);
    font-size: 0.92em;
    letter-spacing: -0.01em;
  }
  .caption {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The bare buttons of the tab (doors, pills, rows) reset, not the shared .btn and .icon-btn; at no weight, their own classes win. */
  :where(button:not(.btn, .icon-btn)) {
    font: inherit;
    color: inherit;
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
  }
  .wf-top {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px 4px;
  }
  /* Colonne / Diagramme (ESC-245, choice 196): the preview's `.seg.wf-view`. */
  .wf-view {
    flex: none;
    display: inline-grid;
    grid-template-columns: 1fr 1fr;
    padding: 2px;
    border-radius: 7px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .wf-view button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 22px;
    border-radius: 5px;
    color: var(--muted);
    transition:
      background-color 0.2s,
      color 0.2s;
  }
  .wf-view button:hover,
  .wf-view button[aria-selected='true'] {
    color: var(--text);
  }
  .wf-view button[aria-selected='true'] {
    background: var(--raise-2);
    box-shadow:
      0 1px 2px rgba(0, 0, 0, calc(0.4 * var(--shade, 1))),
      inset 0 0 0 1px var(--line-2);
  }
  .wf-view button:focus-visible {
    outline: 2px solid var(--focus, var(--text));
    outline-offset: 1px;
  }
  .menu-at {
    position: relative;
    min-width: 0;
  }
  .select {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    min-width: 0;
    height: 30px;
    padding: 0 9px 0 11px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-weight: 500;
  }
  .select b {
    overflow: hidden;
    color: var(--text);
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .select:hover {
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .wf-menu {
    position: absolute;
    z-index: 5;
    top: 34px;
    left: 0;
    display: grid;
    min-width: 180px;
    padding: 4px;
    border-radius: 9px;
    background: var(--panel);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 8px 24px rgba(0, 0, 0, 0.25);
  }
  .wf-menu button {
    padding: 6px 9px;
    border-radius: 6px;
    color: var(--soft);
    text-align: left;
    white-space: nowrap;
  }
  .wf-menu button:hover,
  .wf-menu button[aria-checked='true'] {
    background: var(--hover);
    color: var(--text);
  }
  .wf-top .grow {
    flex: 1;
    min-width: 0;
  }
  .wf-top .icon-btn {
    width: 24px;
    height: 24px;
    border-radius: 6px;
  }
  .turn {
    display: grid;
  }
  .turn.up {
    transform: rotate(180deg);
  }
  .wf-cap {
    padding: 0 14px 10px;
  }
  .wf-cap.sub {
    padding-top: 6px;
  }
  .wf-wait {
    padding-top: 14px;
  }
  .wf-gates {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 5px;
    padding: 0 14px 12px;
    color: var(--muted);
    font-size: 11.5px;
  }
  .wf-gates .lbl {
    margin-right: 2px;
  }
  .gate {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    height: 20px;
    padding: 0 6px;
    overflow: hidden;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font: 500 11px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  button.gate:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  button.gate:disabled {
    cursor: default;
  }
  /* « pilote auto », the door of Escouade's « Tickets »: words, not a command (the preview's `.rrow .plain`). */
  .gate.more,
  .gate.plain {
    font-family: var(--font);
    color: var(--muted);
  }
  .wf-graph {
    position: relative;
    display: grid;
    margin: 0 8px;
    padding: 2px 0 6px;
  }
  .wf-edges {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }
  .wf-edges path {
    fill: none;
    stroke: var(--line-3);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .wf-edges path.outer {
    stroke-dasharray: 3 4;
  }
  .wf-step {
    position: relative;
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr) auto;
    align-items: start;
    padding: 8px 8px 9px 0;
    border-radius: 9px;
  }
  /* The line between two steps, under the nodes: down from each node to the next one. */
  .wf-step + .wf-step::before,
  .wf-step:has(+ .wf-step)::after {
    content: '';
    position: absolute;
    left: 29.25px;
    border-left: 1.5px solid var(--line-2);
  }
  .wf-step + .wf-step::before {
    top: 0;
    height: 11px;
  }
  .wf-step:has(+ .wf-step)::after {
    top: 23px;
    bottom: 0;
  }
  .wf-step.here {
    background: rgba(var(--fg-rgb), 0.045);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .wf-node {
    position: relative;
    z-index: 1;
    width: 12px;
    height: 12px;
    margin: 3px 0 0 24px;
    border-radius: 50%;
    background: var(--panel);
    box-shadow: inset 0 0 0 1.5px var(--line-3);
  }
  .wf-step.busy .wf-node {
    box-shadow: inset 0 0 0 1.5px var(--run);
  }
  .wf-step.here .wf-node {
    background: var(--run);
    box-shadow:
      0 0 0 3px oklch(from var(--run) l c h / 0.18),
      inset 0 0 0 1.5px var(--run);
  }
  /* Where the chosen agent is in its pass (ESC-244, choices 190 to 192): done steps dimmed and ticked, the next underlined. */
  .wf-node {
    display: grid;
    place-items: center;
  }
  .wf-step.done .wf-node {
    background: rgba(var(--fg-rgb), 0.14);
    box-shadow: none;
    color: var(--soft);
  }
  .wf-step.done .wf-name {
    color: var(--muted);
    font-weight: 550;
  }
  .wf-step.done .wf-line .gate,
  .wf-step.done .wf-detail {
    opacity: 0.75;
  }
  .wf-step.next .wf-name,
  .wf-step.alt .wf-name {
    text-decoration: underline;
    text-decoration-color: oklch(from var(--run) l c h / 0.75);
    text-decoration-thickness: 1.5px;
    text-underline-offset: 4px;
  }
  .wf-step.alt .wf-name {
    text-decoration-style: dotted;
    text-decoration-color: var(--line-3);
  }
  .wf-step.next .wf-node {
    box-shadow: inset 0 0 0 1.5px oklch(from var(--run) l c h / 0.75);
  }
  .wf-next {
    display: inline-flex;
    flex: none;
    align-items: center;
    height: 18px;
    padding: 0 6px;
    border-radius: 5px;
    background: oklch(from var(--run) l c h / 0.12);
    color: var(--text);
    font-size: 11px;
    font-weight: 550;
    white-space: nowrap;
  }
  .wf-next.alt {
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }
  .wf-end.next {
    color: var(--soft);
  }
  .wf-end .wf-next {
    margin-right: 6px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .wf-step.guess .wf-node {
    background: var(--panel);
    box-shadow: none;
    outline: 1.5px dashed var(--line-3);
    outline-offset: -1.5px;
  }
  .wf-step.guess .wf-name {
    color: var(--muted);
  }
  .wf-main {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .wf-line {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .wf-name {
    color: var(--text);
    font-weight: 600;
    white-space: nowrap;
  }
  .wf-line .gate {
    min-width: 0;
  }
  /* A step of a turn is a sentence of PROMPT.md: it goes on the next line. */
  .wf-name.wrap {
    white-space: normal;
  }
  .wf-detail {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 3px 6px;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.4;
  }
  .wf-detail .sa,
  .wf-detail .fl {
    color: var(--soft);
    font: 500 11px var(--mono);
  }
  .wf-detail > :global(.icon) {
    color: var(--dim);
  }
  .wf-detail .back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--soft);
  }
  .wf-detail .back :global(.icon) {
    color: var(--dim);
  }
  .wf-who {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    padding-left: 8px;
  }
  .who {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 140px;
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 550;
  }
  .who:hover {
    color: var(--text);
  }
  .who i {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--run);
  }
  .who span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who[aria-current='true'] {
    background: oklch(from var(--run) l c h / 0.1);
    box-shadow: inset 0 0 0 1px oklch(from var(--run) l c h / 0.4);
    color: var(--text);
  }
  .who.off i {
    background: var(--idle);
  }
  .who.wait i {
    background: transparent;
    box-shadow: inset 0 0 0 1.5px var(--wait);
  }
  .wf-end {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    align-items: center;
    padding: 6px 8px 2px 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  .wf-end > :global(.icon) {
    margin-left: 24px;
    color: var(--dim);
  }
  /* Folded: one line a step, its agents beside it. */
  .wf-graph.folded .wf-detail,
  .wf-graph.folded .wf-line .gate {
    display: none;
  }
  .wf-graph.folded .wf-step {
    align-items: center;
    padding-block: 4px;
  }
  .wf-graph.folded .wf-node {
    margin-top: 0;
  }
  .wf-graph.folded .wf-who {
    flex-direction: row;
  }
  .wf-graph.folded .who {
    max-width: 116px;
  }
  .wf-graph.folded .wf-step + .wf-step::before {
    height: calc(50% - 6px);
  }
  .wf-graph.folded .wf-step:has(+ .wf-step)::after {
    top: calc(50% + 6px);
  }
  .wf-h {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 8px;
    row-gap: 2px;
    margin-top: 8px;
    padding: 14px 14px 6px;
    border-top: 1px solid var(--line);
    color: var(--soft);
    font-size: 12px;
    font-weight: 550;
  }
  .wf-h .n {
    overflow: hidden;
    color: var(--muted);
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wf-h .grow {
    flex: 1;
  }
  .link {
    color: var(--p-ink);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }
  .link:hover {
    text-decoration: underline;
  }
  .wf-list {
    display: grid;
    gap: 1px;
    padding: 0 8px;
  }
  .wf-ag {
    display: grid;
    grid-template-columns: 8px minmax(0, 1fr) auto;
    align-items: center;
    column-gap: 10px;
    row-gap: 2px;
    width: 100%;
    padding: 7px 8px;
    border-radius: 8px;
    text-align: left;
  }
  .wf-ag:hover {
    background: var(--hover);
  }
  .wf-ag[aria-current='true'] {
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .wf-ag > i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--run);
  }
  .wf-ag.off > i {
    background: var(--idle);
  }
  .wf-ag.wait > i {
    background: transparent;
    box-shadow: inset 0 0 0 1.5px var(--wait);
  }
  .wf-ag .nm {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 550;
  }
  .wf-ag .nm > span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wf-ag .nm > span:not(:first-child) {
    flex: none;
  }
  .ticket-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--p-ink);
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  .emark {
    display: inline-flex;
    align-items: center;
    height: 16px;
    padding: 0 5px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-size: 10.5px;
    font-weight: 560;
    line-height: 1;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }
  .smark {
    display: inline-flex;
    align-items: center;
    height: 19px;
    padding: 0 6px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--soft);
    font-size: 11px;
    font-weight: 550;
    white-space: nowrap;
  }
  .wf-ag .nums {
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .wf-ag .where {
    display: flex;
    grid-column: 2 / -1;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  .wf-ag .where .t {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wf-ag .where b {
    color: var(--soft);
    font-weight: 550;
  }
  .wf-ag .where .mono {
    font-size: 11px;
  }
  .wf-ag.off .nm {
    color: var(--muted);
  }
  .wf-ag .wf-partial {
    grid-column: 2 / -1;
  }
  .wf-ag .nx {
    grid-column: 2 / -1;
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pips {
    display: inline-flex;
    flex: none;
    gap: 2px;
  }
  .pips i {
    width: 10px;
    height: 3px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.12);
  }
  .pips i.done {
    background: rgba(var(--fg-rgb), 0.42);
  }
  .pips i.on {
    background: var(--run);
  }
  /* One agent's history: a strip in time, then the steps. */
  .wf-strip {
    display: flex;
    gap: 2px;
    height: 8px;
    margin: 2px 16px 10px;
  }
  .wf-strip i {
    min-width: 3px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), var(--a, 0.2));
  }
  .wf-strip i.s1 {
    --a: 0.14;
  }
  .wf-strip i.s2 {
    --a: 0.24;
  }
  .wf-strip i.s3 {
    --a: 0.36;
  }
  .wf-strip i.s4 {
    --a: 0.5;
  }
  .wf-strip i.out {
    --a: 0.07;
  }
  .wf-strip i.bad {
    background: oklch(from var(--del) l c h / 0.75);
  }
  .wf-strip i.on {
    background: var(--run);
  }
  .wf-hist {
    display: grid;
    padding: 0 8px 14px;
  }
  .ev {
    position: relative;
    display: grid;
    grid-template-columns: 40px 14px minmax(0, 1fr) auto;
    align-items: start;
    column-gap: 8px;
    padding: 6px 8px;
    border-radius: 7px;
    font-size: 12px;
  }
  .ev + .ev::before {
    content: '';
    position: absolute;
    top: -7px;
    left: 62.25px;
    height: 14px;
    border-left: 1.5px solid var(--line);
  }
  .ev .at {
    color: var(--dim);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .ev .dot {
    width: 8px;
    height: 8px;
    margin: 4px 0 0 3px;
    border-radius: 50%;
    background: rgba(var(--fg-rgb), 0.42);
  }
  .ev.on .dot {
    background: var(--run);
    box-shadow: 0 0 0 3px oklch(from var(--run) l c h / 0.18);
  }
  .ev.bad .dot {
    background: var(--del);
  }
  .ev.out .dot {
    background: rgba(var(--fg-rgb), 0.18);
  }
  .ev > :global(.icon) {
    margin-top: 1px;
    color: var(--muted);
  }
  .ev .what {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .ev .what b {
    color: var(--text);
    font-weight: 550;
  }
  .ev .what span {
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.4;
  }
  .ev .what .mono {
    font-size: 11px;
  }
  .ev .num {
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .ev.on {
    background: oklch(from var(--run) l c h / 0.06);
  }
  .ev.on .num {
    color: var(--run);
  }
  .ev.bad .what b {
    color: var(--del);
  }
  .ev.back {
    background: rgba(var(--fg-rgb), 0.03);
  }
  .ev.back .what b,
  .ev.out .what b {
    color: var(--soft);
  }
  .wf-band {
    display: grid;
    grid-template-columns: 14px minmax(0, 1fr);
    align-items: start;
    gap: 8px;
    margin: 2px 14px 10px;
    padding: 8px 10px;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 12px;
    line-height: 1.4;
  }
  .wf-band > :global(.icon) {
    margin-top: 1px;
    color: var(--muted);
  }
  .wf-partial {
    display: grid;
    grid-template-columns: 12px minmax(0, 1fr);
    gap: 6px;
    margin-top: 4px;
    padding: 6px 8px;
    border-radius: 7px;
    background: rgba(var(--fg-rgb), 0.035);
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.4;
  }
  .wf-partial > :global(.icon) {
    margin-top: 2px;
  }
  /* A loop of one prompt (Ralph): its turns, and the checklist it ticks. */
  .wf-ring {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 14px;
    margin: 4px 14px 10px;
    padding: 12px 14px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .ring {
    width: 58px;
    height: 58px;
  }
  .ring circle {
    fill: none;
    stroke-width: 4;
  }
  .ring .bg {
    stroke: rgba(var(--fg-rgb), 0.08);
  }
  .ring .fg {
    stroke: var(--p-ink);
    stroke-linecap: round;
  }
  .ring text {
    fill: var(--text);
    font: 600 13px var(--font);
    font-variant-numeric: tabular-nums;
  }
  .ring .sub {
    fill: var(--muted);
    font-size: 9.5px;
    font-weight: 500;
  }
  .wf-ring .txt {
    display: grid;
    gap: 4px;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.4;
  }
  .wf-ring .txt b {
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }
  .wf-turns {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(26px, 1fr));
    gap: 3px;
    margin: 0 14px 4px;
  }
  .wf-turns i {
    height: 18px;
    border-radius: 4px;
    background: rgba(var(--fg-rgb), 0.1);
    color: var(--muted);
    font: 500 10.5px/18px var(--font);
    font-style: normal;
    font-variant-numeric: tabular-nums;
    text-align: center;
  }
  .wf-turns i.ok {
    background: rgba(var(--fg-rgb), 0.16);
    color: var(--soft);
  }
  .wf-turns i.bad {
    background: oklch(from var(--del) l c h / 0.16);
    color: var(--del);
  }
  .wf-turns i.on {
    background: oklch(from var(--run) l c h / 0.16);
    box-shadow: inset 0 0 0 1px var(--run);
    color: var(--text);
  }
  .wf-turns i.todo {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--dim);
  }
  .wf-turns i.todo.next {
    box-shadow: inset 0 0 0 1px oklch(from var(--run) l c h / 0.75);
    color: var(--soft);
  }
  .wf-empty {
    display: grid;
    justify-items: start;
    gap: 10px;
    max-width: 440px;
    margin: 22px 18px;
  }
  .wf-empty .ic {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }
  .wf-empty h3 {
    margin: 4px 0 0;
    color: var(--text);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .wf-empty p {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .wf-empty .btns {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 4px;
  }
  .wf-looked {
    display: grid;
    gap: 2px;
    width: 100%;
    margin-top: 6px;
    padding: 8px 12px;
    border-radius: 9px;
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12px;
  }
  .wf-looked div {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 10px;
    padding: 3px 0;
    color: var(--muted);
  }
  .wf-looked div .mono {
    overflow: hidden;
    color: var(--soft);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wf-looked div span:last-child {
    font-variant-numeric: tabular-nums;
  }
  /* Narrow (the window of 1028 px): the agents of a step go under it. */
  @container git-side (max-width: 420px) {
    .who {
      max-width: 150px;
    }
    .wf-step {
      grid-template-columns: 44px minmax(0, 1fr);
    }
    .wf-who {
      grid-column: 2;
      flex-flow: row wrap;
      justify-content: flex-start;
      padding: 6px 0 0;
    }
    .wf-graph.folded .wf-step {
      grid-template-columns: 44px auto minmax(0, 1fr);
    }
    .wf-graph.folded .wf-who {
      grid-column: 3;
      justify-content: flex-end;
      padding: 0;
      flex-wrap: nowrap;
      min-width: 0;
    }
    .ev {
      grid-template-columns: 34px 14px minmax(0, 1fr) auto;
    }
    .ev + .ev::before {
      left: 56.25px;
    }
  }
  /* The open agent's node breathes; nothing moves for whoever asked for less motion. */
  @media (prefers-reduced-motion: no-preference) {
    .wf-step.here .wf-node {
      animation: wf-breathe 2.4s var(--ease) infinite;
    }
  }
  @keyframes wf-breathe {
    50% {
      box-shadow:
        0 0 0 6px oklch(from var(--run) l c h / 0.06),
        inset 0 0 0 1.5px var(--run);
    }
  }
</style>
