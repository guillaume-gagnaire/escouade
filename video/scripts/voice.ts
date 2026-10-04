// The voice over, line by line: `npm run voice` (ELEVENLABS_API_KEY set), `-- --check` to transcribe it back.
// Writes public/voice/<scene>.<line>.mp3 and src/voice.json; a line whose text has not changed is kept.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { lineKey, SCRIPT, spoken } from '../src/script';
import { mp3Duration, similarity, speak, transcribe, type Voice, type Word } from './eleven';

/** Corentin: young, French, bright and enthusiastic: a launch, not a lesson. */
const VOICE: Voice = { id: 'IHngRooVccHyPqB4uQkG', model: 'eleven_v4', seed: 7 };
/** Lines taken again with another seed, when the first take said a name wrong (heard back by --check). */
const RETAKES: Record<string, number> = { 'intro.hello': 200 };
const seedOf = (key: string) => RETAKES[key] ?? VOICE.seed;
/** Under this share of its words heard back, a line is reported. */
const CLOSE_ENOUGH = 0.85;

export interface Clip {
  text: string;
  voice: string;
  model: string;
  seed?: number;
  file: string;
  duration: number;
  words: Word[];
  heard?: string;
  match?: number;
}

const MANIFEST = new URL('../src/voice.json', import.meta.url);
const PUBLIC = new URL('../public/', import.meta.url);
const old: Record<string, Clip> = existsSync(MANIFEST) ? JSON.parse(readFileSync(MANIFEST, 'utf8')) : {};
const check = process.argv.includes('--check');

const jobs = SCRIPT.flatMap((s) =>
  s.lines.map((l, i) => ({
    key: lineKey(s.id, l.id),
    text: spoken(l),
    previous: i > 0 ? spoken(s.lines[i - 1]) : undefined,
    next: i < s.lines.length - 1 ? spoken(s.lines[i + 1]) : undefined,
  })),
);

mkdirSync(new URL('voice/', PUBLIC), { recursive: true });
// What is already done, kept: a request that fails later loses nothing that was paid for.
const manifest: Record<string, Clip> = Object.fromEntries(jobs.filter((j) => old[j.key]).map((j) => [j.key, old[j.key]]));
const save = () => {
  const ordered = Object.fromEntries(jobs.filter((j) => manifest[j.key]).map((j) => [j.key, manifest[j.key]]));
  writeFileSync(MANIFEST, JSON.stringify(ordered, null, 1) + '\n');
};
/** An mp3's frames can end before the last word does (encoder delay): the clip lasts at least until then. */
const lasting = (seconds: number, words: Word[]) => Math.max(seconds, (words.at(-1)?.end ?? 0) + 0.05);
const fresh = (c: Clip | undefined, key: string, text: string) =>
  c &&
  c.text === text &&
  c.voice === VOICE.id &&
  c.model === VOICE.model &&
  (c.seed ?? VOICE.seed) === seedOf(key) &&
  existsSync(new URL(c.file, PUBLIC));

async function run(job: (typeof jobs)[number]) {
  let clip = old[job.key];
  if (!fresh(clip, job.key, job.text)) {
    const speech = await speak(job.text, { ...VOICE, seed: seedOf(job.key) }, job);
    const file = `voice/${job.key}.mp3`;
    writeFileSync(new URL(file, PUBLIC), speech.audio);
    clip = {
      text: job.text,
      voice: VOICE.id,
      model: VOICE.model,
      seed: seedOf(job.key),
      file,
      duration: lasting(mp3Duration(speech.audio), speech.words),
      words: speech.words,
    };
    console.log(`voix  ${job.key} (${clip.duration.toFixed(1)} s)`);
  }
  if (check && clip.heard === undefined) {
    clip.heard = await transcribe(new Uint8Array(readFileSync(new URL(clip.file, PUBLIC))), clip.file);
    clip.match = Math.round(similarity(job.text, clip.heard) * 100) / 100;
  }
  clip.duration = lasting(clip.duration, clip.words);
  manifest[job.key] = clip;
  save();
}

// A few requests at a time.
const queue = [...jobs];
await Promise.all(
  Array.from({ length: 4 }, async () => {
    for (let job = queue.shift(); job; job = queue.shift()) await run(job);
  }),
);

save();
const total = jobs.reduce((n, j) => n + manifest[j.key].duration, 0);
console.log(`${jobs.length} répliques, ${total.toFixed(1)} s de voix`);
if (check) {
  // « Claude » heard as « Cloud »: said the English way, in a French sentence.
  const misheard = (j: (typeof jobs)[number]) => /claude/i.test(j.text) && /\bcloud\b/i.test(manifest[j.key].heard ?? '');
  const off = jobs.filter((j) => (manifest[j.key].match ?? 0) < CLOSE_ENOUGH || misheard(j));
  for (const j of off)
    console.log(`À réécouter : ${j.key} (${manifest[j.key].match})\n  dit : ${j.text}\n  entendu : ${manifest[j.key].heard}`);
  console.log(off.length ? `${off.length} réplique(s) à réécouter` : 'Toutes les répliques sont entendues comme écrites.');
}
