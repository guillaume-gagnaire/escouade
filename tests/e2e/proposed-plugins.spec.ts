// The plugins a project proposes (PLG-14, ESC-288) on the real app, against the fake store of
// tests/fixtures/store/fake-store.mjs: a repository whose .escouade.yml has plugins.recommended is added, the opening
// says it proposes plugins; the band of Settings › Project names those not installed, each with its page in the store,
// and installs nothing; one installed from its page leaves the band; « Pas pour ce projet » silences it, and it stays
// silent after a restart of the window. Captures of the band in both themes go to ESCOUADE_E2E_SHOTS when it is set.

import { execFileSync } from 'node:child_process';
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

const FILE = `version: 1
plugins:
  recommended:
    - dev.exemple.slack
    - dev.exemple.team-rules
    - dev.exemple.high-contrast
  dev.exemple.slack:
    channel: '#equipe'
`;

const state = async () => (await fetch(`${fake.url}/_fake/state`)).json();

async function projectSettings(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await dialog.getByRole('tab', { name: 'Projets' }).click();
  return dialog;
}

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

test('a project proposes its plugins: named with their store page, nothing installed by itself, silenced for the project', async ({
  app,
}) => {
  test.setTimeout(180_000);
  const { page } = app;
  // Committed, as a team shares it: the repository stays clean.
  fs.writeFileSync(path.join(app.repo, '.escouade.yml'), FILE);
  execFileSync('git', ['add', '.escouade.yml'], { cwd: app.repo, stdio: 'pipe' });
  execFileSync('git', ['commit', '-qm', 'escouade.yml'], { cwd: app.repo, stdio: 'pipe' });
  await addProject(page, app.repo);
  await expect(page.getByText(/propose 3 plugins : Réglages › Projets\./)).toBeVisible();

  // The band: the three, what each asks for, nothing installed.
  let settings = await projectSettings(page);
  const band = settings.getByRole('region', { name: 'Plugins proposés par ce projet' });
  await expect(band).toContainText('Ce projet propose 3 plugins');
  await expect(band.locator('li')).toHaveCount(3);
  await expect(band).toContainText('Slack');
  await expect(band).toContainText("Règles de l'équipe");
  await expect(band).toContainText('Lance un programme');
  if (SHOTS) {
    fs.mkdirSync(SHOTS, { recursive: true });
    for (const t of ['dark', 'light'] as const) {
      await theme(page, t);
      await band.scrollIntoViewIfNeeded();
      await settings.screenshot({ path: path.join(SHOTS, `app-proposed-plugins-${t}.png`) });
    }
    await theme(page, 'dark');
  }

  // « Voir dans le store »: the plugin's page, nothing downloaded.
  await band.locator('li', { hasText: "Règles de l'équipe" }).getByRole('button', { name: 'Voir dans le store' }).click();
  await expect(settings).toBeHidden();
  const head = page.locator('.phead');
  await expect(head).toContainText("Règles de l'équipe");
  const asked: string[] = (await state()).requests;
  expect(asked.filter((r) => /archive|version\.json|\/download/.test(r))).toEqual([]);

  // Installed from its page by the user, its permissions shown first: it leaves the band.
  await page.getByRole('button', { name: /^Installer la v/ }).click();
  const dialog = page.getByRole('dialog', { name: "Installer Règles de l'équipe ?" });
  await expect(dialog).toContainText('Décide des autorisations');
  await dialog.getByRole('button', { name: 'Installer', exact: true }).click();
  await expect(dialog).toBeHidden();

  settings = await projectSettings(page);
  await expect(band).toContainText('Ce projet propose 2 plugins');
  await expect(band).not.toContainText("Règles de l'équipe");

  // « Pas pour ce projet »: silent, and still after the window is loaded again.
  await band.getByRole('button', { name: 'Pas pour ce projet' }).click();
  await expect(band).toBeHidden();
  await settings.getByRole('button', { name: 'Annuler' }).click();
  await page.reload();
  settings = await projectSettings(page);
  await expect(settings.getByRole('button', { name: 'Exporter vers .escouade.yml' })).toBeVisible();
  await expect(settings.getByRole('region', { name: 'Plugins proposés par ce projet' })).toHaveCount(0);
});
