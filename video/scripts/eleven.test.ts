import { describe, expect, it } from 'vitest';
import { mp3Duration, similarity, wordsOf } from './eleven';

describe('wordsOf', () => {
  it('groups the characters of an alignment into timed words', () => {
    const text = 'Un  onglet, deux.';
    const chars = [...text];
    const words = wordsOf({
      characters: chars,
      character_start_times_seconds: chars.map((_, i) => i * 0.1),
      character_end_times_seconds: chars.map((_, i) => i * 0.1 + 0.1),
    });
    expect(words.map((w) => w.word)).toEqual(['Un', 'onglet,', 'deux.']);
    expect(words[1].start).toBeCloseTo(0.4);
    expect(words[1].end).toBeCloseTo(1.1);
    expect(words[2].end).toBeCloseTo(1.7);
  });
});

describe('similarity', () => {
  it('is 1 for the same words, whatever the case, accents and punctuation', () => {
    expect(similarity("Contrôle J t'emmène au prochain agent.", 'controle j temmene au prochain agent')).toBe(1);
  });

  it('hears numbers written in figures and plurals that sound the same', () => {
    expect(similarity('Cinq projets, douze agents… et trente terminaux ?', '5 projets, 12 agents et 30 terminal')).toBe(1);
    expect(similarity('leurs boucles et leur coût', 'leur boucle et leurs coûts')).toBe(1);
  });

  it('counts the words heard in order, out of those expected', () => {
    expect(similarity('un deux trois quatre', 'un trois quatre')).toBe(0.75);
    expect(similarity('un deux trois quatre', 'quatre trois deux un')).toBe(0.25);
    expect(similarity('un deux', '')).toBe(0);
  });
});

/** A CBR MPEG-1 Layer III frame at 128 kb/s, 44.1 kHz: 417 bytes (418 padded), 1152 samples. */
function frame(padded = false) {
  const b = new Uint8Array(padded ? 418 : 417);
  b.set([0xff, 0xfb, 0x90 | (padded ? 0x02 : 0), 0x64]);
  return b;
}

describe('mp3Duration', () => {
  it('adds up the frames after the ID3 tag', () => {
    const tag = new Uint8Array(10 + 20);
    tag.set([0x49, 0x44, 0x33, 4, 0, 0, 0, 0, 0, 20]);
    const frames = [frame(), frame(true), frame(), frame()];
    const buf = new Uint8Array(tag.length + frames.reduce((n, f) => n + f.length, 0));
    let o = 0;
    for (const part of [tag, ...frames]) {
      buf.set(part, o);
      o += part.length;
    }
    expect(mp3Duration(buf)).toBeCloseTo((4 * 1152) / 44100, 6);
  });
});
