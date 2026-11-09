// The split screen (ESC-392): the work area of a project cut into panes, side by side or one above the other, each
// showing one content (an agent, a terminal, a launch log or the editor). A binary tree: a leaf shows a content, a
// split cuts its place in two along `dir` at `ratio`. A content is never shown twice, so a leaf is named by its
// content's key. Pure functions: the state (`state.svelte.ts`) keeps the tree of each project in `ui.panes`.

export type PaneItem = { kind: 'agent' | 'term' | 'launch'; id: string } | { kind: 'editor' };

/** `row`: side by side (a on the left); `col`: one above the other (a on top). */
export type PaneDir = 'row' | 'col';

export type PaneNode = { leaf: PaneItem } | { dir: PaneDir; ratio: number; a: PaneNode; b: PaneNode };

export interface PaneLayout {
  root: PaneNode;
  /** The key of the active pane's content. */
  active: string;
}

/** Where a content dropped on a pane goes: beside it on one side, or in its place. */
export type PaneZone = 'left' | 'right' | 'top' | 'bottom' | 'center';

/** The most panes a project shows at once. */
export const MAX_PANES = 4;
/** The least a pane keeps, in pixels: what a composer or a terminal still holds. */
export const MIN_PANE_W = 320;
export const MIN_PANE_H = 200;
/** The ratio a split is held between. */
export const MIN_RATIO = 0.15;

export function itemKey(item: PaneItem): string {
  return item.kind === 'editor' ? 'editor' : `${item.kind}:${item.id}`;
}

export const isLeaf = (n: PaneNode): n is { leaf: PaneItem } => 'leaf' in n;

/** The contents shown, left to right then top to bottom. */
export function leaves(node: PaneNode): PaneItem[] {
  return isLeaf(node) ? [node.leaf] : [...leaves(node.a), ...leaves(node.b)];
}

export function has(layout: PaneLayout | null, item: PaneItem): boolean {
  const k = itemKey(item);
  return !!layout && leaves(layout.root).some((x) => itemKey(x) === k);
}

export function activeItem(layout: PaneLayout): PaneItem {
  const all = leaves(layout.root);
  return all.find((x) => itemKey(x) === layout.active) ?? all[0];
}

function mapLeaves(node: PaneNode, f: (item: PaneItem) => PaneNode): PaneNode {
  if (isLeaf(node)) return f(node.leaf);
  return { ...node, a: mapLeaves(node.a, f), b: mapLeaves(node.b, f) };
}

/** The tree without the leaf `key`: its sibling takes its parent's place. Null when it was the only leaf. */
function without(node: PaneNode, key: string): PaneNode | null {
  if (isLeaf(node)) return itemKey(node.leaf) === key ? null : node;
  const a = without(node.a, key);
  const b = without(node.b, key);
  if (!a) return b;
  if (!b) return a;
  return { ...node, a, b };
}

/** A layout of one pane is no split screen: null, and the caller shows that content alone. */
function normal(root: PaneNode | null, active: string): PaneLayout | null {
  if (!root || isLeaf(root)) return null;
  const all = leaves(root).map(itemKey);
  return { root, active: all.includes(active) ? active : all[0] };
}

/**
 * Opens `item` beside the pane `target` (on `zone`), or in its place (`center`). Without a layout, `target` is the
 * content shown alone. A content already shown elsewhere moves there (in its place, the two swap). `item` becomes the
 * active pane. Returns the layout, null when one pane is left (the content to show alone is then `item`).
 */
export function place(layout: PaneLayout | null, target: PaneItem, zone: PaneZone, item: PaneItem): PaneLayout | null {
  const tk = itemKey(target);
  const ik = itemKey(item);
  const root: PaneNode = layout?.root ?? { leaf: target };
  if (tk === ik) return layout ? { ...layout, active: ik } : null;
  if (zone === 'center') {
    const shown = leaves(root).some((x) => itemKey(x) === ik);
    const next = mapLeaves(root, (x) => {
      const k = itemKey(x);
      if (k === tk) return { leaf: item };
      if (shown && k === ik) return { leaf: target };
      return { leaf: x };
    });
    return normal(next, ik);
  }
  const rest = without(root, ik) ?? { leaf: target };
  if (!leaves(rest).some((x) => itemKey(x) === tk)) return normal(rest, ik);
  const dir: PaneDir = zone === 'left' || zone === 'right' ? 'row' : 'col';
  const first = zone === 'left' || zone === 'top';
  const next = mapLeaves(rest, (x) =>
    itemKey(x) === tk ? { dir, ratio: 0.5, a: first ? { leaf: item } : { leaf: x }, b: first ? { leaf: x } : { leaf: item } } : { leaf: x },
  );
  return normal(next, ik);
}

/** Closes the pane `key`. The active pane closed, the one in its place (its sibling's first) is active. */
export function close(layout: PaneLayout, key: string): PaneLayout | null {
  const root = without(layout.root, key);
  if (!root) return null;
  return normal(root, layout.active === key ? itemKey(leaves(siblingOf(layout.root, key) ?? root)[0]) : layout.active);
}

function siblingOf(node: PaneNode, key: string): PaneNode | null {
  if (isLeaf(node)) return null;
  if (isLeaf(node.a) && itemKey(node.a.leaf) === key) return node.b;
  if (isLeaf(node.b) && itemKey(node.b.leaf) === key) return node.a;
  return siblingOf(node.a, key) ?? siblingOf(node.b, key);
}

/** Only the contents `keep` says still exist: an agent archived, a terminal closed leave their pane. */
export function prune(layout: PaneLayout, keep: (item: PaneItem) => boolean): PaneLayout | null {
  let root: PaneNode | null = layout.root;
  for (const x of leaves(layout.root)) if (root && !keep(x)) root = without(root, itemKey(x));
  if (root === layout.root) return layout;
  return normal(root, layout.active);
}

/** The split at `path` (`a`/`b` from the root, `''` the root) set to `ratio`, held between the bounds. */
export function setRatio(layout: PaneLayout, path: string, ratio: number): PaneLayout {
  const r = Math.min(1 - MIN_RATIO, Math.max(MIN_RATIO, ratio));
  const go = (node: PaneNode, rest: string): PaneNode => {
    if (isLeaf(node)) return node;
    if (!rest) return { ...node, ratio: r };
    return rest[0] === 'a' ? { ...node, a: go(node.a, rest.slice(1)) } : { ...node, b: go(node.b, rest.slice(1)) };
  };
  return { ...layout, root: go(layout.root, path) };
}

/** The split holding the pane `key` turned the other way (side by side ↔ one above the other). */
export function flip(layout: PaneLayout, key: string): PaneLayout {
  const go = (node: PaneNode): PaneNode => {
    if (isLeaf(node)) return node;
    const here = [node.a, node.b].some((n) => isLeaf(n) && itemKey(n.leaf) === key);
    if (here) return { ...node, dir: node.dir === 'row' ? 'col' : 'row' };
    return { ...node, a: go(node.a), b: go(node.b) };
  };
  return { ...layout, root: go(layout.root) };
}

export interface PaneRect {
  key: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Where each pane lies in a unit square. */
export function rects(node: PaneNode, x = 0, y = 0, w = 1, h = 1): PaneRect[] {
  if (isLeaf(node)) return [{ key: itemKey(node.leaf), x, y, w, h }];
  if (node.dir === 'row')
    return [...rects(node.a, x, y, w * node.ratio, h), ...rects(node.b, x + w * node.ratio, y, w * (1 - node.ratio), h)];
  return [...rects(node.a, x, y, w, h * node.ratio), ...rects(node.b, x, y + h * node.ratio, w, h * (1 - node.ratio))];
}

/** The pane next to `key` on `side` (the one facing it most, the nearest first), null at the edge. */
export function neighbor(layout: PaneLayout, key: string, side: Exclude<PaneZone, 'center'>): string | null {
  const all = rects(layout.root);
  const me = all.find((r) => r.key === key);
  if (!me) return null;
  const eps = 1e-6;
  const overlap = (a0: number, a1: number, b0: number, b1: number) => Math.min(a1, b1) - Math.max(a0, b0);
  let best: { key: string; gap: number; over: number } | null = null;
  for (const r of all) {
    if (r.key === key) continue;
    let gap: number;
    let over: number;
    if (side === 'left' || side === 'right') {
      gap = side === 'left' ? me.x - (r.x + r.w) : r.x - (me.x + me.w);
      over = overlap(me.y, me.y + me.h, r.y, r.y + r.h);
    } else {
      gap = side === 'top' ? me.y - (r.y + r.h) : r.y - (me.y + me.h);
      over = overlap(me.x, me.x + me.w, r.x, r.x + r.w);
    }
    if (gap < -eps || over <= eps) continue;
    if (!best || gap < best.gap - eps || (Math.abs(gap - best.gap) <= eps && over > best.over)) best = { key: r.key, gap, over };
  }
  return best?.key ?? null;
}

/**
 * Whether `zone` may be dropped on a pane of `w` × `h` pixels, `count` panes shown: the center always; a side while
 * there is room for one more pane and both halves keep their least size.
 */
export function zoneFits(zone: PaneZone, w: number, h: number, count: number): boolean {
  if (zone === 'center') return true;
  if (count >= MAX_PANES) return false;
  return zone === 'left' || zone === 'right' ? w >= 2 * MIN_PANE_W : h >= 2 * MIN_PANE_H;
}

/** The zone of a pane under the pointer, from where it is in the pane (0–1 on each axis): its middle replaces. */
export function zoneAt(fx: number, fy: number): PaneZone {
  const dx = fx - 0.5;
  const dy = fy - 0.5;
  if (Math.abs(dx) < 0.17 && Math.abs(dy) < 0.17) return 'center';
  if (Math.abs(dx) >= Math.abs(dy)) return dx < 0 ? 'left' : 'right';
  return dy < 0 ? 'top' : 'bottom';
}

/** A layout read back from the settings, checked: anything malformed is dropped. */
export function parseLayout(raw: unknown): PaneLayout | null {
  const node = (v: unknown, depth: number): PaneNode | null => {
    if (!v || typeof v !== 'object' || depth > 8) return null;
    const o = v as Record<string, unknown>;
    if ('leaf' in o) {
      const l = o.leaf as Record<string, unknown> | null;
      if (!l || typeof l !== 'object') return null;
      if (l.kind === 'editor') return { leaf: { kind: 'editor' } };
      if ((l.kind === 'agent' || l.kind === 'term' || l.kind === 'launch') && typeof l.id === 'string')
        return { leaf: { kind: l.kind, id: l.id } };
      return null;
    }
    if ((o.dir !== 'row' && o.dir !== 'col') || typeof o.ratio !== 'number') return null;
    const a = node(o.a, depth + 1);
    const b = node(o.b, depth + 1);
    if (!a || !b) return a ?? b;
    return { dir: o.dir, ratio: Math.min(1 - MIN_RATIO, Math.max(MIN_RATIO, o.ratio)), a, b };
  };
  if (!raw || typeof raw !== 'object') return null;
  const o = raw as Record<string, unknown>;
  const root = node(o.root, 0);
  const keys = root ? leaves(root).map(itemKey) : [];
  if (new Set(keys).size !== keys.length) return null;
  return normal(root, typeof o.active === 'string' ? o.active : '');
}
