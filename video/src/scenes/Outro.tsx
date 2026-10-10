import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues, useTitle } from '../cues';
import { C, MONO } from '../theme';
import { useFmt } from '../lang';
import { Logo } from '../ui/Logo';
import { Caption, Stage } from '../ui/Stage';

/** Logo, name, link, then fade to black. */
export const Outro: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr } = useFmt();
  const caption = useTitle();
  const e = pop(frame, fps, 0, 12);
  const name = pop(frame, fps, 12, 16);
  const link = pop(frame, fps, c.word('bye', 'télécharge-le') - 6, 18);
  return (
    <Stage>
      <div style={{ position: 'absolute', inset: 0, opacity: 1 - ramp(frame, c.length - 50, 46) }}>
        <div
          style={{
            position: 'absolute',
            inset: 0,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 14,
            paddingBottom: 190,
          }}
        >
          <Logo size={210} stack={[e, e, e]} spark={e} spin={frame * 0.5} />
          <div style={{ fontSize: 130, fontWeight: 800, letterSpacing: -5, lineHeight: 1, opacity: Math.min(1, name) }}>Escouade</div>
          <div style={{ fontSize: 42, fontWeight: 500, color: C.muted, opacity: Math.min(1, name) }}>
            {tr('Le poste de pilotage de tes agents Claude Code', 'The cockpit for your Claude Code agents')}
          </div>
        </div>
        <Caption text={caption} delay={c.word('bye', 'gratuit') - 4} top={760} />
        <div
          style={{
            position: 'absolute',
            top: 860,
            left: 0,
            right: 0,
            textAlign: 'center',
            fontFamily: MONO,
            fontSize: 30,
            color: C.brand,
            opacity: Math.min(1, link),
            transform: `translateY(${(1 - link) * 20}px)`,
          }}
        >
          github.com/guillaume-gagnaire/escouade
        </div>
      </div>
    </Stage>
  );
};
