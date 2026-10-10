<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from './lib/ipc';
  import { app } from './lib/state.svelte';
  import { handleShortcut } from './lib/shortcuts';
  import { watchForUpdates } from './lib/updater';
  import { watchPresence } from './lib/presence';
  import { createWarmer } from './lib/warm';
  import Board from './components/board/Board.svelte';
  import ContextMenu from './components/ContextMenu.svelte';
  import Conversation from './components/Conversation.svelte';
  import ConvSearch from './components/ConvSearch.svelte';
  import DiffModal from './components/DiffModal.svelte';
  import EditorView from './components/editor/EditorView.svelte';
  import QuickOpen from './components/QuickOpen.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import CommitModal from './components/modals/CommitModal.svelte';
  import ConfirmModal from './components/modals/ConfirmModal.svelte';
  import NewProjectModal from './components/modals/NewProjectModal.svelte';
  import RenameModal from './components/modals/RenameModal.svelte';
  import SettingsModal from './components/modals/SettingsModal.svelte';
  import TestLaunchModal from './components/modals/TestLaunchModal.svelte';
  import ImportModal from './components/modals/ImportModal.svelte';
  import UpdateModal from './components/modals/UpdateModal.svelte';
  import Overview from './components/Overview.svelte';
  import RunView from './components/RunView.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Stats from './components/Stats.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import TerminalView from './components/TerminalView.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import Toasts from './components/Toasts.svelte';
  import Welcome from './components/Welcome.svelte';

  let initError = $state<string | null>(null);

  onMount(() => {
    app.init().catch((e) => (initError = String(e)));
    if (!import.meta.env.PROD) return;
    const stopChecks = watchForUpdates();
    const stopPresence = watchPresence();
    return () => {
      stopChecks();
      stopPresence();
    };
  });

  $effect(() => {
    void app.ui.view;
    void app.project?.color;
    app.applyTheme();
  });

  const warmSelected = createWarmer((id) => api.warmAgent(id).catch(() => {}));
  $effect(() => warmSelected(app.agent));

  // An agent that needed a look stops blinking once it is on screen, window in front.
  $effect(() => {
    void app.agent?.id;
    void app.term;
    void app.runCommand;
    void app.editorOn;
    void app.boardOn;
    void app.ui.view;
    app.markSeen();
  });

  function onKeydown(e: KeyboardEvent) {
    if (handleShortcut(e)) e.preventDefault();
  }
</script>

<svelte:window onkeydown={onKeydown} onfocus={() => app.markSeen()} />

<div class="root">
  <TitleBar />
  <div class="body">
    {#if initError}
      <div class="fatal">Impossible de démarrer : {initError}</div>
    {:else if !app.ready}
      <div class="fatal"></div>
    {:else if app.ui.view === 'overview'}
      <Overview />
    {:else if app.ui.view === 'stats'}
      <Stats />
    {:else if app.project}
      {@const project = app.project}
      <Sidebar {project} />
      {#if app.boardOn}
        {#key project.id}
          <Board {project} />
        {/key}
      {:else if app.editorOn}
        {#key project.id}
          <EditorView {project} />
        {/key}
      {:else if app.runCommand}
        <RunView cmd={app.runCommand} {project} />
      {:else if app.term}
        <TerminalView term={app.term} {project} />
      {:else if app.agent}
        {#key app.agent.id}
          <Conversation agent={app.agent} {project} />
        {/key}
        {#if app.split}
          <SidePanel {project} agent={app.agent} docked />
        {:else if app.filesOpen}
          <SidePanel {project} agent={app.agent} />
        {/if}
      {:else}
        <div class="noagent">
          <span>Aucun agent dans ce projet.</span>
          <button class="btn primary" onclick={() => app.newAgent()}>+ Nouvel agent</button>
        </div>
      {/if}
    {:else}
      <Welcome />
    {/if}
  </div>
  <StatusBar />
</div>

{#if app.modal?.kind === 'newProject'}
  <NewProjectModal />
{:else if app.modal?.kind === 'settings'}
  <SettingsModal
    tab={app.modal.tab}
    projectId={app.modal.projectId}
    section={app.modal.section}
    resume={app.modal.resume}
    suggest={app.modal.suggest}
  />
{:else if app.modal?.kind === 'diff'}
  <DiffModal
    projectId={app.modal.projectId}
    agentId={app.modal.agentId}
    paths={app.modal.paths}
    title={app.modal.title}
    commit={app.modal.commit}
    wholeProject={app.modal.wholeProject}
  />
{:else if app.modal?.kind === 'confirm'}
  <ConfirmModal {...app.modal} />
{:else if app.modal?.kind === 'rename'}
  <RenameModal title={app.modal.title} value={app.modal.value} onSubmit={app.modal.onSubmit} />
{:else if app.modal?.kind === 'testLaunch'}
  <TestLaunchModal agentId={app.modal.agentId} />
{:else if app.modal?.kind === 'import'}
  <ImportModal projectId={app.modal.projectId} />
{:else if app.modal?.kind === 'update' && app.update}
  <UpdateModal version={app.update.version} notes={app.update.notes} />
{:else if app.modal?.kind === 'notes'}
  <UpdateModal version={app.modal.version} notes={app.modal.notes} installed />
{:else if app.modal?.kind === 'convSearch'}
  <ConvSearch />
{:else if app.modal?.kind === 'quickOpen'}
  <QuickOpen projectId={app.modal.projectId} source={app.modal.source} />
{:else if app.modal?.kind === 'commit'}
  <CommitModal projectId={app.modal.projectId} agentId={app.modal.agentId} />
{/if}

<ContextMenu />
<Toasts />

<style>
  .root {
    height: 100vh;
    min-width: 1000px;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    overflow: hidden;
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .fatal {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--del);
  }
  .noagent {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--muted);
  }
</style>
