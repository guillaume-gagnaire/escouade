// ElevenLabs: the voice, its transcription and the music. The key comes from ELEVENLABS_API_KEY, never from a file.

const API = 'https://api.elevenlabs.io/v1';

export interface Alignment {
  characters: string[];
  character_start_times_seconds: number[];
  character_end_times_seconds: number[];
}

export interface Word {
  word: string;
  start: number;
  end: number;
}

/** The words of an alignment, each from its first character's start to its last one's end. */
export function wordsOf(a: Alignment): Word[] {
  const words: Word[] = [];
  let current: Word | null = null;
  a.characters.forEach((c, i) => {
    if (/\s/.test(c)) {
      current = null;
      return;
    }
    if (!current) {
      current = { word: '', start: a.character_start_times_seconds[i], end: 0 };
      words.push(current);
    }
    current.word += c;
    current.end = a.character_end_times_seconds[i];
  });
  return words;
}

const NUMBERS = 'zero un deux trois quatre cinq six sept huit neuf dix onze douze treize quatorze quinze seize'.split(' ');
const TENS: Record<string, string> = {
  '20': 'vingt',
  '30': 'trente',
  '40': 'quarante',
  '50': 'cinquante',
  '60': 'soixante',
  '100': 'cent',
};

/** A word as it sounds: a number in figures as in letters, without the plural's silent ending. */
const sound = (w: string) => {
  const n = NUMBERS[Number(w)] ?? TENS[w] ?? w;
  return n.length > 3 ? n.replace(/aux$/, 'al').replace(/[sx]$/, '') : n;
};

const tokens = (s: string) =>
  s
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .replace(/['’]/g, '')
    .split(/[^a-z0-9]+/)
    .filter(Boolean)
    .map(sound);

/** Share of the expected words found, in order, in what was heard (longest common subsequence). */
export function similarity(expected: string, heard: string): number {
  const a = tokens(expected);
  const b = tokens(heard);
  if (!a.length) return 1;
  let prev = new Array<number>(b.length + 1).fill(0);
  for (const x of a) {
    const row = [0];
    for (let j = 0; j < b.length; j++) row.push(x === b[j] ? prev[j] + 1 : Math.max(prev[j + 1], row[j]));
    prev = row;
  }
  return prev[b.length] / a.length;
}

const BITRATES = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320];
const RATES = [44100, 48000, 32000];

/** Seconds of an MPEG-1 Layer III file: its frames, after an ID3v2 tag. */
export function mp3Duration(buf: Uint8Array): number {
  let o = 0;
  if (buf[0] === 0x49 && buf[1] === 0x44 && buf[2] === 0x33) o = 10 + ((buf[6] << 21) | (buf[7] << 14) | (buf[8] << 7) | buf[9]);
  let seconds = 0;
  while (o + 4 <= buf.length) {
    if (buf[o] !== 0xff || (buf[o + 1] & 0xfe) !== 0xfa) {
      o++;
      continue;
    }
    const bitrate = BITRATES[buf[o + 2] >> 4] * 1000;
    const rate = RATES[(buf[o + 2] >> 2) & 3];
    if (!bitrate || !rate) {
      o++;
      continue;
    }
    seconds += 1152 / rate;
    o += Math.floor((144 * bitrate) / rate) + ((buf[o + 2] >> 1) & 1);
  }
  return seconds;
}

function key(): string {
  const k = process.env.ELEVENLABS_API_KEY;
  if (!k) throw new Error('Définis ELEVENLABS_API_KEY (clé de l’API ElevenLabs).');
  return k;
}

async function post(path: string, body: unknown): Promise<Response> {
  const res = await fetch(`${API}${path}`, {
    method: 'POST',
    headers: { 'xi-api-key': key(), 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`ElevenLabs ${path} : ${res.status} ${(await res.text()).slice(0, 400)}`);
  return res;
}

export interface Speech {
  audio: Uint8Array;
  words: Word[];
}

export interface Voice {
  id: string;
  model: string;
  seed: number;
}

/** A line said by `voice`, with the timing of each word; the text around it keeps the tone continuous. */
export async function speak(text: string, voice: Voice, around: { previous?: string; next?: string }): Promise<Speech> {
  const res = await post(`/text-to-speech/${voice.id}/with-timestamps?output_format=mp3_44100_128`, {
    text,
    model_id: voice.model,
    language_code: 'fr',
    seed: voice.seed,
    previous_text: around.previous,
    next_text: around.next,
  });
  const json = (await res.json()) as { audio_base64: string; alignment: Alignment };
  return { audio: new Uint8Array(Buffer.from(json.audio_base64, 'base64')), words: wordsOf(json.alignment) };
}

/** What a speech-to-text model hears in an audio file. */
export async function transcribe(audio: Uint8Array, name: string): Promise<string> {
  const form = new FormData();
  form.append('model_id', 'scribe_v1');
  form.append('language_code', 'fra');
  form.append('file', new Blob([new Uint8Array(audio)], { type: 'audio/mpeg' }), name);
  const res = await fetch(`${API}/speech-to-text`, { method: 'POST', headers: { 'xi-api-key': key() }, body: form });
  if (!res.ok) throw new Error(`ElevenLabs speech-to-text : ${res.status} ${(await res.text()).slice(0, 400)}`);
  return ((await res.json()) as { text: string }).text;
}

export interface Section {
  section_name: string;
  positive_local_styles: string[];
  negative_local_styles: string[];
  duration_ms: number;
  lines: string[];
}

export interface CompositionPlan {
  positive_global_styles: string[];
  negative_global_styles: string[];
  sections: Section[];
}

/** An instrumental track following `plan`, section by section. */
export async function compose(plan: CompositionPlan): Promise<Uint8Array> {
  // Instrumental by its plan: no lines, and vocals among the styles to avoid (force_instrumental only goes with a prompt).
  const res = await post('/music?output_format=mp3_44100_128', { composition_plan: plan, model_id: 'music_v1' });
  return new Uint8Array(await res.arrayBuffer());
}
