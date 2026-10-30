<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { applyAppearance, themeChoice } from './lib/appearance';
  import { applyPluginTheme, chosenTheme } from './lib/plugin-theme.svelte';
  import { plugins } from './lib/plugins-view.svelte';
  import { DEFAULT_FONT_SIZE, watchZoom } from './lib/display.svelte';
  import { t } from './lib/i18n';
  import { api } from './lib/ipc';
  import { app } from './lib/state.svelte';
  import { dictation } from './lib/dictation.svelte';
  import { conversation } from './lib/conversation.svelte';
  import { dictationKey, handleShortcut, holdPrint } from './lib/shortcuts';
  import { setTerminalFontSize } from './lib/terminals';
  import { watchForUpdates } from './lib/updater';
  import { watchPresence } from './lib/presence';
  import { createWarmer } from './lib/warm';
  import { offerClaudeMemory } from './lib/memory';
  import Board from './components/board/Board.svelte';
  import DocsView from './components/docs/DocsView.svelte';
  import TicketForm from './components/board/TicketForm.svelte';
  import MergedBranchesModal from './components/branches/MergedBranchesModal.svelte';
  import NewBranchModal from './components/branches/NewBranchModal.svelte';
  import ContextMenu from './components/ContextMenu.svelte';
  import Icon from './components/Icon.svelte';
  import IconSprite from './components/IconSprite.svelte';
  import Conversation from './components/Conversation.svelte';
  import ConversationOverlay from './components/ConversationOverlay.svelte';
  import ConvSearch from './components/ConvSearch.svelte';
  import DiffModal from './components/DiffModal.svelte';
  import EditorView from './components/editor/EditorView.svelte';
  import QuickOpen from './components/QuickOpen.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import SubagentView from './components/SubagentView.svelte';
  import TaskView from './components/TaskView.svelte';
  import AccountModal from './components/modals/AccountModal.svelte';
  import CommitModal from './components/modals/CommitModal.svelte';
  import PrModal from './components/modals/PrModal.svelte';
  import ConfirmModal from './components/modals/ConfirmModal.svelte';
  import MergeReposModal from './components/modals/MergeReposModal.svelte';
  import CommitReposModal from './components/modals/CommitReposModal.svelte';
  import { isMulti } from './lib/repos';
  import NewProjectModal from './components/modals/NewProjectModal.svelte';
  import RenameModal from './components/modals/RenameModal.svelte';
  import NewDiscussionModal from './components/docs/NewDiscussionModal.svelte';
  import SettingsModal from './components/modals/SettingsModal.svelte';
  import TestLaunchModal from './components/modals/TestLaunchModal.svelte';
  import ImportModal from './components/modals/ImportModal.svelte';
  import TelemetryModal from './components/modals/TelemetryModal.svelte';
  import TeamNameModal from './components/modals/TeamNameModal.svelte';
  import UpdateModal from './components/modals/UpdateModal.svelte';
  import ClaudeMemoryModal from './components/modals/ClaudeMemoryModal.svelte';
  import Overview from './components/Overview.svelte';
  import ProjectRail from './components/ProjectRail.svelte';
  import { railWidth } from './lib/rail-width.svelte';
  import { sideWidth } from './lib/side-width.svelte';
  import RunView from './components/RunView.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Stats from './components/Stats.svelte';
  import PluginsSide from './components/plugins/PluginsSide.svelte';
  import PluginsView from './components/plugins/PluginsView.svelte';
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

  // « Thème » as saved, once the settings are read (until then, the one the page kept: main.ts).
  $effect(() => {
    if (app.ready) applyAppearance(themeChoice(app.settings.theme));
  });

  // « Couleurs » (PLG-7): the plugins read once the settings are, then the theme chosen among those of the plugins on,
  // at once and as either changes; none (or its plugin off or gone), Escouade's colours.
  $effect(() => {
    if (app.ready) untrack(() => void plugins.load());
  });
  $effect(() => {
    if (plugins.loaded) applyPluginTheme(chosenTheme(app.settings.pluginTheme));
  });
  // The links `escouade://plugin/<id>` (STORE-11): the one the app was started with, then each one handed over.
  $effect(() => {
    void app.deepLinks;
    if (app.ready) untrack(() => void plugins.takeLinks());
  });

  // « Zoom de l’interface » as saved, held to what the window holds, once the settings are read (the backend zoomed the
  // window before showing it: this is for a page loaded again, and for a window resized), then the sizes of the
  // terminals and of the editor, as they change.
  $effect(() => {
    if (app.ready) return untrack(() => watchZoom());
  });
  $effect(() => {
    if (app.ready) setTerminalFontSize(app.settings.terminalFontSize || DEFAULT_FONT_SIZE);
  });
  $effect(() => {
    const size = app.settings.editorFontSize || DEFAULT_FONT_SIZE;
    document.documentElement.style.setProperty('--editor-font', `${size}px`);
  });

  // What Claude Code remembers of the project on screen, offered once to its shared memory (ESC-255), when no other
  // window is open.
  $effect(() => {
    const id = app.ui.view === 'project' ? app.ui.activeProject : null;
    if (app.ready && !app.modal && id) untrack(() => void offerClaudeMemory(id));
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
    void app.docsOn;
    void app.ui.view;
    app.markSeen();
  });

  function onKeydown(e: KeyboardEvent) {
    if (handleShortcut(e)) e.preventDefault();
  }

  // Before any field: the dictation's keys are never typed (a terminal would send them to its program).
  function onKeyCapture(e: KeyboardEvent) {
    if (dictationKey(e)) {
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    if (e.type === 'keydown') holdPrint(e);
  }
</script>

<!-- The window left while a press held dictates: its key will not be seen let go. -->
<svelte:window
  onkeydowncapture={onKeyCapture}
  onkeyupcapture={onKeyCapture}
  onkeydown={onKeydown}
  onfocus={() => {
    app.markSeen();
    void app.refreshEngines();
  }}
  onblur={() => dictation.release()}
  bind:innerWidth={railWidth.window}
/>
<!-- The window hidden (to the tray, by the app's own button or the system's): the dictation stops. -->
<svelte:document onvisibilitychange={() => document.hidden && dictation.cancel()} />

<!-- The window (ESC-158): the title bar across it, the projects' rail on the left, the status bar at the bottom; the
     sidebar on the frame and the work on its panel. -->
<div class="root" style:--rail="{railWidth.shown}px" style:--side="{sideWidth.shown}px">
  <TitleBar />
  <ProjectRail />
  <div class="body">
    {#if initError}
      <div class="work"><div class="fatal">{t('shell.app.cantStart', { error: initError })}</div></div>
    {:else if !app.ready}
      <div class="work"></div>
    {:else if app.ui.view === 'overview'}
      <div class="work"><Overview /></div>
    {:else if app.ui.view === 'stats'}
      <div class="work"><Stats /></div>
    {:else if app.ui.view === 'plugins'}
      <PluginsSide />
      <div class="work"><PluginsView /></div>
    {:else if app.project}
      {@const project = app.project}
      <Sidebar {project} />
      <div class="work">
        {#if app.boardOn}
          {#key project.id}
            <Board {project} />
          {/key}
        {:else if app.docsOn}
          {#key project.id}
            <DocsView {project} />
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
            <!-- One of its subagents' conversation (ESC-13), or the view of one of its background tasks (ESC-14), in place of its own. -->
            {#if app.subView[app.agent.id]}
              {#key app.subView[app.agent.id]}
                <SubagentView agent={app.agent} {project} call={app.subView[app.agent.id] ?? ''} />
              {/key}
            {:else if app.taskView[app.agent.id]}
              {#key app.taskView[app.agent.id]}
                <TaskView agent={app.agent} task={app.taskView[app.agent.id] ?? ''} />
              {/key}
            {:else}
              <Conversation agent={app.agent} {project} />
            {/if}
          {/key}
          {#if app.split}
            <SidePanel {project} agent={app.agent} docked />
          {:else if app.filesOpen}
            <SidePanel {project} agent={app.agent} />
          {/if}
        {:else}
          <div class="noagent">
            <span>{t('shell.app.noAgent')}</span>
            <button class="btn primary" onclick={() => app.newAgent()}><Icon name="plus" size={14} />{t('shell.app.newAgent')}</button>
            <!-- The project's open pull requests (ESC-305), in the side panel's « PR et CI » tab without an agent. -->
            <button
              class="btn ghost"
              onclick={() => {
                app.panelTab = 'hosts';
                app.filesOpen = true;
              }}><Icon name="pr" size={14} />{t('shell.app.projectPrs')}</button
            >
          </div>
          {#if app.split || app.filesOpen}
            <SidePanel {project} agent={null} />
          {/if}
        {/if}
      </div>
    {:else}
      <div class="work"><Welcome /></div>
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
    refs={app.modal.refs}
    wholeProject={app.modal.wholeProject}
    repo={app.modal.repo ?? null}
  />
{:else if app.modal?.kind === 'confirm'}
  <!-- A choice can ask the next question (save, then delete): a dialog of its own, which takes the focus. -->
  {#key app.modal}
    <ConfirmModal {...app.modal} />
  {/key}
{:else if app.modal?.kind === 'newDiscussion'}
  <NewDiscussionModal projectId={app.modal.projectId} preset={app.modal.preset} />
{:else if app.modal?.kind === 'rename'}
  <RenameModal title={app.modal.title} value={app.modal.value} onSubmit={app.modal.onSubmit} />
{:else if app.modal?.kind === 'account'}
  <AccountModal accountId={app.modal.accountId} back={app.modal.back} engine={app.modal.engine} reserveFor={app.modal.reserveFor} />
{:else if app.modal?.kind === 'testLaunch'}
  <TestLaunchModal agentId={app.modal.agentId} />
{:else if app.modal?.kind === 'import'}
  <ImportModal projectId={app.modal.projectId} />
{:else if app.modal?.kind === 'update' && app.update}
  <UpdateModal version={app.update.version} notes={app.update.notes} back={app.modal.back} />
{:else if app.modal?.kind === 'claudeMemory'}
  <ClaudeMemoryModal projectId={app.modal.projectId} offered={app.modal.offered} />
{:else if app.modal?.kind === 'telemetry'}
  <TelemetryModal />
{:else if app.modal?.kind === 'teamName'}
  <TeamNameModal projectId={app.modal.projectId} then={app.modal.then} />
{:else if app.modal?.kind === 'notes'}
  <UpdateModal version={app.modal.version} notes={app.modal.notes} installed />
{:else if app.modal?.kind === 'convSearch'}
  <ConvSearch />
{:else if app.modal?.kind === 'quickOpen'}
  <QuickOpen projectId={app.modal.projectId} source={app.modal.source} />
{:else if app.modal?.kind === 'commit' && isMulti(app.projects.find((p) => app.modal?.kind === 'commit' && p.id === app.modal.projectId))}
  <CommitReposModal projectId={app.modal.projectId} agentId={app.modal.agentId} />
{:else if app.modal?.kind === 'commit'}
  <CommitModal projectId={app.modal.projectId} agentId={app.modal.agentId} resume={app.modal.resume} />
{:else if app.modal?.kind === 'pullRequest'}
  <PrModal agentId={app.modal.agentId} />
{:else if app.modal?.kind === 'mergeRepos'}
  <MergeReposModal agentId={app.modal.agentId} />
{:else if app.modal?.kind === 'newBranch'}
  <NewBranchModal projectId={app.modal.projectId} start={app.modal.start} resume={app.modal.resume} repo={app.modal.repo ?? null} />
{:else if app.modal?.kind === 'mergedBranches'}
  <MergedBranchesModal projectId={app.modal.projectId} repo={app.modal.repo ?? null} />
{:else if app.modal?.kind === 'ticket'}
  <TicketForm projectId={app.modal.projectId} ticket={app.modal.ticket} onsubmit={app.modal.onSubmit} resume={app.modal.resume} />
{/if}

{#if conversation.overlay}
  <ConversationOverlay />
{/if}
<ContextMenu />
<Toasts />
<IconSprite />

<style>
  .root {
    position: relative;
    isolation: isolate;
    height: 100vh;
    min-width: 1000px;
    display: grid;
    grid-template-columns: var(--rail) minmax(0, 1fr);
    grid-template-rows: 44px minmax(0, 1fr) 30px;
    background: var(--frame);
    overflow: hidden;
  }
  /* The project's light (lib/theme.ts): it falls from the title bar over the whole window. */
  .root::before {
    content: '';
    position: absolute;
    inset: 0 0 auto;
    height: 520px;
    z-index: -1;
    background: radial-gradient(80% 100% at 18% 0%, color-mix(in oklch, var(--p) var(--glow), transparent), transparent 75%);
    pointer-events: none;
  }
  .body {
    grid-column: 2;
    grid-row: 2;
    display: flex;
    min-width: 0;
    min-height: 0;
    padding-right: 8px;
  }
  /* The work, on a panel in relief: its corners rounded, a line around it and a shadow under it. */
  .work {
    position: relative;
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    border-radius: 12px;
    background: var(--panel);
    box-shadow:
      0 0 0 1px var(--line),
      0 8px 24px -6px rgba(0, 0, 0, calc(0.5 * var(--shade)));
    overflow: hidden;
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
