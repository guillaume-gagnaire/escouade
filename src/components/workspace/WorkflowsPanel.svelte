<script lang="ts">
  // « Workflows » of the canvas's floating bar (ESC-689): what the Workflow tab of the side panel offered for the whole
  // project, now in the « Workspace ». The project's workflows as it has them, « Tickets » with them; for each, its
  // agents where they are, a click opening its diagram with that agent's path; « Schéma » opening it without one (a
  // door to choose); « Copier en Mermaid »; the way to Réglages › Projets › Kanban › Workflow. A project with none says why.
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Project, Workflow, WorkflowTrack } from '../../lib/types';
  import { diagramOf, toMermaid } from '../../lib/workflow-diagram';
  import { labelIn, stepsOf, TICKETS, workflowTitle } from '../../lib/workflow-view';
  import { workflows } from '../../lib/workflows.svelte';
  import { projectTracks, projectWorkflows } from '../../lib/workspace-workflow';
  import Icon from '../Icon.svelte';
  import StatusDot from '../StatusDot.svelte';

  let { project, onclose }: { project: Project; onclose: () => void } = $props();

  const pid = $derived(project.id);
  const scan = $derived(workflows.scans[pid] ?? null);
  const tickets = $derived(Object.values(app.tickets).filter((tk) => tk.projectId === pid));
  const tracks = $derived(projectTracks(pid, workflows.list(pid), tickets, app.agents, app.now));
  const list = $derived(projectWorkflows(scan, tickets, tracks));
  const maxLoops = $derived(Math.max(0, ...tickets.map((tk) => tk.maxLoops)));

  const nameOf = (id: string) => app.agents[id]?.name ?? id;
  const open = (w: Workflow, agentId: string | null) => app.openWorkflowDiagram(pid, w.id, agentId);
  function settings() {
    app.modal = { kind: 'settings', page: 'board', projectId: pid, at: 'workflow' };
  }

  /** What its line says of a workflow: its steps, a prompt run over and over, or Escouade's loop of the tickets. */
  const caption = (w: Workflow) =>
    w.id === TICKETS
      ? t('workflow.ticketsEscouade')
      : w.kind === 'prompt'
        ? t('workflow.promptCaption')
        : t('workflow.steps', { count: stepsOf(w).length });

  /** Where an agent is in it: its step and its stay, its ticket's loop, or out of it. */
  function whereOf(w: Workflow, tr: WorkflowTrack): string {
    if (tr.outside) return t('workspace.wf.outside');
    if (!tr.step) return t('workspace.wf.between');
    const bits = [w.id === TICKETS ? tr.step : labelIn(w, tr.step)];
    const now = [...tr.passages].reverse().find((p) => p.workflow === w.id && p.step === tr.step);
    if (now && now.pass > 1) bits.push(t('workflow.pass', { n: now.pass }));
    if (tr.ticketLoop) bits.push(t('workflow.ticketLoop', { n: tr.ticketLoop[0], max: tr.ticketLoop[1] }));
    return bits.join(' · ');
  }

  /** « Copier en Mermaid »: the workflow without its agents, for a doc or a PR (choice 209). */
  async function copy(w: Workflow) {
    const code = toMermaid(diagramOf(w, maxLoops));
    try {
      await navigator.clipboard.writeText(code);
      app.toast(t('workflow.dg.copied', { count: code.split('\n').length }), 'ok');
    } catch {
      app.toast(t('workflow.dg.cantCopy'), 'error');
    }
  }

  let head = $state<HTMLElement>();
  $effect(() => head?.focus());
  function key(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    onclose();
  }
</script>

<!-- Its own wheel and pointer: the canvas under it neither scrolls nor pans. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="wfs"
  id="ws-workflows"
  aria-labelledby="ws-workflows-title"
  data-testid="workspace-workflows"
  onkeydown={key}
  onwheel={(e) => e.stopPropagation()}
  onpointerdown={(e) => e.stopPropagation()}
>
  <header>
    <h2 id="ws-workflows-title" tabindex="-1" bind:this={head}>
      <Icon name="flow" size={14} /><span>{t('workspace.wf.title')}</span>
    </h2>
    <span class="grow"></span>
    <button class="icon-btn" aria-label={t('workflow.settings')} use:tip={t('workflow.settings')} onclick={settings}
      ><Icon name="sliders" size={14} /></button
    >
    <button class="icon-btn" aria-label={t('workspace.wf.close')} use:tip={t('workspace.wf.close')} onclick={onclose}
      ><Icon name="close" size={14} /></button
    >
  </header>

  {#if !scan}
    <p class="note">{t('workflow.reading')}</p>
  {:else if !list.length && scan.off}
    <!-- « Pas de workflow pour ce projet » (ESC-309): what it does, and where it is undone. -->
    <div class="none" data-testid="workflow-off">
      <h3>{t('workflow.offTitle')}</h3>
      <p>{t('workflow.offBody', { project: project.name })}</p>
      <button class="btn" onclick={settings}>{t('workflow.offUndo')}</button>
    </div>
  {:else if !list.length}
    <div class="none">
      <h3>{t('workflow.emptyTitle', { project: project.name })}</h3>
      <p>{t('workflow.emptyBody', { project: project.name })}</p>
      {#if scan.searched.length || scan.alone.length || scan.untraced.length}
        <ul class="looked">
          {#each scan.searched.slice(0, 6) as dir (dir)}
            <li><span class="mono">{dir}</span><span>{t('workflow.looked')}</span></li>
          {/each}
          {#each scan.alone.slice(0, 3) as a (a.path)}
            <li><span class="mono">{a.command}</span><span>{t('workflow.alone')}</span></li>
          {/each}
          {#each scan.untraced as u (u.name)}
            <li><span class="mono">{u.name}</span><span>{t('workflow.untraced')}</span></li>
          {/each}
        </ul>
      {/if}
      <div class="btns">
        <button class="btn" onclick={settings}>{t('workflow.describe')}</button>
        <button class="btn" onclick={settings}><Icon name="sparkle" size={14} />{t('workflow.analyze')}</button>
      </div>
      <p>{t('workflow.tickets', { project: project.name })}</p>
    </div>
  {:else}
    <ul class="list">
      {#each list as l (l.workflow.id)}
        {@const w = l.workflow}
        <li class="wf" data-workflow={w.id}>
          <div class="wf-head">
            <div class="wf-name">
              <b>{workflowTitle(w)}</b>
              <span class="cap"
                >{caption(w)} · {l.tracks.length ? t('workspace.wf.following', { count: l.tracks.length }) : t('workspace.wf.nobody')}</span
              >
            </div>
            <button class="icon-btn" aria-label={t('workflow.dg.copy')} use:tip={t('workflow.dg.copyTip')} onclick={() => copy(w)}
              ><Icon name="copy" size={14} /></button
            >
            <button class="btn sm" use:tip={t('workspace.wf.diagramTip', { name: workflowTitle(w) })} onclick={() => open(w, null)}
              ><Icon name="diagram" size={14} />{t('workspace.wf.diagram')}</button
            >
          </div>
          {#if l.tracks.length}
            <ul class="who">
              {#each l.tracks as tr (tr.agentId)}
                {@const a = app.agents[tr.agentId]}
                <li>
                  <button
                    class:off={!!tr.outside}
                    use:tip={t('workspace.wf.open', { workflow: workflowTitle(w), name: nameOf(tr.agentId) })}
                    onclick={() => open(w, tr.agentId)}
                  >
                    {#if a}<StatusDot status={a.status} />{/if}
                    <span class="nm">{nameOf(tr.agentId)}</span>
                    <span class="at">{whereOf(w, tr)}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  /* A floating surface of the canvas, as its bar: raised, ringed, with the shadow of the app's floating surfaces. */
  .wfs {
    position: absolute;
    top: 58px;
    right: 16px;
    z-index: 2;
    display: flex;
    flex-direction: column;
    width: min(360px, calc(100% - 32px));
    max-height: calc(100% - 74px);
    border-radius: 12px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -8px rgba(0, 0, 0, calc(0.55 * var(--shade)));
    color: var(--text);
    font-size: 13px;
    cursor: default;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 8px 8px 14px;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    outline: none;
  }
  h2 :global(.icon) {
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  .list {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 4px 0 6px;
    overflow-y: auto;
    list-style: none;
  }
  .wf + .wf {
    border-top: 1px solid var(--line);
  }
  .wf-head {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 10px 10px 6px 14px;
  }
  .wf-name {
    display: grid;
    flex: 1;
    min-width: 0;
    gap: 1px;
  }
  .wf-name b {
    overflow: hidden;
    font-weight: 590;
    letter-spacing: -0.01em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cap {
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn.sm {
    height: 26px;
    padding: 0 9px;
    gap: 6px;
    font-size: 12px;
  }
  .who {
    margin: 0 0 6px;
    padding: 0 6px;
    list-style: none;
  }
  .who button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .who button:hover {
    background: var(--hover);
  }
  .who button:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .who .nm {
    flex: none;
    max-width: 50%;
    overflow: hidden;
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who .at {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-align: right;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who .off .nm {
    color: var(--soft);
    font-weight: 500;
  }
  .note,
  .none {
    margin: 0;
    padding: 14px;
    overflow-y: auto;
  }
  .note {
    color: var(--muted);
  }
  .none h3 {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 600;
  }
  .none p {
    margin: 0 0 10px;
    color: var(--soft);
    line-height: 1.5;
  }
  .looked {
    display: grid;
    gap: 3px;
    margin: 0 0 12px;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }
  .looked li {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    color: var(--muted);
  }
  .looked .mono {
    overflow: hidden;
    color: var(--soft);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btns {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 12px;
  }
</style>
