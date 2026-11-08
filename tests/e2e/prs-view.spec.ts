// The project's « Pull requests » view (ESC-65) on the real app, against the fake GitHub of tests/fixtures/fake-trackers.mjs:
// beside the Kanban, the open pull requests with their CI, reviews, conflict, fork and age, the three filters; then
// « Lancer un agent » on a fork's pull request (fetched by refs/pull/<n>/head into pr-<n>) and, with the keyboard, on one
// of the repository's own branches: an agent in a worktree on each. Nothing is written on the host.

import { execFileSync } from 'node:child_process';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const writes = async (): Promise<{ method: string; path: string }[]> => (await call(`${github.url}/__writes`)).json();
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/**
 * A call to the fake host, once more when its connection was reset: the spec run before this one had its own fake on the
 * same port, and a connection kept alive to it may be reused once.
 */
async function call(url: string, init?: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch {
    return fetch(url, init);
  }
}

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 1 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
  process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
});

test.beforeEach(async () => {
  await call(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
  delete process.env.ESCOUADE_PR_READ_EVERY_MS;
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

/** A pull request on the fake GitHub: opened, then given what the test wants of it. */
async function pull(head: string, title: string, over: Record<string, unknown>) {
  const r = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title, body: '', head, base: 'main' }),
  });
  const { number } = (await r.json()) as { number: number };
  await call(`${github.url}/__github/pulls/${number}`, { method: 'POST', body: JSON.stringify(over) });
  return number;
}

/** The worktrees of the repository other than its folder: branch → commit. */
function worktrees(repo: string): Record<string, string> {
  const out: Record<string, string> = {};
  let head = '';
  for (const line of git(repo, 'worktree', 'list', '--porcelain').split('\n')) {
    if (line.startsWith('HEAD ')) head = line.slice(5);
    if (line.startsWith('branch ')) out[line.slice('branch refs/heads/'.length)] = head;
  }
  return out;
}

test('the project’s pull requests, their filters, and an agent started on a fork’s and on its own branch', async ({ app }) => {
  // Two agents, each on a worktree of its own: longer than the default.
  test.setTimeout(300_000);
  const { page } = app;
  // github.com/acme/demo, its pushes in a bare repository: someone's branch, and a fork's pull request kept as refs/pull/102/head.
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  git(app.repo, 'checkout', '-qb', 'feat/cursor');
  git(app.repo, 'commit', '-q', '--allow-empty', '-m', 'feat: pagination par curseur');
  git(app.repo, 'push', '-q', 'origin', 'feat/cursor');
  const own = git(app.repo, 'rev-parse', 'HEAD');
  git(app.repo, 'checkout', '-qb', 'fork-work', 'main');
  git(app.repo, 'commit', '-q', '--allow-empty', '-m', 'i18n: espagnol');
  git(app.repo, 'push', '-q', 'origin', 'fork-work:refs/pull/102/head');
  const forked = git(app.repo, 'rev-parse', 'HEAD');
  git(app.repo, 'checkout', '-q', 'main');
  git(app.repo, 'branch', '-qD', 'feat/cursor', 'fork-work');

  const hour = (h: number) => new Date(Date.now() - h * 3600_000).toISOString();
  expect(await pull('feat/cursor', 'Pagination par curseur', { author: 'ada', checks: 'PENDING', createdAt: hour(3) })).toBe(101);
  await pull('i18n-es', 'Traductions espagnoles', {
    author: 'marta',
    headRepo: 'marta/demo',
    checks: 'SUCCESS',
    approvals: 1,
    reviewers: ['ada'],
    createdAt: hour(20),
  });
  await pull('fix/tz', 'Corrige le fuseau des échéances', {
    author: 'bob',
    checks: 'FAILURE',
    mergeable: 'CONFLICTING',
    changesRequested: true,
    threads: 2,
    createdAt: hour(50),
  });

  const seeded = (await writes()).length;
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);

  // Beside the Kanban, in the sidebar.
  const tab = page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true });
  await tab.click();
  await expect(tab).toHaveAttribute('aria-pressed', 'true');
  // The four views fit the sidebar's line, at its default width and at its narrowest.
  const fits = () => page.locator('.side .seg').evaluate((el) => el.scrollWidth <= el.clientWidth + 1);
  expect(await fits()).toBe(true);
  const view = page.locator('main.prs');
  await expect(view.getByRole('heading', { name: 'Pull requests' })).toBeVisible();
  const rows = view.getByRole('listitem');
  await expect(rows).toHaveCount(3);
  const row = (n: number) => rows.filter({ hasText: `#${n}` });
  await expect(row(101)).toContainText('CI en cours');
  await expect(row(102)).toContainText('fork marta');
  await expect(row(102)).toContainText('1 approbation');
  await expect(row(102)).toContainText('Ta relecture');
  await expect(row(103)).toContainText('Conflit');
  await expect(row(103)).toContainText('Changements demandés');
  await expect(row(103)).toContainText('2 fils ouverts');
  await expect(row(103)).toContainText('CI en échec');
  await expect(row(103)).toContainText('2 j');
  await expect(row(102)).toContainText('20 h');

  // The three filters: @ada's, those whose review is asked of her, all.
  const filters = view.getByRole('tablist', { name: 'Quelles PR' });
  await filters.getByRole('tab', { name: 'Les miennes 1' }).click();
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText('#101');
  await filters.getByRole('tab', { name: 'Relecture demandée 1' }).click();
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText('#102');
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'prs-view-review.png') });
  await filters.getByRole('tab', { name: 'Toutes 3' }).click();
  await expect(rows).toHaveCount(3);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'prs-view.png') });

  // « Lancer un agent » on the fork's: a worktree on pr-102, at the fork's commit.
  await row(102).hover();
  await row(102).getByRole('button', { name: 'Lancer un agent' }).click();
  const cards = page.locator('.side .card');
  await expect(cards).toHaveCount(1, { timeout: 60_000 });
  await expect.poll(() => worktrees(app.repo)['pr-102']).toBe(forked);
  await expect(view).toHaveCount(0);

  // Back to the view: the agent is on it, « Ouvrir l'agent » in place of another.
  await tab.click();
  await expect(row(102).locator('.tag.agent')).toBeVisible();
  await row(102).hover();
  await expect(row(102).getByRole('button', { name: 'Ouvrir l’agent' })).toBeVisible();

  // With the keyboard: from the first row (the most recent, #103), ↓ then End to the last, #101, the repository's own branch; Entrée.
  await row(103)
    .getByRole('button', { name: /^PR #103/ })
    .focus();
  await page.keyboard.press('ArrowDown');
  await expect(row(102).getByRole('button', { name: /^PR #102/ })).toBeFocused();
  await page.keyboard.press('End');
  await expect(row(101).getByRole('button', { name: /^PR #101/ })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(cards).toHaveCount(2, { timeout: 60_000 });
  await expect.poll(() => worktrees(app.repo)['feat/cursor']).toBe(own);

  // Read only: nothing written on the host.
  expect(
    (await writes()).slice(seeded).filter((w) => w.path.startsWith('/github/') && w.path !== '/github/graphql' && w.method !== 'GET'),
  ).toEqual([]);
});
