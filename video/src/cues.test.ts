import { describe, expect, it } from 'vitest';
import { cuesOf } from './cues';
import { FPS, sceneOf, type Placed } from './timeline';

const scene: Placed = {
  id: 'board',
  title: 'T',
  titleEn: 'T',
  from: 600,
  durationInFrames: 300,
  cues: { a: 15, b: 120 },
  lines: [
    {
      key: 'board.a',
      id: 'a',
      text: 'À faire, en cours.',
      en: 'To do, in progress.',
      from: 615,
      durationInFrames: 90,
      words: [
        { word: 'À', start: 0, end: 0.1 },
        { word: 'faire,', start: 0.1, end: 0.5 },
        { word: 'en', start: 1, end: 1.1 },
        { word: 'cours.', start: 1.1, end: 1.5 },
      ],
    },
    { key: 'board.b', id: 'b', text: 'b', en: 'b', from: 720, durationInFrames: 60, words: [] },
  ],
};

describe('cues', () => {
  const c = cuesOf(scene);

  it('give a line’s start and end from the scene’s start', () => {
    expect([c.at('a'), c.end('a'), c.at('b'), c.end('b'), c.length]).toEqual([15, 105, 120, 180, 300]);
  });

  it('give the moment a word is said, whatever its accents and punctuation', () => {
    expect(c.word('a', 'cours')).toBe(15 + Math.round(1.1 * FPS));
    expect(c.word('a', 'a')).toBe(15);
  });

  it('find a word after its elision', () => {
    const s = {
      ...scene,
      lines: [
        {
          ...scene.lines[0],
          words: [
            { word: "d'effort", start: 0.5, end: 0.9 },
            { word: 'l’agent', start: 1, end: 1.4 },
          ],
        },
      ],
    };
    expect(cuesOf(s).word('a', 'effort')).toBe(15 + Math.round(0.5 * FPS));
    expect(cuesOf(s).word('a', 'agent')).toBe(15 + FPS);
  });

  it('take a whole word, never the start of another: « mode » is not « modèle »', () => {
    const s = {
      ...scene,
      lines: [
        {
          ...scene.lines[0],
          words: [
            { word: 'modèle,', start: 0.5, end: 0.9 },
            { word: 'mode', start: 2, end: 2.3 },
            { word: 'moitié-moitié,', start: 3, end: 3.6 },
          ],
        },
      ],
    };
    expect(cuesOf(s).word('a', 'mode')).toBe(15 + 2 * FPS);
    expect(cuesOf(s).word('a', 'moitié-moitié')).toBe(15 + 3 * FPS);
    expect(() => cuesOf(s).word('a', 'moitié')).toThrow(/moitié/);
  });

  it('refuse a line or a word the scene does not have', () => {
    expect(() => c.at('z')).toThrow(/z/);
    expect(() => c.word('a', 'tester')).toThrow(/tester/);
  });

  it('work on the real voice', () => {
    const board = cuesOf(sceneOf('board'));
    expect(board.word('switch', 'terminé')).toBeGreaterThan(board.word('switch', 'tester'));
  });
});
