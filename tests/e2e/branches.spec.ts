import { execFileSync } from 'node:child_process';
import { addProject, expect, test } from './fixture';

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

test('a branch is created from the status bar, the folder switched to it, and the git graph shows it as the current one', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  const branch = page.locator('footer .sync');
  await expect(branch).toContainText('⎇ main');

  // From the branch picker: « Nouvelle branche… », a name, « et y passer » (ticked by default).
  await branch.click();
  await page.getByRole('dialog', { name: 'Branches' }).getByRole('button', { name: 'Nouvelle branche…' }).click();
  const dialog = page.getByRole('dialog', { name: 'Nouvelle branche' });
  await dialog.getByRole('textbox', { name: 'Nom' }).fill('feat/e2e');
  await expect(dialog.getByRole('checkbox', { name: 'et y passer' })).toBeChecked();
  await dialog.getByRole('button', { name: 'Créer' }).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByText('Branche « feat/e2e » créée : tu es dessus.')).toBeVisible();

  // The status bar and the repository agree on where the folder is.
  await expect(branch).toContainText('⎇ feat/e2e');
  expect(git(app.repo, 'branch', '--show-current')).toBe('feat/e2e');
  // It started where main is, and tracks nothing.
  expect(git(app.repo, 'rev-parse', 'feat/e2e')).toBe(git(app.repo, 'rev-parse', 'main'));

  // The graph (« Historique » of the side panel) labels the commit with the current branch, and keeps main as another.
  await page.getByRole('button', { name: /Fichiers/ }).click();
  await page.getByRole('tab', { name: /Historique/ }).click();
  const graph = page.locator('.graph');
  await expect(graph.locator('.ref.head')).toHaveText('feat/e2e');
  await expect(graph.locator('.ref.branch')).toHaveText('main');
});
