#!/usr/bin/env node
// The templates of `npm create escouade-plugin`, as an author gets them (ESC-181, PLG-9): each one written to a
// temporary folder against this repository's @escouade-dev/plugin, installed, then its typecheck, its tests, its build,
// `escouade-plugin validate`, `escouade-plugin test` (the conformance bench of the app, ESCOUADE_PLUGIN_BIN) and `pack`.
// CI runs it after building the app's escouade-plugin; any step that fails fails it.
//
//   ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node sdk/scripts/template-ci.mjs
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { create, KINDS } from '../create-escouade-plugin/index.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const SDK = join(ROOT, 'sdk', 'plugin');
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
  if (r.status !== 0) {
    console.error(`✗ ${command} ${args.join(' ')}: exit ${r.status ?? r.signal}`);
    process.exit(1);
  }
}

run(ROOT, 'node', 'sdk/plugin/scripts/generate.mjs', '--check');
run(ROOT, 'npx', 'tsc', '-p', 'sdk/plugin/tsconfig.json');
const base = mkdtempSync(join(tmpdir(), 'escouade-template-'));
try {
  for (const kind of KINDS) {
    const folder = join(base, `my-${kind}`);
    create({ folder, kind, id: `dev.ci.my-${kind}`, sdk: `file:${SDK}` });
    run(folder, 'npm', 'install', '--no-audit', '--no-fund');
    if (kind === 'program') {
      run(folder, 'npm', 'run', 'typecheck');
      run(folder, 'npm', 'test');
      run(folder, 'npm', 'run', 'build');
    }
    run(folder, 'npx', 'escouade-plugin', 'validate', '.');
    run(folder, 'npx', 'escouade-plugin', 'test', '.');
    run(folder, 'npx', 'escouade-plugin', 'pack', '.');
  }
  console.log('\n✓ both templates pass validate and the conformance bench');
} finally {
  rmSync(base, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
}
