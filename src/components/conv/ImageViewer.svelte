<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { trapFocus } from '../../lib/focus';
  import { copyImage, openSource, type ViewedImage } from '../../lib/gallery';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  // The images of a conversation seen full size (ESC-171): one at a time, fitted to the window as it changes; zoomed by
  // a click on it, Ctrl or ⌘ and the wheel, the trackpad's pinch, the buttons or + − 0, moved about by dragging or the
  // arrows; the previous and the next by the buttons, the arrows (fitted) or the strip of thumbnails; Échap or a click
  // beside it to close. An image is an `<img>` of a `data:` URL the core read: an SVG's scripts never run.
  let { images, index = $bindable(0), onclose }: { images: ViewedImage[]; index?: number; onclose: () => void } = $props();

  const current = $derived(images[Math.min(index, images.length - 1)]);
  const many = $derived(images.length > 1);
  let overlay = $state<HTMLDivElement>();
  let closer = $state<HTMLButtonElement>();
  let stage = $state<HTMLDivElement>();
  let img = $state<HTMLImageElement>();
  let strip = $state<HTMLDivElement>();

  // ---------- its size ----------
  /** The most it zooms, and a step of the buttons and the keys. */
  const MAX = 8;
  const STEP = 1.25;
  let roomW = $state(0);
  let roomH = $state(0);
  /** The image's own size, read once it is loaded (0 before). */
  let natural = $state({ w: 0, h: 0 });
  /** The zoom chosen; null: fitted to the room, as the window changes. */
  let scale = $state<number | null>(null);
  /** Fitted, never larger than itself, with the room of the side buttons. */
  const fit = $derived(
    natural.w && roomW ? Math.max(0.01, Math.min(1, (roomW - (many ? 144 : 48)) / natural.w, (roomH - 32) / natural.h)) : 1,
  );
  const z = $derived(scale ?? fit);
  const zoomed = $derived(z > fit + 0.001);

  /** The zoom set to `v`, between fitted and `MAX`, the point `at` of the window (the room's middle) staying where it is. */
  async function zoomTo(v: number, at?: { x: number; y: number }) {
    if (!stage || !img) return;
    const next = Math.min(MAX, Math.max(fit, v));
    const room = stage.getBoundingClientRect();
    const p = at ?? { x: room.left + room.width / 2, y: room.top + room.height / 2 };
    const was = img.getBoundingClientRect();
    const fx = was.width ? (p.x - was.left) / was.width : 0.5;
    const fy = was.height ? (p.y - was.top) / was.height : 0.5;
    scale = next > fit + 0.001 ? next : null;
    await tick();
    if (!img) return;
    const now = img.getBoundingClientRect();
    stage.scrollLeft += now.left + fx * now.width - p.x;
    stage.scrollTop += now.top + fy * now.height - p.y;
  }

  /** The image `i` (around the ends), fitted; the one shown stays as it is (its `<img>` is not made again, nor read). */
  function show(i: number) {
    const next = (i + images.length) % images.length;
    if (next === index) return;
    index = next;
    scale = null;
    natural = { w: 0, h: 0 };
  }

  // The thumbnail of the image shown stays in sight in the strip.
  $effect(() => {
    void index;
    strip?.querySelector('[aria-current="true"]')?.scrollIntoView?.({ inline: 'center', block: 'nearest' });
  });

  // ---------- the hand ----------
  function wheel(e: WheelEvent) {
    // The wheel alone scrolls the image zoomed; with Ctrl or ⌘ (a pinch, in WebView2), it zooms.
    if (!e.ctrlKey && !e.metaKey) return;
    e.preventDefault();
    void zoomTo(z * Math.exp(-Math.max(-50, Math.min(50, e.deltaY)) * 0.005), { x: e.clientX, y: e.clientY });
  }
  // The trackpad's pinch in WebKit (macOS): its own gesture events, not the wheel's.
  $effect(() => {
    const el = stage;
    if (!el) return;
    let from = 1;
    const start = (e: Event) => (e.preventDefault(), (from = z));
    const change = (e: Event) => {
      e.preventDefault();
      const g = e as Event & { scale: number; clientX: number; clientY: number };
      void zoomTo(from * g.scale, { x: g.clientX, y: g.clientY });
    };
    el.addEventListener('gesturestart', start);
    el.addEventListener('gesturechange', change);
    return () => (el.removeEventListener('gesturestart', start), el.removeEventListener('gesturechange', change));
  });

  // Zoomed, the image drags by hand (the scrollbars stay the room's); a drag that ends beside it does not close the viewer.
  let drag: { x: number; y: number; l: number; t: number; id: number } | null = null;
  let dragging = $state(false);
  function down(e: PointerEvent) {
    dragging = false;
    if (e.button !== 0 || !zoomed || !stage || e.target !== img) return;
    drag = { x: e.clientX, y: e.clientY, l: stage.scrollLeft, t: stage.scrollTop, id: e.pointerId };
  }
  function move(e: PointerEvent) {
    if (!drag || !stage) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (!dragging && Math.abs(dx) + Math.abs(dy) < 4) return;
    if (!dragging) stage.setPointerCapture(drag.id);
    dragging = true;
    stage.scrollLeft = drag.l - dx;
    stage.scrollTop = drag.t - dy;
  }
  const up = () => (drag = null);

  /** A click on the image zooms it to its size (twice a small one's) at that point, or fits it again; beside it, closes. */
  function click(e: MouseEvent) {
    if (dragging) return;
    if (e.target === img) void zoomTo(zoomed ? fit : fit < 1 ? 1 : 2, { x: e.clientX, y: e.clientY });
    else if (e.target === e.currentTarget) onclose();
  }

  // ---------- the keys ----------
  const PAN: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
  function onkeydown(e: KeyboardEvent) {
    // A window opened over it closes first.
    const all = document.querySelectorAll('.overlay');
    if (all.length > 1 && all[all.length - 1] !== overlay) return;
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    } else if (e.ctrlKey || e.metaKey || e.altKey) {
      // Ctrl + and Ctrl − stay the zoom of the interface.
      return;
    } else if (e.key === '+' || e.key === '=') void zoomTo(z * STEP);
    else if (e.key === '-') void zoomTo(z / STEP);
    else if (e.key === '0') void zoomTo(fit);
    // Zoomed, the arrows move about the image; fitted, ← → go to the previous and the next.
    else if (zoomed && PAN[e.key]) stage?.scrollBy(PAN[e.key][0] * 80, PAN[e.key][1] * 80);
    else if (many && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) show(index + (e.key === 'ArrowLeft' ? -1 : 1));
    else return;
    e.preventDefault();
  }

  // ---------- its actions ----------
  async function copy() {
    if (!img) return;
    try {
      await copyImage(img);
      app.toast(t('conv.sent.copied'));
    } catch (e) {
      app.refused(e);
    }
  }
  async function save() {
    if (current && (await app.run(api.saveImage(current.url, current.name)))) app.toast(t('conv.sent.saved'));
  }
  const open = (reveal: boolean) => current?.source && app.run(openSource(current.source, reveal));

  onMount(() => closer?.focus());

  /** Out of the conversation, under the body: a column with `container-type` holds a `position: fixed` child in it. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

<svelte:window {onkeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  bind:this={overlay}
  class="overlay viewer"
  role="dialog"
  aria-modal="true"
  aria-label={t('conv.sent.viewer')}
  tabindex="-1"
  use:portal
  use:trapFocus
  onclick={(e) => e.target === e.currentTarget && onclose()}
>
  <div class="bar">
    <b class="name" use:tip={{ text: current?.name ?? '', cut: true }}>{current?.name}</b>
    {#if many}<span class="count">{t('conv.sent.counter', { n: index + 1, count: images.length })}</span>{/if}
    <div class="zoom" role="group" aria-label={t('conv.sent.zoom')}>
      <button
        class="round"
        aria-label={t('conv.sent.zoomOut')}
        use:tip={t('conv.sent.zoomOut')}
        disabled={!zoomed}
        onclick={() => zoomTo(z / STEP)}><Icon name="min" size={15} /></button
      >
      <button
        class="pct"
        aria-label={zoomed ? t('conv.sent.fit') : t('conv.sent.actual')}
        use:tip={zoomed ? t('conv.sent.fit') : t('conv.sent.actual')}
        disabled={!zoomed && fit >= 1}
        onclick={() => zoomTo(zoomed ? fit : 1)}>{Math.round(z * 100)} %</button
      >
      <button
        class="round"
        aria-label={t('conv.sent.zoomIn')}
        use:tip={t('conv.sent.zoomIn')}
        disabled={z >= MAX}
        onclick={() => zoomTo(z * STEP)}><Icon name="plus" size={15} /></button
      >
    </div>
    <button class="round" aria-label={t('conv.sent.copy')} use:tip={t('conv.sent.copy')} onclick={copy}
      ><Icon name="copy" size={15} /></button
    >
    <button class="round" aria-label={t('conv.sent.save')} use:tip={t('conv.sent.save')} onclick={save}
      ><Icon name="download" size={15} /></button
    >
    {#if current?.source}
      {@const web = current.source.kind === 'remote'}
      <button
        class="round"
        aria-label={web ? t('conv.sent.openWeb') : t('conv.sent.openName', { name: current.name })}
        use:tip={web ? t('conv.sent.openWeb') : t('common.open')}
        onclick={() => open(false)}><Icon name="ext" size={15} /></button
      >
      {#if !web}
        <button
          class="round"
          aria-label={t('conv.sent.revealName', { name: current.name })}
          use:tip={t('conv.sent.reveal')}
          onclick={() => open(true)}><Icon name="folder" size={15} /></button
        >
      {/if}
    {/if}
    <button bind:this={closer} class="round" aria-label={t('conv.sent.close')} onclick={onclose}><Icon name="close" size={16} /></button>
  </div>
  <div class="middle">
    <!-- A click on the image zooms it, beside it closes the viewer; the keys are the window's. -->
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      bind:this={stage}
      bind:clientWidth={roomW}
      bind:clientHeight={roomH}
      class="stage"
      class:zoomed
      class:dragging
      onclick={click}
      onwheel={wheel}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
    >
      {#key index}
        {#if current}
          <img
            bind:this={img}
            src={current.url}
            alt={current.name}
            draggable="false"
            class:sized={natural.w > 0}
            style:width={natural.w ? `${natural.w * z}px` : null}
            style:height={natural.w ? `${natural.h * z}px` : null}
            onload={(e) => {
              const i = e.currentTarget as HTMLImageElement;
              natural = { w: i.naturalWidth || roomW, h: i.naturalHeight || roomH };
            }}
          />
        {/if}
      {/key}
    </div>
    {#if many}
      <button class="round side prev" aria-label={t('conv.sent.prev')} onclick={() => show(index - 1)}
        ><Icon name="right" size={18} /></button
      >
      <button class="round side next" aria-label={t('conv.sent.next')} onclick={() => show(index + 1)}
        ><Icon name="right" size={18} /></button
      >
    {/if}
  </div>
  {#if many}
    <div class="strip" bind:this={strip} role="group" aria-label={t('conv.sent.strip')}>
      {#each images as im, i (i)}
        <button
          class="mini"
          aria-current={i === index}
          aria-label={t('conv.sent.thumb', { n: i + 1, count: images.length, name: im.name })}
          onclick={() => show(i)}><img src={im.url} alt="" draggable="false" /></button
        >
      {/each}
    </div>
  {/if}
</div>

<style>
  /* Over everything, the image on a dark scrim in both themes: an image is seen as it is, not on the theme's paper. The
     bar, the room of the image, the strip of thumbnails, in a column. */
  .viewer {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    flex-direction: column;
    background: rgba(8, 9, 12, 0.88);
    color: #e8e9ec;
    outline: none;
    /* Nothing to select: a quick second click zooms, never highlights the image (WebKit wants its prefix). */
    -webkit-user-select: none;
    user-select: none;
    animation: fade 0.16s var(--ease);
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 48px;
    padding: 0 12px 0 20px;
    font-size: 13px;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }
  .count {
    margin-right: 8px;
    color: #a3a6ad;
    font-variant-numeric: tabular-nums;
  }
  .zoom {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-right: 8px;
  }
  .round,
  .pct {
    display: grid;
    place-items: center;
    flex: none;
    width: 34px;
    height: 34px;
    border: none;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
    color: #e8e9ec;
    cursor: pointer;
  }
  .pct {
    width: auto;
    min-width: 58px;
    padding: 0 10px;
    border-radius: 17px;
    font: inherit;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .round:hover:not(:disabled),
  .pct:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.16);
  }
  .round:disabled,
  .pct:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .round:focus-visible,
  .pct:focus-visible,
  .mini:focus-visible {
    outline: 2px solid var(--p-ink, #8ab4ff);
    outline-offset: 2px;
  }
  /* The room of the image: it scrolls once the image is larger, the image in the middle while it is not (`margin: auto`
     keeps its edges in reach, which centring the box would not). */
  .middle {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .stage {
    flex: 1;
    min-width: 0;
    display: flex;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-color: rgba(255, 255, 255, 0.25) transparent;
  }
  .stage img {
    flex: none;
    margin: auto;
    border-radius: 6px;
    box-shadow: 0 18px 60px rgba(0, 0, 0, 0.5);
    /* A transparent PNG or an SVG drawn in black still reads. */
    background: repeating-conic-gradient(#2a2c31 0 25%, #222428 0 50%) 0 0 / 16px 16px;
    cursor: zoom-in;
  }
  /* Before its size is read: held in the room all the same. */
  .stage img:not(.sized) {
    max-width: calc(100% - 48px);
    max-height: calc(100% - 32px);
  }
  .stage.zoomed img {
    cursor: grab;
  }
  .stage.dragging,
  .stage.dragging img {
    cursor: grabbing;
  }
  .side {
    position: absolute;
    top: 50%;
    width: 40px;
    height: 40px;
    transform: translateY(-50%);
  }
  .prev {
    left: 16px;
  }
  .prev :global(svg) {
    transform: rotate(180deg);
  }
  .next {
    right: 16px;
  }
  /* The thumbnails, in the middle while they hold, scrolled once they do not (the auto margins of the ends). */
  .strip {
    flex: none;
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding: 8px 16px 12px;
    scrollbar-color: rgba(255, 255, 255, 0.25) transparent;
  }
  .mini {
    flex: none;
    width: 56px;
    height: 56px;
    padding: 0;
    overflow: hidden;
    border: none;
    border-radius: 7px;
    background: rgba(255, 255, 255, 0.06);
    opacity: 0.5;
    cursor: pointer;
    transition: opacity 0.15s var(--ease);
  }
  .mini:first-child {
    margin-left: auto;
  }
  .mini:last-child {
    margin-right: auto;
  }
  .mini:hover {
    opacity: 0.85;
  }
  .mini[aria-current='true'] {
    opacity: 1;
    box-shadow: 0 0 0 2px #e8e9ec;
  }
  .mini img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .viewer {
      animation: none;
    }
    .mini {
      transition: none;
    }
  }
</style>
