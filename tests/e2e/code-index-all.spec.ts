// « Index du code sur tous les projets » (ESC-293), in Réglages › Projets: off by default; turned on, a project that
// follows it is read and its own section says it follows the app's setting, while a project turned off by hand keeps its
// choice and says so; turned off, the app's confirmation says how many projects lose their index. `ESCOUADE_E2E_SHOTS`:
// both sections in both themes.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

function repoAt(dir: string) {
  fs.mkdirSync(path.join(dir, 'src'), { recursive: true });
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'ignore' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(dir, 'src', 'shop.ts'), 'export function pay() { return charge(); }\nfunction charge() {}\n');
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shots(page: Page, of: Locator, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  await of.scrollIntoViewIfNeeded();
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.mouse.move(0, 0);
    await of.screenshot({ path: path.join(SHOTS, `${name}-${t}.png`) });
  }
  await theme(page, 'dark');
}

test('the app’s setting turns on the projects that follow it and leaves a project turned off by hand off', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await page.setViewportSize({ width: 1280, height: 860 });
  // Two projects: « demo-api » follows the app's setting, « boutique » is turned off by hand.
  await addProject(page, app.repo, { firstAgent: false });
  const shop = repoAt(path.join(app.root, 'boutique'));
  await addProject(page, shop, { firstAgent: false, name: 'boutique' });

  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('tab', { name: 'Projets' }).click();
  const pill = (name: string) => settings.locator('.proj-pill', { hasText: name });
  const own = settings.locator('section[data-anchor="code-index"]');
  const all = settings.locator('section[data-anchor="code-index-all"]');
  const projectSwitch = own.getByRole('switch', { name: 'Indexer ce projet' });
  const allSwitch = all.getByRole('switch', { name: 'Index du code sur tous les projets' });

  // Off by default, followed by both.
  await expect(allSwitch).toHaveAttribute('aria-checked', 'false');
  await expect(all.getByText('Suivie par 2 projets sur 2.')).toBeVisible();

  // « boutique »: turned off by hand (on, then off), its own choice.
  await pill('boutique').click();
  await projectSwitch.click();
  await expect(own.locator('[data-code-index-status]')).toContainText(/fichiers?/, { timeout: 30_000 });
  await projectSwitch.click();
  await expect(projectSwitch).toHaveAttribute('aria-checked', 'false');
  await expect(own.locator('[data-code-index-origin="chosen"]')).toContainText('Choisi pour ce projet.');
  await expect(all.getByText('Suivie par 1 projet sur 2.')).toBeVisible();

  // The app's setting on: a toast says how many projects are read, the window answers meanwhile.
  await allSwitch.click();
  await expect(allSwitch).toHaveAttribute('aria-checked', 'true');
  await expect(page.getByText('Index du code sur tous les projets allumé : 1 projet sera lu, en arrière-plan.')).toBeVisible();
  // « boutique » stays off, its choice kept.
  await expect(projectSwitch).toHaveAttribute('aria-checked', 'false');
  await expect(own.locator('[data-code-index-origin="chosen"]')).toBeVisible();
  // « demo-api » follows it: on, read, and says where its state comes from.
  await pill('demo-api').click();
  await expect(projectSwitch).toHaveAttribute('aria-checked', 'true');
  await expect(own.locator('[data-code-index-origin="follows"]')).toContainText('Suit le réglage de l’app (allumé).');
  await expect(own.locator('[data-code-index-status]')).toContainText(/\d+ fichiers? · \d+ symboles?/, { timeout: 30_000 });
  await shots(page, own, 'index-projet-suit');
  await shots(page, all, 'index-tous-les-projets');
  // « boutique » back to the app's setting: on, read.
  await pill('boutique').click();
  await shots(page, own, 'index-projet-choisi');
  await own.getByRole('button', { name: 'Suivre le réglage de l’app' }).click();
  await expect(own.locator('[data-code-index-origin="follows"]')).toBeVisible();
  await expect(projectSwitch).toHaveAttribute('aria-checked', 'true');
  await expect(own.locator('[data-code-index-status]')).toContainText(/fichiers?/, { timeout: 30_000 });

  // Off: the app's confirmation says how many, then their index is erased.
  await allSwitch.click();
  const confirm = page.getByRole('dialog', { name: 'Couper l’index du code de 2 projets ?' });
  await expect(confirm).toBeVisible();
  await expect(confirm.getByText(/Les 2 projets qui suivent ce réglage perdent leur index/)).toBeVisible();
  await confirm.getByRole('button', { name: 'Couper et effacer' }).click();
  await expect(page.getByText('Index du code sur tous les projets coupé : l’index de 2 projets est effacé.')).toBeVisible();
  // Back to the settings, the switch off, the project off with it.
  await expect(settings).toBeVisible();
  await expect(allSwitch).toHaveAttribute('aria-checked', 'false');
  await expect(projectSwitch).toHaveAttribute('aria-checked', 'false');
  // Nothing downloaded for the search by meaning.
  expect(fs.existsSync(path.join(app.root, 'data', 'vectors'))).toBe(false);
});
