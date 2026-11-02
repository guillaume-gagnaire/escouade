// The CI of the target branch and of the branches pushed without a pull request (ESC-63) on the real app, against the
// fake GitHub of tests/fixtures/fake-trackers.mjs: the pill of the board's header (« main ✗ depuis 2 h »), its jobs in a
// window, a failed one's log in reading only, sent to an agent, « Relancer les jobs en échec » on a click only, the
// autopilot paused while the target is red when the board asks it; and a ticket « Valider et pousser » whose branch's CI
// shows on its card « Terminé ».

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

/** What a test wants of the CI of branch `name` on the fake GitHub. */
const setBranch = (name: string, over: Record<string, unknown>) =>
  fetch(`${github.url}/__github/branches/${encodeURIComponent(name)}`, { method: 'POST', body: JSON.stringify(over) });

test('the target’s CI shows in the header, its log opens in reading only and goes to an agent, its failed jobs run again on a click, and it pauses the autopilot when asked', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  remote(app);
  await setBranch('main', { checks: 'FAILURE', thirdParty: true });
  // With its first agent: the one the log goes to.
  await addProject(page, app.repo);
  await connectGithub(page);
  await page.getByRole('button', { name: /^Kanban/ }).click();

  // The pill of the header, red since its job ended.
  const pill = page.locator('[data-ci-pill="target"]');
  await expect(pill).toHaveText('main ✗ depuis 2 h', { timeout: 60_000 });
  await expect(pill).toHaveAttribute('aria-label', /CI de main : 2\/3 jobs en échec/);
  // Off by default: the autopilot is not paused.
  await expect(page.locator('[data-red-target]')).toHaveCount(0);

  // Its jobs, a third-party one without a log.
  await pill.click();
  const win = page.getByRole('dialog', { name: 'CI de main' });
  const jobs = win.getByRole('region', { name: 'CI' });
  await expect(jobs).toContainText('2 échecs sur 3');
  await expect(jobs.locator('[data-check="ci/jenkins"]')).toContainText('CI tierce : un statut et une adresse, pas de journal');
  // The log of a failed one, as a terminal shows it: its colours, its error in red, nothing to type in.
  await jobs.getByRole('button', { name: 'test', exact: true }).click();
  const log = jobs.getByRole('listitem', { name: 'Journal de test' });
  await expect(log).toContainText('FAIL src/main.test.ts');
  await expect(log.locator('pre.term .fg-1')).toHaveText('FAIL');
  await expect(log.locator('pre.term .ln.error')).toContainText('Process completed with exit code 1.');
  await expect(log.getByRole('textbox')).toHaveCount(0);
  if (SHOTS) await win.screenshot({ path: path.join(SHOTS, 'ci-target-log.png') });

  // « Envoyer à un agent »: the job and the end of its log go to the project's agent, as a message.
  await jobs.getByRole('button', { name: 'test · Envoyer à un agent' }).click();
  const choice = page.getByRole('menuitem').first();
  const agentName = (await choice.textContent())!.trim();
  await choice.click();
  await expect(page.getByText(`Journal de test envoyé à ${agentName}.`)).toBeVisible();

  // Nothing written on the host until the click; then the failed job's run, once.
  expect(await writes()).toEqual([]);
  await win.getByRole('button', { name: 'Relancer les jobs en échec' }).click();
  await expect(page.getByText('1 exécution relancée.')).toBeVisible();
  expect((await writes()).map((w) => `${w.method} ${w.path}`)).toEqual(['POST /github/repos/acme/demo/actions/runs/9/rerun-failed-jobs']);
  await expect(pill).toHaveText('main · CI en cours');
  await win.getByRole('button', { name: 'Fermer' }).click();

  // « Pause quand la CI de la cible est rouge », switched on: red again, the autopilot starts nothing; « Lancer » does.
  await page.getByRole('button', { name: /Après validation/ }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  const pause = settings.getByRole('switch', { name: 'Pause quand la CI de la cible est rouge' });
  await expect(pause).toHaveAttribute('aria-checked', 'false');
  await pause.click();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);
  await setBranch('main', { checks: 'FAILURE' });
  await pill.click();
  await win.getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(pill).toHaveText('main ✗ depuis 2 h');
  await win.getByRole('button', { name: 'Fermer' }).click();
  const band = page.locator('[data-red-target]');
  await expect(band).toContainText('Pilote auto en pause : la CI de main est rouge depuis 2 h');
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Attendre la CI [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  const todo = page.getByRole('region', { name: 'À faire' });
  await expect(todo.getByText('En attente : CI de la cible rouge')).toBeVisible();
  if (SHOTS) await page.locator('.board').screenshot({ path: path.join(SHOTS, 'ci-target-pause.png') });

  // Green again: the pause lifts and the ticket starts by itself.
  await setBranch('main', { checks: 'SUCCESS' });
  await pill.click();
  await win.getByRole('button', { name: 'Rafraîchir' }).click();
  await expect(pill).toHaveText('main ✓');
  await win.getByRole('button', { name: 'Fermer' }).click();
  await expect(band).toHaveCount(0);
  await expect(todo.locator('.card')).toHaveCount(0, { timeout: 30_000 });
});

test('a ticket « Valider et pousser » shows its branch’s CI on its card « Terminé »', async ({ app }) => {
  test.setTimeout(240_000);
  const { page } = app;
  remote(app);
  await setBranch('ticket/dem-1', { checks: 'FAILURE' });
  await addProject(page, app.repo, { firstAgent: false });
  await connectGithub(page);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  const col = (name: string) => page.getByRole('region', { name });
  await page.getByRole('button', { name: /Après validation/ }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('radio', { name: /Pousser la branche du ticket/ }).click();
  await settings.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(settings).toHaveCount(0);
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Pousser sans PR [ok]');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await col('À tester').getByRole('button', { name: 'Valider et pousser' }).click({ timeout: 90_000 });

  const card = col('Terminé').locator('.card');
  const pill = card.locator('[data-ci-pill="pushed"]');
  await expect(pill).toHaveText('ticket/dem-1 ✗ depuis 2 h', { timeout: 60_000 });
  // Nothing published but the push.
  expect((await writes()).filter((w) => w.path.startsWith('/github/repos'))).toEqual([]);
  await pill.click();
  const win = page.getByRole('dialog', { name: 'CI de ticket/dem-1' });
  await expect(win.getByRole('region', { name: 'CI' })).toContainText('1 échec sur 2');
  if (SHOTS) await card.screenshot({ path: path.join(SHOTS, 'ci-pushed-card.png') });
});
