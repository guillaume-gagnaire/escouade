<script lang="ts">
  import { hueOf, markOf } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';

  // A plugin's tile (choice 219): its logo when it has one, else its two letters on a hue drawn from its id, as the
  // store's tile (store/app/components/PluginTile.vue). `src`: the logo given (a plugin read before it is installed);
  // `installed`: the logo of an installed plugin, read once by `plugin_icon` (PLG-13). Says nothing to a screen reader:
  // its row has the name.
  let {
    id,
    name,
    src,
    installed = false,
    size = 40,
  }: { id: string; name: string; src?: string | null; installed?: boolean; size?: number } = $props();
  let broken = $state(false);
  $effect(() => {
    if (installed && src === undefined) void plugins.loadIcon(id);
  });
  const logo = $derived(src !== undefined ? src : installed ? plugins.icons[id] : null);
</script>

<span class="tile" style:--s="{size}px" style:--h={hueOf(id)} aria-hidden="true">
  {#if logo && !broken}
    <img src={logo} alt="" draggable="false" onerror={() => (broken = true)} />
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
