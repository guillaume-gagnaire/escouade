import { spawn, spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { readFileSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, isAbsolute, join, normalize, resolve, sep } from 'node:path';
import type { Compilation, Compiler } from './checks';
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

/** What one compilation is given: the manifest, the bundle's path in the plugin and its bytes. */
export interface CompileInput {
  manifest: Buffer;
  js: string;
  bundle: Buffer;
  /** A plugin of Escouade's own (`dev.escouade.*` from the app's repository): read with `--first-party`. */
  firstParty?: boolean;
}

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

/** The module of `run.js` compiled twice: the same bytes both times, or the reason it is refused. */
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
/** A module: 32 MB at most (team-rules and slack weigh less than 2). */
export const MODULE_MAX = 32 * 1024 * 1024;
const OUTPUT_MAX = 1024 * 1024;

export type Isolation = 'unshare' | 'none';

export interface SandboxOptions {
  /** The program and the words before `build` (`escouade-plugin`; the tests: `node fake.mjs`). */
  command: string[];
  /** The pinned Javy the image holds, given to escouade-plugin by `ESCOUADE_JAVY`. */
  javy?: string;
  /** `auto` (an empty network namespace when the kernel allows it), `require` (never without), `none`. */
  isolation?: 'auto' | 'require' | 'none';
  timeoutMs?: number;
  memoryBytes?: number;
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

/** The resident memory of a process group, in bytes, read in /proc (Linux); 0 elsewhere. */
function groupMemory(pgid: number): number {
  if (process.platform !== 'linux') return 0;
  let total = 0;
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
      if (Number(fields[2]) === pgid) total += Number(fields[21]) * 4096;
    } catch {
      // The process ended meanwhile.
    }
  }
  return total;
}

/** The environment of a compilation: its own folders, nothing of the store's variables. */
function envOf(dir: string, javy: string | undefined): Env {
  const env: Env = {
    PATH: process.platform === 'win32' ? process.env.PATH : '/usr/local/bin:/usr/bin:/bin',
    HOME: dir,
    TMPDIR: dir,
    LANG: 'C.UTF-8',
    ESCOUADE_DATA_DIR: join(dir, 'data'),
  };
  if (javy) env.ESCOUADE_JAVY = javy;
  if (process.platform === 'win32') {
    for (const k of ['SystemRoot', 'SYSTEMROOT', 'windir', 'PATHEXT', 'COMSPEC']) if (process.env[k]) env[k] = process.env[k];
    env.TEMP = env.TMP = env.USERPROFILE = env.LOCALAPPDATA = env.APPDATA = dir;
  }
  return env;
}

/** The last lines of what the compilation said on its error output, for the author. */
function said(stderr: string): string {
  const lines = stderr
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
  return (lines.slice(-6).join(' ') || 'escouade-plugin build failed without a word').slice(0, 600);
}

/**
 * The store's compiler: `escouade-plugin build` in a child process, sandboxed as the header says. One call, one
 * compilation in a new folder, removed after.
 */
export function sandboxedCompiler(o: SandboxOptions): CompileOne {
  const timeout = o.timeoutMs ?? COMPILE_TIMEOUT;
  const memory = o.memoryBytes ?? COMPILE_MEMORY;
  return async (input) => {
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
      const { code, stdout, stderr, killed } = await run(exe, args, envOf(dir, o.javy), dir, timeout, memory);
      if (killed === 'time') throw new Refused(`the compilation took more than ${Math.round(timeout / 1000)} s`);
      if (killed === 'memory') throw new Refused(`the compilation took more than ${Math.round(memory / 1024 / 1024)} MiB`);
      // escouade-plugin says why it refuses a bundle on a line of its own that starts with ✗; anything else (a crash,
      // a program that is not there) is the store's, and the version waits to be checked again.
      if (code !== 0) {
        if (stderr.includes('✗')) throw new Refused(said(stderr));
        throw new Error(`escouade-plugin build stopped (${code}): ${said(stderr)}`);
      }
      let report: { size?: number; sha256?: string; compiler?: Compiler };
      try {
        report = JSON.parse(stdout);
      } catch {
        throw new Error(`escouade-plugin build --json did not answer JSON: ${stdout.slice(0, 200)}`);
      }
      const wasm = (await readdir(out)).filter((n) => n.endsWith('.wasm'));
      if (wasm.length !== 1) throw new Error(`escouade-plugin build wrote ${wasm.length} modules`);
      const bytes = await readFile(join(out, wasm[0]!));
      if (bytes.length > MODULE_MAX) throw new Refused(`the module weighs more than ${MODULE_MAX / 1024 / 1024} MB`);
      const hash = sha256(bytes);
      const c = report.compiler;
      if (report.sha256 !== hash || report.size !== bytes.length || !c?.javy || !c.engine || !c.prelude)
        throw new Error('escouade-plugin build: its report does not name the module it wrote');
      return { bytes, sha256: hash, compiler: { javy: c.javy, engine: c.engine, prelude: c.prelude } };
    } finally {
      await rm(dir, { recursive: true, force: true }).catch(() => {});
    }
  };
}

/** Runs a program in its own process group, its outputs bounded, killed past its time or its memory. */
function run(
  exe: string,
  args: string[],
  env: Env,
  cwd: string,
  timeout: number,
  memory: number,
): Promise<{ code: number | null; stdout: string; stderr: string; killed: 'time' | 'memory' | null }> {
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
    let killed: 'time' | 'memory' | null = null;
    child.stdout.on('data', (d: Buffer) => {
      if (stdout.length < OUTPUT_MAX) stdout += d.toString('utf8');
    });
    child.stderr.on('data', (d: Buffer) => {
      if (stderr.length < OUTPUT_MAX) stderr += d.toString('utf8');
    });
    const kill = (why: 'time' | 'memory') => {
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
      if (child.pid && groupMemory(child.pid) > memory) kill('memory');
    }, 250);
    const done = () => {
      clearTimeout(timer);
      clearInterval(watch);
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
  return sandboxedCompiler({
    // A script of Node (the end-to-end tests' fake, tests/fake/fake-escouade-plugin.mjs) runs on this Node.
    command: /\.m?js$/.test(program) ? [process.execPath, program] : [program],
    javy: (env.STORE_JAVY ?? '').trim() || undefined,
    isolation: isolation === 'require' || isolation === 'none' ? isolation : 'auto',
  });
}
