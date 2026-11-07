// A fake container engine for the review's bench (scripts/plugin-review/bench.ts): run as `node fake-engine.mjs <args>`,
// it writes the arguments it got to FAKE_ENGINE_LOG and answers as `docker run … escouade-plugin test /plugin --json`
// would, by FAKE_ENGINE_MODE: `pass`, `refused` (the sandbox refused a connection and a write), `garbage` (no JSON),
// `hang` (never ends: the time limit). It runs nothing of the plugin.
//
// The steps of a third party's `run.wasm` (scripts/plugin-review/container.ts) stand on tests/fake/fake-escouade-plugin.mjs,
// its volumes being folders under FAKE_ENGINE_VOLUMES: it fails when `build` or `read` is given a network, when `build`
// can write to the tools.
import { spawnSync } from 'node:child_process';
import { appendFileSync, mkdirSync, readdirSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
if (process.env.FAKE_ENGINE_LOG) appendFileSync(process.env.FAKE_ENGINE_LOG, `${JSON.stringify(args)}\n`);
if (args[0] === 'kill' || args[0] === 'volume') process.exit(0);

const volumes = process.env.FAKE_ENGINE_VOLUMES ?? join(tmpdir(), 'fake-engine-volumes');
const mounted = (at) => {
  const v = args.find((a, i) => args[i - 1] === '--volume' && (a.includes(`:${at}:`) || a.endsWith(`:${at}`)));
  return v?.slice(0, v.lastIndexOf(`:${at}`));
};
const toolsIn = () => join(volumes, mounted('/tools'));
const cli = (words, env) => {
  const r = spawnSync(process.execPath, [fileURLToPath(new URL('./fake-escouade-plugin.mjs', import.meta.url)), ...words], {
    encoding: 'utf8',
    env: { PATH: process.env.PATH, ESCOUADE_DATA_DIR: toolsIn(), ...env },
  });
  process.stdout.write(r.stdout);
  process.stderr.write(r.stderr);
  process.exit(r.status);
};
if (args.includes('chown')) process.exit(0);
if (args.includes('fetch')) cli(['fetch', mounted('/plugin'), ...args.slice(args.indexOf('fetch') + 2)]);
if (args.includes('--out')) {
  if (!args.includes('--network=none') || !args.some((a) => a.endsWith(':/tools:ro'))) {
    console.error('build was given a network, or the tools to write to');
    process.exit(1);
  }
  const out = join(volumes, mounted('/work'), 'out');
  mkdirSync(out, { recursive: true });
  const words = args.slice(args.indexOf('build'));
  words[words.indexOf('/plugin')] = mounted('/plugin');
  words[words.indexOf('--out') + 1] = out;
  cli(words, { CARGO_NET_OFFLINE: 'true' });
}
if (args.some((a) => a.includes('cat /work/out'))) {
  if (!args.includes('--network=none')) process.exit(1);
  const out = join(volumes, mounted('/work'), 'out');
  process.stdout.write(
    readFileSync(
      join(
        out,
        readdirSync(out).find((n) => n.endsWith('.wasm')),
      ),
    ),
  );
  process.exit(0);
}
const mode = process.env.FAKE_ENGINE_MODE ?? 'pass';
const outcomes = (verdict) => [
  { scenario: 'manifest', verdict: 'pass', said: 'le manifeste tient' },
  {
    scenario: 'initialize',
    verdict,
    said: verdict === 'pass' ? 'répond en 12 ms' : 'ne répond pas',
    ...(verdict === 'fail' ? { missing: 'une réponse à initialize' } : {}),
  },
];
if (mode === 'pass') console.log(JSON.stringify({ plugin: 'x 1.0.0', outcomes: outcomes('pass'), timings: [] }));
else if (mode === 'refused') {
  console.log(
    JSON.stringify({
      plugin: 'x 1.0.0',
      outcomes: outcomes('fail'),
      timings: [],
      seen: [
        { kind: 'stderr', line: 'Error: getaddrinfo EAI_AGAIN collect.q7x-telemetry.net' },
        { kind: 'stderr', line: "Error: EROFS: read-only file system, open '/plugin/x'" },
        { kind: 'log', level: 'info', message: 'bonjour' },
      ],
    }),
  );
  process.exitCode = 1;
} else if (mode === 'ended') {
  // As the real bench answered for the trapped plugin under podman (2026-10-29): the program ended before it was
  // kept, its last lines in the scenario's words, none in `seen`.
  console.log(
    JSON.stringify({
      plugin: 'x 0.0.1',
      outcomes: [
        { scenario: 'manifest', verdict: 'pass', said: 'valide, avec un programme' },
        {
          scenario: 'initialize',
          verdict: 'fail',
          said: "le programme s'est terminé avant de répondre (code 0) — slak: fetch failed EAI_AGAIN | slak: EROFS: read-only file system, open '/plugin/cache.json' EROFS | slak: ENOENT: no such file or directory, open '/tmp/.ssh/id_ed25519' ENOENT",
          missing: 'réponds à `initialize`',
        },
      ],
      timings: [],
    }),
  );
  process.exitCode = 1;
} else if (mode === 'garbage') {
  console.log('Segmentation fault');
  process.exitCode = 139;
} else setInterval(() => {}, 1000);
