<script lang="ts">
  import { tick, untrack } from 'svelte';
  import type { EntryKind } from '../../lib/editor/create';
  import { isDeleteKey, type FileStatus, type TreeRow } from '../../lib/editor/tree';
  import { t } from '../../lib/i18n';
  import FileIcon from './FileIcon.svelte';
  import NewFileField from './NewFileField.svelte';

  let {
    rows,
    active,
    ontoggle,
    onopen,
    onmenu,
    check = () => null,
    oncreate = async () => null,
    oncancel = () => {},
    adding = 'file',
    renaming = null,
    renameCheck = () => null,
    onrename = async () => null,
    onrenamecancel = () => {},
    onrenamerow,
    ondeleterow,
  }: {
    rows: TreeRow[];
    active: string | null;
    ontoggle: (dir: string) => void;
    onopen: (path: string) => void;
    /** A right click on a row, or on the free space below them (`row` null). */
    onmenu?: (e: MouseEvent, row: TreeRow | null) => void;
    /** For the `new` row: why a name cannot be created, creating it, giving it up. */
    check?: (name: string) => string | null;
    oncreate?: (name: string) => Promise<string | null>;
    oncancel?: () => void;
    /** What the `new` row names: a file or a folder. */
    adding?: EntryKind;
    /** The file or folder renamed, its row a field holding its name: why a name cannot be taken, renaming, giving up. */
    renaming?: { kind: EntryKind; path: string } | null;
    renameCheck?: (name: string) => string | null;
    onrename?: (name: string) => Promise<string | null>;
    onrenamecancel?: () => void;
    /** F2 on a row, and Delete (Cmd+Backspace too on macOS). */
    onrenamerow?: (row: TreeRow) => void;
    ondeleterow?: (row: TreeRow) => void;
  } = $props();

  // As VS Code draws them: 8 px from the edge, 12 px a level, a 16 px chevron whose middle the level's guide goes through.
  const PAD = 8;
  const INDENT = 12;
  const pad = (depth: number) => PAD + depth * INDENT;
  const COLOR: Record<FileStatus, string> = { M: 'var(--wait)', A: 'var(--add)', D: 'var(--del)' };

  let el = $state<HTMLDivElement>();
  /** The row holding the focus, the tree's one Tab stop; the file shown or the first row until one does. */
  let focused = $state<string | null>(null);
  const keyOf = (r: TreeRow) => `${r.kind}:${r.path}`;
  /** The row of the file or folder renamed: a field, not a row to go to. */
  const renamed = (r: TreeRow) => !!renaming && r.kind === renaming.kind && r.path === renaming.path;
  const isField = (r: TreeRow) => r.kind === 'new' || renamed(r);
  const stop = $derived.by(() => {
    const keys = rows.filter((r) => !isField(r)).map(keyOf);
    if (focused && keys.includes(focused)) return focused;
    return active && keys.includes(`file:${active}`) ? `file:${active}` : (keys[0] ?? null);
  });

  const focusRow = (i: number) => el?.querySelector<HTMLElement>(`[data-index="${i}"]`)?.focus();

  /**
   * Puts the focus on the row of the file or folder `path` once the rows given are drawn (a field closed, a file
   * renamed or made), else on the tree's Tab stop: the row is gone.
   */
  export async function focusPath(kind: EntryKind, path: string) {
    await tick();
    const i = rows.findIndex((r) => r.kind === kind && r.path === path);
    if (i >= 0) return focusRow(i);
    el?.querySelector<HTMLElement>('[role="treeitem"][tabindex="0"]')?.focus();
  }

  // The keys of a tree (WAI-ARIA), as VS Code's explorer has them; Enter and Space are the buttons' own.
  function onkeydown(e: KeyboardEvent) {
    const i = Number((e.target as HTMLElement).dataset.index);
    const r = rows[i];
    if (!r) return;
    // F2 alone, as Delete: with a modifier, it is another shortcut.
    const f2 = e.key === 'F2' && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey;
    if (r.kind !== 'new' && (f2 || isDeleteKey(e))) {
      e.preventDefault();
      return f2 ? onrenamerow?.(r) : ondeleterow?.(r);
    }
    const shown = rows.flatMap((row, j) => (isField(row) ? [] : [j]));
    const at = shown.indexOf(i);
    const move: Record<string, () => void> = {
      ArrowDown: () => at + 1 < shown.length && focusRow(shown[at + 1]),
      ArrowUp: () => at > 0 && focusRow(shown[at - 1]),
      Home: () => focusRow(shown[0]),
      End: () => focusRow(shown[shown.length - 1]),
      ArrowRight: () => {
        if (r.kind !== 'dir') return;
        if (!r.open) ontoggle(r.path);
        else if (rows[shown[at + 1]]?.depth > r.depth) focusRow(shown[at + 1]);
      },
      ArrowLeft: () => {
        if (r.kind === 'dir' && r.open) return ontoggle(r.path);
        const parent = shown
          .slice(0, at)
          .reverse()
          .find((j) => rows[j].kind === 'dir' && rows[j].depth < r.depth);
        if (parent !== undefined) focusRow(parent);
      },
    };
    if (!Object.hasOwn(move, e.key)) return;
    e.preventDefault();
    move[e.key]();
  }

  // The file shown is brought into view, as VS Code's explorer reveals it; not again when other folders open.
  $effect(() => {
    const a = active;
    if (!a) return;
    untrack(() => {
      const i = rows.findIndex((r) => r.kind === 'file' && r.path === a);
      if (i >= 0) el?.querySelector<HTMLElement>(`[data-index="${i}"]`)?.scrollIntoView?.({ block: 'nearest' });
    });
  });
</script>

{#snippet chevron(open: boolean)}
  <svg class:open width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
    ><path d="M6 4l4 4-4 4" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" /></svg
  >
{/snippet}

<!-- A click on the free space focuses the tree itself: its Tab stop takes the focus, for the arrows to work. -->
<div
  class="tree"
  role="tree"
  aria-label={t('common.files')}
  tabindex="-1"
  bind:this={el}
  {onkeydown}
  onfocus={(e) => e.target === el && el.querySelector<HTMLElement>('[role="treeitem"][tabindex="0"]')?.focus({ preventScroll: true })}
  oncontextmenu={(e) => onmenu?.(e, null)}
>
  {#each rows as r, i (keyOf(r))}
    {#snippet guides()}
      {#each { length: r.depth } as _, level (level)}
        <span class="guide" style:left="{pad(level) + 8}px"></span>
      {/each}
    {/snippet}
    {#if isField(r)}
      {@const kind = r.kind === 'new' ? adding : r.kind}
      <div
        class="row new"
        role="none"
        style:padding-left="{pad(r.depth)}px"
        style:--field-left="{pad(r.depth) + (kind === 'file' ? 40 : 20)}px"
        oncontextmenu={(e) => e.stopPropagation()}
      >
        {@render guides()}
        <span class="twistie"
          >{#if kind === 'dir'}{@render chevron(r.kind === 'dir' && r.open)}{/if}</span
        >
        {#if r.kind === 'new'}
          <NewFileField {check} onsubmit={oncreate} {oncancel} {kind} />
        {:else}
          {@const name = r.path.slice(r.path.lastIndexOf('/') + 1)}
          <NewFileField
            check={renameCheck}
            onsubmit={onrename}
            oncancel={onrenamecancel}
            {kind}
            value={name}
            label={t('editor.newField.rename', { name })}
          />
        {/if}
      </div>
    {:else}
      {@const color = r.status ? COLOR[r.status] : r.inside ? COLOR[r.inside] : undefined}
      <button
        class="row"
        class:on={r.kind === 'file' && r.path === active}
        class:ignored={r.ignored}
        role="treeitem"
        aria-level={r.depth + 1}
        aria-expanded={r.kind === 'dir' ? r.open : undefined}
        aria-selected={r.kind === 'file' && r.path === active}
        tabindex={keyOf(r) === stop ? 0 : -1}
        title={r.ignored ? t('editor.tree.ignoredByGit') : r.path}
        data-index={i}
        style:padding-left="{pad(r.depth)}px"
        onfocus={() => (focused = keyOf(r))}
        onclick={() => (r.kind === 'dir' ? ontoggle(r.path) : onopen(r.path))}
        oncontextmenu={(e) => {
          e.stopPropagation();
          onmenu?.(e, r);
        }}
      >
        {@render guides()}
        <span class="twistie"
          >{#if r.kind === 'dir'}{@render chevron(r.open)}{/if}</span
        >
        {#if r.kind === 'file'}<FileIcon path={r.path} />{/if}
        <span class="name" style:color>{r.name}</span>
        {#if r.status}
          <span class="st mono" style:color>{r.status}</span>
        {:else if r.inside}
          <span class="dot" style:background={color} aria-hidden="true"></span>
        {/if}
      </button>
    {/if}
  {/each}
</div>

<style>
  .tree {
    min-height: 100%;
    display: flex;
    flex-direction: column;
    padding: 2px 0 12px;
    outline: none;
  }
  .row {
    position: relative;
    width: 100%;
    flex: none;
    height: 22px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding-right: 10px;
    border: none;
    border-radius: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }
  .row:hover {
    background: var(--elev);
  }
  /* A file git ignores, shown because the project copies it into its worktrees: faded as in VS Code's explorer. */
  .row.ignored {
    color: var(--dim);
  }
  .row.ignored :global(.ficon) {
    opacity: 0.55;
  }
  .row.on {
    background: var(--elev2);
  }
  .row:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  .row.new {
    cursor: default;
  }
  .row.new:hover {
    background: transparent;
  }
  .guide {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--line);
    pointer-events: none;
  }
  .tree:hover .guide {
    background: var(--line2);
  }
  .twistie {
    width: 16px;
    height: 16px;
    flex: none;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .twistie svg {
    transition: transform 0.1s ease-out;
  }
  .twistie svg.open {
    transform: rotate(90deg);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .st {
    flex: none;
    font-size: 11px;
    font-weight: 600;
  }
  .dot {
    width: 6px;
    height: 6px;
    flex: none;
    margin-right: 2px;
    border-radius: 50%;
  }
  @media (prefers-reduced-motion: reduce) {
    .twistie svg {
      transition: none;
    }
  }
</style>
