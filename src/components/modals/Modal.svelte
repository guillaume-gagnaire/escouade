<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { trapFocus } from '../../lib/focus';
  import { t } from '../../lib/i18n';
  import Icon from '../Icon.svelte';

  let {
    title,
    width = 540,
    tall = false,
    onclose,
    children,
    footer,
    lead,
    badge,
    height,
    flush = false,
    frameFocus = false,
  }: {
    title: string;
    width?: number;
    tall?: boolean;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
    /** Before the title: an icon of what is asked. */
    lead?: Snippet;
    /** Beside the title: what the window is at. */
    badge?: Snippet;
    /** A height of its own (the assistant's, which does not move from one step to the next). */
    height?: string;
    /** Its content fills the body, edge to edge (the assistant's rail and page). */
    flush?: boolean;
    /** The dialog itself takes the focus, not its first button: Entrée confirms nothing, what was typed elsewhere is not
     * taken for an answer (« Supprimer l’agent », ESC-511). */
    frameFocus?: boolean;
  } = $props();

  // Only a click on the overlay closes it: not the end of a selection dragged out of a field (what was typed there
  // would be lost).
  let downOnOverlay = false;
  let overlay = $state<HTMLDivElement>();
  let frame = $state<HTMLDivElement>();
  // After `trapFocus`, which gave it to the first button.
  onMount(() => {
    if (frameFocus) frame?.focus();
  });
  /** Échap closes the modal on top alone: one opened over another (« Ajouter un compte… » from the assistant) first. */
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    const all = document.querySelectorAll('.overlay');
    if (all.length > 1 && all[all.length - 1] !== overlay) return;
    onclose();
  }
</script>

<svelte:window {onkeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  bind:this={overlay}
  class="overlay"
  onmousedown={(e) => (downOnOverlay = e.target === e.currentTarget)}
  onclick={(e) => downOnOverlay && e.target === e.currentTarget && onclose()}
>
  <div
    bind:this={frame}
    class="modal"
    class:tall
    use:trapFocus
    style:--w="{width}px"
    style:height
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={title}
  >
    <div class="head">
      {#if lead}{@render lead()}{/if}
      <h2 class="t">{title}</h2>
      {#if badge}{@render badge()}{/if}
      <button class="icon-btn close" onclick={onclose} aria-label={t('common.close')}><Icon name="close" size={14} /></button>
    </div>
    <div class="body" class:flush>{@render children()}</div>
    {#if footer}<div class="foot">{@render footer()}</div>{/if}
  </div>
</div>

<style>
  /* The windows of the 2.0 design (lot 11a, the preview's `.scrim` and `.modal`): a raised panel on the scrim, its
     header of 54 px under a rule, its buttons on the right under another. */
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: var(--scrim);
    animation: fade 0.2s ease-out;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .modal {
    display: flex;
    flex-direction: column;
    width: min(var(--w), 100vw - 48px);
    max-height: calc(100vh - 96px);
    border-radius: 14px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 24px 64px -12px rgba(0, 0, 0, calc(0.75 * var(--shade)));
    animation: rise 0.35s var(--ease);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.985);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .modal {
      animation: none;
    }
  }
  .modal:focus {
    outline: none;
  }
  /* A window to write in: its height does not follow its content, which would move it at every line typed. */
  .modal.tall {
    height: min(88vh, 860px);
    max-height: calc(100vh - 40px);
  }
  /* The footer stays at the foot of the window, whatever the content leaves free. */
  .modal.tall .body {
    flex: 1 1 auto;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 54px;
    padding: 0 12px 0 20px;
    border-bottom: 1px solid var(--line);
  }
  .t {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.01em;
  }
  .close {
    margin-left: auto;
    width: 30px;
    height: 30px;
    padding: 0;
    border-radius: 6px;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 18px 20px 20px;
    overflow: auto;
    min-height: 0;
  }
  .body.flush {
    flex: 1 1 auto;
    gap: 0;
    padding: 0;
    overflow: hidden;
  }
  .foot {
    flex: none;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--line);
  }
  /* What the window is at, beside its title (the assistant's folder and step). */
  .head :global(.chip-sm) {
    display: inline-flex;
    flex: none;
    align-items: center;
    height: 20px;
    padding: 0 7px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
</style>
