import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { C, MONO } from '../theme';
import { Sfx } from './Stage';

/** Keys, big, in the middle of the screen; `press` (0 to 1) pushes them down. */
export const Keys: FC<{ keys: string[]; enter: number; press: number }> = ({ keys, enter, press }) => (
  <div
    style={{
      position: 'absolute',
      left: 0,
      right: 0,
      top: 470,
      zIndex: 60,
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      gap: 18,
      opacity: Math.min(1, enter),
      transform: `scale(${0.8 + 0.2 * Math.min(1, enter)})`,
    }}
  >
    {keys.map((k, i) => (
      <div key={k} style={{ display: 'flex', alignItems: 'center', gap: 18 }}>
        {i ? <span style={{ fontSize: 44, color: C.muted }}>+</span> : null}
        <span
          style={{
            minWidth: 110,
            height: 110,
            padding: '0 26px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 18,
            background: C.elev,
            border: `2px solid ${C.line2}`,
            borderBottomWidth: 8 - 5 * press,
            transform: `translateY(${press * 5}px)`,
            fontFamily: MONO,
            fontSize: 44,
            fontWeight: 600,
            boxShadow: '0 20px 50px rgba(0, 0, 0, 0.5)',
          }}
        >
          {k}
        </span>
      </div>
    ))}
  </div>
);

/** Keys shown before `at`, pressed at `at`, gone a second later. */
export const KeyPress: FC<{ keys: string[]; at: number }> = ({ keys, at }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = pop(frame, fps, at - 22, 16) - ramp(frame, at + 22, 12);
  if (enter <= 0) return null;
  return (
    <>
      <Sfx at={at} name="click" />
      <Keys keys={keys} enter={enter} press={ramp(frame, at - 3, 3) - ramp(frame, at + 5, 5)} />
    </>
  );
};
