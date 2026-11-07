// The pull requests of the tickets (ESC-59) on the real app, against the fake GitHub of tests/fixtures/fake-trackers.mjs:
// « Valider + PR » opens the ticket's pull request through GitHub's API, the ticket goes « En revue » and its card follows
// its state, its CI, its approvals and its conflict; « Rafraîchir » reads it at once; merged on GitHub, the ticket is done.

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
  github = await startFakeTrackers(base ? base + 2 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
});

test.beforeEach(async () => {
  await fetch(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
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

/** What a test wants of pull request `n` on the fake GitHub. */
const setPull = (n: number, over: Record<string, unknown>) =>
  fetch(`${github.url}/__github/pulls/${n}`, { method: 'POST', body: JSON.stringify(over) });

test('a ticket’s pull request opens by the API, waits « En revue » with its state and CI, and its merge ends it', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  remote(app);
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const col = (name: string) => page.getByRole('region', { name });
  // « En revue » only once the board opens pull requests.
  await expect(col('À tester')).toBeVisible();
  await expect(col('En revue')).toHaveCount(0);
  await page.getByRole('button', { name: /Après validation/ }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('radio', { name: /Ouvrir une pull request/ }).click();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);
  await expect(col('En revue').getByText('Aucune PR en revue')).toBeVisible();

  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Connexion par jeton [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });

  // Opened through the API, not by gh nor in the browser: the card « En revue » shows it.
  const card = col('En revue').locator('.card');
  await expect(card).toHaveCount(1, { timeout: 60_000 });
  const line = card.locator('[data-ticket-pr="101"]');
  await expect(line).toContainText('PR #101');
  await expect(line).toContainText('ouverte');
  const opened = (await writes()).find((w) => w.path === '/github/repos/acme/demo/pulls');
  expect(opened?.body.head).toBe('ticket/dem-1');
  expect(opened?.body.base).toBe('main');
  await expect(line).toContainText('CI en cours · 2 jobs', { timeout: 30_000 });

  // Approved, its CI green, a conflict with main: read at once on « Rafraîchir ».
  await setPull(101, { checks: 'SUCCESS', approved: true, mergeable: 'CONFLICTING' });
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(line).toContainText('approuvée');
  await expect(line).toContainText('CI verte · 2 jobs');
  await expect(card.locator('.prfacts')).toHaveText('1 approbation · conflit avec main');
  if (SHOTS) await col('En revue').screenshot({ path: path.join(SHOTS, 'kanban-pr-review.png') });

  // Merged on GitHub: done, with its number.
  await setPull(101, { state: 'MERGED' });
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(col('Terminé').locator('.card')).toContainText('PR #101 mergée dans main');
  await expect(col('Terminé').locator('.card .outcome [data-icon="merge"]')).toHaveCount(1);
  await expect(col('En revue').locator('.card')).toHaveCount(0);
});

test('a failed CI goes back to the ticket’s agent: « L’agent corrige » by hand, by itself with the autopilot (ESC-60)', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  remote(app);
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
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Connexion par jeton [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });
  const card = col('En revue').locator('.card');
  await expect(card.locator('[data-ticket-pr="101"]')).toBeVisible({ timeout: 60_000 });
  const fixes = () =>
    git(path.join(app.root, 'origin.git'), 'log', '--format=%s', 'ticket/dem-1')
      .split(/\r?\n/)
      .filter((l) => l.startsWith('fix: CI de')).length;

  // Without the autopilot: its CI fails, a third-party CI without log is said, nothing goes by itself.
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  await setPull(101, { checks: 'FAILURE', thirdParty: true });
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(card.locator('[data-ticket-pr="101"]')).toContainText('2/3 jobs en échec');
  await expect(card.locator('[data-pr-nolog]')).toHaveText('ci/jenkins : CI tierce, sans journal');
  const fix = card.getByRole('button', { name: "L'agent corrige" });
  await expect(fix).toBeVisible();
  if (SHOTS) await col('En revue').screenshot({ path: path.join(SHOTS, 'kanban-pr-ci-failed.png') });

  // One click: its agent, restored with its worktree, gets the failed jobs and their logs, commits; pushed, the ticket
  // is back « En revue ».
  await fix.click();
  await expect.poll(fixes, { timeout: 60_000 }).toBe(1);
  await expect(card.locator('[data-pr-fixing]')).toHaveCount(0, { timeout: 30_000 });
  await expect(card.locator('[data-pr-fix-error]')).toHaveCount(0);
  await expect(col('En revue').locator('.card')).toHaveCount(1);

  // With the autopilot: once its CI ran again and failed again, it goes by itself.
  await setPull(101, { checks: 'PENDING', thirdParty: false });
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(card.locator('[data-ticket-pr="101"]')).toContainText('CI en cours');
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  await setPull(101, { checks: 'FAILURE' });
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect.poll(fixes, { timeout: 60_000 }).toBe(2);
  await expect(card.locator('[data-pr-fixing]')).toHaveCount(0, { timeout: 30_000 });

  // Green: nothing to fix.
  await setPull(101, { checks: 'SUCCESS' });
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(card.locator('[data-ticket-pr="101"]')).toContainText('CI verte');
  await expect(fix).toHaveCount(0);
});

/** A ticket « En revue » with its pull request #101 open on the fake GitHub, its CI running. */
async function inReview(app: { page: Page; root: string; repo: string }) {
  const { page } = app;
  remote(app);
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: /Après validation/ }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('radio', { name: /Ouvrir une pull request/ }).click();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Connexion par jeton [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await page.getByRole('region', { name: 'À tester' }).getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });
  const col = page.getByRole('region', { name: 'En revue' });
  const card = col.locator('.card');
  await expect(card.locator('[data-ticket-pr="101"]')).toContainText('CI en cours', { timeout: 60_000 });
  return { col, card };
}

test('« Merger quand c’est vert » without GitHub’s auto-merge: Escouade says it waits, then merges at the end of the CI (ESC-62)', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  // The repository allows no squash (the Kanban's) and no auto-merge.
  await fetch(`${github.url}/__github/repo`, {
    method: 'POST',
    body: JSON.stringify({ allow_squash_merge: false, allow_rebase_merge: false }),
  });
  const { col, card } = await inReview(app);
  // Its CI runs: no « Merger », « Merger quand c'est vert ».
  await setPull(101, { approved: true });
  await col.getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(card.locator('[data-ticket-pr="101"]')).toContainText('approuvée');
  await expect(card.getByRole('button', { name: /^Merger \(/ })).toHaveCount(0);
  await card.getByRole('button', { name: 'Merger quand c’est vert' }).click();
  await expect(
    page.getByText('Pas d’auto-merge sur GitHub pour ce dépôt : Escouade attend la fin de la CI et merge elle-même'),
  ).toBeVisible();
  await expect(card.locator('[data-pr-wait]')).toContainText('Escouade la merge (merge commit) dès qu’elle est approuvée et verte');
  await expect(card.getByRole('button', { name: 'Ne plus attendre' })).toBeVisible();
  expect((await writes()).some((w) => w.body?.operationName === 'EnableAutoMerge')).toBe(false);
  if (SHOTS) await col.screenshot({ path: path.join(SHOTS, 'kanban-pr-merge-wait.png') });

  // Green: Escouade merges it with a merge commit, the ticket is done.
  await setPull(101, { checks: 'SUCCESS' });
  await col.getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(page.getByRole('region', { name: 'Terminé' }).locator('.card')).toContainText('PR #101 mergée dans main', {
    timeout: 30_000,
  });
  const merged = (await writes()).find((w) => w.method === 'PUT' && w.path === '/github/repos/acme/demo/pulls/101/merge');
  expect(merged?.body.merge_method).toBe('merge');
});

test('« Merger quand c’est vert » turns on GitHub’s auto-merge, and « Merger » merges approved and green (ESC-62)', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  await fetch(`${github.url}/__github/repo`, { method: 'POST', body: JSON.stringify({ allow_auto_merge: true }) });
  const { col, card } = await inReview(app);
  await card.getByRole('button', { name: 'Merger quand c’est vert' }).click();
  await expect(card.locator('[data-pr-wait]')).toHaveText('Auto-merge de GitHub activé : mergée dès que la CI est verte');
  const asked = (await writes()).find((w) => w.body?.operationName === 'EnableAutoMerge');
  expect(asked?.body.variables.method).toBe('SQUASH');
  await expect(card.getByRole('button', { name: 'Merger quand c’est vert' })).toHaveCount(0);

  // Approved and green: « Merger » with the Kanban's squash.
  await setPull(101, { checks: 'SUCCESS', approved: true });
  await col.getByRole('button', { name: 'Rafraîchir' }).click();
  await card.getByRole('button', { name: 'Merger (squash)' }).click();
  await expect(page.getByRole('region', { name: 'Terminé' }).locator('.card')).toContainText('PR #101 mergée dans main', {
    timeout: 30_000,
  });
  const merged = (await writes()).find((w) => w.method === 'PUT' && w.path === '/github/repos/acme/demo/pulls/101/merge');
  expect(merged?.body.merge_method).toBe('squash');
});

test('review comments wait for « L’agent corrige les retours », even with the autopilot; a conflict, for « L’agent met à jour la branche » (ESC-61)', async ({
  app,
}) => {
  test.setTimeout(300_000);
  const { page } = app;
  remote(app);
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const col = (name: string) => page.getByRole('region', { name });
  await page.getByRole('button', { name: /Après validation/ }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('radio', { name: /Ouvrir une pull request/ }).click();
  // Off by default: nothing is published under the threads.
  await expect(settings.getByRole('switch', { name: 'Répondre sous les fils corrigés' })).toHaveAttribute('aria-checked', 'false');
  await expect(settings.getByRole('group', { name: 'Mise à jour de la branche' }).getByRole('button', { name: 'Merge' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Connexion par jeton [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });
  const card = col('En revue').locator('.card');
  await expect(card.locator('[data-ticket-pr="101"]')).toBeVisible({ timeout: 60_000 });
  const origin = path.join(app.root, 'origin.git');
  const log = () => git(origin, 'log', '--format=%s', 'ticket/dem-1');

  // Bob asks for changes on a line: counted on the card; the autopilot (on) leaves it for the click.
  await setPull(101, { changesRequested: true });
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(card.locator('[data-pr-feedback]')).toHaveText('1 retour de relecture');
  const fixReviews = card.getByRole('button', { name: "L'agent corrige les retours" });
  await expect(fixReviews).toBeVisible();
  await page.waitForTimeout(3_000);
  await expect(card.locator('[data-pr-fixing]')).toHaveCount(0);
  expect(log()).not.toContain('fix: retours de');
  if (SHOTS) await col('En revue').screenshot({ path: path.join(SHOTS, 'kanban-pr-feedback.png') });

  // One click: its agent gets the comment, commits; pushed, nothing published on GitHub, and the comment is not offered again.
  await fixReviews.click();
  await expect.poll(log, { timeout: 60_000 }).toContain('fix: retours de DEM-1');
  await expect(card.locator('[data-pr-fixing]')).toHaveCount(0, { timeout: 30_000 });
  await expect(card.locator('[data-pr-fix-error]')).toHaveCount(0);
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(card.locator('[data-pr-feedback]')).toHaveCount(0);
  const published = (await writes()).filter((w) => /replies|resolveReviewThread|\/comments$/.test(`${w.path} ${JSON.stringify(w.body)}`));
  expect(published).toEqual([]);

  // main moved on GitHub, and the pull request conflicts: the target merged into its branch, pushed.
  execFileSync('git', ['commit', '-q', '--allow-empty', '-m', 'chore: ailleurs'], { cwd: app.repo });
  git(app.repo, 'push', '-q', 'origin', 'main');
  await setPull(101, { changesRequested: false, mergeable: 'CONFLICTING' });
  await col('En revue').getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(card.getByText('conflit avec main')).toBeVisible();
  const update = card.getByRole('button', { name: "L'agent met à jour la branche" });
  if (SHOTS) await col('En revue').screenshot({ path: path.join(SHOTS, 'kanban-pr-conflict.png') });
  await update.click();
  await expect.poll(log, { timeout: 60_000 }).toContain('chore: ailleurs');
  await expect(card.locator('[data-pr-fixing]')).toHaveCount(0, { timeout: 30_000 });
  await expect(card.locator('[data-pr-fix-error]')).toHaveCount(0);
  expect(log()).toMatch(/Merge/);
});
