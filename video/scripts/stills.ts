// Stills of scenes, to check them: `npm run stills -- <scene ids> [--every=<seconds>]`.
// By default, one at each line's start, middle and end; one bundle for them all.

import { mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { bundle } from '@remotion/bundler';
import { renderStill, selectComposition } from '@remotion/renderer';
import { FPS, TIMELINE } from '../src/timeline';

const args = process.argv.slice(2);
// At least a frame apart: a step of 0 would never end.
const every = Math.max(0, Number(args.find((a) => a.startsWith('--every='))?.slice(8) ?? 0));
const only = args.filter((a) => !a.startsWith('--'));
const scenes = TIMELINE.filter((s) => !only.length || only.includes(s.id));

const serveUrl = await bundle({ entryPoint: fileURLToPath(new URL('../src/index.ts', import.meta.url)) });
const composition = await selectComposition({ serveUrl, id: 'Presentation' });
mkdirSync('out/stills', { recursive: true });
for (const s of scenes) {
  const frames = new Set<number>();
  if (every) for (let f = 0; f < s.durationInFrames; f += Math.max(1, Math.round(every * FPS))) frames.add(f);
  else
    for (const l of s.lines) {
      const at = l.from - s.from;
      [at + 10, at + Math.round(l.durationInFrames / 2), at + l.durationInFrames - 2].forEach((f) => frames.add(f));
    }
  frames.add(s.durationInFrames - 2);
  for (const f of [...frames].sort((a, b) => a - b)) {
    const output = `out/stills/${s.id}-${String(f).padStart(4, '0')}.jpg`;
    await renderStill({ composition, serveUrl, output, frame: s.from + f, imageFormat: 'jpeg', jpegQuality: 80 });
    console.log(output);
  }
}
