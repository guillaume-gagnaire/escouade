<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { editorJump, parseQuickQuery, QUICK_OPEN_MAX, rankFiles, recentFiles } from '../lib/editor/quick-open';
  import { trees } from '../lib/editor/trees.svelte';
  import { trapFocus } from '../lib/focus';
  import { basename, fInt } from '../lib/format';
  import { t } from '../lib/i18n';
  import { keyLabel } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import FileIcon from './editor/FileIcon.svelte';

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
    const shown = app.editorOn && app.project?.id === projectId && ed?.source === source ? ed.places[source]?.active : null;
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

  // The chosen file stays in view as the arrows move through a long list.
  $effect(() => {
    void shown;
    list?.querySelector(`#${optionId(cursor)}`)?.scrollIntoView?.({ block: 'nearest' });
  });

  const optionId = (rank: number) => `quick-open-${rank}`;
  const folder = (path: string) => path.slice(0, Math.max(0, path.lastIndexOf('/')));

  function close() {
    app.modal = null;
  }

  function move(by: number) {
    const n = shown.length;
    if (n) active = (cursor + by + n) % n;
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
    const inEditor = app.editorOn && app.project?.id === pid && app.editor[pid]?.source === src;
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
      <span class="ic" aria-hidden="true">⌕</span>
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
        oninput={() => (active = 0)}
        onkeydown={onKeydown}
      />
      <span class="where" title={place}>{place}</span>
      <button class="icon-btn" style="width:28px;height:28px;font-size:16px" onclick={close} aria-label={t('common.close')}>×</button>
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
          onmousemove={() => (active = rank)}
        >
          <FileIcon {path} />
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
        ><kbd>↑</kbd><kbd>↓</kbd>
        {t('editor.quickOpen.keyChoose')} <span class="sep">·</span> <kbd>{keyLabel('Enter')}</kbd>
        {t('editor.quickOpen.keyOpen')} <span class="sep">·</span>
        <kbd>{keyLabel('Esc')}</kbd>
        {t('editor.quickOpen.keyClose')}</span
      >
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    background: rgba(12, 10, 8, 0.6);
    backdrop-filter: blur(3px);
    animation: fade 0.12s ease-out;
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
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 14px;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55);
    overflow: hidden;
    animation: ccFadeIn 0.15s ease-out;
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
    padding: 12px 12px 12px 18px;
    border-bottom: 1px solid var(--line);
  }
  .ic {
    color: var(--dim);
    font-size: 16px;
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
    color: var(--dim);
  }
  .where {
    flex: none;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
    color: var(--dim);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 6px 8px;
  }
  .list:empty {
    display: none;
  }
  .hit {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: var(--r-sm);
    cursor: pointer;
    white-space: nowrap;
  }
  .hit.on {
    background: var(--elev2);
  }
  .name {
    flex: none;
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 13px;
    font-weight: 700;
    color: var(--text);
  }
  .dir {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--dim);
  }
  .hit.on .dir {
    color: var(--muted);
  }
  .tag {
    flex: none;
    margin-left: auto;
    padding: 0 6px;
    border-radius: 99px;
    border: 1px solid var(--line2);
    font-size: 10.5px;
    color: var(--dim);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 18px;
    border-top: 1px solid var(--line);
    font-size: 11.5px;
    color: var(--dim);
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
    white-space: nowrap;
  }
  kbd {
    font-family: var(--mono);
    font-size: 10.5px;
    padding: 0 4px;
    margin-right: 2px;
    border: 1px solid var(--line2);
    border-radius: 4px;
    color: var(--muted);
  }
  .sep {
    margin: 0 4px;
  }
</style>
