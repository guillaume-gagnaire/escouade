import { describe, expect, it } from 'vitest';
import { clamp } from './resize';

describe('clamp', () => {
  it('keeps a value that fits', () => {
    expect(clamp(240, 160, 500)).toBe(240);
  });
  it('holds a value to the bounds', () => {
    expect(clamp(100, 160, 500)).toBe(160);
    expect(clamp(900, 160, 500)).toBe(500);
  });
  it('lets the minimum win when the room is narrower than it', () => {
    expect(clamp(240, 160, 100)).toBe(160);
  });
});
