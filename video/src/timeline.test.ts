import { describe, expect, it } from 'vitest';
import type { SceneScript } from './script';
import { buildTimeline, FPS, GAP, LEAD, TAIL, TIMELINE, TOTAL_FRAMES } from './timeline';
import VOICE from './voice.json';

const script = [
  { id: 'intro', title: 'A', shows: [], lead: 1, lines: [{ id: 'a', text: 'a' }] },
  {
    id: 'chaos',
    title: 'B',
    shows: [],
    tail: 2,
    lines: [
      { id: 'x', text: 'x', hold: 1 },
      { id: 'y', text: 'y' },
    ],
  },
] satisfies SceneScript[];
const voice = {
  'intro.a': { duration: 2, words: [{ word: 'a', start: 0.1, end: 1.9 }] },
  'chaos.x': { duration: 3, words: [] },
  'chaos.y': { duration: 1.5, words: [] },
};

describe('buildTimeline', () => {
  const t = buildTimeline(script, voice);

  it('starts each line after the scene’s entry, the line before, its hold and a breath', () => {
    expect(t[0].cues).toEqual({ a: 1 * FPS });
    expect(t[1].cues).toEqual({ x: Math.round(LEAD * FPS), y: Math.round((LEAD + 3 + 1 + GAP) * FPS) });
  });

  it('makes a scene last as long as what it says, plus its exit', () => {
    expect(t[0].durationInFrames).toBe(Math.ceil((1 + 2 + TAIL) * FPS));
    expect(t[1].durationInFrames).toBe(Math.ceil((LEAD + 3 + 1 + GAP + 1.5 + 2) * FPS));
  });

  it('places the scenes end to end, and the lines in the whole video', () => {
    expect(t[0].from).toBe(0);
    expect(t[1].from).toBe(t[0].durationInFrames);
    expect(t[1].lines[1]).toMatchObject({ key: 'chaos.y', from: t[1].from + t[1].cues.y, durationInFrames: Math.ceil(1.5 * FPS) });
    expect(t[0].lines[0].words).toEqual([{ word: 'a', start: 0.1, end: 1.9 }]);
  });

  it('refuses a line without its voice', () => {
    expect(() => buildTimeline(script, { 'intro.a': voice['intro.a'] })).toThrow(/chaos\.x/);
  });
});

describe('the video', () => {
  it('takes the time its voice needs, with no limit', () => {
    const voiced = Object.values(VOICE as Record<string, { duration: number }>).reduce((n, c) => n + c.duration, 0);
    expect(TOTAL_FRAMES / FPS).toBeGreaterThan(voiced);
    expect(TOTAL_FRAMES).toBe(TIMELINE.reduce((n, s) => n + s.durationInFrames, 0));
  });

  it('never lets a line run over the next one or out of its scene', () => {
    for (const s of TIMELINE) {
      s.lines.forEach((l, i) => {
        const next = s.lines[i + 1];
        expect(l.from + l.durationInFrames, l.key).toBeLessThanOrEqual(next ? next.from : s.from + s.durationInFrames);
      });
    }
  });
});
