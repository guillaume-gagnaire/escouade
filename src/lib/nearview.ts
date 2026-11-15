// Drawing as it comes near the screen (ESC-467): a long list (the files of a pull request, their parts) keeps a place of
// the right height for what is not drawn yet, and draws it once it comes within `margin` of its scrolling box. One
// IntersectionObserver per box and margin. Without one (a test's page), everything is near at once.

type Watch = { io: IntersectionObserver; calls: Map<Element, () => void> };
const watches = new Map<string, WeakMap<Element, Watch>>();

function watchOf(root: Element | null, margin: string): Watch | null {
  if (typeof IntersectionObserver === 'undefined') return null;
  let byRoot = watches.get(margin);
  if (!byRoot) watches.set(margin, (byRoot = new WeakMap()));
  const key = root ?? document.documentElement;
  let w = byRoot.get(key);
  if (!w) {
    const calls = new Map<Element, () => void>();
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (!e.isIntersecting) continue;
          const call = calls.get(e.target);
          calls.delete(e.target);
          io.unobserve(e.target);
          call?.();
        }
      },
      { root, rootMargin: margin },
    );
    w = { io, calls };
    byRoot.set(key, w);
  }
  return w;
}

/** Calls `onNear` once, when `node` comes within `margin` of `root` (its scrolling box; null: the window). */
export function nearView(node: Element, opts: { root: Element | null; onNear: () => void; margin?: string }) {
  const w = watchOf(opts.root, opts.margin ?? '800px 0px');
  if (!w) {
    opts.onNear();
    return {};
  }
  w.calls.set(node, opts.onNear);
  w.io.observe(node);
  return {
    destroy() {
      w.calls.delete(node);
      w.io.unobserve(node);
    },
  };
}
