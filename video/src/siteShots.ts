// The website's pictures, from the video (`npm run site-images`): which moment of which scene, and where each language's file goes.
// The moments follow the French voice, so the English pictures are the same moments with the interface in English.

import type { Cues } from './cues';
import type { Lang } from './lang';
import { sceneOf, type SceneId } from './timeline';

export interface SiteShot {
  name: string;
  scene: SceneId;
  /** The frame of the scene, from its cues (no spotlight, no pointer on the way). */
  frame: (c: Cues) => number;
}

export const SITE_SHOTS: SiteShot[] = [
  { name: 'agents', scene: 'agents', frame: (c) => c.at('card') - 12 },
  { name: 'chat', scene: 'chat', frame: (c) => c.word('diff', 'déplie-les') + 50 },
  { name: 'notifications', scene: 'notify', frame: (c) => c.end('jump') - 4 },
  { name: 'git', scene: 'git', frame: (c) => c.word('split', 'pendant') + 40 },
  { name: 'editor', scene: 'editor', frame: (c) => c.word('look', 'recherche') + 12 },
  { name: 'board', scene: 'loop', frame: (c) => c.word('loop', 'repart') + 30 },
  { name: 'test', scene: 'test', frame: (c) => c.at('open') + 2 },
  { name: 'integrations', scene: 'integrations', frame: (c) => c.word('import', 'critères') + 8 },
  { name: 'launch', scene: 'launch', frame: (c) => c.word('crash', 'plante') + 24 },
  { name: 'stats', scene: 'stats', frame: (c) => c.word('page', 'entrée') - 2 },
  { name: 'remote', scene: 'remote', frame: (c) => c.length - 6 },
];

/** The poster: the Kanban scene's last moments, with its title. */
export const POSTER_SCENE: SceneId = 'board';
export const posterFrame = () => {
  const board = sceneOf(POSTER_SCENE);
  return board.from + board.durationInFrames - 8;
};

/** A picture of the website, relative to `website/public`: French at the root of `images/`, the others in their folder. */
export const imagePath = (lang: Lang, name: string) => (lang === 'fr' ? `images/${name}.jpg` : `images/${lang}/${name}.jpg`);

/** The subtitles file, beside the video. */
export const subtitlesFile = (lang: Lang) => (lang === 'fr' ? 'escouade.vtt' : `escouade.${lang}.vtt`);
