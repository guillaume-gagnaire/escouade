<script lang="ts">
  import { app, type Toast } from '../lib/state.svelte';

  function act(t: Toast) {
    app.dismissToast(t.id);
    t.action?.onClick();
  }
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}">
      {t.text}{#if t.action}<button class="act" onclick={() => act(t)}>{t.action.label}</button>{/if}
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 44px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 420px;
    pointer-events: none;
  }
  .toast {
    padding: 10px 14px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4);
    font-size: 12.5px;
    line-height: 1.45;
    white-space: pre-wrap;
    animation: ccFadeIn 0.15s ease-out;
    pointer-events: auto;
  }
  .error {
    border-color: var(--del);
  }
  .ok {
    border-color: var(--ok);
  }
  .act {
    margin-left: 10px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--accent);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .act:hover {
    text-decoration: underline;
  }
</style>
