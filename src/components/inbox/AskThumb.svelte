<script lang="ts">
  import { inbox } from '../../lib/inbox/inbox.svelte';
  import type { AskFile, WindowAsk } from '../../lib/types';

  // A small render of a request's image (the queue's line of a choice, a file's chip): read from the core's copy once, an
  // `<img>` of an object URL (an SVG is never an image here: it is a file, docs/INBOX.md § 4.1).
  let { ask, file, alt, big = false }: { ask: Pick<WindowAsk, 'id'>; file: AskFile; alt: string; big?: boolean } = $props();

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

<span class="thumb" class:big>
  {#if url}<img src={url} {alt} draggable="false" />{/if}
</span>

<style>
  .thumb {
    display: block;
    width: 46px;
    height: 29px;
    flex: none;
    overflow: hidden;
    border-radius: 3px;
    background: var(--well);
    box-shadow: 0 0 0 1px var(--line-2);
  }
  .thumb.big {
    width: 44px;
    height: 34px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: top left;
  }
</style>
