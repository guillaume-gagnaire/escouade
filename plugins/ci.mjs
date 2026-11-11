#!/usr/bin/env node
// The plugins of Escouade's own hand in plugins/<id>/ (PLG-10's examples, ESC-182; their ids dev.escouade.*, ESC-314),
// each as its author checks it before a tag (docs/PLUGINS-DESIGN.md §8.6.4). For all, their LICENSE and CHANGELOG. Then by sort:
// - with a package.json: `npm install` (their package-lock.json is not committed: plugins/<id>/.gitignore), the
//   typecheck, their tests against the SDK's fake Escouade, and their dist/plugin.mjs built again and compared to the
//   one committed (Escouade runs the plugin as the repository holds it);
// - with a Cargo.toml: their manifest's version against the crate's, and `cargo test`, all the crates of a workspace at
//   once (that of src-tauri, or this folder's for those in `run.wasm`);
// - declarative: their `node --test`.
// Then `escouade-plugin validate` and the conformance bench of the app (ESCOUADE_PLUGIN_BIN), `--first-party` for their
// reserved ids: for a `run.js` plugin (slack, team-rules) the bench plays dist/plugin.mjs compiled to WASM in the app's
// own host, with no Node, and for a `run.wasm` one the module `escouade-plugin build` makes of its crate. CI runs this
// after building the app's escouade-plugin and the SDK.
//
//   npx tsc -p sdk/plugin/tsconfig.json
//   ESCOUADE_PLUGIN_BIN=src-tauri/target/debug/escouade-plugin node plugins/ci.mjs
//
// `--changed [<git revision>]` runs only the plugins a change touches, compared to that revision (by default to where
// the branch left main, working tree included): the CI of a branch. docs/PLUGINS-DESIGN.md §8.6.4 says what runs them all.
//
// The plugins are checked side by side, as many at once as the machine has cores (ESCOUADE_CI_JOBS for another number):
// each compilation of a module has a folder of its own, and the download of the toolchain they share is locked across
// processes (escouade-plugin's rust.rs). One after the other, their 23 modules took 6 to 11 min of the CI's Linux. The
// output of each plugin is printed whole once it is checked.
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { availableParallelism, tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { listPlugins, parseArgs, problemsOf, touchedBy } from './ci-lib.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);
// The workspace of the plugins that are still crates of the app, and the folder of the toolchain it pins: cargo run from
// the folder of one of them would take the machine's default one. Those in `run.wasm` are the workspace of this folder.
const WORKSPACE = join(ROOT, 'src-tauri');

let options;
try {
  options = parseArgs(process.argv.slice(2));
} catch (e) {
  console.error(e.message);
  process.exit(2);
}

/** The output of a git command in the repository, or null when it fails. */
function git(...args) {
  const r = spawnSync('git', args, { cwd: ROOT, encoding: 'utf8' });
  return r.status === 0 ? r.stdout : null;
}

/** The files changed since `base` (or where the branch left main), in or out of a commit; null when git cannot say. */
function changedFiles(base) {
  const from = base ?? ['origin/main', 'main'].find((ref) => git('rev-parse', '--verify', '--quiet', ref) !== null);
  const fork = from && git('merge-base', from, 'HEAD')?.trim();
  if (!fork) return null;
  // Each side of a rename: the folder a file left is touched too.
  const changed = git('diff', '--name-only', '--no-renames', '-z', fork);
  const added = git('ls-files', '--others', '--exclude-standard', '-z');
  if (changed === null || added === null) return null;
  return `${changed}${added}`.split('\0').filter(Boolean);
}

const all = listPlugins(HERE);
let plugins = all;
if (options.changed) {
  const files = changedFiles(options.base);
  if (files === null) {
    // A clone too shallow to compare must not make a branch pass by testing nothing.
    console.warn(`git cannot compare with ${options.base ?? 'main'}: all the plugins run`);
  } else {
    plugins = touchedBy(files, all);
    console.log(`${files.length} files changed: ${plugins.length} of ${all.length} plugins to check`);
    if (plugins.length === 0) process.exit(0);
  }
}

const bin = process.env.ESCOUADE_PLUGIN_BIN && resolve(process.env.ESCOUADE_PLUGIN_BIN);
if (!bin || !existsSync(bin)) {
  console.error(`ESCOUADE_PLUGIN_BIN must name the app's escouade-plugin (cargo build --bin escouade-plugin): ${bin ?? 'unset'}`);
  process.exit(2);
}

// The version of Escouade that serves `run.js` plugins in WASM, as the tool says it (`escouade-plugin --version`, its
// `wasm-since` line): the plugins of this folder are judged for it, whatever version this build is.
const since = /^wasm-since (\S+)\r?$/m.exec(spawnSync(bin, ['--version'], { encoding: 'utf8' }).stdout ?? '')?.[1];
if (!since) {
  console.error(`${bin} --version does not say "wasm-since": build escouade-plugin from this tree`);
  process.exit(2);
}

/** A command run for one plugin, its output kept in `log`, printed whole once the plugin is checked: whether it passed. */
function run(log, cwd, command, ...args) {
  log.push(`\n$ ${[command, ...args].join(' ')}   (${cwd})\n`);
  // On Windows, npm and npx are .cmd scripts, which only a shell starts (the words here have no spaces to quote).
  const windows = process.platform === 'win32';
  return new Promise((done) => {
    const child = spawn(windows ? [command, ...args].join(' ') : command, windows ? [] : args, {
      cwd,
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: windows,
      env: { ...process.env, ESCOUADE_PLUGIN_BIN: bin },
    });
    child.stdout.on('data', (d) => log.push(String(d)));
    child.stderr.on('data', (d) => log.push(String(d)));
    child.on('error', (e) => {
      log.push(`✗ ${command} ${args.join(' ')}: ${e.message}\n`);
      done(false);
    });
    child.on('close', (code, signal) => {
      if (code !== 0) log.push(`✗ ${command} ${args.join(' ')}: exit ${code ?? signal}\n`);
      done(code === 0);
    });
  });
}

/** The steps of a plugin in TypeScript, until one fails: whether all passed. */
async function checkTypeScript({ name }, folder, log) {
  if (!(await run(log, folder, 'npm', 'install', '--no-audit', '--no-fund'))) return false;
  if (!(await run(log, folder, 'npm', 'run', 'typecheck')) || !(await run(log, folder, 'npm', 'test'))) return false;
  const built = join(tmp, `${name}.mjs`);
  const esbuild = ['esbuild', 'src/main.ts', '--bundle', '--platform=node', '--format=esm', '--target=node20', `--outfile=${built}`];
  if (!(await run(log, folder, 'npx', ...esbuild))) return false;
  if (lf(readFileSync(built, 'utf8')) !== lf(readFileSync(join(folder, 'dist', 'plugin.mjs'), 'utf8'))) {
    log.push(`✗ ${name}: dist/plugin.mjs is not the build of src/ (npm run build, then commit dist/)\n`);
    return false;
  }
  log.push(`✓ ${name}: dist/plugin.mjs is the build of src/\n`);
  return true;
}

/** Each step of a plugin, until one fails: whether all passed. */
async function check(plugin, log) {
  const { name, id, version } = plugin;
  const folder = join(HERE, name);
  const problems = problemsOf(folder, plugin);
  if (problems.length > 0) {
    for (const problem of problems) log.push(`✗ ${name}: ${problem}\n`);
    return false;
  }
  if (plugin.node) {
    if (!(await checkTypeScript(plugin, folder, log))) return false;
  } else if (!plugin.cargo && plugin.nodeTests && !(await run(log, folder, 'node', '--test'))) {
    return false;
  }
  // By its relative folder, as an author runs it beside the plugin; a reserved id is Escouade's own hand.
  const hand = String(id).startsWith('dev.escouade.') ? ['--first-party'] : [];
  const judged = ['--escouade', since];
  if (!(await run(log, HERE, bin, 'validate', name, ...hand, ...judged))) return false;
  if (!plugin.wasm) return run(log, HERE, bin, 'test', name, ...hand, ...judged);
  // Compiled once, as the store does it, then played: `test` would compile it again.
  const module = join(tmp, `${id}-${version}.wasm`);
  if (!(await run(log, HERE, bin, 'build', name, ...hand, '--out', tmp))) return false;
  return run(log, HERE, bin, 'test', name, ...hand, ...judged, '--wasm', module);
}

/** The tests of the crates of a workspace, in one go: one build, whatever their number. `--no-fail-fast`: as for the plugins. */
async function testCrates(workspace, crates) {
  if (crates.length === 0) return true;
  const log = [];
  const passed = await run(log, workspace, 'cargo', 'test', '--locked', '--no-fail-fast', ...crates.flatMap((c) => ['-p', c]));
  process.stdout.write(log.join(''));
  return passed;
}

/** `work` on each item, `width` at a time: their results, in the order of the items. */
async function eachAtOnce(items, width, work) {
  const results = new Array(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const i = next++;
      results[i] = await work(items[i]);
    }
  };
  await Promise.all(Array.from({ length: Math.max(1, Math.min(width, items.length)) }, worker));
  return results;
}

const lf = (s) => s.replace(/\r\n/g, '\n');
const tmp = mkdtempSync(join(tmpdir(), 'escouade-examples-'));
try {
  const names = plugins.map((p) => p.name);
  // Every plugin, even after one that fails: one failure must not hide another.
  const crates = plugins.filter((p) => p.cargo?.name);
  const names_of = (list) => list.map((p) => p.cargo.name);
  const started = Date.now();
  const tested = [
    await testCrates(WORKSPACE, names_of(crates.filter((p) => !p.wasm))),
    await testCrates(HERE, names_of(crates.filter((p) => p.wasm))),
  ].every(Boolean);
  if (crates.length > 0) console.log(`\ncargo test of ${crates.length} crates: ${Math.round((Date.now() - started) / 1000)} s`);
  const jobs = Number(process.env.ESCOUADE_CI_JOBS) || availableParallelism();
  const checking = Date.now();
  const passed = await eachAtOnce(plugins, jobs, async (plugin) => {
    const log = [];
    const ok = await check(plugin, log).catch((e) => {
      log.push(`✗ ${plugin.name}: ${e.message}\n`);
      return false;
    });
    process.stdout.write(`\n──── ${plugin.name}${ok ? '' : ' ✗'}\n${log.join('')}`);
    return ok;
  });
  console.log(`\n${plugins.length} plugins checked, ${jobs} at a time: ${Math.round((Date.now() - checking) / 1000)} s`);
  const failed = plugins.filter((_, i) => !passed[i]).map((p) => p.name);
  if (!tested) console.error(`\n✗ cargo test fails: its output above says for which crates`);
  if (failed.length > 0) console.error(`\n✗ ${failed.length} of ${names.length} plugins fail: ${failed.join(', ')}`);
  if (!tested || failed.length > 0) {
    process.exitCode = 1;
  } else {
    console.log(`\n✓ ${names.length} plugins pass their tests, validate and, for those it can start, the bench: ${names.join(', ')}`);
  }
} finally {
  rmSync(tmp, { recursive: true, force: true, maxRetries: 5, retryDelay: 300 });
}
