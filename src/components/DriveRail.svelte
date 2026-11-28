<script lang="ts" module>
  /** The height of a pin, and the least room between two on the rail. */
  export const PIN = 18;
  const GAP = 3;

  /**
   * Where each pin sits on a track `track` pixels high: the middle of the view the agent left, as a share of the page's
   * height then; never two over each other (the later one goes below), never past the track's end.
   */
  export function pinTops(pins: { n: number; y: number; height: number; view: number }[], track: number): Map<number, number> {
    const room = Math.max(0, track - PIN);
    const wanted = pins
      .map((p) => ({ n: p.n, at: p.height > 0 ? Math.min(1, Math.max(0, (p.y + p.view / 2) / p.height)) : 0 }))
      .sort((a, b) => a.at - b.at || a.n - b.n);
    const out = new Map<number, number>();
    let next = 0;
    for (const w of wanted) {
      const top = Math.min(room, Math.max(next, Math.round(w.at * track - PIN / 2)));
      out.set(w.n, top);
      next = top + PIN + GAP;
    }
    return out;
  }

  /** The part of the page on screen, on the track: its top and height in pixels (at least a pin's). */
  export function viewOnTrack(place: { y: number; height: number; view: number } | null, track: number) {
    if (!place || place.height <= 0 || track <= 0) return null;
    const h = Math.max(PIN, Math.min(track, (place.view / place.height) * track));
    const top = Math.min(track - h, Math.max(0, (place.y / place.height) * track));
    return { top: Math.round(top), height: Math.round(h) };
  }
</script>

<script lang="ts">
  // The rail of an agent's page (ESC-680, AB-8; docs/AGENT-PAGES.md § 12, direction A « Les repères »): Escouade's,
  // along the page and out of its webview. Each gesture of the agent's is a numbered pin, the same ③ as its row in the
  // conversation and the band's caption, where the agent had put the page; the part of the page on screen shows on its
  // track. A pin clicked takes the page back there. Where the page is comes from the core, read while the rail shows.
  import { onDestroy } from 'svelte';
  import { revisitPin } from '../lib/drives.svelte';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { tip } from '../lib/tip';
  import type { DrivePin, DrivePlace } from '../lib/types';

  let {
    pageKey,
    agent,
    pins,
    live = true,
  }: {
    /** The row: `page:p-12`. */
    pageKey: string;
    agent: string;
    pins: DrivePin[];
    /** The page shows: where it is is read only then. */
    live?: boolean;
  } = $props();

  let track = $state(0);
  let place = $state<DrivePlace | null>(null);
  const tops = $derived(pinTops(pins, track));
  const shown = $derived(viewOnTrack(place, track));
  const current = $derived(pins.length ? pins[pins.length - 1].n : null);

  let timer: ReturnType<typeof setInterval> | undefined;
  async function read() {
    try {
      place = await api.browserDrivePlace(pageKey);
    } catch {
      // The page is not there: the track alone.
    }
  }
  $effect(() => {
    clearInterval(timer);
    if (!live || !pins.length) return;
    void read();
    timer = setInterval(read, 700);
    return () => clearInterval(timer);
  });
  // A new pin: the page moved with it.
  $effect(() => {
    void pins.length;
    if (live) void read();
  });
  onDestroy(() => clearInterval(timer));
</script>

<nav class="rail" aria-label={t('devpage.drive.rail', { agent })} data-testid="drive-rail">
  <div class="trk" bind:clientHeight={track}>
    <span class="line" aria-hidden="true"></span>
    {#if shown}
      <span class="vp" aria-hidden="true" data-testid="drive-rail-view" style:top="{shown.top}px" style:height="{shown.height}px"></span>
    {/if}
    {#each pins as p (p.n)}
      <button
        class="pin"
        class:cur={p.n === current}
        style:top="{tops.get(p.n) ?? 0}px"
        aria-label={t('devpage.drive.pinTip', { n: p.n, what: p.what })}
        use:tip={{ text: t('devpage.drive.pinTip', { n: p.n, what: p.what }), aria: false, side: 'right' }}
        data-pin={p.n}
        onclick={() => revisitPin(pageKey, p.n)}>{p.n}</button
      >
    {/each}
  </div>
</nav>

<style>
  /* A narrow strip of the window beside the page (the design's « .pins »): the panel's ground, a hairline to the page. */
  .rail {
    flex: none;
    width: 26px;
    position: relative;
    background: var(--panel);
    box-shadow: inset -1px 0 0 var(--line);
  }
  .trk {
    position: absolute;
    inset: 10px 0;
  }
  .line {
    position: absolute;
    left: 12px;
    top: 0;
    bottom: 0;
    width: 2px;
    border-radius: 1px;
    background: var(--line-2);
  }
  /* The part of the page on screen. */
  .vp {
    position: absolute;
    left: 9px;
    width: 8px;
    border-radius: 4px;
    background: color-mix(in oklch, var(--p) 30%, transparent);
    transition:
      top 0.2s var(--ease),
      height 0.2s var(--ease);
  }
  .pin {
    position: absolute;
    left: 4px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 9px;
    background: var(--panel);
    box-shadow:
      0 0 0 2px var(--panel),
      inset 0 0 0 1.5px color-mix(in oklch, var(--p) 55%, transparent);
    color: var(--p-ink);
    font: inherit;
    font-size: 10px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    transition: top 0.2s var(--ease);
  }
  .pin:hover {
    background: color-mix(in oklch, var(--p) 18%, var(--panel));
  }
  .pin:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  /* The agent's last gesture: filled. */
  .pin.cur {
    background: var(--p);
    box-shadow: 0 0 0 2px var(--panel);
    color: #0b0c0e;
  }
  @media (prefers-reduced-motion: reduce) {
    .vp,
    .pin {
      transition: none;
    }
  }
</style>
