<script lang="ts">
  // The VS Code view, the editor of a project (ESC-447), alone or in a pane.
  // The workbench is the page's only one (`lib/vscode/workbench.ts`, loaded on the first opening, in chunks of its own):
  // this view lends it its place while it is shown and takes it back when it goes, so that its tabs survive a pane
  // moved, closed and opened again. The source picked (the project's checkout or an agent's worktree) is a folder of its
  // workspace; the file the editor is asked to show (`app.openEditor`) opens in a tab of it. It speaks Escouade's
  // language (`lib/vscode/load.ts`) and takes its theme and fonts as they change (ESC-472): a change of language reaches
  // it when the window reloads, which its bar offers.
  import { onDestroy, onMount, untrack } from 'svelte';
  import { sourceAgent } from '../../lib/editor/buffers.svelte';
  import { locale, t } from '../../lib/i18n';
  import { keepTabs, loadWorkbench, page, reopening, workbenchLang } from '../../lib/vscode/load';
  import { viewUnsaved } from '../../lib/vscode/unsaved.svelte';
  import { viewShown } from '../../lib/vscode/shown.svelte';
  import { fInt } from '../../lib/format';
  import { mayFocus, paneCtx } from '../../lib/pane-context';
  import { tip } from '../../lib/tip';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import { worktreeLabel } from '../../lib/worktrees';
  import { api } from '../../lib/ipc';
  import type { Installed } from '../../lib/vscode/extensions';
  import Icon from '../Icon.svelte';
  import ExtensionsPanel from './ExtensionsPanel.svelte';
  import SourcePicker from './SourcePicker.svelte';

  let { project }: { project: Project } = $props();
  /** Its pane of the split screen, if it is in one: only the active pane takes the keyboard. */
  const pane = paneCtx();
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
  /** The name of a source's folder in the workspace. */
  function folderName(of: string) {
    const a = agents.find((x) => x.id === of);
    if (!a) return project.name;
    return a.worktree ? `${a.name} · ${worktreeLabel(project.path, a.worktree.path)}` : a.name;
  }
  const name = $derived(folderName(source));
  /** Escouade's language changed since the workbench was loaded in this page. */
  const otherLang = $derived(phase === 'ready' && workbenchLang() !== locale.ui);
  const wanted = $derived(st?.places[source]?.active ?? null);
  /**
   * The file of VS Code's active editor, when it is of this project: how it differs from its base (ESC-476), as the
   * old editor said it above its text, and « Comparer ».
   */
  const shown = $derived(phase === 'ready' && viewShown.file?.projectId === project.id ? viewShown.file : null);
  const shownName = $derived(shown ? (shown.path.split('/').pop() ?? shown.path) : '');
  const reference = $derived(shown?.reference ?? t('editor.vscode.theBase'));

  onMount(() => {
    void (async () => {
      try {
        const mod = await loadWorkbench(locale.ui);
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
        // The extensions installed, laid once the workbench is there (no request to Open VSX: the core's list).
        void relayExtensions(mod);
        await reopen(mod);
      } catch (e) {
        failure = String(e);
        phase = 'failed';
        console.error('[vscode]', e);
      }
    })();
  });

  /** The panel of the extensions of Open VSX (ESC-481), beside the workbench. */
  let extOpen = $state(false);
  /** The projects whose folders the workbench shows, as the panel is opened: their `.escouade.yml` and privacy count. */
  let extProjects = $state<string[]>([]);

  function toggleExtensions() {
    if (!extOpen) extProjects = wb?.viewProjects() ?? [project.id];
    extOpen = !extOpen;
  }

  /** What the core says is installed, laid in the workbench: the declarative extensions that run, the theme chosen. */
  async function layExtensions(installed: Installed | null) {
    if (installed) await wb?.useExtensions(installed.extensions, installed.theme);
  }

  /** The projects of the workspace the extensions were laid for last. */
  let laidFor: string[] = [];

  /** What the core says installed for the projects of the workspace (no request to Open VSX), laid. */
  async function relayExtensions(mod: NonNullable<typeof wb>) {
    try {
      laidFor = mod.viewProjects();
      await layExtensions(await api.vsxInstalled(laidFor));
    } catch (e) {
      console.warn('[vscode] extensions', e);
    }
  }

  /** The keyboard was given to the view once it showed: a file asked for since takes it with it. */
  let welcomed = false;

  // The source picked is a folder of the workspace. The file asked for (a link of a conversation, « Ouvrir un fichier »,
  // a file of the side panel) opens in a tab, at its line, the keyboard in it (ESC-464): each request once, so that a
  // pane moved or opened again does not bring the cursor back to it. A workbench just started shows the file the editor
  // had on screen; the view shown takes the keyboard.
  $effect(() => {
    const at = src;
    const rel = wanted;
    const folder = name;
    const asked = st?.reveal ?? null;
    const pid = project.id;
    if (phase !== 'ready' || !wb) return;
    const mod = wb;
    void mod
      .workbench(at, folder, app.settings.editorFontSize)
      .then(async () => {
        // A project come into the workspace brings its `.escouade.yml` and its privacy: the extensions laid again.
        if (!mod.viewProjects().every((p) => laidFor.includes(p))) void relayExtensions(mod);
        if (asked) {
          untrack(() => {
            const cur = app.editor[pid];
            if (cur?.reveal?.seq === asked.seq) cur.reveal = null;
          });
          welcomed = true;
          return mod.openFile(at, asked.path, asked);
        }
        if (rel && !mod.hasTabs()) await mod.openFile(at, rel);
        if (!welcomed && untrack(() => mayFocus(pane))) await mod.focus();
        welcomed = true;
      })
      .catch((e) => app.toast(String(e), 'error'));
  });

  /** The tabs of its project kept by a reload for the language, opened again, the one shown last. */
  async function reopen(mod: NonNullable<typeof wb>) {
    const tabs = reopening.tabs.filter((x) => x.projectId === project.id);
    reopening.tabs = [];
    for (const tab of tabs) {
      const at = { projectId: tab.projectId, agentId: sourceAgent(tab.source) };
      await mod.workbench(at, folderName(tab.source), app.settings.editorFontSize);
      await mod.openFile(at, tab.path);
    }
  }

  // « Taille du texte de l'éditeur », changed: at once.
  $effect(() => {
    const size = app.settings.editorFontSize;
    if (phase === 'ready') wb?.setFontSize(size);
  });

  /** The window reloaded for VS Code to speak Escouade's language: the modified files asked about first, the tabs kept. */
  function reloadForLanguage() {
    void app.askUnsaved(viewUnsaved.of(), () => {
      if (wb) keepTabs(wb.openTabs());
      page.reload();
    });
  }

  // Its pane made active from the keyboard (Ctrl+Alt+arrow): the view takes the keyboard.
  $effect(() => {
    if (!app.paneFocus) return;
    if (untrack(() => pane?.active) && wb) void wb.focus();
  });

  onDestroy(() => {
    alive = false;
    if (place) wb?.detach(place);
  });
</script>

<div class="vscode-view" data-testid="vscode-view">
  <header class="vscode-bar">
    <button class="btn" data-testid="vscode-back" onclick={() => app.leaveEditor(project.id)}
      ><span class="back"><Icon name="right" size={14} /></span>{t('editor.head.back')}</button
    >
    <SourcePicker
      {project}
      {source}
      {agents}
      git={app.git[project.id]}
      onpick={(s) => app.openEditor({ projectId: project.id, source: s })}
    />
    {#if shown}
      <span class="vs" data-testid="vscode-changes">
        {#if shown.isNew}
          {t('editor.diff.newFile', { reference })}
        {:else if shown.count}
          <span class="add">{t('editor.diff.changedCount', { count: shown.count, n: fInt(shown.count) })}</span>
          {t('editor.diff.changedVs', { count: shown.count, reference })}
        {:else}
          {t('editor.diff.same', { reference })}
        {/if}
      </span>
      {#if shown.isNew || shown.count}
        <button
          class="btn compare"
          data-testid="vscode-compare"
          use:tip={t('editor.vscode.compareTip', { name: shownName, reference })}
          onclick={() => void viewShown.compare().catch((e) => app.toast(String(e), 'error'))}>{t('editor.vscode.compare')}</button
        >
      {/if}
    {/if}
    {#if otherLang}
      <span class="lang-note" data-testid="vscode-lang-note">{t('editor.vscode.langNote')}</span>
      <button class="btn reload" data-testid="vscode-reload" use:tip={t('editor.vscode.reloadTip')} onclick={reloadForLanguage}
        >{t('editor.vscode.reload')}</button
      >
    {:else}
      <span class="what">{t('editor.vscode.what')}</span>
    {/if}
    {#if phase === 'ready'}
      <button
        class="btn ext"
        class:on={extOpen}
        data-testid="vscode-ext-open"
        aria-pressed={extOpen}
        aria-label={t('editor.ext.open')}
        use:tip={t('editor.ext.openTip')}
        onclick={toggleExtensions}><Icon name="blocks" size={14} /><span class="ext-label">{t('editor.ext.open')}</span></button
      >
    {/if}
  </header>
  <div class="vscode-body">
    <div class="vscode-place" bind:this={place}></div>
    {#if extOpen && phase === 'ready'}
      <ExtensionsPanel projects={extProjects} onclose={() => (extOpen = false)} onlaid={layExtensions} />
    {/if}
  </div>
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
  .vscode-body {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
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
  .vscode-bar .what,
  .vscode-bar .lang-note {
    margin-left: auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vscode-bar .reload {
    flex: none;
    color: var(--p-ink);
  }
  .vscode-bar .ext {
    flex: none;
  }
  .vscode-bar .ext.on {
    color: var(--p-ink);
  }
  /* A narrow bar (a small window, a pane): what says where it is (« VS Code ») goes, « Extensions » keeps its icon,
     its name and its tooltip; the source, the count of the changes and « Comparer » (A7) keep their room. */
  .vscode-view {
    container: vscode-view / inline-size;
  }
  @container vscode-view (max-width: 900px) {
    .vscode-bar .what {
      display: none;
    }
    .vscode-bar .what + .ext {
      margin-left: auto;
    }
    .vscode-bar .ext-label {
      display: none;
    }
  }
  /* How the file shown differs from its base, as the old editor said it above its text (ESC-476). */
  .vscode-bar .vs {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
  }
  .vscode-bar .vs .add {
    color: var(--add);
  }
  .vscode-bar .compare {
    flex: none;
    padding: 2px 8px;
    border-radius: 6px;
  }
  .vscode-bar .compare:hover {
    background: var(--hover);
    color: var(--text);
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
