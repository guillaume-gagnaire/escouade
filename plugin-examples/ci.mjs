#!/usr/bin/env node
// PLG-10's examples (ESC-182), each as its author would check it before a tag: for those in TypeScript, `npm install`, the
// typecheck, their tests against the SDK's fake Escouade, and their dist/plugin.mjs built again and compared to the one
// committed (Escouade runs the plugin as the repository holds it); then, for all, `escouade-plugin validate` and the
// conformance bench of the app (ESCOUADE_PLUGIN_BIN). CI runs it after building the app's escouade-plugin and the SDK.
//
//   npx tsc -p sdk/plugin/tsconfig.json
//   ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugin-examples/ci.mjs
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
  if (r.status !== 0) {
    console.error(`✗ ${command} ${args.join(' ')}: exit ${r.status ?? r.signal}`);
    process.exit(1);
  }
}

const lf = (s) => s.replace(/\r\n/g, '\n');
const examples = readdirSync(HERE, { withFileTypes: true })
  .filter((d) => d.isDirectory() && existsSync(join(HERE, d.name, 'escouade-plugin.json')))
  .map((d) => d.name);
const tmp = mkdtempSync(join(tmpdir(), 'escouade-examples-'));
try {
  for (const name of examples) {
    const folder = join(HERE, name);
    if (existsSync(join(folder, 'package.json'))) {
      run(folder, 'npm', 'install', '--no-audit', '--no-fund');
      run(folder, 'npm', 'run', 'typecheck');
      run(folder, 'npm', 'test');
      const built = join(tmp, `${name}.mjs`);
      run(folder, 'npx', 'esbuild', 'src/main.ts', '--bundle', '--platform=node', '--format=esm', '--target=node20', `--outfile=${built}`);
      if (lf(readFileSync(built, 'utf8')) !== lf(readFileSync(join(folder, 'dist', 'plugin.mjs'), 'utf8'))) {
        console.error(`✗ ${name}: dist/plugin.mjs is not the build of src/ (npm run build, then commit dist/)`);
        process.exit(1);
      }
      console.log(`✓ ${name}: dist/plugin.mjs is the build of src/`);
    }
    run(HERE, bin, 'validate', name);
    run(HERE, bin, 'test', name);
  }
  console.log(`\n✓ ${examples.length} examples pass validate and the conformance bench: ${examples.join(', ')}`);
} finally {
  rmSync(tmp, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
}
