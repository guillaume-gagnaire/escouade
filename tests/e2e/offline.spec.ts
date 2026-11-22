// « Mode hors ligne » (audit SEC-18, ESC-596), on the real app against the fake store of tests/fixtures/store/fake-store.mjs:
// turned on in Réglages › Avancé › Réseau, it is said in the status bar, and the store (its catalogue and its revocation
// list) is no longer read, by itself nor at « Chercher maintenant »; imposed by a policy of this device (the debug build's
// `ESCOUADE_TEST_POLICIES`), it is on from the start, nothing is read, and its switch is locked.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { addProject, expect, openSettingsPage, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';

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

const storeReads = async () =>
  ((await (await fetch(`${fake.url}/_fake/state`)).json()).requests as string[]).filter((r) =>
    /^GET \/(api\/v1\/(catalog|revocations)\.json|mirror\/)/.test(r),
  ).length;

test('turned on, it is said in the status bar and the store is read no more', async ({ app }) => {
  test.setTimeout(180_000);
  const { page } = app;
  await addProject(page, app.repo);
  // Read at the start, while it is off.
  await expect.poll(storeReads, { timeout: 30_000 }).toBeGreaterThan(0);
  const offline = page.getByTestId('status-offline');
  await expect(offline).toHaveCount(0);

  const dialog = await openSettingsPage(page, 'Avancé');
  const sw = dialog.getByRole('switch', { name: 'Mode hors ligne' });
  await sw.scrollIntoViewIfNeeded();
  await expect(sw).toHaveAttribute('aria-checked', 'false');
  await sw.click();
  await expect(sw).toHaveAttribute('aria-checked', 'true');
  await expect(dialog).toHaveAttribute('data-saving', 'false');
  await dialog.locator('.s-head .close').click();
  await expect(offline).toHaveText('Hors ligne');

  // « Chercher maintenant » reads nothing, and says why.
  const before = await storeReads();
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Mises à jour/ })
    .click();
  const work = page.locator('.plugins-work');
  const look = work.getByRole('button', { name: 'Chercher maintenant' });
  await look.click();
  await expect(look).toBeEnabled();
  await expect(work.getByRole('status')).toHaveText(/^Mode hors ligne : offres du \d\d\/\d\d \d\d:\d\d$/);
  // « Relire » the revocation list either.
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
  await work.getByRole('button', { name: 'Relire maintenant' }).click();
  await page.waitForTimeout(2_000);
  expect(await storeReads()).toBe(before);

  // A click on the word opens its setting.
  await offline.click();
  await expect(page.getByRole('dialog', { name: 'Réglages' }).getByRole('switch', { name: 'Mode hors ligne' })).toBeVisible();
});

test.describe('imposed by a policy of this device', () => {
  const policies = path.join(os.tmpdir(), `escouade-e2e-policies-${process.pid}.json`);
  fs.writeFileSync(policies, JSON.stringify({ 'network.offline': true }));
  test.use({ appEnv: { ESCOUADE_TEST_POLICIES: policies } });

  test('it is on from the start, nothing is read, and its switch is locked', async ({ app }) => {
    test.setTimeout(120_000);
    const before = await storeReads();
    const { page } = app;
    await addProject(page, app.repo);
    await expect(page.getByTestId('status-offline')).toHaveText('Hors ligne');
    const dialog = await openSettingsPage(page, 'Avancé');
    const sw = dialog.getByRole('switch', { name: 'Mode hors ligne' });
    await sw.scrollIntoViewIfNeeded();
    await expect(sw).toHaveAttribute('aria-checked', 'true');
    await expect(sw).toBeDisabled();
    await expect(dialog.getByText(/Imposé par la stratégie de ce poste\./)).toBeVisible();
    await page.waitForTimeout(3_000);
    expect(await storeReads()).toBe(before);
  });
});
