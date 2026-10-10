// Helpers for the elements whose size changes: the width one reports, the one a handle sets.

/** `value` held between `min` and `max` (`min` wins when the room is narrower than it). */
export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

/** Calls `onWidth` with the element's content width now and whenever it changes. */
export function observeWidth(node: HTMLElement, onWidth: (width: number) => void) {
  const ro = new ResizeObserver((entries) => {
    const e = entries[entries.length - 1];
    if (e) onWidth(e.contentRect.width);
  });
  ro.observe(node);
  return { destroy: () => ro.disconnect() };
}
