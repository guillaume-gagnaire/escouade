import { describe, expect, it } from 'vitest';
import { pointerAt, type PointerKey } from './Cursor';

const keys: PointerKey[] = [
  [10, 0, 0],
  [30, 100, 50, true],
  [50, 100, 150],
];

describe('pointerAt', () => {
  it('waits at its first key, then glides from key to key', () => {
    expect(pointerAt(0, keys)).toEqual({ x: 0, y: 0, click: 0 });
    expect(pointerAt(20, keys)).toMatchObject({ x: 50, y: 25 });
    expect(pointerAt(30, keys)).toMatchObject({ x: 100, y: 50 });
    expect(pointerAt(99, keys)).toMatchObject({ x: 100, y: 150 });
  });

  it('ripples for a moment after a click', () => {
    expect(pointerAt(29, keys).click).toBe(0);
    expect(pointerAt(30, keys).click).toBe(0);
    expect(pointerAt(37, keys).click).toBeCloseTo(0.5);
    expect(pointerAt(45, keys).click).toBe(0);
  });
});
