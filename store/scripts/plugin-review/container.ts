import { spawn, spawnSync } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import {
  checkedModule,
  materialise,
  MODULE_MAX,
  Refused,
  rustCompilerOf,
  said,
  type Compiled,
  type CompileOne,
} from '../../server/utils/compile';
import type { Engine } from './bench';

// A third party's `run.wasm` compiled again by the review (ESC-337, docs/PLUGINS.md §8.5.5): never on the bare machine.
// cargo runs in native the build scripts and the procedural macros of what it compiles, and an `include_bytes!` with an
// absolute path would bring a file of this machine into the module. So the steps of the store (docs/STORE.md §3.3), each
// in a container of its own, which shows neither `HOME`, nor `~/.escouade-signing`, nor the repository:
//
// 1. `prepare`: the two volumes given to the unprivileged user; nothing of the plugin is mounted;
// 2. `fetch`, with a network: `escouade-plugin fetch` refuses what the closed list refuses before cargo starts, and nothing
//    of the plugin runs. Its toolchain and crates stay in a volume of their own, downloaded once;
// 3. `build`, with `--network=none`, the volume of the tools read-only: whatever a script does there, it leaves nothing
//    for the next review. Its `target` is in the volume of this compilation, removed after;
// 4. `read`, with `--network=none`: the module, on the standard output.
//
// What the host mounts is read-only, the plugin's regular files and the Linux `escouade-plugin`, and no folder of this
// machine is writable from a container: a link a script leaves behind has nothing to point at here. Bubblewrap is not
// needed: it does not run under the default seccomp of a real Docker (ESC-356), and the container already is the sandbox.

/** `rust:…-trixie`: `cc` links the scripts and the macros, and Debian 13's glibc runs the `escouade-plugin` of Ubuntu 24.04. */
export const DEFAULT_RUST_IMAGE = 'docker.io/library/rust:1.98.0-trixie';
/** Toolchain and crates, kept from one review to the next; only `fetch` writes there, and it runs nothing of a plugin. */
export const TOOLS_VOLUME = 'escouade-review-rust-1.98.0';
/** The pull of the image and the first download of the toolchain: 654 MB. */
export const FETCH_LIMIT = 15 * 60_000;
export const BUILD_LIMIT = 10 * 60_000;
const QUICK_LIMIT = 60_000;
/** `lto = "fat"` takes 1.5 GiB (docs/PLUGINS.md §8.5.3): 3 GiB leaves the rest to `build.rs` and to the macros. */
const MEMORY = '3g';
const OOM = 137;

export interface ContainerSetup {
  engine: Engine;
  /** A Linux x64 `escouade-plugin`, mounted read-only. */
  cli: string;
  image: string;
  /** The folder the plugin's files are written to, outside the repository: the only one a container sees of this machine. */
  work: string;
  /** Tests: another volume, shorter limits. */
  tools?: string;
  fetchLimit?: number;
  buildLimit?: number;
}

/** What every step but `prepare` has: read-only, no capability, an unprivileged user, the program mounted read-only. */
function confined(name: string, cli: string): string[] {
  return [
    'run',
    '--rm',
    '--name',
    name,
    '--read-only',
    '--tmpfs',
    '/tmp:rw,size=256m',
    `--memory=${MEMORY}`,
    `--memory-swap=${MEMORY}`,
    '--pids-limit=1024',
    '--cap-drop=ALL',
    '--security-opt=no-new-privileges',
    '--user=65534:65534',
    '--env=HOME=/tmp',
    '--env=LANG=C.UTF-8',
    '--volume',
    `${cli}:/usr/local/bin/escouade-plugin:ro`,
  ];
}

const hand = (firstParty?: boolean) => (firstParty ? ['--first-party'] : []);

/** Step 1: the volumes for the unprivileged user. As root but for `chown`, without a network, without the plugin. */
export function prepareArgs(name: string, image: string, tools: string, work: string): string[] {
  return [
    'run',
    '--rm',
    '--name',
    name,
    '--network=none',
    '--read-only',
    '--cap-drop=ALL',
    '--cap-add=CHOWN',
    '--security-opt=no-new-privileges',
    '--user=0:0',
    '--volume',
    `${tools}:/tools`,
    '--volume',
    `${work}:/work`,
    image,
    'chown',
    '65534:65534',
    '/tools',
    '/work',
  ];
}

/** Step 2: `escouade-plugin fetch`, with a network, into the volume of the tools. */
export function fetchArgs(name: string, s: ContainerSetup, tools: string, plugin: string, firstParty?: boolean): string[] {
  return [
    ...confined(name, s.cli),
    '--env=ESCOUADE_DATA_DIR=/tools',
    '--volume',
    `${tools}:/tools`,
    '--volume',
    `${plugin}:/plugin:ro`,
    s.image,
    'escouade-plugin',
    'fetch',
    '/plugin',
    '--lang',
    'en',
    ...hand(firstParty),
  ];
}

/** Step 3: `escouade-plugin build`, without a network, the tools read-only, on a copy of the crates it may write to. */
export function buildArgs(name: string, s: ContainerSetup, tools: string, work: string, plugin: string, firstParty?: boolean): string[] {
  const script =
    'mkdir -p /work/data/tools /work/out && cp -r /tools/tools/cargo-home /work/data/tools/cargo-home && exec escouade-plugin "$@"';
  return [
    ...confined(name, s.cli),
    '--network=none',
    '--env=ESCOUADE_DATA_DIR=/work/data',
    '--env=ESCOUADE_RUST_TOOLCHAIN=/tools/tools/rust-1.98.0',
    '--env=CARGO_NET_OFFLINE=true',
    '--volume',
    `${tools}:/tools:ro`,
    '--volume',
    `${work}:/work`,
    '--volume',
    `${plugin}:/plugin:ro`,
    s.image,
    'sh',
    '-ec',
    script,
    'sh',
    'build',
    '/plugin',
    '--out',
    '/work/out',
    '--json',
    '--lang',
    'en',
    ...hand(firstParty),
  ];
}

/** Step 4: the module, read by a container of its own: no folder of this machine is writable by a compilation. */
export function readArgs(name: string, s: ContainerSetup, work: string): string[] {
  return [...confined(name, s.cli), '--network=none', '--volume', `${work}:/work:ro`, s.image, 'sh', '-ec', 'cat /work/out/*.wasm'];
}

interface Ran {
  code: number | null;
  stdout: Buffer;
  stderr: string;
  killed: boolean;
}

/** Runs `args`, its output bounded (a module and its report), the container killed past `limit`. */
function step(e: Engine, args: string[], limit: number, name: string): Promise<Ran> {
  const { program, prefix = [] } = e;
  return new Promise((done, failed) => {
    const child = spawn(program, [...prefix, ...args], { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
    const chunks: Buffer[] = [];
    let size = 0;
    let stderr = '';
    let killed = false;
    const kill = () => {
      killed = true;
      spawnSync(program, [...prefix, 'kill', name], { timeout: 20_000, windowsHide: true });
      child.kill();
    };
    child.stdout.on('data', (d: Buffer) => {
      size += d.length;
      if (size > MODULE_MAX + 1024 * 1024) return kill();
      chunks.push(d);
    });
    child.stderr.on('data', (d: Buffer) => {
      if (stderr.length < 200_000) stderr += d.toString('utf8');
    });
    const timer = setTimeout(kill, limit);
    child.on('error', (err) => {
      clearTimeout(timer);
      failed(new Error(`${program} could not start: ${err.message}`));
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      done({ code, stdout: Buffer.concat(chunks), stderr, killed });
    });
  });
}

/** Stops on a failed step: a line of ✗ is the plugin's, the rest the machine's; both leave the review « not compiled ». */
function checked(what: string, r: Ran, limit: number): Ran {
  if (r.killed) throw new Error(`${what} : stopped after ${Math.round(limit / 1000)} s, or its output is too big`);
  if (r.code === OOM)
    throw new Error(`${what} : killed, most often out of memory: the container is given ${MEMORY}, its machine needs that much`);
  if (r.code !== 0) {
    if (r.stderr.includes('✗')) throw new Refused(said(r.stderr));
    throw new Error(`${what} stopped (${r.code}): ${said(r.stderr)}`);
  }
  return r;
}

/**
 * The review's compiler of a third party's `run.wasm`: one compilation in new containers, as the header says. The plugin's
 * files are written again as regular files: what the archive held, never a link.
 */
export function containerCompiler(s: ContainerSetup): CompileOne {
  const tools = s.tools ?? TOOLS_VOLUME;
  const fetchLimit = s.fetchLimit ?? FETCH_LIMIT;
  const buildLimit = s.buildLimit ?? BUILD_LIMIT;
  return async (input): Promise<Compiled> => {
    if (!('wasm' in input)) throw new Error('a run.js bundle does not compile in the container of the review');
    const id = randomBytes(6).toString('hex');
    const work = `escouade-review-${id}`;
    const named = (step: string) => `escouade-review-${step}-${id}`;
    const { engine } = s;
    mkdirSync(s.work, { recursive: true });
    const dir = mkdtempSync(join(s.work, 'compile-'));
    try {
      const plugin = join(dir, 'plugin');
      await materialise(input.files, plugin);
      const run = async (what: string, args: string[], limit: number, name: string) =>
        checked(what, await step(engine, args, limit, name), limit);
      await run('prepare', prepareArgs(named('prepare'), s.image, tools, work), fetchLimit, named('prepare'));
      await run('fetch', fetchArgs(named('fetch'), s, tools, plugin, input.firstParty), fetchLimit, named('fetch'));
      const built = await run('build', buildArgs(named('build'), s, tools, work, plugin, input.firstParty), buildLimit, named('build'));
      const read = await run('read', readArgs(named('read'), s, work), QUICK_LIMIT, named('read'));
      return checkedModule(read.stdout, built.stdout.toString('utf8'), rustCompilerOf);
    } finally {
      spawnSync(engine.program, [...(engine.prefix ?? []), 'volume', 'rm', '--force', work], { timeout: 60_000, windowsHide: true });
      rmSync(dir, { recursive: true, force: true });
    }
  };
}
