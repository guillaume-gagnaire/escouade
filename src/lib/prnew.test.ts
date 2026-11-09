import { describe, expect, it } from 'vitest';
import { freshIn, newCount, opened, readSeen, seenOf, type PrSeen } from './prnew';

const none = new Set<number>();

describe('the pull requests new since the PR tab was last opened (ESC-403)', () => {
  it('counts nothing at the first reading of a project', () => {
    const s = readSeen(undefined, [1, 2, 37], none, false);
    expect(newCount(s)).toBe(0);
    expect(newCount(undefined)).toBe(0);
  });

  it('counts the open ones that came after, not those Escouade opened', () => {
    let s = readSeen(undefined, [3, 4], none, false);
    s = readSeen(s, [3, 4, 5, 6], new Set([6]), false);
    expect(newCount(s)).toBe(1);
    // Escouade's stays its own even once its agent or ticket is gone.
    s = readSeen(s, [3, 4, 5, 6, 7], none, false);
    expect(newCount(s)).toBe(2);
    expect(s.own).toContain(6);
  });

  it('forgets one closed before it was seen, and one older that comes back up the list', () => {
    let s = readSeen(undefined, [10, 11], none, false);
    s = readSeen(s, [10, 11, 12], none, false);
    expect(newCount(s)).toBe(1);
    s = readSeen(s, [11], none, false);
    expect(newCount(s)).toBe(0);
    // #2, older than the list's 30, comes back when it is updated: not new.
    s = readSeen(s, [2, 11], none, false);
    expect(newCount(s)).toBe(0);
  });

  it('counts the first ones of a project read empty', () => {
    let s = readSeen(undefined, [], none, false);
    s = readSeen(s, [1], none, false);
    expect(newCount(s)).toBe(1);
  });

  it('opening the tab sets it back to zero and marks them in the list until the next opening', () => {
    let s = readSeen(undefined, [3], none, false);
    s = readSeen(s, [3, 4, 5], none, false);
    s = opened(s)!;
    expect(newCount(s)).toBe(0);
    expect(freshIn(s)).toEqual(new Set([4, 5]));
    // Still marked while the tab is shown, and those that come then join them.
    s = readSeen(s, [3, 4, 5, 8], none, true);
    expect(newCount(s)).toBe(0);
    expect(freshIn(s)).toEqual(new Set([4, 5, 8]));
    // The next opening: nothing new since, nothing marked.
    s = opened(s)!;
    expect(freshIn(s)).toEqual(new Set());
    expect(opened(undefined)).toBeUndefined();
  });

  it('a first reading made with the tab shown counts and marks nothing', () => {
    const s = readSeen(undefined, [3, 4], none, true);
    expect(newCount(s)).toBe(0);
    expect(freshIn(s)).toEqual(new Set());
  });

  it('what was seen is read back whole after a restart, and anything else is ignored', () => {
    let s = readSeen(undefined, [3], none, false);
    s = readSeen(s, [3, 4], none, false);
    const back = seenOf(JSON.parse(JSON.stringify(s)));
    expect(newCount(back)).toBe(1);
    expect(seenOf(null)).toBeUndefined();
    expect(seenOf({ mark: 'x' })).toBeUndefined();
    expect(seenOf({ mark: 3, open: [4, 'y'], own: [], fresh: [] } as unknown)).toEqual({
      mark: 3,
      open: [4],
      own: [],
      fresh: [],
    } satisfies PrSeen);
  });
});
