// The website's images, video and subtitles, from the video: `npm run site-images` (after `npm run render` and
// `npm run web-video`). The images and the subtitles are made in each language: the pictures at the same moments of
// the French voice, with the interface in English for the English page. `-- --lang=en` makes one language only.
// Two guards: the tests compare the subtitles files byte for byte to the script's, and src/scenes/frenchText.test.tsx
// keeps the French interface's text in a snapshot. The French images are deterministic: after a change to the
// interface, `npm run site-images -- --lang=fr` and `git status` must show none of them modified.

import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { bundle } from '@remotion/bundler';
import { renderStill, selectComposition } from '@remotion/renderer';
import { cuesOf } from '../src/cues';
import { LANGS } from '../src/lang';
import { imagePath, posterFrame, SITE_SHOTS, subtitlesFile } from '../src/siteShots';
import { webvtt } from '../src/subtitles';
import { sceneOf, TIMELINE } from '../src/timeline';

const SITE = '../website/public';
const only = process.argv.find((a) => a.startsWith('--lang='))?.slice(7);
const langs = LANGS.filter((l) => !only || l === only);
if (!langs.length) throw new Error(`Langue inconnue : ${only}`);

const serveUrl = await bundle({ entryPoint: fileURLToPath(new URL('../src/index.ts', import.meta.url)) });
for (const lang of langs) {
  for (const s of SITE_SHOTS) {
    const inputProps = { scene: s.scene, lang };
    const composition = await selectComposition({ serveUrl, id: 'Shot', inputProps });
    const frame = s.frame(cuesOf(sceneOf(s.scene)));
    const output = `${SITE}/${imagePath(lang, s.name)}`;
    mkdirSync(dirname(output), { recursive: true });
    await renderStill({ composition, serveUrl, output, frame, inputProps, imageFormat: 'jpeg', jpegQuality: 85 });
    console.log(`${imagePath(lang, s.name)} (${s.scene}, image ${frame})`);
  }
  // The poster: the board, with the scene's title.
  const inputProps = { lang };
  const presentation = await selectComposition({ serveUrl, id: 'Presentation', inputProps });
  await renderStill({
    composition: presentation,
    serveUrl,
    output: `${SITE}/${imagePath(lang, 'poster')}`,
    frame: posterFrame(),
    inputProps,
    imageFormat: 'jpeg',
    jpegQuality: 85,
  });
  writeFileSync(`${SITE}/${subtitlesFile(lang)}`, webvtt(TIMELINE, lang));
  console.log(`${imagePath(lang, 'poster')}, ${subtitlesFile(lang)}`);
}
// The video is the French one, for both pages; the logo is shared.
if (existsSync('out/escouade-web.mp4')) {
  copyFileSync('out/escouade-web.mp4', `${SITE}/escouade.mp4`);
  console.log('escouade.mp4');
} else console.log('out/escouade-web.mp4 absent : escouade.mp4 reste celui du site (npm run render, puis npm run web-video)');
copyFileSync('../public/logo.svg', `${SITE}/logo.svg`);
