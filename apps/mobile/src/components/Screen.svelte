<script lang="ts">
  // A screen above the tabs (MO-5): the viewer, a PDF, a file, a command read whole, a conversation. Its header (back,
  // its title and what it is about), its body; the system's back closes it (`nav.svelte.ts`), as its « ‹ » does.
  import { onMount, type Snippet } from 'svelte';
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';

  let {
    title,
    sub = '',
    label,
    onClose,
    children,
    side,
    foot,
    dark = false,
  }: {
    title: string;
    sub?: string;
    label: string;
    onClose: () => void;
    children: Snippet;
    side?: Snippet;
    foot?: Snippet;
    /** A dark ground, whatever the theme (the viewer). */
    dark?: boolean;
  } = $props();

  let root = $state<HTMLElement>();
  onMount(() => {
    const before = document.activeElement as HTMLElement | null;
    root?.focus();
    return () => before?.focus?.();
  });
</script>

<div
  class="screen"
  class:dark
  role="dialog"
  aria-modal="true"
  aria-label={label}
  tabindex="-1"
  bind:this={root}
  onkeydown={(e) => e.key === 'Escape' && onClose()}
>
  <header>
    <button type="button" class="back" aria-label={t('mobile.talk.back')} onclick={onClose}><Icon name="left" size={22} /></button>
    <span class="tx">
      <span class="tt">{title}</span>
      {#if sub}<span class="sb">{sub}</span>{/if}
    </span>
    {#if side}{@render side()}{/if}
  </header>
  <div class="bd">{@render children()}</div>
  {#if foot}<footer>{@render foot()}</footer>{/if}
</div>

<style>
  .screen {
    position: fixed;
    inset: 0;
    z-index: 20;
    max-width: 600px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    outline: none;
  }
  .screen.dark {
    background: #0b0c0e;
    color: #f7f8f8;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: calc(6px + var(--safe-top)) calc(var(--gut) - 6px + var(--safe-right)) 6px calc(var(--gut) - 10px + var(--safe-left));
  }
  .back {
    flex: none;
    width: var(--target);
    height: var(--target);
    border: 0;
    border-radius: var(--r-m);
    display: grid;
    place-items: center;
    background: transparent;
    color: inherit;
  }
  .tx {
    flex: 1;
    min-width: 0;
    display: grid;
  }
  .tt {
    font-size: 17px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sb {
    font-size: var(--small);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .bd {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 0 calc(var(--gut) + var(--safe-right)) 12px calc(var(--gut) + var(--safe-left));
  }
  footer {
    flex: none;
    padding: 8px calc(var(--gut) + var(--safe-right)) calc(8px + var(--safe-bottom)) calc(var(--gut) + var(--safe-left));
    box-shadow: inset 0 1px 0 var(--line);
  }
</style>
