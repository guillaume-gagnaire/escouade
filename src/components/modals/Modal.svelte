<script lang="ts">
  import type { Snippet } from 'svelte';
  import { trapFocus } from '../../lib/focus';
  import { t } from '../../lib/i18n';

  let {
    title,
    width = 540,
    onclose,
    children,
    footer,
  }: { title: string; width?: number; onclose: () => void; children: Snippet; footer?: Snippet } = $props();

  // Only a click on the overlay closes it: not the end of a selection dragged out of a field (what was typed there
  // would be lost).
  let downOnOverlay = false;
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="overlay"
  onmousedown={(e) => (downOnOverlay = e.target === e.currentTarget)}
  onclick={(e) => downOnOverlay && e.target === e.currentTarget && onclose()}
>
  <div
    class="modal"
    use:trapFocus
    style:width="{width}px"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={title}
  >
    <div class="head">
      <span class="t">{title}</span>
      <div style="flex:1"></div>
      <button class="icon-btn" style="width:28px;height:28px;font-size:16px" onclick={onclose} aria-label={t('common.close')}>×</button>
    </div>
    <div class="body">{@render children()}</div>
    {#if footer}<div class="foot">{@render footer()}</div>{/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(12, 10, 8, 0.6);
    backdrop-filter: blur(3px);
    animation: fade 0.12s ease-out;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .modal {
    max-width: calc(100vw - 40px);
    max-height: calc(100vh - 40px);
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 14px;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55);
    animation: ccFadeIn 0.15s ease-out;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 18px 18px 0 24px;
  }
  .t {
    font-size: 17px;
    font-weight: 700;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 20px 24px 22px;
    overflow: auto;
    min-height: 0;
  }
  .foot {
    flex: none;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 18px 18px;
    border-top: 1px solid var(--line);
  }
</style>
