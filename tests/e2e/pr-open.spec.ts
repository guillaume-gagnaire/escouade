// « Ouvrir dans le navigateur » (ESC-402) on the real app, against the fake GitHub of tests/fixtures/fake-trackers.mjs: every
// button that opens a pull request gives its web page to `open_url`, on a click and by the keyboard, and a refusal of the
// system is said in a toast with its reason. Nothing opens for real: `open_url` is answered in the page.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const PAGE = (n: number) => `https://github.com/acme/demo/pull/${n}`;

/** A call to the fake host, once more when a connection kept alive to the spec before was reset. */
async function call(url: string, init?: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch {
    return fetch(url, init);
  }
}

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 3 : 0);
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

/**
 * What the app asks `open_url` to open, answered in the page (caught on the `fetch` of Tauri's IPC, as links.spec.ts does):
 * no browser opens. `refuse()` makes the next one fail, as the core does when the system cannot open it.
 */
async function watch(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as { esc402Opened: string[]; esc402Refuse: string | null };
    w.esc402Opened = [];
    w.esc402Refuse = null;
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.includes('open_url')) {
        w.esc402Opened.push(JSON.parse(String(init?.body ?? '{}')).url);
        const refused = w.esc402Refuse;
        w.esc402Refuse = null;
        return Promise.resolve(
          new Response(JSON.stringify(refused ?? null), {
            headers: { 'Content-Type': 'application/json', 'Tauri-Response': refused ? 'error' : 'ok' },
          }),
        );
      }
      return fetch(input, init);
    };
  });
  const opened = () => page.evaluate(() => (window as unknown as { esc402Opened: string[] }).esc402Opened);
  const refuse = (why: string) => page.evaluate((w) => ((window as unknown as { esc402Refuse: string | null }).esc402Refuse = w), why);
  return { opened, refuse };
}

/** Clicks it, then presses Enter on it: each must ask `open_url` for `url`. */
async function opensBoth(page: Page, opened: () => Promise<string[]>, button: Locator, url: string, what: string) {
  const before = (await opened()).length;
  await button.click();
  await expect.poll(async () => (await opened()).slice(before), { message: `${what}: click` }).toEqual([url]);
  await button.focus();
  await page.keyboard.press('Enter');
  await expect.poll(async () => (await opened()).slice(before), { message: `${what}: Enter` }).toEqual([url, url]);
}

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

/** github.com/acme/demo, whose pushes land in a bare repository of the test. */
function remote(app: { root: string; repo: string }) {
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
}

/** A pull request on the fake GitHub, then given what the test wants of it. */
async function pull(head: string, title: string, over: Record<string, unknown> = {}) {
  const r = await call(`${github.url}/github/repos/acme/demo/pulls`, {
    method: 'POST',
    body: JSON.stringify({ title, body: '', head, base: 'main' }),
  });
  const { number } = (await r.json()) as { number: number };
  await call(`${github.url}/__github/pulls/${number}`, { method: 'POST', body: JSON.stringify(over) });
  return number;
}

test('an agent’s pull request opens its page from its header, its card, the status bar, its menu and « PR et CI »', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  remote(app);
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  const cards = page.locator('.side .card');
  await expect(cards).toHaveCount(1, { timeout: 60_000 });
  // The agent's worktree: the repository's own comes first in the list.
  const worktrees = () =>
    git(app.repo, 'worktree', 'list', '--porcelain')
      .split('\n')
      .filter((l) => l.startsWith('worktree '))
      .map((l) => l.slice('worktree '.length))
      .slice(1);
  await expect.poll(() => worktrees().length).toBe(1);
  const tree = worktrees()[0];
  const branch = git(tree, 'branch', '--show-current');
  fs.writeFileSync(path.join(tree, 'login.ts'), 'export const login = 1;\n');
  git(tree, 'add', '-A');
  git(tree, 'commit', '-qm', 'feat: connexion par jeton');
  git(tree, 'push', '-q', 'origin', branch);
  const n = await pull(branch, 'Connexion par jeton', { checks: 'SUCCESS' });
  // Shown again: its pull request is read.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Agents', exact: true }).click();
  await cards.first().click();
  const name = `Ouvrir la PR #${n} dans le navigateur (ouverte)`;
  await expect(page.getByRole('button', { name })).toHaveCount(3);
  const { opened, refuse } = await watch(page);

  await opensBoth(page, opened, page.locator('main.conv header').getByRole('button', { name }), PAGE(n), 'header');
  await opensBoth(page, opened, page.locator(`.side [data-prs]`).getByRole('button', { name }), PAGE(n), 'card');
  await opensBoth(page, opened, page.locator('footer').getByRole('button', { name }), PAGE(n), 'status bar');

  // The menu of its card: « Voir la PR #n ».
  let before = (await opened()).length;
  await cards.first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: `Voir la PR #${n}` }).click();
  await expect.poll(async () => (await opened()).slice(before)).toEqual([PAGE(n)]);
  await cards.first().click({ button: 'right' });
  await page.getByRole('menuitem', { name: `Voir la PR #${n}` }).focus();
  await page.keyboard.press('Enter');
  await expect.poll(async () => (await opened()).slice(before)).toEqual([PAGE(n), PAGE(n)]);

  // « PR et CI » of the side panel, opened by its CI under the card.
  await page.locator('.side [data-prs] .prci').click();
  const head = page.locator('.hosts section.head');
  await expect(head).toBeVisible();
  await opensBoth(page, opened, head.getByRole('button', { name }), PAGE(n), '« PR et CI »');

  // Refused by the system: said, with its reason.
  before = (await opened()).length;
  await refuse('aucun navigateur');
  await page.locator('footer').getByRole('button', { name }).click();
  await expect(page.getByText(/aucun navigateur/)).toBeVisible();
  expect((await opened()).slice(before)).toEqual([PAGE(n)]);
});

test('the project’s pull requests and a review open the pull request’s page, by a click and by the keyboard', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  remote(app);
  git(app.repo, 'checkout', '-qb', 'feat/lien');
  fs.writeFileSync(path.join(app.repo, 'lien.ts'), 'export const lien = 1;\n');
  git(app.repo, 'add', '.');
  git(app.repo, 'commit', '-qm', 'feat: lien magique');
  git(app.repo, 'push', '-q', 'origin', 'feat/lien');
  git(app.repo, 'checkout', '-q', 'main');
  git(app.repo, 'branch', '-qD', 'feat/lien');
  const n = await pull('feat/lien', 'Connexion par lien magique', { author: 'bob' });
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);
  const { opened } = await watch(page);

  await page.locator('.side .seg').getByRole('button', { name: 'PR', exact: true }).click();
  const row = page
    .locator('main.prs')
    .getByRole('listitem')
    .filter({ hasText: `#${n}` });
  await row.hover();
  await opensBoth(page, opened, row.getByRole('button', { name: 'Ouvrir dans le navigateur' }), PAGE(n), 'list');

  await row.hover();
  await row.getByRole('button', { name: 'Faire relire' }).click();
  const view = page.locator('main.rv');
  await expect(view.getByRole('heading', { name: /Connexion par lien magique/ })).toBeVisible();
  await opensBoth(page, opened, view.getByRole('button', { name: 'Ouvrir dans le navigateur' }), PAGE(n), 'review');
});

test('a ticket’s pull request opens its page from its card « En revue »', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  remote(app);
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const col = (name: string) => page.getByRole('region', { name });
  await page.getByRole('button', { name: /Après validation/ }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('radio', { name: /Ouvrir une pull request/ }).click();
  await closeSettings(settings);
  await expect(settings).toHaveCount(0);
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Connexion par jeton [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });
  const card = col('En revue').locator('.card');
  const line = card.locator('[data-ticket-pr="101"]');
  await expect(line).toContainText('PR #101', { timeout: 60_000 });
  const { opened } = await watch(page);
  await opensBoth(page, opened, line.getByRole('button', { name: /Ouvrir la PR #101 dans le navigateur/ }), PAGE(101), 'ticket card');
});
