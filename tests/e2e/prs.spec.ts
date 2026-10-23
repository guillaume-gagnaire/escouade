// The pull requests of an ordinary agent (ESC-64) on the real app, against the fake GitHub of
// tests/fixtures/fake-trackers.mjs: opened from the header of its conversation (pushed to a local
// remote that stands for github.com/acme/demo, its title proposed by the fake claude), then its CI
// and its review read on its card, and the two corrections handed to the agent on a click.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const writes = async (): Promise<{ method: string; path: string; body: any }[]> => (await fetch(`${github.url}/__writes`)).json();

test.beforeAll(async () => {
  github = await startFakeTrackers(0);
  // Read by the app the fixture starts next: github.com's API is the fake one, and the pull
  // requests are read again at each selection of the agent.
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
  process.env.ESCOUADE_PR_READ_EVERY_MS = '0';
});

test.beforeEach(async () => {
  await fetch(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
  delete process.env.ESCOUADE_PR_READ_EVERY_MS;
});

/** Connects github.com's account (the fake one's) in Réglages › Intégrations. */
async function connectGithub(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Intégrations' }).click();
  await settings.getByRole('button', { name: 'Connecter…' }).nth(2).click();
  const form = settings.getByRole('form', { name: 'Connexion à GitHub Issues' });
  await form.getByLabel(/Jeton/).fill('ghp_jeton');
  await form.getByRole('button', { name: 'Connecter' }).click();
  await expect(settings.getByText('Connecté · @ada').first()).toBeVisible();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);
}

/** The Kanban, then the agents again: the agent's conversation is shown anew, and its pull requests read again. */
async function showAgain(page: Page) {
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Agents', exact: true }).click();
  await page.locator('.side .card').first().click();
}

test('an ordinary agent opens its pull request, and its card follows its CI and its review', async ({ app }) => {
  const { page } = app;
  // github.com/acme/demo, whose pushes land in a bare repository of the test.
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  const cards = page.locator('.side .card');
  await expect(cards).toHaveCount(1, { timeout: 60_000 });
  // Shown before github.com's account was there: no « Ouvrir une PR » yet.
  await expect(page.getByRole('button', { name: 'Ouvrir une PR' })).toHaveCount(0);
  await connectGithub(page);
  // Shown again: it finds its code host.
  await showAgain(page);

  // The project's first agent, in its worktree, and its work committed there. The repository's own
  // comes first in the list: left out by its place, as its path may be spelled otherwise (8.3 names
  // of the Windows runner).
  const worktrees = () =>
    git(app.repo, 'worktree', 'list', '--porcelain')
      .split('\n')
      .filter((l) => l.startsWith('worktree '))
      .map((l) => l.slice('worktree '.length))
      .slice(1);
  await expect.poll(() => worktrees().length).toBe(1);
  const wt = worktrees()[0];
  const branch = git(wt, 'branch', '--show-current');
  fs.writeFileSync(path.join(wt, 'login.ts'), 'export const login = 1;\n');
  git(wt, 'add', '-A');
  git(wt, 'commit', '-qm', 'feat: connexion par jeton');

  // « Ouvrir une PR » from the header: the title and the description proposed, read, then pushed and opened.
  await page.getByRole('button', { name: 'Ouvrir une PR' }).click();
  const dialog = page.getByRole('dialog', { name: 'Ouvrir une PR sur GitHub' });
  await expect(dialog.getByRole('textbox', { name: 'Titre' })).toHaveValue('Le travail de l’agent, proposé par le faux claude');
  await expect(dialog.getByRole('textbox', { name: 'Description' })).toHaveValue('- feat: connexion par jeton');
  await expect(dialog.getByText(`· ${branch} → main`)).toBeVisible();
  expect((await writes()).filter((w) => w.path === '/github/repos/acme/demo/pulls')).toHaveLength(0);
  await dialog.getByRole('checkbox', { name: 'En brouillon' }).check();
  await dialog.getByRole('button', { name: 'Pousser et ouvrir' }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByText('PR #101 ouverte.')).toBeVisible();
  const created = (await writes()).filter((w) => w.path === '/github/repos/acme/demo/pulls');
  expect(created.map((w) => w.body)).toEqual([
    {
      title: 'Le travail de l’agent, proposé par le faux claude',
      body: '- feat: connexion par jeton',
      head: branch,
      base: 'main',
      draft: true,
    },
  ]);
  expect(git(bare, 'log', '-1', '--format=%s', branch)).toBe('feat: connexion par jeton');
  // Its number in the header and under the card; « Ouvrir une PR » gone while it is open.
  await expect(page.getByRole('button', { name: 'Ouvrir la PR #101 dans le navigateur (brouillon)' })).toHaveCount(2);
  await expect(page.getByRole('button', { name: 'Ouvrir une PR' })).toHaveCount(0);

  // Its CI fails and a reviewer asks for changes: read again when the agent is shown.
  await fetch(`${github.url}/__github/pulls/101`, {
    method: 'POST',
    body: JSON.stringify({ checks: 'FAILURE', changesRequested: true, draft: false }),
  });
  await showAgain(page);
  const prs = page.getByRole('group', { name: /^Pull requests de / });
  await expect(prs.getByRole('button', { name: 'Ouvrir la PR #101 dans le navigateur (ouverte)' })).toHaveText('✗#101');
  // Nothing sent by itself: the corrections wait for the click.
  await expect(prs.getByRole('button', { name: 'L’agent corrige', exact: true })).toBeVisible();
  await expect(prs.getByRole('button', { name: 'L’agent corrige les retours' })).toBeVisible();
  await expect(page.getByText(/La CI de la PR #101/)).toHaveCount(0);

  await prs.getByRole('button', { name: 'L’agent corrige', exact: true }).click();
  await expect(page.getByText('Envoyé à', { exact: false }).first()).toBeVisible();
  await expect(
    page
      .locator('.conv')
      .getByText(/La CI de la PR #101 \(https:\/\/github\.com\/acme\/demo\/pull\/101\) échoue/)
      .first(),
  ).toBeVisible();
  await expect(
    page
      .locator('.conv')
      .getByText(/FAIL src\/app\.test\.ts/)
      .first(),
  ).toBeVisible();
  // Its turn over, then the review's feedback.
  await expect(
    page
      .locator('.conv')
      .getByText(/Bonjour, tu as dit : La CI de la PR #101/)
      .first(),
  ).toBeVisible({ timeout: 30_000 });
  await prs.getByRole('button', { name: 'L’agent corrige les retours' }).click();
  await expect(
    page
      .locator('.conv')
      .getByText(/a des retours de relecture/)
      .first(),
  ).toBeVisible();
  await expect(
    page
      .locator('.conv')
      .getByText(/Ce nom est peu clair\./)
      .first(),
  ).toBeVisible();
  // The pull request stays on the branch it was opened from.
  expect(git(wt, 'branch', '--show-current')).toBe(branch);
});

test('no « Ouvrir une PR » in a project whose remote is on no code host Escouade opens pull requests on', async ({ app }) => {
  const { page } = app;
  // No remote at all.
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  const cards = page.locator('.side .card');
  await expect(cards).toHaveCount(1, { timeout: 60_000 });
  // Its pull requests were asked for when it was shown: none, and nothing to offer.
  await expect(page.getByRole('button', { name: 'Ouvrir la PR', exact: false })).toHaveCount(0);
  await page.waitForTimeout(2_000);
  await expect(page.getByRole('button', { name: 'Ouvrir une PR' })).toHaveCount(0);
  await cards.first().click({ button: 'right' });
  await expect(page.getByRole('menuitem', { name: 'Intégrer main' })).toBeVisible();
  await expect(page.getByRole('menuitem', { name: 'Ouvrir une PR…' })).toHaveCount(0);
  await page.keyboard.press('Escape');

  // A GitLab remote: GitLab has no client yet, nothing either.
  git(app.repo, 'remote', 'add', 'origin', 'git@gitlab.com:acme/demo.git');
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  await expect(cards).toHaveCount(2);
  await page.waitForTimeout(2_000);
  await expect(page.getByRole('button', { name: 'Ouvrir une PR' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Ouvrir une MR' })).toHaveCount(0);

  // On github.com with its account: offered (the next agent shown reads it).
  git(app.repo, 'remote', 'set-url', 'origin', 'https://github.com/acme/demo.git');
  await connectGithub(page);
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  await expect(cards).toHaveCount(3);
  await expect(page.getByRole('button', { name: 'Ouvrir une PR' })).toBeVisible();
});

test('each agent follows the pull requests of the worktree it went into by itself, never the project’s folder’s (ESC-161)', async ({
  app,
}) => {
  const { page } = app;
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  // No worktree per agent: two agents that go into worktrees of their own, as Claude Code's EnterWorktree makes them.
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  const cards = page.locator('.side .card');
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  await expect(cards).toHaveCount(1, { timeout: 60_000 });
  await send(page, '[worktree feat/a] au travail');
  await expect(page.locator('.conv').getByText('Bonjour, tu as dit : [worktree feat/a] au travail')).toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: '+ Nouvel agent' }).first().click();
  await expect(cards).toHaveCount(2);
  await send(page, '[worktree feat/b] au travail');
  await expect(page.locator('.conv').getByText('Bonjour, tu as dit : [worktree feat/b] au travail')).toBeVisible({ timeout: 30_000 });

  // The project's folder on a third branch; a pull request on each of the three.
  git(app.repo, 'checkout', '-q', '-b', 'dev');
  for (const head of ['feat/a', 'feat/b', 'dev'])
    await fetch(`${github.url}/github/repos/acme/demo/pulls`, {
      method: 'POST',
      body: JSON.stringify({ title: `Le travail de ${head}`, body: '', head, base: 'main' }),
    });
  const chip = (scope: Page | Locator, n: number) => scope.getByRole('button', { name: `Ouvrir la PR #${n} dans le navigateur (ouverte)` });
  const anyChip = (scope: Locator) => scope.getByRole('button', { name: /^Ouvrir la PR #/ });

  // Each one shown again reads its own: the group under its card has its pull request alone.
  for (const i of [0, 1]) await cards.nth(i).click();
  const groups = page.getByRole('group', { name: /^Pull requests de / });
  await expect(groups).toHaveCount(2);
  const header = page.locator('.conv .head');
  for (const [n, branch] of [
    [101, 'feat/a'],
    [102, 'feat/b'],
  ] as const) {
    const group = groups.filter({ has: chip(page, n) });
    await expect(group).toHaveCount(1);
    await expect(anyChip(group)).toHaveCount(1);
    const name = (await group.getAttribute('aria-label'))!.replace('Pull requests de ', '');
    await cards.filter({ has: page.getByText(name, { exact: true }) }).click();
    // Its header says its branch, not the project's folder's, and its pull request alone.
    await expect(header.getByText(` / ${branch}`)).toBeVisible();
    await expect(anyChip(header)).toHaveCount(1);
    await expect(chip(header, n)).toHaveCount(1);
  }
  // None of dev's, the project's folder's branch, anywhere.
  await expect(chip(page, 103)).toHaveCount(0);
  expect(git(app.repo, 'branch', '--show-current')).toBe('dev');
});
