// What plugins/ci.mjs decides without running anything: the sort of a plugin, what its folder must hold, which plugins a
// change touches. Its tests are in scripts/plugins-ci.test.ts.
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

export const MANIFEST = 'escouade-plugin.json';
const APP_PROGRAM = '{escouade}/escouade-plugins';
/** What the manifest of a plugin the app gives says as its version: the app puts its own (`builtin::manifest_text`). */
const THE_APPS_VERSION = '0.0.0';

/**
 * What judges or builds every plugin: a change there runs them all. `crates/` is the kit and the bench's library, the
 * rest the bench's other half, the manifests' schema, the SDK of the TypeScript ones, the Rust of all, and this script.
 */
const FOR_ALL = [
  'crates/',
  'schemas/',
  'sdk/plugin/',
  'src-tauri/src/bin/escouade-plugin.rs',
  'src-tauri/src/plugins/',
  'src-tauri/Cargo.lock',
  'src-tauri/rust-toolchain.toml',
  'plugins/ci.mjs',
  'plugins/ci-lib.mjs',
  'plugins/Cargo.toml',
  'plugins/Cargo.lock',
  'plugins/rust-toolchain.toml',
];

/** `name` and `version` of the [package] of a Cargo.toml, and the folders of its path dependencies under `plugins/`. */
export function readCargo(text) {
  const section = /^\[package\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/m.exec(text)?.[1] ?? '';
  const field = (key) => new RegExp(`^${key}\\s*=\\s*"([^"]*)"`, 'm').exec(section)?.[1];
  const plugins = [...text.matchAll(/path\s*=\s*"\.\.\/([^"./]+)\/?"/g)].map((m) => m[1]);
  return { name: field('name'), version: field('version'), plugins };
}

/** The plugin in `folder`, as ci.mjs treats it. */
export function readPlugin(folder, name) {
  const manifest = JSON.parse(readFileSync(join(folder, MANIFEST), 'utf8'));
  const has = (file) => existsSync(join(folder, file));
  const cargo = has('Cargo.toml') ? readCargo(readFileSync(join(folder, 'Cargo.toml'), 'utf8')) : null;
  return {
    name,
    id: manifest.id,
    version: manifest.version,
    // The app's program (docs/PLUGINS.md §8.6.1): `--builtin`, and no bench, which has no such program to start.
    ofTheApp: manifest.run?.command === APP_PROGRAM,
    wasm: typeof manifest.run?.wasm === 'string',
    node: has('package.json'),
    nodeTests: has('test'),
    cargo,
  };
}

/** The plugins of `folder`, by the name of their subfolder. */
export function listPlugins(folder) {
  return readdirSync(folder, { withFileTypes: true })
    .filter((d) => d.isDirectory() && existsSync(join(folder, d.name, MANIFEST)))
    .map((d) => readPlugin(join(folder, d.name), d.name));
}

/** What is wrong with the folder of `plugin`, whatever its sort, in sentences. */
export function problemsOf(folder, plugin) {
  const problems = [];
  for (const file of ['LICENSE', 'CHANGELOG.md']) {
    const path = join(folder, file);
    if (!existsSync(path) || statSync(path).size === 0) problems.push(`${file} is missing or empty`);
  }
  const { cargo } = plugin;
  if (cargo) {
    if (!cargo.name || !cargo.version) problems.push('Cargo.toml has no [package] name and version');
    // Only for the time the app stamps it: a module's manifest says its own version, as the store reads it.
    const stamped = plugin.ofTheApp && plugin.version === THE_APPS_VERSION;
    if (cargo.version && !stamped && cargo.version !== plugin.version) {
      problems.push(`${MANIFEST} says version ${plugin.version}, Cargo.toml ${cargo.version}`);
    }
    if (plugin.wasm && plugin.ofTheApp) problems.push(`${MANIFEST} has both the app's program and run.wasm`);
  }
  return problems;
}

/**
 * Those of `plugins` that `files` (paths from the repository's root, in `/`) touch: their folder, or a plugin they
 * depend on by path, and all of them for a file of FOR_ALL.
 */
export function touchedBy(files, plugins) {
  if (files.some((f) => FOR_ALL.some((p) => (p.endsWith('/') ? f.startsWith(p) : f === p)))) return plugins;
  const touched = new Set(plugins.filter((p) => files.some((f) => f.startsWith(`plugins/${p.name}/`))).map((p) => p.name));
  // Until nothing is added: mistral stands on acp, and whatever stands on mistral on acp too.
  for (let grew = true; grew;) {
    grew = false;
    for (const p of plugins) {
      if (!touched.has(p.name) && p.cargo?.plugins.some((name) => touched.has(name))) {
        touched.add(p.name);
        grew = true;
      }
    }
  }
  return plugins.filter((p) => touched.has(p.name));
}

/** What the arguments ask: `--changed [<ref>]`, a Git revision to compare with, by default where `main` was left. */
export function parseArgs(args) {
  const [flag, base, ...rest] = args;
  if (flag === undefined) return { changed: false };
  if (flag !== '--changed' || base?.startsWith('-') || rest.length > 0) {
    throw new Error(`usage: node plugins/ci.mjs [--changed [<git revision>]], not: ${args.join(' ')}`);
  }
  return { changed: true, base };
}
