#!/usr/bin/env node
// The templates of `npm create escouade-plugin`, as an author gets them (ESC-181, PLG-9): each one written to a
// temporary folder against this repository's @escouade-dev/plugin, installed, then its typecheck, its tests, its build,
// `escouade-plugin validate`, `escouade-plugin test` (the conformance bench of the app, ESCOUADE_PLUGIN_BIN) and `pack`.
// The Rust template has no npm: `cargo test`, the check for wasm32-wasip1, then the same `validate` and bench, against the
// kit of crates.io with its own Cargo.lock, as an outside author has it. Then the examples of the documentation
// (sdk/doc-examples), which the same steps hold to the code the pages show (ESC-342).
// CI runs it after building the app's escouade-plugin; any step that fails fails it.
//
//   ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node sdk/scripts/template-ci.mjs
import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
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

// The version of Escouade that serves plugins in WASM (`wasm-since`): what a template's `escouade.min` asks.
const since = /^wasm-since (\S+)\r?$/m.exec(spawnSync(bin, ['--version'], { encoding: 'utf8' }).stdout ?? '')?.[1];
if (!since) {
  console.error(`${bin} --version does not say "wasm-since": build escouade-plugin from this tree`);
  process.exit(2);
}

// The temporary folders are outside src-tauri, whose rust-toolchain.toml brings wasm32-wasip1: cargo is told the same one.
const toolchain = /^channel = "(.+)"/m.exec(readFileSync(join(ROOT, 'src-tauri', 'rust-toolchain.toml'), 'utf8'))?.[1];

function run(cwd, command, ...args) {
  console.log(`\n$ ${[command, ...args].join(' ')}   (${cwd})`);
  // On Windows, npm and npx are .cmd scripts, which only a shell starts (the words here have no spaces to quote).
  const windows = process.platform === 'win32';
  const r = spawnSync(windows ? [command, ...args].join(' ') : command, windows ? [] : args, {
    cwd,
    stdio: 'inherit',
    shell: windows,
    env: { ...process.env, ESCOUADE_PLUGIN_BIN: bin, RUSTUP_TOOLCHAIN: toolchain },
  });
  if (r.status !== 0) {
    console.error(`✗ ${command} ${args.join(' ')}: exit ${r.status ?? r.signal}`);
    process.exit(1);
  }
}

run(ROOT, 'node', 'sdk/plugin/scripts/generate.mjs', '--check');
run(ROOT, 'npx', 'tsc', '-p', 'sdk/plugin/tsconfig.json');
/** A plugin in Rust, as its author checks it: nothing of npm, the bench on the module `escouade-plugin` compiles. */
function rust(folder) {
  run(folder, 'cargo', 'test', '--locked');
  run(folder, 'cargo', 'check', '--locked', '--target', 'wasm32-wasip1');
  run(folder, bin, 'validate', '.', '--escouade', since);
  run(folder, bin, 'test', '.', '--escouade', since);
}

const base = mkdtempSync(join(tmpdir(), 'escouade-template-'));
try {
  for (const kind of KINDS) {
    const folder = join(base, `my-${kind}`);
    create({ folder, kind, id: `dev.ci.my-${kind}`, sdk: `file:${SDK}` });
    if (kind === 'rust') {
      rust(folder);
      continue;
    }
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
  // The examples of the documentation, in a folder of their own as an author would have them.
  const example = join(base, 'doc-example');
  cpSync(join(ROOT, 'sdk', 'doc-examples', 'rust'), example, { recursive: true, filter: (from) => !/[\\/]target([\\/]|$)/.test(from) });
  rust(example);
  run(ROOT, 'npx', 'tsc', '-p', 'sdk/doc-examples/ts/tsconfig.json');
  const bundled = join(base, 'doc-example-ts');
  cpSync(join(ROOT, 'sdk', 'doc-examples', 'ts'), bundled, { recursive: true });
  run(
    ROOT,
    'npx',
    'esbuild',
    'sdk/doc-examples/ts/src/main.ts',
    '--bundle',
    '--platform=node',
    '--format=esm',
    '--target=node20',
    '--tsconfig=sdk/doc-examples/ts/tsconfig.json',
    `--outfile=${join(bundled, 'dist', 'plugin.mjs')}`,
  );
  run(bundled, bin, 'validate', '.', '--escouade', since);
  run(bundled, bin, 'test', '.', '--escouade', since);
  console.log('\n✓ the templates and the examples of the documentation pass validate and the conformance bench');
} finally {
  rmSync(base, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
}
