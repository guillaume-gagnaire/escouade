<script lang="ts">
  import { speakerOf } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { cancelHeld, editHeld, sendNow, type Held } from '../../lib/outbox.svelte';
  import { tip } from '../../lib/tip';
  import type { EngineId, UserItem } from '../../lib/types';
  import UserMessage from './UserMessage.svelte';

  // A message the outbox holds, with when it goes and what can be done until then.
  let { held, engine = null }: { held: Held; engine?: EngineId | null } = $props();

  const item = $derived<UserItem>({
    kind: 'user',
    id: held.id,
    text: held.body,
    images: held.files.filter((f) => f.kind === 'image').length,
    files: held.files.filter((f) => f.kind !== 'image').map((f) => f.name),
    ts: 0,
    queued: false,
  });

  let now = $state(Date.now());
  $effect(() => {
    if (held.at === null) return;
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });
  const left = $derived(held.at === null ? 0 : Math.max(0, Math.ceil((held.at - now) / 1000)));
</script>

<UserMessage {item} {engine}>
  {#snippet extra()}
    {#if held.mode === 'turnEnd'}
      <span class="note" use:tip={t('conv.held.turnEndHint', { engine: speakerOf(engine) })}>{t('conv.held.turnEnd')}</span>
    {:else}
      <span class="note">{t('conv.held.delay', { s: left })}</span>
      <button class="note act" onclick={() => sendNow(held.id)}>{t('conv.held.sendNow')}</button>
    {/if}
    <button class="note act" onclick={() => editHeld(held.id)}>{t('conv.held.edit')}</button>
    <button class="note act" onclick={() => cancelHeld(held.id)}>{t('conv.held.cancel')}</button>
  {/snippet}
</UserMessage>

<style>
  /* As the notes under a message sent (UserMessage's `.note`). */
  .note {
    color: var(--muted);
    font-size: 11.5px;
  }
  .act {
    border: none;
    background: none;
    padding: 0;
    font-family: inherit;
    cursor: pointer;
  }
  .act:hover {
    color: var(--p-ink);
    text-decoration: underline;
  }
</style>
