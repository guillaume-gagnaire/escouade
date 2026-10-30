// Launches the real app on a throwaway data folder, with the fake `claude` CLI, and connects
// Playwright to its WebView2 through the DevTools protocol.

import { test as base, chromium, expect, type Browser, type Page } from '@playwright/test';
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { waitForPortToClose } from './port';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(ROOT, 'src-tauri', 'target', 'debug', 'escouade.exe');
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
  /** When the launch the test is on was started (ms since the epoch, just before its process): the startup bench's origin (INT-0). */
  launchedAt: () => number;
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
  }[];
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
function startApp(env: NodeJS.ProcessEnv, stderr: string, args: string[] = []): Instance {
  const errFd = fs.openSync(stderr, 'a');
  const at = Date.now();
  const child = spawn(EXE, args, { env, stdio: ['ignore', 'ignore', errFd] });
  fs.closeSync(errFd);
  const instance: Instance = { child, at, stopped: null, browser: null };
  child.on('exit', (code, signal) => {
    instance.stopped = signal ? `killed by ${signal}` : `exit code ${code} (0x${((code ?? 0) >>> 0).toString(16)})`;
  });
  child.on('error', (e) => (instance.stopped = `could not start: ${e.message}`));
  return instance;
}

/** Ends the app and its whole process tree (the fake CLIs, the WebView2); the port it listened on may still answer a moment. */
async function killApp(instance: Instance) {
  await instance.browser?.close().catch(() => {});
  instance.browser = null;
  if (instance.child.pid) {
    try {
      execFileSync('taskkill', ['/PID', String(instance.child.pid), '/T', '/F'], { stdio: 'ignore' });
    } catch {
      // already gone
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

/** How the app's WebView2 was started, when its DevTools endpoint cannot be reached (CI diagnostics). */
function webviewDiagnostics(root: string): string {
  const webviewDir = path.join(root, 'webview');
  const out = [`webview data folder: ${fs.existsSync(webviewDir) ? fs.readdirSync(webviewDir).join(', ') || '(empty)' : '(missing)'}`];
  try {
    const ours = execFileSync(
      'powershell',
      ['-NoProfile', '-Command', 'Get-CimInstance Win32_Process -Filter "Name=\'msedgewebview2.exe\'" | ForEach-Object { $_.CommandLine }'],
      { encoding: 'utf8', timeout: 20_000 },
    )
      .split('\n')
      .filter((l) => l.includes(path.basename(root)));
    const browser = ours.find((l) => !l.includes('--type='));
    out.push(`WebView2 processes of this app: ${ours.length}`);
    if (browser) {
      out.push(`  remote debugging: ${browser.match(/--remote-debugging-port=\d+/)?.[0] ?? '(not passed)'}`);
      out.push(`  browser flags: ${[...new Set(browser.match(/--[a-z][a-z0-9-]*/g) ?? [])].join(' ')}`);
    }
  } catch (e) {
    out.push(`WebView2 processes: ${(e as Error).message.split('\n')[0]}`);
  }
  const vars = Object.entries(process.env).filter(([k]) => k.startsWith('WEBVIEW2'));
  out.push(`WEBVIEW2_* in the test environment: ${vars.map(([k, v]) => `${k}=${v}`).join(' ') || '(none)'}`);
  return out.join('\n');
}

export type Language = 'fr' | 'en';

/** What the helpers below click and read, in each language of the interface. */
const LABELS = {
  fr: {
    addProject: /Ajouter un projet/,
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

/** The usage endpoint the app asks: a closed port, unless a test gives its own (`appEnv`). */
const NO_USAGE_API = 'http://127.0.0.1:9/api/oauth/usage';
/** The store the app reads: a closed port, unless a test starts the fake one (tests/fixtures/store/fake-store.mjs). */
const NO_STORE = 'http://127.0.0.1:9';

/**
 * The app's window is kept out of sight (off every screen, never focused, no taskbar button): the tests run while the user
 * works, without taking their keyboard. `ESCOUADE_E2E_SHOW=1` shows it, in front, to watch a test.
 */
export const SHOW = !!process.env.ESCOUADE_E2E_SHOW && process.env.ESCOUADE_E2E_SHOW !== '0';

/**
 * WebView2 draws and runs its timers in full even though its window is not seen: Chromium slows down a window it takes for
 * hidden or covered. Off the screens, the window was drawn without them in short tests (ESC-126): they keep it so in the long
 * ones. The same in ci.yml, where they reach WebView2 by the machine policy.
 */
const RENDER_UNSEEN = '--disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-background-timer-throttling';

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

export const test = base.extend<{
  app: App;
  appEnv: Record<string, string>;
  appArgs: string[];
  language: Language;
  preStart: { run: (root: string) => Record<string, unknown> | void };
}>({
  // Variables for the app on top of the fixture's, which they replace (`test.use({ appEnv: … })`):
  // a fake usage endpoint, Principal's folder with a fake-limit in it.
  appEnv: [{}, { option: true }],
  // The arguments of the first launch (`test.use({ appArgs: ['escouade://plugin/<id>'] })`, STORE-11): a restart has none.
  appArgs: [[], { option: true }],
  // The language of the interface the app starts in (`test.use({ language: 'en' })`): French, whatever the machine's.
  language: ['fr', { option: true }],
  // `run` is called before the app starts, with the test's folder (`test.use({ preStart: { run: (root) => … } })`): files it
  // may put there (a second Claude account's folder and sign-in), and the settings to start with, merged into settings.json.
  // (An object: Playwright reads a function given as an option for a fixture.)
  preStart: [{ run: () => ({}) }, { option: true }],
  app: async ({ appEnv, appArgs, language, preStart }, use, testInfo) => {
    if (!fs.existsSync(EXE)) throw new Error(`build the app first: npx tauri build --debug --no-bundle (missing ${EXE})`);
    // CI runs elevated, where WebView2 ignores WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: the workflow
    // opens a fixed port through the machine policy instead (tests run one at a time).
    const port = process.env.ESCOUADE_E2E_CDP_PORT
      ? Number(process.env.ESCOUADE_E2E_CDP_PORT)
      : 9400 + (testInfo.workerIndex * 50 + Math.floor(Math.random() * 50));
    // The instance of the test before may still be dying, and answering on this port (always the same on CI):
    // connect() would attach to it. Wait for it to be gone.
    if (!(await waitForPortToClose(port))) {
      throw new Error(`something still listens on port ${port} after 30 s: the app of the test before is not gone`);
    }
    const root = testRoot();
    const data = path.join(root, 'data');
    fs.mkdirSync(data);
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
      WEBVIEW2_USER_DATA_FOLDER: path.join(root, 'webview'),
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} ${RENDER_UNSEEN}`,
      ESCOUADE_HIDDEN_WINDOW: SHOW ? undefined : '1',
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
      ESCOUADE_USAGE_API: NO_USAGE_API,
      // Nor store.escouade.dev (STORE-9): a closed port, unless a test starts the fake store and names it.
      ESCOUADE_STORE_URL: process.env.ESCOUADE_STORE_URL ?? NO_STORE,
      ESCOUADE_STORE_MIRROR_URL: process.env.ESCOUADE_STORE_MIRROR_URL ?? NO_STORE,
      ESCOUADE_STORE_TEST_KEY: process.env.ESCOUADE_STORE_TEST_KEY,
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
      ...appEnv,
    };
    // The launch the test is on (the last one, after a restart): the one the end of the test must kill.
    let instance = startApp(env, stderr, appArgs);
    // Attaches Playwright to the WebView2 of a launch.
    const attach = async (launch: Instance): Promise<Page> => {
      try {
        const { browser, page } = await connect(port, () => launch.stopped);
        launch.browser = browser;
        return page;
      } catch (e) {
        const appLog = tail(path.join(data, 'app.log'));
        const errLog = tail(stderr);
        throw new Error(
          `${(e as Error).message}\n--- app.log ---\n${appLog}\n` +
            (errLog === appLog ? '' : `--- stderr ---\n${errLog}\n`) +
            `--- WebView2 ---\n${webviewDiagnostics(root)}`,
        );
      }
    };
    try {
      const page = await attach(instance);
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
        launches,
        launchedAt: () => instance.at,
        restart: async () => {
          // The app saves what it shows a moment after a change: not killed in the middle of it.
          await new Promise((r) => setTimeout(r, SAVE_MS));
          await killApp(instance);
          if (!(await waitForPortToClose(port))) {
            throw new Error(`something still listens on port ${port} 30 s after the app was killed: it cannot be started again`);
          }
          instance = startApp(env, stderr);
          app.page = await attach(instance);
          return app.page;
        },
        reconnect: async () => {
          await instance.browser?.close().catch(() => {});
          app.page = await attach(instance);
          return app.page;
        },
        restartAs: (other: string) => {
          env.ESCOUADE_DATA_DIR = other;
          return app.restart();
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
        return `--- page ---\n${shown}\n--- app (${instance.stopped ?? 'running'}) ---\n${tail(path.join(data, 'app.log'), 25)}`;
      };
      try {
        await expect(page.getByRole('button', { name: LABELS[language].addProject }).first()).toBeVisible();
      } catch (e) {
        throw new Error(`${(e as Error).message.split('\n')[0]}\n${await state()}`);
      }
      await use(app);
      if (testInfo.status !== testInfo.expectedStatus) throw new Error(`the app after the failure\n${await state()}`);
    } finally {
      await killApp(instance);
      // The next test must not find this instance still answering on the port (it waits for that too, before it
      // starts; a test that passed does not fail for it).
      if (!(await waitForPortToClose(port))) console.warn(`port ${port} is still taken 30 s after the app was killed`);
      // Windows releases the handles of killed processes (a terminal's shell sits in the repo)
      // with a delay: a temporary folder left behind must not fail the test.
      try {
        if (!FIXED_ROOT) fs.rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
      } catch (e) {
        console.warn(`could not remove ${root}: ${(e as Error).message}`);
      }
    }
  },
});

/**
 * Opens « Nouveau projet » from « + » of the projects' rail (ESC-158), through its menu when projects were closed (it then
 * lists them first): « Ajouter un projet… ».
 */
export async function openNewProject(page: Page, lang: Language = 'fr') {
  const words = LABELS[lang];
  await page.getByRole('button', { name: words.addProject }).first().click();
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

export async function send(page: Page, text: string) {
  const box = page.getByRole('textbox').last();
  await box.fill(text);
  await box.press('Enter');
}

export { expect, FAKE, FAKE_AGY, FAKE_CODEX, FAKE_COPILOT, FAKE_CURSOR, FAKE_GEMINI, FAKE_OPENCODE, FAKE_VIBE };
