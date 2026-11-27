<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { applyAppearance, themeChoice } from './lib/appearance';
  import { applyPluginTheme, chosenTheme } from './lib/plugin-theme.svelte';
  import { plugins } from './lib/plugins-view.svelte';
  import { DEFAULT_FONT_SIZE, watchZoom } from './lib/display.svelte';
  import { t } from './lib/i18n';
  import { api } from './lib/ipc';
  import { app } from './lib/state.svelte';
  import { showHostEvent } from './lib/server-moves.svelte';
  import { onHostEvent } from './lib/transport';
  import { dictation } from './lib/dictation.svelte';
  import { conversation } from './lib/conversation.svelte';
  import { dictationKey, handleShortcut, holdPrint, inVscode } from './lib/shortcuts';
  import { setTerminalFontSize } from './lib/terminals';
  import { watchForUpdates } from './lib/updater';
  import { watchPresence } from './lib/presence';
  import { createWarmer } from './lib/warm';
  import { heatClock } from './lib/heat.svelte';
  import { offerClaudeMemory } from './lib/memory';
  import { closePage, devpages } from './lib/devpage.svelte';
  import { hasTab } from './lib/code-host';
  import { WATCH_TICK_MS } from './lib/prnew';
  import Board from './components/board/Board.svelte';
  import PrsView from './components/prs/PrsView.svelte';
  import WorkspaceView from './components/workspace/WorkspaceView.svelte';
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
  import VscodeView from './components/editor/VscodeView.svelte';
  import QuickOpen from './components/QuickOpen.svelte';
  import SidePanel from './components/SidePanel.svelte';
  import SubagentView from './components/SubagentView.svelte';
  import TaskView from './components/TaskView.svelte';
  import AccountModal from './components/modals/AccountModal.svelte';
  import CommitModal from './components/modals/CommitModal.svelte';
  import PrModal from './components/modals/PrModal.svelte';
  import CiModal from './components/modals/CiModal.svelte';
  import ConfirmModal from './components/modals/ConfirmModal.svelte';
  import DeleteFolderModal from './components/modals/DeleteFolderModal.svelte';
  import DeleteAgentModal from './components/modals/DeleteAgentModal.svelte';
  import HarnessConsentModal from './components/plugins/HarnessConsentModal.svelte';
  import HarnessLaunchModal from './components/plugins/HarnessLaunchModal.svelte';
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
  import Onboarding from './components/onboarding/Onboarding.svelte';
  import FeaturesTourModal from './components/features/FeaturesTourModal.svelte';
  import { loadTour, tour } from './lib/features-tour.svelte';
  import { onboarding } from './lib/onboarding.svelte';
  import DiscoverMap from './components/discover/DiscoverMap.svelte';
  import DiscoverTip from './components/discover/DiscoverTip.svelte';
  import NewBand from './components/discover/NewBand.svelte';
  import { keyPressed, load as loadDiscover } from './lib/discover.svelte';
  import FeatureOffer from './components/FeatureOffer.svelte';
  import { start as startOffers, stop as stopOffers } from './lib/offers.svelte';
  import TeamNameModal from './components/modals/TeamNameModal.svelte';
  import TeamModal from './components/team/TeamModal.svelte';
  import UpdateModal from './components/modals/UpdateModal.svelte';
  import ClaudeMemoryModal from './components/modals/ClaudeMemoryModal.svelte';
  import Overview from './components/Overview.svelte';
  import InboxQueue from './components/inbox/InboxQueue.svelte';
  import InboxScreen from './components/inbox/InboxScreen.svelte';
  import { inbox } from './lib/inbox/inbox.svelte';
  import { inboxViews } from './lib/inbox/views.svelte';
  import ProjectRail from './components/ProjectRail.svelte';
  import { railWidth } from './lib/rail-width.svelte';
  import { sideWidth } from './lib/side-width.svelte';
  import RunView from './components/RunView.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Stats from './components/Stats.svelte';
  import PluginsSide from './components/plugins/PluginsSide.svelte';
  import PluginsView from './components/plugins/PluginsView.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import OverseerScreen from './components/overseer/OverseerScreen.svelte';
  import { overseer } from './lib/overseer.svelte';
  import TerminalView from './components/TerminalView.svelte';
  import DevPage from './components/DevPage.svelte';
  import AgentPageView from './components/AgentPageView.svelte';
  import { isAgentPage } from './lib/devpage.svelte';
  import { loadRows } from './lib/agentpages.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import TlsBanner from './components/TlsBanner.svelte';
  import RecoveryBanner from './components/RecoveryBanner.svelte';
  import TeamSettingsBand from './components/team/TeamSettingsBand.svelte';
  import BreakerBand from './components/team/BreakerBand.svelte';
  import TeamHintBand from './components/team/TeamHintBand.svelte';
  import DemoBand from './components/onboarding/DemoBand.svelte';
  import TeamPourModal from './components/board/TeamPourModal.svelte';
  import TeamHandoffModal from './components/board/TeamHandoffModal.svelte';
  import TeamHandoffTakeModal from './components/board/TeamHandoffTakeModal.svelte';
  import ConnectionBand from './components/ConnectionBand.svelte';
  import ConnectServerModal from './components/servers/ConnectServerModal.svelte';
  import ServerScreen from './components/servers/ServerScreen.svelte';
  import { connection } from './lib/connection.svelte';
  import { loadServers, servers } from './lib/servers.svelte';
  import { goToSpace, startSpaces, syncSpaces, takeOpening } from './lib/spaces.svelte';
  import Toasts from './components/Toasts.svelte';
  import Welcome from './components/Welcome.svelte';
  import PaneDrop from './components/panes/PaneDrop.svelte';
  import PaneGrid from './components/panes/PaneGrid.svelte';
  import { itemKey } from './lib/panes';

  let initError = $state<string | null>(null);

  // The window on a server that stopped answering it, or that it does not reach yet (ESC-537): its own screen, in place
  // of the space.
  const serverScreen = $derived(
    connection.server !== null && (connection.state === 'refused' || (connection.state === 'connecting' && connection.tries > 0)),
  );

  onMount(() => {
    app.init().catch((e) => (initError = String(e)));
    // The agents' pages whose row the backend kept for this run (a reload of the window).
    void loadRows();
    // The servers this computer knows (ESC-537): the machine is said once one is.
    void loadServers();
    // The other spaces (ESC-539): a connection to each, read every few seconds; a notification of one brings it.
    const inTauri = '__TAURI_INTERNALS__' in window;
    const stopSpaces = inTauri ? startSpaces() : () => {};
    let unlisten: (() => void) | null = null;
    if (inTauri) {
      void import('@tauri-apps/api/event').then(({ listen }) =>
        listen<{ space: string; agentId?: string; projectId?: string }>('space-focus', (e) => {
          void goToSpace(e.payload.space, { agentId: e.payload.agentId ?? undefined, projectId: e.payload.projectId ?? undefined });
        }).then((u) => (unlisten = u)),
      );
    }
    // On a server, what its host publishes (a notification, a chime) is shown here, named by its space (ESC-538).
    const stopHost = onHostEvent(showHostEvent);
    if (!import.meta.env.PROD)
      return () => {
        stopHost();
        stopSpaces();
        unlisten?.();
      };
    const stopChecks = watchForUpdates();
    const stopPresence = watchPresence();
    return () => {
      stopHost();
      stopChecks();
      stopPresence();
      stopSpaces();
      unlisten?.();
    };
  });

  // A server added or removed (ESC-539): the connections to the spaces follow.
  $effect(() => {
    void servers.view?.servers.map((s) => s.id).join();
    untrack(() => void syncSpaces());
  });

  // Loaded again on a space to show an agent there (« Ouvrir » of « Partout », a notification): shown once ready.
  $effect(() => {
    if (app.ready) untrack(takeOpening);
  });

  // A feature switched off while its view is shown (F-1): the statistics leave for the project, the Overseer's screen
  // closes.
  $effect(() => {
    if (app.ui.view === 'stats' && !app.feature('stats')) untrack(() => app.closeStats());
    if (overseer.open && !app.feature('overseer')) overseer.open = false;
  });

  // A project's launch switched off (ESC-658): its pages close (they open again with a new launch, once it is on).
  $effect(() => {
    // Each browser says its project (ESC-657): an agent's launch, a launch command's, or one opened alone.
    for (const [id, page] of Object.entries(devpages.pages)) if (!app.partOn(page.projectId, 'launch')) untrack(() => closePage(id));
  });

  $effect(() => {
    void app.ui.view;
    void app.project?.color;
    app.applyTheme();
  });

  // The discovery of the features (ESC-506): read once the app is ready and the first launch is not on screen (its dot
  // on « Aide », its band, its bubbles).
  $effect(() => {
    if (app.ready && !onboarding.shown) untrack(() => void loadDiscover());
  });
  // The assistant of the features (F-2, ESC-559): open by itself at the first window of the version that brings the
  // choice, on an installation updated to it; never over the first launch, whose station it is.
  $effect(() => {
    if (app.ready && !onboarding.shown) untrack(() => void loadTour());
  });

  // The proposals of a feature switched off at the right moment (F-3, ESC-558): asked now and then once the app is
  // ready, and whenever the core noticed something (`offers.svelte.ts`).
  $effect(() => {
    if (!app.ready) return;
    untrack(() => startOffers());
    return () => stopOffers();
  });

  // The one clock of the caches' degrees (ESC-398): it ticks while one of them is warm, and stops with the last.
  $effect(() => {
    const agents = Object.values(app.agents).map((a) => ({ heat: a.heat }));
    untrack(() => heatClock.watch(agents));
  });

  // « Thème » as saved, once the settings are read (until then, the one the page kept: main.ts).
  $effect(() => {
    if (app.ready) applyAppearance(themeChoice(app.settings.theme));
  });

  // « Couleurs » (PLG-7): the plugins read once the settings are, then the theme chosen among those of the plugins on,
  // at once and as either changes; none (or its plugin off or gone), Escouade's colours.
  // Read again when a gesture installed a plugin of the seed (ESC-373).
  $effect(() => {
    void app.pluginsSeeded;
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

  // The PR tab's badge (ESC-403): the list of the project on screen read in the background, window visible, the
  // backend choosing when (two minutes after any reading, longer after a failure). Nothing is said of a failure.
  $effect(() => {
    const id = app.ready && app.ui.view === 'project' ? app.ui.activeProject : null;
    if (!id || !hasTab(app.codeHosts[id], 'pullRequests')) return;
    const watch = () => {
      if (document.visibilityState === 'visible') api.projectPrsWatch(id).catch(() => {});
    };
    watch();
    const timer = setInterval(watch, WATCH_TICK_MS);
    return () => clearInterval(timer);
  });

  // The box « À toi » (ESC-510) is the MCP server's (docs/INBOX.md § 6): read as the core tells it changed, and as the server
  // is turned on; off, nothing of it stays (its view closes, its count goes), without a restart.
  $effect(() => {
    void app.inboxTick;
    const on = app.ready && app.settings.mcpEnabled;
    untrack(() => {
      if (on) void inbox.load();
      else {
        inbox.clear();
        if (app.ui.view === 'inbox') app.closeInbox();
      }
    });
  });

  // What Escouade puts in the box of its own (ESC-513): its views read once the server is on (then by their sources'
  // events, `inboxViews.take`), gone when it is off; the scope the summary's click asked for.
  $effect(() => {
    const on = app.ready && app.settings.mcpEnabled;
    untrack(() => (on ? void inboxViews.load() : inboxViews.reset()));
  });
  $effect(() => {
    const scope = app.inboxScopeAsked;
    if (!scope) return;
    untrack(() => {
      inbox.scope = scope;
      app.inboxScopeAsked = null;
    });
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
    void app.workspaceOn;
    void app.ui.view;
    void app.panes;
    app.markSeen();
  });

  function onKeydown(e: KeyboardEvent) {
    if (!handleShortcut(e)) return;
    e.preventDefault();
    // A key the app keeps in the VS Code view (ESC-464) never reaches VS Code too: its handler is on the window, after
    // this one (the workbench starts after the app).
    if (inVscode(e.target)) e.stopImmediatePropagation();
  }

  // Before any field: the dictation's keys are never typed (a terminal would send them to its program).
  function onKeyCapture(e: KeyboardEvent) {
    if (dictationKey(e)) {
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    if (e.type === 'keydown') {
      holdPrint(e);
      // No bubble of the discovery while the user types (ESC-506).
      keyPressed();
    }
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
  {#if app.ready}
    <!-- The bands across the window: the files recovered at the start (ESC-469), the certificates not checked (SEC-07),
         a team’s circuit breaker (TS-15), a team’s settings (TS-13), the connection to the core lost on the remote API
         (ESC-495). -->
    <div class="bands">
      <ConnectionBand />
      <RecoveryBanner />
      <TlsBanner />
      <BreakerBand />
      <TeamSettingsBand />
      <TeamHintBand />
      <DemoBand />
      {#if !onboarding.shown}
        <NewBand />
      {/if}
    </div>
  {/if}
  {#if app.ready && onboarding.shown}
    <!-- The first launch (ESC-504): the whole window but its title bar and its status bar, until it closes. -->
    <Onboarding />
  {:else}
    <ProjectRail />
    <div class="body">
      {#if serverScreen}
        <div class="work"><ServerScreen /></div>
      {:else if initError}
        <div class="work"><div class="fatal">{t('shell.app.cantStart', { error: initError })}</div></div>
      {:else if !app.ready}
        <div class="work"></div>
      {:else if app.ui.view === 'overview'}
        <div class="work">
          {#if app.overviewCanvas}<WorkspaceView />{:else}<Overview />{/if}
        </div>
      {:else if app.ui.view === 'inbox' && app.settings.mcpEnabled}
        <InboxQueue />
        <div class="work"><InboxScreen /></div>
      {:else if app.ui.view === 'stats' && app.feature('stats')}
        <div class="work"><Stats /></div>
      {:else if app.ui.view === 'discover'}
        <!-- Aide › À découvrir (ESC-506). -->
        <div class="work"><DiscoverMap /></div>
      {:else if app.ui.view === 'plugins'}
        <PluginsSide />
        <div class="work"><PluginsView /></div>
      {:else if app.project}
        {@const project = app.project}
        <Sidebar {project} />
        {#if app.panesOn && app.panes}
          <!-- The split screen (ESC-392): each pane its own panel on the frame. -->
          <div class="work panes" data-testid="panes"><PaneGrid node={app.panes.root} {project} /></div>
        {:else}
          <!-- What is shown alone carries its key: a content dragged from the sidebar is dropped beside it (ESC-392). -->
          <div class="work" data-pane-key={app.singleItem ? itemKey(app.singleItem) : undefined}>
            {#if app.workspaceOn}
              {#key project.id}
                <WorkspaceView {project} />
              {/key}
            {:else if app.boardOn}
              {#key project.id}
                <Board {project} />
              {/key}
            {:else if app.prsOn}
              {#key project.id}
                <PrsView {project} />
              {/key}
            {:else if app.docsOn}
              {#key project.id}
                <DocsView {project} />
              {/key}
            {:else if app.editorOn}
              {#key project.id}
                <VscodeView {project} />
              {/key}
            {:else if app.runCommand}
              <RunView cmd={app.runCommand} {project} />
            {:else if app.term}
              <TerminalView term={app.term} {project} />
            {:else if app.pageShown}
              <!-- A browser of the project (ESC-657), shown alone as a terminal is. -->
              {#key app.pageShown}
                {#if isAgentPage(app.pageShown)}
                  <!-- An agent's page (AP-3), in its shelter. -->
                  <AgentPageView pageKey={app.pageShown} />
                {:else}
                  <DevPage pageKey={app.pageShown} />
                {/if}
              {/key}
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
        {/if}
      {:else}
        <div class="work"><Welcome /></div>
      {/if}
    </div>
  {/if}
  <StatusBar />
</div>

{#if overseer.open && app.feature('overseer')}
  <OverseerScreen />
{/if}

<!-- A bubble of the discovery (ESC-506), on the real control. -->
<DiscoverTip />
<!-- A proposal of a feature switched off (F-3, ESC-558), at the bottom right. -->
<FeatureOffer />

{#if app.modal?.kind === 'newProject'}
  <NewProjectModal />
{:else if app.modal?.kind === 'connectServer'}
  <ConnectServerModal existing={app.modal.existing} />
{:else if app.modal?.kind === 'settings'}
  <SettingsModal
    page={app.modal.page}
    projectId={app.modal.projectId}
    at={app.modal.at}
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
{:else if app.modal?.kind === 'deleteAgent'}
  {#key app.modal}
    <DeleteAgentModal agentId={app.modal.agentId} />
  {/key}
{:else if app.modal?.kind === 'deleteFolder'}
  {#key app.modal}
    <DeleteFolderModal projectId={app.modal.projectId} folderId={app.modal.folderId} />
  {/key}
{:else if app.modal?.kind === 'harnessConsent'}
  {#key app.modal}
    <HarnessConsentModal {...app.modal} />
  {/key}
{:else if app.modal?.kind === 'harnessLaunch'}
  {#key app.modal}
    <HarnessLaunchModal engine={app.modal.engine} agentId={app.modal.agentId} />
  {/key}
{:else if app.modal?.kind === 'newDiscussion'}
  <NewDiscussionModal projectId={app.modal.projectId} preset={app.modal.preset} />
{:else if app.modal?.kind === 'rename'}
  <RenameModal title={app.modal.title} value={app.modal.value} onSubmit={app.modal.onSubmit} />
{:else if app.modal?.kind === 'account'}
  <AccountModal
    accountId={app.modal.accountId}
    back={app.modal.back}
    engine={app.modal.engine}
    reserveFor={app.modal.reserveFor}
    forAgent={app.modal.forAgent}
    onclose={app.modal.onboarding ? () => (app.modal = null) : undefined}
  />
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
{:else if app.modal?.kind === 'team'}
  <TeamModal step={app.modal.step} link={app.modal.link} teamId={app.modal.teamId} expected={app.modal.expected} />
{:else if app.modal?.kind === 'teamPour'}
  <TeamPourModal projectId={app.modal.projectId} />
{:else if app.modal?.kind === 'teamHandoff'}
  <TeamHandoffModal ticketId={app.modal.ticketId} />
{:else if app.modal?.kind === 'teamHandoffTake'}
  <TeamHandoffTakeModal ticketId={app.modal.ticketId} engines={app.modal.engines} onPick={app.modal.onPick} onCancel={app.modal.onCancel} />
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
{:else if app.modal?.kind === 'ci'}
  <CiModal of={app.modal.of} projectId={app.modal.projectId} />
{:else if app.modal?.kind === 'mergeRepos'}
  <MergeReposModal agentId={app.modal.agentId} />
{:else if app.modal?.kind === 'newBranch'}
  <NewBranchModal projectId={app.modal.projectId} start={app.modal.start} resume={app.modal.resume} repo={app.modal.repo ?? null} />
{:else if app.modal?.kind === 'mergedBranches'}
  <MergedBranchesModal projectId={app.modal.projectId} repo={app.modal.repo ?? null} />
{:else if app.modal?.kind === 'ticket'}
  <TicketForm projectId={app.modal.projectId} ticket={app.modal.ticket} onsubmit={app.modal.onSubmit} resume={app.modal.resume} />
{:else if app.modal?.kind === 'schedule'}
  <!-- « Programmer » (ESC-448): the ticket's form for a scheduled task; it saves the task itself. -->
  <TicketForm projectId={app.modal.projectId} scheduling task={app.modal.task} onsubmit={() => false} resume={app.modal.resume} />
{/if}

{#if tour.shown && !onboarding.shown && !app.modal}
  <FeaturesTourModal />
{/if}

<!-- The Overseer's voice is its own screen's (ESC-468), never this overlay. -->
{#if conversation.overlay && conversation.agentId !== overseer.view.agentId}
  <ConversationOverlay />
{/if}
<ContextMenu />
<PaneDrop />
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
    /* The second row, empty unless a band holds it (`TlsBanner`). */
    grid-template-rows: 44px auto minmax(0, 1fr) 30px;
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
  .bands {
    grid-column: 2;
    grid-row: 2;
    display: flex;
    flex-direction: column;
    margin: 0 8px 0 0;
  }
  .body {
    grid-column: 2;
    grid-row: 3;
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
  /* The split screen: no panel of its own, the panes are (and their ring shows past their edge). */
  .work.panes {
    background: none;
    box-shadow: none;
    border-radius: 0;
    overflow: visible;
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
