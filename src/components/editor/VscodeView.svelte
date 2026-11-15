<script lang="ts">
  // The VS Code view, the editor of a project (ESC-447), alone or in a pane.
  // The workbench is the page's only one (`lib/vscode/workbench.ts`, loaded on the first opening, in chunks of its own):
  // this view lends it its place while it is shown and takes it back when it goes, so that its tabs survive a pane
  // moved, closed and opened again. The source picked (the project's checkout or an agent's worktree) is a folder of its
  // workspace; the file the editor is asked to show (`app.openEditor`) opens in a tab of it.
  import { onDestroy, onMount } from 'svelte';
  import { sourceAgent } from '../../lib/editor/buffers.svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import { worktreeLabel } from '../../lib/worktrees';
  import Icon from '../Icon.svelte';
  import SourcePicker from './SourcePicker.svelte';

  let { project }: { project: Project } = $props();
  let place: HTMLDivElement;
  let phase = $state<'loading' | 'ready' | 'failed'>('loading');
  let failure = $state('');
  /** The workbench's module, once loaded ('ready'). */
  let wb: typeof import('../../lib/vscode/workbench') | null = null;
  let alive = true;

  const st = $derived(app.editor[project.id]);
  const source = $derived(st?.source ?? 'project');
  const agents = $derived(app.projectAgents.filter((a) => a.worktree));
  const src = $derived({ projectId: project.id, agentId: sourceAgent(source) });
  /** The name of the source's folder in the workspace. */
  const name = $derived.by(() => {
    const a = agents.find((x) => x.id === source);
    if (!a) return project.name;
    return a.worktree ? `${a.name} · ${worktreeLabel(project.path, a.worktree.path)}` : a.name;
  });
  const wanted = $derived(st?.places[source]?.active ?? null);

  onMount(() => {
    void (async () => {
      try {
        const mod = await import('../../lib/vscode/workbench');
        await mod.workbench(src, name, app.settings.editorFontSize);
        if (!alive) return;
        // In a pane: a press on the workbench makes it active, a hover shows its ×, as anywhere in a pane.
        const pane = place.closest<HTMLElement>('[data-pane-key]');
        mod.attach(
          place,
          pane
            ? {
                down: () => {
                  if (pane.dataset.paneActive !== 'true') app.activatePane(pane.dataset.paneKey ?? '');
                },
                hover: (on) => pane.classList.toggle('through', on),
              }
            : {},
        );
        wb = mod;
        phase = 'ready';
      } catch (e) {
        failure = String(e);
        phase = 'failed';
        console.error('[vscode]', e);
      }
    })();
  });

  // The source picked is a folder of the workspace, its file asked for opened in a tab.
  $effect(() => {
    const at = src;
    const rel = wanted;
    const folder = name;
    if (phase !== 'ready' || !wb) return;
    const mod = wb;
    void mod
      .workbench(at, folder, app.settings.editorFontSize)
      .then(() => (rel ? mod.openFile(at, rel) : undefined))
      .catch((e) => app.toast(String(e), 'error'));
  });

  onDestroy(() => {
    alive = false;
    if (place) wb?.detach(place);
  });
</script>

<div class="vscode-view" data-testid="vscode-view">
  <header class="vscode-bar">
    <button class="btn" data-testid="vscode-back" onclick={() => app.closeEditor(project.id)}
      ><span class="back"><Icon name="right" size={14} /></span>{t('editor.head.back')}</button
    >
    <SourcePicker
      {project}
      {source}
      {agents}
      git={app.git[project.id]}
      onpick={(s) => app.openEditor({ projectId: project.id, source: s })}
    />
    <span class="what">{t('editor.vscode.what')}</span>
  </header>
  <div class="vscode-place" bind:this={place}></div>
  {#if phase === 'failed'}
    <p class="vscode-msg" role="alert">{t('editor.vscode.failed', { error: failure })}</p>
  {:else if phase === 'loading'}
    <p class="vscode-msg">{t('editor.vscode.loading')}</p>
  {/if}
</div>

<style>
  .vscode-view {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--panel);
  }
  .vscode-place {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  /* Above the workbench's layer (`#vscode-layer`, z-index 3), so that the menu of the source opens over the
     workbench; under the × of a pane (z-index 5). */
  .vscode-bar {
    position: relative;
    z-index: 4;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 10px;
    border-bottom: 1px solid var(--line);
    background: var(--frame);
    color: var(--muted);
    font-size: 12px;
  }
  .vscode-bar .btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 0;
    background: none;
    color: var(--soft);
    font: inherit;
    cursor: pointer;
  }
  .vscode-bar .back {
    display: inline-flex;
    transform: rotate(180deg);
  }
  .vscode-bar .what {
    margin-left: auto;
  }
  /* In a pane, its × stands at the bar's right end. */
  :global(.pane) .vscode-bar {
    padding-right: 34px;
  }
  .vscode-msg {
    position: absolute;
    inset: 36px 0 0;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
</style>
