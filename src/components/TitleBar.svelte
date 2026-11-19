<script lang="ts">
  import { paneDrag } from '../lib/pane-drag.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { dictation } from '../lib/dictation.svelte';
  import { t } from '../lib/i18n';
  import { deltaDetail } from '../lib/repos';
  import { menu } from '../lib/menu.svelte';
  import { IS_MAC, keyLabel } from '../lib/platform';
  import { projectMenu } from '../lib/project-rail';
  import { app } from '../lib/state.svelte';
  import { overseer } from '../lib/overseer.svelte';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';
  import PrivateMark from './PrivateMark.svelte';
  import SpacePlate from './servers/SpacePlate.svelte';
  import { connection } from '../lib/connection.svelte';
  import { knowsServers } from '../lib/servers.svelte';
  import { spacesShown, spacesState } from '../lib/spaces.svelte';
  import { windowSpace } from '../lib/transport';

  // The title bar of the 2.0 design (ESC-158), with no tabs: over the projects' rail, the logo (Windows, Linux) or the
  // room of macOS's traffic lights; over the agents' sidebar, the active project's name and its menu; the search in the
  // middle; the editor and the window's buttons on the right. The projects themselves are on the rail (ProjectRail).

  const win = getCurrentWindow();
  let maximized = $state(false);
  let nameButton = $state<HTMLButtonElement>();
  /** Its menu is the one open. */
  let menuOpen = $state(false);
  $effect(() => {
    if (!menu.open) menuOpen = false;
  });

  $effect(() => {
    win.isMaximized().then((m) => (maximized = m));
    const un = win.onResized(() => win.isMaximized().then((m) => (maximized = m)));
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  });

  /** The first start: the logo and the window's buttons alone (choice 49). */
  const first = $derived(app.projects.length === 0);
  // The project on screen (none behind « Vue d'ensemble » or the statistics).
  const activeProject = $derived(app.ui.view === 'project' ? (app.projects.find((p) => p.id === app.ui.activeProject) ?? null) : null);
  const changes = $derived(activeProject ? (app.git[activeProject.id]?.total ?? 0) : 0);
  /** A shortcut as the bar writes it: « Ctrl K ». */
  const kbd = (s: string) => keyLabel(s).split('+').join(' ');

  // The machine (ESC-537, docs/SERVER-UX.md § 4): once this computer knows a server, the space before the project — its
  // plate and name, the SSH host (not at 1024), amber with « reconnexion » while cut; « Ce poste » on this computer.
  const server = $derived(connection.server);
  const cut = $derived(server !== null && connection.state !== 'open');
  const machineShown = $derived(knowsServers());
  // « Partout » (ESC-539, direction B): no machine in particular, each row says its own.
  const everywhere = $derived(app.ui.view === 'overview' && spacesShown() && spacesState.filter === 'all');

  function openMenu() {
    if (!activeProject || !nameButton) return;
    menu.showAt(nameButton, projectMenu(activeProject));
    menuOpen = true;
  }
</script>

<header class="bar" class:mac={IS_MAC} data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    <!-- macOS draws its own buttons (traffic lights) in this corner. -->
    <div class="corner" data-tauri-drag-region>
      {#if !IS_MAC}<img class="mark" src="/logo.svg" alt="Escouade" draggable="false" />{/if}
    </div>
    {#if everywhere}
      <span class="machine" role="status" data-testid="title-machine"><Icon name="overview" size={13} /><b>{t('spaces.all')}</b></span>
      <span class="slash" aria-hidden="true">/</span><b class="view">{t('common.overview')}</b>
    {:else if machineShown}
      <span
        class="machine"
        class:cut
        role="status"
        aria-label={t('servers.machineLabel', { name: server ? server.name : t('servers.thisComputer') })}
        data-testid="title-machine"
      >
        <SpacePlate name={server?.name ?? ''} here={!server} {cut} size={18} />
        <b>{server ? server.name : t('servers.thisComputer')}</b>
        {#if server}<span class="host">{server.ssh}</span>{/if}
        {#if cut && connection.state !== 'refused'}<span class="re"><Icon name="refresh" size={11} />{t('servers.reconnecting')}</span>{/if}
      </span>
      {#if activeProject && !first}<span class="slash" aria-hidden="true">/</span>{/if}
    {/if}
    {#if activeProject && !first}
      <button
        class="proj"
        bind:this={nameButton}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        aria-label={t('nav.titleBar.projectMenu', { name: activeProject.name })}
        onclick={openMenu}
        oncontextmenu={(e) => menu.show(e, projectMenu(activeProject))}
      >
        <span class="sw" aria-hidden="true"></span>
        <span class="name">{activeProject.name}</span>
        {#if activeProject.private}<PrivateMark />{/if}
        {#if changes > 0}<span
            class="delta"
            use:tip={app.git[activeProject.id]?.repos?.length ? deltaDetail(app.git[activeProject.id]) : t('nav.titleBar.delta')}
            >Δ {changes}</span
          >{/if}
        <span class="chev"><Icon name="down" size={14} /></span>
      </button>
    {/if}
  </div>
  {#if !first}
    <button
      class="search"
      aria-label={t('nav.titleBar.searchLabel', { key: keyLabel('Ctrl+K') })}
      onclick={() => (app.modal = { kind: 'convSearch' })}
    >
      <Icon name="search" size={14} />
      <span class="what">{t('nav.titleBar.search')}</span>
      <kbd>{kbd('Ctrl+K')}</kbd>
    </button>
  {:else}
    <span data-tauri-drag-region></span>
  {/if}
  <div class="right" data-tauri-drag-region>
    {#if app.settings.overseer?.enabled && !first}
      <!-- The Overseer's discussion (ESC-462): its screen comes with OV-2. -->
      <button
        class="btn-bar"
        class:on={overseer.open}
        aria-pressed={overseer.open}
        aria-label={t('overseer.panel.openTitle')}
        use:tip={t('overseer.panel.openTitle')}
        onclick={() => (overseer.open = !overseer.open)}
        data-overseer-button
      >
        <Icon name="sparkle" />
        <span class="lbl">{t('overseer.panel.open')}</span>
      </button>
    {/if}
    {#if activeProject && !first}
      <button
        class="btn-bar"
        data-discover="editor"
        aria-label={t('nav.titleBar.openEditor')}
        use:tip={t('nav.titleBar.openEditorTitle')}
        onclick={() => app.openEditor({ projectId: activeProject.id, source: 'project' })}
        onpointerdown={(e) => paneDrag.press(e, { kind: 'editor' }, t('common.editor'))}
      >
        <Icon name="code" />
        <span class="lbl">{t('common.editor')}</span>
      </button>
    {/if}
    {#if !IS_MAC}
      <div class="controls">
        <button class="ctl" use:tip={t('nav.titleBar.minimize')} aria-label={t('nav.titleBar.minimize')} onclick={() => win.minimize()}>
          <Icon name="min" size={14} />
        </button>
        <button
          class="ctl"
          use:tip={maximized ? t('nav.titleBar.restore') : t('nav.titleBar.maximize')}
          aria-label={t('nav.titleBar.maximize')}
          onclick={() => win.toggleMaximize()}
        >
          <Icon name={maximized ? 'restore' : 'max'} size={14} />
        </button>
        <button
          class="ctl close"
          use:tip={t('nav.titleBar.closeWindow')}
          aria-label={t('common.close')}
          onclick={() => {
            // Closed to the tray: no one sees a dictation under way any more.
            dictation.cancel();
            // A space's own window (ESC-539) closes; the main one goes to the tray.
            void (windowSpace ? win.close() : win.hide());
          }}
        >
          <Icon name="close" size={14} />
        </button>
      </div>
    {/if}
  </div>
</header>

<style>
  .bar {
    grid-column: 1 / -1;
    grid-row: 1;
    height: 44px;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    user-select: none;
  }
  .left,
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    align-self: stretch;
  }
  .right {
    grid-column: 3;
    justify-content: flex-end;
  }
  /* Windows and Linux: the logo over the rail. macOS: the room of its three buttons. */
  .corner {
    flex: none;
    display: grid;
    place-items: center;
    width: var(--rail);
    align-self: stretch;
    container: corner / inline-size;
  }
  /* The rail widened to a list (ESC-168): the logo stays over its tiles. */
  @container corner (min-width: 160px) {
    .mark {
      justify-self: start;
      margin-left: 15px;
    }
  }
  .bar.mac .corner {
    width: var(--mac-corner);
  }
  .bar.mac .right {
    padding-right: 8px;
  }
  .mark {
    width: 22px;
    height: 22px;
    display: block;
    pointer-events: none;
  }
  /* The machine (ESC-537): the space's plate and name, its SSH host; amber while cut. */
  .machine {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 7px;
    height: 28px;
    margin-left: 2px;
    padding: 0 9px 0 5px;
    border-radius: 8px;
    color: var(--text);
    font-size: 13px;
    white-space: nowrap;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .machine b {
    font-weight: 620;
  }
  .machine .host {
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
  }
  .machine.cut {
    color: var(--wait);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .machine.cut b {
    color: var(--wait);
  }
  .machine .re {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-weight: 600;
  }
  .view {
    font-size: 13px;
    font-weight: 620;
    white-space: nowrap;
  }
  .slash {
    color: var(--dim);
    margin: 0 2px;
  }
  /* At 1024, the host leaves the bar: the plate and the name stay (§ 11). */
  @media (max-width: 1100px) {
    .machine .host {
      display: none;
    }
  }
  /* The active project, over its sidebar: its name, its changes, the same menu as its tile. */
  .proj {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 30px;
    margin-left: 2px;
    padding: 0 8px 0 9px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    font: 600 13px/1 var(--font);
    letter-spacing: -0.005em;
    white-space: nowrap;
    cursor: pointer;
    transition: background-color 0.15s;
  }
  .proj:hover,
  .proj[aria-expanded='true'] {
    background: var(--hover);
  }
  .sw {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 2.5px;
    background: var(--p);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .delta {
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .chev {
    display: inline-flex;
    color: var(--muted);
  }
  .search {
    grid-column: 2;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    width: min(340px, 33vw);
    height: 30px;
    padding: 0 6px 0 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.025);
    color: var(--muted);
    font: 400 13px/1 var(--font);
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .search:hover {
    border-color: var(--line-2);
  }
  .what {
    flex: 1;
    text-align: left;
  }
  .btn-bar {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 9px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font: 500 13px/1 var(--font);
    white-space: nowrap;
    cursor: pointer;
    transition:
      background-color 0.15s,
      color 0.15s;
  }
  .btn-bar:hover {
    background: var(--hover);
    color: var(--text);
  }
  .controls {
    display: flex;
    align-self: stretch;
    margin-left: 6px;
  }
  .ctl {
    width: 46px;
    display: grid;
    place-items: center;
    border: none;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .ctl :global(.icon) {
    stroke-width: 1.1;
  }
  .ctl:hover {
    background: var(--hover);
    color: var(--text);
  }
  .ctl.close:hover {
    background: var(--win-close);
    color: var(--on-win-close);
  }
</style>
