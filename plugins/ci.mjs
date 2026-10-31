#!/usr/bin/env node
// The plugins of Escouade's own hand in plugins/<id>/ (PLG-10's examples, ESC-182; their ids dev.escouade.*, ESC-314),
// each as its author checks it before a tag: for those in TypeScript, `npm install`, the typecheck, their tests against
// the SDK's fake Escouade, and their dist/plugin.mjs built again and compared to the one committed (Escouade runs the
// plugin as the repository holds it); for the declarative ones, their `node --test`; then, for all, `escouade-plugin
// validate` and the conformance bench of the app (ESCOUADE_PLUGIN_BIN), `--first-party` for their reserved ids. CI
// runs it after building the app's escouade-plugin and the SDK.
//
//   npx tsc -p sdk/plugin/tsconfig.json
//   ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const bin = process.env.ESCOUADE_PLUGIN_BIN && resolve(process.env.ESCOUADE_PLUGIN_BIN);
if (!bin || !existsSync(bin)) {
  console.error(`ESCOUADE_PLUGIN_BIN must name the app's escouade-plugin (cargo build --bin escouade-plugin): ${bin ?? 'unset'}`);
  process.exit(2);
}

function run(cwd, command, ...args) {
  console.log(`\n$ ${[command, ...args].join(' ')}   (${cwd})`);
  // On Windows, npm and npx are .cmd scripts, which only a shell starts (the words here have no spaces to quote).
  const windows = process.platform === 'win32';
  const r = spawnSync(windows ? [command, ...args].join(' ') : command, windows ? [] : args, {
    cwd,
    stdio: 'inherit',
    shell: windows,
    env: { ...process.env, ESCOUADE_PLUGIN_BIN: bin },
  });
  if (r.status !== 0) console.error(`✗ ${command} ${args.join(' ')}: exit ${r.status ?? r.signal}`);
  return r.status === 0;
}

/** Each step of an example, until one fails: whether all passed. */
function check(name) {
  const folder = join(HERE, name);
  if (existsSync(join(folder, 'package.json'))) {
    if (!run(folder, 'npm', 'install', '--no-audit', '--no-fund')) return false;
    if (!run(folder, 'npm', 'run', 'typecheck') || !run(folder, 'npm', 'test')) return false;
    const built = join(tmp, `${name}.mjs`);
    if (
      !run(folder, 'npx', 'esbuild', 'src/main.ts', '--bundle', '--platform=node', '--format=esm', '--target=node20', `--outfile=${built}`)
    ) {
      return false;
    }
    if (lf(readFileSync(built, 'utf8')) !== lf(readFileSync(join(folder, 'dist', 'plugin.mjs'), 'utf8'))) {
      console.error(`✗ ${name}: dist/plugin.mjs is not the build of src/ (npm run build, then commit dist/)`);
      return false;
    }
    console.log(`✓ ${name}: dist/plugin.mjs is the build of src/`);
  } else if (existsSync(join(folder, 'test')) && !run(folder, 'node', '--test')) {
    return false;
  }
  // By its relative folder, as an author runs it beside the plugin; a reserved id is Escouade's own hand.
  const id = JSON.parse(readFileSync(join(folder, 'escouade-plugin.json'), 'utf8')).id;
  const hand = String(id).startsWith('dev.escouade.') ? ['--first-party'] : [];
  return run(HERE, bin, 'validate', name, ...hand) && run(HERE, bin, 'test', name, ...hand);
}

const lf = (s) => s.replace(/\r\n/g, '\n');
const examples = readdirSync(HERE, { withFileTypes: true })
  .filter((d) => d.isDirectory() && existsSync(join(HERE, d.name, 'escouade-plugin.json')))
  .map((d) => d.name);
const tmp = mkdtempSync(join(tmpdir(), 'escouade-examples-'));
try {
  // Every example, even after one that fails: one failure must not hide another.
  const failed = examples.filter((name) => !check(name));
  if (failed.length > 0) {
    console.error(`\n✗ ${failed.length} of ${examples.length} plugins fail: ${failed.join(', ')}`);
    process.exitCode = 1;
  } else {
    console.log(`\n✓ ${examples.length} plugins pass validate and the conformance bench: ${examples.join(', ')}`);
  }
} finally {
  rmSync(tmp, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
}
