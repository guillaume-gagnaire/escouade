import { spawn, spawnSync } from 'node:child_process';
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { inside } from './extract';
import { firstPartyRepo, type Env } from '../../server/utils/config';
import { RESERVED_PREFIX } from '../../../schemas/plugin-manifest.mjs';

// The conformance bench (PLG-3, `escouade-plugin test`) played on the plugin in a container without network (docs/STORE.md
// §4.2, step 4): read-only, `/tmp` in memory, bounded in memory, processes and time, no capability, an unprivileged user,
// the plugin's folder and the Linux `escouade-plugin` mounted read-only. It is the only place the plugin's code runs
// during a review. Without a container engine, or without a Linux `escouade-plugin` to put in it, the bench is not
// played and the report says so first.
//
// A `run.js` plugin (ESC-324) is played without a container: the module the store published, by this machine's
// `escouade-plugin test --wasm <module> --offline`, in the app's WASM host, which gives it no file but its folder's
// read, no process, no environment and, offline, no network (every `net/fetch` refused). Its code never runs as a
// process here.

export interface Engine {
  /** `docker`, `podman`, or the program the tests give. */
  program: string;
  /** Arguments put before `run`… (a script for `node`, in the tests). */
  prefix?: string[];
}

export interface BenchSetup {
  engine: Engine | null;
  /** A Linux `escouade-plugin` (x64), mounted in the container. */
  cli: string | null;
  image: string;
  /** How long the bench may take, in ms. */
  timeout: number;
  /** This machine's `escouade-plugin`: it plays a `run.js` plugin's module in WASM and compiles its bundle again. */
  local?: string | null;
}

export interface Seen {
  kind: string;
  line?: string;
  message?: string;
  method?: string;
  key?: string;
}

export interface BenchOutcome {
  scenario: string;
  verdict: 'pass' | 'fail' | 'skip';
  said: string;
  missing?: string;
}

export type BenchResult =
  | { played: false; why: string }
  | {
      played: true;
      passed: boolean;
      outcomes: BenchOutcome[];
      /** What the plugin did on its own; the refusals of the sandbox among them. */
      seen: Seen[];
      refused: string[];
      /** The command, as it ran: the report shows its limits. */
      command: string;
      /** Where it ran: a container without network, or the app's WASM host, offline (a `run.js` plugin). */
      where?: 'container' | 'wasm';
      /** What came out when the bench did not answer its JSON (a crash, the time out). */
      raw?: string;
      /** The version of Escouade it was played as, as the bench says it. */
      escouade?: string;
      /** It was played as the version under development, by `--escouade`: a first-party plugin, never a third party's. */
      development?: boolean;
    };

// Named in full (podman refuses a short name where no registry is set for it); Debian 13, whose glibc (2.41) runs the
// `escouade-plugin` built on Ubuntu 24.04 (2.39), which bookworm's (2.36) does not.
export const DEFAULT_IMAGE = 'docker.io/library/node:22-trixie-slim';
export const DEFAULT_TIMEOUT = 5 * 60_000;

/** A container engine that answers: `ESCOUADE_REVIEW_ENGINE` (`none` to play no bench), else docker, else podman. */
export function findEngine(env = process.env): { engine: Engine | null; why?: string } {
  const asked = env.ESCOUADE_REVIEW_ENGINE;
  if (asked === 'none') return { engine: null, why: 'ESCOUADE_REVIEW_ENGINE=none' };
  const tried: string[] = [];
  for (const program of asked ? [asked] : ['docker', 'podman']) {
    const r = spawnSync(program, ['version', '--format', '{{.Server.Version}}'], { encoding: 'utf8', timeout: 20_000 });
    if (r.status === 0) return { engine: { program } };
    tried.push(`${program} : ${r.error ? 'introuvable' : (r.stderr || r.stdout || `code ${r.status}`).trim().split('\n')[0]}`);
  }
  return { engine: null, why: `aucun moteur de conteneur (${tried.join(' ; ')})` };
}

/** The Linux `escouade-plugin`: `ESCOUADE_PLUGIN_LINUX`, else the one on the PATH under Linux x64. */
export function findCli(env = process.env, platform = process.platform, arch = process.arch): string | null {
  if (env.ESCOUADE_PLUGIN_LINUX) return existsSync(env.ESCOUADE_PLUGIN_LINUX) ? env.ESCOUADE_PLUGIN_LINUX : null;
  if (platform !== 'linux' || arch !== 'x64') return null;
  const r = spawnSync('sh', ['-c', 'command -v escouade-plugin'], { encoding: 'utf8' });
  return r.status === 0 && r.stdout.trim() ? r.stdout.trim() : null;
}

/**
 * This machine's `escouade-plugin`: `ESCOUADE_PLUGIN` (a `.mjs` runs with node, for the tests), else the one on the
 * PATH, else the one built in the repository (`src-tauri/target/release`, then `debug`).
 */
export function findLocalCli(env = process.env, repo?: string, platform = process.platform): string | null {
  if (env.ESCOUADE_PLUGIN) return existsSync(env.ESCOUADE_PLUGIN) ? env.ESCOUADE_PLUGIN : null;
  const exe = platform === 'win32' ? 'escouade-plugin.exe' : 'escouade-plugin';
  const r =
    platform === 'win32'
      ? spawnSync('where', ['escouade-plugin'], { encoding: 'utf8', windowsHide: true })
      : spawnSync('sh', ['-c', 'command -v escouade-plugin'], { encoding: 'utf8' });
  const onPath = r.status === 0 ? ((r.stdout ?? '').split(/\r?\n/)[0] ?? '').trim() : '';
  if (onPath) return onPath;
  for (const profile of ['release', 'debug']) {
    const at = repo ? join(repo, 'src-tauri', 'target', profile, exe) : '';
    if (at && existsSync(at)) return at;
  }
  return null;
}

/** How the local `escouade-plugin` is run: a `.mjs` with this node (the tests' fake). */
export const localCommand = (cli: string): string[] => (/\.m?js$/.test(cli) ? [process.execPath, cli] : [cli]);

/**
 * Makes the folder the bench plays: the plugin's files, and its Linux program from the release where its manifest
 * names it (`run.linux`, else `run.command`), so that what runs is what was read and fingerprinted.
 */
export function benchFolder(src: string, dest: string, manifest: Record<string, unknown>, linuxBinary: Buffer | null): string {
  cpSync(src, dest, { recursive: true, verbatimSymlinks: true });
  const run = (manifest.run ?? null) as { command?: string; linux?: string } | null;
  const program = run?.linux ?? run?.command;
  if (program && inside(dest, program)) {
    const at = join(dest, program);
    if (linuxBinary) {
      mkdirSync(dirname(at), { recursive: true });
      writeFileSync(at, linuxBinary);
    }
    if (existsSync(at) && statSync(at).isFile()) chmodSync(at, 0o755);
  }
  return dest;
}

/**
 * Whether the bench plays the plugin with `--first-party` (ESC-314): an id `dev.escouade.*` from the app's repository
 * (`plugins/<id>/`), which the store already let through by its GitHub id. It takes the reserved ids, never the app's
 * programs (`{escouade}/…`); any other plugin is played as a third party's.
 */
export function firstPartyBench(id: string, repository: string, env: Env = process.env): boolean {
  const repo = repository
    .replace(/^https:\/\/github\.com\//i, '')
    .replace(/\.git$/i, '')
    .toLowerCase();
  return id.startsWith(RESERVED_PREFIX) && repo === firstPartyRepo(env).toLowerCase();
}

/**
 * The version of Escouade under development (ESC-343): `NEXT` of the site, the version the roadmap announces, whose test
 * fails once it is out. The first-party plugins need it before it exists: the chief publishes and signs them, the
 * release comes after (docs/PLUGINS.md §8.7).
 */
export function developmentVersion(site: string): string | undefined {
  const next = /^export const NEXT = '(\d+\.\d+)(\.\d+)?';/m.exec(site);
  return next ? `${next[1]}${next[2] ?? '.0'}` : undefined;
}

/** `developmentVersion` of this repository; none when the site's file is not there. */
export function readDevelopmentVersion(repo: string): string | undefined {
  try {
    return developmentVersion(readFileSync(join(repo, 'website', 'app', 'data', 'site.ts'), 'utf8'));
  } catch {
    return undefined;
  }
}

/** The version the bench is asked to play: the one under development for a first-party plugin; a third party's, none. */
export const playedAs = (firstParty: boolean, development?: string) => (firstParty ? development : undefined);

/** What a bench is told of the plugin's hand; `--escouade` never goes without `--first-party`. */
function handArgs(firstParty: boolean, development?: string): string[] {
  const as = playedAs(firstParty, development);
  return [...(firstParty ? ['--first-party'] : []), ...(as ? ['--escouade', as] : [])];
}

/** The arguments of `<engine> run`, every limit of the sandbox in them. */
export function runArgs(folder: string, cli: string, image: string, name: string, firstParty = false, development?: string): string[] {
  return [
    'run',
    '--rm',
    '--name',
    name,
    '--network=none',
    '--read-only',
    '--tmpfs',
    '/tmp:rw,size=64m',
    '--memory=512m',
    '--memory-swap=512m',
    '--pids-limit=256',
    '--cpus=1',
    '--cap-drop=ALL',
    '--security-opt=no-new-privileges',
    '--user=65534:65534',
    '--env=HOME=/tmp',
    '--volume',
    `${folder}:/plugin:ro`,
    '--volume',
    `${cli}:/usr/local/bin/escouade-plugin:ro`,
    image,
    'escouade-plugin',
    'test',
    '/plugin',
    '--json',
    '--lang',
    'fr',
    ...handArgs(firstParty, development),
  ];
}

/**
 * The lines of what the plugin did that say the sandbox refused something: a connection, a write. `raw` is text of
 * several lines; a scenario's words hold the last lines of a program that ended, joined by ` | ` after a ` — `.
 */
export function refusals(seen: Seen[], raw = ''): string[] {
  const re =
    /is not in permissions\.network|ENETUNREACH|EAI_AGAIN|ENOTFOUND|ECONNREFUSED|getaddrinfo|Network is unreachable|network unreachable|EROFS|Read-only file system|EACCES|Permission denied|EPERM/i;
  const lines = [...seen.map((s) => s.line ?? s.message ?? ''), ...raw.split(/\n| \| | — /)];
  return [...new Set(lines.map((l) => l.trim()).filter((l) => l && re.test(l)))];
}

/** The arguments of the local `escouade-plugin` that plays a `run.js` plugin's module, offline. */
export function wasmArgs(folder: string, module: string, firstParty = false, development?: string): string[] {
  return ['test', folder, '--wasm', module, '--offline', '--json', '--lang', 'fr', ...handArgs(firstParty, development)];
}

/**
 * Plays the bench on the module `module` of a `run.js` plugin in `folder`, by this machine's `escouade-plugin`, in the
 * app's WASM host, offline, without a container; in an environment of its own (none of the review's variables, the
 * administration token among them).
 */
export async function playWasm(
  setup: BenchSetup,
  folder: string,
  module: string,
  firstParty = false,
  development?: string,
): Promise<BenchResult> {
  if (!setup.local)
    return {
      played: false,
      why: 'pas d’`escouade-plugin` sur ce poste pour jouer le module WASM (ESCOUADE_PLUGIN, le PATH ou src-tauri/target)',
    };
  const [program, ...before] = localCommand(setup.local);
  const args = wasmArgs(folder, module, firstParty, development);
  const home = mkdtempSync(join(tmpdir(), 'escouade-review-bench-'));
  const env: NodeJS.ProcessEnv = { PATH: process.env.PATH, HOME: home, TMPDIR: home, LANG: 'C.UTF-8' };
  if (process.platform === 'win32') {
    for (const k of ['SystemRoot', 'SYSTEMROOT', 'windir', 'PATHEXT', 'COMSPEC']) if (process.env[k]) env[k] = process.env[k];
    env.TEMP = env.TMP = env.USERPROFILE = env.LOCALAPPDATA = env.APPDATA = home;
  }
  try {
    const out = await spawnBench(program!, [...before, ...args], setup.timeout, env);
    return withWhere(readBench(out, `escouade-plugin ${args.join(' ')}`, setup.timeout), 'wasm', playedAs(firstParty, development));
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
}

/** Plays the bench on `folder` in a container; the result says why when it was not played. */
export async function playBench(
  setup: BenchSetup,
  folder: string,
  why?: string,
  firstParty = false,
  development?: string,
): Promise<BenchResult> {
  if (!setup.engine) return { played: false, why: why ?? 'aucun moteur de conteneur' };
  if (!setup.cli)
    return {
      played: false,
      why: 'pas d’`escouade-plugin` pour Linux à mettre dans le conteneur (ESCOUADE_PLUGIN_LINUX : le binaire Linux x64, de l’AppImage ou de `cargo build --bin escouade-plugin` sous Linux)',
    };
  const name = `escouade-review-${randomBytes(6).toString('hex')}`;
  const args = runArgs(folder, setup.cli, setup.image, name, firstParty, development);
  const { program, prefix = [] } = setup.engine;
  const command = [program, ...args].join(' ');
  const out = await spawnBench(program, [...prefix, ...args], setup.timeout, undefined, () => {
    spawnSync(program, [...prefix, 'kill', name], { timeout: 20_000 });
  });
  return withWhere(readBench(out, command, setup.timeout), 'container', playedAs(firstParty, development));
}

/** Where it ran, and the version it was asked to play: the bench's own word, else this one. */
const withWhere = (r: BenchResult, where: 'container' | 'wasm', as?: string): BenchResult =>
  r.played ? { ...r, where, ...(as ? { development: true, escouade: r.escouade ?? as } : {}) } : r;

interface Ran {
  code: number | null;
  stdout: string;
  stderr: string;
  timedOut: boolean;
}

/** Runs the bench's program, its outputs bounded, stopped past `timeout` (`onTimeout` first: the container's kill). */
function spawnBench(program: string, args: string[], timeout: number, env?: NodeJS.ProcessEnv, onTimeout?: () => void): Promise<Ran> {
  return new Promise<Ran>((done) => {
    const child = spawn(program, args, { stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true, ...(env ? { env } : {}) });
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    child.stdout.on('data', (d) => (stdout = (stdout + d).slice(-4_000_000)));
    child.stderr.on('data', (d) => (stderr = (stderr + d).slice(-200_000)));
    const timer = setTimeout(() => {
      timedOut = true;
      onTimeout?.();
      child.kill();
    }, timeout);
    child.on('error', (e) => {
      clearTimeout(timer);
      done({ code: null, stdout, stderr: `${stderr}${e.message}`, timedOut });
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      done({ code, stdout, stderr, timedOut });
    });
  });
}

/** What the bench answered: its report, or what came out when it gave none. */
function readBench(out: Ran, command: string, timeout: number): BenchResult {
  const raw = `${out.stdout}\n${out.stderr}`.trim();
  type Answer = { outcomes?: BenchOutcome[]; seen?: Seen[]; escouade?: string } | null;
  let report: Answer;
  try {
    report = JSON.parse(out.stdout) as Answer;
  } catch {
    report = null;
  }
  if (!report?.outcomes) {
    const said = out.timedOut
      ? `arrêté au bout de ${Math.round(timeout / 1000)} s`
      : `le banc n’a pas rendu son rapport (code ${out.code})`;
    return {
      played: true,
      passed: false,
      outcomes: [],
      seen: [],
      refused: refusals([], raw),
      command,
      raw: `${said}\n${raw}`.slice(0, 20_000),
    };
  }
  const seen = report.seen ?? [];
  return {
    played: true,
    passed: report.outcomes.every((o) => o.verdict !== 'fail'),
    outcomes: report.outcomes,
    seen,
    refused: refusals(seen, [out.stderr, ...report.outcomes.map((o) => o.said)].join('\n')),
    command,
    ...(report.escouade ? { escouade: report.escouade } : {}),
  };
}
