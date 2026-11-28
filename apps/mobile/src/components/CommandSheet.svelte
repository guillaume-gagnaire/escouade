<script lang="ts">
  // A command beyond twelve lines, read in full screen (MO-5, M4): every line, then « Autoriser », armed once the last
  // line was seen, under the system's lock (M3). Read to its end here, the line's fold arms too.
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import { answerStop, conversationOf } from '../lib/act';
  import { detailOf, type Detail } from '../lib/detail';
  import { openNext, overlay, currentThread } from '../lib/feed.svelte';
  import { closeSheet } from '../lib/nav.svelte';
  import { phone, stateOf } from '../lib/phone-spaces.svelte';
  import CommandBox from './CommandBox.svelte';
  import Screen from './Screen.svelte';
  import { fullRead } from './StopFold.svelte';

  const line = overlay.command!.line;
  let fetched = $state<Detail | null>(null);
  const detail = $derived(fetched ?? detailOf(line.request, null));
  let read = $state(false);
  let busy = $state(false);
  let said = $state<string | null>(null);
  const space = $derived(phone.spaces.find((s) => s.id === line.space.id));
  const cut = $derived(!space || stateOf(space) !== 'open');

  $effect(() => {
    conversationOf(line.space.id, line.agentId)
      .then((items) => (fetched = detailOf(line.request, items)))
      .catch(() => {});
  });

  function seen() {
    read = true;
    fullRead.add(line.key);
  }

  async function allow() {
    busy = true;
    said = null;
    const before = currentThread();
    const o = await answerStop(line, 'allow');
    busy = false;
    if (o.ok) {
      closeSheet();
      openNext(before, line.key);
    } else said = 'locked' in o ? t('mobile.lock.cancelled') : o.error;
  }
</script>

<Screen title={line.title} sub={`${line.agentName} · ${line.space.name}`} label={t('mobile.answer.command')} onClose={closeSheet}>
  {#if detail.context}<p class="ctx">{detail.context}</p>{/if}
  <CommandBox text={detail.text} prompt={detail.shell} onEnd={seen} />
  {#snippet foot()}
    {#if cut}<p class="note warn">{t('mobile.answer.offline')}</p>{/if}
    {#if said}<p class="note" role="status">{said}</p>{/if}
    <div class="acts">
      <button type="button" class="b" onclick={closeSheet}>{t('mobile.viewer.close')}</button>
      <button type="button" class="b go" disabled={busy || cut || !read || !detail.whole} onclick={allow}>
        <Icon name="lock" size={16} />{t('mobile.answer.allow')}
      </button>
    </div>
    {#if !read}<p class="note">{t('mobile.answer.readToEnd')}</p>{/if}
  {/snippet}
</Screen>

<style>
  .ctx {
    margin: 4px 0 10px;
    font-size: 14px;
    color: var(--soft);
  }
  .acts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--gap-target);
  }
  .b {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: calc(var(--target) + 2px);
    border: 0;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-weight: 650;
    font-size: 16px;
  }
  .b.go {
    background: var(--text);
    color: var(--panel);
    box-shadow: none;
  }
  .b:disabled {
    opacity: 0.45;
  }
  .note {
    margin: 6px 0 0;
    font-size: var(--small);
    color: var(--muted);
  }
  .warn {
    color: var(--wait);
  }
</style>
