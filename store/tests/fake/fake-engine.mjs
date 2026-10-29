// A fake container engine for the review's bench (scripts/plugin-review/bench.ts): run as `node fake-engine.mjs <args>`,
// it writes the arguments it got to FAKE_ENGINE_LOG and answers as `docker run … escouade-plugin test /plugin --json`
// would, by FAKE_ENGINE_MODE: `pass`, `refused` (the sandbox refused a connection and a write), `garbage` (no JSON),
// `hang` (never ends: the time limit). It runs nothing of the plugin.
import { appendFileSync } from 'node:fs';

const args = process.argv.slice(2);
if (process.env.FAKE_ENGINE_LOG) appendFileSync(process.env.FAKE_ENGINE_LOG, `${JSON.stringify(args)}\n`);
if (args[0] === 'kill') process.exit(0);
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
