// The control of a pull request and its merge box (ESC-488, PR-8 of the PR in Escouade) on the real app, against the fake
// GitHub of tests/fixtures/fake-trackers.mjs: a ticket's pull request opened from its card « En revue », marked ready from
// a draft, a review asked of marta, a label put, a failed check run again alone, its branch brought up to date with main,
// then merged with squash after a confirmation that says what goes, its branch deleted on GitHub and the ticket « Terminé ».
// Each gesture is a click, written on GitHub in the user's name (the fake records it). At both sizes: narrow
// (ESCOUADE_E2E_VIEWPORT=1024x728) the control opens over the view. ESCOUADE_E2E_SHOTS=<folder>: its screenshots, in both
// themes.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage, closeSettings } from './fixture';
import { startFakeTrackers } from '../fixtures/fake-trackers.mjs';

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

type Asked = { method: string; path: string; body: Record<string, any> | null };
// What was written on GitHub: its writes, GraphQL's mutations only.
const READS = ['Pulls', 'OpenPulls', 'PullsOfBranch', 'PullRequestId', 'PrControl', 'PrCandidates', 'PrThreads', 'Threads', 'RepoParent'];
const writes = async () =>
  ((await (await call(`${github.url}/__writes`)).json()) as Asked[]).filter(
    (w) => w.method !== 'GET' && !(w.path.endsWith('/graphql') && READS.includes(String(w.body?.operationName ?? ''))),
  );
const setPull = (n: number, over: Record<string, unknown>) =>
  call(`${github.url}/__github/pulls/${n}`, { method: 'POST', body: JSON.stringify(over) });

test.beforeAll(async () => {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  github = await startFakeTrackers(base ? base + 8 : 0);
  process.env.ESCOUADE_GITHUB_API = `${github.url}/github`;
});

test.beforeEach(async () => {
  await call(`${github.url}/__reset`, { method: 'POST' });
});

test.afterAll(async () => {
  await github?.close();
});

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

/** The control of the pull request: beside the view when it is wide, over it when it is narrow. */
async function control(view: Locator): Promise<Locator> {
  const aside = view.getByRole('complementary', { name: 'Contrôle de la PR' });
  const toggle = view.getByRole('button', { name: 'Contrôle de la PR' });
  if (!(await aside.isVisible()) && (await toggle.isVisible())) await toggle.click();
  await expect(aside).toBeVisible();
  return aside;
}

/** Reads the pull request again, as « Relire la PR » does. */
async function reread(view: Locator) {
  await view.getByRole('button', { name: 'Relire la PR' }).click();
}

test('the control of a ticket’s pull request: ready, a reviewer, a label, a check run again, the branch updated, merged with squash and its branch deleted, the ticket done', async ({
  app,
}) => {
  test.setTimeout(480_000);
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
  const line = col('En revue').locator('.card [data-ticket-pr="101"]');
  await expect(line).toContainText('PR #101', { timeout: 60_000 });
  // What was written before (the pull request opened by the validation) is not the view's.
  const before = (await writes()).length;
  const since = async () => (await writes()).slice(before);

  // On GitHub: a draft, its check `test` failed, behind main, an approval required by the base's rules.
  await setPull(101, { draft: true, checks: 'FAILURE', behind: true, reviewRequired: true, labels: ['auth'] });
  await line.getByRole('button', { name: /^Ouvrir la PR #101 \(/ }).click();
  const view = page.getByRole('main', { name: 'La PR #101' });
  await expect(view.getByRole('tabpanel')).toBeVisible({ timeout: 60_000 });
  const aside = await control(view);
  const box = aside.getByRole('region', { name: 'Prête à merger ?' });
  await expect(box.locator('[data-row="draft"]')).toContainText('Brouillon', { timeout: 30_000 });
  await expect(box.locator('[data-row="checks"]')).toContainText('1 contrôle en échec');
  await expect(box.locator('[data-row="branch"]')).toContainText('En retard sur main');
  await expect(box.locator('[data-row="reviews"]')).toContainText('Relecture demandée');
  await expect(box.getByRole('button', { name: 'Squash et merger' })).toBeDisabled();
  // Nothing written by itself: opening reads.
  expect(await since()).toEqual([]);
  await shots(page, 'pr-control-blocked');

  // A draft made ready.
  await box.locator('[data-row="draft"]').getByRole('button', { name: 'Prête à relire' }).click();
  await expect(box.locator('[data-row="draft"]')).toHaveCount(0, { timeout: 30_000 });
  expect((await since()).map((w) => w.body?.operationName)).toEqual(['MarkReady']);

  // A review asked of marta, among those GitHub offers.
  await aside.getByRole('button', { name: 'Demander une relecture' }).first().click();
  const people = aside.getByRole('group', { name: 'Relecteurs à demander' });
  await people.getByLabel('marta').check();
  await expect(aside.locator('.rvr').filter({ hasText: 'marta' })).toContainText('demandée', { timeout: 30_000 });
  await people.getByRole('button', { name: 'Fermer le contrôle' }).click();

  // A label put; a third party's name stays text.
  await aside.getByRole('button', { name: 'Modifier les étiquettes' }).click();
  const labels = aside.getByRole('group', { name: 'Étiquettes de la PR' });
  await expect(labels.getByLabel('<img src=x onerror=alert(1)>')).toBeVisible();
  await expect(labels.locator('img')).toHaveCount(0);
  await labels.getByLabel('bug').check();
  await expect(aside.locator('.lbls')).toContainText('bug', { timeout: 30_000 });
  await labels.getByRole('button', { name: 'Fermer le contrôle' }).click();

  // The failed check run again alone, from the box.
  await box.locator('[data-row="checks"]').getByRole('button', { name: 'Relancer l’échec' }).click();
  await expect(box).toContainText('Contrôle relancé.', { timeout: 30_000 });

  // The branch updated with main by GitHub, a merge chosen.
  await box.locator('[data-row="branch"]').getByRole('button', { name: 'Mettre à jour' }).click();
  await aside.getByRole('menuitem', { name: 'Fusionner main dans la branche' }).click();
  await expect(box).toContainText('Branche mise à jour par GitHub', { timeout: 30_000 });
  await expect(box.locator('[data-row="branch"]')).toContainText('Pas de conflit', { timeout: 30_000 });

  const asked = await since();
  expect(asked.map((w) => `${w.method} ${w.path.replace(/^\/github/, '')} ${w.body?.operationName ?? ''}`.trim())).toEqual([
    'POST /graphql MarkReady',
    'POST /repos/acme/demo/pulls/101/requested_reviewers',
    'POST /repos/acme/demo/issues/101/labels',
    'POST /repos/acme/demo/actions/jobs/77/rerun',
    'POST /graphql UpdateBranch',
  ]);
  expect(asked[1].body?.reviewers).toEqual(['marta']);
  expect(asked[2].body?.labels).toEqual(['bug']);
  expect(asked[4].body?.variables.method).toBe('MERGE');

  // Green and approved: ready to merge.
  await setPull(101, { checks: 'SUCCESS', approved: true });
  await reread(view);
  await expect(box).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  await expect(box.getByRole('heading')).toHaveText('Prête à merger');
  await shots(page, 'pr-control-ready');

  // The merge, confirmed: squash (the Kanban's), its branch deleted on GitHub, the ticket done.
  await box.getByRole('button', { name: 'Squash et merger' }).click();
  const confirm = box.getByRole('alertdialog', { name: 'Merger la PR #101 ?' });
  await expect(confirm).toContainText('Squash et merger dans main : un seul commit sur main.');
  await expect(confirm).toContainText('La branche ticket/dem-1 est supprimée sur GitHub.');
  await expect(confirm).toContainText(/[A-Z]+-1 passe en Terminé\./);
  await shots(page, 'pr-control-confirm');
  await confirm.getByRole('button', { name: 'Merger' }).click();
  await expect(box).toContainText('Mergée dans main (Squash et merger).', { timeout: 60_000 });
  await expect(box).toContainText('Branche ticket/dem-1 supprimée.');
  await expect(box).toContainText(/[A-Z]+-1 passé en Terminé\./);
  // Read again: merged, the box says it.
  await expect(view.locator('.hd .pill-st')).toHaveText('Fusionnée', { timeout: 30_000 });
  await expect(box.getByRole('heading')).toHaveText('Fusionnée');
  await shots(page, 'pr-control-merged');
  const merged = (await since()).filter((w) => /\/merge$|git\/refs/.test(w.path));
  expect(merged.map((w) => `${w.method} ${w.path.replace(/^\/github/, '')}`)).toEqual([
    'PUT /repos/acme/demo/pulls/101/merge',
    'DELETE /repos/acme/demo/git/refs/heads/ticket/dem-1',
  ]);
  expect(merged[0].body?.merge_method).toBe('squash');

  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(col('Terminé').locator('.card')).toContainText('PR #101 mergée dans main', { timeout: 30_000 });
});

test('a refusal is said in the box: a token without the right, a conflict appeared meanwhile', async ({ app }) => {
  test.setTimeout(360_000);
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
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Connexion par jeton [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });
  const line = col('En revue').locator('.card [data-ticket-pr="101"]');
  await expect(line).toContainText('PR #101', { timeout: 60_000 });
  await setPull(101, { checks: 'SUCCESS', approved: true, writeStatus: 403 });
  await line.getByRole('button', { name: /^Ouvrir la PR #101 \(/ }).click();
  const view = page.getByRole('main', { name: 'La PR #101' });
  await expect(view.getByRole('tabpanel')).toBeVisible({ timeout: 60_000 });
  const aside = await control(view);
  const box = aside.getByRole('region', { name: 'Prête à merger ?' });
  await expect(box).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });

  // The token may not write: said, nothing changed.
  await aside.getByRole('checkbox', { name: 'Repasser la PR en brouillon' }).click();
  await expect(box.getByRole('alert')).toContainText('Ton jeton GitHub n’a pas le droit de faire cela', { timeout: 30_000 });
  await shots(page, 'pr-control-error');

  // A conflict appeared since it was read: the merge is refused, said in the box.
  await setPull(101, { writeStatus: null, mergeable: 'CONFLICTING' });
  await box.getByRole('button', { name: 'Squash et merger' }).click();
  await box.getByRole('alertdialog').getByRole('button', { name: 'Merger' }).click();
  await expect(box.getByRole('alert')).toContainText('Un conflit est apparu entre-temps avec main', { timeout: 30_000 });
  expect((await writes()).some((w) => /git\/refs/.test(w.path))).toBe(false);
  await expect(box).not.toContainText('Mergée');
});

test('merged, the view says it at once: a read of before that arrives after never brings back « Ouverte » (a race forced by the fake)', async ({
  app,
}) => {
  test.setTimeout(360_000);
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
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Connexion par jeton [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider + PR' }).click({ timeout: 90_000 });
  const line = col('En revue').locator('.card [data-ticket-pr="101"]');
  await expect(line).toContainText('PR #101', { timeout: 60_000 });
  await setPull(101, { checks: 'SUCCESS', approved: true });
  await line.getByRole('button', { name: /^Ouvrir la PR #101 \(/ }).click();
  const view = page.getByRole('main', { name: 'La PR #101' });
  await expect(view.getByRole('tabpanel')).toBeVisible({ timeout: 60_000 });
  const aside = await control(view);
  const box = aside.getByRole('region', { name: 'Prête à merger ?' });
  await expect(box).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });

  // What the core keeps of the pull request is no longer fresh (20 s): « Relire la PR » asks GitHub, whose description
  // answers 3 s late, « Ouverte »; meanwhile the merge, whose reads of the merged pull request answer 4 s late.
  await page.waitForTimeout(21_000);
  await call(`${github.url}/__github/delays`, { method: 'POST', body: JSON.stringify({ description: 3000, mergedPulls: 4000 }) });
  await view.getByRole('button', { name: 'Relire la PR' }).click();
  await box.getByRole('button', { name: 'Squash et merger' }).click();
  await box.getByRole('alertdialog').getByRole('button', { name: 'Merger' }).click();
  await expect(box).toContainText('Mergée dans main (Squash et merger).', { timeout: 60_000 });
  await expect(view.locator('.hd .pill-st')).toHaveText('Fusionnée', { timeout: 30_000 });
  // The late reads have all answered: still merged.
  await page.waitForTimeout(8_000);
  await expect(view.locator('.hd .pill-st')).toHaveText('Fusionnée');
  await expect(box.getByRole('heading')).toHaveText('Fusionnée');
  await call(`${github.url}/__github/delays`, { method: 'POST', body: '{}' });

  // The list « Pull requests » no longer has it among the open ones; the ticket is done.
  await view.getByRole('button', { name: 'Retour aux PR' }).click();
  await expect(page.locator('main.prs')).not.toContainText('#101', { timeout: 30_000 });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(col('Terminé').locator('.card')).toContainText('PR #101 mergée dans main', { timeout: 30_000 });
});
