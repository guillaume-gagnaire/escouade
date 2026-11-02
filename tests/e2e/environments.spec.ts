// The deployments (ESC-67) on the real app, against the fake GitHub of tests/fixtures/fake-trackers.mjs: a ticket « En
// revue » opens the preview its pull request has; merged on GitHub, it is done, and the view « Environnements » shows
// what runs where, the ticket in staging, production's failed deployment with its job, and the ticket's card says
// « staging ✓ · production ⏳ ».

import { execFileSync } from 'node:child_process';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

let github: { url: string; close: () => Promise<void> };

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
const writes = async (): Promise<{ method: string; path: string; body: any }[]> => (await fetch(`${github.url}/__writes`)).json();
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 5 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
});

test.beforeEach(async () => {
  await fetch(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
});

/** github.com/acme/demo, whose pushes land in a bare repository of the test: that repository. */
function remote(app: { root: string; repo: string }) {
  const bare = path.join(app.root, 'origin.git');
  // On main, whatever the machine's init.defaultBranch: a clone of it then checks main out (the CI's is master).
  git(app.root, 'init', '-q', '--bare', '-b', 'main', bare);
  const url = 'https://github.com/acme/demo.git';
  git(app.repo, 'remote', 'add', 'origin', url);
  git(app.repo, 'config', `url.${bare.replace(/\\/g, '/')}.insteadOf`, url);
  git(app.repo, 'push', '-q', 'origin', 'main');
  return bare;
}

/** Merges `branch` into main on « GitHub » (the bare repository), as its merge button would: the merge commit. */
function mergeOnHost(root: string, bare: string, branch: string): string {
  const clone = path.join(root, 'host-clone');
  git(root, 'clone', '-q', '-b', 'main', bare, clone);
  const who = ['-c', 'user.name=Ada', '-c', 'user.email=ada@example.com'];
  git(clone, ...who, 'merge', '-q', '--no-ff', '-m', `Merge ${branch}`, `origin/${branch}`);
  git(clone, 'push', '-q', 'origin', 'main');
  return git(clone, 'rev-parse', 'HEAD');
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

const post = (route: string, body: unknown) => fetch(`${github.url}${route}`, { method: 'POST', body: JSON.stringify(body) });

test('the preview opens from the card in review, then the environments say where the merged ticket runs', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  const bare = remote(app);
  const before = git(app.repo, 'rev-parse', 'HEAD');
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const col = (name: string) => page.getByRole('region', { name });
  await page.getByRole('button', { name: /Après validation/ }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('radio', { name: /Ouvrir une pull request/ }).click();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);

  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Panier persistant [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });

  // « En revue »: its preview (a Vercel deployment of its branch) opens from its card.
  const card = col('En revue').locator('.card');
  await expect(card).toHaveCount(1, { timeout: 60_000 });
  const preview = card.getByRole('button', { name: 'Ouvrir l’aperçu' });
  await expect(preview).toBeVisible({ timeout: 30_000 });
  if (SHOTS) await col('En revue').screenshot({ path: path.join(SHOTS, 'environments-preview.png') });

  // Merged on GitHub; staging runs the merge, production the commit before it and its newer deployment failed.
  const merge = mergeOnHost(app.root, bare, 'ticket/dem-1');
  await post('/__github/pulls/101', { state: 'MERGED', mergeCommit: merge });
  await post('/__github/deployments', [
    { environment: 'production', sha: merge, state: 'failure', log: 'https://github.com/acme/demo/actions/runs/9/job/90' },
    { environment: 'staging', sha: merge, state: 'success', url: 'https://staging.example.com', author: 'ada' },
    { environment: 'production', sha: before, state: 'success', url: 'https://example.com', author: 'grace' },
  ]);
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(col('Terminé').locator('.card')).toContainText('PR #101 mergée dans main', { timeout: 30_000 });

  // The view « Environnements »: each lane with what runs there, the ticket in staging, production's failure.
  await page.getByRole('button', { name: 'Environnements' }).click();
  const lane = (name: string) => page.locator(`[data-env-lane="${name}"]`);
  await expect(lane('staging')).toContainText('DEM-1', { timeout: 30_000 });
  await expect(lane('staging')).toContainText(`${merge.slice(0, 7)} · il y a 1 h · par ada`);
  await expect(lane('production')).toContainText(before.slice(0, 7));
  await expect(lane('production').locator('[data-env-failed]')).toHaveText(`Dernier déploiement en échec · ${merge.slice(0, 7)}`);
  await expect(page.locator('[data-not-in-prod]')).toHaveText('1 ticket pas encore en production');
  if (SHOTS) await page.locator('main.board').screenshot({ path: path.join(SHOTS, 'environments-view.png') });
  // Nothing was published on GitHub to read them.
  expect((await writes()).filter((w) => w.path.startsWith('/github/') && w.path.includes('deployments'))).toEqual([]);

  // Back to the tickets: the done card says where it runs.
  await page.getByRole('button', { name: 'Tickets' }).click();
  const envs = col('Terminé').locator('[data-ticket-envs]');
  await expect(envs).toHaveText('staging · production');
  await expect(envs.locator('.dep-env.deployed [data-icon="check"]')).toHaveCount(1);
  await expect(envs.locator('.dep-env:not(.deployed) [data-icon="clock"]')).toHaveCount(1);
  if (SHOTS) await col('Terminé').screenshot({ path: path.join(SHOTS, 'environments-card.png') });
});
