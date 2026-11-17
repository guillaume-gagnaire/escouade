<script lang="ts">
  import { inbox } from '../../lib/inbox/inbox.svelte';
  import type { AskFile, WindowAsk } from '../../lib/types';

  // An image of a request at its size in its room (a plate, the comparison): read from the core's copy once.
  let { ask, file, alt }: { ask: Pick<WindowAsk, 'id'>; file: AskFile; alt: string } = $props();

  let url = $state<string | null>(null);
  $effect(() => {
    let live = true;
    url = null;
    inbox
      .imageUrl(ask, file)
      .then((u) => live && (url = u))
      .catch(() => {});
    return () => {
      live = false;
    };
  });
</script>

{#if url}
  <img src={url} {alt} draggable="false" data-file={file.file} />
{:else}
  <span class="ph" aria-hidden="true"></span>
{/if}

<style>
  .ph {
    display: block;
    aspect-ratio: 16 / 10;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.04);
  }
</style>
