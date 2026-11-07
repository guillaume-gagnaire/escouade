// Démarrage du prototype (ESC-365), lancé comme l'e2e lance l'app : Date.now() juste avant spawn, puis l'heure de la
// première frame peinte que le prototype écrit (SPIKE_OUT). node startup-spike.mjs [lancements=10]
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const exe = path.join(here, '..', 'target', 'release', 'gpui-spike.exe');
const out = path.join(here, '..', 'results', 'startup-node.jsonl');
const n = Number(process.argv[2] ?? 10);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const times = [];
for (let i = 0; i < n; i++) {
  const file = path.join(here, '..', 'results', `startup-node-${i}.jsonl`);
  fs.rmSync(file, { force: true });
  const at = Date.now();
  const child = spawn(exe, [], { env: { ...process.env, SPIKE_OUT: file }, stdio: 'ignore' });
  let frame = NaN;
  for (let t = 0; t < 200 && Number.isNaN(frame); t++) {
    await sleep(50);
    if (fs.existsSync(file)) {
      const l = fs
        .readFileSync(file, 'utf8')
        .split('\n')
        .find((x) => x.includes('startup'));
      if (l) frame = JSON.parse(l).first_frame_epoch_ms;
    }
  }
  times.push(frame - at);
  console.log(`launch ${i + 1}: window ${frame - at} ms`);
  child.kill();
  await sleep(1500);
}
const s = [...times].sort((a, b) => a - b);
const res = { kind: 'startup-node', launches: n, median: s[Math.floor((n - 1) / 2)], min: s[0], max: s[n - 1], all: times };
fs.appendFileSync(out, JSON.stringify(res) + '\n');
console.log(JSON.stringify(res));
