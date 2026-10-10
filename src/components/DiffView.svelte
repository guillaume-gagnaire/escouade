<script lang="ts">
  import { splitRows, type DiffLine } from '../lib/diff';

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
</script>

{#if tooLarge}
  <p class="note">Diff trop volumineux pour être affiché.</p>
{:else if long && drawn === 0}
  <div class="note">
    <span>Diff volumineux ({lines.length} lignes)</span>
    <button class="btn" onclick={() => (drawn = SLICE)}>Afficher</button>
  </div>
{:else if split}
  <!-- Side by side wraps long lines so both columns stay aligned in narrow panes. -->
  <div class="rows">
    {#each rows.slice(0, shown) as r, i (i)}
      <div class="srow">
        {#each [r.left, r.right] as l, side (side)}
          <div class="cell {l ? (l.kind === 'ctx' ? '' : l.kind) : 'void'}">
            <span class="no">{l && l.kind !== 'meta' ? (side === 0 ? (l.oldNo ?? '') : (l.newNo ?? '')) : ''}</span>
            <span class="txt">{l?.text ?? ''}</span>
          </div>
        {/each}
      </div>
    {/each}
  </div>
{:else}
  <div class="rows unified">
    {#each lines.slice(0, shown) as l, i (i)}
      <div class="urow {l.kind}">
        <span class="no">{l.oldNo ?? ''}</span>
        <span class="no">{l.newNo ?? ''}</span>
        <span class="sign">{l.kind === 'add' ? '+' : l.kind === 'del' ? '−' : ''}</span>
        <span class="txt">{l.text}</span>
      </div>
    {/each}
  </div>
{/if}
{#if long && drawn > 0 && rest > 0}
  <div class="note more">
    <button class="btn" onclick={() => (drawn += SLICE)}>Afficher {Math.min(SLICE, rest)} lignes de plus</button>
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
  }
  .note.more {
    padding: 14px;
  }
  .rows {
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.55;
  }
  .rows.unified {
    min-width: max-content;
  }
  .urow {
    display: flex;
    white-space: pre;
  }
  .no {
    width: 48px;
    flex: none;
    padding-right: 8px;
    text-align: right;
    color: var(--dim);
    user-select: none;
  }
  .sign {
    width: 18px;
    flex: none;
    text-align: center;
    user-select: none;
  }
  .txt {
    padding-right: 20px;
  }
  .urow.add,
  .cell.add {
    background: color-mix(in oklch, var(--add) 14%, transparent);
  }
  .urow.del,
  .cell.del {
    background: color-mix(in oklch, var(--del) 14%, transparent);
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
    background: color-mix(in oklch, var(--info) 8%, transparent);
  }
  .srow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }
  .cell {
    display: flex;
    min-width: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    border-right: 1px solid var(--line);
  }
  .cell .txt {
    flex: 1;
    min-width: 0;
    padding-right: 12px;
  }
  .cell.void {
    background: repeating-linear-gradient(135deg, transparent 0 6px, rgba(255, 255, 255, 0.02) 6px 12px);
  }
</style>
