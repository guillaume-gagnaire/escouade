<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { openPath } from '@tauri-apps/plugin-opener';
  import { api } from '../lib/ipc';
  import { menu } from '../lib/menu.svelte';
  import { askCloseProject } from '../lib/project-actions';
  import { app } from '../lib/state.svelte';
  import { IS_MAC } from '../lib/platform';
  import { PROJECT_COLORS } from '../lib/theme';
  import type { Project } from '../lib/types';

  const win = getCurrentWindow();
  let maximized = $state(false);
  let dragId = $state<string | null>(null);
  let dropIndex = $state<number | null>(null);

  $effect(() => {
    win.isMaximized().then((m) => (maximized = m));
    const un = win.onResized(() => win.isMaximized().then((m) => (maximized = m)));
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  });

  // The project on screen (none behind the statistics).
  const activeProject = $derived(app.ui.view === 'project' ? (app.projects.find((p) => p.id === app.ui.activeProject) ?? null) : null);

  function stats(p: Project) {
    const agents = Object.values(app.agents).filter((a) => a.projectId === p.id && !a.archived);
    const seen = app.attentionIn(p.id);
    // The most pressing kind sets the blink's color: an error, then a question, then an end.
    const alert = seen.some((a) => a.status === 'error')
      ? 'var(--del)'
      : seen.some((a) => a.status === 'waiting')
        ? 'var(--wait)'
        : seen.length
          ? 'var(--ok)'
          : null;
    return {
      waiting: agents.filter((a) => a.status === 'waiting').length,
      running: agents.some((a) => a.status === 'running'),
      changes: app.git[p.id]?.total ?? 0,
      alert,
      alertTitle: seen.length ? `À voir : ${seen.map((a) => a.name).join(', ')}` : null,
    };
  }

  function tabMenu(e: MouseEvent, p: Project) {
    menu.show(e, [
      {
        label: 'Renommer…',
        onClick: () => {
          app.modal = {
            kind: 'rename',
            title: 'Renommer le projet',
            value: p.name,
            onSubmit: (name) => save({ ...p, name }),
          };
        },
      },
      { label: 'Couleur', colors: { values: PROJECT_COLORS, selected: p.color, onPick: (color) => save({ ...p, color }) } },
      { label: '', separator: true },
      {
        label: p.worktreePerAgent ? 'Désactiver le worktree par agent' : 'Activer le worktree par agent',
        hint: 'nouveaux agents',
        onClick: () => save({ ...p, worktreePerAgent: !p.worktreePerAgent }),
      },
      {
        label: 'Réglages du projet…',
        onClick: () => {
          app.modal = { kind: 'settings', tab: 'projects', projectId: p.id };
        },
      },
      { label: 'Ouvrir le dossier', onClick: () => openPath(p.path).catch((err) => app.toast(String(err), 'error')) },
      { label: '', separator: true },
      { label: 'Fermer le projet…', danger: true, onClick: () => askCloseProject(p) },
    ]);
  }

  async function save(p: Project) {
    const i = app.projects.findIndex((x) => x.id === p.id);
    if (i >= 0) app.projects[i] = p;
    await app.run(api.updateProject(p));
  }

  function onDrop() {
    if (dragId === null || dropIndex === null) return;
    const list = [...app.projects];
    const from = list.findIndex((p) => p.id === dragId);
    const [moved] = list.splice(from, 1);
    list.splice(dropIndex > from ? dropIndex - 1 : dropIndex, 0, moved);
    app.projects = list;
    app.run(api.reorderProjects(list.map((p) => p.id)));
    ((dragId = null), (dropIndex = null));
  }
</script>

<header class="bar" class:mac={IS_MAC} data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <img class="mark" src="/logo.svg" alt="Escouade" draggable="false" />
  </div>
  <nav class="tabs" data-tauri-drag-region>
    {#each app.projects as p, i (p.id)}
      {@const s = stats(p)}
      {@const active = app.ui.view === 'project' && app.ui.activeProject === p.id}
      <button
        class="tab"
        class:active
        class:alert={!!s.alert && !active}
        class:drop-before={dropIndex === i && dragId !== p.id}
        style:--tab-color={p.color}
        style:--alert={s.alert}
        draggable="true"
        title={s.alertTitle && !active ? `${s.alertTitle}\n${p.path}` : p.path}
        onclick={() => app.selectProject(p.id)}
        oncontextmenu={(e) => tabMenu(e, p)}
        onauxclick={(e) => e.button === 1 && tabMenu(e, p)}
        ondragstart={(e) => {
          dragId = p.id;
          e.dataTransfer?.setData('text/plain', p.id);
        }}
        ondragover={(e) => {
          e.preventDefault();
          const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
          dropIndex = e.clientX < r.left + r.width / 2 ? i : i + 1;
        }}
        ondrop={(e) => {
          e.preventDefault();
          onDrop();
        }}
        ondragend={() => ((dragId = null), (dropIndex = null))}
      >
        <span class="swatch" style:background={p.color}></span>
        <span class="name">{p.name}</span>
        {#if s.running && !s.waiting}<span class="dot" style="width:6px;height:6px;background:var(--ok)"></span>{/if}
        {#if s.changes > 0}
          <span class="delta" title="Modifications git non commitées">Δ {s.changes}</span>
        {/if}
        {#if s.waiting > 0}<span class="pill" title="Agents en attente de réponse">{s.waiting}</span>{/if}
      </button>
    {/each}
    <button
      class="add"
      title="Ajouter un projet"
      aria-label="Ajouter un projet"
      onclick={() => {
        app.modal = { kind: 'newProject' };
      }}>+</button
    >
  </nav>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="right">
    {#if activeProject}
      <button
        class="tab stats"
        aria-label="Ouvrir l’éditeur du projet"
        title="Parcourir et éditer les fichiers du projet"
        onclick={() => app.openEditor({ projectId: activeProject.id, source: 'project' })}
      >
        <svg width="14" height="12" viewBox="0 0 14 12" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"
          ><path d="M4.5 2.5 1 6l3.5 3.5M9.5 2.5 13 6l-3.5 3.5" /></svg
        >
        Éditeur
      </button>
    {/if}
    <button class="tab stats" class:active={app.ui.view === 'stats'} onclick={() => app.openStats()}>
      <span class="bars"><span style="height:6px"></span><span style="height:12px"></span><span style="height:9px"></span></span>
      Stats
    </button>
  </div>
  <!-- macOS draws its own buttons (traffic lights) at the left of the bar. -->
  {#if !IS_MAC}
    <div class="controls">
      <button class="ctl" title="Réduire" aria-label="Réduire" onclick={() => win.minimize()}>
        <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0 5h10" stroke="currentColor" stroke-width="1" /></svg>
      </button>
      <button class="ctl" title={maximized ? 'Restaurer' : 'Agrandir'} aria-label="Agrandir" onclick={() => win.toggleMaximize()}>
        {#if maximized}
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"
            ><path d="M2.5 2.5V.5h7v7h-2" /><rect x=".5" y="2.5" width="7" height="7" /></svg
          >
        {:else}
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"
            ><rect x=".5" y=".5" width="9" height="9" /></svg
          >
        {/if}
      </button>
      <button class="ctl close" title="Fermer (l'app reste dans la zone de notification)" aria-label="Fermer" onclick={() => win.hide()}>
        <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1" /></svg>
      </button>
    </div>
  {/if}
</header>

<style>
  .bar {
    height: 46px;
    flex: none;
    display: flex;
    align-items: stretch;
    padding: 0 0 0 12px;
    background: var(--panel);
    border-bottom: 1px solid var(--line2);
    user-select: none;
  }
  /* Room for the traffic lights (window buttons drawn by macOS over the bar). */
  .bar.mac {
    padding-left: 84px;
    padding-right: 8px;
  }
  .brand {
    display: flex;
    align-items: center;
    padding-right: 14px;
  }
  .mark {
    width: 24px;
    height: 24px;
    display: block;
    pointer-events: none;
  }
  .tabs {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    min-width: 0;
    overflow: hidden;
  }
  .tab {
    height: 37px;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 14px;
    border-width: 1px 1px 0 1px;
    border-style: solid;
    border-color: transparent;
    border-top: 2px solid transparent;
    border-radius: var(--r) var(--r) 0 0;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
    cursor: pointer;
    margin-bottom: -1px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tab:hover:not(.active) {
    color: var(--text);
    background: color-mix(in oklch, var(--elev) 50%, transparent);
  }
  .tab.active {
    background: var(--bg);
    border-color: var(--line2);
    border-top-color: var(--tab-color, transparent);
    color: var(--text);
  }
  /* A project with an agent to look at blinks until that agent is seen. */
  .tab.alert {
    color: var(--text);
    border-top-color: var(--alert);
    animation: tabAlert 1.1s ease-in-out infinite alternate;
  }
  @keyframes tabAlert {
    from {
      background: transparent;
    }
    to {
      background: color-mix(in oklch, var(--alert) 26%, transparent);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tab.alert {
      animation: none;
      background: color-mix(in oklch, var(--alert) 20%, transparent);
    }
  }
  .tab.stats.active {
    border-top-color: var(--line2);
  }
  .drop-before {
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    flex: none;
  }
  .name {
    font-weight: 600;
  }
  .delta {
    font-family: var(--mono);
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 99px;
    background: var(--elev2);
    color: var(--muted);
  }
  .add {
    height: 28px;
    width: 28px;
    margin: 0 0 5px 6px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    flex: none;
  }
  .add:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .spacer {
    flex: 1;
    min-width: 24px;
  }
  .right {
    display: flex;
    align-items: flex-end;
    padding-right: 10px;
  }
  .stats {
    font-weight: 600;
    gap: 8px;
    padding: 0 16px;
  }
  .bars {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 12px;
  }
  .bars span {
    width: 3px;
    background: currentColor;
    border-radius: 1px;
  }
  .controls {
    display: flex;
    align-items: stretch;
  }
  .ctl {
    width: 46px;
    border: none;
    background: transparent;
    color: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .ctl:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .ctl.close:hover {
    background: #c42b1c;
    color: #fff;
  }
</style>
