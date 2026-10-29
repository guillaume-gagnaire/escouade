import { spawn, spawnSync } from 'node:child_process';
import { chmodSync, cpSync, existsSync, mkdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { inside } from './extract';

// The conformance bench (PLG-3, `escouade-plugin test`) played on the plugin in a container without network (docs/STORE.md
// §4.2, step 4): read-only, `/tmp` in memory, bounded in memory, processes and time, no capability, an unprivileged user,
// the plugin's folder and the Linux `escouade-plugin` mounted read-only. It is the only place the plugin's code runs
// during a review. Without a container engine, or without a Linux `escouade-plugin` to put in it, the bench is not
// played and the report says so first.

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
      /** What came out when the bench did not answer its JSON (a crash, the time out). */
      raw?: string;
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

/** The arguments of `<engine> run`, every limit of the sandbox in them. */
export function runArgs(folder: string, cli: string, image: string, name: string): string[] {
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
  ];
}

/**
 * The lines of what the plugin did that say the sandbox refused something: a connection, a write. `raw` is text of
 * several lines; a scenario's words hold the last lines of a program that ended, joined by ` | ` after a ` — `.
 */
export function refusals(seen: Seen[], raw = ''): string[] {
  const re =
    /ENETUNREACH|EAI_AGAIN|ENOTFOUND|ECONNREFUSED|getaddrinfo|Network is unreachable|network unreachable|EROFS|Read-only file system|EACCES|Permission denied|EPERM/i;
  const lines = [...seen.map((s) => s.line ?? s.message ?? ''), ...raw.split(/\n| \| | — /)];
  return [...new Set(lines.map((l) => l.trim()).filter((l) => l && re.test(l)))];
}

/** Plays the bench on `folder`; the result says why when it was not played. */
export async function playBench(setup: BenchSetup, folder: string, why?: string): Promise<BenchResult> {
  if (!setup.engine) return { played: false, why: why ?? 'aucun moteur de conteneur' };
  if (!setup.cli)
    return {
      played: false,
      why: 'pas d’`escouade-plugin` pour Linux à mettre dans le conteneur (ESCOUADE_PLUGIN_LINUX : le binaire Linux x64, de l’AppImage ou de `cargo build --bin escouade-plugin` sous Linux)',
    };
  const name = `escouade-review-${randomBytes(6).toString('hex')}`;
  const args = runArgs(folder, setup.cli, setup.image, name);
  const { program, prefix = [] } = setup.engine;
  const command = [program, ...args].join(' ');
  const out = await new Promise<{ code: number | null; stdout: string; stderr: string; timedOut: boolean }>((done) => {
    const child = spawn(program, [...prefix, ...args], { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    child.stdout.on('data', (d) => (stdout = (stdout + d).slice(-4_000_000)));
    child.stderr.on('data', (d) => (stderr = (stderr + d).slice(-200_000)));
    const timer = setTimeout(() => {
      timedOut = true;
      spawnSync(program, [...prefix, 'kill', name], { timeout: 20_000 });
      child.kill();
    }, setup.timeout);
    child.on('error', (e) => {
      clearTimeout(timer);
      done({ code: null, stdout, stderr: `${stderr}${e.message}`, timedOut });
    });
    child.on('close', (code) => {
      clearTimeout(timer);
      done({ code, stdout, stderr, timedOut });
    });
  });
  const raw = `${out.stdout}\n${out.stderr}`.trim();
  type Answer = { outcomes?: BenchOutcome[]; seen?: Seen[] } | null;
  let report: Answer;
  try {
    report = JSON.parse(out.stdout) as Answer;
  } catch {
    report = null;
  }
  if (!report?.outcomes) {
    const said = out.timedOut
      ? `arrêté au bout de ${Math.round(setup.timeout / 1000)} s`
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
  };
}
