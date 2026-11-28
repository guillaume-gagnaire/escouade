<script lang="ts">
  // An image of a request (MO-5), read from its space's core once (`inbox_file`), shown as soon as it came.
  import type { AskFile, WindowAsk } from '$shared/lib/types';
  import { imageUrl } from '../lib/act';

  let { space, ask, file, alt = '' }: { space: string; ask: WindowAsk; file: AskFile; alt?: string } = $props();
  let src = $state<string | null>(null);

  $effect(() => {
    let live = true;
    src = null;
    imageUrl(space, ask, file)
      .then((u) => live && (src = u))
      .catch(() => {});
    return () => (live = false);
  });
</script>

{#if src}<img {src} {alt} draggable="false" />{:else}<span class="ph" aria-hidden="true"></span>{/if}

<style>
  img,
  .ph {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: top left;
  }
  .ph {
    background: rgba(var(--fg-rgb), 0.05);
  }
</style>
