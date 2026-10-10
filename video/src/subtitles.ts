// The voice's subtitles, as WebVTT: each line cut into short pieces, timed by its words.

import type { Lang } from './lang';
import { FPS, type Placed, type PlacedLine } from './timeline';

/** Seconds as hh:mm:ss.mmm. */
export function stamp(seconds: number): string {
  const ms = Math.round(seconds * 1000);
  const p = (n: number, w = 2) => String(n).padStart(w, '0');
  return `${p(Math.floor(ms / 3600000))}:${p(Math.floor(ms / 60000) % 60)}:${p(Math.floor(ms / 1000) % 60)}.${p(ms % 1000, 3)}`;
}

/** A line's text cut after its punctuation into pieces of `max` characters at most. */
export function chunks(text: string, max: number): string[] {
  // Punctuation ends a clause before a space only: « .env » stays whole.
  const clauses = text.split(/(?<=[,:;.?!…])\s+/);
  const out: string[] = [];
  for (const c of clauses) {
    const last = out.at(-1);
    if (last !== undefined && last.length + 1 + c.length <= max) out[out.length - 1] = `${last} ${c}`;
    else if (c.length <= max) out.push(c);
    else {
      // A clause too long on its own: cut between words.
      let cur = '';
      for (const w of c.split(' ')) {
        if (cur && cur.length + 1 + w.length > max) {
          out.push(cur);
          cur = w;
        } else cur = cur ? `${cur} ${w}` : w;
      }
      out.push(cur);
    }
  }
  // A last piece too short to be read on its own (« terminé. ») stays with the one before.
  if (out.length > 1 && out[out.length - 1].length < MIN_PIECE) out.splice(-2, 2, `${out[out.length - 2]} ${out[out.length - 1]}`);
  return out;
}

/** Characters under which a piece would flash by. */
const MIN_PIECE = 15;

/** What a piece the eyes cannot take in costs: too short to read, or too long for the screen. */
const shape = (len: number, max: number) => 3 * Math.max(0, MIN_PIECE - len) + 3 * Math.max(0, len - max);

/** What a cut between two words costs when it does not follow a stop. */
const NO_STOP = 12;

/**
 * `text` in as many pieces as `like` has weights (or `like` pieces of the same weight): cut between words, each cut
 * where the weights before it have their share of the text, after a stop when there is one near, never leaving a piece
 * too short to read. The English of a line is cut like the French one, weighted by the time each French piece lasts, so
 * that each English piece keeps its French twin's time and a pace the eyes follow.
 */
export function fit(text: string, like: number | number[], max: number): string[] {
  const weights = typeof like === 'number' ? Array<number>(like).fill(1) : like;
  const n = weights.length;
  if (n <= 1) return [text];
  const words = text.split(' ');
  if (words.length < n) throw new Error(`Impossible de couper en ${n} morceaux : ${text}`);
  const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);
  // Where each word starts, and whether a stop ends the word before it.
  const start: number[] = [];
  let at = 0;
  for (const w of words) {
    start.push(at);
    at += w.length + 1;
  }
  const stop = (k: number) => /[,:;.?!…]$/.test(words[k - 1]);
  // Cut j falls where the weights up to the piece j have their share of the text.
  const target = weights.slice(0, -1).map((_, j) => (sum(weights.slice(0, j + 1)) / sum(weights)) * text.length);
  /** Characters of the piece from word `from` up to (not including) word `to`. */
  const size = (from: number, to: number) => (to < words.length ? start[to] - 1 : text.length) - start[from];
  // best[j][k]: the cheapest way to make the first j + 1 pieces, the next one starting at word k.
  const best: { cost: number; from: number }[][] = [];
  for (let j = 0; j < n - 1; j++) {
    best[j] = [];
    for (let k = j + 1; k < words.length; k++) {
      const cut = Math.abs(start[k] - 0.5 - target[j]) + (stop(k) ? 0 : NO_STOP);
      if (j === 0) {
        best[j][k] = { cost: cut + shape(size(0, k), max), from: 0 };
        continue;
      }
      for (let p = j; p < k; p++) {
        const prev = best[j - 1][p];
        if (!prev) continue;
        const cost = prev.cost + cut + shape(size(p, k), max);
        if (!best[j][k] || cost < best[j][k].cost) best[j][k] = { cost, from: p };
      }
    }
  }
  let k = -1;
  let cost = Infinity;
  best[n - 2].forEach((c, i) => {
    const all = c.cost + shape(size(i, words.length), max);
    if (all < cost) [k, cost] = [i, all];
  });
  const cuts: number[] = [];
  for (let j = n - 2; j >= 0; j--) {
    cuts.unshift(k);
    k = best[j][k].from;
  }
  return [0, ...cuts].map((from, i) => words.slice(from, cuts[i] ?? words.length).join(' '));
}

const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

export interface Cue {
  start: number;
  end: number;
  text: string;
}

/**
 * A line's pieces, each from its first word to its last one (the voice's words, matched by their rank). The English
 * has the French pieces' times, one for one: the voice is French, and the picture follows it.
 */
export function cuesOf(line: PlacedLine, max = 84, lang: Lang = 'fr'): Cue[] {
  const at = line.from / FPS;
  const parts = chunks(line.text, max);
  const total = parts.reduce((n, p) => n + p.split(' ').length, 0);
  const words = line.words;
  const time = (rank: number, edge: 'start' | 'end') => {
    if (!words.length) return edge === 'start' ? 0 : line.durationInFrames / FPS;
    const w = words[Math.min(words.length - 1, Math.round((rank / Math.max(1, total - 1)) * (words.length - 1)))];
    return w[edge];
  };
  let rank = 0;
  const cues = parts.map((text, i) => {
    const n = text.split(' ').length;
    const cue = {
      start: at + (i === 0 ? 0 : time(rank, 'start')),
      end: at + (i === parts.length - 1 ? Math.max(time(rank + n - 1, 'end'), words.at(-1)?.end ?? 0) : time(rank + n - 1, 'end')),
      text,
    };
    rank += n;
    return cue;
  });
  if (lang === 'fr') return cues;
  const english = fit(
    line.en,
    cues.map((c) => c.end - c.start),
    max,
  );
  return cues.map((c, i) => ({ ...c, text: english[i] }));
}

export function webvtt(timeline: readonly Placed[], lang: Lang = 'fr'): string {
  const cues = timeline.flatMap((s) => s.lines.flatMap((l) => cuesOf(l, 84, lang)));
  const body = cues.map((c, i) => `${i + 1}\n${stamp(c.start)} --> ${stamp(c.end)}\n${escape(c.text)}`).join('\n\n');
  return `WEBVTT\n\n${body}\n`;
}
