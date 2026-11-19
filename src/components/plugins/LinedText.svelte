<script lang="ts">
  import { lineCount } from '../../lib/plugin-fields';

  // A field on several lines (`multiline`, ESC-526): a text in the font of fixed width, its lines numbered in a gutter that
  // scrolls with it, `rows` lines visible. The lines are not wrapped: a line of a configuration stays one line.
  let {
    value = $bindable(),
    rows,
    label,
    onchange,
  }: { value?: string; rows: number; label: string; onchange?: (value: string) => void } = $props();

  let top = $state(0);
  const count = $derived(Math.max(rows, lineCount(value ?? '')));
</script>

<div class="lined" style:--rows={rows}>
  <div class="gutter" aria-hidden="true">
    <div style:transform="translateY({-top}px)">
      {#each { length: count } as _, i (i)}<span>{i + 1}</span>{/each}
    </div>
  </div>
  <textarea
    aria-label={label}
    spellcheck="false"
    autocomplete="off"
    wrap="off"
    {rows}
    bind:value
    onscroll={(e) => (top = e.currentTarget.scrollTop)}
    onchange={(e) => onchange?.(e.currentTarget.value)}></textarea>
</div>

<style>
  .lined {
    --lh: 18.6px;
    position: relative;
    display: flex;
    height: calc(var(--rows) * var(--lh) + 18px);
    flex: 1 0 100%;
    min-width: 0;
    border-radius: 7px;
    background: var(--well, var(--raise));
    box-shadow: inset 0 0 0 1px var(--line-2);
    font: 12px/var(--lh) var(--mono);
    overflow: hidden;
  }
  .lined:focus-within {
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--p) 60%, transparent),
      0 0 0 3px color-mix(in oklch, var(--p) 14%, transparent);
  }
  .gutter {
    flex: none;
    width: 34px;
    padding: 9px 0;
    overflow: hidden;
    color: var(--dim);
    text-align: right;
    user-select: none;
  }
  .gutter div {
    display: grid;
    padding-right: 10px;
  }
  .gutter span {
    height: var(--lh);
  }
  textarea {
    flex: 1 1 auto;
    min-width: 0;
    height: 100%;
    margin: 0;
    padding: 9px 11px 9px 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--soft);
    font: inherit;
    white-space: pre;
    resize: none;
    overflow: auto;
  }
</style>
