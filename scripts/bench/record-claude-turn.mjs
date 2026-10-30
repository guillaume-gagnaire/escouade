// Records a long real turn of Claude Code for the stream bench (INT-0, docs/PLUGINS.md §4.8): the frames of
// `claude -p --output-format stream-json --include-partial-messages`, one a line as they came, and when each came (ms since the
// launch), to tests/fixtures/claude/long-turn.jsonl and long-turn.times.json. The paths of the machine are put out of it
// (the user's folder becomes C:\Users\user, the folder of the turn C:\Users\user\repo, and what is left of the user's name `user`).
//
//   node scripts/bench/record-claude-turn.mjs [--model sonnet]
//
// It runs in a new temporary folder holding a copy of a few files of the repository, with reading tools only.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const OUT = path.join(ROOT, 'tests', 'fixtures', 'claude');
const model = process.argv.includes('--model') ? process.argv[process.argv.indexOf('--model') + 1] : 'sonnet';

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'esc-int0-'));
const repo = path.join(work, 'repo');
for (const file of ['src-tauri/src/hub.rs', 'src-tauri/src/conv.rs', 'src-tauri/src/engine/bench/replay.rs']) {
  fs.mkdirSync(path.dirname(path.join(repo, file)), { recursive: true });
  fs.copyFileSync(path.join(ROOT, file), path.join(repo, file));
}

const prompt =
  'Lis les trois fichiers de src-tauri/src (hub.rs, conv.rs, engine/bench/replay.rs), un par un avec Read, puis cherche avec ' +
  'Grep les usages de `ConvOp` dans ce dossier. Écris ensuite une explication détaillée, en français, d’environ 8 000 mots, en plusieurs parties, ' +
  'de la façon dont une conversation est construite puis envoyée à la fenêtre : chaque structure, chaque fonction, et un ' +
  'exemple pas à pas. Ne modifie aucun fichier.';

const env = { ...process.env };
// Started from a session of Claude Code: the CLI must not take itself for its child.
for (const k of Object.keys(env)) if (k.startsWith('CLAUDECODE') || k === 'CLAUDE_CODE_ENTRYPOINT') delete env[k];

const started = Date.now();
const child = spawn(
  'claude',
  [
    '-p',
    prompt,
    '--output-format',
    'stream-json',
    '--include-partial-messages',
    '--verbose',
    '--model',
    model,
    // Nothing of the user's own setup: their settings, hooks, plugins, MCP servers and skills.
    '--setting-sources',
    'project',
    '--strict-mcp-config',
    '--disable-slash-commands',
    '--allowedTools',
    'Read',
    'Glob',
    'Grep',
  ],
  { cwd: repo, env, stdio: ['ignore', 'pipe', 'inherit'] },
);

const frames = [];
const times = [];
let rest = '';
child.stdout.on('data', (chunk) => {
  const at = Date.now() - started;
  rest += chunk.toString('utf8');
  let nl;
  while ((nl = rest.indexOf('\n')) >= 0) {
    const line = rest.slice(0, nl).trim();
    rest = rest.slice(nl + 1);
    if (!line) continue;
    frames.push(line);
    times.push(at);
  }
});

child.on('exit', (code) => {
  if (code !== 0) {
    console.error(`claude stopped with ${code}`);
    process.exit(1);
  }
  const home = os.homedir();
  const esc = (s) => JSON.stringify(s).slice(1, -1);
  const out = frames.map((line) => {
    let l = line;
    // In the JSON of a line, a path's backslashes are doubled; both forms are put out.
    for (const [from, to] of [
      [repo, 'C:\\Users\\user\\repo'],
      [work, 'C:\\Users\\user'],
      [home, 'C:\\Users\\user'],
    ]) {
      l = l
        .split(esc(from))
        .join(esc(to))
        .split(esc(from.replaceAll('\\', '/')))
        .join(esc(to.replaceAll('\\', '/')));
    }
    // What is left of the user's name: in a folder of Claude Code's (C--Users-<name>-…), or split across partial JSON.
    l = l.split(os.userInfo().username).join('user');
    JSON.parse(l);
    return l;
  });
  fs.mkdirSync(OUT, { recursive: true });
  fs.writeFileSync(path.join(OUT, 'long-turn.jsonl'), out.join('\n') + '\n');
  fs.writeFileSync(path.join(OUT, 'long-turn.times.json'), JSON.stringify(times) + '\n');
  fs.rmSync(work, { recursive: true, force: true });
  console.log(`${frames.length} frames over ${times.at(-1)} ms`);
});
