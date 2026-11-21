// Launches the real app on a throwaway data folder, with the fake `claude` CLI, and connects
// Playwright to its WebView2 through the DevTools protocol.

import { test as base, chromium, expect as baseExpect, type Browser, type Locator, type Page } from '@playwright/test';
import { execFile, execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { freePort, waitForPortToClose } from './port';
import { endTree, listenersOn } from './proctree';
import { serverProgram, startBareServer, startServer, type BareServer, type Server } from './server';
import { lookDiff } from './srgb';
import { updateTrouble } from './update-guard';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(ROOT, 'src-tauri', 'target', 'debug', 'escouade.exe');
/**
 * The program of `second()`: a copy of the app under another name, beside it (made by `secondProgram()`). On an elevated CI
 * runner, WebView2 takes its arguments from the machine policy, by the program's name only (ci.yml): under the app's own
 * name, the second app would get the first one's DevTools port, which it cannot open (ESC-496). Under its own, it has a
 * line of its own there.
 */
const SECOND_EXE = path.join(path.dirname(EXE), `escouade-second${path.extname(EXE)}`);
/**
 * `ESCOUADE_E2E_POLICY_LIKE=1`: the WebView2 of each launch as an elevated runner starts it (ESC-496), without elevation.
 * There, WebView2 ignores the `WEBVIEW2_*` variables: its folder is the one the app gives by its API
 * (`ESCOUADE_WEBVIEW_DIR`, as always), and its arguments are the machine policy's line for the program's name, whoever
 * launches it (`POLICY_PORTS`, the same as ci.yml's), not the ones `second()` would choose for it.
 */
const POLICY_LIKE = !!process.env.ESCOUADE_E2E_POLICY_LIKE && process.env.ESCOUADE_E2E_POLICY_LIKE !== '0';
const FAKE = path.join(ROOT, 'tests', 'fixtures', 'fake-claude.cmd');
/** The fake `codex app-server` (tests/fixtures/fake-codex.mjs), for a test that runs agents on Codex. */
const FAKE_CODEX = path.join(ROOT, 'tests', 'fixtures', 'fake-codex.cmd');
/** The fake `opencode serve` (tests/fixtures/fake-opencode.mjs), for a test that runs agents on OpenCode. */
const FAKE_OPENCODE = path.join(ROOT, 'tests', 'fixtures', 'fake-opencode.cmd');
/** The fake `copilot --server --stdio` (tests/fixtures/fake-copilot.mjs), for a test that runs agents on Copilot CLI. */
const FAKE_COPILOT = path.join(ROOT, 'tests', 'fixtures', 'fake-copilot.cmd');
/** The fake `agent acp` of Cursor CLI (tests/fixtures/fake-cursor.mjs), for a test that runs agents on Cursor CLI. */
const FAKE_CURSOR = path.join(ROOT, 'tests', 'fixtures', 'fake-cursor.cmd');
/** The fake `vibe-acp` of Mistral Vibe (tests/fixtures/fake-vibe.mjs), for a test that runs agents on Mistral Vibe. */
const FAKE_VIBE = path.join(ROOT, 'tests', 'fixtures', 'fake-vibe.cmd');
/** The fake `agy` of Antigravity CLI (tests/fixtures/fake-agy.mjs), for a test that runs agents on Antigravity CLI. */
const FAKE_AGY = path.join(ROOT, 'tests', 'fixtures', 'fake-agy.cmd');
/** The fake `gemini` of Gemini CLI (tests/fixtures/fake-gemini.mjs), for a test that runs agents on Gemini CLI. */
const FAKE_GEMINI = path.join(ROOT, 'tests', 'fixtures', 'fake-gemini.cmd');

export interface App {
  /** The window of the app: a new one after each `restart()` (read `app.page` again, do not keep the old one). */
  page: Page;
  /**
   * Stops the app and starts it again on the same data folder, with the same environment: what the app finds again when it is
   * started anew (the projects, the agents and their conversations, as it saved them). Gives the new window, also in `app.page`.
   */
  restart: () => Promise<Page>;
  /**
   * `restart()` on another data folder (ESC-190: a second user of the same repository, with settings of their own): from then
   * on, every start is on it. Its `settings.json` is the test's to write first.
   */
  restartAs: (data: string) => Promise<Page>;
  /**
   * A second app at the same time as the first, on another data folder (TS-6: two clients of a team, live): its window. It
   * runs from a copy of the program (`escouade-second.exe`), with a WebView2 folder of its own (`webview-2` of the test's);
   * its DevTools port is `ESCOUADE_E2E_SECOND_CDP_PORT`, else the first one's plus one (on CI, the machine policy's line for
   * that copy, ci.yml); it is killed with the test (ESC-496).
   */
  second: (data: string) => Promise<Page>;
  /** When the launch the test is on was started (ms since the epoch, just before its process): the startup bench's origin (INT-0). */
  launchedAt: () => number;
  /** Why the launch the test is on ended (`exit code 0 (0x0)` for an app that quit by itself), null while it runs. */
  stopped: () => string | null;
  /** Attaches Playwright again to the running app's window (after its page's process crashed): gives it, also in `app.page`. */
  reconnect: () => Promise<Page>;
  /** A git repository with one commit, ready to be added as a project. */
  repo: string;
  data: string;
  /** The test's folder: the app's data (`data`), the repository and Principal's Claude Code folder (`claude`) are in it. */
  root: string;
  /** Principal's `.claude.json`: where the fake `claude mcp add` declares Escouade's server (the app's `CLAUDE_CONFIG_DIR` is a test folder). */
  claudeJson: string;
  /** Principal's `config.toml` of Codex: where « Les agents peuvent piloter Escouade » declares the server (`CODEX_HOME` is a test folder). */
  codexToml: string;
  /** OpenCode's global config, where the switch declares the server (`XDG_CONFIG_HOME` is a test folder). */
  opencodeJson: string;
  /** Principal's `mcp-config.json` of Copilot CLI (`COPILOT_HOME` is a test folder). */
  copilotJson: string;
  /** Launches of the fake CLI: argv, cwd, its process id (a new session of it is `sess-<pid>`), the proxy, the Claude Code folder (`CLAUDE_CONFIG_DIR`) and the task list variable it received (null when unset). */
  launches: () => {
    argv: string[];
    cwd: string;
    pid: number;
    proxy: string | null;
    configDir: string | null;
    todoTools: string | null;
    /** `DISABLE_AUTOUPDATER`: its own update off (ESC-133). */
    autoUpdater: string | null;
    /** `CLAUDE_CODE_ARTIFACT`: its artifacts on in stream-json (ESC-400). */
    artifact: string | null;
  }[];
  /**
   * With `test.use({ server: true })` (ESC-514): the `escouade-server` the window speaks to, through a local forward to its
   * API, its device paired by `escouade-server pair`; its data folder is `server-data` of the test's, with the same settings
   * as the app's (the fake `claude`). Stopped with the test; a `restart()` restarts the app, not the server.
   */
  server: Server | null;
  /**
   * With `test.use({ sshServer: true })` (ESC-537): an `escouade-server` not paired yet, reached by « Se connecter à un
   * serveur » through the fake `ssh` (tests/fixtures/fake-ssh.mjs, given to the app by ESCOUADE_SSH), the hosts of a
   * test `~/.ssh/config` (ESCOUADE_SSH_CONFIG): `travail` (the server), `labo-inconnu` (an unknown host key). `dir`: the
   * fake's folder (`down`, `cut`).
   *
   * What changes machines (ESC-538), of the test's: the server's home folder for « Un dossier du serveur » (the test's
   * root, ESCOUADE_BROWSE_HOME), this computer's downloads (`<root>/downloads`, ESCOUADE_DOWNLOADS_DIR), what this
   * computer would open (`<root>/opened.log`, ESCOUADE_OPEN_LOG) and the notifications it would show
   * (`<root>/alerts.log`, ESCOUADE_ALERT_LOG): nothing opens nor shows for real.
   */
  ssh: {
    server: BareServer;
    program: string;
    dir: string;
    env: NodeJS.ProcessEnv;
    /**
     * With `test.use({ sshServer: 2 })` (ESC-539, two spaces): a second `escouade-server`, host `perso` of the test's
     * ~/.ssh/config, its own data folder; its port on the server is 46481 (the dialog's « Port d'Escouade sur le
     * serveur »). Its fake ssh plays `down-perso` and `cut-perso` alone.
     */
    second: { server: BareServer; env: NodeJS.ProcessEnv } | null;
  } | null;
}

function git(cwd: string, ...args: string[]) {
  execFileSync('git', args, { cwd, stdio: 'pipe' });
}

function makeRepo(root: string): string {
  const repo = path.join(root, 'demo-api');
  fs.mkdirSync(path.join(repo, 'src'), { recursive: true });
  git(repo, 'init', '-q', '-b', 'main');
  git(repo, 'config', 'user.email', 'e2e@test');
  git(repo, 'config', 'user.name', 'e2e');
  git(repo, 'config', 'core.autocrlf', 'false');
  fs.writeFileSync(path.join(repo, 'src', 'app.ts'), 'const a = 1;\n');
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', 'init');
  return repo;
}

/**
 * `ESCOUADE_E2E_ROOT`: the test's folder at a path that shows nothing of the machine (the site's screenshots,
 * tests/site-shots), emptied at the start and kept at the end, to be looked at. Emptied only when it holds the mark the tests
 * left there: never a folder of the user's.
 */
const FIXED_ROOT = process.env.ESCOUADE_E2E_ROOT;
const ROOT_MARK = '.escouade-e2e-root';

/** The test's folder: a new temporary one, or `FIXED_ROOT` emptied but its mark. */
function testRoot(): string {
  if (!FIXED_ROOT) return fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-'));
  const fixed = FIXED_ROOT;
  if (fs.existsSync(fixed) && fs.readdirSync(fixed).length > 0) {
    if (!fs.existsSync(path.join(fixed, ROOT_MARK)))
      throw new Error(`ESCOUADE_E2E_ROOT: ${fixed} is not empty and was not made by the tests: not touched`);
    for (const name of fs.readdirSync(fixed))
      if (name !== ROOT_MARK) fs.rmSync(path.join(fixed, name), { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
  }
  fs.mkdirSync(fixed, { recursive: true });
  fs.writeFileSync(path.join(fixed, ROOT_MARK), '');
  return fixed;
}

/** Last lines of a log file, for failure messages (CI annotations are all we get there). */
function tail(file: string, lines = 40): string {
  if (!fs.existsSync(file)) return `(no ${path.basename(file)})`;
  const text = fs.readFileSync(file, 'utf8').trimEnd();
  return text ? text.split('\n').slice(-lines).join('\n') : `(${path.basename(file)} is empty)`;
}

/** A launch of the app: its process, why it ended (null while it runs) and, once attached, its WebView2. */
interface Instance {
  child: ChildProcess;
  /** When it was started (ms since the epoch). */
  at: number;
  stopped: string | null;
  /** When it ended, as Node saw it (null while it runs): its pid may be another process's after. */
  exitedAt: number | null;
  browser: Browser | null;
}

/** The sign-in of Cursor CLI the app finds (`ESCOUADE_CURSOR_AUTH`): a file of the test's, with nothing in it. */
/** The file that stands for Antigravity's Google sign-in on the machine, made unless `signedOut`. */
function agySignIn(root: string, signedOut: boolean): string {
  const file = path.join(root, 'agy-signed-in');
  if (!signedOut && !fs.existsSync(file)) fs.writeFileSync(file, 'signed in');
  return file;
}

function cursorAuth(root: string): string {
  const file = path.join(root, 'cursor-auth.json');
  if (!fs.existsSync(file)) fs.writeFileSync(file, '{}');
  return file;
}

/**
 * Starts the app with `env` and `args` (a link `escouade://`, as the system gives it); its stderr is appended to `stderr` (a
 * restart keeps what the launch before wrote).
 */
function startApp(env: NodeJS.ProcessEnv, stderr: string, args: string[] = [], program = EXE): Instance {
  const errFd = fs.openSync(stderr, 'a');
  const at = Date.now();
  const child = spawn(program, args, { env, stdio: ['ignore', 'ignore', errFd] });
  fs.closeSync(errFd);
  const instance: Instance = { child, at, stopped: null, exitedAt: null, browser: null };
  child.on('exit', (code, signal) => {
    instance.exitedAt = Date.now();
    instance.stopped = signal ? `killed by ${signal}` : `exit code ${code} (0x${((code ?? 0) >>> 0).toString(16)})`;
  });
  child.on('error', (e) => (instance.stopped = `could not start: ${e.message}`));
  return instance;
}

/**
 * Ends the app and its whole process tree (the fake CLIs, the terminals, the WebView2), and waits until none of them is left:
 * the next test must find nothing of this one (its DevTools port above all, ESC-522). By their pids, read from the list of
 * the machine's processes: the app while it runs, what it started and theirs, orphans included, and the WebView2 processes
 * on its own folder `webview` (`<webview>\EBWebView` in their command line: one whose app is gone; `webview` is no prefix
 * of the second app's `webview-2` there). Never by a program's name: `escouade.exe` is also the user's own app.
 * (`taskkill /T` alone returned before the tree was gone, and missed the orphans.)
 */
async function killApp(instance: Instance, webview: string) {
  await instance.browser?.close().catch(() => {});
  instance.browser = null;
  const pid = instance.child.pid;
  if (!pid) return;
  const folder = path.join(webview, 'EBWebView').toLowerCase();
  const { left, said } = await endTree(
    () => ({ pid, at: instance.at, exitedAt: instance.exitedAt }),
    (p) => p.cmd.toLowerCase().includes(folder),
    // A WebView2 killed on a loaded machine may take more than 30 s to be gone (ESC-522): the next app does not start
    // meanwhile, its DevTools port would not open.
    120_000,
  );
  if (left.length) {
    console.warn(
      `processes of the app still there 120 s after it was ended:\n${left.map((p) => `  ${p.pid} ${p.cmd.slice(0, 160)}`).join('\n')}\n  taskkill: ${said}`,
    );
  }
}

/** The DevTools port CI gives (`ESCOUADE_E2E_CDP_PORT`, the machine policy's), else 0: one drawn free by each test. */
const FIXED_PORT = Number(process.env.ESCOUADE_E2E_CDP_PORT) || 0;

/** WebView2's « the requested resource is in use » as an app that cannot have its WebView2 folder logs it (0x800700AA). */
const FOLDER_IN_USE = '0x800700AA';

function sizeOf(file: string): number {
  return fs.existsSync(file) ? fs.statSync(file).size : 0;
}

/** What `file` got after its first `from` bytes. */
function since(file: string, from: number): string {
  if (!fs.existsSync(file)) return '';
  const buf = fs.readFileSync(file);
  return buf.subarray(Math.min(from, buf.length)).toString('utf8');
}

/**
 * How many WebView2 processes have `folder` (an app's WebView2 folder) in their command line: its `EBWebView`, WebView2's
 * own, so that `webview` is not taken for a prefix of `webview-2`.
 */
async function webviewProcessesOn(folder: string): Promise<number> {
  const needle = path.join(folder, 'EBWebView').toLowerCase().replace(/'/g, "''");
  const script =
    `@(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | ` +
    `Where-Object { $_.CommandLine -and $_.CommandLine.ToLower().Contains('${needle}') }).Count`;
  return new Promise((resolve) =>
    execFile('powershell', ['-NoProfile', '-Command', script], { encoding: 'utf8', timeout: 20_000 }, (err, out) =>
      resolve(err ? 0 : Number(out.trim()) || 0),
    ),
  );
}

/**
 * Waits until no WebView2 process of a killed app holds its folder any more (`taskkill /T` returns before they are all
 * gone; the next start on the folder would stop at 0x800700AA): the number still there after `timeoutMs`, 0 when none is.
 */
async function waitForWebviewGone(folder: string, timeoutMs = 20_000): Promise<number> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const left = await webviewProcessesOn(folder);
    if (left === 0 || Date.now() >= deadline) return left;
    await new Promise((r) => setTimeout(r, 250));
  }
}

/**
 * The app's page once it is there: a start shows a blank page first (`about:blank`), then the app's, seconds later on
 * a slow runner; a page attached in between has no `__TAURI_INTERNALS__` yet, and may be navigated under a call.
 */
async function appReady(page: Page): Promise<Page> {
  for (let tries = 1; ; tries++) {
    try {
      await page.waitForFunction(() => '__TAURI_INTERNALS__' in window && document.readyState === 'complete', null, { timeout: 60_000 });
      if (REMOTE && !(await page.evaluate(() => '__ESCOUADE_REMOTE__' in window))) {
        throw new Error('ESCOUADE_E2E_REMOTE: the window is on Tauri’s IPC, not on the remote API (see app.log)');
      }
      return page;
    } catch (e) {
      if (tries === 3 || !String(e).includes('Execution context was destroyed')) throw e;
      await page.waitForLoadState('load').catch(() => {});
    }
  }
}

async function connect(port: number, stopped: () => string | null): Promise<{ browser: Browser; page: Page }> {
  let lastError = '';
  let pageless = 0;
  for (let i = 0; i < 120; i++) {
    const status = stopped();
    if (status) throw new Error(`the app stopped before exposing its WebView2 DevTools endpoint (${status})`);
    try {
      const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
      const page = browser.contexts()[0]?.pages()[0];
      if (page) return { browser, page };
      pageless++;
      await browser.close();
    } catch (e) {
      lastError = String((e as Error).message ?? e).split('\n')[0];
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  const targets = await fetch(`http://127.0.0.1:${port}/json/list`)
    .then((r) => r.text())
    .catch((e) => `unreachable: ${e.message}`);
  throw new Error(
    `the app did not expose its WebView2 DevTools endpoint on port ${port} ` +
      `(connected without a page ${pageless} times; last error: ${lastError || 'none'}; /json/list: ${targets.slice(0, 300)})`,
  );
}

/** The pages a DevTools port lists (`/json/list`): their id and address, or why it does not answer. */
async function targetsOn(port: number): Promise<string> {
  return fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(5000) })
    .then((r) => r.json())
    .then((list: { id: string; type: string; url: string }[]) => {
      const pages = list.filter((t) => t.type === 'page');
      return `${pages.length} page(s)${pages.map((t) => `\n    ${t.id.slice(0, 8)} ${t.url}`).join('')}`;
    })
    .catch((e) => `unreachable (${(e as { cause?: { code?: string } }).cause?.code ?? (e as Error).message})`);
}

/**
 * How the WebView2 of the test's apps were started, when an app's DevTools endpoint cannot be reached or gives another
 * app's page (CI diagnostics): each app's WebView2 folder, each browser process of the test with its folder and port, the
 * pages each port lists. Two apps on one browser (one folder), or a port opened by none, show there (ESC-496).
 */
async function webviewDiagnostics(root: string, ports: number[]): Promise<string> {
  const out: string[] = [];
  for (const name of ['webview', 'webview-2']) {
    const dir = path.join(root, name);
    if (name === 'webview' || fs.existsSync(dir)) {
      out.push(`${name} folder: ${fs.existsSync(dir) ? fs.readdirSync(dir).join(', ') || '(empty)' : '(missing)'}`);
    }
  }
  try {
    const ours = execFileSync(
      'powershell',
      ['-NoProfile', '-Command', 'Get-CimInstance Win32_Process -Filter "Name=\'msedgewebview2.exe\'" | ForEach-Object { $_.CommandLine }'],
      { encoding: 'utf8', timeout: 20_000 },
    )
      .split('\n')
      .filter((l) => l.includes(path.basename(root)));
    const browsers = ours.filter((l) => !l.includes('--type='));
    out.push(`WebView2 processes of this test: ${ours.length}, of which browsers: ${browsers.length}`);
    for (const browser of browsers) {
      const folder = browser.match(/--user-data-dir="?([^"]+?)"?(\s--|$)/)?.[1] ?? '(not passed)';
      out.push(`  browser on ${path.relative(root, folder) || folder}`);
      out.push(`    remote debugging: ${browser.match(/--remote-debugging-port=\d+/g)?.join(' ') ?? '(not passed)'}`);
      out.push(`    browser flags: ${[...new Set(browser.match(/--[a-z][a-z0-9-]*/g) ?? [])].join(' ')}`);
    }
  } catch (e) {
    out.push(`WebView2 processes: ${(e as Error).message.split('\n')[0]}`);
  }
  for (const port of ports) out.push(`port ${port}: ${await targetsOn(port)}`);
  const vars = Object.entries(process.env).filter(([k]) => k.startsWith('WEBVIEW2'));
  out.push(`WEBVIEW2_* in the test environment: ${vars.map(([k, v]) => `${k}=${v}`).join(' ') || '(none)'}`);
  if (POLICY_LIKE) out.push('ESCOUADE_E2E_POLICY_LIKE: arguments by the program’s name, as the machine policy of an elevated runner');
  return out.join('\n');
}

/**
 * The WebView2 of a launch of `program`, its DevTools on `port`, its data in `folder`: the folder by the app's API
 * (`ESCOUADE_WEBVIEW_DIR`, followed by an elevated process too), the arguments by `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`
 * (ignored there: ci.yml gives the same ones by the machine policy). `WEBVIEW2_USER_DATA_FOLDER` is never given: it would
 * override the app's folder here and not on CI, where it is ignored. With `POLICY_LIKE`, the arguments are the ones the
 * policy gives the program, whatever `port` the test asked for.
 */
function webviewEnv(program: string, port: number, folder: string, firstPort: number): NodeJS.ProcessEnv {
  const policyPort = POLICY_LIKE ? policyPortOf(program, firstPort) : port;
  return {
    ESCOUADE_WEBVIEW_DIR: folder,
    WEBVIEW2_USER_DATA_FOLDER: undefined,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:
      policyPort === null ? undefined : `--remote-debugging-port=${policyPort} ${RENDER_UNSEEN} ${FIXED_SCALE}`,
  };
}

/**
 * The DevTools port the machine policy of ci.yml opens for a program, by its name: the app's own (`escouade.exe`) on the
 * first port, its copy for `second()` (`escouade-second.exe`) on the next one; none for another name.
 */
function policyPortOf(program: string, firstPort: number): number | null {
  const name = path.basename(program).toLowerCase();
  if (name === path.basename(EXE).toLowerCase()) return firstPort;
  if (name === path.basename(SECOND_EXE).toLowerCase()) return firstPort + 1;
  return null;
}

/**
 * The copy of the app `second()` runs (`SECOND_EXE`), made again when the app was built since: the same size and time as
 * the app's program. A copy still running (an app of a test interrupted) cannot be replaced: said so.
 */
function secondProgram(): string {
  const same = () => {
    const [a, b] = [EXE, SECOND_EXE].map((p) => (fs.existsSync(p) ? fs.statSync(p) : null));
    return !!a && !!b && b.size === a.size && Math.abs(b.mtimeMs - a.mtimeMs) < 2000;
  };
  if (same()) return SECOND_EXE;
  // Copied aside, then put in place at once: two workers may make it at the same time.
  const part = `${SECOND_EXE}.${process.pid}.part`;
  try {
    fs.copyFileSync(EXE, part);
    const { atime, mtime } = fs.statSync(EXE);
    fs.utimesSync(part, atime, mtime);
    fs.renameSync(part, SECOND_EXE);
  } catch (e) {
    fs.rmSync(part, { force: true });
    if (!same()) throw new Error(`could not copy the app to ${SECOND_EXE} for a second app (still running?): ${(e as Error).message}`);
  }
  return SECOND_EXE;
}

export type Language = 'fr' | 'en';

/** What the helpers below click and read, in each language of the interface. */
const LABELS = {
  fr: {
    addProject: /Ajouter un projet/,
    firstLaunch: 'Premier lancement',
    newProject: 'Nouveau projet',
    path: 'C:\\chemin\\vers\\le\\projet',
    repo: /Dépôt git détecté · branche main · propre/,
    firstAgent: 'Créer un premier agent',
    worktrees: 'Un worktree par agent',
    atFirstMessage: 'Créer le worktree au premier message, au nom de l’agent',
    create: 'Créer le projet',
    without: /^Créer sans l.assistant$/,
    blank: /^Partir de zéro/,
    next: 'Suivant',
    summary: /Récapitulatif/,
  },
  en: {
    addProject: /Add a project/,
    firstLaunch: 'First launch',
    newProject: 'New project',
    path: 'C:\\path\\to\\the\\project',
    repo: /Git repository detected · branch main · clean/,
    firstAgent: 'Create a first agent',
    worktrees: 'One worktree per agent',
    atFirstMessage: 'Create the worktree at the first message, under the agent’s name',
    create: 'Create project',
    without: 'Create without the assistant',
    blank: /^Start from scratch/,
    next: 'Next',
    summary: /Summary/,
  },
} as const;

/** The store the app reads: a closed port, unless a test starts the fake one (tests/fixtures/store/fake-store.mjs). */
const NO_STORE = 'http://127.0.0.1:9';

/**
 * The app's window is kept out of sight (off every screen, never focused, no taskbar button): the tests run while the user
 * works, without taking their keyboard. `ESCOUADE_E2E_SHOW=1` shows it, in front, to watch a test.
 */
export const SHOW = !!process.env.ESCOUADE_E2E_SHOW && process.env.ESCOUADE_E2E_SHOW !== '0';

/**
 * `ESCOUADE_E2E_REMOTE=1` (ESC-495): the window speaks to the core by the remote API's WebSocket on the loopback, not by
 * Tauri's IPC (`src/lib/transport.ts`): the app is started with `ESCOUADE_REMOTE_API` (the port of
 * `ESCOUADE_E2E_REMOTE_PORT`, else a free one) and a device token of the test (`ESCOUADE_REMOTE_API_TOKEN`), and a
 * window that would still be on the IPC fails the test. The CI's part « remote API » runs a set of specs so.
 */
export const REMOTE = !!process.env.ESCOUADE_E2E_REMOTE && process.env.ESCOUADE_E2E_REMOTE !== '0';
const REMOTE_PORT = Number(process.env.ESCOUADE_E2E_REMOTE_PORT ?? 0);

/**
 * `ESCOUADE_E2E_VIEWPORT=1024x728`: the size of the window at each start, as the CI's runner bounds it (its screen); else
 * the window keeps its own (1440 × 900: off every screen, nothing bounds it). A test that sets its own size still does.
 * The window is made that size (`ESCOUADE_WINDOW_SIZE`), and its page too: the page alone, the app would hold its zoom
 * to a window of 1440 × 900 (ESC-440).
 */
const VIEWPORT = /^(\d+)x(\d+)$/.exec(process.env.ESCOUADE_E2E_VIEWPORT ?? '');

/**
 * `ESCOUADE_E2E_SLOW_IPC=duplicate_agent=1500,send_message=300`: those commands of the window reach the core that many ms
 * late, at each start, as on a slow runner, to bring out a test that acts before the state it waits for is shown (ESC-498:
 * a message typed while « Dupliquer la conversation » was under way went to the original, or stayed in its draft).
 */
const SLOW_IPC = new Map(
  (process.env.ESCOUADE_E2E_SLOW_IPC ?? '')
    .split(',')
    .map((s) => s.split('='))
    .filter(([cmd, ms]) => cmd && Number(ms) > 0)
    .map(([cmd, ms]) => [cmd.trim(), Number(ms)] as const),
);

async function slowIpc(page: Page) {
  // Windows' WebView2 calls a command at http://ipc.localhost/<command>. Through CDP, paused for those commands alone:
  // `page.route` would pause every request of the page on its way to the test, and slow the app's start under load.
  const cdp = await page.context().newCDPSession(page);
  cdp.on('Fetch.requestPaused', (e) => {
    const ms = SLOW_IPC.get(new URL(e.request.url).pathname.slice(1)) ?? 0;
    setTimeout(() => cdp.send('Fetch.continueRequest', { requestId: e.requestId }).catch(() => {}), ms);
  });
  const patterns = [...SLOW_IPC.keys()].map((cmd) => ({ urlPattern: `http://ipc.localhost/${cmd}*` }));
  await cdp.send('Fetch.enable', { patterns });
}

/**
 * WebView2 draws and runs its timers in full even though its window is not seen: Chromium slows down a window it takes for
 * hidden or covered. Off the screens, the window was drawn without them in short tests (ESC-126): they keep it so in the long
 * ones. The same in ci.yml, where they reach WebView2 by the machine policy.
 */
const RENDER_UNSEEN = '--disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-background-timer-throttling';

/**
 * The page drawn at a scale of 1, as on a CI runner, whatever the display's (125 % or 150 % is common on a laptop or a wide
 * screen): at another scale, widths and heights come out fractional (a row of 20.79 px, 391 px for a viewport of 390) and the
 * tests, which compare pixels, fail with nothing wrong in the app.
 */
const FIXED_SCALE = '--force-device-scale-factor=1';

/** How long `restart()` leaves the app to write its state before it ends it. */
const SAVE_MS = 1500;

/** The machine's account of a harness, as « Ajouter un compte… » adds it (`accounts::principal_of`). */
function machineAccount(engine: string): Record<string, unknown> {
  return {
    id: engine === 'claude' ? 'principal' : `principal-${engine}`,
    name: 'Principal',
    engine,
    configDir: '',
    claudePath: '',
    active: true,
  };
}

/** The variable that gives the fake program of each harness other than Claude Code, by harness. */
const FAKE_OF: Record<string, string> = {
  codex: 'ESCOUADE_CODEX',
  opencode: 'ESCOUADE_OPENCODE',
  copilot: 'ESCOUADE_COPILOT',
  cursor: 'ESCOUADE_CURSOR',
  vibe: 'ESCOUADE_VIBE',
  agy: 'ESCOUADE_AGY',
  gemini: 'ESCOUADE_GEMINI',
};

/**
 * The accounts a test starts with (a first installation has none, ESC-128): the machine's account of Claude Code, and of each
 * harness whose fake program the test gives (`appEnv`), before those it gives (`preStart`) unless it gives them; none
 * but its own when it says `noAccounts`.
 */
function accountsFor(given: Record<string, unknown>, appEnv: Record<string, string>): unknown[] {
  const own = (given.accounts as { id: string }[] | undefined) ?? [];
  if (given.noAccounts) return own;
  const engines = ['claude', ...Object.keys(FAKE_OF).filter((e) => appEnv[FAKE_OF[e]])];
  const machine = engines.map(machineAccount).filter((m) => !own.some((a) => a.id === m.id));
  return [...machine, ...own];
}

/**
 * `expect`, with `toLookLike(expected)`: what the « look » specs compare an element of the app with the preview's by. Both
 * looks (strings, numbers, arrays, objects of them) are the same when every colour in them is, in sRGB, within a unit of a
 * channel (`oklch()` here, `lab()` there, one colour), and everything else is equal to the letter (srgb.ts, ESC-459).
 */
const expect = baseExpect.extend({
  toLookLike(received: unknown, expected: unknown) {
    const diff = lookDiff(received, expected);
    return {
      pass: diff.length === 0,
      name: 'toLookLike',
      message: () => (diff.length ? `the looks differ (colours in sRGB):\n  ${diff.join('\n  ')}` : 'the looks are the same'),
    };
  },
});

export const test = base.extend<{
  app: App;
  appEnv: Record<string, string>;
  appArgs: string[];
  language: Language;
  preStart: { run: (root: string) => Record<string, unknown> | void };
  server: boolean;
  sshServer: boolean | number;
}>({
  // An `escouade-server` to connect to through the fake `ssh` (`test.use({ sshServer: true })`, ESC-537): `app.ssh`.
  sshServer: [false, { option: true }],
  // The window on an `escouade-server` of the test's (`test.use({ server: true })`, ESC-514): `app.server`.
  server: [false, { option: true }],
  // Variables for the app on top of the fixture's, which they replace (`test.use({ appEnv: … })`):
  // Principal’s folder with a fake-limit in it.
  appEnv: [{}, { option: true }],
  // The arguments of the first launch (`test.use({ appArgs: ['escouade://plugin/<id>'] })`, STORE-11): a restart has none.
  appArgs: [[], { option: true }],
  // The language of the interface the app starts in (`test.use({ language: 'en' })`): French, whatever the machine's.
  language: ['fr', { option: true }],
  // `run` is called before the app starts, with the test's folder (`test.use({ preStart: { run: (root) => … } })`): files it
  // may put there (a second Claude account's folder and sign-in), and the settings to start with, merged into settings.json.
  // (An object: Playwright reads a function given as an option for a fixture.)
  preStart: [{ run: () => ({}) }, { option: true }],
  // Its own time, apart from the test's: starting an app on a loaded machine, and waiting until the one of the test
  // before is gone (its WebView2 may take more than 30 s, ESC-522), is not the test's work.
  app: [
    async ({ appEnv, appArgs, language, preStart, server, sshServer }, use, testInfo) => {
      if (!fs.existsSync(EXE)) throw new Error(`build the app first: npx tauri build --debug --no-bundle (missing ${EXE})`);
      // CI runs elevated, where WebView2 ignores WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: the workflow
      // opens a fixed port through the machine policy instead (tests run one at a time), by the program's name
      // (`second()` runs a copy under another name for that, ESC-496; ESCOUADE_E2E_POLICY_LIKE=1 does the same here).
      // Outside CI, a port drawn at random, free with the next one (`second()`'s): another worktree's e2e may run an app on
      // the one drawn, which would never close; another is drawn then (ESC-496).
      const port = FIXED_PORT
        ? FIXED_PORT
        : await freePort(() => 9400 + (testInfo.workerIndex * 50 + Math.floor(Math.random() * 50)), { span: 2 });
      // The instance of the test before may still be dying, and answering on the fixed port of CI: connect() would attach to
      // it. Wait for it to be gone.
      if (FIXED_PORT && !(await waitForPortToClose(port))) {
        throw new Error(
          `something still listens on port ${port} after 30 s: the app of the test before is not gone (${await listenersOn(port)})`,
        );
      }
      const root = testRoot();
      const data = path.join(root, 'data');
      fs.mkdirSync(data);
      // Every data folder the test's apps ran on (`restartAs`), whose logs the end of the test reads.
      const datas = new Set([data]);
      // French unless a test asks otherwise, whatever the machine's language (CI runners are English): the tests read the
      // French interface.
      const given = (preStart.run(root) ?? {}) as Record<string, unknown>;
      fs.writeFileSync(
        path.join(data, 'settings.json'),
        JSON.stringify({
          claudePath: FAKE,
          sound: false,
          osNotifications: false,
          idleStopMinutes: 0,
          language,
          // The question of the first start answered (a sandbox never sends anything anyway).
          telemetry: false,
          // And the other question of the first start, the user's name for the shared tickets (ESC-190): put off.
          nameAsked: true,
          // Settings of this version: no update to bring them through (ESC-128), unless a test says otherwise.
          accountsVersion: 1,
          // No automatic restart for an update of Escouade: the hidden window of the tests reads as nobody there, and a
          // version published during a run would restart the app under test (a message typed then stays in its field).
          autoUpdate: false,
          // A message sent during a turn goes at once, as the tests expect.
          midTurnDelaySecs: 0,
          ...given,
          accounts: accountsFor(given, appEnv),
        }),
      );
      // A repository `run` made already (the site's screenshots make their own) is kept.
      const repo = fs.existsSync(path.join(root, 'demo-api', '.git')) ? path.join(root, 'demo-api') : makeRepo(root);
      const log = path.join(root, 'fake-claude.jsonl');
      const stderr = path.join(root, 'app-stderr.log');
      // The same for every start of the test, `restart()`'s included: the data folder is what the app finds again.
      const env: NodeJS.ProcessEnv = {
        ...process.env,
        ESCOUADE_DATA_DIR: data,
        FAKE_CLAUDE_LOG: log,
        // Sent "from claude.ai" to an agent once its Remote Control is on.
        FAKE_CLAUDE_REMOTE_MESSAGE: 'Message depuis le téléphone',
        ...webviewEnv(EXE, port, path.join(root, 'webview'), port),
        ESCOUADE_HIDDEN_WINDOW: SHOW ? undefined : '1',
        // The window itself at that size, not only its page: the app holds its zoom to the window's size (ESC-440).
        ESCOUADE_WINDOW_SIZE: VIEWPORT ? `${VIEWPORT[1]}x${VIEWPORT[2]}` : undefined,
        // Never the machine user's own sign-in nor Anthropic's endpoint: Principal's Claude Code
        // folder is a test one (with no sign-in in it), and the quota is asked of nobody.
        CLAUDE_CONFIG_DIR: path.join(root, 'claude'),
        // The fake `claude mcp …` keeps its servers in a folder of the test's, never in the user's home.
        FAKE_CLAUDE_HOME: path.join(root, 'home'),
        CLAUDE_SECURESTORAGE_CONFIG_DIR: undefined,
        // Set for every agent of Escouade (« Les agents tiennent une liste de tâches »), so inherited when one runs
        // the tests: the app never forces it off, and the fake CLI would see it whatever the setting (ESC-111).
        CLAUDE_CODE_ENABLE_TODO_TOOLS: undefined,
        // The same for what turns Claude Code's own update off (`DISABLE_AUTOUPDATER`, given to every claude Escouade
        // launches since ESC-133): inherited, the app would take it for the user's choice and never run `claude update`.
        // A test that wants it sets it in `appEnv`.
        DISABLE_AUTOUPDATER: undefined,
        DISABLE_UPDATES: undefined,
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: undefined,
        // Nor store.escouade.dev (STORE-9): a closed port, unless a test starts the fake store and names it.
        ESCOUADE_STORE_URL: process.env.ESCOUADE_STORE_URL ?? NO_STORE,
        ESCOUADE_STORE_MIRROR_URL: process.env.ESCOUADE_STORE_MIRROR_URL ?? NO_STORE,
        ESCOUADE_STORE_TEST_KEY: process.env.ESCOUADE_STORE_TEST_KEY,
        // Nor Open VSX (the editor's extensions, ESC-481): a closed port, unless a test starts the fake one and names it.
        ESCOUADE_TEST_OPEN_VSX: NO_STORE,
        // Nor the user's box nor a STUN server (the phone's tunnel, ESC-567): nothing is looked at, nothing opened, unless
        // a test names its fake box (`fake;natpmp=…;stun=…`).
        ESCOUADE_PHONE_NET: 'off',
        // Nor the real updates of Escouade (ESC-543): a debug build given no test server checks none (`updates::endpoints_of`),
        // so that none is downloaded, and none installed at « Quitter » over the app of the machine. Only the tests of the
        // updates give theirs (`appEnv: { ESCOUADE_UPDATE_ENDPOINT }`); an inherited one is not taken. The end of each test
        // reads the log for it (`updateTrouble`).
        ESCOUADE_UPDATE_ENDPOINT: undefined,
        // No Codex but the fake one a test asks for (`appEnv: { ESCOUADE_CODEX: FAKE_CODEX }`), never the machine's.
        ESCOUADE_CODEX: 'none',
        // Codex's Principal is a folder of the test's (its `config.toml`, « Les agents peuvent piloter Escouade »), never the user's ~/.codex.
        CODEX_HOME: path.join(root, 'codex'),
        FAKE_CODEX_LOG: path.join(root, 'fake-codex.jsonl'),
        // The same for OpenCode (`appEnv: { ESCOUADE_OPENCODE: FAKE_OPENCODE }`); its sessions kept in the test's folder.
        ESCOUADE_OPENCODE: 'none',
        FAKE_OPENCODE_LOG: path.join(root, 'fake-opencode.jsonl'),
        FAKE_OPENCODE_HOME: path.join(root, 'fake-opencode'),
        // OpenCode's global config (`opencode.json`, where the switch declares the server): a folder of the test's,
        // never the user's ~/.config/opencode, even when the switch takes the entry out of a harness not installed.
        XDG_CONFIG_HOME: path.join(root, 'xdg'),
        // The same for Copilot CLI (`appEnv: { ESCOUADE_COPILOT: FAKE_COPILOT }`): never the machine's, its Principal
        // (`COPILOT_HOME`) a folder of the test's, never the user's ~/.copilot.
        ESCOUADE_COPILOT: 'none',
        COPILOT_HOME: path.join(root, 'copilot'),
        FAKE_COPILOT_LOG: path.join(root, 'fake-copilot.jsonl'),
        // The same for Cursor CLI (`appEnv: { ESCOUADE_CURSOR: FAKE_CURSOR }`): never the machine's, nor its sign-in
        // (`%APPDATA%\Cursor\auth.json`): a file of the test's stands for it.
        ESCOUADE_CURSOR: 'none',
        ESCOUADE_CURSOR_AUTH: cursorAuth(root),
        FAKE_CURSOR_LOG: path.join(root, 'fake-cursor.jsonl'),
        // Its folder of the workspaces (`~/.cursor/projects`: transcripts and trust of its interface, « Vue Cursor CLI »), a
        // folder of the test's, never the user's.
        ESCOUADE_CURSOR_PROJECTS: path.join(root, 'cursor-projects'),
        // The same for Mistral Vibe (`appEnv: { ESCOUADE_VIBE: FAKE_VIBE }`): never the machine's; each agent has its own
        // VIBE_HOME in the app's data, never the user's ~/.vibe.
        ESCOUADE_VIBE: 'none',
        FAKE_VIBE_LOG: path.join(root, 'fake-vibe.jsonl'),
        // The same for Antigravity CLI (`appEnv: { ESCOUADE_AGY: FAKE_AGY }`): each agent has its own home in the app's data,
        // never the user's ~/.gemini.
        ESCOUADE_AGY: 'none',
        FAKE_AGY_LOG: path.join(root, 'fake-agy.jsonl'),
        // Its Google sign-in (the keychain's entry `gemini:antigravity`, ESC-271): a file of the test's stands for it, there
        // unless the test says `agySignedOut`; the fake `agy` writes it when it is signed in in its terminal.
        ESCOUADE_AGY_SIGNED_IN: agySignIn(root, !!given.agySignedOut),
        // The same for Gemini CLI (`appEnv: { ESCOUADE_GEMINI: FAKE_GEMINI }`): each agent has its own GEMINI_CLI_HOME in the
        // app's data, never the user's ~/.gemini.
        ESCOUADE_GEMINI: 'none',
        FAKE_GEMINI_LOG: path.join(root, 'fake-gemini.jsonl'),
        ...(REMOTE ? { ESCOUADE_REMOTE_API: String(REMOTE_PORT), ESCOUADE_REMOTE_API_TOKEN: randomBytes(16).toString('hex') } : {}),
        ...appEnv,
      };
      // The window on an `escouade-server` of the test's (ESC-514): started on a data folder of its own with the same settings,
      // a device paired, a local forward to it; the app's own core serves nothing. Its ports: the worktree's
      // (`ESCOUADE_PORT_BASE` + 2 and + 3) when given, else free ones.
      let served: Server | null = null;
      if (server) {
        const serverData = path.join(root, 'server-data');
        fs.mkdirSync(serverData);
        fs.copyFileSync(path.join(data, 'settings.json'), path.join(serverData, 'settings.json'));
        const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
        served = await startServer(
          serverProgram(EXE),
          serverData,
          { ...env, ESCOUADE_DATA_DIR: serverData },
          {
            server: base ? base + 2 : 0,
            forward: base ? base + 3 : 0,
          },
        );
        delete env.ESCOUADE_REMOTE_API;
        env.ESCOUADE_REMOTE_URL = served.url;
        env.ESCOUADE_REMOTE_API_TOKEN = served.token;
      }
      // An `escouade-server` reached through the fake `ssh` (ESC-537): on the worktree's base + 2 when given. The app is
      // given the fake and a ~/.ssh/config of the test's; never a real SSH.
      let sshed: App['ssh'] = null;
      if (sshServer) {
        const serverData = path.join(root, 'server-data');
        fs.mkdirSync(serverData);
        fs.copyFileSync(path.join(data, 'settings.json'), path.join(serverData, 'settings.json'));
        const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
        const dir = path.join(root, 'fake-ssh');
        fs.mkdirSync(dir);
        // The ports a program of the server takes later (the fake `codex login`'s), mapped by the fake `ssh` (ESC-538).
        const portsFile = path.join(dir, 'ports');
        const serverEnv = { ...env, ESCOUADE_DATA_DIR: serverData, ESCOUADE_BROWSE_HOME: root, FAKE_SSH_PORTS_FILE: portsFile };
        const program = serverProgram(EXE);
        const bare = await startBareServer(program, serverData, serverEnv, base ? base + 2 : 0);
        const config = path.join(root, 'ssh-home', '.ssh', 'config');
        fs.mkdirSync(path.dirname(config), { recursive: true });
        fs.writeFileSync(
          config,
          'Host travail\n  HostName escouade.test.internal\n  User escouade\n\nHost labo-inconnu\n  HostName labo.test.internal\n  User gpu\n  ProxyJump travail\n\n' +
            (Number(sshServer) >= 2 ? 'Host perso\n  HostName vps.test.example\n  User escouade\n\n' : '') +
            'Host *\n  ServerAliveInterval 30\n',
        );
        env.ESCOUADE_SSH = process.execPath;
        env.ESCOUADE_SSH_PREFIX = path.join(ROOT, 'tests', 'fixtures', 'fake-ssh.mjs');
        env.ESCOUADE_SSH_CONFIG = config;
        env.FAKE_SSH_DIR = dir;
        env.FAKE_SSH_PORTS = `46480=${bare.port}`;
        // A second server (ESC-539): base + 3, its port on the server 46481.
        let second: { server: BareServer; env: NodeJS.ProcessEnv } | null = null;
        if (Number(sshServer) >= 2) {
          const data2 = path.join(root, 'server-data-2');
          fs.mkdirSync(data2);
          fs.copyFileSync(path.join(data, 'settings.json'), path.join(data2, 'settings.json'));
          const env2 = { ...env, ESCOUADE_DATA_DIR: data2 };
          const bare2 = await startBareServer(program, data2, env2, base ? base + 3 : 0);
          env.FAKE_SSH_PORTS += `,46481=${bare2.port}`;
          second = { server: bare2, env: env2 };
        }
        env.FAKE_SSH_SERVER = program;
        env.FAKE_SSH_DATA = serverData;
        env.FAKE_SSH_PORTS_FILE = portsFile;
        env.ESCOUADE_DOWNLOADS_DIR = path.join(root, 'downloads');
        env.ESCOUADE_OPEN_LOG = path.join(root, 'opened.log');
        env.ESCOUADE_ALERT_LOG = path.join(root, 'alerts.log');
        sshed = { server: bare, program, dir, env: serverEnv, second };
      }
      // The launch the test is on (the last one, after a restart): the one the end of the test must kill.
      let instance: Instance | null = null;
      const current = () => instance as Instance;
      // The first app's WebView2 folder, which its browser holds until its last process is gone.
      const webviewFolder = path.join(root, 'webview');
      /** The second app of a test of two live clients, and its port. */
      const others: { instance: Instance; port: number; data: string }[] = [];
      // The ports of the test's apps, for the diagnostics.
      const ports = () => [port, ...others.map((o) => o.port)];
      // Attaches Playwright to the WebView2 of a launch: the first app's (on `port`), or a second one's (`on`).
      const attach = async (launch: Instance, on = { port, data: env.ESCOUADE_DATA_DIR ?? data, stderr }): Promise<Page> => {
        try {
          const { browser, page } = await connect(on.port, () => launch.stopped);
          if (VIEWPORT) await page.setViewportSize({ width: Number(VIEWPORT[1]), height: Number(VIEWPORT[2]) });
          launch.browser = browser;
          // On the app's page, loaded: a route set on the blank page it starts with is lost when it loads.
          if (SLOW_IPC.size) await slowIpc(await appReady(page));
          return page;
        } catch (e) {
          throw new Error(`${(e as Error).message}\n${await launchLogs(on.data, on.stderr)}`);
        }
      };
      const mainOn = () => ({ port, data: env.ESCOUADE_DATA_DIR ?? data, stderr });
      /**
       * Starts a launch (`start`, given to `set`) and attaches to it. A launch that stops as it starts because its WebView2
       * folder is still held (0x800700AA, « resource in use »: the browser of the instance killed just before, not quite gone)
       * is started once more, after that browser is gone, and the test says so; any other stop is the test's failure.
       */
      const startAttached = async (
        start: () => Instance,
        set: (launch: Instance) => void,
        on: { port: number; data: string; stderr: string },
        folder: string,
      ): Promise<Page> => {
        const marks = [on.stderr, path.join(on.data, 'app.log')].map((file) => ({ file, from: sizeOf(file) }));
        let launch = start();
        set(launch);
        try {
          return await attach(launch, on);
        } catch (e) {
          if (!launch.stopped || !marks.some((m) => since(m.file, m.from).includes(FOLDER_IN_USE))) throw e;
          const note = `the app stopped as it started, its WebView2 folder still in use (${FOLDER_IN_USE}, ${launch.stopped}): started once more`;
          console.warn(`${testInfo.title}: ${note}`);
          testInfo.annotations.push({ type: 'webview-in-use', description: note });
          await killApp(launch, folder);
          const left = await waitForWebviewGone(folder);
          if (left) console.warn(`${testInfo.title}: ${left} WebView2 process(es) still on ${folder}`);
          launch = start();
          set(launch);
          return attach(launch, on);
        }
      };
      const launchLogs = async (dataDir: string, errFile: string) => {
        const appLog = tail(path.join(dataDir, 'app.log'));
        const errLog = tail(errFile);
        return (
          `--- app.log ---\n${appLog}\n` +
          (errLog === appLog ? '' : `--- stderr ---\n${errLog}\n`) +
          `--- WebView2 ---\n${await webviewDiagnostics(root, ports())}`
        );
      };
      try {
        const page = await startAttached(
          () => startApp(env, stderr, appArgs),
          (launch) => (instance = launch),
          mainOn(),
          webviewFolder,
        );
        const launches = () =>
          fs.existsSync(log)
            ? fs
                .readFileSync(log, 'utf8')
                .trim()
                .split('\n')
                .filter(Boolean)
                .map((l) => JSON.parse(l))
            : [];
        const app: App = {
          page,
          repo,
          data,
          root,
          claudeJson: path.join(root, 'claude', '.claude.json'),
          codexToml: path.join(root, 'codex', 'config.toml'),
          opencodeJson: path.join(root, 'xdg', 'opencode', 'opencode.json'),
          copilotJson: path.join(root, 'copilot', 'mcp-config.json'),
          server: served,
          ssh: sshed,
          launches,
          launchedAt: () => current().at,
          stopped: () => current().stopped,
          restart: async () => {
            // The app saves what it shows a moment after a change: not killed in the middle of it.
            await new Promise((r) => setTimeout(r, SAVE_MS));
            await killApp(current(), webviewFolder);
            if (!(await waitForPortToClose(port))) {
              throw new Error(
                `something still listens on port ${port} 30 s after the app was killed: it cannot be started again (${await listenersOn(port)})`,
              );
            }
            // Nor its WebView2 browser, which holds the folder a while after the port is closed: the next start would stop
            // there (0x800700AA, « resource in use »).
            const left = await waitForWebviewGone(webviewFolder);
            if (left) console.warn(`${testInfo.title}: ${left} WebView2 process(es) still on ${webviewFolder} after the restart's wait`);
            app.page = await appReady(
              await startAttached(
                () => startApp(env, stderr),
                (launch) => (instance = launch),
                mainOn(),
                webviewFolder,
              ),
            );
            return app.page;
          },
          reconnect: async () => {
            await current()
              .browser?.close()
              .catch(() => {});
            app.page = await appReady(await attach(current()));
            return app.page;
          },
          restartAs: (dataDir: string) => {
            env.ESCOUADE_DATA_DIR = dataDir;
            datas.add(dataDir);
            return app.restart();
          },
          second: async (dataDir: string) => {
            // The next port (checked free with the first one's, outside CI), or the one given; with POLICY_LIKE, always the
            // next one, as ci.yml's policy.
            let port2 = process.env.ESCOUADE_E2E_SECOND_CDP_PORT ? Number(process.env.ESCOUADE_E2E_SECOND_CDP_PORT) : port + 1;
            if (!FIXED_PORT && !POLICY_LIKE && !process.env.ESCOUADE_E2E_SECOND_CDP_PORT) {
              let next = port2;
              port2 = await freePort(() => next++);
            } else if (!(await waitForPortToClose(port2))) throw new Error(`something still listens on port ${port2}: no second app`);
            // Its own program (the policy's line of an elevated runner is by name), WebView2 folder and port: nothing shared
            // with the first app but what the test gives both (a team's server), not even a local storage (ESC-496).
            const program = secondProgram();
            const env2: NodeJS.ProcessEnv = {
              ...env,
              ESCOUADE_DATA_DIR: dataDir,
              ...webviewEnv(program, port2, path.join(root, 'webview-2'), port),
              // Its own port of the remote API, next to the first app's.
              ...(REMOTE ? { ESCOUADE_REMOTE_API: String(REMOTE_PORT ? REMOTE_PORT + 1 : 0) } : {}),
            };
            const errFile = path.join(root, 'app-2-stderr.log');
            const other = { instance: null as unknown as Instance, port: port2, data: dataDir };
            others.push(other);
            const p = await appReady(
              await startAttached(
                () => startApp(env2, errFile, [], program),
                (launch) => (other.instance = launch),
                { port: port2, data: dataDir, stderr: errFile },
                path.join(root, 'webview-2'),
              ),
            );
            // The page attached is the second app's, not the first one's seen through another port: the app says its data folder.
            const shown = await p.evaluate(() =>
              (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string): Promise<string> } }).__TAURI_INTERNALS__.invoke('data_dir'),
            );
            if (path.resolve(shown).toLowerCase() !== path.resolve(dataDir).toLowerCase()) {
              throw new Error(
                `the page on port ${port2} is not the second app's: its data folder is ${shown}, not ${dataDir}\n` +
                  (await launchLogs(dataDir, errFile)),
              );
            }
            await expect(p.getByRole('button', { name: LABELS[language].addProject }).first()).toBeVisible({ timeout: 45_000 });
            return p;
          },
        };
        // What the app showed and logged, for a failure on CI (annotations are all we get there).
        const state = async () => {
          const shown = await Promise.race([
            app.page
              .evaluate(() =>
                JSON.stringify({ ready: document.readyState, url: location.href, text: document.body?.innerText.slice(0, 300) }),
              )
              .catch((e) => `page unreachable: ${(e as Error).message.split('\n')[0]}`),
            new Promise<string>((r) => setTimeout(() => r('page not answering'), 5000)),
          ]);
          const second = others.map(
            (o) => `\n--- second app (${o.instance.stopped ?? 'running'}, port ${o.port}) ---\n${tail(path.join(o.data, 'app.log'), 25)}`,
          );
          return (
            `--- page ---\n${shown}\n--- app (${current().stopped ?? 'running'}) ---\n` +
            `${tail(path.join(env.ESCOUADE_DATA_DIR ?? data, 'app.log'), 25)}${second.join('')}`
          );
        };
        try {
          // A runner's window thread stops for 7 to 10 s at times while the app starts (« window thread: paused 9725 ms » in
          // its log), its page still blank: longer than the 15 s of an expect, which failed a test there before it began
          // (shell.spec.ts, tall-fields.spec.ts on the CI of 2026-11-08, ESC-393).
          // Or the first launch (ESC-504), which takes the rail's place on a new installation.
          await expect(
            page
              .getByRole('button', { name: LABELS[language].addProject })
              .first()
              .or(page.getByRole('region', { name: LABELS[language].firstLaunch })),
          ).toBeVisible({ timeout: 45_000 });
        } catch (e) {
          throw new Error(`${(e as Error).message.split('\n')[0]}\n${await state()}`);
        }
        await use(app);
        if (testInfo.status !== testInfo.expectedStatus) throw new Error(`the app after the failure\n${await state()}`);
        // Never the real updates of Escouade (ESC-543): what an app of the test logged of them fails the test.
        const troubles = [...datas].flatMap((d) => {
          const file = path.join(d, 'app.log');
          return fs.existsSync(file) ? updateTrouble(fs.readFileSync(file, 'utf8')) : [];
        });
        if (troubles.length) throw new Error(`an app of the test went for a real update of Escouade (ESC-543):\n${troubles.join('\n')}`);
      } finally {
        for (const other of others) {
          if (other.instance) await killApp(other.instance, path.join(root, 'webview-2'));
          await waitForPortToClose(other.port);
        }
        if (instance) await killApp(instance, webviewFolder);
        await served?.stop();
        await sshed?.server.stop();
        await sshed?.second?.server.stop();
        // The next test must not find this instance still answering on the port (it waits for that too, before it
        // starts; a test that passed does not fail for it).
        if (!(await waitForPortToClose(port)))
          console.warn(`port ${port} is still taken 30 s after the app was killed: ${await listenersOn(port)}`);
        // Windows releases the handles of killed processes (a terminal's shell sits in the repo)
        // with a delay: a temporary folder left behind must not fail the test.
        try {
          if (!FIXED_ROOT) fs.rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
        } catch (e) {
          console.warn(`could not remove ${root}: ${(e as Error).message}`);
        }
      }
    },
    { timeout: 300_000 },
  ],
});

/**
 * Opens « Nouveau projet » from « + » of the projects' rail (ESC-158), through its menu when projects were closed (it then
 * lists them first): « Ajouter un projet… ».
 */
/**
 * The scale of the app's window (1 on a CI runner, 1.25 on a display at 125 %). The page is drawn at a scale of 1
 * (FIXED_SCALE), so what the app measures in logical pixels (the window's size over this scale) reaches the page in physical
 * ones: `innerWidth` is the logical width times this scale.
 */
export async function windowScale(page: Page): Promise<number> {
  return page.evaluate(async () => {
    const t = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<number> } }).__TAURI_INTERNALS__;
    return t.invoke('plugin:window|scale_factor', { label: 'main' }).catch(() => 1);
  });
}

export async function openNewProject(page: Page, lang: Language = 'fr') {
  const words = LABELS[lang];
  // With no project yet, the panel's own button, the way of a first start: it opens the window at once. The rail's « + »,
  // first in the page, asks the core for the closed projects before it opens anything, and that answer came after the
  // 15 s below as a loaded machine started the app (ESC-522). Which one is known once the work area says it: empty until the
  // app is ready (App.svelte, `!app.ready`), then the panel without a harness until the harness' plugin is loaded
  // (`enginesFound` empty, filled by `refreshEngines`), and only then its button; or the work area of a project.
  const panel = page.locator('.work').getByRole('button', { name: words.addProject });
  const project = page.locator('.work:not(:empty):not(:has(.welcome))');
  await expect(panel.or(project).first()).toBeVisible({ timeout: 60_000 });
  await ((await panel.count()) ? panel.first() : page.getByRole('button', { name: words.addProject }).first()).click();
  const item = page.getByRole('menuitem', { name: words.addProject });
  const dialog = page.getByRole('dialog');
  await expect(item.or(dialog).first()).toBeVisible();
  if (await item.isVisible()) await item.click();
}

/**
 * Adds `repo` as a project through the "Nouveau projet" dialog (`lang: 'en'`: "New project"), with a first agent.
 * The dialog is read in the language of the interface the test started in, French unless it says so. With Escouade's
 * defaults: « Créer sans l'assistant », or the assistant from scratch when a worktree per agent or no first agent is
 * asked for. A worktree per agent is made with the agent, as `agent-N`, unless `atFirstMessage` keeps Escouade's
 * default (ESC-156: made at its first message, under its name).
 */
export async function addProject(
  page: Page,
  repo: string,
  opts: { worktrees?: boolean; atFirstMessage?: boolean; name?: string; firstAgent?: boolean; lang?: Language } = {},
) {
  const words = LABELS[opts.lang ?? 'fr'];
  await openNewProject(page, opts.lang);
  const dialog = page.getByRole('dialog', { name: words.newProject });
  await dialog.getByPlaceholder(words.path).fill(repo);
  await expect(dialog.getByText(words.repo)).toBeVisible();
  if (opts.name) await dialog.locator('input').nth(1).fill(opts.name);
  if (!opts.worktrees && opts.firstAgent !== false) {
    await dialog.getByRole('button', { name: words.without }).click();
  } else {
    await dialog.getByRole('radio', { name: words.blank }).click();
    if (opts.worktrees) {
      await dialog.getByRole('button', { name: words.next, exact: true }).click();
      await dialog.getByRole('switch', { name: words.worktrees }).click();
      if (!opts.atFirstMessage) await dialog.getByRole('switch', { name: words.atFirstMessage }).click();
    }
    await dialog.getByRole('button', { name: words.summary }).click();
    if (opts.firstAgent === false) await dialog.getByRole('switch', { name: words.firstAgent }).click();
    await dialog.getByRole('button', { name: words.create }).click();
  }
  await expect(dialog).toBeHidden();
}

/**
 * Lancement and Terminaux, folded to their header in a short window (choice 9 of the 2.0 design; the runner's screen is
 * 768 px tall): unfolded by their title, for a test that uses their rows. Nothing to do in a taller window.
 */
export async function unfoldSections(page: Page) {
  // One at a time: the list shrinks as each one unfolds (`all()` would look for an nth that is gone).
  const folded = page.locator('.sec-h > .fold[aria-expanded="false"]');
  while ((await folded.count()) > 0) await folded.first().click();
}

/** The pages of a project in the bar of the settings (ESC-379), shown under the project unfolded. */
const PROJECT_PAGES = new Set([
  'Projet',
  'Git et worktrees',
  'Lancement',
  'Kanban',
  'Tickets importés',
  'Équipe',
  'Confidentialité',
  'Project',
  'Git and worktrees',
  'Launch',
  'Imported tickets',
  'Team',
  'Privacy',
]);

/** The bar of the settings window: its pages, its projects (direction A, ESC-379). */
export const settingsNav = (where: Page | Locator) => where.getByRole('dialog', { name: /^(Réglages|Settings)$/ }).getByRole('navigation');

/** An entry of the bar of the settings: a page by its name (« À propos », « Comptes et harness »), or a project. */
export function settingsEntry(where: Page | Locator, name: string | RegExp) {
  const nav = 'goto' in where ? settingsNav(where) : where.getByRole('navigation');
  return nav.getByRole('button', typeof name === 'string' ? { name, exact: true } : { name });
}

/**
 * Opens the settings on a page (ESC-379, in place of the 2.0 tabs): `where` is the app's page (the window is opened by
 * « Réglages (Ctrl+,) » when it is not), or the settings dialog. A page of a project (« Kanban », « Projet »…) is the
 * one of `project` (its name; else the one chosen, else the first), unfolded first. Answers the dialog.
 */
export async function openSettingsPage(where: Page | Locator, name: string | RegExp, opts: { project?: string | RegExp } = {}) {
  let dialog: Locator;
  if ('goto' in where) {
    dialog = where.getByRole('dialog', { name: /^(Réglages|Settings)$/ });
    if (!(await dialog.isVisible())) await where.getByRole('button', { name: /^(Réglages|Settings) \(/ }).click();
  } else dialog = where;
  const nav = dialog.getByRole('navigation');
  if (opts.project !== undefined) {
    const row = nav
      .locator('button[aria-expanded]')
      .filter({ has: dialog.page().locator('.lbl', { hasText: opts.project }) })
      .first();
    if ((await row.getAttribute('aria-expanded')) !== 'true') await row.click();
  } else if (typeof name === 'string' && PROJECT_PAGES.has(name) && !(await settingsEntry(dialog, name).isVisible())) {
    await nav.locator('button[aria-expanded]').first().click();
  }
  const entry = settingsEntry(dialog, name);
  await entry.click();
  await expect(entry).toHaveAttribute('aria-current', 'page');
  return dialog;
}

/** Unfolds « Avancé » at the foot of the page shown (direction A): the rare settings it folds. Answers the dialog. */
export async function openAdvanced(dialog: Locator) {
  const fold = dialog.locator('#settings-panel').getByRole('button', { name: /^(Avancé|Advanced)/ });
  if ((await fold.getAttribute('aria-expanded')) !== 'true') await fold.click();
  await expect(fold).toHaveAttribute('aria-expanded', 'true');
  return dialog;
}

/**
 * Closes the settings window once what was changed in it is saved (ESC-380): no « Enregistrer », each setting is saved
 * as it changes, a text when it loses the focus. The field typed in is left first (a click on the page's title).
 */
export async function closeSettings(dialog: Locator) {
  await dialog.locator('.s-head h1').click();
  await expect(dialog).toHaveAttribute('data-saving', 'false');
  await dialog.locator('.s-head .close').click();
  await expect(dialog).toBeHidden();
}

/**
 * « Dupliquer la conversation » from the menu of the agent `name`'s card, then waits for the copy to be the conversation
 * shown, its composer its own, as the user does before writing to it. Not sooner: the copy is made in the core first, and a
 * message typed meanwhile goes to the original, still shown, or stays in its draft (ESC-498, codex.spec.ts on the CI).
 */
export async function duplicateConversation(page: Page, name: string) {
  await page.locator('.card', { hasText: name }).first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Dupliquer la conversation' }).click();
  const copy = `${name} (copie)`;
  await expect(page.locator('main.conv h1')).toHaveText(copy);
  await expect(page.getByRole('textbox').last()).toHaveAccessibleName(`Envoyer un message à ${copy}…`);
}

export async function send(page: Page, text: string) {
  const box = page.getByRole('textbox').last();
  await box.fill(text);
  await box.press('Enter');
}

export { expect, FAKE, FAKE_AGY, FAKE_CODEX, FAKE_COPILOT, FAKE_CURSOR, FAKE_GEMINI, FAKE_OPENCODE, FAKE_VIBE };
