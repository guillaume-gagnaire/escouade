// The sort of the seed (ESC-373, docs/PLUGINS.md §8.7) on the real app, its files those of tests/fixtures/store/fake-store.mjs:
// a first installation installs nothing; « Ajouter un compte… » on a harness at the start installs its plugin from the seed,
// with no network, and says it; the view Plugins proposes the others and installs one in a click; a plugin uninstalled is
// not put back at the next start; Réglages › Plugins (ESC-384) proposes it again, and installs it in a click.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, FAKE_COPILOT, openSettingsPage, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';

const COPILOT = 'dev.escouade.copilot-cli';
const TRELLO = 'dev.escouade.trello';

let fake: { url: string; key: string; close: () => Promise<void> };

test.beforeAll(async () => {
  fake = await startFakeStore(0);
  process.env.ESCOUADE_STORE_TEST_KEY = fake.key;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-seed-'));
  await fetch(`${fake.url}/_fake/seed`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      dir,
      plugins: [
        { id: COPILOT, version: '2.2.0' },
        { id: TRELLO, version: '2.2.0' },
      ],
    }),
  });
  process.env.ESCOUADE_SEED_DIR = dir;
});

test.afterAll(async () => {
  await fake?.close();
  delete process.env.ESCOUADE_STORE_TEST_KEY;
  delete process.env.ESCOUADE_SEED_DIR;
});

async function openInstalled(page: Page) {
  if (!(await page.locator('.plugins-side').isVisible())) await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
}

/** The ids `plugins.json` lists. */
function installed(data: string): string[] {
  const file = path.join(data, 'plugins.json');
  if (!fs.existsSync(file)) return [];
  return (JSON.parse(fs.readFileSync(file, 'utf8')).plugins as { id: string }[]).map((p) => p.id);
}

test.describe('a first installation, the seed sorted', () => {
  test.use({ appEnv: { ESCOUADE_COPILOT: FAKE_COPILOT }, preStart: { run: () => ({ noAccounts: true }) } });

  test('installs nothing; a harness chosen at the start installs its plugin, with no network, and says it', async ({ app }) => {
    const { page } = app;
    const welcome = page.getByRole('region', { name: 'Aucun compte' });
    const copilot = welcome.getByRole('listitem').filter({ hasText: 'Copilot CLI' });
    await expect(copilot).toBeVisible();
    // Nothing of the seed installed by itself.
    expect(installed(app.data)).not.toContain(COPILOT);
    expect(installed(app.data)).not.toContain(TRELLO);

    // « Ajouter un compte… » on Copilot CLI: its plugin from the seed, said in a line, then the account's window.
    await copilot.getByRole('button', { name: 'Ajouter un compte…' }).click();
    await expect(page.locator('.toast').filter({ hasText: 'Plugin Copilot CLI installé' }).first()).toBeVisible();
    await expect(page.getByRole('dialog', { name: 'Ajouter un compte Copilot CLI' })).toBeVisible();
    await expect.poll(() => installed(app.data)).toContain(COPILOT);
    expect(installed(app.data)).not.toContain(TRELLO);
  });
});

test.describe('the seed sorted, the view Plugins', () => {
  test('proposes what the configuration does not use, installs it in a click, and does not put back what was uninstalled', async ({
    app,
  }) => {
    test.setTimeout(180_000);
    let { page } = app;
    // A Claude Code account, whose plugin this seed does not hold: nothing of it installed by itself.
    await addProject(page, app.repo);
    await openInstalled(page);
    const proposed = page.getByRole('region', { name: 'Proposés par Escouade' });
    await expect(proposed).toContainText('Copilot CLI');
    await expect(proposed).toContainText('Trello');
    await expect(proposed).toContainText('Dans l’app, sans téléchargement');
    await expect(page.getByRole('table', { name: 'Ajoutés' })).toHaveCount(0);

    // « Installer »: in one click, no window, said in a line.
    await proposed.locator('.line').filter({ hasText: 'Trello' }).getByRole('button', { name: 'Installer' }).click();
    await expect(page.locator('.toast').filter({ hasText: 'Plugin Trello installé' }).first()).toBeVisible();
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(page.getByRole('table', { name: 'Ajoutés' })).toContainText('Trello');
    await expect(proposed).not.toContainText('Trello');

    // Uninstalled, it is not put back at the next start.
    await page
      .getByRole('table', { name: 'Ajoutés' })
      .getByRole('button', { name: /Trello/ })
      .click();
    await page.getByRole('button', { name: 'Désinstaller' }).click();
    await page.getByRole('dialog', { name: 'Désinstaller Trello ?' }).getByRole('button', { name: 'Désinstaller' }).click();
    await expect(page.getByRole('region', { name: 'Proposés par Escouade' })).toContainText('Tu l’as désinstallé (v2.2.0)');
    page = await app.restart();
    await openInstalled(page);
    await expect(page.getByRole('region', { name: 'Proposés par Escouade' })).toContainText('Tu l’as désinstallé (v2.2.0)');
    await expect(page.getByRole('table', { name: 'Ajoutés' })).toHaveCount(0);
    expect(installed(app.data)).not.toContain(TRELLO);

    // Réglages › Plugins: the seed's not installed, said why; « Installer » in one click, with no window, from the seed.
    const settings = await openSettingsPage(page, 'Plugins');
    const seed = settings.getByRole('region', { name: 'Proposés par Escouade' });
    await expect(seed).toContainText('Dans l’installeur, rien à télécharger');
    await expect(seed.getByRole('group', { name: 'Copilot CLI' })).toContainText('Dans l’app, sans téléchargement');
    await expect(seed.getByRole('group', { name: 'Trello' })).toContainText('Tu l’as désinstallé (v2.2.0)');
    await seed.getByRole('button', { name: 'Installer Trello' }).click();
    await expect(page.locator('.toast').filter({ hasText: 'Plugin Trello installé' }).first()).toBeVisible();
    await expect(settings.getByRole('dialog')).toHaveCount(0);
    await expect(settings.getByRole('region', { name: 'Installés' }).getByRole('group', { name: 'Trello' })).toContainText(
      'Actif sur 1 projet sur 1',
    );
    await expect(seed.getByRole('group', { name: 'Trello' })).toHaveCount(0);
    await expect.poll(() => installed(app.data)).toContain(TRELLO);

    // The page, to set beside design/settings-2-2/captures/a-plugins*.png, when asked for.
    const shots = process.env.ESCOUADE_E2E_SHOTS;
    if (shots) {
      fs.mkdirSync(shots, { recursive: true });
      await page.setViewportSize({ width: 1440, height: 900 });
      await page.mouse.move(2, 400);
      for (const t of ['dark', 'light'] as const) {
        await page.evaluate((t) => (document.documentElement.dataset.theme = t), t);
        await page.screenshot({ path: path.join(shots, `app-a-plugins${t === 'light' ? '-light' : ''}.png`) });
      }
    }
  });
});
