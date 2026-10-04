// The website's video: `npm run web-video` (after `npm run render`). Its sound brought to -16 LUFS (two passes of
// loudnorm), a lighter H.264 and its index first, to start playing at once. The true peak is aimed at -2.5 dBTP: the AAC
// encoding adds about 1 dB on top, which keeps the file under -1 dBTP. The CRF is 27: the picture always moves (slides,
// push-in), which costs bits, and the interface text stays sharp at 1080p.

import { spawnSync } from 'node:child_process';

const IN = 'out/presentation.mp4';
const OUT = 'out/escouade-web.mp4';
const TARGET = 'I=-16:TP=-2.5:LRA=11';

/** Remotion's own ffmpeg; returns what it wrote on stderr (where loudnorm prints its measures). */
function ffmpeg(args: string[]): string {
  const r = spawnSync('npx', ['remotion', 'ffmpeg', '-hide_banner', '-nostats', '-y', ...args], { shell: true, encoding: 'utf8' });
  if (r.status !== 0) throw new Error(`ffmpeg a échoué :\n${r.stderr.slice(-2000)}`);
  return r.stderr;
}

const first = ffmpeg(['-i', IN, '-vn', '-af', `loudnorm=${TARGET}:print_format=json`, '-f', 'null', '-']);
const m = JSON.parse(first.slice(first.lastIndexOf('{'), first.lastIndexOf('}') + 1)) as Record<string, string>;
console.log(`Mesuré : ${m.input_i} LUFS, crête ${m.input_tp} dBTP`);

const loudnorm = `loudnorm=${TARGET}:measured_I=${m.input_i}:measured_TP=${m.input_tp}:measured_LRA=${m.input_lra}:measured_thresh=${m.input_thresh}:offset=${m.target_offset}`;
// prettier-ignore
ffmpeg([
  '-i', IN,
  '-c:v', 'libx264', '-preset', 'slow', '-crf', '27', '-pix_fmt', 'yuv420p',
  '-af', loudnorm, '-ar', '48000', '-c:a', 'aac', '-b:a', '160k',
  '-movflags', '+faststart',
  OUT,
]);
console.log(OUT);
