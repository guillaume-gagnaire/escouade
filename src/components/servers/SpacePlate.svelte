<script lang="ts">
  // The plate of a space (ESC-537, S6): a screen for this computer, a letter for a server; neutral, the colour stays the
  // projects'. Amber while its connection is cut.
  import { plateOf } from '../../lib/plate';
  import Icon from '../Icon.svelte';

  let { name, here = false, cut = false, size = 20 }: { name: string; here?: boolean; cut?: boolean; size?: number } = $props();
</script>

<span class="plate" class:cut style:--s="{size}px" aria-hidden="true">
  {#if here}<Icon name="monitor" size={Math.round(size * 0.6)} />{:else}{plateOf(name)}{/if}
</span>

<style>
  .plate {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: var(--s);
    height: var(--s);
    border-radius: calc(var(--s) * 0.3);
    background: var(--raise-2);
    color: var(--text);
    font-size: calc(var(--s) * 0.55);
    font-weight: 700;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .plate.cut {
    color: var(--wait);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
</style>
