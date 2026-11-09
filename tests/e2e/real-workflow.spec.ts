// The real trial of the Workflow tracking on each harness (ESC-199, LOOP-9 of ESC-191): the app (debug build, the test's
// data) drives the real Claude Code (chat and Claude Code view), Codex, OpenCode, Copilot CLI and Cursor CLI (chat and
// view) in a trial project of the test's: a small workflow of two doors laid out where the harness keeps its commands
// (`etude` runs `ecrire`), an agent that goes through them, and what the app saw of it: the steps, the history, the files
// written in `.escouade/workflows/` (no absolute path, no date, untouched by a second reading), no commit by the app.
//
// Skipped unless ESCOUADE_E2E_REAL names what to try (`claude,claude-view,codex,opencode,copilot,cursor,cursor-view`):
// each costs real usage. The programs are named as for real-harnesses.spec.ts (ESCOUADE_REAL_CLAUDE / _CODEX / _OPENCODE /
// _COPILOT / _CURSOR). Nothing is read of a token: the configuration files of each harness are saved before (bytes), put
// back after and compared. Run it with TMP and TEMP outside of %TEMP% (Codex), Claude Code's own variables unset.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test, type App, openSettingsPage } from './fixture';
import { keepClaude, keepCodex, keepCopilot } from './real-keep';

const WANTED = new Set((process.env.ESCOUADE_E2E_REAL ?? '').split(',').map((s) => s.trim()));
const OUT = process.env.ESCOUADE_E2E_REAL_OUT ?? path.join(os.tmpdir(), 'escouade-real-workflow');
const HOME = os.homedir();
const MODEL = process.env.REAL_OLLAMA_MODEL ?? 'qwen3-coder:30b-32k';
const LONG = 20 * 60_000;

type Harness = 'claude' | 'codex' | 'opencode' | 'copilot' | 'cursor';
const NAMES: Record<Harness, string> = {
  claude: 'Claude Code',
  codex: 'Codex',
  opencode: 'OpenCode',
  copilot: 'Copilot CLI',
  cursor: 'Cursor CLI',
};
/** Where each harness keeps the doors of a project (commands or prompts), as Escouade reads them. */
const DOORS: Record<Harness, string> = {
  claude: '.claude/commands',
  codex: '.agents/prompts',
  opencode: '.opencode/command',
  copilot: '.github/prompts',
  cursor: '.cursor/commands',
};

let shot = 0;
async function snap(page: Page, name: string) {
  fs.mkdirSync(OUT, { recursive: true });
  await page.screenshot({ path: path.join(OUT, `${String(++shot).padStart(2, '0')}-${name}.png`) }).catch(() => {});
}
const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

/** The two doors of the trial's workflow, in the harness's own folder; committed (the project is added clean). */
function layOut(repo: string, h: Harness) {
  const dir = path.join(repo, DOORS[h]);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(
    path.join(dir, 'etude.md'),
    '---\ndescription: Studies the need\n---\nWrite the single line "etude faite" in the file etude.txt. Then go on with /ecrire.\n',
  );
  fs.writeFileSync(
    path.join(dir, 'ecrire.md'),
    '---\ndescription: Writes the result\n---\nRead etude.txt, then write the single line "ecriture faite" in the file ecrire.txt.\n',
  );
  git(repo, '-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'add', '-A');
  git(repo, '-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qm', 'Workflow of the trial');
}

/** The bytes of files, now, and a function that puts them back (a file that was not there is taken out) and compares. */
function keepFiles(files: string[]) {
  const saved = files.map((f) => [f, fs.existsSync(f) ? fs.readFileSync(f) : null] as const);
  return () => {
    const report: string[] = [];
    for (const [f, bytes] of saved) {
      const now = fs.existsSync(f) ? fs.readFileSync(f) : null;
      const same = (now === null && bytes === null) || (now !== null && bytes !== null && now.equals(bytes));
      if (!same) {
        report.push(path.basename(f));
        if (bytes) fs.writeFileSync(f, bytes);
        else fs.rmSync(f, { force: true });
      }
      const after = fs.existsSync(f) ? fs.readFileSync(f) : null;
      expect((after === null && bytes === null) || (after !== null && bytes !== null && after.equals(bytes)), `${f} as it was`).toBe(true);
    }
    console.log(`config files: ${files.length} checked; changed by the trial and put back: ${report.join(', ') || 'none'}`);
  };
}

/** Takes out of a folder of the user's what names the trial's folders (`escouade-js-tests`). */
function dropOurs(dir: string) {
  if (!fs.existsSync(dir)) return 0;
  let n = 0;
  for (const name of fs.readdirSync(dir)) {
    if (name.toLowerCase().includes('escouade-js-tests')) {
      fs.rmSync(path.join(dir, name), { recursive: true, force: true });
      n++;
    }
  }
  return n;
}

function configFiles(h: Harness): string[] {
  const j = path.join;
  switch (h) {
    case 'claude':
      return [j(HOME, '.claude', 'settings.json')];
    case 'codex':
      return [j(HOME, '.codex', 'config.toml'), j(HOME, '.codex', 'hooks.json')];
    case 'copilot':
      return [j(HOME, '.copilot', 'config.json'), j(HOME, '.copilot', 'mcp-config.json')];
    case 'cursor':
      return ['cli-config.json', 'mcp.json', 'hooks.json'].map((f) => j(HOME, '.cursor', f));
    case 'opencode':
      return [j(HOME, '.config', 'opencode', 'opencode.json')];
  }
}

function realEnv(h: Harness): Record<string, string> {
  const env: Record<string, string | undefined> = {
    CLAUDECODE: undefined,
    CLAUDE_CODE_ENTRYPOINT: undefined,
    CLAUDE_CODE_CHILD_SESSION: undefined,
  };
  if (h === 'claude') env.CLAUDE_CONFIG_DIR = undefined;
  if (h === 'codex') {
    env.ESCOUADE_CODEX = process.env.ESCOUADE_REAL_CODEX;
    env.CODEX_HOME = undefined;
  }
  if (h === 'opencode') {
    env.ESCOUADE_OPENCODE = process.env.ESCOUADE_REAL_OPENCODE;
    for (const d of ['DATA', 'STATE', 'CACHE']) env[`XDG_${d}_HOME`] = path.join(os.tmpdir(), 'opencode-xdg', d.toLowerCase());
  }
  if (h === 'copilot') {
    env.ESCOUADE_COPILOT = process.env.ESCOUADE_REAL_COPILOT;
    for (const k of ['COPILOT_HOME', 'GH_TOKEN', 'GITHUB_TOKEN', 'COPILOT_GITHUB_TOKEN']) env[k] = undefined;
  }
  if (h === 'cursor') {
    env.ESCOUADE_CURSOR = process.env.ESCOUADE_REAL_CURSOR;
    // The user's own sign-in and workspaces (the transcripts the view follows), not the test's.
    env.ESCOUADE_CURSOR_AUTH = undefined;
    env.ESCOUADE_CURSOR_PROJECTS = path.join(HOME, '.cursor', 'projects');
    env.MSYSTEM = undefined;
    env.EXEPATH = undefined;
  }
  return env as Record<string, string>;
}

function realSettings(h: Harness, root: string): Record<string, unknown> {
  const s: Record<string, unknown> = {
    claudeSelfUpdate: false,
    ...(h === 'claude' ? { defaultModel: 'haiku', claudePath: process.env.ESCOUADE_REAL_CLAUDE } : {}),
  };
  if (h === 'opencode') {
    s.writerEngine = 'opencode';
    s.writerModel = `ollama/${MODEL}`;
    const config = path.join(root, 'xdg', 'opencode');
    fs.mkdirSync(config, { recursive: true });
    fs.writeFileSync(
      path.join(config, 'opencode.json'),
      JSON.stringify(
        {
          $schema: 'https://opencode.ai/config.json',
          provider: {
            ollama: {
              npm: '@ai-sdk/openai-compatible',
              name: 'Ollama (local)',
              options: { baseURL: 'http://127.0.0.1:11434/v1' },
              models: { [MODEL]: { name: MODEL, tool_call: true, limit: { context: 32768, output: 8192 } } },
            },
          },
          model: `ollama/${MODEL}`,
          small_model: `ollama/${MODEL}`,
          enabled_providers: ['ollama'],
        },
        null,
        2,
      ),
    );
  }
  return s;
}

async function newAgent(page: Page, h: Harness) {
  const menu = page.getByRole('button', { name: 'Autres façons de créer un agent' });
  const item = page.getByRole('menuitemradio', { name: NAMES[h] });
  if (await menu.isVisible()) {
    await menu.click();
    await expect(page.getByRole('menuitem').first()).toBeVisible();
  }
  if (await item.isVisible()) await item.click();
  else {
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  }
  await expect(page.locator('main.conv header.head')).toContainText(NAMES[h]);
}

const shownCard = (page: Page) => page.locator('.card.sel').first();
async function idle(page: Page, timeout = LONG) {
  await expect.poll(async () => (await shownCard(page).textContent()) ?? '', { timeout, intervals: [2_000] }).not.toMatch(/En cours/);
}

/** A message, its permissions answered as the user would, up to the end of the turn. */
async function turn(page: Page, text: string) {
  // Sent again once if the composer was not ready (the agent was still starting): nothing in the conversation then.
  for (let i = 0; i < 3; i++) {
    await send(page, text);
    if (
      await page
        .locator('main.conv')
        .getByText(text.slice(0, 20))
        .first()
        .waitFor({ timeout: 20_000 })
        .then(
          () => true,
          () => false,
        )
    )
      break;
    console.log('message not in the conversation: sent again');
  }
  await page.waitForTimeout(4_000);
  const started = Date.now();
  let quiet = 0;
  // Idle on two looks in a row (the card says « En cours » while it works).
  while (quiet < 2) {
    expect(Date.now() - started, 'the turn ended').toBeLessThan(LONG);
    const ask = page.getByTestId('permission-pending').first();
    if (await ask.isVisible()) await ask.getByRole('button', { name: 'Autoriser', exact: true }).click();
    const busy = /En cours/.test((await shownCard(page).textContent()) ?? '');
    quiet = busy ? 0 : quiet + 1;
    await page.waitForTimeout(2_500);
  }
}

async function openTab(page: Page) {
  const side = page.locator('aside.git-side');
  if (!(await side.isVisible())) await page.keyboard.press('Control+Shift+B');
  await side.getByRole('tab', { name: /^Workflow/ }).click();
  return side.getByTestId('workflow');
}

/** What the tab says, for the notes. */
async function dump(page: Page, label: string) {
  const tab = await openTab(page);
  const text = ((await tab.textContent()) ?? '').replace(/\s+/g, ' ').trim();
  console.log(`${label}: ${text.slice(0, 1200)}`);
  await snap(page, label);
  return tab;
}

/** The files `.escouade/workflows/*.yml` of the project: their texts, by name. */
function workflowFiles(repo: string): Record<string, string> {
  const dir = path.join(repo, '.escouade', 'workflows');
  if (!fs.existsSync(dir)) return {};
  return Object.fromEntries(fs.readdirSync(dir).map((f) => [f, fs.readFileSync(path.join(dir, f), 'utf8')]));
}

/** The checks of the files written after a real reading (ESC-195). */
function checkFiles(repo: string, commitsBefore: string) {
  const files = workflowFiles(repo);
  const names = Object.keys(files);
  console.log(`files: ${names.join(', ')}`);
  expect(names.length, 'a file written for the workflow read').toBeGreaterThan(0);
  for (const [name, text] of Object.entries(files)) {
    // No absolute path (a drive, a root, the trial's folder), no date.
    expect(text, `${name}: no drive path`).not.toMatch(/\b[A-Za-z]:\\|\b[A-Za-z]:\/(?!\/)/);
    expect(text, `${name}: no trial folder`).not.toMatch(/escouade-js-tests|\/Users\/|\\Users\\/);
    expect(text, `${name}: no date`).not.toMatch(/\b20\d\d-\d\d-\d\d\b|\d\d:\d\d:\d\d/);
  }
  // Never committed by the app.
  expect(git(repo, 'log', '--format=%h %s')).toBe(commitsBefore);
  expect(git(repo, 'status', '--porcelain', '--', '.escouade')).toMatch(/\.escouade/);
}

/** « Relire » in Réglages › Projets › Kanban › Workflow, then whether the files were touched (mtime and text). */
async function rereadLeavesFiles(page: Page, repo: string) {
  const before = Object.fromEntries(
    Object.keys(workflowFiles(repo)).map((f) => [f, fs.statSync(path.join(repo, '.escouade', 'workflows', f)).mtimeMs]),
  );
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Kanban');
  const reread = settings.getByRole('button', { name: /^Relire/ });
  if (await reread.isVisible()) {
    await reread.click();
    await page.waitForTimeout(3_000);
  } else console.log('no « Relire » button found');
  await snap(page, 'settings-workflow');
  await page.keyboard.press('Escape');
  for (const [f, m] of Object.entries(before)) {
    expect(fs.statSync(path.join(repo, '.escouade', 'workflows', f)).mtimeMs, `${f} untouched by a second reading`).toBe(m);
  }
}

async function chatTrial(app: App, h: Harness) {
  const { page } = app;
  await page.setViewportSize({ width: 1280, height: 800 });
  layOut(app.repo, h);
  const commits = git(app.repo, 'log', '--format=%h %s');
  await addProject(page, app.repo, { firstAgent: false });
  await newAgent(page, h);
  await snap(page, `${h}-agent`);
  let tab = await dump(page, `${h}-before`);
  await turn(page, '/etude Fais ce que dit cette commande.');
  await dump(page, `${h}-after-etude`);
  await turn(page, '/ecrire Fais ce que dit cette commande.');
  tab = await dump(page, `${h}-after-ecrire`);
  await expect(tab.locator('.wf-step')).toHaveCount(2, { timeout: 30_000 });
  const history = await tab.locator('.wf-hist .ev .what b').allTextContents();
  console.log(`${h}: history: ${history.join(' > ')}`);
  expect(history.length, 'both doors in the history').toBeGreaterThanOrEqual(2);
  checkFiles(app.repo, commits);
  await rereadLeavesFiles(app.page, app.repo);
  // A teammate's file: the doors taken out of the project, the workflow is still followed from its file.
  fs.rmSync(path.join(app.repo, DOORS[h].split('/')[0]), { recursive: true, force: true });
  await rereadLeavesFiles(app.page, app.repo);
  const again = await dump(page, `${h}-no-sources`);
  await expect(again.locator('.wf-step')).toHaveCount(2);
  expect(git(app.repo, 'log', '--format=%h %s')).toBe(commits);
}

/** The view of the harness's own interface: a door typed there, then back, the step seen. */
async function viewTrial(app: App, h: Harness, trust: string[]) {
  const { page } = app;
  await page.setViewportSize({ width: 1280, height: 800 });
  layOut(app.repo, h);
  const commits = git(app.repo, 'log', '--format=%h %s');
  await addProject(page, app.repo, { firstAgent: false });
  await newAgent(page, h);
  await turn(page, 'Réponds seulement par le mot : prêt.');
  await page.getByRole('button', { name: `Vue ${NAMES[h]}` }).click();
  const view = page.locator('main .tui');
  await expect(view.locator('.xterm')).toBeVisible({ timeout: 60_000 });
  await page.waitForTimeout(8_000);
  await view.locator('.xterm').click();
  for (const key of trust) {
    await page.keyboard.press(key);
    await page.waitForTimeout(800);
  }
  await page.waitForTimeout(5_000);
  await snap(page, `${h}-view-ready`);
  await page.keyboard.type('/etude Fais ce que dit cette commande.');
  await page.keyboard.press('Enter');
  const file = path.join(app.repo, 'etude.txt');
  const started = Date.now();
  for (let i = 1; !fs.existsSync(file); i++) {
    expect(Date.now() - started, 'etude.txt written from the view').toBeLessThan(LONG);
    await page.waitForTimeout(3_000);
    if (i % 5 === 0) {
      await snap(page, `${h}-view-wait`);
      await page.keyboard.press('Enter');
    }
  }
  await page.waitForTimeout(10_000);
  await idle(page);
  await snap(page, `${h}-view-done`);
  const tab = await dump(page, `${h}-view-tab`);
  const history = await tab.locator('.wf-hist .ev .what b').allTextContents();
  console.log(`${h}: view history: ${history.join(' > ')}`);
  expect(history.length, 'the door typed in the view is a step').toBeGreaterThanOrEqual(1);
  expect(git(app.repo, 'log', '--format=%h %s')).toBe(commits);
}

const TRUST: Record<Harness, string[]> = {
  claude: ['ArrowDown', 'Enter'],
  codex: ['Enter'],
  opencode: [],
  copilot: ['ArrowDown', 'Enter'],
  cursor: ['Enter'],
};

for (const [key, h, view] of [
  ['claude', 'claude', false],
  ['claude-view', 'claude', true],
  ['codex', 'codex', false],
  ['opencode', 'opencode', false],
  ['copilot', 'copilot', false],
  ['cursor', 'cursor', false],
  ['cursor-view', 'cursor', true],
] as const) {
  test.describe(`the real ${NAMES[h]}${view ? ' (view)' : ''}`, () => {
    test.skip(!WANTED.has(key), `ESCOUADE_E2E_REAL does not name ${key}`);
    test.setTimeout(2 * LONG);
    test.use({
      actionTimeout: 60_000,
      appEnv: WANTED.has(key) ? realEnv(h) : {},
      preStart: { run: (root: string) => (WANTED.has(key) ? realSettings(h, root) : {}) },
    });

    test(`follows the workflow of a trial project`, async ({ app }) => {
      const back = keepFiles(configFiles(h));
      const keeps = [h === 'claude' && keepClaude(), h === 'codex' && keepCodex(), h === 'copilot' && keepCopilot()].filter(
        Boolean,
      ) as (() => void)[];
      try {
        if (view) await viewTrial(app, h, TRUST[h]);
        else await chatTrial(app, h);
      } finally {
        await snap(app.page, `${h}${view ? '-view' : ''}-end`);
        // Sessions and entries of the trial's folders; Cursor's workspaces are the only ones kept here.
        // What Cursor's interface wrote of the trial, kept for the notes (the folders are taken out just after).
        if (h === 'cursor') {
          const root = path.join(HOME, '.cursor', 'projects');
          for (const n of fs.existsSync(root) ? fs.readdirSync(root) : []) {
            if (!n.toLowerCase().includes('escouade-js-tests')) continue;
            fs.cpSync(path.join(root, n, 'agent-transcripts'), path.join(OUT, `transcripts-${n.slice(-12)}`), {
              recursive: true,
              force: true,
            });
          }
        }
        if (h === 'cursor') console.log(`cursor: ${dropOurs(path.join(HOME, '.cursor', 'projects'))} workspaces of the trial taken out`);
        for (const k of keeps) k();
        back();
      }
    });
  });
}
