import type { FC } from 'react';
import type { SceneId } from '../timeline';
import { Agents } from './Agents';
import { Chaos } from './Chaos';
import { Chat } from './Chat';
import { ComposerScene } from './ComposerScene';
import { EditorScene } from './EditorScene';
import { GitScene } from './GitScene';
import { Notify } from './Notify';
import { Intro } from './Intro';
import { Projects } from './Projects';
import { BoardScene } from './BoardScene';
import { Loop } from './Loop';
import { TestScene } from './TestScene';
import { Validate } from './Validate';
import { LaunchScene } from './LaunchScene';
import { Terminals } from './Terminals';
import { StatsScene } from './StatsScene';
import { Remote } from './Remote';
import { More } from './More';
import { Outro } from './Outro';

export const SCENES: Record<SceneId, FC> = {
  intro: Intro,
  chaos: Chaos,
  projects: Projects,
  agents: Agents,
  chat: Chat,
  composer: ComposerScene,
  notify: Notify,
  git: GitScene,
  editor: EditorScene,
  board: BoardScene,
  loop: Loop,
  test: TestScene,
  validate: Validate,
  launch: LaunchScene,
  terminals: Terminals,
  stats: StatsScene,
  remote: Remote,
  more: More,
  outro: Outro,
};
