import { interpolate, spring } from 'remotion';

const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const;

/** 0 before `from`, 1 after `from + dur`, linear in between. */
export const ramp = (frame: number, from: number, dur: number) => interpolate(frame, [from, from + dur], [0, 1], clamp);

/** A spring from 0 to 1 starting at `from`; it may overshoot a little. */
export const pop = (frame: number, fps: number, from: number, damping = 14) =>
  frame < from ? 0 : spring({ frame: frame - from, fps, config: { damping } });

/** `text` as typed from `from`, `cps` characters per second. */
export const typed = (text: string, frame: number, fps: number, from: number, cps = 45) =>
  text.slice(0, Math.max(0, Math.floor(((frame - from) / fps) * cps)));

/** A value going from `a` to `b` over [from, from + dur]. */
export const count = (frame: number, from: number, dur: number, a: number, b: number) => a + (b - a) * ramp(frame, from, dur);

/** A number the French way: 1,87. */
export const fr = (n: number, digits = 2) => n.toFixed(digits).replace('.', ',');

/** Frames of a scene's entry and of its exit: one scene pushes the other out. */
export const SCENE_IN = 10;
export const SCENE_OUT = 8;
const SLIDE = 160;
const BLUR = 12;
const PUSH = 0.025;

const easeOut = (t: number) => 1 - (1 - t) ** 3;
const easeIn = (t: number) => t ** 3;

/**
 * Where a scene is at `frame` of its `duration`: it slides in from the right, blurred, slides out to the left the
 * same way, and the camera pushes in slowly all along; the first scene fades in in place, the last one stays.
 */
export function sceneMotion(frame: number, duration: number, ends: { first?: boolean; last?: boolean } = {}) {
  const enter = ends.first ? 1 : easeOut(ramp(frame, 0, SCENE_IN));
  // Over to its last frame, which is the furthest out.
  const exit = ends.last ? 0 : easeIn(ramp(frame, duration - SCENE_OUT, SCENE_OUT - 1));
  const blur = BLUR * (1 - enter + exit);
  return {
    x: SLIDE * (1 - enter) - SLIDE * exit,
    blur: blur < 0.05 ? 0 : blur,
    opacity: Math.min(ends.first ? ramp(frame, 0, SCENE_IN) : 0.1 + 0.9 * enter, 1 - 0.8 * exit),
    scale: 1 + PUSH * ramp(frame, 0, duration),
  };
}
