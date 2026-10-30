// The side panel's « PR et CI » tab (ESC-305) on the real app, against the fake GitHub of tests/fixtures/fake-trackers.mjs:
// the agent's pull request with its CI (each check, its time, the log of a failed one, « Relancer les échecs » on a click
// only), its reviews in reading and its deployment; « Ouvrir une PR… » without one; the project's open pull requests
// without an agent. At the runner's size the panel is 340 px, where the tab leaves the strip (choice 189): the CI on the
// agent's card opens it.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const writes = async (): Promise<{ method: string; path: string; body: any }[]> => (await fetch(`${github.url}/__writes`)).json();
/** What the app wrote on the host, its GraphQL reads (POSTs) left out. */
const written = async () => (await writes()).filter((w) => w.path.startsWith('/github/') && w.path !== '/github/graphql');
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 1 : 0);
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

/** github.com/acme/demo, whose pushes land in a bare repository of the test. */
function remote(app: { root: string; repo: string }) {
  const bare = path.join(app.root, 'origin.git');
  git(app.root, 'init', '-q', '--bare', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
}

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

const openPull = (head: string, title: string) =>
  fetch(`${github.url}/github/repos/acme/demo/pulls`, { method: 'POST', body: JSON.stringify({ title, body: '', head, base: 'main' }) });

test('the tab shows the agent’s pull request, its CI with the log of a failure, its reviews and its deployment', async ({ app }) => {
  const { page } = app;
  remote(app);
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  const cards = page.locator('.side .card');
  await expect(cards).toHaveCount(1, { timeout: 60_000 });
  const wt = git(app.repo, 'worktree', 'list', '--porcelain')
    .split('\n')
    .filter((l) => l.startsWith('worktree '))
    .map((l) => l.slice('worktree '.length))
    .slice(1)[0];
  const branch = git(wt, 'branch', '--show-current');
  fs.writeFileSync(path.join(wt, 'login.ts'), 'export const login = 1;\n');
  git(wt, 'add', '-A');
  git(wt, 'commit', '-qm', 'feat: connexion par jeton');

  // Its pull request on the host, its CI failed and a review asking for changes.
  await openPull(branch, 'Connexion par jeton');
  const setUp = (await written()).length;
  await fetch(`${github.url}/__github/pulls/101`, { method: 'POST', body: JSON.stringify({ checks: 'FAILURE', changesRequested: true }) });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Agents', exact: true }).click();
  await cards.first().click();
  const group = page.getByRole('group', { name: /^Pull requests de / });
  await expect(group).toContainText('#101 → main');
  // Its CI in words beside « #101 → main », which opens the tab.
  await group.getByRole('button', { name: 'CI en échec' }).click();
  const side = page.locator('.git-side');
  await expect(side.getByRole('region', { name: 'PR #101 · Connexion par jeton' })).toBeVisible();
  await expect(side.getByRole('region', { name: 'PR #101 · Connexion par jeton' })).toContainText(`${branch} → main`);
  await expect(side.getByRole('region', { name: 'PR #101 · Connexion par jeton' })).toContainText('créée il y a 2 h par ada');
  await expect(side).not.toContainText('dans la 2.1');

  const ci = side.getByRole('region', { name: 'CI' });
  await expect(ci).toContainText('1 échec sur 2');
  const failed = ci.locator('[data-check="test"]');
  await expect(failed).toContainText('échouée');
  await expect(failed).toContainText('2m 30s');
  await expect(ci.locator('[data-check="lint"]')).toContainText('réussie');
  await expect(ci.locator('[data-check="lint"]')).toContainText('0m 12s');
  // Its log, on a click.
  await failed.getByRole('button', { name: 'test', exact: true }).click();
  await expect(ci.getByRole('listitem', { name: 'Journal de test' })).toContainText('FAIL src/app.test.ts');

  const reviews = side.getByRole('region', { name: 'Revues et commentaires' });
  await expect(reviews).toContainText('bob demande des changements');
  await expect(reviews).toContainText('src/app.ts:1');
  await expect(reviews).toContainText('Ce nom est peu clair.');
  await expect(side.getByRole('region', { name: 'Déploiements' })).toContainText('demo-pr-101.vercel.app');
  if (SHOTS) await side.screenshot({ path: path.join(SHOTS, 'pr-tab-failed.png') });

  // Nothing written on the host until the click; then the failed job's run, once.
  expect((await written()).slice(setUp)).toEqual([]);
  await ci.getByRole('button', { name: 'Relancer les échecs' }).click();
  await expect(page.getByText('1 exécution relancée.')).toBeVisible();
  expect((await written()).slice(setUp).map((w) => `${w.method} ${w.path}`)).toEqual([
    'POST /github/repos/acme/demo/actions/runs/7/rerun-failed-jobs',
  ]);

  // A second agent, without a pull request: « Ouvrir une PR… ».
  await page.getByRole('button', { name: 'Nouvel agent', exact: true }).first().click();
  await expect(cards).toHaveCount(2);
  await expect(side.getByText('Pas de PR sur cette branche')).toBeVisible();
  await side.getByRole('button', { name: 'Ouvrir une PR…' }).click();
  // Its window, which says it has nothing to propose yet.
  await expect(page.getByRole('dialog', { name: /^Ouvrir une PR sur/ }).getByRole('alert')).toContainText('Rien à proposer');
});

test('without an agent, the tab lists the project’s open pull requests', async ({ app }) => {
  const { page } = app;
  remote(app);
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  await connectGithub(page);
  await openPull('feat/sombre', 'Mode sombre');
  await openPull('feat/footer', 'Lien du footer');
  await fetch(`${github.url}/__github/pulls/102`, { method: 'POST', body: JSON.stringify({ checks: 'SUCCESS' }) });
  await page.getByRole('button', { name: 'PR et CI du projet' }).click();
  const list = page.locator('.git-side').getByRole('region', { name: 'PR ouvertes du projet' });
  await expect(list.getByRole('listitem')).toHaveCount(2);
  await expect(list.getByRole('listitem').first()).toContainText('Lien du footer');
  await expect(list.getByRole('listitem').first()).toContainText('CI verte');
  await expect(list.getByRole('listitem').nth(1)).toContainText('CI en cours');
  if (SHOTS) await page.locator('.git-side').screenshot({ path: path.join(SHOTS, 'pr-tab-project.png') });
});
