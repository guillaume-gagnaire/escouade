<script lang="ts">
  // ESC-444 (study): the VS Code view in place of the editor, alone or in a pane. The workbench is the page's only one
  // (`lib/vscode/workbench.ts`): this view lends it its place while it is shown and gives it back to the park when it
  // goes, so its tabs survive (it is laid over this view's place, never moved into it) a pane moved, closed and opened again. The file the editor is asked to show
  // (`app.openEditor`: a link of a conversation, Quick Open) opens in a tab of it.
  import { onDestroy, onMount } from 'svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import type { Project } from '../../lib/types';

  let { project }: { project: Project } = $props();
  let place: HTMLDivElement;
  let status = $state('');
  let failed = $state('');
  let hostNote = $state('');
  let mod: typeof import('../../lib/vscode/workbench') | null = null;

  /** The language servers started, by project. */
  const servers = ((globalThis as { __escouadeLsp?: Set<string> }).__escouadeLsp ??= new Set<string>());

  onMount(() => {
    status = 'Chargement de la vue VS Code…';
    const t0 = performance.now();
    void (async () => {
      try {
        mod = await import('../../lib/vscode/workbench');
        const { vscodeFlags } = await import('../../lib/vscode/flag.svelte');
        await mod.workbench(project.id, { nodeHost: vscodeFlags.nodeHost });
        if (!place.isConnected) return;
        // In a pane: a press on the workbench makes it active, a hover shows its ×, as anywhere in a pane.
        const pane = place.closest<HTMLElement>('[data-pane-key]');
        await mod.attach(
          place,
          pane
            ? {
                down: () => {
                  const key = pane.dataset.paneKey ?? '';
                  if (pane.dataset.paneActive !== 'true') app.activatePane(key);
                },
                hover: (on) => pane.classList.toggle('through', on),
              }
            : {},
        );
        status = '';
        hostNote = mod.nodeHost ? ` · hôte Node ${mod.nodeHost.node}` : '';
        (globalThis as { __vscodeOpenMs?: number }).__vscodeOpenMs = Math.round(performance.now() - t0);
        void startServer();
      } catch (e) {
        failed = String(e);
        console.error('[vscode]', e);
      }
    })();
  });

  async function startServer() {
    // With the Node extension host, the language extensions run there (the core's servers read `file:` URIs).
    if (servers.has(project.id) || !mod || mod.nodeHost) return;
    servers.add(project.id);
    const { api } = await import('../../lib/ipc');
    const tree = await api.fsTree(project.id, null);
    const server = tree.files.includes('Cargo.toml') ? 'rust' : tree.files.some((f) => /\.(ts|js)x?$/.test(f)) ? 'typescript' : null;
    const root = mod.folderOf(project.id);
    if (!server || !root) return;
    const { startLanguageServer } = await import('../../lib/vscode/lsp');
    try {
      const r = await startLanguageServer(project.id, server, root);
      (globalThis as { __escouadeLspRunning?: unknown[] }).__escouadeLspRunning ??= [];
      (globalThis as unknown as { __escouadeLspRunning: unknown[] }).__escouadeLspRunning.push({ projectId: project.id, server, ...r });
    } catch (e) {
      console.warn('[vscode] language server not started:', e);
    }
  }

  // The file the editor is asked to show.
  const wanted = $derived.by(() => {
    const st = app.editor[project.id];
    return st?.source === 'project' ? (st.places.project?.active ?? null) : null;
  });
  $effect(() => {
    const rel = wanted;
    if (rel && mod && !status) void mod.openFile(project.id, rel);
  });

  onDestroy(() => {
    if (place) mod?.detach(place);
  });
</script>

<div class="vscode-view" data-testid="vscode-view">
  <div class="vscode-bar">
    <button class="btn" data-testid="vscode-back" onclick={() => app.closeEditor(project.id)}
      ><span class="back"><Icon name="right" size={14} /></span>Conversation</button
    >
    <span class="what">Vue VS Code · étude ESC-444{hostNote}</span>
  </div>
  <div class="vscode-place" bind:this={place}></div>
  {#if failed}
    <p class="vscode-msg" role="alert">La vue VS Code n'a pas démarré : {failed}</p>
  {:else if status}
    <p class="vscode-msg">{status}</p>
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
  .vscode-bar {
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
  .vscode-msg {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--muted);
    font-size: 13px;
  }
</style>
