// The tab « Fichiers » of a pull request's view (ESC-467, PR-3 of the PR in Escouade) on the real app, against the fake
// GitHub of tests/fixtures/fake-trackers.mjs: its diff from git (the pull request's refs fetched by the app), its tree, a
// generated file folded until « Afficher », « Vu » marked and lost when a pushed commit changes the file, one commit's
// files, and « Ouvrir dans l'éditeur » at a line in the VS Code view. Nothing is written on the host.
// ESCOUADE_E2E_SHOTS=<folder>: its screenshots, in both themes.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';
import { editorLines, vscodeTabs, workbench } from './vscode';

let github: { url: string; close: () => Promise<void> };
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

async function call(url: string, init?: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch {
    return fetch(url, init);
  }
}

type Asked = { method: string; path: string };
const writes = async () =>
  ((await (await call(`${github.url}/__writes`)).json()) as Asked[]).filter((w) => !w.path.endsWith('/graphql')).map((w) => w.path);

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 6 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
  process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
  process.env.ESCOUADE_PR_WATCH_EVERY_MS = '1000';
});

test.beforeEach(async () => {
  await call(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
  delete process.env.ESCOUADE_PR_READ_EVERY_MS;
  delete process.env.ESCOUADE_PR_WATCH_EVERY_MS;
});

async function connectGithub(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Services connectés');
  await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
  const form = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
  await form.getByLabel(/Jeton/).fill('ghp_jeton');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText('Connecté · @ada').first()).toBeVisible();
  await closeSettings(settings);
  await expect(settings).toHaveCount(0);
}

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const size = process.env.ESCOUADE_E2E_VIEWPORT ? `-${process.env.ESCOUADE_E2E_VIEWPORT}` : '';
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(SHOTS, `${name}${size}-${t}.png`) });
  }
  await theme(page, 'dark');
}

const commit = (repo: string, message: string, files: Record<string, string>) => {
  for (const [p, text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(repo, p)), { recursive: true });
    fs.writeFileSync(path.join(repo, p), text);
  }
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', message);
  return git(repo, 'rev-parse', 'HEAD');
};

const LINK_V1 = [
  'import { randomBytes } from "node:crypto";',
  '',
  'export async function sendMagicLink(email: string) {',
  '  const token = randomBytes(32).toString("base64url");',
  '  return mailer.send(email, link(token));',
  '}',
  '',
].join('\n');

test('the tab « Fichiers »: the diff from git, its tree, « Vu » lost by a new commit, a commit’s files, the editor', async ({ app }) => {
  test.setTimeout(420_000);
  const { page } = app;
  // github.com/acme/demo, whose pushes land in a bare repository of the test.
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  // The pull request's branch, checked out in the project's folder: « Ouvrir dans l'éditeur » opens it there.
  git(app.repo, 'checkout', '-qb', 'feat/lien');
  const first = commit(app.repo, 'feat: lien magique', {
    'src/auth/magic-link.ts': LINK_V1,
    'package-lock.json': JSON.stringify({ name: 'demo', lockfileVersion: 3, packages: {} }, null, 2) + '\n',
  });
  const second = commit(app.repo, 'feat: la session', { 'src/auth/session.ts': 'export const session = { ttl: 600 };\n' });
  const push = () => git(app.repo, 'push', '-qf', 'origin', 'feat/lien', 'feat/lien:refs/pull/101/head');
  push();
  const created = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title: 'Connexion par lien magique', body: 'Le lien.', head: 'feat/lien', base: 'main' }),
  });
  const { number: n } = (await created.json()) as { number: number };
  const commits = [
    { sha: first, message: 'feat: lien magique', author: 'bob' },
    { sha: second, message: 'feat: la session', author: 'bob' },
  ];
  await call(`${github.url}/__github/pulls/${n}`, { method: 'POST', body: JSON.stringify({ author: 'bob', commits }) });

  // What the app writes on the host from here: nothing.
  const before = await writes();
  // The project is added on main, then its folder goes to the pull request's branch.
  git(app.repo, 'checkout', '-q', 'main');
  await addProject(page, app.repo, { firstAgent: false });
  git(app.repo, 'checkout', '-q', 'feat/lien');
  await connectGithub(page);
  await page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true }).click();
  const row = page
    .locator('main.prs')
    .getByRole('listitem')
    .filter({ hasText: `#${n}` });
  await row.getByRole('button', { name: new RegExp(`PR #${n}`) }).click();
  const view = page.getByRole('main', { name: `La PR #${n}` });
  await expect(view.getByRole('tabpanel')).toBeVisible({ timeout: 60_000 });
  await view.getByRole('tab', { name: /^Fichiers/ }).click();

  // The diff, git's: three files, the lockfile folded as generated.
  await expect(view.getByRole('tab', { name: 'Fichiers, 0/3 vus' })).toBeVisible({ timeout: 30_000 });
  const link = view.getByRole('region', { name: 'src/auth/magic-link.ts' });
  await expect(link.getByRole('table')).toContainText('mailer.send(email, link(token));');
  const lock = view.getByRole('region', { name: 'package-lock.json' });
  await expect(lock).toContainText('Fichier généré, replié');
  // The tree, filtered (it may be folded away at the small size).
  const treeButton = view.getByRole('button', { name: 'Afficher l’arbre des fichiers' });
  if (await treeButton.isVisible()) await treeButton.click();
  const tree = view.getByRole('navigation', { name: 'Fichiers modifiés' });
  await expect(tree).toContainText('session.ts');
  await shots(page, 'pr-files');
  await tree.getByRole('searchbox', { name: 'Filtrer les fichiers' }).fill('session');
  await expect(view.getByRole('region')).toHaveCount(1);
  await tree.getByRole('searchbox').fill('');
  await expect(view.getByRole('region')).toHaveCount(3);
  if (await view.getByRole('button', { name: 'Masquer l’arbre des fichiers' }).isVisible()) {
    const box = await view.boundingBox();
    if ((box?.width ?? 0) < 760) await view.getByRole('button', { name: 'Masquer l’arbre des fichiers' }).click();
  }
  await lock.getByRole('button', { name: 'Afficher' }).click();
  await expect(lock.getByRole('table')).toContainText('lockfileVersion');

  // « Vu »: magic-link.ts and session.ts, kept.
  await link.getByRole('checkbox', { name: 'src/auth/magic-link.ts vu' }).check();
  await view.getByRole('region', { name: 'src/auth/session.ts' }).getByRole('checkbox').check();
  await expect(view.getByRole('tab', { name: 'Fichiers, 2/3 vus' })).toBeVisible();
  await expect(link.getByRole('table')).toHaveCount(0);

  // A commit pushed changes magic-link.ts: the view reads it again, the file is no longer « Vu »; session.ts is.
  const third = commit(app.repo, 'fix: durée du lien', {
    'src/auth/magic-link.ts': LINK_V1 + 'export const TTL_MS = 10 * 60 * 1000;\n',
  });
  push();
  await call(`${github.url}/__github/pulls/${n}`, {
    method: 'POST',
    body: JSON.stringify({
      updatedAt: '2026-11-15T12:00:00Z',
      threads: 1,
      commits: [...commits, { sha: third, message: 'fix: durée du lien', author: 'bob' }],
    }),
  });
  await expect(view.getByRole('tab', { name: 'Fichiers, 1/3 vus' })).toBeVisible({ timeout: 120_000 });
  await expect(link.getByRole('checkbox')).not.toBeChecked();
  await expect(link.getByRole('table')).toContainText('TTL_MS');
  await expect(view.getByRole('region', { name: 'src/auth/session.ts' }).getByRole('checkbox')).toBeChecked();
  await shots(page, 'pr-files-new-commit');

  // One commit: from the tab « Commits », its own files.
  await view.getByRole('tab', { name: /^Commits/ }).click();
  await view.getByRole('button', { name: `Voir les fichiers du commit ${first.slice(0, 7)}` }).click();
  await expect(view.getByRole('combobox', { name: 'Commits montrés' })).toHaveValue(first);
  await expect(view.getByRole('region')).toHaveCount(2, { timeout: 30_000 });
  await expect(view.getByRole('region', { name: 'src/auth/session.ts' })).toHaveCount(0);
  await view.getByRole('combobox', { name: 'Commits montrés' }).selectOption('pr');
  await expect(view.getByRole('region')).toHaveCount(3);

  // « Ouvrir dans l'éditeur » at a line: the VS Code view, on the file.
  await link.getByRole('button', { name: 'Ouvrir src/auth/magic-link.ts à la ligne 7 dans l’éditeur' }).click();
  await expect(workbench(page)).toBeVisible({ timeout: 60_000 });
  await expect.poll(() => vscodeTabs(page), { timeout: 60_000 }).toContain('magic-link.ts');
  await expect(editorLines(page)).toContainText('TTL_MS', { timeout: 30_000 });
  // At the line asked: the cursor's line number is 7.
  await expect(workbench(page).locator('.editor-instance .line-numbers.active-line-number').first()).toHaveText('7', { timeout: 30_000 });
  await shots(page, 'pr-files-editor');
  expect((await writes()).slice(before.length)).toEqual([]);
});

test('a large pull request (300 files, 20 000 lines) stays fluid: only what is near the screen is drawn', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  git(app.repo, 'checkout', '-qb', 'feat/gros');
  const files: Record<string, string> = {};
  for (let f = 0; f < 300; f++) {
    const lines = Array.from({ length: 66 }, (_, i) => `export const v${f}_${i} = ${i} * 2; // ligne ${i + 1}`);
    files[`src/m${String(Math.floor(f / 30)).padStart(2, '0')}/f${String(f).padStart(3, '0')}.ts`] = lines.join('\n') + '\n';
  }
  commit(app.repo, 'feat: 300 fichiers', files);
  git(app.repo, 'push', '-q', 'origin', 'feat/gros', 'feat/gros:refs/pull/101/head');
  const created = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title: 'Trois cents fichiers', body: '', head: 'feat/gros', base: 'main' }),
  });
  const { number: n } = (await created.json()) as { number: number };

  git(app.repo, 'checkout', '-q', 'main');
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  await page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true }).click();
  await page
    .locator('main.prs')
    .getByRole('listitem')
    .filter({ hasText: `#${n}` })
    .getByRole('button', { name: new RegExp(`PR #${n}`) })
    .click();
  const view = page.getByRole('main', { name: `La PR #${n}` });
  await expect(view.getByRole('tabpanel')).toBeVisible({ timeout: 90_000 });
  const start = Date.now();
  await view.getByRole('tab', { name: /^Fichiers/ }).click();
  await expect(view.getByRole('tab', { name: 'Fichiers, 0/300 vus' })).toBeVisible({ timeout: 30_000 });
  await expect(view.getByRole('region')).toHaveCount(300);
  await expect(view.locator('.row').first()).toBeVisible();
  const shown = Date.now() - start;
  // 300 heads, but only the lines near the screen: far from the 20 000.
  const drawn = await view.locator('.row').count();
  expect(drawn).toBeLessThan(2_000);
  expect(shown).toBeLessThan(10_000);
  // The last file, from the tree: there at once, drawn when reached.
  const treeButton = view.getByRole('button', { name: 'Afficher l’arbre des fichiers' });
  if (await treeButton.isVisible()) await treeButton.click();
  const jump = Date.now();
  await view
    .getByRole('navigation', { name: 'Fichiers modifiés' })
    .getByRole('button', { name: /f299\.ts/ })
    .click();
  await expect(view.getByRole('region', { name: 'src/m09/f299.ts' }).getByRole('table')).toContainText('ligne 66');
  expect(Date.now() - jump).toBeLessThan(5_000);
  expect(await view.locator('.row').count()).toBeLessThan(4_000);
  await shots(page, 'pr-files-large');
});
