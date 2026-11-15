// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { lookDiff, sameValue, toSrgb } from './srgb';

describe('toSrgb', () => {
  it('reads rgb(), with commas or spaces, and its alpha', () => {
    expect(toSrgb('rgb(21, 23, 28)')).toEqual([21, 23, 28, 255]);
    expect(toSrgb('rgb(21 23 28 / 50%)')).toEqual([21, 23, 28, 128]);
    expect(toSrgb('rgba(0, 0, 0, 0)')).toEqual([0, 0, 0, 0]);
    expect(toSrgb('transparent')).toEqual([0, 0, 0, 0]);
  });

  it('knows the primaries and the white of Lab, Oklab and their polar forms', () => {
    expect(toSrgb('oklch(1 0 0)')).toEqual([255, 255, 255, 255]);
    expect(toSrgb('oklab(0 0 0)')).toEqual([0, 0, 0, 255]);
    expect(toSrgb('lab(100 0 0)')).toEqual([255, 255, 255, 255]);
    expect(toSrgb('lab(0 0 0)')).toEqual([0, 0, 0, 255]);
    expect(toSrgb('oklch(0.628 0.2577 29.23)')).toEqual([255, 0, 0, 255]);
    expect(toSrgb('lab(54.29 80.8 69.89)')).toEqual([255, 0, 0, 255]);
    expect(toSrgb('lch(54.29 106.84 40.85)')).toEqual([255, 0, 0, 255]);
    expect(toSrgb('color(srgb 1 0.5 0)')).toEqual([255, 128, 0, 255]);
  });

  it('gives the same sRGB to the two writings of the preview’s tint', () => {
    const a = toSrgb('oklch(0.84 0.14 82)')!;
    const b = toSrgb('lab(81.57 12.31 61.40)')!;
    const c = toSrgb('oklch(0.840003 0.140009 82.0039)')!;
    for (const x of [b, c]) a.forEach((v, i) => expect(Math.abs(v - x[i])).toBeLessThanOrEqual(1));
  });

  it('is null on what is not a colour', () => {
    expect(toSrgb('12px')).toBeNull();
    expect(toSrgb('var(--p)')).toBeNull();
  });
});

describe('sameValue', () => {
  it('accepts a unit off in a channel, not two', () => {
    expect(sameValue('rgb(10, 20, 30)', 'rgb(11, 19, 30)')).toBe(true);
    expect(sameValue('rgb(10, 20, 30)', 'rgb(12, 20, 30)')).toBe(false);
  });

  it('compares the colours of a shadow channel by channel and the rest as written', () => {
    const a = 'oklch(0.2 0 0 / 0.3) 0px 8px 24px 0px, rgba(0, 0, 0, 0.5) 0px 0px 0px 1px inset';
    const b = 'rgba(0, 0, 0, 0.3) 0px 8px 24px 0px, rgb(0 0 0 / 0.5) 0px 0px 0px 1px inset';
    expect(sameValue(a, a)).toBe(true);
    expect(sameValue('rgba(0, 0, 0, 0.3) 0px 8px 24px 0px', 'rgba(0, 0, 0, 0.3) 0px 8px 25px 0px')).toBe(false);
    expect(sameValue('rgba(0, 0, 0, 0.3) 0px 0px 0px 1px inset', 'none')).toBe(false);
    expect(sameValue('none', 'rgba(0, 0, 0, 0.3) 0px 0px 0px 1px inset')).toBe(false);
    expect(sameValue(b, b)).toBe(true);
  });

  it('keeps what is not a colour exact: a weight, a keyword', () => {
    expect(sameValue('500 12px Inter', '550 12px Inter')).toBe(false);
    expect(sameValue('none', 'none')).toBe(true);
  });
});

describe('lookDiff', () => {
  it('walks objects and arrays and names where they differ, colours in sRGB', () => {
    const a = { background: 'oklch(0.84 0.14 82)', shadow: 'none', font: '500 12px Inter', xs: ['rgb(1, 2, 3)'] };
    const b = { background: 'lab(81.57 12.31 61.40)', shadow: 'none', font: '550 12px Inter', xs: ['rgb(1, 2, 4)'] };
    expect(lookDiff(a, b)).toEqual(['font: 500 12px Inter  ≠  550 12px Inter']);
    expect(lookDiff(a, { ...b, font: a.font })).toEqual([]);
    expect(lookDiff({ x: 'rgb(0, 0, 0)' }, { x: 'rgb(0, 0, 3)' })[0]).toContain('x: rgb(0 0 0)  ≠  rgb(0 0 3)');
  });
});
