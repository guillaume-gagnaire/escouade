// The group « Pull requests » of the overview (ESC-68) on the real app, against the fake GitHub of
// tests/fixtures/fake-trackers.mjs: what to fix, to merge and to review, read from the project's list, and a line that
// opens the project's pull requests. Then each notification of the code hosts, ticked one by one in the settings.

import { execFileSync } from 'node:child_process';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

async function call(url: string, init?: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch {
    return fetch(url, init);
  }
}

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 2 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
});

test.beforeEach(async () => {
  await call(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
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

async function pull(head: string, title: string, over: Record<string, unknown>) {
  const r = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title, body: '', head, base: 'main' }),
  });
  const { number } = (await r.json()) as { number: number };
  await call(`${github.url}/__github/pulls/${number}`, { method: 'POST', body: JSON.stringify(over) });
  return number;
}

test('the overview says which pull requests wait for the user, and each news is set on its own', async ({ app }) => {
  const { page } = app;
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');

  const hour = (h: number) => new Date(Date.now() - h * 3600_000).toISOString();
  const fix = await pull('fix/tz', 'Corrige le fuseau des échéances', {
    author: 'ada',
    checks: 'FAILURE',
    mergeable: 'CONFLICTING',
    changesRequested: true,
    threads: 2,
    createdAt: hour(50),
  });
  const merge = await pull('feat/csv', 'Export CSV des tickets', { author: 'ada', checks: 'SUCCESS', approvals: 2, createdAt: hour(5) });
  const review = await pull('i18n-es', 'Traductions espagnoles', {
    author: 'marta',
    checks: 'SUCCESS',
    reviewers: ['ada'],
    createdAt: hour(20),
  });
  // Neither the user's nor asked of them: it stays in the view « Pull requests ».
  await pull('feat/other', 'Le travail de quelqu’un d’autre', { author: 'bob', checks: 'FAILURE', createdAt: hour(2) });

  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);

  await page.keyboard.press('Control+Shift+A');
  const view = page.locator('main.overview');
  const group = view.locator('section').filter({ has: page.getByRole('heading', { name: /^Pull requests/ }) });
  await expect(group.getByRole('heading', { level: 3 })).toHaveText(['À corriger1', 'À merger1', 'Relectures demandées1']);
  const line = (n: number) => group.locator(`li[data-pr="${n}"]`);
  await expect(group.locator('li[data-pr]')).toHaveCount(3);
  await expect(line(fix)).toContainText('Corrige le fuseau');
  await expect(line(fix)).toContainText('Conflit');
  await expect(line(merge)).toContainText('2 approbations');
  await expect(line(review)).toContainText('Ta relecture');
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'esc-68-overview.png') });

  // Nothing is lost at the width of the CI, in the narrower columns.
  const overflows = await group.locator('.row.pr .line').evaluateAll((rows) => rows.some((r) => r.scrollWidth > r.clientWidth + 1));
  expect(overflows).toBe(false);

  // A line with no agent opens the project's pull requests.
  await line(review).click();
  await expect(page.locator('main.prs')).toBeVisible();

  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Notifications');
  await expect(settings.getByRole('checkbox', { name: 'Relecture demandée' })).toBeChecked();
  for (const name of [
    'CI en échec',
    'Changements demandés',
    'PR approuvée',
    'PR mergée',
    'Conflit',
    'Déploiement en production réussi',
    'Déploiement en échec',
  ])
    await expect(settings.getByRole('checkbox', { name })).toBeVisible();
  if (SHOTS) await settings.screenshot({ path: path.join(SHOTS, 'esc-68-reglages.png') });
  await settings.getByText('PR approuvée').click();
  await expect(settings.getByRole('checkbox', { name: 'PR approuvée' })).not.toBeChecked();
  await expect(settings.getByRole('checkbox', { name: 'PR mergée' })).toBeChecked();
});
