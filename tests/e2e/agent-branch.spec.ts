// The branch of the agent open (PR #17, tried on the real app in ESC-238), in the window of the CI runner (1024 × 728):
// the status bar follows the agent shown and goes back to the project's folder's on the board, its picker switches the
// agent's worktree (the agent told with the next message), refuses what it can't take, and its pull request is there
// with how many more, opened in the browser (caught here: no browser opens). Against the fake GitHub of
// tests/fixtures/fake-trackers.mjs, github.com/acme/demo pushing to a bare repository of the test.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8', stdio: 'pipe' }).trim();

test.beforeAll(async () => {
  github = await startFakeTrackers(0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
  process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
});

test.afterAll(async () => {
  await github?.close();
  delete process.env.ESCOUADE_PR_READ_EVERY_MS;
});

/** Connects github.com's account (the fake one's) in Réglages › Services connectés. */
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

/**
 * What the app asks to open in the browser, answered here: caught on the `fetch` of Tauri's IPC, as links.spec.ts does
 * (`page.route` misses the WebView2's IPC requests from time to time).
 */
async function watchOpened(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as { opened: string[] };
    w.opened = [];
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.includes('open_url')) {
        w.opened.push(JSON.parse(String(init?.body ?? '{}')).url);
        return Promise.resolve(new Response('null', { headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } }));
      }
      return fetch(input, init);
    };
  });
  return () => page.evaluate(() => (window as unknown as { opened: string[] }).opened);
}

/** A project on github.com/acme/demo (a bare repository of the test), a worktree per agent, its account connected. */
async function project(app: { page: Page; root: string; repo: string }) {
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', '-u', 'origin', 'main');
  await addProject(app.page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(app.page);
  return bare;
}

/** The agents' worktrees, in the order they were made (the repository's own folder, first in git's list, left out). */
const worktrees = (repo: string) =>
  git(repo, 'worktree', 'list', '--porcelain')
    .split('\n')
    .filter((l) => l.startsWith('worktree '))
    .map((l) => l.slice('worktree '.length))
    .slice(1);

test('the status bar follows the agent open, its picker switches its worktree, refuses what it can’t, and the agent is told', async ({
  app,
}) => {
  const { page } = app;
  await page.setViewportSize({ width: 1024, height: 728 });
  await project(app);
  git(app.repo, 'branch', 'feat/local');
  const cards = page.locator('.side .card');
  for (const n of [1, 2]) {
    await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
    await expect(cards).toHaveCount(n, { timeout: 60_000 });
  }
  await expect.poll(() => worktrees(app.repo).length).toBe(2);
  const [wt] = worktrees(app.repo);
  const mine = git(wt, 'branch', '--show-current');

  // The agent open: its worktree's branch, not published yet; the board: the project's folder's.
  const name = mine.replace('escouade/', '');
  const card = cards.filter({ hasText: name });
  await card.click();
  const bar = page.locator('footer.bar');
  const branch = bar.locator('.branch');
  await expect(branch).toContainText(mine);
  await expect(branch).toContainText('non publiée');
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(branch).toContainText('main');
  await expect(branch).not.toContainText(mine);
  await page.getByRole('button', { name: 'Agents', exact: true }).click();
  await card.click();
  await expect(branch).toContainText(mine);

  // Its picker opens on its branch, not on main, which it can't take.
  await branch.click();
  const picker = page.getByRole('dialog', { name: `Branche de ${name}` });
  await expect(picker).toBeVisible();
  const option = (b: string) => picker.getByRole('option').filter({ has: page.locator('.name', { hasText: new RegExp(`^${b}$`) }) });
  await expect(option(mine)).toHaveAttribute('aria-selected', 'true');
  await expect(option('main')).toHaveAttribute('aria-disabled', 'true');
  // The other agent's branch: refused, why said under the list.
  const other = git(worktrees(app.repo)[1], 'branch', '--show-current');
  await picker.getByRole('combobox').fill(other);
  await expect(option(other)).toHaveAttribute('aria-disabled', 'true');
  await expect(picker.locator('.note')).toHaveText(
    `La branche « ${other} » est utilisée par l’agent ${other.replace('escouade/', '')}, dans son worktree.`,
  );
  await option(other).click({ force: true });
  await expect(picker).toBeVisible();
  expect(git(wt, 'branch', '--show-current')).toBe(mine);

  // Uncommitted changes: refused, said in the picker that stays open, its foot not covered.
  fs.writeFileSync(path.join(wt, 'src', 'app.ts'), 'const a = 2;\n');
  await picker.getByRole('combobox').fill('feat/local');
  await page.keyboard.press('Enter');
  await expect(picker.getByRole('alert')).toHaveText(
    'Il reste des changements non commités dans le worktree de l’agent : commite-les ou annule-les avant de changer de branche.',
  );
  await expect(page.locator('.toast')).toHaveCount(0);
  await expect(picker.getByRole('button', { name: 'Dossier du projet…' })).toBeVisible();
  expect(git(wt, 'branch', '--show-current')).toBe(mine);
  git(wt, 'checkout', '--', 'src/app.ts');

  // The switch, by the keyboard: the branch made for it, which had nothing of its own, goes.
  await page.keyboard.press('Enter');
  await expect(picker).toHaveCount(0);
  await expect(page.getByText(`${name} passe sur feat/local.`)).toBeVisible();
  await expect(
    page.locator('.conv').getByText(`Branche : ${mine} → feat/local. « ${mine} » n’avait aucun commit à elle : supprimée.`),
  ).toBeVisible();
  await expect(branch).toContainText('feat/local');
  await expect(branch).toBeFocused();
  expect(git(wt, 'branch', '--show-current')).toBe('feat/local');
  expect(git(app.repo, 'branch', '--list', mine)).toBe('');

  // The next message: the agent is told first (the fake claude says back what it got), never in the bubble.
  await send(page, 'et maintenant ?');
  await expect(
    page
      .locator('.conv')
      .getByText(/Bonjour, tu as dit : Note d'Escouade : l'utilisateur a fait passer ton dossier de travail de la branche/),
  ).toBeVisible({ timeout: 30_000 });
  await expect(page.locator('.conv').getByText('et maintenant ?', { exact: true }).first()).toBeVisible();
  await expect(page.locator('.conv').getByText(/^Note d'Escouade/)).toHaveCount(0);
  // Told once.
  await send(page, 'encore');
  await expect(page.locator('.conv').getByText('Bonjour, tu as dit : encore', { exact: true })).toBeVisible({ timeout: 30_000 });

  // It switches by itself: followed.
  git(wt, 'switch', '-q', '-c', 'feat/seul');
  await expect(branch).toContainText('feat/seul', { timeout: 15_000 });
  await expect(page.locator('.conv').getByText('Branche : feat/local → feat/seul.')).toBeVisible();
});

test('the pull request of the agent’s branch is beside it in the status bar, with how many more, and opens in the browser', async ({
  app,
}) => {
  const { page } = app;
  await page.setViewportSize({ width: 1024, height: 728 });
  await project(app);
  const opened = await watchOpened(page);
  const cards = page.locator('.side .card');
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  await expect(cards).toHaveCount(1, { timeout: 60_000 });
  await expect.poll(() => worktrees(app.repo).length).toBe(1);
  const mine = git(worktrees(app.repo)[0], 'branch', '--show-current');
  for (const title of ['Premier travail', 'Deuxième travail', 'Le travail de bob'])
    await fetch(`${github.url}/github/repos/acme/demo/pulls`, {
      method: 'POST',
      body: JSON.stringify({ title, body: '', head: mine, base: 'main' }),
    });
  // The third one, from bob's fork, on a branch of the same name: not the agent's.
  await fetch(`${github.url}/__github/pulls/103`, { method: 'POST', body: JSON.stringify({ headRepo: 'bob/demo' }) });
  // Shown again: its pull requests read.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Agents', exact: true }).click();
  await cards.first().click();

  const chip = page.locator('footer.bar').getByRole('button', { name: 'Ouvrir la PR #102 dans le navigateur (ouverte)' });
  await expect(chip).toBeVisible();
  await expect(chip).toHaveText('#102+1 autre');
  // Whole in the window, beside the branch.
  const box = (await chip.boundingBox())!;
  expect(box.x + box.width).toBeLessThanOrEqual(1024);
  await chip.click();
  await expect.poll(opened).toEqual(['https://github.com/acme/demo/pull/102']);

  // The board: the project's folder's branch, no pull request.
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.locator('footer.bar .branch')).toContainText('main');
  await expect(page.locator('footer.bar .pr')).toHaveCount(0);
});
