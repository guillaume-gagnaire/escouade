import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop } from '../anim';
import { useCues, useTitle } from '../cues';
import { C } from '../theme';
import { useFmt } from '../lang';
import { Logo } from '../ui/Logo';
import { Caption, Stage } from '../ui/Stage';

/** The logo builds up, its tile, its chevron, its spark, then the name, while the voice says it. */
export const Intro: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr } = useFmt();
  const caption = useTitle();
  const title = pop(frame, fps, c.word('hello', 'escouade') - 4, 16);
  const sub = pop(frame, fps, c.word('hello', 'environnement'), 16);
  return (
    <Stage>
      <div
        style={{
          position: 'absolute',
          inset: 0,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 16,
          paddingBottom: 150,
        }}
      >
        <Logo size={250} tile={pop(frame, fps, 0)} chevron={pop(frame, fps, 14)} spark={pop(frame, fps, 30, 10)} spin={frame * 0.4} />
        <div
          style={{
            fontSize: 150,
            fontWeight: 800,
            letterSpacing: -6,
            lineHeight: 1,
            opacity: Math.min(1, title),
            transform: `translateY(${(1 - title) * 40}px)`,
          }}
        >
          Escouade
        </div>
        <div
          style={{ fontSize: 46, fontWeight: 500, color: C.muted, opacity: Math.min(1, sub), transform: `translateY(${(1 - sub) * 30}px)` }}
        >
          {tr('L’environnement de développement pensé pour tes agents', 'The development environment built for your agents')}
        </div>
      </div>
      <Caption text={caption} delay={c.word('hello', 'agents')} top={860} />
    </Stage>
  );
};
