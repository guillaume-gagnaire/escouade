<script lang="ts">
  import { untrack, type Snippet } from 'svelte';
  import { t } from '../../lib/i18n';
  import { tokenize, type Piece } from '../../lib/markdown';
  import { nearView } from '../../lib/nearview';
  import { CHUNK_LINES, LINE_PX, chunks, editorLine, foldReason, langOf, sides, statusLetter } from '../../lib/prfiles';
  import { tip } from '../../lib/tip';
  import type { PrContentsFile, PrLine } from '../../lib/types';
  import Icon from '../Icon.svelte';

  // A file of a pull request in the tab « Fichiers » (ESC-467): its head (fold, status, path, counts, « Ouvrir dans
  // l'éditeur », « Vu ») kept at the top while it scrolls, then its unified diff colored by its language. A binary, a
  // generated or a large file is folded until « Afficher »; a file marked « Vu » folds itself. Its lines are drawn by
  // parts as they come near the screen (`root`, the list's scrolling box), each part keeping its height before. `under`
  // draws what goes under a line (the threads of PR-4 and PR-5), after it, in the flow; `above` what goes above its lines
  // (its threads apart, ESC-474). `onComment` offers « + » on each line to comment it (Maj+clic: up to it). `reveal` opens
  // it and draws the part of a line, for « Suivante » to reach its thread.
  let {
    file,
    viewed,
    root,
    canOpen,
    onViewed,
    onOpen,
    under,
    above,
    openThreads = 0,
    marked,
    reveal = null,
    onComment,
  }: {
    file: PrContentsFile;
    viewed: boolean;
    root: HTMLElement | null;
    canOpen: boolean;
    onViewed: (on: boolean) => void;
    onOpen: (line: number | null) => void;
    under?: Snippet<[PrContentsFile, PrLine, number]>;
    above?: Snippet<[PrContentsFile]>;
    openThreads?: number;
    marked?: Record<number, boolean>;
    reveal?: { index: number; seq: number } | null;
    onComment?: (line: PrLine, index: number, extend: boolean) => void;
  } = $props();

  const reason = $derived(foldReason(file));
  /** Unfolded by « Afficher ». */
  let shown = $state(false);
  /** Folded by its chevron; a file marked « Vu » folds, unmarked unfolds. */
  let closed = $state(false);
  let seenViewed: boolean | undefined;
  $effect.pre(() => {
    const v = viewed;
    if (v !== seenViewed) closed = v;
    seenViewed = v;
  });

  /** The parts drawn: a part comes near, it is drawn, and stays. */
  let near = $state<Record<number, boolean>>({});
  const parts = $derived(chunks(file.lines));
  /** The lines a folded file says it hides: its own, not its hunks' heads. */
  const changedLines = $derived(file.lines.filter((l) => l.kind !== '@').length);
  const open = $derived(!closed && (!reason || (shown && reason !== 'binary')));

  /** Each line's colored pieces, once its file came near: null before, or for a language the app does not color. */
  let colored = $state<(Piece[] | null)[] | null>(null);
  /** The lines colored (or being colored): another diff of the file (a commit pushed) is colored again. */
  let coloring: PrLine[] | null = null;
  async function color() {
    const lang = langOf(file.path);
    const lines = file.lines;
    if (coloring === lines || !lang || lines.length > 3000) return;
    coloring = lines;
    const s = sides(lines);
    try {
      const [tip, base] = await Promise.all([tokenize(s.tip, lang), s.base.length ? tokenize(s.base, lang) : Promise.resolve([])]);
      if (!tip || !base || coloring !== lines) return;
      colored = s.at.map((a) => (a.i < 0 ? null : ((a.side === 'tip' ? tip : base)[a.i] ?? null)));
    } catch (e) {
      console.warn('diff colouring failed', e);
    }
  }

  function partNear(i: number) {
    near[i] = true;
    void color();
  }

  // Its lines changed (read again after a push): the old colours go, the new lines are colored once drawn.
  let seenText: string | null = null;
  $effect.pre(() => {
    const text = file.lines.map((l) => l.kind + l.text).join('\n');
    if (seenText !== null && text !== seenText) {
      colored = null;
      if (untrack(() => Object.keys(near).length)) void color();
    }
    seenText = text;
  });

  // « Suivante » reached a thread of this file: unfolded, the part of its line drawn, then the thread brought into view.
  let linesW = $state(0);
  let seenReveal = 0;
  $effect(() => {
    const r = reveal;
    if (!r || r.seq === seenReveal) return;
    seenReveal = r.seq;
    closed = false;
    shown = true;
    if (r.index >= 0) {
      const pi = Math.floor(r.index / CHUNK_LINES);
      if (!near[pi]) partNear(pi);
    }
  });

  const shownPath = $derived(file.previousPath && file.previousPath !== file.path ? `${file.previousPath} → ${file.path}` : file.path);
</script>

<section class="file" class:viewed data-path={file.path} aria-label={file.path}>
  <header class="fh">
    <button
      class="icon-btn chev"
      aria-expanded={!closed}
      aria-label={closed ? t('prs.page.files.unfold', { path: file.path }) : t('prs.page.files.fold', { path: file.path })}
      onclick={() => (closed = !closed)}><Icon name={closed ? 'right' : 'down'} size={13} /></button
    >
    <span class="st {file.status}" use:tip={t(`prs.page.files.status.${file.status}`)}>{statusLetter(file.status)}</span>
    <span class="p mono" use:tip={{ text: shownPath, cut: true }}>{shownPath}</span>
    {#if !file.binary}<span class="stat mono"><span class="add">+{file.additions}</span><span class="del">−{file.deletions}</span></span
      >{/if}
    {#if file.generated}<span class="tag">{t('prs.page.files.generatedTag')}</span>{/if}
    {#if openThreads}<span class="thc" aria-label={t('prs.page.threads.fileOpen', { count: openThreads, path: file.path })}
        ><Icon name="edit" size={11} />{openThreads}</span
      >{/if}
    <span class="sp"></span>
    {#if canOpen && file.status !== 'removed'}
      <button
        class="icon-btn"
        aria-label={t('prs.page.files.openFile', { path: file.path })}
        use:tip={t('prs.page.files.openFile', { path: file.path })}
        onclick={() => onOpen(null)}><Icon name="code" size={14} /></button
      >
    {/if}
    <label class="vu" class:on={viewed}>
      <input
        type="checkbox"
        checked={viewed}
        onchange={(e) => onViewed(e.currentTarget.checked)}
        aria-label={t('prs.page.files.viewedOf', { path: file.path })}
      />
      <span aria-hidden="true">{t('prs.page.files.viewed')}</span>
    </label>
  </header>

  {#if !closed}
    {#if above}{@render above(file)}{/if}
    {#if reason && !shown}
      <div class="fold">
        <span>{t(`prs.page.files.folded.${reason}`, { count: changedLines })}</span>
        {#if reason !== 'binary' && file.lines.length}
          <button class="btn" onclick={() => (shown = true)}>{t('prs.page.files.show')}</button>
        {/if}
      </div>
    {:else if !file.lines.length}
      <div class="fold">{t(file.binary ? 'prs.page.files.folded.binary' : 'prs.page.files.noLines')}</div>
    {:else if open}
      <div
        class="lines"
        role="table"
        aria-label={t('prs.page.files.linesOf', { path: file.path })}
        bind:clientWidth={linesW}
        style:--under-w="{linesW}px"
      >
        {#each parts as part, pi (pi)}
          {#if near[pi]}
            <div class="part">
              {#each part.lines as l, k (k)}
                {@const i = part.from + k}
                {#if l.kind === '@'}
                  <div class="row hunk" role="row"><span class="hh" role="cell">{l.text}</span></div>
                {:else}
                  {@const at = canOpen ? editorLine(file, i) : null}
                  <div
                    class="row {l.kind === '+' ? 'add' : l.kind === '-' ? 'del' : 'ctx'}"
                    class:marked={marked?.[i]}
                    role="row"
                    data-new={l.new}
                    data-old={l.old}
                  >
                    <span class="no" role="cell">{l.old ?? ''}</span>
                    {#if at}
                      <span class="no" role="cell"
                        ><button
                          class="ln"
                          tabindex="-1"
                          aria-label={t('prs.page.files.openLine', { path: file.path, line: at })}
                          onclick={() => onOpen(at)}>{l.new ?? ''}</button
                        ></span
                      >
                    {:else}
                      <span class="no" role="cell">{l.new ?? ''}</span>
                    {/if}
                    <span class="sg" role="cell"
                      >{#if onComment}<button
                          class="plus"
                          tabindex="-1"
                          aria-label={l.kind === '-'
                            ? t('prs.page.threads.commentLineRemoved', { line: l.old ?? '' })
                            : t('prs.page.threads.commentLine', { line: l.new ?? '' })}
                          use:tip={t('prs.page.threads.commentLineHint')}
                          onclick={(e) => onComment(l, i, e.shiftKey)}><Icon name="plus" size={11} /></button
                        >{/if}<span class="sgn">{l.kind === '+' ? '+' : l.kind === '-' ? '−' : ''}</span></span
                    >
                    <span class="tx" role="cell"
                      >{#if colored?.[i]}{#each colored[i]! as p, pi2 (pi2)}<span style:color={p.color}>{p.text}</span
                          >{/each}{:else}{l.text}{/if}</span
                    >
                  </div>
                {/if}
                {#if under}{@render under(file, l, i)}{/if}
              {/each}
            </div>
          {:else}
            <div class="part wait" style:height="{part.lines.length * LINE_PX}px" use:nearView={{ root, onNear: () => partNear(pi) }}></div>
          {/if}
        {/each}
      </div>
      {#if file.cut}<div class="fold cut">{t('prs.page.files.cut')}</div>{/if}
    {/if}
  {/if}
</section>

<style>
  .file {
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    background: var(--well);
    /* The head sticks to the top of the list: its corners are drawn by it, not cut. */
  }
  .fh {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 8px 0 4px;
    border-radius: 10px 10px 0 0;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 12px;
    min-width: 0;
  }
  .file:has(> .fh .chev[aria-expanded='false']) .fh {
    border-radius: 10px;
  }
  .chev {
    color: var(--muted);
  }
  .st {
    flex: none;
    width: 16px;
    text-align: center;
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 700;
    color: var(--wait);
  }
  .st.added {
    color: var(--add);
  }
  .st.removed {
    color: var(--del);
  }
  .st.renamed,
  .st.copied {
    color: var(--info);
  }
  .p {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 550;
  }
  .stat {
    flex: none;
    display: inline-flex;
    gap: 6px;
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }
  .tag {
    flex: none;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .sp {
    flex: 1;
  }
  .vu {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    border-radius: 6px;
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .vu:hover {
    color: var(--soft);
  }
  .vu.on {
    color: var(--run);
    box-shadow: inset 0 0 0 1px oklch(from var(--run) l c h / 0.4);
    background: oklch(from var(--run) l c h / 0.07);
  }
  .vu input {
    margin: 0;
    accent-color: var(--run);
  }
  .fold {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 18px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .fold.cut {
    padding: 10px;
    border-top: 1px solid var(--line);
  }
  .lines {
    padding: 4px 0 6px;
    overflow-x: auto;
    color: var(--code-fg);
    font-family: var(--mono);
    font-size: 12px;
    line-height: 20px;
    /* The colours of the syntax (lib/markdown.ts `tokenize`): the app's tokens of both themes. */
    --shiki-foreground: var(--code-fg);
    --shiki-background: transparent;
    --shiki-token-keyword: var(--syn-k);
    --shiki-token-function: var(--syn-f);
    --shiki-token-string: var(--syn-s);
    --shiki-token-string-expression: var(--syn-s);
    --shiki-token-constant: var(--syn-n);
    --shiki-token-parameter: var(--code-fg);
    --shiki-token-punctuation: var(--code-fg);
    --shiki-token-comment: var(--code-dim);
    --shiki-token-link: var(--ansi-cy);
  }
  .part {
    min-width: max-content;
  }
  .row {
    display: grid;
    grid-template-columns: 46px 46px 18px 1fr;
    height: 20px;
    white-space: pre;
  }
  .row.add {
    background: oklch(from var(--add) l c h / 0.09);
  }
  .row.del {
    background: oklch(from var(--del) l c h / 0.1);
  }
  .row.add .sg {
    color: var(--add);
  }
  .row.del .sg {
    color: var(--del);
  }
  .no {
    padding-right: 10px;
    text-align: right;
    color: var(--code-ln);
    user-select: none;
    font: inherit;
  }
  .ln {
    all: unset;
    color: var(--code-ln);
    cursor: pointer;
    user-select: none;
  }
  .ln:hover {
    color: var(--text);
    text-decoration: underline;
  }
  .row.add .no,
  .row.add .ln {
    background: oklch(from var(--add) l c h / 0.08);
  }
  .row.del .no {
    background: oklch(from var(--del) l c h / 0.09);
  }
  .sg {
    position: relative;
    text-align: center;
    user-select: none;
  }
  /* « + » to comment a line: shown on the row hovered, over its sign. */
  .plus {
    all: unset;
    position: absolute;
    inset: 1px 0;
    display: none;
    place-items: center;
    border-radius: 4px;
    color: var(--brain);
    background: oklch(from var(--brain) l c h / 0.22);
    box-shadow: inset 0 0 0 1px oklch(from var(--brain) l c h / 0.6);
    cursor: pointer;
  }
  .row:hover .plus,
  .plus:focus-visible {
    display: grid;
  }
  .row.marked {
    box-shadow: inset 2px 0 0 var(--brain);
  }
  .thc {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 650;
    color: var(--wait);
    box-shadow: inset 0 0 0 1px oklch(from var(--wait) l c h / 0.45);
  }
  .tx {
    padding-right: 16px;
  }
  .hunk {
    display: block;
    color: var(--info);
    background: oklch(from var(--info) l c h / 0.07);
  }
  .hh {
    padding-left: 110px;
  }
</style>
