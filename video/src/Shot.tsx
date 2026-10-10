import type { FC } from 'react';
import { AbsoluteFill } from 'remotion';
import { SceneContext } from './cues';
import { LangContext, type Lang } from './lang';
import { SCENES } from './scenes';
import type { SceneId } from './timeline';
import { CaptionsContext } from './ui/Stage';

/** One scene without its text, framed on the app: the website's images, in the language of the page that shows them. */
export const Shot: FC<{ scene: SceneId; lang?: Lang }> = ({ scene, lang = 'fr' }) => {
  const Scene = SCENES[scene];
  return (
    <AbsoluteFill style={{ overflow: 'hidden' }}>
      <LangContext.Provider value={lang}>
        <CaptionsContext.Provider value={false}>
          <SceneContext.Provider value={scene}>
            <div style={{ position: 'absolute', left: 0, top: -120, width: 1920, height: 1080 }}>
              <Scene />
            </div>
          </SceneContext.Provider>
        </CaptionsContext.Provider>
      </LangContext.Provider>
    </AbsoluteFill>
  );
};
