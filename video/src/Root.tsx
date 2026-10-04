import type { FC } from 'react';
import { Composition } from 'remotion';
import { Presentation } from './Presentation';
import { Shot } from './Shot';
import { FPS, sceneOf, TOTAL_FRAMES, type SceneId } from './timeline';

export const Root: FC = () => (
  <>
    <Composition id="Presentation" component={Presentation} durationInFrames={TOTAL_FRAMES} fps={FPS} width={1920} height={1080} />
    <Composition
      id="Shot"
      component={Shot}
      durationInFrames={1800}
      fps={FPS}
      width={1920}
      height={960}
      defaultProps={{ scene: 'projects' as SceneId }}
      calculateMetadata={({ props }) => ({ durationInFrames: sceneOf(props.scene).durationInFrames })}
    />
  </>
);
