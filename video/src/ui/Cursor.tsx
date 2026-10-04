import type { FC } from 'react';
import { useCurrentFrame } from 'remotion';
import { soft } from '../theme';
import { Sfx } from './Stage';

/** A mouse pointer at (x, y); `click` (0 to 1) draws its ripple. */
export const Cursor: FC<{ x: number; y: number; click?: number; opacity?: number }> = ({ x, y, click = 0, opacity = 1 }) => (
  <div style={{ position: 'absolute', left: x, top: y, width: 0, height: 0, zIndex: 100, opacity }}>
    {click > 0 && click < 1 ? (
      <span
        style={{
          position: 'absolute',
          left: -26 * click,
          top: -26 * click,
          width: 52 * click,
          height: 52 * click,
          borderRadius: '50%',
          border: `3px solid ${soft('#ffffff', (1 - click) * 90)}`,
        }}
      />
    ) : null}
    <svg
      width="26"
      height="32"
      viewBox="0 0 26 32"
      style={{ position: 'absolute', left: -3, top: -2, filter: 'drop-shadow(0 3px 6px rgba(0,0,0,0.5))' }}
    >
      <path d="M3 2 L3 26 L9 20 L13 30 L17 28 L13 18 L22 18 Z" fill="#ffffff" stroke="#1b1512" strokeWidth="1.6" strokeLinejoin="round" />
    </svg>
  </div>
);

/** A key of a pointer's path: at frame `at`, at (x, y); `click` clicks there. */
export type PointerKey = [at: number, x: number, y: number, click?: boolean];

const CLICK = 14;
const ease = (t: number) => (t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2);

/** Where the pointer is at `frame` along its keys, and its click ripple (0 to 1, 0 when none). */
export function pointerAt(frame: number, keys: PointerKey[]): { x: number; y: number; click: number } {
  const i = keys.findIndex((k) => k[0] > frame);
  let x: number;
  let y: number;
  if (i === 0) [, x, y] = keys[0];
  else if (i === -1) [, x, y] = keys[keys.length - 1];
  else {
    const [a, b] = [keys[i - 1], keys[i]];
    const t = ease((frame - a[0]) / (b[0] - a[0]));
    x = a[1] + (b[1] - a[1]) * t;
    y = a[2] + (b[2] - a[2]) * t;
  }
  const last = keys.filter((k) => k[3] && k[0] <= frame).at(-1);
  const click = last && frame - last[0] < CLICK ? (frame - last[0]) / CLICK : 0;
  return { x, y, click };
}

/** The pointer following its keys between `from` and `to` (it fades in and out), with a click sound on each click. */
export const PointerPath: FC<{ keys: PointerKey[]; from?: number; to?: number }> = ({
  keys,
  from = keys[0][0] - 8,
  to = keys[keys.length - 1][0] + 20,
}) => {
  const frame = useCurrentFrame();
  const p = pointerAt(frame, keys);
  const opacity = Math.max(0, Math.min(1, (frame - from) / 8, (to - frame) / 8));
  return (
    <>
      {keys
        .filter((k) => k[3])
        .map((k) => (
          <Sfx key={k[0]} at={k[0]} name="click" />
        ))}
      {opacity > 0 ? <Cursor x={p.x} y={p.y} click={p.click} opacity={opacity} /> : null}
    </>
  );
};
