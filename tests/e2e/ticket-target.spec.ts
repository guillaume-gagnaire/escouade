import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, test } from './fixture';

// A ticket's own target branch (ESC-552), with the fake `claude`: chosen in its form (« Cible » › « Une autre branche… »),
// said on its card (« vers release »), its worktree started from it and its validation merged into it, the Kanban's
// target left as it was.

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', ...args], { cwd, encoding: 'utf8', stdio: 'pipe' }).trim();

test('a ticket aimed at another branch than the Kanban’s starts from it and is validated into it', async ({ app }) => {
  // A ticket goes to « À tester » and is merged on the way.
  test.slow();
  const { page } = app;
  // « release » has a file main has not: the ticket's worktree starts from it.
  git(app.repo, 'switch', '-q', '-c', 'release');
  fs.writeFileSync(path.join(app.repo, 'release.txt'), 'release\n');
  git(app.repo, 'add', 'release.txt');
  git(app.repo, 'commit', '-qm', 'release only');
  git(app.repo, 'switch', '-q', 'main');
  const mainBefore = git(app.repo, 'rev-parse', 'main');
  await addProject(page, app.repo, { firstAgent: false });
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await dialog.getByRole('textbox', { name: 'Titre du ticket' }).fill('Correctif de la ligne release');
  // Its target: the Kanban's by default, another one chosen in the picker.
  const field = dialog.getByRole('button', { name: /^Cible/ });
  await expect(field).toContainText('Celle du Kanban');
  await field.click();
  await page.getByRole('menuitem', { name: 'Une autre branche…' }).click();
  const picker = page.getByRole('dialog', { name: 'Branche cible' });
  await picker.getByText('release', { exact: true }).click();
  await expect(picker).toBeHidden();
  await expect(field).toContainText('release');
  await dialog.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(dialog).toBeHidden();

  // To test, its card says where it goes.
  const review = page.getByRole('region', { name: 'À tester' });
  await expect(review.getByText('DEM-1', { exact: true })).toBeVisible({ timeout: 60_000 });
  await expect(review.locator('[data-ticket-target]')).toHaveText('vers release');
  await review.getByRole('button', { name: 'Valider et merger' }).click();
  const done = page.getByRole('region', { name: 'Terminé' });
  await expect(done.getByText('Mergé dans release · squash')).toBeVisible({ timeout: 60_000 });
  // Its work is on release, which it started from; main, the Kanban's target, is as it was.
  expect(git(app.repo, 'show', 'release:dem-1.txt')).toMatch(/^Boucle \d$/);
  expect(git(app.repo, 'show', 'release:release.txt')).toBe('release');
  expect(git(app.repo, 'rev-parse', 'main')).toBe(mainBefore);
});
