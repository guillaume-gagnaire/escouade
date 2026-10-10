import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { chunks, cuesOf, fit, stamp, webvtt } from './subtitles';
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
    en: 'One two three, four five six.',
    words: ['Un', 'deux', 'trois,', 'quatre', 'cinq', 'six.'].map((word, i) => ({ word, start: i * 0.5, end: i * 0.5 + 0.4 })),
  };

  it('time each piece of a line by its words', () => {
    const cues = cuesOf(line, 16);
    expect(cues.map((c) => c.text)).toEqual(['Un deux trois,', 'quatre cinq six.']);
    expect(cues[0].start).toBeCloseTo(1);
    expect(cues[1].start).toBeCloseTo(1 + 1.5);
    expect(cues[1].end).toBeCloseTo(1 + 2.9);
  });

  it('give the English the French pieces’ times, one for one', () => {
    const fr = cuesOf(line, 16);
    const en = cuesOf(line, 16, 'en');
    expect(en.map((c) => c.text)).toEqual(['One two three,', 'four five six.']);
    expect(en.map((c) => [c.start, c.end])).toEqual(fr.map((c) => [c.start, c.end]));
  });

  it('cut the English into as many pieces as the French has, whatever its own punctuation', () => {
    // No stop in the English: it is cut between words.
    const flat = { ...line, en: 'One two three four five six' };
    expect(cuesOf(flat, 16, 'en').map((c) => c.text)).toEqual(['One two three', 'four five six']);
    // More stops than the French: pieces are joined.
    const many = { ...line, en: 'One, two, three, four, five, six.' };
    const joined = cuesOf(many, 16, 'en').map((c) => c.text);
    expect(joined).toHaveLength(2);
    expect(joined.join(' ')).toBe(many.en);
    // A French line in one piece stays one piece.
    expect(cuesOf({ ...line, text: 'Un deux.', en: 'One, two. Three, four.' }, 84, 'en')).toHaveLength(1);
  });
});

describe('fit', () => {
  it('splits a text into exactly n pieces and keeps every word, in order', () => {
    const text = 'Switch to the Kanban: to do, in progress, to test, done. Then add a ticket with its criteria.';
    for (const n of [1, 2, 3, 4, 5]) {
      const parts = fit(text, n, 84);
      expect(parts, String(n)).toHaveLength(n);
      expect(parts.join(' ')).toBe(text);
    }
  });

  it('prefers to cut after a stop, near the middle, and leaves no piece too short to read', () => {
    expect(fit('Open the editor, then save the file. Done.', 2, 84)).toEqual(['Open the editor,', 'then save the file. Done.']);
  });
});

describe('webvtt', () => {
  const vtt = webvtt(TIMELINE);

  it('escapes what WebVTT reads as markup', () => {
    const line: PlacedLine = { key: 'x.a', id: 'a', text: 'Tom & Léa <3', en: 'Tom & Lea <3', from: 0, durationInFrames: 30, words: [] };
    const scene = { id: 'intro' as const, title: '', titleEn: '', from: 0, durationInFrames: 30, cues: { a: 0 }, lines: [line] };
    expect(webvtt([scene])).toContain('Tom &amp; Léa &lt;3');
    expect(webvtt([scene], 'en')).toContain('Tom &amp; Lea &lt;3');
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

describe('the English subtitles', () => {
  const fr = webvtt(TIMELINE);
  const en = webvtt(TIMELINE, 'en');
  const cuesOfFile = (vtt: string) =>
    vtt
      .replace(/^WEBVTT\n\n/, '')
      .split('\n\n')
      .map((block) => {
        const [id, time, ...text] = block.trim().split('\n');
        return { id, time, text: text.join(' ') };
      });

  it('have the French ones’ cues: the same number, the same numbers, the same times', () => {
    const f = cuesOfFile(fr);
    const e = cuesOfFile(en);
    expect(e.length).toBe(f.length);
    expect(e.map((c) => [c.id, c.time])).toEqual(f.map((c) => [c.id, c.time]));
  });

  it('say every English line of the script, in order, and nothing else', () => {
    expect(en.startsWith('WEBVTT\n\n')).toBe(true);
    const text = cuesOfFile(en)
      .map((c) => c.text)
      .join(' ');
    expect(text).toBe(TIMELINE.flatMap((s) => s.lines.map((l) => l.en)).join(' '));
  });

  it('give each line as many cues as the French gives it, at the same times', () => {
    for (const s of TIMELINE)
      for (const l of s.lines) {
        const f = cuesOf(l);
        const e = cuesOf(l, 84, 'en');
        expect(e.length, l.key).toBe(f.length);
        e.forEach((c, i) => expect([c.start, c.end], `${l.key} #${i}`).toEqual([f[i].start, f[i].end]));
      }
  });

  it('stay readable: short pieces, and at a pace the eyes follow while the French is spoken', () => {
    for (const s of TIMELINE)
      for (const l of s.lines)
        for (const c of cuesOf(l, 84, 'en')) {
          expect(c.text.length, `${l.key} « ${c.text} »`).toBeLessThanOrEqual(100);
          expect(c.text.length, `${l.key} « ${c.text} »`).toBeGreaterThanOrEqual(8);
          // On screen at least a second, at 26 characters per second at most.
          expect(c.end - c.start, `${l.key} « ${c.text} »`).toBeGreaterThanOrEqual(1);
          expect(c.text.length / (c.end - c.start), `${l.key} « ${c.text} »`).toBeLessThanOrEqual(26);
        }
  });
});

describe('the website’s subtitle files', () => {
  /** What `npm run site-images` wrote for the website. */
  const published = (file: string) => readFileSync(new URL(`../../website/public/${file}`, import.meta.url), 'utf8');

  it('are what the script says now, byte for byte: edit a line, run npm run site-images again', () => {
    expect(published('escouade.vtt'), 'escouade.vtt').toBe(webvtt(TIMELINE, 'fr'));
    expect(published('escouade.en.vtt'), 'escouade.en.vtt').toBe(webvtt(TIMELINE, 'en'));
  });
});
