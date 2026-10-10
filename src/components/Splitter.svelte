<script lang="ts">
  import { clamp } from '../lib/resize';

  // The handle between a column and what follows it, on the column's right: dragged right, the column grows. It
  // only reports widths, the column's owner sets them and keeps the one chosen. `value` is the width shown, held
  // between `min` and `max` already. On the vertical axis (`axis: 'y'`) it is the handle under a block: dragged down,
  // the block grows, and what it reports and keeps are heights.
  let {
    value,
    min,
    max,
    reset,
    onreset,
    label,
    title,
    axis = 'x',
    step = 16,
    onresize,
    oncommit,
  }: {
    value: number;
    min: number;
    max: number;
    /** The size a double click goes back to. */
    reset?: number;
    /** What going back is, when the owner keeps no size to go back to (the block is of its natural size): instead of `reset`. */
    onreset?: () => void;
    label: string;
    /** Its tooltip. */
    title?: string;
    /** 'x': the right edge of a column, sized in width. 'y': the bottom edge of a block, sized in height. */
    axis?: 'x' | 'y';
    /** What an arrow key moves by. */
    step?: number;
    /** The size the block has now, for every move of the handle. */
    onresize: (size: number) => void;
    /** The size the block keeps: once the handle is let go, or after a key or a double click. */
    oncommit: (size: number) => void;
  } = $props();
  const vertical = $derived(axis === 'y');

  // The drag in progress: not reactive, a move must not cost more than the column's own width changing.
  let gesture: { id: number; at: number; from: number; last: number; moved: boolean } | null = null;
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
    gesture = { id: e.pointerId, at: vertical ? e.clientY : e.clientX, from: value, last: value, moved: false };
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
    const width = clamp(Math.round(gesture.from + (vertical ? e.clientY : e.clientX) - gesture.at), min, max);
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

  /** Back to the usual size: the owner's way if it has one, else the size it gave. */
  function back() {
    if (onreset) onreset();
    else if (reset !== undefined) set(reset);
  }

  function key(e: KeyboardEvent) {
    // The shortcuts with a modifier are the app's.
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    // Under a block, Enter and Escape go back to the usual size, as a double click does.
    if (vertical && (e.key === 'Enter' || e.key === 'Escape')) {
      e.preventDefault();
      back();
      return;
    }
    const keys: Record<string, number> = vertical
      ? { ArrowUp: value - step, ArrowDown: value + step, Home: min, End: max }
      : { ArrowLeft: value - step, ArrowRight: value + step, Home: min, End: max };
    const to = keys[e.key];
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
  class:row={vertical}
  class:dragging
  role="separator"
  aria-orientation={vertical ? 'horizontal' : 'vertical'}
  aria-label={label}
  {title}
  aria-valuenow={value}
  aria-valuemin={min}
  aria-valuemax={max}
  tabindex="0"
  onpointerdown={down}
  onpointermove={move}
  onpointerup={end}
  onpointercancel={end}
  onlostpointercapture={end}
  ondblclick={back}
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
  /* Under a block (the plan of a conversation): a band across it, with a grip of 36 x 3 px in its middle. Half of it
     overlaps what follows, so the block keeps its size. */
  .splitter.row {
    width: auto;
    height: 10px;
    margin-bottom: -5px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    cursor: row-resize;
  }
  .splitter.row::before {
    inset: 0;
  }
  .splitter.row::after {
    position: static;
    width: 36px;
    height: 3px;
    border-radius: 2px;
    background: var(--line2);
    opacity: 1;
  }
  .splitter.row:hover {
    background: color-mix(in oklch, var(--accent) 12%, transparent);
  }
  .splitter.row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .splitter.row:focus-visible::after,
  .splitter.row.dragging::after {
    background: var(--accent);
  }
</style>
