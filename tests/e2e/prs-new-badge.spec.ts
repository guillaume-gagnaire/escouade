// The PR tab's badge of the new pull requests (ESC-403) on the real app, against the fake GitHub of
// tests/fixtures/fake-trackers.mjs: the list of the project on screen is read in the background (every 2 minutes,
// `ESCOUADE_PR_WATCH_EVERY_MS` here); the first reading counts nothing; a pull request opened on the host then shows « 1 »
// on the tab, read out « PR, 1 nouvelle », in both themes; opening the tab takes it away and marks the row « Nouvelle »
// until the tab is opened again.

import { execFileSync } from 'node:child_process';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** A call to the fake host, once more when a connection kept alive from the spec before was reset. */
async function call(url: string, init?: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch {
    return fetch(url, init);
  }
}

/** How many times the app read the project's open pull requests. */
const listReads = async () =>
  ((await (await call(`${github.url}/__writes`)).json()) as { path: string; body?: { operationName?: string } }[]).filter(
    (w) => w.path === '/github/graphql' && w.body?.operationName === 'OpenPulls',
  ).length;

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 1 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
  process.env.ESCOUADE_PR_WATCH_EVERY_MS = '1000';
});

test.beforeEach(async () => {
  await call(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
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

async function pull(head: string, title: string, author: string) {
  const r = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title, body: '', head, base: 'main' }),
  });
  const { number } = (await r.json()) as { number: number };
  await call(`${github.url}/__github/pulls/${number}`, { method: 'POST', body: JSON.stringify({ author }) });
  return number;
}

/** The contrast of the badge's figure on its ground (WCAG), as drawn. */
const contrast = (badge: Locator) =>
  badge.evaluate((el) => {
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 1;
    const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
    const rgb = (c: string) => {
      ctx.clearRect(0, 0, 1, 1);
      ctx.fillStyle = c;
      ctx.fillRect(0, 0, 1, 1);
      return [...ctx.getImageData(0, 0, 1, 1).data.slice(0, 3)];
    };
    const lum = (c: string) => {
      const [r, g, b] = rgb(c).map((v) => {
        const s = v / 255;
        return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const style = getComputedStyle(el);
    const [a, b] = [lum(style.color), lum(style.backgroundColor)].sort((x, y) => y - x);
    return (a + 0.05) / (b + 0.05);
  });

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

test('a pull request come on the host shows on the PR tab until the tab is opened, then is marked in the list', async ({ app }) => {
  // The background reading is offered every 30 s by the window.
  test.setTimeout(240_000);
  const { page } = app;
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  expect(await pull('feat/old', 'Une PR d’avant', 'bob')).toBe(101);

  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);
  const tab = page.locator('.side .seg').getByRole('button', { name: /^PR/ });
  await expect(tab).toBeVisible();

  // The first reading of the project, made in the background: #101 already there counts nothing.
  await expect.poll(listReads, { timeout: 90_000 }).toBeGreaterThan(0);
  const badge = tab.getByTestId('prs-new');
  await expect(badge).toHaveCount(0);
  await expect(tab).toHaveAccessibleName('PR');

  // A pull request opened on the host: the next background reading shows it, the tab left closed.
  expect(await pull('feat/new', 'Une PR nouvelle', 'marta')).toBe(102);
  await expect(badge).toHaveText('1', { timeout: 90_000 });
  await expect(tab).toHaveAccessibleName('PR, 1 nouvelle');
  await expect(page.locator('main.prs')).toHaveCount(0);
  // It fits the sidebar's line, and reads in both themes.
  expect(await page.locator('.side .seg').evaluate((el) => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    expect(await contrast(badge), t).toBeGreaterThanOrEqual(4.5);
    if (SHOTS) await page.locator('.side').screenshot({ path: path.join(SHOTS, `prs-new-badge-${t}.png`) });
  }
  await theme(page, 'dark');

  // Opening the tab takes it away; the new one is marked in the list, the one before is not.
  await tab.click();
  await expect(badge).toHaveCount(0);
  await expect(tab).toHaveAccessibleName('PR');
  const rows = page.locator('main.prs').getByRole('listitem');
  await expect(rows).toHaveCount(2);
  await expect(rows.filter({ hasText: '#102' }).getByText('Nouvelle', { exact: true })).toBeVisible();
  await expect(rows.filter({ hasText: '#101' }).getByText('Nouvelle', { exact: true })).toHaveCount(0);
  if (SHOTS) await page.screenshot({ path: path.join(SHOTS, 'prs-new-list.png') });

  // Until the tab is opened again.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await tab.click();
  await expect(rows).toHaveCount(2);
  await expect(page.locator('main.prs').getByText('Nouvelle', { exact: true })).toHaveCount(0);
});
