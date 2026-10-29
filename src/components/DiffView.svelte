<script lang="ts">
  import { tick } from 'svelte';
  import { clipLine, splitRows, type DiffLine } from '../lib/diff';
  import { t } from '../lib/i18n';

  // Diff lines, unified (merged, red and green lines) or side by side. Each line is a DOM node, so a
  // file of more than FOLD_AT lines stays folded until asked for, then is drawn SLICE lines at a time.
  // `tooLarge`: the backend did not send this file's diff.
  let { lines, split, tooLarge = false }: { lines: DiffLine[]; split: boolean; tooLarge?: boolean } = $props();

  const FOLD_AT = 1500;
  const SLICE = 500;

  // How much of a long diff is drawn: nothing while it is folded. Kept when the diff is refreshed,
  // so that a file being edited does not fold itself back under the reader.
  let drawn = $state(0);
  const long = $derived(lines.length > FOLD_AT);
  // Side by side pairs a deletion with its addition: slicing rows, not lines, never tears a pair.
  const rows = $derived(split && !tooLarge ? splitRows(lines) : []);
  const total = $derived(split ? rows.length : lines.length);
  const shown = $derived(long ? drawn : total);
  const rest = $derived(total - shown);

  let moreButton = $state<HTMLButtonElement>();
  let rowsBox = $state<HTMLElement>();

  // The button pressed goes away with the fold, or with the last slice: the focus moves on to the
  // next slice's button, else to the rows, so that the keyboard neither falls back to the page (out
  // of a modal's focus trap) nor starts over. Without scrolling: the reader stays where they were.
  async function draw(upTo: number) {
    drawn = upTo;
    await tick();
    (moreButton ?? rowsBox)?.focus({ preventScroll: true });
  }
</script>

<!-- A line drawn up to LINE_SHOWN characters, then how many it leaves out (ESC-290). -->
{#snippet lineText(text: string)}{@const c = clipLine(text)}<span class="txt"
    >{c.text}{#if c.cut}<span class="cut">{t('git.diff.lineCut', { count: c.cut })}</span>{/if}</span
  >{/snippet}

{#if tooLarge}
  <p class="note">{t('git.diff.tooLarge')}</p>
{:else if long && drawn === 0}
  <div class="note">
    <span>{t('git.diff.large', { n: lines.length })}</span>
    <button class="btn" onclick={() => draw(SLICE)}>{t('common.show')}</button>
  </div>
{:else if split}
  <!-- Side by side wraps long lines so both columns stay aligned in narrow panes. -->
  <div class="rows" tabindex="-1" bind:this={rowsBox}>
    {#each rows.slice(0, shown) as r, i (i)}
      <div class="srow">
        {#each [r.left, r.right] as l, side (side)}
          <div class="cell {l ? (l.kind === 'ctx' ? '' : l.kind) : 'void'}">
            <span class="no">{l && l.kind !== 'meta' ? (side === 0 ? (l.oldNo ?? '') : (l.newNo ?? '')) : ''}</span>
            {@render lineText(l?.text ?? '')}
          </div>
        {/each}
      </div>
    {/each}
  </div>
{:else}
  <div class="rows unified" tabindex="-1" bind:this={rowsBox}>
    {#each lines.slice(0, shown) as l, i (i)}
      <div class="urow {l.kind}">
        <!-- One number, as the preview draws it: the old one of a line removed, the new one of the others. -->
        <span class="no">{(l.kind === 'del' ? l.oldNo : l.newNo) ?? ''}</span>
        <span class="sign">{l.kind === 'add' ? '+' : l.kind === 'del' ? '−' : ''}</span>
        {@render lineText(l.text)}
      </div>
    {/each}
  </div>
{/if}
{#if long && drawn > 0 && rest > 0}
  <div class="note more">
    <button class="btn" bind:this={moreButton} onclick={() => draw(drawn + SLICE)}
      >{t('git.diff.showMore', { count: Math.min(SLICE, rest) })}</button
    >
  </div>
{/if}

<style>
  .note {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    margin: 0;
    padding: 40px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .note.more {
    padding: 14px;
  }
  /* The lines of a diff as the preview draws them (`.code.diff`): mono 12 px on 1.7, a gutter of numbers, the sign. */
  .rows {
    padding: 12px 0;
    color: var(--code-fg);
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.7;
  }
  .rows:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .rows.unified {
    min-width: max-content;
  }
  .urow {
    display: grid;
    grid-template-columns: 34px 16px 1fr;
    padding: 0 14px;
    white-space: pre;
  }
  .no {
    padding-right: 10px;
    text-align: right;
    color: var(--code-ln);
    user-select: none;
  }
  .sign {
    user-select: none;
  }
  .urow.add,
  .cell.add {
    background: oklch(from var(--add) l c h / 0.08);
  }
  .urow.del,
  .cell.del {
    background: oklch(from var(--del) l c h / 0.09);
  }
  .urow.add .sign {
    color: var(--add);
  }
  .urow.del .sign {
    color: var(--del);
  }
  .urow.meta,
  .cell.meta {
    color: var(--info);
    background: oklch(from var(--info) l c h / 0.07);
  }
  .srow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }
  .cell {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .cut {
    padding-left: 6px;
    color: var(--muted);
    font-family: var(--font);
    user-select: none;
  }
  .cell + .cell {
    box-shadow: inset 1px 0 0 var(--line);
  }
  .cell .txt {
    padding-right: 12px;
  }
  .cell.void {
    background: repeating-linear-gradient(135deg, transparent 0 6px, var(--hover) 6px 12px);
  }
</style>
