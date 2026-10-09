<script lang="ts">
  import type { UserItem } from '../../lib/types';

  let { item, waiting = false }: { item: UserItem; waiting?: boolean } = $props();
</script>

<div class="wrap">
  <div class="bubble">
    {#if item.images > 0}
      <span class="chip">🖼 {item.images} image{item.images > 1 ? 's' : ''}</span>
    {/if}
    {#each item.files ?? [] as name, i (i)}
      <span class="chip">📄 {name}</span>
    {/each}
    <span class="text">{item.text}</span>
    <!-- Sent while the worktree is prepared: the backend takes it once that is over. -->
    {#if waiting}<span class="note">En attente de la préparation…</span>{/if}
    <!-- Sent while Claude worked: the CLI takes it at the turn's next step (after the running tool). -->
    {#if item.queued}<span class="note" title="Claude en tient compte dès sa prochaine étape">transmis pendant le tour</span>{/if}
    {#if item.origin === 'remote'}<span class="note" title="Envoyé depuis claude.ai ou l’app Claude (remote control)">depuis claude.ai</span
      >{/if}
  </div>
</div>

<style>
  .wrap {
    display: flex;
    justify-content: flex-end;
  }
  .bubble {
    max-width: 78%;
    padding: 11px 15px;
    border-radius: var(--r);
    background: var(--user);
    font-size: 14px;
    line-height: 1.55;
    text-wrap: pretty;
    overflow-wrap: anywhere;
  }
  /* Only the message keeps its line breaks and spaces, not the template's. */
  .text {
    white-space: pre-wrap;
  }
  .chip {
    display: inline-block;
    margin: 0 8px 4px 0;
    padding: 1px 7px;
    border-radius: 99px;
    background: var(--elev2);
    font-size: 11.5px;
    color: var(--muted);
  }
  .note {
    display: block;
    margin-top: 6px;
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--dim);
  }
</style>
