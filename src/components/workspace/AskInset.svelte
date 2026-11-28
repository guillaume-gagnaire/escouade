<script lang="ts">
  // The requests an agent waits on, in its card on the « Workspace » (spec 2026-11-25-vue-workspace-design.md, « La
  // question ») : the conversation's own cards, mounted as they are. `mine`: whether a request of that subagent (none: the
  // agent's own) is this card's; `label`: say whose a subagent's request is. Its conversation is kept loaded by its card
  // (AgentCard) while it waits.
  import { conversationOf } from '../../lib/conversations.svelte';
  import { t } from '../../lib/i18n';
  import { subName } from '../../lib/workspace-layout';
  import type { Agent, PermissionItem, QuestionItem } from '../../lib/types';
  import PermissionCard from '../conv/PermissionCard.svelte';
  import QuestionCard from '../conv/QuestionCard.svelte';

  let {
    agent,
    mine,
    maxH,
    label = false,
    onfocusback,
  }: {
    agent: Agent;
    mine: (sub: string | undefined) => boolean;
    maxH: number;
    label?: boolean;
    /** The card it held the focus in went (answered): its card takes it back, the composer the cards give it to is not here. */
    onfocusback?: () => void;
  } = $props();

  const conv = $derived(conversationOf(agent.id));
  const asks = $derived(
    agent.pending
      .map((id) => conv.items.find((i) => i.id === id))
      .filter((i): i is QuestionItem | PermissionItem => (i?.kind === 'question' || i?.kind === 'permission') && mine(i.sub ?? undefined)),
  );

  let box = $state<HTMLElement>();
  let held = false;
  // Before the cards change, whether the focus is in one; after, whether it fell to the page with the one that went.
  $effect.pre(() => {
    void asks;
    held = !!box?.contains(document.activeElement);
  });
  /** The wheel scrolls the inset while it can that way; at its end, or with nothing to scroll, the canvas pans. A pinch zooms. */
  function wheel(e: WheelEvent) {
    const el = e.currentTarget as HTMLElement;
    const room = e.deltaY < 0 ? el.scrollTop > 0 : e.deltaY > 0 && el.scrollTop + el.clientHeight < el.scrollHeight - 1;
    if (room && !e.ctrlKey && !e.metaKey) e.stopPropagation();
  }
  $effect(() => {
    void asks;
    if (held && (!document.activeElement || document.activeElement === document.body)) onfocusback?.();
    held = false;
  });
</script>

{#if agent.pending.length && !conv.loaded && mine(undefined)}
  <p class="loading">{t('workspace.ask.loading')}</p>
{:else if asks.length}
  <!-- `current` false: no card takes the answer keys nor shows them, several may wait on screen at once. -->
  <div class="ask" bind:this={box} style:max-height="{maxH}px" onwheel={wheel}>
    {#each asks as item (item.id)}
      {#if label && item.sub}<p class="from">{t('workspace.ask.fromSub', { name: subName(agent, conv.items, item.sub) })}</p>{/if}
      {#if item.kind === 'question'}
        <QuestionCard {item} agentId={agent.id} pending current={false} />
      {:else}
        <PermissionCard {item} agentId={agent.id} cwd={agent.cwd} pending current={false} />
      {/if}
    {/each}
  </div>
{/if}

<style>
  .ask {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 6px;
    overflow: auto;
    overscroll-behavior: contain;
  }
  .from {
    margin: 0;
    padding-left: 16px;
    color: var(--wait);
    font-size: 12px;
    font-weight: 550;
  }
  .loading {
    margin: 6px 0 0;
    padding-left: 16px;
    color: var(--muted);
    font-size: 12px;
  }
</style>
