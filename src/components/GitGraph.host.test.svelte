<script lang="ts">
  // A component of the tests only: the graph, the menu and the dialogs its entries open, as App.svelte arranges them (a
  // dialog is mounted in the same flush as the change of `app.modal`, which is what takes the focus at that moment).
  import { app } from '../lib/state.svelte';
  import type { Agent, Project } from '../lib/types';
  import ConfirmModal from './modals/ConfirmModal.svelte';
  import NewBranchModal from './branches/NewBranchModal.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import DiffModal from './DiffModal.svelte';
  import GitGraph from './GitGraph.svelte';

  let { project, agent }: { project: Project; agent: Agent | null } = $props();
</script>

<GitGraph {project} {agent} />

{#if app.modal?.kind === 'diff'}
  <DiffModal
    projectId={app.modal.projectId}
    agentId={app.modal.agentId}
    paths={app.modal.paths}
    title={app.modal.title}
    commit={app.modal.commit}
    refs={app.modal.refs}
    wholeProject={app.modal.wholeProject}
  />
{:else if app.modal?.kind === 'confirm'}
  {#key app.modal}
    <ConfirmModal {...app.modal} />
  {/key}
{:else if app.modal?.kind === 'newBranch'}
  <NewBranchModal projectId={app.modal.projectId} start={app.modal.start} resume={app.modal.resume} />
{/if}

<ContextMenu />
