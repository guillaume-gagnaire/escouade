import type { FC } from 'react';
import { AbsoluteFill } from 'remotion';
import { SceneContext } from './cues';
import { SCENES } from './scenes';
import type { SceneId } from './timeline';
import { CaptionsContext } from './ui/Stage';

/** One scene without its text, framed on the app: the website's images. */
export const Shot: FC<{ scene: SceneId }> = ({ scene }) => {
  const Scene = SCENES[scene];
  return (
    <AbsoluteFill style={{ overflow: 'hidden' }}>
      <CaptionsContext.Provider value={false}>
        <SceneContext.Provider value={scene}>
          <div style={{ position: 'absolute', left: 0, top: -120, width: 1920, height: 1080 }}>
            <Scene />
          </div>
        </SceneContext.Provider>
      </CaptionsContext.Provider>
    </AbsoluteFill>
  );
};
