<script lang="ts">
  import { engineName } from '../../lib/engines';
  import { fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import type { HandoverItem } from '../../lib/types';

  // The change of harness in the conversation (ESC-88): « Suite sur Codex · quota Claude Code épuisé jusqu’à 18:00 ».
  let { item }: { item: HandoverItem } = $props();

  const text = $derived.by(() => {
    const words = { harness: engineName(item.to), from: engineName(item.from) };
    // An agent a teammate passed (TS-17): « Passé par Alice · suite sur Codex ».
    if (item.by) return t('handover.separator.passed', { name: item.by, harness: words.harness });
    if (!item.limited) return t('handover.separator.asked', words);
    if (item.until === null) return t('handover.separator.limited', words);
    return t('handover.separator.limitedUntil', { ...words, time: fTime(item.until) });
  });
  // Where it goes on, in bold, before why.
  const cut = $derived(text.indexOf(' · '));
</script>

<div class="sep" role="separator" aria-label={text} data-testid="handover">
  <span class="text"
    >{#if cut > 0}<b>{text.slice(0, cut)}</b>{text.slice(cut)}{:else}{text}{/if}</span
  >
</div>

<style>
  .sep {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 14px 0 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .sep::before,
  .sep::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--line-2);
  }
  .text {
    flex: none;
    max-width: 80%;
    overflow-wrap: anywhere;
    text-align: center;
  }
  b {
    color: var(--soft);
    font-weight: 550;
  }
</style>
