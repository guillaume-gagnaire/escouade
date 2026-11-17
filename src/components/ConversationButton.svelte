<script lang="ts">
  import { conversation } from '../lib/conversation.svelte';
  import { conversationOf } from '../lib/conversations.svelte';
  import { fBytes } from '../lib/format';
  import { t } from '../lib/i18n';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';
  import type { Agent } from '../lib/types';

  // « Conversation » in the composer of the Escouade view (ESC-97): a click starts the voice conversation with the
  // agent, its panel says where it is (listening, transcribing, the agent at work, its answer read aloud) and stops it.
  let { agent }: { agent: Agent } = $props();

  // Only its id: the agent object is replaced at each update of the backend.
  const agentId = $derived(agent.id);
  const mine = $derived(conversation.agentId === agentId);
  const phase = $derived(mine ? conversation.phase : 'idle');
  const conv = $derived(conversationOf(agentId));
  const progress = $derived(mine ? conversation.progress : null);
  const pct = $derived(progress && progress.total ? Math.min(100, (progress.done / progress.total) * 100) : 0);

  // The agent's state and conversation, as they change: its questions are read, the end of its turn has its passage read.
  $effect(() => {
    if (conversation.agentId === agentId) conversation.observe(agent, conv.items);
  });

  // Another agent in this composer, or the composer gone: the conversation stops.
  $effect(() => {
    const id = agentId;
    return () => {
      if (conversation.agentId === id) conversation.stop();
    };
  });

  function toggle() {
    if (mine && conversation.active) conversation.stop();
    else void conversation.start(agentId);
  }
</script>

<div class="conv-wrap">
  <!-- What to download first; once it listens, the conversation fills the window (ConversationOverlay). -->
  {#if phase === 'offer' || phase === 'downloading'}
    <div class="panel" role="status" aria-live="polite" data-conversation-phase={phase}>
      {#if phase === 'offer' && conversation.offer}
        <span class="small">
          {conversation.offer.kind === 'model'
            ? t('voice.conversation.offerModel', { model: conversation.offer.id, size: fBytes(conversation.offer.size) })
            : t('voice.conversation.offerVoice', { voice: conversation.offer.id, size: fBytes(conversation.offer.size) })}
        </span>
        <div class="actions">
          <button class="btn ghost" onclick={() => conversation.stop()}>{t('voice.conversation.cancel')}</button>
          <button class="btn primary" onclick={() => conversation.download()}>{t('voice.conversation.download')}</button>
        </div>
      {:else if phase === 'downloading' && progress}
        <span class="muted small"
          >{t('voice.conversation.downloading', { name: progress.id, done: fBytes(progress.done), total: fBytes(progress.total) })}</span
        >
        <div class="meter" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(pct)}>
          <div class="fill" style:transform="scaleX({pct / 100})"></div>
        </div>
        <div class="actions">
          <button class="btn ghost" onclick={() => conversation.stop()}>{t('voice.conversation.cancel')}</button>
        </div>
      {/if}
    </div>
  {/if}
  <button
    class="icon-btn conv"
    data-discover="voice"
    class:on={mine && conversation.active}
    aria-pressed={mine && conversation.active}
    aria-label={t('voice.conversation.button')}
    use:tip={t('voice.conversation.buttonTitle')}
    onclick={toggle}
  >
    <Icon name="wave" size={14} />
  </button>
</div>

<style>
  .conv-wrap {
    position: relative;
    flex: none;
  }
  .conv.on {
    background: color-mix(in oklch, var(--p) 14%, transparent);
    color: var(--p-ink);
  }
  .panel {
    position: absolute;
    right: 0;
    bottom: calc(100% + 8px);
    z-index: 40;
    width: 340px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 16px 40px -12px rgba(0, 0, 0, calc(0.7 * var(--shade)));
    font-size: 12.5px;
    text-align: left;
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
    line-height: 1.45;
  }
  .meter {
    height: 4px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.1);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--p-ink);
    transform-origin: left;
    transition: transform 0.06s linear;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
</style>
