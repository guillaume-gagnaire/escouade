import { describe, expect, it } from 'vitest';
import { chunks, cuesOf, stamp, webvtt } from './subtitles';
import { FPS, TIMELINE, type PlacedLine } from './timeline';

describe('stamp', () => {
  it('writes WebVTT times', () => {
    expect(stamp(0)).toBe('00:00:00.000');
    expect(stamp(3723.5)).toBe('01:02:03.500');
  });
});

describe('chunks', () => {
  it('cut a long line at its punctuation into pieces of two short lines at most', () => {
    const text =
      "En disposition moitié-moitié, la conversation est à gauche, et à droite, les fichiers modifiés et leur diff, mis à jour pendant que l'agent écrit.";
    const parts = chunks(text, 84);
    expect(parts.join(' ')).toBe(text);
    for (const p of parts) expect(p.length).toBeLessThanOrEqual(84);
    expect(parts.length).toBeGreaterThan(1);
  });

  it('never cut inside a word like « .env »', () => {
    expect(chunks('avec tes fichiers .env, et ses propres ports pour les tests de bout en bout.', 30).join('|')).toContain('.env,');
  });

  it('leave no short piece on its own: it joins the one before', () => {
    const text = "Et maintenant, le tableau. Passe d'Agents à Tableau : à faire, en cours, à tester, terminé.";
    const parts = chunks(text, 84);
    expect(parts.join(' ')).toBe(text);
    for (const p of parts) expect(p.length, p).toBeGreaterThanOrEqual(15);
  });

  it('keep a short line whole', () => {
    expect(chunks('Et depuis ton téléphone.', 84)).toEqual(['Et depuis ton téléphone.']);
  });
});

describe('cuesOf', () => {
  const line: PlacedLine = {
    key: 'x.a',
    id: 'a',
    text: 'Un deux trois, quatre cinq six.',
    from: 30,
    durationInFrames: 90,
    words: ['Un', 'deux', 'trois,', 'quatre', 'cinq', 'six.'].map((word, i) => ({ word, start: i * 0.5, end: i * 0.5 + 0.4 })),
  };

  it('time each piece of a line by its words', () => {
    const cues = cuesOf(line, 16);
    expect(cues.map((c) => c.text)).toEqual(['Un deux trois,', 'quatre cinq six.']);
    expect(cues[0].start).toBeCloseTo(1);
    expect(cues[1].start).toBeCloseTo(1 + 1.5);
    expect(cues[1].end).toBeCloseTo(1 + 2.9);
  });
});

describe('webvtt', () => {
  const vtt = webvtt(TIMELINE);

  it('escapes what WebVTT reads as markup', () => {
    const line: PlacedLine = { key: 'x.a', id: 'a', text: 'Tom & Léa <3', from: 0, durationInFrames: 30, words: [] };
    const scene = { id: 'intro' as const, title: '', from: 0, durationInFrames: 30, cues: { a: 0 }, lines: [line] };
    expect(webvtt([scene])).toContain('Tom &amp; Léa &lt;3');
  });

  it('is a WebVTT file with every line of the voice, in order and without overlap', () => {
    expect(vtt.startsWith('WEBVTT\n\n')).toBe(true);
    const times = [...vtt.matchAll(/(\d\d):(\d\d):(\d\d)\.(\d{3}) --> (\d\d):(\d\d):(\d\d)\.(\d{3})/g)].map((m) => {
      const t = (h: string, mi: string, s: string, ms: string) => +h * 3600 + +mi * 60 + +s + +ms / 1000;
      return [t(m[1], m[2], m[3], m[4]), t(m[5], m[6], m[7], m[8])];
    });
    for (let i = 1; i < times.length; i++) expect(times[i][0]).toBeGreaterThanOrEqual(times[i - 1][1] - 0.001);
    const text = vtt
      .replace(/^WEBVTT\n\n/, '')
      .replace(/\d+\n[\d:.]+ --> [\d:.]+\n/g, '')
      .replace(/\n+/g, ' ')
      .trim();
    expect(text).toBe(TIMELINE.flatMap((s) => s.lines.map((l) => l.text)).join(' '));
    expect(times.at(-1)![1]).toBeLessThanOrEqual(TIMELINE.reduce((n, s) => n + s.durationInFrames, 0) / FPS);
  });
});
