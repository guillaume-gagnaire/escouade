import { describe, expect, it } from 'vitest';
import { FREE, musicVolume, UNDER } from './mix';
import { FPS, GAP, TIMELINE, TOTAL_FRAMES } from './timeline';

const lines = [
  { from: 300, durationInFrames: 90 },
  { from: 390 + Math.round(GAP * FPS), durationInFrames: 60 },
];
const total = 900;
const vol = (f: number) => musicVolume(f, lines, total);

describe('musicVolume', () => {
  it('fades in over the first second', () => {
    expect(vol(0)).toBe(0);
    expect(vol(FPS / 2)).toBeGreaterThan(0);
    expect(vol(FPS / 2)).toBeLessThan(FREE);
    expect(vol(2 * FPS)).toBeCloseTo(FREE);
  });

  it('goes under the voice while it speaks, before its first word', () => {
    expect(vol(300)).toBeCloseTo(UNDER);
    expect(vol(350)).toBeCloseTo(UNDER);
    expect(UNDER).toBeLessThan(FREE / 2);
  });

  it('stays under through the breath between two lines', () => {
    const end = lines[1].from + lines[1].durationInFrames;
    for (let f = 300; f < end; f++) expect(vol(f), String(f)).toBeLessThan(UNDER + 0.02);
  });

  it('comes back up once the voice is over', () => {
    expect(vol(470 + 2 * FPS)).toBeCloseTo(FREE);
  });

  it('fades out with the picture', () => {
    expect(vol(total)).toBe(0);
    expect(vol(total - FPS)).toBeLessThan(vol(total - 2 * FPS));
  });

  it('stays under the voice from the first to the last word of every line of the video', () => {
    const all = TIMELINE.flatMap((s) => s.lines);
    for (const l of all)
      for (const w of l.words) {
        const f = l.from + Math.round(w.start * FPS);
        expect(musicVolume(f, all, TOTAL_FRAMES), `${l.key} « ${w.word} »`).toBeCloseTo(UNDER);
      }
  });
});
