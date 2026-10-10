<script lang="ts">
  import { clamp } from '../lib/resize';

  // The handle between a column and what follows it, on the column's right: dragged right, the column grows. It
  // only reports widths, the column's owner sets them and keeps the one chosen. `value` is the width shown, held
  // between `min` and `max` already.
  let {
    value,
    min,
    max,
    reset,
    label,
    step = 16,
    onresize,
    oncommit,
  }: {
    value: number;
    min: number;
    max: number;
    /** The width a double click goes back to. */
    reset: number;
    label: string;
    /** What an arrow key moves by. */
    step?: number;
    /** The width the column has now, for every move of the handle. */
    onresize: (width: number) => void;
    /** The width the column keeps: once the handle is let go, or after a key or a double click. */
    oncommit: (width: number) => void;
  } = $props();

  // The drag in progress: not reactive, a move must not cost more than the column's own width changing.
  let gesture: { id: number; x: number; from: number; last: number; moved: boolean } | null = null;
  let dragging = $state(false);

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    // A press while a drag is still on means the release of that drag never came: it is over, and this one starts.
    finish();
    try {
      // The moves keep coming when the pointer leaves the handle, over the code or out of the window.
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    } catch {
      // Refused (the pointer is gone already): the drag follows the events that still reach the handle.
    }
    gesture = { id: e.pointerId, x: e.clientX, from: value, last: value, moved: false };
    dragging = true;
  }

  function move(e: PointerEvent) {
    if (!gesture || e.pointerId !== gesture.id) return;
    // The button was let go where no event came back to the handle: the drag is over, not stuck to the pointer.
    if (e.buttons === 0) {
      finish();
      return;
    }
    // Whole pixels: a scaled screen gives the pointer fractions.
    const width = clamp(Math.round(gesture.from + e.clientX - gesture.x), min, max);
    if (width === gesture.last) return;
    gesture.last = width;
    gesture.moved = true;
    onresize(width);
  }

  function end(e: PointerEvent) {
    if (gesture && e.pointerId === gesture.id) finish();
  }

  // Ends the drag in progress, whatever ended it: the release, a cancelled pointer, a lost capture, the window losing the
  // focus (a press released over another application sends nothing here), or a drag found stuck.
  function finish() {
    if (!gesture) return;
    const { last, moved } = gesture;
    gesture = null;
    dragging = false;
    // A click that moved nothing keeps nothing; once the column was moved, the width it ends at is kept, back at its start or not.
    if (moved) oncommit(last);
  }

  function set(width: number) {
    const next = clamp(width, min, max);
    if (next === value) return;
    onresize(next);
    oncommit(next);
  }

  function key(e: KeyboardEvent) {
    // The shortcuts with a modifier are the app's.
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    const to = { ArrowLeft: value - step, ArrowRight: value + step, Home: min, End: max }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    set(to);
  }
</script>

<svelte:window onblur={finish} />

<!-- A separator that takes the focus is a widget (the window splitter pattern): Svelte only knows the static one. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="splitter"
  class:dragging
  role="separator"
  aria-orientation="vertical"
  aria-label={label}
  aria-valuenow={value}
  aria-valuemin={min}
  aria-valuemax={max}
  tabindex="0"
  onpointerdown={down}
  onpointermove={move}
  onpointerup={end}
  onpointercancel={end}
  onlostpointercapture={end}
  ondblclick={() => set(reset)}
  onkeydown={key}
></div>

<style>
  .splitter {
    position: relative;
    /* Above the neighbours, whose own stacking (the editor's) would otherwise cover the grip. */
    z-index: 1;
    flex: none;
    width: 1px;
    background: var(--line);
    cursor: col-resize;
    /* A drag must not scroll or select. */
    touch-action: none;
    user-select: none;
    outline: none;
  }
  /* The grip is wider than the line it draws, which is too thin to catch with a mouse; it leans on the code's side so
     the column's scrollbar stays within reach. */
  .splitter::before {
    content: '';
    position: absolute;
    inset: 0 -4px 0 -2px;
  }
  .splitter::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -1px;
    width: 3px;
    background: var(--accent);
    opacity: 0;
  }
  .splitter:hover::after {
    opacity: 0.5;
  }
  .splitter:focus-visible::after,
  .splitter.dragging::after {
    opacity: 1;
  }
  @media (prefers-reduced-motion: no-preference) {
    .splitter::after {
      transition: opacity 0.12s;
    }
  }
</style>
