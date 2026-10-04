// The website's images, video and subtitles, from the video: `npm run site-images` (after `npm run render` and
// `npm run web-video`).

import { copyFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { bundle } from '@remotion/bundler';
import { renderStill, selectComposition } from '@remotion/renderer';
import { cuesOf } from '../src/cues';
import { webvtt } from '../src/subtitles';
import { sceneOf, TIMELINE, type SceneId } from '../src/timeline';

const SITE = '../website/public';
/** Each image: a scene, at a moment of its voice (no spotlight, no pointer on the way). */
const SHOTS: { name: string; scene: SceneId; frame: (c: ReturnType<typeof cuesOf>) => number }[] = [
  { name: 'agents', scene: 'agents', frame: (c) => c.at('card') - 12 },
  { name: 'chat', scene: 'chat', frame: (c) => c.word('diff', 'déplie-les') + 50 },
  { name: 'notifications', scene: 'notify', frame: (c) => c.end('jump') - 4 },
  { name: 'git', scene: 'git', frame: (c) => c.word('split', 'pendant') + 40 },
  { name: 'editor', scene: 'editor', frame: (c) => c.word('look', 'recherche') + 12 },
  { name: 'board', scene: 'loop', frame: (c) => c.word('loop', 'repart') + 30 },
  { name: 'test', scene: 'test', frame: (c) => c.at('open') + 2 },
  { name: 'launch', scene: 'launch', frame: (c) => c.word('crash', 'plante') + 24 },
  { name: 'stats', scene: 'stats', frame: (c) => c.word('page', 'entrée') - 2 },
  { name: 'remote', scene: 'remote', frame: (c) => c.length - 6 },
];

const serveUrl = await bundle({ entryPoint: fileURLToPath(new URL('../src/index.ts', import.meta.url)) });
mkdirSync(`${SITE}/images`, { recursive: true });
for (const s of SHOTS) {
  const composition = await selectComposition({ serveUrl, id: 'Shot', inputProps: { scene: s.scene } });
  const frame = s.frame(cuesOf(sceneOf(s.scene)));
  await renderStill({
    composition,
    serveUrl,
    output: `${SITE}/images/${s.name}.jpg`,
    frame,
    inputProps: { scene: s.scene },
    imageFormat: 'jpeg',
    jpegQuality: 85,
  });
  console.log(`images/${s.name}.jpg (${s.scene}, image ${frame})`);
}
// The poster: the board, with the scene's title.
const presentation = await selectComposition({ serveUrl, id: 'Presentation' });
const board = sceneOf('board');
await renderStill({
  composition: presentation,
  serveUrl,
  output: `${SITE}/images/poster.jpg`,
  frame: board.from + board.durationInFrames - 8,
  imageFormat: 'jpeg',
  jpegQuality: 85,
});
writeFileSync(`${SITE}/escouade.vtt`, webvtt(TIMELINE));
copyFileSync('out/escouade-web.mp4', `${SITE}/escouade.mp4`);
copyFileSync('../public/logo.svg', `${SITE}/logo.svg`);
console.log('images/poster.jpg, escouade.vtt, escouade.mp4');
