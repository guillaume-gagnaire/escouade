<script lang="ts">
  import type { Snippet } from 'svelte';

  // A titled box of settings rows (or, `plain`, what is laid out by itself), with a note under it; `anchor`:
  // what the modal scrolls to when opened on it.
  let {
    title,
    note,
    plain = false,
    anchor,
    children,
  }: { title: string; note?: string; plain?: boolean; anchor?: string; children: Snippet } = $props();
</script>

<section class="group" data-anchor={anchor}>
  <h3>{title}</h3>
  {#if plain}{@render children()}{:else}<div class="box">{@render children()}</div>{/if}
  {#if note}<p class="note">{note}</p>{/if}
</section>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h3 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--dim);
  }
  .box {
    display: flex;
    flex-direction: column;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--bg);
  }
  .note {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--dim);
    text-wrap: pretty;
  }
</style>
