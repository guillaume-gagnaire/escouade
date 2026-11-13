// The updates' page (ESC-436) on the real app, against the fake store of tests/fixtures/store/fake-store.mjs: an update
// leaves the page where it was (a new install alone opens the plugin's page); « Tout mettre à jour · N » at its head asks
// once for those verified that ask for nothing more, then each of the others in its own window, what it adds marked; a
// version not verified of a plugin whose installed one was (essai B-16 of the audit, SEC-29, ESC-438) has its own window,
// its warning said, never the window for all. The versions
// are published after the start (POST /_fake/release), the store read again 31 s later (ESC-405). ESCOUADE_E2E_SHOTS names
// a folder for the screenshots of the page before and after.
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
  await page.screenshot({ path: path.join(SHOTS, `updates-${name}${size}.png`), animations: 'disabled' });
}

/** Installs `search`'s line of the store that holds `line`, by `button` of its window: a new install opens its page. */
async function installFromStore(page: Page, search: string, line: string, button: string) {
  const work = page.locator('.plugins-work');
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Store/ })
    .click();
  await expect(work.locator('.sub')).toContainText('catalogue lu à');
  await page.getByRole('searchbox', { name: 'Chercher dans le store' }).fill(search);
  await work.locator('.rowp', { hasText: line }).first().getByRole('button', { name: 'Installer' }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: button, exact: true }).click();
  await expect(dialog).toBeHidden();
  await expect(page.locator('.phead')).toBeVisible();
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

test('an update leaves the updates page where it is; « Tout mettre à jour » asks once for those that ask for nothing more', async ({
  app,
}) => {
  test.setTimeout(300_000);
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  const work = page.locator('.plugins-work');
  const ups = work.locator('.ups');

  // New installs: each opens its plugin's page.
  await installFromStore(page, 'contraste', 'Contraste élevé', 'Installer quand même');
  await installFromStore(page, 'slack', 'escouade-dev', 'Installer quand même');
  await installFromStore(page, 'nuit', 'Nuit', 'Installer');

  // One update, from its line: its window, what it adds marked; then the page stays the updates'.
  await release('dev.exemple.slack', '1.1.0');
  await lookForUpdates(page);
  const line = ups.locator('.up', { hasText: 'Slack' });
  await expect(line).toContainText('v1.0.0 → v1.1.0');
  await line.getByRole('button', { name: 'Mettre à jour' }).click();
  const own = page.getByRole('dialog', { name: 'Mettre à jour Slack ?' });
  await expect(own).toContainText('Cette version demande plus que la v1.0.0 installée');
  await expect(own.locator('.new')).toHaveCount(1);
  await expect(own).toContainText('files.slack.com');
  await own.getByRole('button', { name: 'Mettre à jour', exact: true }).click();
  await expect(own).toBeHidden();
  await expect(line).toContainText('À jour en v1.1.0');
  await expect(work.getByText('Tes plugins sont à jour.')).toBeVisible();
  await expect(work.locator('h2')).toHaveText('Mises à jour');
  await expect(page.locator('.phead')).toHaveCount(0);

  // Three updates: the high contrast's, verified, asks for nothing more; Slack's connects to one more host; Nuit's 1.1.0
  // asks for nothing more either, but nobody reviewed it, when its 1.0.0 installed was verified (B-16).
  await release('dev.exemple.high-contrast', '1.2.0');
  await release('dev.exemple.slack', '1.2.0');
  await release('dev.exemple.nuit', '1.1.0');
  await lookForUpdates(page);
  await expect(ups.locator('.up', { hasText: 'Nuit' })).toContainText('v1.0.0 → v1.1.0');
  await expect(ups.locator('.up', { hasText: 'Slack' })).toContainText('v1.1.0 → v1.2.0');
  await expect(ups.locator('.up', { hasText: 'Contraste élevé' })).toContainText('v1.1.0 → v1.2.0');
  const all = work.getByRole('button', { name: 'Tout mettre à jour · 3' });
  await expect(all).toBeInViewport();
  await shot(page, 'before');
  await all.click();

  // One window for those verified that ask for nothing more, and it says so: nothing installed before its gesture.
  const batch = page.getByRole('dialog', { name: 'Mettre à jour 1 plugin ?' });
  await expect(batch).toContainText('Contraste élevé');
  await expect(batch).toContainText('v1.1.0 → v1.2.0');
  await expect(batch).toContainText('Vérifié par Escouade');
  await expect(batch).toContainText('Toutes ces versions sont vérifiées par Escouade');
  await expect(batch).not.toContainText('Nuit');
  await expect(batch).not.toContainText('Non vérifié');
  await expect(batch).toContainText('Ensuite, 2 mises à jour qui demandent plus, ou ne sont pas vérifiées');
  await expect(ups.locator('.up', { hasText: 'Slack' })).toContainText('En attente de ton accord');
  await expect(ups.locator('.up', { hasText: 'Nuit' })).toContainText('En attente de ton accord');
  await shot(page, 'batch');
  await batch.getByRole('button', { name: 'Mettre à jour', exact: true }).click();

  // Then Slack's and Nuit's, each in its own window: Slack's what it adds marked, Nuit's the warning of a version nobody
  // reviewed (B-16), as when it is asked for alone.
  const slackOwn = page.getByRole('dialog', { name: 'Mettre à jour Slack ?' });
  const nuitOwn = page.getByRole('dialog', { name: 'Mettre à jour Nuit ?' });
  for (let n = 0; n < 2; n++) {
    await expect(slackOwn.or(nuitOwn)).toBeVisible();
    const own = (await nuitOwn.isVisible()) ? nuitOwn : slackOwn;
    if (own === nuitOwn) {
      await expect(own).toContainText('Personne n’a relu cette version.');
      await expect(own).toContainText('Aucune : version non vérifiée');
      await expect(own).not.toContainText('Vérifié par Escouade');
      await shot(page, 'unverified');
    } else {
      await expect(own).toContainText('hooks.slack.com');
      await expect(own.locator('.new')).toHaveCount(1);
      await shot(page, 'own');
    }
    await expect(ups.locator('.up', { hasText: 'Contraste élevé' })).toContainText('À jour en v1.2.0');
    await own.getByRole('button', { name: 'Mettre à jour', exact: true }).click();
    await expect(own).toBeHidden();
  }

  // The page stays the updates', and says so.
  await expect(work.locator('h2')).toHaveText('Mises à jour');
  await expect(work.getByText('Tes plugins sont à jour.')).toBeVisible();
  await expect(ups.locator('.up', { hasText: 'Slack' })).toContainText('À jour en v1.2.0');
  await expect(work.getByRole('button', { name: /Tout mettre à jour/ })).toHaveCount(0);
  await shot(page, 'after');

  // What is installed is what was agreed to.
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
  const added = page.getByRole('table', { name: 'Ajoutés' });
  await expect(added.getByRole('row', { name: /^Slack/ })).toContainText('1.2.0');
  await expect(added.getByRole('row', { name: /^Contraste élevé/ })).toContainText('1.2.0');
  await expect(added.getByRole('row', { name: /^Nuit/ })).toContainText('1.1.0');
});
