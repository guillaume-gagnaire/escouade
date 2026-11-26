<script lang="ts">
  // The diagram of a project's workflow, enlarged over the « Workspace » (ESC-689): the one the Workflow tab drew
  // (WorkflowDiagram, as it was), from an agent's card with that agent's path, or from « Workflows » without one (a door
  // to start from, to choose). Its agents on their steps; « Copier en Mermaid » in its bar; Échap closes it.
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import type { WorkflowTrack } from '../../lib/types';
  import { diagramOf, entriesOf, entryOf, pathOf, progressOf, toMermaid, type DgAgent } from '../../lib/workflow-diagram';
  import { workflows } from '../../lib/workflows.svelte';
  import { projectTracks, projectWorkflows } from '../../lib/workspace-workflow';

  let { projectId, workflow, agentId = null }: { projectId: string; workflow: string; agentId?: string | null } = $props();

  // Loaded the first time it shows.
  const diagram = import('../WorkflowDiagram.svelte');

  const tickets = $derived(Object.values(app.tickets).filter((tk) => tk.projectId === projectId));
  const tracks = $derived(projectTracks(projectId, workflows.list(projectId), tickets, app.agents, app.now));
  const listed = $derived(projectWorkflows(workflows.scans[projectId] ?? null, tickets, tracks).find((l) => l.workflow.id === workflow));
  const wf = $derived(listed?.workflow ?? null);
  const inWf = $derived(listed?.tracks ?? []);
  /** The agent whose path is drawn: the one of the card, while it is in this workflow. */
  const reader = $derived(agentId ? (inWf.find((tr) => tr.agentId === agentId) ?? null) : null);
  const model = $derived(wf ? diagramOf(wf, Math.max(0, ...tickets.map((tk) => tk.maxLoops))) : null);
  const progress = $derived(wf && model ? progressOf(wf, model, reader) : null);
  const entries = $derived(wf && model ? entriesOf(wf, model) : []);
  const entry = $derived(wf && model ? entryOf(wf, model, reader) : null);
  const path = $derived(wf && model ? pathOf(wf, model, reader) : []);
  const nameOf = (id: string) => app.agents[id]?.name ?? id;

  /** The step an agent is shown on: its own, or the one it left to do something else. */
  function onStep(tr: WorkflowTrack): string | null {
    if (!tr.outside) return tr.step;
    return [...tr.passages].reverse().find((p) => p.workflow === tr.workflow && p.step)?.step ?? null;
  }
  /** Who is on each step: the agent read first. */
  const agents = $derived.by(() => {
    const out: Record<string, DgAgent[]> = {};
    for (const tr of inWf) {
      const step = onStep(tr);
      if (!step || tr.outside) continue;
      const cur = tr.agentId === reader?.agentId;
      const a = { id: tr.agentId, name: nameOf(tr.agentId), cur, wait: app.agents[tr.agentId]?.status === 'waiting' };
      (out[step] ??= [])[cur ? 'unshift' : 'push'](a);
    }
    return out;
  });

  const close = () => (app.workflowDiagram = null);

  /** « Copier en Mermaid »: the workflow without its agents, for a doc or a PR (choice 209). */
  async function copy() {
    if (!model) return;
    const code = toMermaid(model);
    try {
      await navigator.clipboard.writeText(code);
      app.toast(t('workflow.dg.copied', { count: code.split('\n').length }), 'ok');
    } catch {
      app.toast(t('workflow.dg.cantCopy'), 'error');
    }
  }
</script>

{#if model && progress}
  {#await diagram then { default: Diagram }}
    <Diagram
      {model}
      {progress}
      {entries}
      {entry}
      {path}
      reader={reader ? nameOf(reader.agentId) : ''}
      {agents}
      big
      alone
      onclose={close}
      oncopy={copy}
    />
  {/await}
{/if}
