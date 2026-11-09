<script lang="ts">
  import Icon from './Icon.svelte';
  import { onMount, tick } from 'svelte';
  import { editorJump, parseQuickQuery, QUICK_OPEN_MAX, rankFiles, recentFiles } from '../lib/editor/quick-open';
  import { trees } from '../lib/editor/trees.svelte';
  import { trapFocus } from '../lib/focus';
  import { pointerMoves } from '../lib/hover';
  import { basename, fInt } from '../lib/format';
  import { t } from '../lib/i18n';
  import { keyLabel } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import FileIcon from './editor/FileIcon.svelte';
  import { tip } from '../lib/tip';

  // « Ouvrir un fichier » (Ctrl+P): the files of a source of a project, ranked for what is typed. `nom:42` opens the
  // file at its line 42.

  let { projectId, source }: { projectId: string; source: string } = $props();

  const tree = $derived(trees.get(projectId, source));
  /** A plain copy, made once per tree: ranking reads every path at each key, which a `$state` proxy slows down. */
  const files = $derived(tree ? [...tree.files] : []);
  const ignored = $derived(new Set(tree?.ignored));

  let list = $state<HTMLDivElement>();
  let text = $state('');
  /** The rank of the file Enter opens. */
  let active = $state(0);
  let failure = $state<string | null>(null);

  const query = $derived(parseQuickQuery(text));
  /** The files opened last, but the one the editor shows: Enter with nothing typed goes back to the one before. */
  const recent = $derived.by(() => {
    const ed = app.editor[projectId];
    const shown = app.editorActive && app.project?.id === projectId && ed?.source === source ? ed.places[source]?.active : null;
    return recentFiles.list(projectId, source).filter((p) => p !== shown);
  });
  const shown = $derived(rankFiles(files, query.name, recent, QUICK_OPEN_MAX));
  const cursor = $derived(Math.min(active, Math.max(0, shown.length - 1)));

  /** Where the files come from: the project, or the worktree of an agent. */
  const place = $derived(
    source === 'project'
      ? (app.projects.find((p) => p.id === projectId)?.name ?? '')
      : t('editor.source.worktree', { name: app.agents[source]?.name ?? '' }),
  );

  /** The statements of the line below the list, one after the other: each is a whole phrase of its own. */
  const joined = (...parts: (string | false)[]) => parts.filter(Boolean).join(' · ');
  const status = $derived.by(() => {
    if (!tree) return t('editor.quickOpen.loading');
    const cut = tree.truncated && t('editor.quickOpen.listTruncated');
    if (!query.name) return joined(t('editor.count.files', { count: tree.files.length, n: fInt(tree.files.length) }), cut);
    if (!shown.length) return joined(t('editor.quickOpen.noMatch'), cut);
    const n =
      shown.length >= QUICK_OPEN_MAX
        ? t('editor.quickOpen.firstResults', { max: QUICK_OPEN_MAX })
        : t('editor.count.results', { count: shown.length, n: fInt(shown.length) });
    return joined(n, query.line ? t('editor.quickOpen.opensAtLine', { line: query.line }) : false, cut);
  });

  // The tree is read again each time: the agents may have created files since it was last read. The one known is shown
  // meanwhile.
  onMount(() => {
    trees.load(projectId, source).catch((e) => (failure = String(e)));
  });

  /** Whether the cursor moved last by the keyboard (the arrows, a search): not by the mouse. */
  let byKeys = true;
  const moved = pointerMoves();

  // The chosen file stays in view as the arrows move through a long list; never under the mouse (ESC-188).
  $effect(() => {
    void shown;
    const at = cursor;
    if (byKeys) list?.querySelector(`#${optionId(at)}`)?.scrollIntoView?.({ block: 'nearest' });
  });

  /** The file the pointer moves onto becomes the cursor's. */
  function hover(e: MouseEvent, rank: number) {
    if (!moved(e)) return;
    byKeys = false;
    active = rank;
  }

  const optionId = (rank: number) => `quick-open-${rank}`;
  const folder = (path: string) => path.slice(0, Math.max(0, path.lastIndexOf('/')));

  function close() {
    app.modal = null;
  }

  function move(by: number) {
    const n = shown.length;
    if (!n) return;
    byKeys = true;
    active = (cursor + by + n) % n;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      move(e.key === 'ArrowDown' ? 1 : -1);
    } else if (e.key === 'Enter' && !e.isComposing) {
      e.preventDefault();
      const path = shown[cursor];
      if (path) open(path);
    }
  }

  /**
   * Opens `path` at the line typed, else at its top (the cursor goes in it, to type right away). Once the palette is
   * gone, so that the focus it gives back is not the one the file takes. The editor on screen does it through its
   * history, to keep the place left (Alt+←); there is no history to keep without it.
   */
  async function open(path: string) {
    // Read now: the props are read from the dialog, which is gone once it is closed.
    const { line = 1, col } = query;
    const [pid, src] = [projectId, source];
    close();
    await tick();
    const target = col ? { path, line, col } : { path, line };
    const inEditor = app.editorActive && app.project?.id === pid && app.editor[pid]?.source === src;
    if (inEditor && editorJump(target)) return;
    app.openEditor({ projectId: pid, source: src, ...target });
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={close}>
  <div
    class="palette"
    use:trapFocus
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={t('editor.quickOpen.title')}
  >
    <div class="search">
      <span class="ic" aria-hidden="true"><Icon name="search" size={16} /></span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        bind:value={text}
        autofocus
        spellcheck="false"
        placeholder={t('editor.quickOpen.placeholder')}
        role="combobox"
        aria-label={t('editor.quickOpen.title')}
        aria-expanded={shown.length > 0}
        aria-controls="quick-open-files"
        aria-autocomplete="list"
        aria-activedescendant={shown.length ? optionId(cursor) : undefined}
        oninput={() => {
          byKeys = true;
          active = 0;
        }}
        onkeydown={onKeydown}
      />
      <span class="where" use:tip={{ text: place, cut: true }}>{place}</span>
      <button class="icon-btn close" onclick={close} aria-label={t('common.close')}><Icon name="close" size={14} /></button>
    </div>
    <div class="list" id="quick-open-files" role="listbox" aria-label={t('common.files')} bind:this={list}>
      {#each shown as path, rank (path)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="hit"
          class:on={rank === cursor}
          id={optionId(rank)}
          role="option"
          tabindex="-1"
          aria-selected={rank === cursor}
          onclick={() => open(path)}
          onmousemove={(e) => hover(e, rank)}
        >
          <FileIcon />
          <span class="name">{basename(path)}</span>
          {#if folder(path)}<span class="dir">{folder(path)}</span>{/if}
          {#if ignored.has(path)}<span class="tag">{t('editor.quickOpen.ignored')}</span>{/if}
        </div>
      {/each}
    </div>
    <div class="foot">
      {#if failure}
        <span class="status error" role="alert">{failure}</span>
      {:else}
        <span class="status" role="status">{status}</span>
      {/if}
      <div style="flex:1"></div>
      <span class="keys" aria-hidden="true"
        ><kbd><Icon name="up" size={11} /><Icon name="pull" size={11} /></kbd>
        {t('editor.quickOpen.keyChoose')} <span class="sep">·</span> <kbd>{keyLabel('Enter')}</kbd>
        {t('editor.quickOpen.keyOpen')} <span class="sep">·</span>
        <kbd>{keyLabel('Esc')}</kbd>
        {t('editor.quickOpen.keyClose')}</span
      >
    </div>
  </div>
</div>

<style>
  /* A palette of the 2.0 design: over the window's scrim, a raised panel 14 px round, the field at its top with its
     icon, the list, the keys at the foot. */
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    background: var(--scrim);
    animation: fade 0.12s var(--ease);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .palette {
    width: 680px;
    max-width: calc(100vw - 40px);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    border-radius: 14px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 24px 64px -12px rgba(0, 0, 0, calc(0.8 * var(--shade)));
    overflow: hidden;
    animation: ccFadeIn 0.15s var(--ease);
  }
  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .palette {
      animation: none;
    }
  }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 54px;
    padding: 0 12px 0 18px;
    border-bottom: 1px solid var(--line);
  }
  .ic {
    display: grid;
    color: var(--muted);
  }
  .search input {
    flex: 1;
    min-width: 0;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 15px;
    outline: none;
  }
  .search input::placeholder {
    color: var(--muted);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 6px;
  }
  .list:empty {
    display: none;
  }
  .hit {
    border-radius: 7px;
    cursor: pointer;
  }
  .hit.on {
    background: rgba(var(--fg-rgb), 0.06);
  }
  .tag {
    flex: none;
    padding: 0 6px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 11px;
    line-height: 18px;
    color: var(--muted);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    padding: 0 18px;
    border-top: 1px solid var(--line);
    font-size: 11.5px;
    color: var(--muted);
  }
  .status {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status.error {
    color: var(--del);
  }
  .keys {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
  }
  .keys kbd {
    gap: 2px;
  }
  .sep {
    margin: 0 4px;
    color: var(--sep);
  }
  .hit {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 10px;
    white-space: nowrap;
  }
  .name {
    flex: none;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .dir {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--muted);
  }
  .tag {
    margin-left: auto;
  }
  .where {
    flex: none;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
    color: var(--muted);
  }
</style>
