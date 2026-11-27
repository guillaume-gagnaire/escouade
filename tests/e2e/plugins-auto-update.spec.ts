// « Mettre à jour tout seuls » (ESC-700) on the real app, against the fake store of tests/fixtures/store/fake-store.mjs:
// the setting, beside « N'installer que des versions vérifiées » and off by default, turned on; a version of the store,
// verified, that asks for nothing more installs by itself after « Chercher maintenant », said at the head of the page
// Mises à jour and in the tab Versions; a version that asks for more stays offered, for a gesture. The versions are
// published after the start (POST /_fake/release), the store read again 31 s later (ESC-405). ESCOUADE_E2E_SHOTS names a
// folder for the screenshots.
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
let fake: { url: string; key: string; close: () => Promise<void> };

test.beforeAll(async () => {
  fake = await startFakeStore(0);
  process.env.ESCOUADE_STORE_URL = fake.url;
  process.env.ESCOUADE_STORE_MIRROR_URL = `${fake.url}/mirror`;
  process.env.ESCOUADE_STORE_TEST_KEY = fake.key;
});

test.afterAll(async () => {
  await fake?.close();
  delete process.env.ESCOUADE_STORE_URL;
  delete process.env.ESCOUADE_STORE_MIRROR_URL;
  delete process.env.ESCOUADE_STORE_TEST_KEY;
});

const release = async (id: string, version: string) =>
  expect(
    (
      await fetch(`${fake.url}/_fake/release`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ id, version }),
      })
    ).status,
  ).toBe(200);

async function shot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const size = process.env.ESCOUADE_E2E_VIEWPORT ? `-${process.env.ESCOUADE_E2E_VIEWPORT}` : '';
  await page.screenshot({ path: path.join(SHOTS, `auto-${name}${size}.png`), animations: 'disabled' });
}

/** The updates' page, the store read again (older than 30 s by then: ESC-405). */
async function lookForUpdates(page: Page) {
  await page.waitForTimeout(31_000);
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Mises à jour/ })
    .click();
  const look = page.locator('.plugins-work').getByRole('button', { name: 'Chercher maintenant' });
  await look.click();
  await expect(look).toBeEnabled();
}

test('a verified version that asks for nothing more installs by itself; one that asks for more stays offered', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  const work = page.locator('.plugins-work');
  const side = page.locator('.plugins-side');

  // Aube 1.0.0, verified, from the store.
  await side.getByRole('button', { name: /^Store/ }).click();
  await expect(work.locator('.sub')).toContainText('catalogue lu à');
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill('aube');
  await work.locator('.rowp', { hasText: 'Aube' }).first().getByRole('button', { name: 'Installer' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'Installer', exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.phead')).toBeVisible();

  // The setting, beside « N'installer que des versions vérifiées », off by default: turned on.
  await side.getByRole('button', { name: /^Installés/ }).click();
  const menu = work.getByRole('combobox', { name: 'Mettre à jour tout seuls' });
  await expect(menu).toBeInViewport();
  await expect(work.getByRole('switch', { name: 'N’installer que des versions vérifiées' })).toBeInViewport();
  await expect(menu).toHaveValue('never');
  await menu.selectOption('verified');
  await expect(menu).toHaveValue('verified');
  await shot(page, 'setting');

  // 1.1.0, verified, asks for nothing more: installed by itself after « Chercher maintenant ».
  await release('dev.exemple.aube', '1.1.0');
  await lookForUpdates(page);
  const done = work.getByText(/1 plugin mis à jour tout seul aujourd’hui/);
  await expect(done).toBeVisible();
  await expect(done).toContainText('Aube');
  await expect(done).toContainText('v1.0.0 → v1.1.0');
  await expect(work.getByText('Tes plugins sont à jour.')).toBeVisible();
  await expect(work.locator('.sub').first()).toContainText('les vérifiées qui ne demandent rien de plus s’installent seules');
  await shot(page, 'done');

  // Its tab Versions says it.
  await done.getByRole('button', { name: 'Aube' }).click();
  await expect(page.locator('.phead')).toBeVisible();
  await expect(page.getByText(/mise à jour toute seule le .*, depuis la v1\.0\.0/)).toBeVisible();
  await expect(page.locator('.vers')).toContainText('1.1.0');

  // 1.2.0, verified, connects to a host: it asks for more, so it stays offered and waits for a gesture.
  await release('dev.exemple.aube', '1.2.0');
  await lookForUpdates(page);
  const line = work.locator('.ups .up', { hasText: 'Aube' });
  await expect(line).toContainText('v1.1.0 → v1.2.0');
  await expect(line.getByRole('button', { name: 'Mettre à jour' })).toBeEnabled();
  await expect(work.getByRole('button', { name: 'Tout mettre à jour · 1' })).toBeInViewport();
  await shot(page, 'offered');
  // Its window says what it adds: nothing installed before the gesture.
  await line.getByRole('button', { name: 'Mettre à jour' }).click();
  const own = page.getByRole('dialog', { name: 'Mettre à jour Aube ?' });
  await expect(own).toContainText('aube.example');
  await own.getByRole('button', { name: 'Annuler' }).click();
  await expect(own).toBeHidden();
  await expect(line).toContainText('v1.1.0 → v1.2.0');
});
