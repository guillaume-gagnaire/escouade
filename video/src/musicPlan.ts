// The music's composition plan, from the timeline: one section per part of the script, as long as its scenes.

import { createHash } from 'node:crypto';
import type { CompositionPlan, Section } from '../scripts/eleven';
import { FPS, type Placed, type SceneId } from './timeline';

// A launch, not a lesson: the beat from the first bar, no calm passage, a peak on the board, a big ending.
const PARTS: { name: string; until: SceneId; styles: string[] }[] = [
  {
    name: 'Intro',
    until: 'chaos',
    styles: ['quick riser into the beat', 'punchy kick and claps from the first bar', 'staccato synth stabs'],
  },
  {
    name: 'Drive',
    until: 'composer',
    styles: ['driving four-on-the-floor', 'syncopated plucked synth bass', 'crisp hi-hats', 'high energy'],
  },
  { name: 'Drive 2', until: 'editor', styles: ['driving beat continues', 'new arpeggiated lead', 'filter sweeps', 'high energy'] },
  {
    name: 'Kanban',
    until: 'integrations',
    styles: ['peak energy', 'bigger drums and claps', 'bright lead hook', 'euphoric but controlled'],
  },
  { name: 'Drive 3', until: 'remote', styles: ['driving groove', 'funky synth bass', 'crisp percussion', 'high energy'] },
  { name: 'Final', until: 'outro', styles: ['final lift', 'big wide chords', 'punchy ending hit on the tonic', 'short tail'] },
];

/** The longest section ElevenLabs takes, in ms. */
const MAX_SECTION = 120000;

export function compositionPlan(timeline: readonly Placed[]): CompositionPlan {
  const ms = (frames: number) => Math.round((frames / FPS) * 1000);
  const total = ms(timeline.reduce((n, s) => n + s.durationInFrames, 0));
  let start = 0;
  let used = 0;
  const sections: Section[] = PARTS.flatMap((p, i) => {
    const end = timeline.findIndex((s) => s.id === p.until);
    const frames = timeline.slice(start, end + 1).reduce((n, s) => n + s.durationInFrames, 0);
    start = end + 1;
    // The last one takes what rounding left, so that the whole lasts as long as the video.
    const duration = i === PARTS.length - 1 ? total - used : ms(frames);
    used += duration;
    // ElevenLabs takes sections of 2 min at most: a longer part is cut into equal ones, of the same style.
    const n = Math.ceil(duration / MAX_SECTION);
    return Array.from({ length: n }, (_, k) => ({
      section_name: n > 1 ? `${p.name} ${k + 1}/${n}` : p.name,
      positive_local_styles: p.styles,
      negative_local_styles: [],
      duration_ms: k < n - 1 ? Math.floor(duration / n) : duration - (n - 1) * Math.floor(duration / n),
      lines: [],
    }));
  });
  return {
    positive_global_styles: [
      'energetic modern electronic',
      'punchy drums',
      'bright synths',
      'tech product launch',
      'instrumental',
      'around 124 BPM',
      'driving and confident',
      'mixed to sit under a voice-over',
    ],
    negative_global_styles: ['vocals', 'singing', 'choir', 'slow tempo', 'ambient drone', 'sad', 'harsh distortion', 'dubstep wobble'],
    sections,
  };
}

/** What identifies a plan: the music must be composed again when it changes. */
export const planKey = (plan: CompositionPlan) => createHash('sha256').update(JSON.stringify(plan)).digest('hex').slice(0, 16);
