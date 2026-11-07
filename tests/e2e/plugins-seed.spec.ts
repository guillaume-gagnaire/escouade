// The app's seed (ESC-338, docs/PLUGINS.md §8.7) on the real app, its files those of tests/fixtures/store/fake-store.mjs (the
// store's bytes and signatures, its key trusted by the debug build): installed at the first start by the store's path, a
// newer version offered and not installed, uninstalled and not put back, proposed by the view, installed again on a click;
// a false signature said; a version withdrawn stopped with its reason, the store out of reach. The configuration uses Trello
// (an account of it is connected): the sort of the seed (ESC-373) installs it, and proposes Discord, on request.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';

const TRELLO = 'dev.escouade.trello';

let fake: { url: string; key: string; close: () => Promise<void> };

test.beforeAll(async () => {
  fake = await startFakeStore(0);
  process.env.ESCOUADE_STORE_TEST_KEY = fake.key;
});

test.afterAll(async () => {
  await fake?.close();
  delete process.env.ESCOUADE_STORE_TEST_KEY;
  delete process.env.ESCOUADE_STORE_URL;
  delete process.env.ESCOUADE_STORE_MIRROR_URL;
  delete process.env.ESCOUADE_SEED_DIR;
});

const post = (p: string, body: unknown) =>
  fetch(`${fake.url}${p}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });

/** A seed of those versions, as the release makes it, for the app the fixture starts next. */
async function seed(plugins: { id: string; version: string }[]) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-seed-'));
  await post('/_fake/seed', { dir, plugins });
  process.env.ESCOUADE_SEED_DIR = dir;
  return dir;
}

/** A Trello account connected, with no secret: the configuration uses Trello. */
const usesTrello = {
  run: (root: string) => {
    fs.writeFileSync(path.join(root, 'data', 'integrations.json'), JSON.stringify({ trello: {} }));
  },
};

async function openInstalled(page: Page) {
  if (!(await page.locator('.plugins-side').isVisible())) await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
}

test.describe('the seed, the store in reach', () => {
  test.use({ preStart: usesTrello });
  test.beforeAll(async () => {
    await seed([{ id: TRELLO, version: '2.2.0' }]);
    process.env.ESCOUADE_STORE_URL = fake.url;
    process.env.ESCOUADE_STORE_MIRROR_URL = `${fake.url}/mirror`;
  });

  test.afterAll(() => {
    delete process.env.ESCOUADE_STORE_URL;
    delete process.env.ESCOUADE_STORE_MIRROR_URL;
  });

  test('is installed by itself, offered its update, uninstalled and proposed, never put back', async ({ app }) => {
    test.setTimeout(180_000);
    let { page } = app;
    await addProject(page, app.repo);
    await openInstalled(page);
    const added = page.getByRole('table', { name: 'Ajoutés' });
    await expect(added).toContainText('Trello');
    await expect(page.getByRole('table', { name: 'Livrés avec Escouade' })).not.toContainText('Trello');

    // The store offers 2.3.0: said, and not installed by itself.
    await page.getByRole('button', { name: 'Relire maintenant' }).click();
    await expect(added).toContainText('2.3.0');
    await expect(added).toContainText('2.2.0');

    // Its page: from the seed, and it uninstalls like any plugin.
    await added.getByRole('button', { name: /Trello/ }).click();
    await expect(page.locator('.plugins-work')).toContainText('l’amorce d’Escouade, sans réseau');
    await page.getByRole('button', { name: 'Désinstaller' }).click();
    await page.getByRole('dialog', { name: 'Désinstaller Trello ?' }).getByRole('button', { name: 'Désinstaller' }).click();
    const proposed = page.getByRole('region', { name: 'Proposés par Escouade' });
    await expect(proposed).toContainText('Tu l’as désinstallé (v2.2.0)');

    // Started anew, the seed does not put it back.
    page = await app.restart();
    await openInstalled(page);
    await expect(page.getByRole('region', { name: 'Proposés par Escouade' })).toContainText('Trello');
    await expect(page.getByRole('table', { name: 'Ajoutés' })).toHaveCount(0);

    // Installed on a click: the store has a newer version, which opens the usual window.
    await page.getByRole('region', { name: 'Proposés par Escouade' }).getByRole('button', { name: 'Installer' }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toContainText('Trello');
    await dialog.getByRole('button', { name: /^(Installer|Mettre à jour)/ }).click();
    await expect(dialog).toBeHidden();
    await expect(page.getByRole('region', { name: 'Proposés par Escouade' })).toHaveCount(0);
  });
});

test.describe('the seed with a false signature, the store out of reach', () => {
  test.use({ preStart: usesTrello });
  test.beforeAll(async () => {
    const dir = await seed([
      { id: TRELLO, version: '2.2.0' },
      { id: 'dev.escouade.discord', version: '1.1.0' },
    ]);
    fs.copyFileSync(path.join(dir, 'dev.escouade.discord', 'version.json.minisig'), path.join(dir, TRELLO, 'version.json.minisig'));
  });

  test('the plugin stays out, its reason said; Discord, on request, proposed', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await openInstalled(page);
    const proposed = page.getByRole('region', { name: 'Proposés par Escouade' });
    await expect(proposed).toContainText('Non installé');
    await expect(proposed).toContainText('La signature de cette version ne tient pas');
    await expect(proposed).toContainText('Discord');
    await expect(proposed).toContainText('Dans l’app, sans téléchargement');
    await expect(page.getByRole('table', { name: 'Ajoutés' })).toHaveCount(0);
  });
});

test.describe('a version of the seed withdrawn, the store out of reach', () => {
  test.use({ preStart: usesTrello });
  test.beforeAll(async () => {
    await post('/_fake/revocations', {
      entries: [
        {
          id: TRELLO,
          versions: ['2.2.0'],
          action: 'disable',
          reason: { fr: 'Envoyait les jetons ailleurs.', en: 'Sent the tokens elsewhere.' },
          date: '2026-11-05',
        },
      ],
    });
    await seed([{ id: TRELLO, version: '2.2.0' }]);
  });

  test('is stopped with its reason', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo);
    await openInstalled(page);
    const band = page.locator('.band');
    await expect(band).toContainText('Trello v2.2.0 a été retiré par Escouade le 2026-11-05.');
    await expect(band).toContainText('Envoyait les jetons ailleurs.');
    await expect(page.getByRole('table', { name: 'Ajoutés' })).toContainText('Désactivé par Escouade');
    await expect(page.locator('.plugins-work')).toContainText('nº 1');
  });
});
