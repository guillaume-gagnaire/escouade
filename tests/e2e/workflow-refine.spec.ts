// « Affiner… » (ESC-196): the model, the account and the cost said before, the fake claude asked on « Lancer » only,
// what it answers checked by the rules and written into the workflow's file (never committed), and an answer out
// of the schema that changes nothing. The project has the /dev workflow of its own skills (frozen copy).

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, test, openSettingsPage } from './fixture';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const DEV = path.join(ROOT, 'tests', 'fixtures', 'workflows', 'dev', 'project');

const commits = (cwd: string) => execFileSync('git', ['log', '--oneline'], { cwd, encoding: 'utf8' }).trim().split(/\r?\n/).length;

test('« Affiner… » says who answers and at what cost, then writes what the rules accept', async ({ app }) => {
  test.setTimeout(180_000);
  const page = app.page;
  fs.cpSync(path.join(DEV, '.claude'), path.join(app.repo, '.claude'), { recursive: true });
  for (const args of [
    ['add', '-A'],
    ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qm', 'dev'],
  ])
    execFileSync('git', args, { cwd: app.repo, stdio: 'pipe' });
  const before = commits(app.repo);
  await addProject(page, app.repo, { lang: 'fr' });

  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Kanban');
  const section = dialog.locator('#sg-workflow');
  await section.scrollIntoViewIfNeeded();
  const shared = path.join(app.repo, '.escouade', 'workflows', 'claude-plan.yml');
  await expect(section.getByRole('button', { name: 'Corriger les étapes de plan' })).toBeVisible();
  await expect.poll(() => fs.existsSync(shared)).toBe(true);
  const read = fs.readFileSync(shared, 'utf8');

  // Said before: nothing is asked, nothing written.
  await section.getByRole('button', { name: 'Affiner…' }).click();
  const plan = section.getByTestId('refine-plan');
  await expect(plan).toContainText('Modèle : Claude Haiku · compte : ');
  await expect(plan).toContainText('au tarif public');
  expect(fs.readFileSync(shared, 'utf8')).toBe(read);

  // « Lancer »: the fake claude answers, the rules accept it, the file says it.
  await plan.getByRole('button', { name: 'Lancer' }).click();
  await expect(section.getByText(/^Workflows affinés : /)).toBeVisible();
  const refined = fs.readFileSync(shared, 'utf8');
  expect(refined).toContain('Précisé par le faux claude');
  expect(refined).toContain('refined:');
  // Escouade commits nothing: the file waits for the user.
  expect(commits(app.repo)).toBe(before);
  // Read again, what the agent made stands.
  await section.getByRole('button', { name: 'Relire' }).click();
  await expect(section.getByRole('button', { name: 'Relire' })).toBeEnabled();
  expect(fs.readFileSync(shared, 'utf8')).toBe(refined);
});

test('an answer out of the schema changes nothing and says so', async ({ app }) => {
  test.setTimeout(180_000);
  const page = app.page;
  fs.cpSync(path.join(DEV, '.claude'), path.join(app.repo, '.claude'), { recursive: true });
  // The marker of a file the fake claude answers in prose to.
  fs.appendFileSync(path.join(app.repo, '.claude', 'skills', 'dev', 'SKILL.md'), '\n[invalide]\n');
  for (const args of [
    ['add', '-A'],
    ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qm', 'dev'],
  ])
    execFileSync('git', args, { cwd: app.repo, stdio: 'pipe' });
  await addProject(page, app.repo, { lang: 'fr' });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Kanban');
  const section = dialog.locator('#sg-workflow');
  await section.scrollIntoViewIfNeeded();
  const shared = path.join(app.repo, '.escouade', 'workflows', 'claude-plan.yml');
  await expect.poll(() => fs.existsSync(shared)).toBe(true);
  const read = fs.readFileSync(shared, 'utf8');
  await section.getByRole('button', { name: 'Affiner…' }).click();
  await section.getByTestId('refine-plan').getByRole('button', { name: 'Lancer' }).click();
  await expect(section.getByText(/rien n.a changé/)).toBeVisible();
  expect(fs.readFileSync(shared, 'utf8')).toBe(read);
});
