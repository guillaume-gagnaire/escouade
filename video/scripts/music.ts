// The music, by ElevenLabs: `npm run music` (ELEVENLABS_API_KEY set). Writes public/music.mp3 and src/music.json.
// Composed again only when the timeline (so the plan) changed.

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { compositionPlan, planKey } from '../src/musicPlan';
import { TIMELINE } from '../src/timeline';
import { compose, mp3Duration } from './eleven';

const MANIFEST = new URL('../src/music.json', import.meta.url);
const FILE = new URL('../public/music.mp3', import.meta.url);
const plan = compositionPlan(TIMELINE);
const key = planKey(plan);
const old = existsSync(MANIFEST) ? (JSON.parse(readFileSync(MANIFEST, 'utf8')) as { key?: string }) : {};

if (old.key === key && existsSync(FILE) && !process.argv.includes('--force')) {
  console.log('La musique est déjà composée pour cette timeline.');
} else {
  console.log(
    `Composition de ${plan.sections.length} sections, ${(plan.sections.reduce((n, s) => n + s.duration_ms, 0) / 1000).toFixed(1)} s…`,
  );
  const audio = await compose(plan);
  writeFileSync(FILE, audio);
  const durationMs = Math.round(mp3Duration(audio) * 1000);
  writeFileSync(
    MANIFEST,
    JSON.stringify({ key, durationMs, sections: plan.sections.map((s) => [s.section_name, s.duration_ms]) }, null, 1) + '\n',
  );
  console.log(`public/music.mp3 : ${(durationMs / 1000).toFixed(1)} s`);
}
