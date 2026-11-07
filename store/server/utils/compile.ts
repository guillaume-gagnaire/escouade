import { spawn, spawnSync } from 'node:child_process';
import { lstat, mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { readFileSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, isAbsolute, join, normalize, resolve, sep } from 'node:path';
import type { Compilation, Compiler, JsCompiler, RustCompiler } from './checks';
import type { Env } from './config';
import { sha256 } from './versions';

// The eleventh check's compiler (ESC-323, docs/STORE.md §3.3): the `run.js` bundle of the frozen archive, never an
// `npm install`, compiled to WASM by `escouade-plugin build` (the pinned Javy and escouade-js it embeds, its prelude),
// twice, each time in a new folder of its own, and refused when the two modules differ.
//
// The store runs in a container of its own on Coolify, without the host's Docker socket (giving it would make root
// on the server anyone who goes through the store): the compilation is a child process of the store, sandboxed by
// what that container allows (the decision of ESC-323):
// - a network namespace of its own, empty (`unshare -rn`), when the kernel lets an unprivileged user make one;
//   `STORE_COMPILE_ISOLATION=require` refuses to compile without it, `none` does not try;
// - 2 minutes, then the whole process group is killed; 1 GiB of resident memory for the group, read in /proc;
// - a new folder each time, holding the manifest and the bundle alone, and a new data folder (no cache, no Javy
//   downloaded: the image's, by `STORE_JAVY`); an environment of its own, without the store's variables.
// Javy runs the bundle's initialisation itself, but inside wasmtime's WASM sandbox (Wizer): the plugin's code
// never runs as a process of the store, has no system call, no file and no network there.
//
// A `run.wasm` plugin is another matter (ESC-336, docs/PLUGINS.md §8.5.3, §8.5.4): cargo runs in native the build scripts
// and the procedural macros of its dependencies, as the user the store runs as, who could read the store's secrets in
// `/proc/<pid>/environ`. So:
// - `escouade-plugin` refuses, before anything runs, what is not on its closed list (`rust_native.json`), and it is the
//   store's defence against the code of an author; the sandbox comes on top of it;
// - each compilation runs in bubblewrap, in a root of its own (`STORE_COMPILE_ROOT`, a tree of the image that holds
//   neither Node nor the store): new user, PID, network, mount, IPC, UTS and cgroup namespaces, a new session, no
//   variable of the store's, no `HOME`, an empty `/etc`, and the folder of this compilation alone in `/work`;
// - there is no fallback: where bubblewrap cannot run (Docker's default seccomp refuses it), a `run.wasm` version waits
//   in `error`, the first hand's as a third party's; `STORE_COMPILE_ISOLATION=none` is heard outside production only;
// - two steps in each sandbox: `escouade-plugin fetch` with a network, which reads no code of the plugin and runs
//   nothing of it (`cargo fetch --locked`, crates.io alone since its refusals come first), then `build` without one;
// - 5 minutes, 2 GiB and 4 GiB of disk per compilation, as cargo downloads and writes what an author's lock asks.

/** What one compilation of a `run.js` plugin is given: the manifest, the bundle's path in the plugin and its bytes. */
export interface CompileJs {
  manifest: Buffer;
  js: string;
  bundle: Buffer;
  /** A plugin of Escouade's own (`dev.escouade.*` from the app's repository): read with `--first-party`. */
  firstParty?: boolean;
}

/** What one compilation of a `run.wasm` plugin is given: the manifest, its `run.wasm` and the plugin's whole folder. */
export interface CompileRust {
  manifest: Buffer;
  wasm: string;
  /** The files of the frozen archive, by their path in the plugin: bytes alone, never a link (`readArchive`). */
  files: ReadonlyMap<string, Buffer>;
  firstParty?: boolean;
}

export type CompileInput = CompileJs | CompileRust;

/** A module made, with what made it. */
export interface Compiled {
  bytes: Buffer;
  sha256: string;
  compiler: Compiler;
}

/** The bundle does not compile (an import refused, a syntax error, a loop at its start): the plugin's fault. */
export class Refused extends Error {}

/** One compilation; throws `Refused` for the plugin's fault, any other error for the store's (the version retries). */
export type CompileOne = (input: CompileInput) => Promise<Compiled>;

/** The module of a plugin compiled twice: the same bytes both times, or the reason it is refused. */
export async function compileTwice(one: CompileOne, input: CompileInput): Promise<{ compilation: Compilation; bytes?: Buffer }> {
  const run = async () => {
    try {
      return await one(input);
    } catch (e) {
      if (e instanceof Refused) return e;
      throw e;
    }
  };
  const first = await run();
  if (first instanceof Refused) return { compilation: { state: 'refused', message: first.message } };
  const second = await run();
  if (second instanceof Refused) return { compilation: { state: 'refused', message: second.message } };
  const same =
    first.sha256 === second.sha256 &&
    first.bytes.equals(second.bytes) &&
    JSON.stringify(first.compiler) === JSON.stringify(second.compiler);
  if (!same) return { compilation: { state: 'different', first: first.sha256, second: second.sha256 } };
  return {
    compilation: { state: 'built', sha256: first.sha256, size: first.bytes.length, compiler: first.compiler },
    bytes: first.bytes,
  };
}

/** Two minutes for a compilation, as the review's container. */
export const COMPILE_TIMEOUT = 120_000;
/** 1 GiB of resident memory for the compilation's processes together. */
export const COMPILE_MEMORY = 1024 * 1024 * 1024;
/** Five minutes for a `run.wasm`, and two for its `cargo fetch` before it (docs/PLUGINS.md §8.5.3). */
export const RUST_TIMEOUT = 300_000;
export const FETCH_TIMEOUT = 120_000;
/** 2 GiB for a `run.wasm`: 1 GiB stops `lto = "fat"`, 1.5 passes (the study's measure). */
export const RUST_MEMORY = 2 * 1024 * 1024 * 1024;
/** 4 GiB in the folder of a `run.wasm`: its registry and its `target`, on the server's disk. */
export const RUST_DISK = 4 * 1024 * 1024 * 1024;
/** A module: 32 MB at most (team-rules and slack weigh less than 2). */
export const MODULE_MAX = 32 * 1024 * 1024;
const OUTPUT_MAX = 1024 * 1024;

export type Isolation = 'unshare' | 'none';

export interface SandboxOptions {
  /** The program and the words before `build` (`escouade-plugin`; the tests: `node fake.mjs`). */
  command: string[];
  /** The pinned Javy the image holds, given to escouade-plugin by `ESCOUADE_JAVY`. */
  javy?: string;
  /**
   * escouade-plugin's data folder (its downloaded Javy): a new one per compilation when left out, as the store wants;
   * the local review keeps one of its own, so that Javy is downloaded once (ESC-324).
   */
  data?: string;
  /** `auto` (an empty network namespace when the kernel allows it), `require` (never without), `none`. */
  isolation?: 'auto' | 'require' | 'none';
  timeoutMs?: number;
  /** How long `escouade-plugin fetch` may take: the local review's first one downloads the toolchain too. */
  fetchTimeoutMs?: number;
  memoryBytes?: number;
  /**
   * Where a `run.wasm` compiles: bubblewrap, or `none` outside production (the tests, a developer's machine). Left out,
   * this store has no sandbox for one, and such a version waits in `error`.
   */
  rust?: RustSandbox | 'none';
  /** Why there is no sandbox here, for the version that waits. */
  noSandbox?: string;
}

/** Bubblewrap and what it shows a `run.wasm` compilation. */
export interface RustSandbox {
  bwrap: string;
  /** The tree of the image that the sandbox takes its `/usr`, `/opt` and certificates from. */
  root: string;
  /** `escouade-plugin` and the toolchain, as the sandbox sees them. */
  program: string;
  toolchain: string;
}

/** What a compilation sees of its folder, in the sandbox, and its programs. */
const WORK = '/work';
const SANDBOX_PATH = '/usr/local/bin:/usr/bin:/bin';

/** bubblewrap's words up to the program: a root of its own, `dir` as `/work`, `env` alone; a network when asked. */
export function bwrapArgs(s: RustSandbox, dir: string, network: boolean, env: Env): string[] {
  const root = s.root.replace(/\/+$/, '');
  return [
    '--unshare-all',
    ...(network ? ['--share-net'] : []),
    '--clearenv',
    '--new-session',
    '--die-with-parent',
    '--ro-bind',
    `${root}/usr`,
    '/usr',
    '--ro-bind',
    `${root}/opt`,
    '/opt',
    ...['bin', 'sbin', 'lib', 'lib64'].flatMap((d) => ['--symlink', `usr/${d}`, `/${d}`]),
    // No `/proc`, neither a new one, which a kernel refuses to mount where the container hides parts of its own, nor
    // the container's, which would show its processes: nothing of a compilation needs it.
    '--dev',
    '/dev',
    '--tmpfs',
    '/tmp',
    // No `/etc` of any kind: an `include_bytes!` with an absolute path finds nothing. What a download needs, for it alone.
    '--dir',
    '/etc',
    ...(network
      ? [
          '--ro-bind',
          `${root}/etc/ssl`,
          '/etc/ssl',
          '--ro-bind-try',
          '/etc/resolv.conf',
          '/etc/resolv.conf',
          '--ro-bind-try',
          '/etc/hosts',
          '/etc/hosts',
        ]
      : []),
    '--bind',
    dir,
    WORK,
    '--chdir',
    WORK,
    ...Object.entries(env).flatMap(([k, v]) => ['--setenv', k, v ?? '']),
    '--',
  ];
}

/**
 * What runs `command` in the sandbox: the program, its words and its environment. Bubblewrap's own stays small too:
 * `--clearenv` empties what its child gets, but its pid 1 in the sandbox keeps what it started with, in `/proc/1/environ`.
 */
export function inSandbox(s: RustSandbox, dir: string, network: boolean, env: Env, command: string[]): [string, string[], Env] {
  return [s.bwrap, [...bwrapArgs(s, dir, network, env), ...command], { PATH: SANDBOX_PATH }];
}

const probedBwrap = new Map<string, string | null>();

/** Why bubblewrap cannot make its sandbox here, in its own words; null when it can. Asked once. */
export function whyNoBubblewrap(s: RustSandbox): string | null {
  if (process.platform !== 'linux') return 'not Linux';
  const key = `${s.bwrap}\0${s.root}`;
  let why = probedBwrap.get(key);
  if (why === undefined) {
    const [exe, args, env] = inSandbox(s, tmpdir(), false, {}, ['/bin/true']);
    const r = spawnSync(exe, args, { encoding: 'utf8', timeout: 10_000, env: env as NodeJS.ProcessEnv });
    why = r.status === 0 ? null : r.error ? r.error.message : `${r.stderr.trim() || 'no word'} (code ${r.status})`;
    probedBwrap.set(key, why);
  }
  return why;
}

let probed: Isolation | null = null;

/** Whether this kernel lets the store's user make a user and network namespace (`unshare -rn`), asked once. */
export function canUnshare(): boolean {
  if (process.platform !== 'linux') return false;
  probed ??= spawnSync('unshare', ['-rn', 'true'], { stdio: 'ignore', timeout: 5_000 }).status === 0 ? 'unshare' : 'none';
  return probed === 'unshare';
}

/** The isolation a compilation runs with, from the options; throws when `require` cannot be met. */
export function isolationOf(wanted: SandboxOptions['isolation'] = 'auto'): Isolation {
  if (wanted === 'none') return 'none';
  if (canUnshare()) return 'unshare';
  if (wanted === 'require')
    throw new Error('STORE_COMPILE_ISOLATION=require: this container cannot make a network namespace (unshare -rn)');
  return 'none';
}

/** The resident memory of a compilation in bytes, read in /proc (Linux), 0 elsewhere: the process group of `root`, and all
 * that descends from it, which bubblewrap's new session takes out of the group. */
function treeMemory(root: number): number {
  if (process.platform !== 'linux') return 0;
  const seen = new Map<number, { ppid: number; pgrp: number; rss: number }>();
  let names: string[];
  try {
    names = readdirSync('/proc');
  } catch {
    return 0;
  }
  for (const name of names) {
    if (!/^\d+$/.test(name)) continue;
    try {
      const stat = readFileSync(`/proc/${name}/stat`, 'utf8');
      // After the command's name in parentheses: state ppid pgrp … rss is the 22nd field after it.
      const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
      seen.set(Number(name), { ppid: Number(fields[1]), pgrp: Number(fields[2]), rss: Number(fields[21]) * 4096 });
    } catch {
      // The process ended meanwhile.
    }
  }
  const under = (pid: number) => {
    for (let at = seen.get(pid); at; at = seen.get(at.ppid)) if (at.pgrp === root || at.ppid === root) return true;
    return pid === root;
  };
  let total = 0;
  for (const [pid, p] of seen) if (under(pid)) total += p.rss;
  return total;
}

/** The bytes under `dir`, its links not followed. */
async function sizeOf(dir: string): Promise<number> {
  let total = 0;
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) total += await sizeOf(path);
    else total += (await lstat(path)).size;
  }
  return total;
}

/** The environment of a compilation: its own folders, nothing of the store's variables. */
function envOf(dir: string, javy: string | undefined, data?: string): Env {
  const env: Env = {
    PATH: process.platform === 'win32' ? process.env.PATH : '/usr/local/bin:/usr/bin:/bin',
    HOME: dir,
    TMPDIR: dir,
    LANG: 'C.UTF-8',
    ESCOUADE_DATA_DIR: data ?? join(dir, 'data'),
  };
  if (javy) env.ESCOUADE_JAVY = javy;
  if (process.platform === 'win32') {
    for (const k of ['SystemRoot', 'SYSTEMROOT', 'windir', 'PATHEXT', 'COMSPEC']) if (process.env[k]) env[k] = process.env[k];
    env.TEMP = env.TMP = env.USERPROFILE = env.LOCALAPPDATA = env.APPDATA = dir;
  }
  return env;
}

/** What the compilation said on its error output, for the author: from its line of ✗, else its last lines. */
export function said(stderr: string): string {
  const lines = stderr
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
  const refused = lines.findIndex((l) => l.includes('✗'));
  const words = refused < 0 ? lines.slice(-6) : lines.slice(refused);
  return (words.join(' ') || 'escouade-plugin build failed without a word').slice(0, 1000);
}

/** The files of a plugin under `to`, each written as a new regular file: nothing in it is a link, and none leaves it. */
export async function materialise(files: ReadonlyMap<string, Buffer>, to: string): Promise<void> {
  for (const [path, bytes] of files) {
    const rel = normalize(path);
    if (!rel || rel === '.' || rel.startsWith('..') || isAbsolute(rel) || /^[A-Za-z]:/.test(rel))
      throw new Refused(`${path}: not a path in the plugin`);
    const at = join(to, rel);
    await mkdir(dirname(at), { recursive: true });
    await writeFile(at, bytes, { flag: 'wx' });
  }
}

/** What a `run.wasm` was compiled with, as `build --json` says it; null when a word is missing. */
export const rustCompilerOf = (c: Partial<RustCompiler>): Compiler | null =>
  c.rust && c.target && c.canon && c.lock ? { rust: c.rust, target: c.target, canon: c.canon, lock: c.lock } : null;

/** The module `build --json` left in `out`, once what it said of it is true; `compiler` reads what made it. */
async function moduleOf(
  out: string,
  stdout: string,
  compiler: (c: Partial<JsCompiler & RustCompiler>) => Compiler | null,
): Promise<Compiled> {
  const wasm = (await readdir(out)).filter((n) => n.endsWith('.wasm'));
  if (wasm.length !== 1) throw new Error(`escouade-plugin build wrote ${wasm.length} modules`);
  return checkedModule(await readFile(join(out, wasm[0]!)), stdout, compiler);
}

/** `bytes`, once what `build --json` said of them on `stdout` is true. */
export function checkedModule(
  bytes: Buffer,
  stdout: string,
  compiler: (c: Partial<JsCompiler & RustCompiler>) => Compiler | null,
): Compiled {
  let report: { size?: number; sha256?: string; compiler?: Partial<JsCompiler & RustCompiler> };
  try {
    report = JSON.parse(stdout);
  } catch {
    throw new Error(`escouade-plugin build --json did not answer JSON: ${stdout.slice(0, 200)}`);
  }
  if (bytes.length > MODULE_MAX) throw new Refused(`the module weighs more than ${MODULE_MAX / 1024 / 1024} MB`);
  const hash = sha256(bytes);
  const made = report.compiler ? compiler(report.compiler) : null;
  if (report.sha256 !== hash || report.size !== bytes.length || !made)
    throw new Error('escouade-plugin build: its report does not name the module it wrote');
  return { bytes, sha256: hash, compiler: made };
}

/** One step of a `run.wasm` compilation in `dir`, in its sandbox; a limit passed or a refusal throws `Refused`. */
async function rustStep(
  o: SandboxOptions,
  dir: string,
  words: string[],
  limits: { network: boolean; timeout: number; memory: number },
): Promise<string> {
  const sandbox = o.rust!;
  const at = sandbox === 'none' ? dir : WORK;
  const env: Env =
    sandbox === 'none'
      ? envOf(dir, undefined, o.data ?? join(dir, 'data'))
      : { PATH: SANDBOX_PATH, TMPDIR: `${WORK}/tmp`, LANG: 'C.UTF-8', ESCOUADE_DATA_DIR: `${WORK}/data` };
  if (sandbox !== 'none') {
    env.ESCOUADE_RUST_TOOLCHAIN = sandbox.toolchain;
    // rustc finds its libraries by `$ORIGIN`, which ld.so reads in `/proc/self/exe`: the sandbox has none.
    env.LD_LIBRARY_PATH = `${sandbox.toolchain}/lib`;
  }
  if (!limits.network) env.CARGO_NET_OFFLINE = 'true';
  const named = words.map((w) => w.replace('{work}', at));
  const [program, ...before] = o.command;
  const [exe, args, outer]: [string, string[], Env] =
    sandbox === 'none' ? [program!, [...before, ...named], env] : inSandbox(sandbox, dir, limits.network, env, [sandbox.program, ...named]);
  const { code, stdout, stderr, killed } = await run(exe, args, outer, dir, limits.timeout, limits.memory, RUST_DISK);
  if (killed === 'time') throw new Refused(`${words[0]} took more than ${Math.round(limits.timeout / 1000)} s`);
  if (killed === 'memory') throw new Refused(`${words[0]} took more than ${Math.round(limits.memory / 1024 / 1024)} MiB`);
  if (killed === 'disk') throw new Refused(`${words[0]} wrote more than ${Math.round(RUST_DISK / 1024 / 1024)} MiB`);
  if (code !== 0) {
    if (stderr.includes('✗')) throw new Refused(said(stderr));
    throw new Error(`escouade-plugin ${words[0]} stopped (${code}): ${said(stderr)}`);
  }
  return stdout;
}

/** One compilation of a `run.wasm` plugin, in a new folder: its dependencies with a network, then the rest without. */
async function compileRust(o: SandboxOptions, input: CompileRust): Promise<Compiled> {
  if (!o.rust) throw new Error(`no sandbox for a run.wasm plugin here: ${o.noSandbox ?? 'not configured'} (docs/STORE.md §3.3)`);
  const dir = await mkdtemp(join(tmpdir(), 'escouade-compile-'));
  try {
    await materialise(input.files, join(dir, 'plugin'));
    await writeFile(join(dir, 'plugin', 'escouade-plugin.json'), input.manifest);
    for (const name of ['out', 'data', 'tmp']) await mkdir(join(dir, name));
    const party = input.firstParty ? ['--first-party'] : [];
    const memory = o.memoryBytes ?? RUST_MEMORY;
    await rustStep(o, dir, ['fetch', '{work}/plugin', '--lang', 'en', ...party], {
      network: true,
      timeout: o.fetchTimeoutMs ?? FETCH_TIMEOUT,
      memory,
    });
    const words = ['build', '{work}/plugin', '--out', '{work}/out', '--json', '--lang', 'en', ...party];
    const stdout = await rustStep(o, dir, words, { network: false, timeout: o.timeoutMs ?? RUST_TIMEOUT, memory });
    return await moduleOf(join(dir, 'out'), stdout, rustCompilerOf);
  } finally {
    await rm(dir, { recursive: true, force: true }).catch(() => {});
  }
}

/**
 * The store's compiler: `escouade-plugin build` in a child process, sandboxed as the header says. One call, one
 * compilation in a new folder, removed after.
 */
export function sandboxedCompiler(o: SandboxOptions): CompileOne {
  const timeout = o.timeoutMs ?? COMPILE_TIMEOUT;
  const memory = o.memoryBytes ?? COMPILE_MEMORY;
  return async (input) => {
    if ('wasm' in input) return compileRust(o, input);
    const isolation = isolationOf(o.isolation);
    const js = normalize(input.js.replace(/^\.\//, ''));
    if (!js || js.startsWith('..') || js.startsWith(sep) || /^[A-Za-z]:/.test(js))
      throw new Refused(`${input.js}: not a path in the plugin`);
    const dir = await mkdtemp(join(tmpdir(), 'escouade-compile-'));
    try {
      const plugin = join(dir, 'plugin');
      const out = join(dir, 'out');
      await mkdir(dirname(join(plugin, js)), { recursive: true });
      await writeFile(join(plugin, 'escouade-plugin.json'), input.manifest);
      await writeFile(join(plugin, js), input.bundle);
      await mkdir(out);
      const [program, ...before] = o.command;
      const words = [...before, 'build', plugin, '--out', out, '--json', '--lang', 'en', ...(input.firstParty ? ['--first-party'] : [])];
      const [exe, args] = isolation === 'unshare' ? ['unshare', ['-rn', '--', program!, ...words]] : [program!, words];
      const { code, stdout, stderr, killed } = await run(exe, args, envOf(dir, o.javy, o.data), dir, timeout, memory);
      if (killed === 'time') throw new Refused(`the compilation took more than ${Math.round(timeout / 1000)} s`);
      if (killed === 'memory') throw new Refused(`the compilation took more than ${Math.round(memory / 1024 / 1024)} MiB`);
      // escouade-plugin says why it refuses a bundle on a line of its own that starts with ✗; anything else (a crash,
      // a program that is not there) is the store's, and the version waits to be checked again.
      if (code !== 0) {
        if (stderr.includes('✗')) throw new Refused(said(stderr));
        throw new Error(`escouade-plugin build stopped (${code}): ${said(stderr)}`);
      }
      return await moduleOf(out, stdout, (c) =>
        c.javy && c.engine && c.prelude ? { javy: c.javy, engine: c.engine, prelude: c.prelude } : null,
      );
    } finally {
      await rm(dir, { recursive: true, force: true }).catch(() => {});
    }
  };
}

/** Runs a program in its own process group, its outputs bounded, killed past its time, its memory or its folder's size. */
function run(
  exe: string,
  args: string[],
  env: Env,
  cwd: string,
  timeout: number,
  memory: number,
  disk?: number,
): Promise<{ code: number | null; stdout: string; stderr: string; killed: 'time' | 'memory' | 'disk' | null }> {
  return new Promise((resolve, reject) => {
    const child = spawn(exe, args, {
      cwd,
      env: env as NodeJS.ProcessEnv,
      stdio: ['ignore', 'pipe', 'pipe'],
      detached: process.platform !== 'win32',
      windowsHide: true,
    });
    let stdout = '';
    let stderr = '';
    let killed: 'time' | 'memory' | 'disk' | null = null;
    child.stdout.on('data', (d: Buffer) => {
      if (stdout.length < OUTPUT_MAX) stdout += d.toString('utf8');
    });
    child.stderr.on('data', (d: Buffer) => {
      if (stderr.length < OUTPUT_MAX) stderr += d.toString('utf8');
    });
    const kill = (why: 'time' | 'memory' | 'disk') => {
      killed ??= why;
      try {
        if (process.platform === 'win32') child.kill('SIGKILL');
        else process.kill(-child.pid!, 'SIGKILL');
      } catch {
        // Already gone.
      }
    };
    const timer = setTimeout(() => kill('time'), timeout);
    const watch = setInterval(() => {
      if (child.pid && treeMemory(child.pid) > memory) kill('memory');
    }, 250);
    let weighing = false;
    const scale = setInterval(() => {
      if (!disk || weighing) return;
      weighing = true;
      sizeOf(cwd)
        .then((size) => size > disk && kill('disk'))
        .catch(() => {})
        .finally(() => (weighing = false));
    }, 3000);
    const done = () => {
      clearTimeout(timer);
      clearInterval(watch);
      clearInterval(scale);
    };
    child.on('error', (e) => {
      done();
      reject(new Error(`the store's compiler ${exe} could not start: ${e.message}`));
    });
    child.on('close', (code) => {
      done();
      resolve({ code, stdout, stderr, killed });
    });
  });
}

/** The store's compiler from its variables: `STORE_ESCOUADE_PLUGIN` (and `STORE_JAVY`), or null without it. */
export function compilerOf(env: Env): CompileOne | null {
  const named = (env.STORE_ESCOUADE_PLUGIN ?? '').trim();
  if (!named) return null;
  // A path relative to the store's folder (each compilation runs in a folder of its own); a bare name, on the PATH.
  const program = /[\\/]/.test(named) && !isAbsolute(named) ? resolve(named) : named;
  const isolation = (env.STORE_COMPILE_ISOLATION ?? '').trim();
  const got = isolation === 'none' ? 'none' : canUnshare() ? 'unshare' : isolation === 'require' ? 'refused' : 'none';
  console.info(
    `[store] compiler: ${program}, ${
      got === 'unshare'
        ? 'without network (unshare -rn)'
        : got === 'refused'
          ? 'no network namespace here: STORE_COMPILE_ISOLATION=require refuses to compile'
          : 'with the container’s network (no namespace allowed here)'
    }`,
  );
  const root = (env.STORE_COMPILE_ROOT ?? '').trim();
  const toolchain = (env.STORE_RUST_TOOLCHAIN ?? '').trim();
  const sandbox = root && toolchain ? { bwrap: 'bwrap', root, program: '/usr/local/bin/escouade-plugin', toolchain } : null;
  const off = isolation === 'none' && env.NODE_ENV !== 'production';
  const noSandbox = off ? null : sandbox ? whyNoBubblewrap(sandbox) : 'STORE_COMPILE_ROOT and STORE_RUST_TOOLCHAIN are not set';
  const rust = off ? 'none' : sandbox && !noSandbox ? sandbox : undefined;
  console.info(
    rust === 'none'
      ? '[store] run.wasm: without a sandbox (STORE_COMPILE_ISOLATION=none, outside production)'
      : rust
        ? '[store] run.wasm: in a sandbox (bubblewrap)'
        : `[store] run.wasm: no sandbox here (${noSandbox}): such versions wait in error`,
  );
  return sandboxedCompiler({
    // A script of Node (the end-to-end tests' fake, tests/fake/fake-escouade-plugin.mjs) runs on this Node.
    command: /\.m?js$/.test(program) ? [process.execPath, program] : [program],
    javy: (env.STORE_JAVY ?? '').trim() || undefined,
    isolation: isolation === 'require' || isolation === 'none' ? isolation : 'auto',
    rust,
    noSandbox: noSandbox ?? undefined,
  });
}
