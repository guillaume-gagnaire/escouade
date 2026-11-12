// The timeline, from the script and the length of its voice: a scene lasts as long as what it says.

import { lineKey, SCRIPT, spoken, type SceneId, type SceneScript } from './script';
import VOICE from './voice.json';

export type { SceneId } from './script';

export const FPS = 30;
/** Seconds: a scene's entry before its first line, the breath between two lines, its exit after the last one.
 *  Short: the voice carries the pace, the picture follows it. */
export const LEAD = 0.35;
export const GAP = 0.25;
export const TAIL = 0.5;

export interface TimedWord {
  word: string;
  /** Seconds from the start of its line. */
  start: number;
  end: number;
}

export interface VoiceClip {
  /** What the voice said (src/voice.json): a line whose text changed since has to be said again. */
  text?: string;
  duration: number;
  words: TimedWord[];
  /** No voice yet: its length is estimated from its text, for the stills of a new line (`npm run voice` makes it). */
  draft?: boolean;
}

export interface PlacedLine {
  key: string;
  id: string;
  text: string;
  /** The English subtitles of the line. */
  en: string;
  /** Not voiced yet (`VoiceClip.draft`): silent. */
  draft?: boolean;
  /** In the whole video. */
  from: number;
  durationInFrames: number;
  words: TimedWord[];
}

export interface Placed {
  id: SceneId;
  title: string;
  titleEn: string;
  from: number;
  durationInFrames: number;
  /** The frame, from the scene's start, at which each line starts. */
  cues: Record<string, number>;
  lines: PlacedLine[];
}

/** Words a second of the voice, about: what a line without its voice yet is given. */
const DRAFT_WPS = 2.7;

/** A line's length and words before its voice exists: evenly spread, at the voice's pace. */
export function draftClip(text: string): VoiceClip {
  const words = text.split(/\s+/).filter(Boolean);
  const step = 1 / DRAFT_WPS;
  return {
    duration: words.length * step + 0.3,
    words: words.map((word, i) => ({ word, start: i * step, end: (i + 1) * step - 0.05 })),
  };
}

/**
 * The scenes placed end to end. A line without its voice, or whose voice says an older text, is refused, unless `draft`:
 * then its length is estimated (`draftClip`) and it is marked, silent; the test of the script asks every line its voice
 * before a render.
 */
export function buildTimeline(script: readonly SceneScript[], voice: Record<string, VoiceClip>, { draft = false } = {}): Placed[] {
  let from = 0;
  return script.map((s) => {
    let t = s.lead ?? LEAD;
    const cues: Record<string, number> = {};
    const lines = s.lines.map((l, i) => {
      const key = lineKey(s.id, l.id);
      const said = voice[key];
      const fresh = said && (said.text === undefined || said.text === spoken(l)) ? said : undefined;
      const clip = fresh ?? (draft ? { ...draftClip(spoken(l)), draft: true } : undefined);
      if (!clip) throw new Error(`Pas de voix à jour pour ${key} : lance npm run voice`);
      if (i) t += GAP;
      cues[l.id] = Math.round(t * FPS);
      const line = {
        key,
        id: l.id,
        text: l.text,
        en: l.en,
        from: from + cues[l.id],
        durationInFrames: Math.ceil(clip.duration * FPS),
        words: clip.words,
        ...(clip.draft ? { draft: true } : {}),
      };
      t += clip.duration + (l.hold ?? 0);
      return line;
    });
    const durationInFrames = Math.ceil((t + (s.tail ?? TAIL)) * FPS);
    const placed = { id: s.id, title: s.title, titleEn: s.titleEn, from, durationInFrames, cues, lines };
    from += durationInFrames;
    return placed;
  });
}

export const TIMELINE: readonly Placed[] = buildTimeline(SCRIPT, VOICE as Record<string, VoiceClip>, { draft: true });
export const TOTAL_FRAMES = TIMELINE.reduce((n, s) => n + s.durationInFrames, 0);

export const sceneOf = (id: SceneId) => TIMELINE.find((s) => s.id === id)!;
export const titleOf = (id: SceneId) => sceneOf(id).title;
