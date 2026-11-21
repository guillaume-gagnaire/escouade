<script lang="ts">
  // A bubble of the discovery (ESC-506, choice O11 of docs/ONBOARDING.md, taken from direction B): on the real control,
  // with a halo around it. At the right moment (`offer`), it says what the control does and what it asks before, with
  // « Compris » and « Écarter »; after « Essayer » on the map (`spot`), it shows where the gesture is. One at a time, a
  // dialog that is not modal, told politely to a screen reader; Échap or a key typed elsewhere closes it.
  import { onDestroy } from 'svelte';
  import { closeTip, discover, dismiss, openMap } from '../../lib/discover.svelte';
  import { app } from '../../lib/state.svelte';
  import { t } from '../../lib/i18n';
  import Icon from '../Icon.svelte';

  const tip = $derived(discover.tip);
  let box = $state<{ top: number; left: number; width: number; height: number } | null>(null);
  let bubble = $state<HTMLElement | undefined>();
  let place = $state<{ top: number; left: number; up: boolean; ax: number }>({ top: 0, left: 0, up: false, ax: 24 });

  /** Where the control is now; the bubble under it, or above when it has no room below. */
  function measure() {
    const el = tip?.el;
    // Gone from the page (its view closed): the bubble goes with it.
    if (!el || !el.isConnected) {
      closeTip();
      return;
    }
    const r = el.getBoundingClientRect();
    box = { top: r.top, left: r.left, width: r.width, height: r.height };
    const w = bubble?.offsetWidth ?? 300;
    const h = bubble?.offsetHeight ?? 140;
    const left = Math.max(12, Math.min(r.left + r.width / 2 - 28, window.innerWidth - w - 12));
    const below = r.bottom + 12 + h <= window.innerHeight - 8;
    place = {
      top: below ? r.bottom + 12 : Math.max(8, r.top - 12 - h),
      left,
      up: below,
      ax: Math.max(14, Math.min(r.left + r.width / 2 - left - 6, w - 26)),
    };
  }

  // A window opened over the screen: a bubble offered at the right moment goes, its control is under the window and
  // Échap is the window's (ESC-522). One that shows where a gesture of the map is may be in that window: it stays.
  $effect(() => {
    if (app.modal && tip?.mode === 'offer') closeTip();
  });

  let timer: ReturnType<typeof setInterval> | undefined;
  $effect(() => {
    if (!tip) return;
    // The control pressed: the gesture is under way, the bubble leaves it room.
    const el = tip.el;
    const pressed = () => closeTip();
    el.addEventListener('pointerdown', pressed);
    measure();
    requestAnimationFrame(measure);
    // The control moves with the layout (a panel opened, the window resized): the bubble follows it, or goes with it.
    timer = setInterval(measure, 300);
    return () => {
      clearInterval(timer);
      el.removeEventListener('pointerdown', pressed);
    };
  });
  onDestroy(() => clearInterval(timer));

  /** A press anywhere but on the bubble's buttons: the user is on with something else, the bubble goes, the press goes
   * on to what is under it (the bubble lets presses through but on its buttons). */
  function elsewhere(e: PointerEvent) {
    if (!tip) return;
    const acts = bubble?.querySelector('.acts');
    if (acts?.contains(e.target as Node)) return;
    closeTip();
  }

  function onkeydown(e: KeyboardEvent) {
    if (!tip) return;
    if (e.key === 'Escape') {
      e.stopPropagation();
      closeTip();
      return;
    }
    // Typing elsewhere: never a bubble while the user types.
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !bubble?.contains(e.target as Node)) closeTip();
  }
</script>

<svelte:window {onkeydown} onresize={measure} onpointerdowncapture={elsewhere} />

{#if tip && box}
  <div
    class="halo"
    aria-hidden="true"
    style:top="{box.top - 3}px"
    style:left="{box.left - 3}px"
    style:width="{box.width + 6}px"
    style:height="{box.height + 6}px"
  ></div>
  <div
    class="tip"
    class:up={place.up}
    role="dialog"
    aria-modal="false"
    aria-label={t('discover.tip.label', { name: t(`discover.items.${tip.id}.name`) })}
    aria-live="polite"
    data-testid="discover-tip"
    data-tip={tip.id}
    data-mode={tip.mode}
    bind:this={bubble}
    style:top="{place.top}px"
    style:left="{place.left}px"
    style:--ax="{place.ax}px"
  >
    <b class="h">{t(`discover.items.${tip.id}.name`)}</b>
    <p>{t(`discover.items.${tip.id}.line`)}</p>
    {#if t(`discover.items.${tip.id}.facts`)}
      <p class="facts">{t(`discover.items.${tip.id}.facts`)}</p>
    {/if}
    <div class="acts">
      <button class="btn small primary" onclick={closeTip}>{t('discover.tip.ok')}</button>
      {#if tip.mode === 'offer'}
        {@const id = tip.id}
        <button class="btn small ghost" onclick={() => void dismiss(id)}>{t('discover.tip.dismiss')}</button>
        <button
          class="btn small ghost"
          onclick={() => {
            discover.selected = id;
            openMap();
          }}><Icon name="flow" size={12} />{t('discover.tip.map')}</button
        >
      {/if}
    </div>
  </div>
{/if}

<style>
  .halo {
    position: fixed;
    z-index: 90;
    border-radius: 8px;
    box-shadow:
      0 0 0 2px var(--teal-ink),
      0 0 0 6px color-mix(in oklch, var(--teal-ink) 22%, transparent);
    pointer-events: none;
  }
  /* It never stands in the way: presses go through it, but on its buttons. */
  .tip {
    pointer-events: none;
    position: fixed;
    z-index: 91;
    width: 300px;
    max-width: calc(100vw - 24px);
    display: grid;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--panel-2, var(--panel));
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--line-2),
      0 12px 28px rgb(0 0 0 / 0.28);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .tip::before {
    content: '';
    position: absolute;
    left: var(--ax);
    width: 12px;
    height: 12px;
    background: inherit;
    transform: rotate(45deg);
    box-shadow: -1px -1px 0 0 var(--line-2);
  }
  .tip.up::before {
    top: -6px;
  }
  .tip:not(.up)::before {
    bottom: -6px;
    box-shadow: 1px 1px 0 0 var(--line-2);
  }
  .h {
    font-size: 13px;
    font-weight: 640;
  }
  p {
    color: var(--soft);
  }
  .facts {
    color: var(--muted);
    font-size: 12px;
  }
  .acts {
    pointer-events: auto;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  @media (prefers-reduced-motion: no-preference) {
    .tip {
      animation: rise 240ms var(--ease);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tip {
      animation: fade 120ms linear;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>
