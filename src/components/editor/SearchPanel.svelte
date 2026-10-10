<script lang="ts">
  import { untrack } from 'svelte';
  import { dirOf } from '../../lib/editor/links';
  import { pieces, SEARCH_MAX, type FileSearch, type Piece, type SearchOption } from '../../lib/editor/search.svelte';
  import { basename, fInt } from '../../lib/format';
  import type { SearchMatch } from '../../lib/types';
  import FileIcon from './FileIcon.svelte';

  // « Rechercher dans les fichiers »: the field and its options, then what the search of the source shown found, file by
  // file, in a tree the keys walk as the files' one; a line found opens its file there.
  let { search, onopen }: { search: FileSearch; onopen: (m: SearchMatch) => void } = $props();

  const uid = $props.id();
  let field = $state<HTMLInputElement>();
  let scroller = $state<HTMLDivElement>();

  /** The field takes the focus, its text selected: what is typed replaces it. */
  export function focus() {
    field?.focus();
    field?.select();
  }

  const OPTIONS: { key: SearchOption; label: string; glyph: string }[] = [
    { key: 'caseSensitive', label: 'Respecter la casse', glyph: 'Aa' },
    { key: 'wholeWord', label: 'Mot entier', glyph: 'ab' },
    { key: 'regex', label: 'Expression régulière', glyph: '.*' },
  ];

  interface Line {
    m: SearchMatch;
    /** Its rank among all the lines found. */
    i: number;
    /** Its text as shown: cut once for each answer, not again at each fold. */
    ps: Piece[];
  }

  const result = $derived(search.result);
  /** The files found, in git's order, with their lines. */
  const files = $derived.by(() => {
    const by = new Map<string, Line[]>();
    (result?.matches ?? []).forEach((m, i) => {
      const line = { m, i, ps: pieces(m) };
      const lines = by.get(m.path);
      if (lines) lines.push(line);
      else by.set(m.path, [line]);
    });
    return [...by].map(([path, lines]) => ({ path, lines }));
  });

  /** The files folded: all open again with each answer. A set rather than keys of a `$state` object, which drops `constructor`. */
  let folded = $state.raw(new Set<string>());
  /** The row holding the focus: the tree's one Tab stop, and its row selected (the first row until one does). */
  let focused = $state<string | null>(null);
  $effect(() => {
    void result;
    untrack(() => {
      folded = new Set();
      focused = null;
      scroller?.scrollTo?.(0, 0);
    });
  });

  function fold(path: string) {
    const s = new Set(folded);
    if (!s.delete(path)) s.add(path);
    folded = s;
  }

  type Row = { kind: 'file'; key: string; path: string; count: number; open: boolean } | { kind: 'line'; key: string; line: Line };
  const rows = $derived(
    files.flatMap(({ path, lines }): Row[] => {
      const open = !folded.has(path);
      const head: Row = { kind: 'file', key: `f:${path}`, path, count: lines.length, open };
      return open ? [head, ...lines.map((line): Row => ({ kind: 'line', key: `l:${line.i}`, line }))] : [head];
    }),
  );
  const stop = $derived(focused && rows.some((r) => r.key === focused) ? focused : (rows[0]?.key ?? null));

  const count = (n: number, one: string, many: string) => `${fInt(n)} ${n > 1 ? many : one}`;
  const status = $derived.by(() => {
    if (search.error || !search.text) return null;
    if (!result) return search.pending ? 'Recherche…' : null;
    const n = result.matches.length;
    return n ? `${count(n, 'résultat', 'résultats')} dans ${count(files.length, 'fichier', 'fichiers')}` : 'Aucun résultat.';
  });
  const limit = $derived(
    result?.timedOut
      ? 'Recherche arrêtée après 10 s : résultats partiels.'
      : result?.truncated
        ? `Résultats limités aux ${fInt(SEARCH_MAX)} premiers.`
        : null,
  );

  const focusRow = (i: number) => scroller?.querySelector<HTMLElement>(`[data-index="${i}"]`)?.focus();

  // The keys of a tree (WAI-ARIA), as the files' tree has them: ↑ from the first row goes back to the field.
  function onkeydown(e: KeyboardEvent) {
    const i = Number((e.target as HTMLElement).dataset.index);
    const r = rows[i];
    if (!r) return;
    const move: Record<string, () => void> = {
      ArrowDown: () => i + 1 < rows.length && focusRow(i + 1),
      ArrowUp: () => (i > 0 ? focusRow(i - 1) : field?.focus()),
      Home: () => focusRow(0),
      End: () => focusRow(rows.length - 1),
      ArrowRight: () => {
        if (r.kind !== 'file') return;
        if (!r.open) fold(r.path);
        else if (rows[i + 1]?.kind === 'line') focusRow(i + 1);
      },
      ArrowLeft: () => {
        if (r.kind === 'file') return r.open && fold(r.path);
        let head = i - 1;
        while (head > 0 && rows[head].kind !== 'file') head--;
        focusRow(head);
      },
    };
    if (!Object.hasOwn(move, e.key)) return;
    e.preventDefault();
    move[e.key]();
  }

  function onfieldkey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.isComposing) {
      e.preventDefault();
      search.now();
    } else if (e.key === 'ArrowDown' && rows.length) {
      e.preventDefault();
      focusRow(0);
    }
  }
</script>

<div class="search">
  <div class="title"><span class="label">Recherche</span></div>
  <div class="box" class:invalid={!!search.error}>
    <input
      bind:this={field}
      value={search.text}
      oninput={(e) => search.type(e.currentTarget.value)}
      onkeydown={onfieldkey}
      placeholder="Rechercher"
      aria-label="Rechercher"
      aria-invalid={!!search.error}
      aria-describedby={search.error ? `${uid}-error` : undefined}
      spellcheck="false"
      autocomplete="off"
    />
    {#each OPTIONS as o (o.key)}
      <!-- The focus stays in the field when the mouse presses an option. -->
      <button
        class="opt mono"
        class:on={search[o.key]}
        aria-pressed={search[o.key]}
        aria-label={o.label}
        title={o.label}
        onmousedown={(e) => e.preventDefault()}
        onclick={() => search.toggle(o.key)}>{o.glyph}</button
      >
    {/each}
  </div>
  {#if search.error}
    <div class="problem" id="{uid}-error" role="alert">{search.error}</div>
  {/if}
  <div class="scroll" bind:this={scroller}>
    <!-- A click on the free space focuses the tree itself: its Tab stop takes the focus, for the arrows to work. -->
    <div
      class="results"
      class:stale={search.pending && !!result}
      role="tree"
      aria-label="Résultats"
      aria-busy={search.pending}
      tabindex="-1"
      {onkeydown}
      onfocus={(e) =>
        e.target === e.currentTarget &&
        e.currentTarget.querySelector<HTMLElement>('[role="treeitem"][tabindex="0"]')?.focus({ preventScroll: true })}
    >
      {#each rows as r, i (r.key)}
        {#if r.kind === 'file'}
          {@const dir = dirOf(r.path)}
          <button
            class="row file"
            role="treeitem"
            aria-level={1}
            aria-expanded={r.open}
            aria-label="{basename(r.path)}{dir ? `, ${dir}` : ''}, {count(r.count, 'résultat', 'résultats')}"
            aria-selected={r.key === stop}
            tabindex={r.key === stop ? 0 : -1}
            title={r.path}
            data-index={i}
            onfocus={() => (focused = r.key)}
            onclick={() => fold(r.path)}
          >
            <span class="twistie">
              <svg class:open={r.open} width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
                ><path
                  d="M6 4l4 4-4 4"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.3"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                /></svg
              >
            </span>
            <FileIcon path={r.path} />
            <span class="name">{basename(r.path)}</span>
            {#if dir}<span class="dir">{dir}</span>{/if}
            <span class="count mono">{r.count}</span>
          </button>
        {:else}
          {@const { m, ps } = r.line}
          <button
            class="row line"
            role="treeitem"
            aria-level={2}
            aria-label="Ligne {m.line} : {ps.map((p) => p.text).join('')}"
            aria-selected={r.key === stop}
            tabindex={r.key === stop ? 0 : -1}
            data-index={i}
            onfocus={() => (focused = r.key)}
            onclick={() => onopen(m)}
          >
            <span class="text mono"
              >{#each ps as p, j (j)}{#if p.hit}<mark>{p.text}</mark>{:else}{p.text}{/if}{/each}</span
            >
          </button>
        {/if}
      {/each}
    </div>
  </div>
  {#if status}
    <div class="foot" role="status">
      <span>{status}</span>
      {#if limit}<span class="limit">{limit}</span>{/if}
    </div>
  {/if}
</div>

<style>
  .search {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .title {
    padding: 10px 14px 8px 16px;
  }
  .label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .box {
    margin: 0 12px 0 14px;
    height: 28px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 0 3px 0 0;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--term);
  }
  .box:focus-within {
    border-color: var(--accent);
  }
  .box.invalid {
    border-color: var(--del);
  }
  .box input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0 8px;
    border: none;
    background: transparent;
    font-size: 13px;
    outline: none;
  }
  .opt {
    width: 22px;
    height: 20px;
    flex: none;
    display: grid;
    place-items: center;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 3px;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
  }
  .opt:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .opt.on {
    border-color: var(--accent);
    background: var(--elev2);
    color: var(--text);
  }
  .problem {
    margin: 0 12px 0 14px;
    padding: 5px 8px;
    border: 1px solid var(--del);
    border-top: none;
    border-radius: 0 0 var(--r-sm) var(--r-sm);
    background: oklch(0.3 0.06 25);
    color: var(--text);
    font-size: 12px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    margin-top: 8px;
    overflow: auto;
  }
  .results {
    min-height: 100%;
    display: flex;
    flex-direction: column;
    padding: 2px 0 12px;
    outline: none;
  }
  /* The answer to come replaces these. */
  .results.stale {
    opacity: 0.55;
  }
  .row {
    width: 100%;
    flex: none;
    height: 22px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 10px 0 8px;
    border: none;
    border-radius: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
    /* Thousands of rows: those out of view cost no layout. */
    content-visibility: auto;
    contain-intrinsic-size: auto 22px;
  }
  .row:hover {
    background: var(--elev);
  }
  .row:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  .row.line {
    padding-left: 44px;
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
    flex: none;
    max-width: 70%;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dir {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 11.5px;
    color: var(--dim);
  }
  .count {
    flex: none;
    margin-left: auto;
    min-width: 18px;
    padding: 0 5px;
    border-radius: 99px;
    background: var(--elev2);
    color: var(--muted);
    font-size: 10.5px;
    line-height: 16px;
    text-align: center;
  }
  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--muted);
  }
  mark {
    background: color-mix(in oklch, var(--wait) 28%, transparent);
    color: var(--text);
    border-radius: 2px;
  }
  .foot {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 7px 14px 8px 16px;
    border-top: 1px solid var(--line);
    font-size: 11px;
    color: var(--dim);
  }
  .limit {
    color: var(--wait);
  }
  @media (prefers-reduced-motion: reduce) {
    .twistie svg {
      transition: none;
    }
  }
</style>
