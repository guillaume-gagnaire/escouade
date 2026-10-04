<script lang="ts">
  import { fileIcon } from '../../lib/editor/icons';

  let { path }: { path: string } = $props();

  const icon = $derived(fileIcon(path));
</script>

<span class="ficon mono" class:long={icon.glyph.length > 2} aria-hidden="true" style:color={icon.color}>{icon.glyph}</span>

<style>
  .ficon {
    width: 16px;
    flex: none;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: -0.04em;
    text-align: center;
    line-height: 1;
  }
  .long {
    font-size: 7.5px;
    letter-spacing: -0.08em;
  }
</style>
