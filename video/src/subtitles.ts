// The voice's subtitles, as WebVTT: each line cut into short pieces, timed by its words.

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

const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

export interface Cue {
  start: number;
  end: number;
  text: string;
}

/** A line's pieces, each from its first word to its last one (the voice's words, matched by their rank). */
export function cuesOf(line: PlacedLine, max = 84): Cue[] {
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
  return parts.map((text, i) => {
    const n = text.split(' ').length;
    const cue = {
      start: at + (i === 0 ? 0 : time(rank, 'start')),
      end: at + (i === parts.length - 1 ? Math.max(time(rank + n - 1, 'end'), words.at(-1)?.end ?? 0) : time(rank + n - 1, 'end')),
      text,
    };
    rank += n;
    return cue;
  });
}

export function webvtt(timeline: readonly Placed[]): string {
  const cues = timeline.flatMap((s) => s.lines.flatMap((l) => cuesOf(l)));
  const body = cues.map((c, i) => `${i + 1}\n${stamp(c.start)} --> ${stamp(c.end)}\n${escape(c.text)}`).join('\n\n');
  return `WEBVTT\n\n${body}\n`;
}
