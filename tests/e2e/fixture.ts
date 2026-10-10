// Launches the real app on a throwaway data folder, with the fake `claude` CLI, and connects
// Playwright to its WebView2 through the DevTools protocol.

import { test as base, chromium, expect, type Browser, type Page } from '@playwright/test';
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const EXE = process.env.ESCOUADE_E2E_EXE ?? path.join(ROOT, 'src-tauri', 'target', 'debug', 'escouade.exe');
const FAKE = path.join(ROOT, 'tests', 'fixtures', 'fake-claude.cmd');

export interface App {
  page: Page;
  /** A git repository with one commit, ready to be added as a project. */
  repo: string;
  data: string;
  /** The test's folder: the app's data (`data`), the repository and Principal's Claude Code folder (`claude`) are in it. */
  root: string;
  /** Launches of the fake CLI: argv, cwd, the proxy and the Claude Code folder (`CLAUDE_CONFIG_DIR`) it received. */
  launches: () => { argv: string[]; cwd: string; proxy: string | null; configDir: string | null }[];
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

/** Last lines of a log file, for failure messages (CI annotations are all we get there). */
function tail(file: string, lines = 40): string {
  if (!fs.existsSync(file)) return `(no ${path.basename(file)})`;
  const text = fs.readFileSync(file, 'utf8').trimEnd();
  return text ? text.split('\n').slice(-lines).join('\n') : `(${path.basename(file)} is empty)`;
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
    worktrees: 'Un worktree git par agent',
    create: 'Créer le projet',
  },
  en: {
    addProject: /Add a project/,
    newProject: 'New project',
    path: 'C:\\path\\to\\the\\project',
    repo: /Git repository detected · branch main · clean/,
    firstAgent: 'Create a first agent',
    worktrees: 'One git worktree per agent',
    create: 'Create project',
  },
} as const;

/** The usage endpoint the app asks: a closed port, unless a test gives its own (`appEnv`). */
const NO_USAGE_API = 'http://127.0.0.1:9/api/oauth/usage';

export const test = base.extend<{
  app: App;
  appEnv: Record<string, string>;
  language: Language;
  preStart: { run: (root: string) => Record<string, unknown> | void };
}>({
  // Variables for the app on top of the fixture's, which they replace (`test.use({ appEnv: … })`):
  // a fake usage endpoint, Principal's folder with a fake-limit in it.
  appEnv: [{}, { option: true }],
  // The language of the interface the app starts in (`test.use({ language: 'en' })`): French, whatever the machine's.
  language: ['fr', { option: true }],
  // `run` is called before the app starts, with the test's folder (`test.use({ preStart: { run: (root) => … } })`): files it
  // may put there (a second Claude account's folder and sign-in), and the settings to start with, merged into settings.json.
  // (An object: Playwright reads a function given as an option for a fixture.)
  preStart: [{ run: () => ({}) }, { option: true }],
  app: async ({ appEnv, language, preStart }, use, testInfo) => {
    if (!fs.existsSync(EXE)) throw new Error(`build the app first: npx tauri build --debug --no-bundle (missing ${EXE})`);
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-e2e-'));
    const data = path.join(root, 'data');
    fs.mkdirSync(data);
    // French unless a test asks otherwise, whatever the machine's language (CI runners are English): the tests read the
    // French interface.
    fs.writeFileSync(
      path.join(data, 'settings.json'),
      JSON.stringify({
        claudePath: FAKE,
        sound: false,
        osNotifications: false,
        idleStopMinutes: 0,
        language,
        ...(preStart.run(root) ?? {}),
      }),
    );
    const repo = makeRepo(root);
    const log = path.join(root, 'fake-claude.jsonl');
    // CI runs elevated, where WebView2 ignores WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: the workflow
    // opens a fixed port through the machine policy instead (tests run one at a time).
    const port = process.env.ESCOUADE_E2E_CDP_PORT
      ? Number(process.env.ESCOUADE_E2E_CDP_PORT)
      : 9400 + (testInfo.workerIndex * 50 + Math.floor(Math.random() * 50));
    const stderr = path.join(root, 'app-stderr.log');
    const errFd = fs.openSync(stderr, 'w');
    const child: ChildProcess = spawn(EXE, [], {
      env: {
        ...process.env,
        ESCOUADE_DATA_DIR: data,
        FAKE_CLAUDE_LOG: log,
        // Sent "from claude.ai" to an agent once its Remote Control is on.
        FAKE_CLAUDE_REMOTE_MESSAGE: 'Message depuis le téléphone',
        WEBVIEW2_USER_DATA_FOLDER: path.join(root, 'webview'),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
        // Never the machine user's own sign-in nor Anthropic's endpoint: Principal's Claude Code
        // folder is a test one (with no sign-in in it), and the quota is asked of nobody.
        CLAUDE_CONFIG_DIR: path.join(root, 'claude'),
        CLAUDE_SECURESTORAGE_CONFIG_DIR: undefined,
        ESCOUADE_USAGE_API: NO_USAGE_API,
        ...appEnv,
      },
      stdio: ['ignore', 'ignore', errFd],
    });
    fs.closeSync(errFd);
    let stopped: string | null = null;
    child.on('exit', (code, signal) => {
      stopped = signal ? `killed by ${signal}` : `exit code ${code} (0x${((code ?? 0) >>> 0).toString(16)})`;
    });
    child.on('error', (e) => (stopped = `could not start: ${e.message}`));
    let browser: Browser | null = null;
    try {
      let page: Page;
      try {
        ({ browser, page } = await connect(port, () => stopped));
      } catch (e) {
        const appLog = tail(path.join(data, 'app.log'));
        const errLog = tail(stderr);
        throw new Error(
          `${(e as Error).message}\n--- app.log ---\n${appLog}\n` +
            (errLog === appLog ? '' : `--- stderr ---\n${errLog}\n`) +
            `--- WebView2 ---\n${webviewDiagnostics(root)}`,
        );
      }
      // What the app showed and logged, for a failure on CI (annotations are all we get there).
      const state = async () => {
        const shown = await Promise.race([
          page
            .evaluate(() =>
              JSON.stringify({ ready: document.readyState, url: location.href, text: document.body?.innerText.slice(0, 300) }),
            )
            .catch((e) => `page unreachable: ${(e as Error).message.split('\n')[0]}`),
          new Promise<string>((r) => setTimeout(() => r('page not answering'), 5000)),
        ]);
        return `--- page ---\n${shown}\n--- app (${stopped ?? 'running'}) ---\n${tail(path.join(data, 'app.log'), 25)}`;
      };
      try {
        await expect(page.getByRole('button', { name: LABELS[language].addProject }).first()).toBeVisible();
      } catch (e) {
        throw new Error(`${(e as Error).message.split('\n')[0]}\n${await state()}`);
      }
      const launches = () =>
        fs.existsSync(log)
          ? fs
              .readFileSync(log, 'utf8')
              .trim()
              .split('\n')
              .filter(Boolean)
              .map((l) => JSON.parse(l))
          : [];
      await use({ page, repo, data, root, launches });
      if (testInfo.status !== testInfo.expectedStatus) throw new Error(`the app after the failure\n${await state()}`);
    } finally {
      await browser?.close().catch(() => {});
      if (child.pid) {
        try {
          execFileSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
        } catch {
          // already gone
        }
      }
      // Windows releases the handles of killed processes (a terminal's shell sits in the repo)
      // with a delay: a temporary folder left behind must not fail the test.
      try {
        fs.rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 });
      } catch (e) {
        console.warn(`could not remove ${root}: ${(e as Error).message}`);
      }
    }
  },
});

/**
 * Adds `repo` as a project through the "Nouveau projet" dialog (`lang: 'en'`: "New project"), with a first agent.
 * The dialog is read in the language of the interface the test started in, French unless it says so.
 */
export async function addProject(
  page: Page,
  repo: string,
  opts: { worktrees?: boolean; name?: string; firstAgent?: boolean; lang?: Language } = {},
) {
  const words = LABELS[opts.lang ?? 'fr'];
  await page.getByRole('button', { name: words.addProject }).first().click();
  const dialog = page.getByRole('dialog', { name: words.newProject });
  await dialog.getByPlaceholder(words.path).fill(repo);
  await expect(dialog.getByText(words.repo)).toBeVisible();
  if (opts.name) await dialog.locator('input').nth(1).fill(opts.name);
  if (opts.firstAgent === false) await dialog.getByRole('switch', { name: words.firstAgent }).click();
  if (opts.worktrees) await dialog.getByRole('switch', { name: words.worktrees }).click();
  await dialog.getByRole('button', { name: words.create }).click();
  await expect(dialog).toBeHidden();
}

export async function send(page: Page, text: string) {
  const box = page.getByRole('textbox').last();
  await box.fill(text);
  await box.press('Enter');
}

export { expect };
