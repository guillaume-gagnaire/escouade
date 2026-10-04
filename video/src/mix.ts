// The music's volume, frame by frame: it fades in, goes under the voice and fades out with the picture.

import { FPS } from './timeline';

/** Volume of the music alone, and under the voice (about 11 dB lower): present, the voice still on top. */
export const FREE = 0.5;
export const UNDER = 0.14;
/** Frames: the music goes down before a line, comes back after it. */
const DOWN = 6;
const HOLD = 8;
const UP = 12;
const FADE_IN = FPS;
const FADE_OUT = 3 * FPS;

const clamp01 = (x: number) => Math.max(0, Math.min(1, x));

/** 1 while a line is said (and just around it), 0 far from it. */
function voiceAt(frame: number, l: { from: number; durationInFrames: number }) {
  const end = l.from + l.durationInFrames + HOLD;
  if (frame < l.from) return clamp01(1 - (l.from - frame) / DOWN);
  if (frame <= end) return 1;
  return clamp01(1 - (frame - end) / UP);
}

export function musicVolume(frame: number, lines: readonly { from: number; durationInFrames: number }[], total: number): number {
  const voice = lines.reduce((v, l) => Math.max(v, voiceAt(frame, l)), 0);
  const level = FREE - (FREE - UNDER) * voice;
  return level * clamp01(frame / FADE_IN) * clamp01((total - frame) / FADE_OUT);
}
