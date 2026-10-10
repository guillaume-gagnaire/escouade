import type { FC, ReactNode } from 'react';
import { AbsoluteFill, Html5Audio, Sequence, staticFile, useCurrentFrame } from 'remotion';
import { sceneMotion } from './anim';
import { SceneContext } from './cues';
import { LangContext, type Lang } from './lang';
import { musicVolume } from './mix';
import { SCENES } from './scenes';
import { C } from './theme';
import { TIMELINE, TOTAL_FRAMES } from './timeline';

const LINES = TIMELINE.flatMap((s) => s.lines);
/** Frames of the whoosh, centred on the cut between two scenes. */
const WHOOSH = 12;

/** A scene sliding in, pushing in slowly, then sliding out (src/anim.ts). */
const Moving: FC<{ duration: number; first: boolean; last: boolean; children: ReactNode }> = ({ duration, first, last, children }) => {
  const m = sceneMotion(useCurrentFrame(), duration, { first, last });
  return (
    <AbsoluteFill
      style={{
        transform: `translateX(${m.x}px) scale(${m.scale})`,
        filter: m.blur ? `blur(${m.blur}px)` : undefined,
        opacity: m.opacity,
      }}
    >
      {children}
    </AbsoluteFill>
  );
};

/** The video. Its interface is French; `lang` draws it in another language, for the website's English poster (the voice stays French). */
export const Presentation: FC<{ lang?: Lang }> = ({ lang = 'fr' }) => (
  <LangContext.Provider value={lang}>
    <AbsoluteFill style={{ background: C.bg }}>
      {TIMELINE.map((s, i) => {
        const Scene = SCENES[s.id];
        return (
          <Sequence key={s.id} name={s.id} from={s.from} durationInFrames={s.durationInFrames}>
            <SceneContext.Provider value={s.id}>
              <Moving duration={s.durationInFrames} first={i === 0} last={i === TIMELINE.length - 1}>
                <Scene />
              </Moving>
            </SceneContext.Provider>
          </Sequence>
        );
      })}
      {TIMELINE.slice(1).map((s) => (
        <Sequence key={`whoosh ${s.id}`} name={`whoosh ${s.id}`} from={s.from - WHOOSH / 2} durationInFrames={WHOOSH} layout="none">
          <Html5Audio src={staticFile('sfx/whoosh.wav')} volume={0.55} />
        </Sequence>
      ))}
      {LINES.map((l) => (
        <Sequence key={l.key} name={`voix ${l.key}`} from={l.from} durationInFrames={l.durationInFrames} layout="none">
          <Html5Audio src={staticFile(`voice/${l.key}.mp3`)} />
        </Sequence>
      ))}
      <Html5Audio src={staticFile('music.mp3')} volume={(f) => musicVolume(f, LINES, TOTAL_FRAMES)} />
    </AbsoluteFill>
  </LangContext.Provider>
);
