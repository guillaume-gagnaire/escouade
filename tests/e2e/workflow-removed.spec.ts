// A workflow deleted, or « Pas de workflow pour ce projet » (ESC-309): the project's own /dev skill (its workflow in
// .escouade/workflows/ of the repository) deleted in Réglages › Projets › Workflow, GSD of Principal's folder (its file on
// this machine) deleted by hand; neither comes back after « Relire » nor after a restart, both said under « Supprimés »;
// one restored; then no workflow for the project: nothing read nor written again, its file deleted by hand stays gone.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const FIXTURES = path.resolve(import.meta.dirname, '..', 'fixtures', 'workflows');

test.use({
  // GSD in Principal's Claude Code folder (the app's CLAUDE_CONFIG_DIR is the test's `claude`).
  preStart: {
    run: (root: string) => fs.cpSync(path.join(FIXTURES, 'gsd', 'home', '.claude'), path.join(root, 'claude'), { recursive: true }),
  },
});

async function open(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('tab', { name: 'Projets' }).click();
  const section = dialog.locator('#sg-workflow');
  await section.scrollIntoViewIfNeeded();
  return { dialog, section };
}

test('a workflow deleted, here or by hand, never comes back; no workflow for the project reads and writes none', async ({ app }) => {
  test.setTimeout(240_000);
  let page = app.page;
  await page.setViewportSize({ width: 1024, height: 728 });
  fs.cpSync(path.join(FIXTURES, 'gsd', 'project', '.planning'), path.join(app.repo, '.planning'), { recursive: true });
  fs.cpSync(path.join(FIXTURES, 'dev', 'project', '.claude'), path.join(app.repo, '.claude'), { recursive: true });
  for (const args of [
    ['add', '-A'],
    ['-c', 'user.email=e2e@test', '-c', 'user.name=e2e', 'commit', '-qm', 'dev'],
  ])
    execFileSync('git', args, { cwd: app.repo, stdio: 'pipe' });
  await addProject(page, app.repo);

  let { dialog, section } = await open(page);
  const dev = path.join(app.repo, '.escouade', 'workflows', 'claude-plan.yml');
  await expect(section.getByRole('button', { name: 'Corriger les étapes de plan' })).toBeVisible();
  await expect(section.getByRole('button', { name: 'Corriger les étapes de GSD' })).toBeVisible();
  expect(fs.existsSync(dev)).toBe(true);
  const projects = path.join(app.data, 'projects');
  const gsd = fs
    .readdirSync(projects)
    .map((id) => path.join(projects, id, 'workflows', 'claude-gsd.yml'))
    .find((p) => fs.existsSync(p))!;
  expect(gsd).toBeTruthy();

  // The repository's /dev deleted in the window: its file goes at once, said under « Supprimés ».
  await section.getByRole('button', { name: 'Corriger les étapes de plan' }).click();
  await section.getByRole('button', { name: 'Supprimer ce workflow' }).click();
  await expect(section.getByRole('button', { name: 'Corriger les étapes de plan' })).toBeHidden();
  await expect(section.getByTestId('wf-removed').filter({ hasText: 'plan' })).toContainText('supprimé, ne sera pas recréé');
  expect(fs.existsSync(dev)).toBe(false);
  // GSD's file deleted by hand: « Relire » sees it as deleted, and writes neither again.
  fs.rmSync(gsd);
  await section.getByRole('button', { name: 'Relire' }).click();
  await expect(section.getByRole('button', { name: 'Corriger les étapes de GSD' })).toBeHidden();
  await expect(section.getByTestId('wf-removed')).toHaveCount(2);
  expect(fs.existsSync(dev) || fs.existsSync(gsd)).toBe(false);
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();

  // Nor after a restart.
  page = await app.restart();
  await page.setViewportSize({ width: 1024, height: 728 });
  ({ dialog, section } = await open(page));
  await expect(section.getByTestId('wf-removed')).toHaveCount(2);
  await section.getByRole('button', { name: 'Relire' }).click();
  await expect(section.getByRole('button', { name: 'Relire' })).toBeEnabled();
  await expect(section.getByTestId('wf-removed')).toHaveCount(2);
  expect(fs.existsSync(dev) || fs.existsSync(gsd)).toBe(false);

  // « Rétablir » GSD: read and written again.
  await section.getByTestId('wf-removed').filter({ hasText: 'GSD' }).getByRole('button', { name: 'Rétablir' }).click();
  await expect(section.getByRole('button', { name: 'Corriger les étapes de GSD' })).toBeVisible();
  expect(fs.existsSync(gsd)).toBe(true);

  // « Pas de workflow pour ce projet »: said, with how to undo it; saved.
  await section.getByRole('radio', { name: 'Pas de workflow pour ce projet' }).click();
  await expect(section.getByTestId('workflows-off')).toContainText('Pour le défaire, choisis « Lire les workflows » et enregistre.');
  await dialog.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(dialog).toBeHidden();
  // Its last file deleted by hand, the app started again: nothing read, nothing written.
  fs.rmSync(gsd);
  const reading = path.join(path.dirname(path.dirname(gsd)), 'workflows.json');
  fs.rmSync(reading, { force: true });
  page = await app.restart();
  await page.setViewportSize({ width: 1024, height: 728 });
  ({ dialog, section } = await open(page));
  await expect(section.getByRole('radio', { name: 'Pas de workflow pour ce projet' })).toHaveAttribute('aria-checked', 'true');
  await expect(section.getByTestId('workflows-off')).toBeVisible();
  await expect(section.getByRole('button', { name: 'Relire' })).toBeHidden();
  expect(fs.existsSync(gsd) || fs.existsSync(dev) || fs.existsSync(reading)).toBe(false);
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  // The Workflow tab says it too, and where it is undone.
  await page.keyboard.press('Control+Shift+B');
  const side = page.locator('aside.git-side');
  await side.getByRole('tab', { name: /^Workflow/ }).click();
  await expect(side.getByTestId('workflow-off')).toContainText('Pas de workflow pour ce projet');
  await expect(side.getByTestId('workflow-off')).toContainText('Réglages › Projets › Workflow, « Lire les workflows »');
});
