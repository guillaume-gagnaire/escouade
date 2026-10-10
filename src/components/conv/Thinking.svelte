<script lang="ts">
  import { t } from '../../lib/i18n';
  import type { ThinkingItem } from '../../lib/types';

  let { item }: { item: ThinkingItem } = $props();
  let open = $state(false);
</script>

{#if item.text.trim() || item.streaming}
  <div class="think">
    <button class="head" onclick={() => (open = !open)} aria-expanded={open}>
      <span class="chev">{open ? '▾' : '▸'}</span>
      {t('conv.thinking.title')}
      {#if item.streaming}<span class="live">…</span>{/if}
    </button>
    {#if open}
      <div class="body">{item.text}</div>
    {/if}
  </div>
{/if}

<style>
  .think {
    margin-left: 34px;
  }
  .head {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    border: none;
    background: transparent;
    color: var(--dim);
    font-size: 12px;
    font-style: italic;
    cursor: pointer;
  }
  .head:hover {
    color: var(--muted);
  }
  .chev {
    font-style: normal;
    font-size: 10px;
  }
  .live {
    animation: ccBlink 1.2s infinite;
  }
  .body {
    margin-top: 6px;
    padding: 10px 14px;
    border-left: 2px solid var(--line2);
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
    white-space: pre-wrap;
    max-height: 360px;
    overflow: auto;
  }
</style>
