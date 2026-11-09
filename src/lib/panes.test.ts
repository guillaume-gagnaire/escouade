import { describe, expect, it } from 'vitest';
import {
  activeItem,
  close,
  flip,
  has,
  itemKey,
  leaves,
  MAX_PANES,
  neighbor,
  parseLayout,
  place,
  prune,
  rects,
  setRatio,
  zoneAt,
  zoneFits,
  type PaneItem,
  type PaneLayout,
} from './panes';

const ag = (id: string): PaneItem => ({ kind: 'agent', id });
const tm = (id: string): PaneItem => ({ kind: 'term', id });
const keys = (l: PaneLayout | null) => (l ? leaves(l.root).map(itemKey) : []);

describe('place', () => {
  it('opens a content beside the one shown alone, active', () => {
    const l = place(null, ag('a'), 'right', tm('t'));
    expect(l?.root).toEqual({ dir: 'row', ratio: 0.5, a: { leaf: ag('a') }, b: { leaf: tm('t') } });
    expect(l?.active).toBe('term:t');
  });

  it('puts it first on the left and on top', () => {
    expect(keys(place(null, ag('a'), 'left', ag('b')))).toEqual(['agent:b', 'agent:a']);
    const top = place(null, ag('a'), 'top', ag('b'));
    expect(top?.root).toMatchObject({ dir: 'col', a: { leaf: ag('b') } });
  });

  it('splits only the pane aimed at, making more than two panes', () => {
    const two = place(null, ag('a'), 'right', ag('b'))!;
    const three = place(two, ag('b'), 'bottom', tm('t'))!;
    expect(keys(three)).toEqual(['agent:a', 'agent:b', 'term:t']);
    expect(three.root).toMatchObject({ dir: 'row', a: { leaf: ag('a') }, b: { dir: 'col', a: { leaf: ag('b') }, b: { leaf: tm('t') } } });
  });

  it('replaces in the middle, and the one shown alone gives no layout', () => {
    expect(place(null, ag('a'), 'center', ag('b'))).toBeNull();
    const two = place(null, ag('a'), 'right', ag('b'))!;
    expect(keys(place(two, ag('a'), 'center', tm('t')))).toEqual(['term:t', 'agent:b']);
  });

  it('never shows a content twice: in the middle the two swap, on a side it moves', () => {
    const two = place(null, ag('a'), 'right', ag('b'))!;
    expect(keys(place(two, ag('a'), 'center', ag('b')))).toEqual(['agent:b', 'agent:a']);
    const three = place(two, ag('b'), 'right', tm('t'))!;
    const moved = place(three, ag('a'), 'bottom', tm('t'))!;
    expect(keys(moved)).toEqual(['agent:a', 'term:t', 'agent:b']);
    // Moved beside the only other pane: still two.
    expect(keys(place(two, ag('a'), 'left', ag('b')))).toEqual(['agent:b', 'agent:a']);
  });

  it('dropped on itself, only becomes active', () => {
    const two = place(null, ag('a'), 'right', ag('b'))!;
    expect(place(two, ag('a'), 'left', ag('a'))).toEqual({ ...two, active: 'agent:a' });
  });
});

describe('close and prune', () => {
  const three = place(place(null, ag('a'), 'right', ag('b')), ag('b'), 'bottom', tm('t'))!;

  it('gives the place to the sibling, and the active pane to it', () => {
    const l = close(three, 'term:t')!;
    expect(keys(l)).toEqual(['agent:a', 'agent:b']);
    expect(l.active).toBe('agent:b');
    expect(close({ ...three, active: 'agent:a' }, 'agent:b')?.active).toBe('agent:a');
  });

  it('down to one pane, there is no layout left', () => {
    expect(close(close(three, 'term:t')!, 'agent:a')).toBeNull();
  });

  it('drops the contents gone, and keeps the same layout when none is', () => {
    expect(prune(three, () => true)).toBe(three);
    expect(keys(prune(three, (x) => x.kind !== 'term'))).toEqual(['agent:a', 'agent:b']);
    expect(prune(three, (x) => x.kind === 'term')).toBeNull();
  });
});

describe('ratio, flip, geometry', () => {
  const two = place(null, ag('a'), 'right', tm('t'))!;

  it('holds the ratio between its bounds', () => {
    expect(setRatio(two, '', 0.7).root).toMatchObject({ ratio: 0.7 });
    expect(setRatio(two, '', 0.01).root).toMatchObject({ ratio: 0.15 });
    expect(setRatio(two, '', 2).root).toMatchObject({ ratio: 0.85 });
  });

  it('turns the split of a pane the other way', () => {
    expect(flip(two, 'term:t').root).toMatchObject({ dir: 'col' });
  });

  it('finds the pane next to another', () => {
    const three = place(two, tm('t'), 'bottom', ag('b'))!;
    expect(rects(three.root).map((r) => r.key)).toEqual(['agent:a', 'term:t', 'agent:b']);
    expect(neighbor(three, 'agent:a', 'right')).toBe('term:t');
    expect(neighbor(three, 'term:t', 'bottom')).toBe('agent:b');
    expect(neighbor(three, 'agent:b', 'left')).toBe('agent:a');
    expect(neighbor(three, 'agent:a', 'left')).toBeNull();
  });

  it('knows its active content', () => {
    expect(activeItem(two)).toEqual(tm('t'));
    expect(has(two, ag('a'))).toBe(true);
    expect(has(two, ag('z'))).toBe(false);
    expect(has(null, ag('a'))).toBe(false);
  });
});

describe('zones', () => {
  it('reads the zone from where the pointer is', () => {
    expect(zoneAt(0.5, 0.5)).toBe('center');
    expect(zoneAt(0.9, 0.5)).toBe('right');
    expect(zoneAt(0.1, 0.4)).toBe('left');
    expect(zoneAt(0.5, 0.05)).toBe('top');
    expect(zoneAt(0.45, 0.95)).toBe('bottom');
  });

  it('opens beside only with room and under the most panes', () => {
    expect(zoneFits('right', 700, 400, 1)).toBe(true);
    expect(zoneFits('right', 500, 400, 1)).toBe(false);
    expect(zoneFits('bottom', 500, 380, 1)).toBe(false);
    expect(zoneFits('bottom', 500, 420, 1)).toBe(true);
    expect(zoneFits('left', 2000, 2000, MAX_PANES)).toBe(false);
    expect(zoneFits('center', 10, 10, MAX_PANES)).toBe(true);
  });
});

describe('parseLayout', () => {
  it('reads back what was written', () => {
    const l = place(place(null, ag('a'), 'right', tm('t')), tm('t'), 'bottom', { kind: 'editor' })!;
    expect(parseLayout(JSON.parse(JSON.stringify(l)))).toEqual(l);
  });

  it('drops what is malformed', () => {
    expect(parseLayout(null)).toBeNull();
    expect(parseLayout({ root: { leaf: ag('a') }, active: 'agent:a' })).toBeNull();
    expect(parseLayout({ root: { dir: 'row', ratio: 0.5, a: { leaf: ag('a') }, b: { leaf: ag('a') } } })).toBeNull();
    const bad = parseLayout({ root: { dir: 'row', ratio: 0.5, a: { leaf: ag('a') }, b: { dir: 'x' } }, active: 'agent:a' });
    expect(bad).toBeNull();
    const fixed = parseLayout({ root: { dir: 'row', ratio: 9, a: { leaf: ag('a') }, b: { leaf: tm('t') } }, active: 'nope' });
    expect(fixed).toEqual({ root: { dir: 'row', ratio: 0.85, a: { leaf: ag('a') }, b: { leaf: tm('t') } }, active: 'agent:a' });
  });
});
