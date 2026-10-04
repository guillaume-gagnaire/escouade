import { describe, expect, it } from 'vitest';
import { chime, click, SR, whoosh } from './sounds';

const rms = (x: Float32Array, from: number, to: number) => {
  let s = 0;
  for (let i = Math.round(from * SR); i < Math.round(to * SR); i++) s += x[i] ** 2;
  return Math.sqrt(s / ((to - from) * SR));
};

describe('chime', () => {
  const c = chime();

  it('is the app’s: 0.8 s, two notes, the second 130 ms after the first', () => {
    expect(c.length).toBe(Math.round(0.8 * SR));
    expect(rms(c, 0, 0.1)).toBeGreaterThan(0.05);
    // The second note starts and lifts the level again.
    expect(rms(c, 0.14, 0.2)).toBeGreaterThan(rms(c, 0.08, 0.12));
  });

  it('never clips and fades out', () => {
    expect(Math.max(...c.map(Math.abs))).toBeLessThanOrEqual(1);
    expect(rms(c, 0.7, 0.8)).toBeLessThan(rms(c, 0, 0.1) / 20);
  });
});

describe('click', () => {
  it('is a short tick, silent after 60 ms', () => {
    const k = click();
    expect(k.length).toBe(Math.round(0.08 * SR));
    expect(rms(k, 0, 0.01)).toBeGreaterThan(0.05);
    expect(rms(k, 0.06, 0.08)).toBeLessThan(0.005);
  });
});

describe('whoosh', () => {
  const w = whoosh();

  it('lasts 0.4 s, swells to its middle and dies away', () => {
    expect(w.length).toBe(Math.round(0.4 * SR));
    expect(rms(w, 0.15, 0.25)).toBeGreaterThan(3 * rms(w, 0, 0.04));
    expect(rms(w, 0.15, 0.25)).toBeGreaterThan(5 * rms(w, 0.36, 0.4));
  });

  it('never clips, and is the same at every render', () => {
    expect(Math.max(...w.map(Math.abs))).toBeLessThanOrEqual(1);
    expect(whoosh()).toEqual(w);
  });
});
