// The shared Kanban's name taken from git (ESC-243), the case of the user who asked: `git config user.name` set globally,
// the first launch's window put off (« Plus tard »). « Partager » takes git's name without asking it again, the settings
// show it filled in, and nothing of it is saved nor written in .escouade.yml. In the runner's window (1024 × 728).

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test, openSettingsPage } from './fixture';

/** The machine's global git config, a file of the test's: git's name of the user, whatever the runner's. */
const GLOBAL = path.join(os.tmpdir(), 'escouade-e2e-shared-git-name.gitconfig');
fs.writeFileSync(GLOBAL, '[user]\n\tname = Alice Globale\n\temail = alice@example.com\n');

test.use({
  // « Plus tard »: asked, no name given.
  preStart: { run: () => ({ userName: '', userId: 'a91f03c2', nameAsked: true }) },
  appEnv: { GIT_CONFIG_GLOBAL: GLOBAL },
});

async function board(page: Page) {
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByRole('region', { name: 'À faire' })).toBeVisible();
}

test('« Partager » without a name takes git’s, without asking, and the settings show it filled in', async ({ app }) => {
  test.setTimeout(180_000);
  let { page } = app;
  await page.setViewportSize({ width: 1024, height: 728 });
  // The repository has no identity of its own: the global one is git's name here.
  execFileSync('git', ['config', '--unset', 'user.name'], { cwd: app.repo });

  await addProject(page, app.repo, { firstAgent: false });
  await board(page);
  await page.getByRole('switch', { name: 'Pilote auto' }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const form = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await form.getByRole('textbox', { name: 'Titre du ticket' }).fill('Export des factures en PDF');
  await form.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(form).toBeHidden();
  const card = page.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /DEM-1 Export des factures en PDF/ });

  // « Partager » from its menu: shared at once, the window asking the name not opened again.
  await card.click({ button: 'right' });
  await page.getByRole('menuitem', { name: 'Partager avec l’équipe' }).click();
  await expect(card.locator('.mark')).toHaveAccessibleName(/Partagé avec l’équipe · \.escouade\/tickets\/[a-z0-9]{8}\.yml/);
  await expect(page.getByRole('dialog', { name: 'Ton nom pour l’équipe' })).toHaveCount(0);
  const dir = path.join(app.repo, '.escouade', 'tickets');
  const [name] = fs.readdirSync(dir).filter((f) => f.endsWith('.yml'));
  expect(fs.readFileSync(path.join(dir, name), 'utf8')).toContain('author: { name: Alice Globale, id: a91f03c2 }');

  // Réglages › Général: « Ton nom » filled with git's, said so.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Général');
  await expect(dialog.getByRole('textbox', { name: 'Ton nom', exact: true })).toHaveValue('Alice Globale');
  await expect(dialog.getByText('Repris de git', { exact: true })).toBeVisible();
  // Réglages › Projets › Équipe: the name in this project, empty, shows git's.
  await openSettingsPage(dialog, 'Équipe');
  const own = dialog.getByRole('textbox', { name: 'Ton nom dans ce projet' });
  await own.scrollIntoViewIfNeeded();
  await expect(own).toHaveValue('');
  await expect(own).toHaveAttribute('placeholder', 'Alice Globale');
  await expect(dialog.getByText(/Vide : Alice Globale, repris de git/)).toBeVisible();
  // The settings dialog holds in the window.
  const box = (await dialog.boundingBox())!;
  expect(box.x + box.width).toBeLessThanOrEqual(1024);
  await page.keyboard.press('Escape');

  // Nothing of git's name kept: neither in the settings nor in the project's .escouade.yml.
  const settings = JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
  expect(settings.userName).toBe('');
  const yml = path.join(app.repo, '.escouade.yml');
  expect(fs.existsSync(yml) ? fs.readFileSync(yml, 'utf8') : '').not.toContain('Alice');

  // A teammate's Kanban: the author's chip says the name taken from git.
  // Committed by the user, as a pull would bring it (Escouade never commits it).
  execFileSync('git', ['add', '.escouade'], { cwd: app.repo });
  execFileSync('git', ['-c', 'user.name=Alice Globale', 'commit', '-qm', 'chore(tickets): DEM-1 partagé'], { cwd: app.repo });
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  fs.writeFileSync(path.join(bobData, 'settings.json'), JSON.stringify({ ...settings, userName: 'Bob Durand', userId: '5d0e77b1' }));
  page = await app.restartAs(bobData);
  await page.setViewportSize({ width: 1024, height: 728 });
  await addProject(page, app.repo, { firstAgent: false });
  await board(page);
  const hers = page.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /DEM-1 Export des factures en PDF/ });
  await expect(hers.locator('.who')).toHaveAccessibleName('Ajouté par Alice Globale');
});
