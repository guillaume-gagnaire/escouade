<script lang="ts">
  import { hueOf, markOf } from '../../lib/plugins';

  // A plugin's tile: its logo when it has one (`src`, given by ESC-292's command), else its two letters on a hue drawn
  // from its id, as the store's tile (store/app/components/PluginTile.vue). A plugin shipped with Escouade has letters
  // too, until ESC-292 chooses its mark. Says nothing to a screen reader: its row has the name.
  let { id, name, src, size = 40 }: { id: string; name: string; src?: string | null; size?: number } = $props();
  let broken = $state(false);
</script>

<span class="tile" style:--s="{size}px" style:--h={hueOf(id)} aria-hidden="true">
  {#if src && !broken}
    <img {src} alt="" onerror={() => (broken = true)} />
  {:else}
    {markOf(name)}
  {/if}
</span>

<style>
  .tile {
    display: grid;
    flex: none;
    place-items: center;
    width: var(--s);
    height: var(--s);
    overflow: hidden;
    border-radius: calc(var(--s) * 0.26);
    background: oklch(0.36 0.07 var(--h));
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    color: oklch(0.93 0.05 var(--h));
    font-size: calc(var(--s) * 0.36);
    font-weight: 680;
    letter-spacing: -0.02em;
    user-select: none;
  }
  :global([data-theme='light']) .tile {
    background: oklch(0.9 0.05 var(--h));
    box-shadow: inset 0 0 0 1px rgba(15, 17, 22, 0.06);
    color: oklch(0.35 0.09 var(--h));
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
