<script lang="ts" module>
  /** The icons the core gave, by their address on Open VSX, for the page's life. */
  const given = new Map<string, string>();
</script>

<script lang="ts">
  // The icon of an extension of Open VSX (ESC-481): asked of the core when it shows (`vsx_image`, a `data:` URL of an
  // image, from the registry alone), never by the page; its sign until then, or when there is none.
  import { api } from '../../lib/ipc';
  import Icon from '../Icon.svelte';

  let { url, projects, off = false, size = 20 }: { url: string | null; projects: string[]; off?: boolean; size?: number } = $props();
  let src = $state<string | null>(null);

  $effect(() => {
    const u = url;
    src = u ? (given.get(u) ?? null) : null;
    if (!u || off || src) return;
    let live = true;
    api
      .vsxImage(projects, u)
      .then((out) => {
        if (!out?.value) return;
        given.set(u, out.value);
        if (live) src = out.value;
      })
      // Without its icon, the card shows its sign: nothing to say.
      .catch(() => {});
    return () => {
      live = false;
    };
  });
</script>

<span class="logo" class:small={size < 20}>
  {#if src}<img {src} alt="" />{:else}<Icon name="blocks" {size} />{/if}
</span>

<style>
  .logo {
    display: grid;
    place-items: center;
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: 8px;
    background: var(--panel);
    color: var(--muted);
    overflow: hidden;
  }
  .logo.small {
    width: 28px;
    height: 28px;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
</style>
