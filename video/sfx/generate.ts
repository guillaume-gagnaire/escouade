// `npm run sfx`: writes the sound effects to public/sfx/, before a render.

import { mkdirSync, writeFileSync } from 'node:fs';
import { chime, click, SR, whoosh } from './sounds';
import { encodeWav } from './wav';

const dir = new URL('../public/sfx/', import.meta.url);
mkdirSync(dir, { recursive: true });
for (const [name, samples] of Object.entries({ chime: chime(), click: click(), whoosh: whoosh() })) {
  writeFileSync(new URL(`${name}.wav`, dir), encodeWav(samples, samples, SR));
  console.log(`public/sfx/${name}.wav`);
}
