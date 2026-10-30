// INT-0, the reference measures (docs/PLUGINS.md §4.8), in one command: `npm run bench:int0 -- [--label before] [--rounds 20]
// [--launches 10] [--flux-only]`.
//
// 1. The stream bench (src-tauri/src/fluxbench.rs, a release build): the recorded streams of each harness replayed through the
//    core, at the fastest and at their recorded pace — delay from a line read to its window event, throughput, CPU, memory.
// 2. On Windows, with the app built (`npx tauri build --debug --no-bundle`, or ESCOUADE_E2E_EXE): the startup bench
//    (tests/e2e/startup-bench.spec.ts: time to the window, to the first agent ready) and the keystroke bench of a terminal view
//    (tests/e2e/terminal-latency.spec.ts, idle, in the « Vue Claude Code »).
//
// The first two write their JSON to bench-results/ (Playwright empties test-results/ at each run), the keystroke bench to
// test-results/ (read right after it); this script then writes their summary, the tables of docs/PLUGINS.md, to
// bench-results/int0-<label>.md and prints it. ESCOUADE_E2E_CDP_PORT, when set, is the port the app's WebView2 is driven on.

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const RESULTS = path.join(ROOT, 'bench-results');
/** Where Playwright leaves the keystroke bench's results (emptied at its next run: read right after it). */
const PLAYWRIGHT_RESULTS = path.join(ROOT, 'test-results');

const arg = (name, fallback) => {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : fallback;
};
const label = arg('label', 'run');
const rounds = arg('rounds', '20');
const launches = arg('launches', '10');
const fluxOnly = process.argv.includes('--flux-only');

function run(what, command, args, cwd, env) {
  console.log(`\n== ${what}`);
  const r = spawnSync(command, args, { cwd, env: { ...process.env, ...env }, stdio: 'inherit', shell: process.platform === 'win32' });
  if (r.status !== 0) {
    console.error(`${what}: failed (${r.status ?? r.signal})`);
    process.exit(1);
  }
}

const read = (file) => (fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, 'utf8')) : null);
const fmt = (x, digits = 0) => (Number.isFinite(x) ? x.toFixed(digits) : '—');

run(
  'stream bench (release)',
  'cargo',
  ['test', '--release', '--lib', 'fluxbench::int0_stream_bench', '--', '--ignored', '--exact', '--nocapture'],
  path.join(ROOT, 'src-tauri'),
  { ESCOUADE_BENCH_LABEL: label, ESCOUADE_BENCH_ROUNDS: rounds },
);

const exe = process.env.ESCOUADE_E2E_EXE ?? path.join(ROOT, 'src-tauri', 'target', 'debug', 'escouade.exe');
const app = !fluxOnly && process.platform === 'win32' && fs.existsSync(exe);
if (!fluxOnly && !app) console.log(`\nno app to measure (${exe}; Windows only): the startup and keystroke benches are skipped`);
if (app) {
  run('startup bench', 'npx', ['playwright', 'test', 'tests/e2e/startup-bench.spec.ts'], ROOT, {
    ESCOUADE_BENCH: '1',
    ESCOUADE_BENCH_LABEL: label,
    ESCOUADE_BENCH_LAUNCHES: launches,
  });
  run('keystroke bench of a terminal view', 'npx', ['playwright', 'test', 'tests/e2e/terminal-latency.spec.ts'], ROOT, {
    ESCOUADE_TERM_BENCH: '1',
    ESCOUADE_TERM_BENCH_IDLE_ONLY: '1',
    ESCOUADE_TERM_BENCH_SHELL: 'claude',
    ESCOUADE_TERM_BENCH_LABEL: `int0-${label}`,
  });
}

const keys = app ? read(path.join(PLAYWRIGHT_RESULTS, `terminal-latency-int0-${label}.json`)) : null;
const lines = [
  `# INT-0, ${label} (${new Date().toISOString().slice(0, 10)}, ${os.cpus()[0]?.model.trim()}, ${os.platform()} ${os.release()})`,
  '',
];
const flux = read(path.join(RESULTS, `int0-flux-${label}.json`));
lines.push(
  `## Le flux (${flux.release ? 'release' : 'debug'}, ${flux.rounds} passes au plus vite)`,
  '',
  '| Flux | Rythme | Lignes | Sans événement | Médiane (µs) | 99e centile (µs) | Max (µs) | Lignes/s | Mo/s | CPU (ms) | Mémoire avant / après / pic (Mo) |',
  '|---|---|---|---|---|---|---|---|---|---|---|',
);
for (const m of flux.measures) {
  lines.push(
    `| ${m.harness} | ${m.pace === 'fastest' ? 'au plus vite' : 'réel'} | ${m.lines} | ${m.silent} | ${fmt(m.medianUs)} | ${fmt(m.p99Us)} | ${fmt(m.maxUs)} | ${fmt(m.linesPerS)} | ${fmt(m.mbPerS, 1)} | ${fmt(m.cpuMs)} | ${fmt(m.memoryBefore / 1e6)} / ${fmt(m.memoryAfter / 1e6)} / ${fmt(m.memoryPeak / 1e6)} |`,
  );
}
const startup = read(path.join(RESULTS, `int0-startup-${label}.json`));
if (app && startup) {
  const s = (x) => `${fmt(x.median)} (${fmt(x.min)} – ${fmt(x.max)})`;
  lines.push(
    '',
    `## Le démarrage (${startup.exe}, ${startup.launches.length} lancements ; médiane, min – max, en ms)`,
    '',
    `- Jusqu'à la fenêtre (première peinture) : ${s(startup.window)} ; premier lancement, dossier neuf : ${fmt(startup.firstLaunch.window)}`,
    `- Premier agent prêt (le process du harness lancé → sa réponse à initialize, mesuré par le cœur) : ${s(startup.processReady)}`,
    `- Du message envoyé à la réponse affichée : ${s(startup.agentReady)}`,
    `- Du lancement à cette réponse (attache de Playwright comprise) : ${s(startup.launchToAnswer)}`,
  );
}
if (app && keys) {
  const idle = keys.phases.find((p) => p.name === 'idle');
  const k = idle?.keyStats['TOTAL key -> painted'] ?? { n: 0 };
  lines.push(
    '',
    `## Une frappe dans une vue terminal (« Vue Claude Code », au repos)`,
    '',
    `- De la touche à l'écho peint : médiane ${fmt(k.median, 1)} ms, 95e centile ${fmt(k.p95, 1)} ms, max ${fmt(k.max, 1)} ms (${k.n} frappes)`,
  );
}
const md = lines.join('\n') + '\n';
fs.mkdirSync(RESULTS, { recursive: true });
fs.writeFileSync(path.join(RESULTS, `int0-${label}.md`), md);
console.log(`\n${md}\n${path.join(RESULTS, `int0-${label}.md`)}`);
