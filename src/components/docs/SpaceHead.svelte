<script lang="ts">
  import type { Snippet } from 'svelte';
  import DocsSync from './DocsSync.svelte';

  // The head of a space of the Docs tab (design/factory/2-specification.png): its title and what it says under it, its
  // buttons on the right, the synchronisation of the docs first (ESC-534), in every space, the space's own buttons after it.
  let { title, line, actions, projectId }: { title: string; line?: Snippet; actions?: Snippet; projectId?: string } = $props();
</script>

<header class="head">
  <div class="h-title">
    <h1>{title}</h1>
    {#if line}<div class="line">{@render line()}</div>{/if}
  </div>
  {#if actions || projectId}
    <div class="actions">
      {#if projectId}<DocsSync {projectId} />{#if actions}<span class="vsep" aria-hidden="true"></span>{/if}{/if}
      {#if actions}{@render actions()}{/if}
    </div>
  {/if}
</header>

<style>
  .head {
    flex: none;
    display: flex;
    align-items: center;
    /* Too narrow for the title, what it says and the buttons (1024 pixels, the synchronisation of ESC-534 with the
       space's own buttons): the buttons go under, on the right, rather than crushing the title. */
    flex-wrap: wrap;
    gap: 8px 16px;
    min-height: 66px;
    padding: 10px 16px 10px 22px;
    border-bottom: 1px solid var(--line);
  }
  .h-title {
    flex: 1 1 260px;
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 640;
    letter-spacing: -0.015em;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 16px;
    color: var(--muted);
    font-size: 13px;
  }
  .actions {
    flex: 0 1 auto;
    margin-left: auto;
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 6px 8px;
    min-width: 0;
  }
  .vsep {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: var(--line-2);
  }
</style>
