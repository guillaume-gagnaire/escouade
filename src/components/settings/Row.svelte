<script lang="ts">
  import type { Snippet } from 'svelte';

  // A setting: its label (and hint), what it does, its control on the right, or under it when `wide`.
  let {
    label,
    hint = '',
    desc = '',
    descId,
    wide = false,
    children,
  }: { label: string; hint?: string; desc?: string; descId?: string; wide?: boolean; children?: Snippet } = $props();
</script>

<div class="row" class:wide>
  <div class="txt">
    <span class="l"
      >{label}{#if hint}&nbsp;<span class="h">{hint}</span>{/if}</span
    >
    {#if desc}<span class="d" id={descId}>{desc}</span>{/if}
  </div>
  {#if children}{@render children()}{/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
  }
  .row:not(:first-child) {
    border-top: 1px solid var(--line);
  }
  .row.wide {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }
  .txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .l {
    font-size: 13px;
    font-weight: 600;
  }
  .h {
    font-weight: 400;
    color: var(--dim);
  }
  .d {
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--muted);
    text-wrap: pretty;
    overflow-wrap: anywhere;
  }
</style>
