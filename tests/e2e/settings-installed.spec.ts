// The settings list only what the installed plugins bring (ESC-407) on the real app, the seed's files those of
// tests/fixtures/store/fake-store.mjs: Claude Code's plugin installed by the sort of the seed (an account of it is
// configured), then uninstalled. Réglages › Comptes et harness no longer shows it, offers no « Non installé » nor
// « Installer », its account stays kept; the search no longer finds what it alone shows; installed again in Réglages ›
// Plugins, it is back with its account. (A build of development runs the other harnesses' modules of the repository,
// which do not uninstall: they stay.)

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, openAdvanced, openSettingsPage, test } from './fixture';
import { startFakeStore } from '../fixtures/store/fake-store.mjs';

const CLAUDE = 'dev.escouade.claude-code';
const SELF_UPDATE = 'Mettre à jour Claude Code automatiquement';

let fake: { url: string; key: string; close: () => Promise<void> };

test.beforeAll(async () => {
  fake = await startFakeStore(0);
  process.env.ESCOUADE_STORE_TEST_KEY = fake.key;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-seed-'));
  await fetch(`${fake.url}/_fake/seed`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ dir, plugins: [{ id: CLAUDE, version: '2.2.0' }] }),
  });
  process.env.ESCOUADE_SEED_DIR = dir;
});

test.afterAll(async () => {
  await fake?.close();
  delete process.env.ESCOUADE_STORE_TEST_KEY;
  delete process.env.ESCOUADE_SEED_DIR;
});

/** The ids of Claude Code's accounts in the settings on the disk. */
function savedAccounts(data: string): string[] {
  const settings = JSON.parse(fs.readFileSync(path.join(data, 'settings.json'), 'utf8'));
  return (settings.accounts as { id: string; engine?: string }[]).filter((a) => !a.engine || a.engine === 'claude').map((a) => a.id);
}

/** What the search of the settings finds for `query`, by the text of each hit. */
async function search(dialog: Locator, query: string): Promise<string[]> {
  const field = dialog.getByRole('searchbox', { name: 'Rechercher un réglage' });
  await field.fill(query);
  // The list is there while something is typed, empty or not.
  const list = dialog.getByRole('listbox', { name: 'Réglages trouvés' });
  await expect(list).toBeAttached();
  const texts = await list.getByRole('option').allTextContents();
  await field.fill('');
  return texts;
}

async function uninstallClaude(page: Page) {
  if (!(await page.locator('.plugins-side').isVisible())) await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .locator('.plugins-side')
    .getByRole('button', { name: /^Installés/ })
    .click();
  await page
    .getByRole('table', { name: 'Ajoutés' })
    .getByRole('button', { name: /Claude Code/ })
    .click();
  await page.getByRole('button', { name: 'Désinstaller' }).click();
  await page.getByRole('dialog', { name: 'Désinstaller Claude Code ?' }).getByRole('button', { name: 'Désinstaller' }).click();
  await expect(page.getByRole('region', { name: 'Proposés par Escouade' })).toContainText('Claude Code');
}

test('a harness uninstalled leaves Comptes et harness and the search, its account kept, and comes back with its plugin', async ({
  app,
}) => {
  test.setTimeout(180_000);
  const { page } = app;
  await addProject(page, app.repo);
  const kept = savedAccounts(app.data);
  expect(kept).not.toEqual([]);

  // Installed by the sort of the seed: its group, its account, its update found by the search.
  let dialog = await openSettingsPage(page, 'Comptes et harness');
  await expect(dialog.getByRole('region', { name: 'Claude Code', exact: true })).toBeVisible();
  await expect(dialog.getByRole('list', { name: 'Comptes Claude Code' }).getByRole('listitem')).toHaveCount(kept.length);
  await openAdvanced(dialog);
  await expect(dialog.getByRole('list', { name: 'Ordre des harness' })).toContainText('Claude Code');
  await expect(dialog.locator('#settings-panel')).toContainText(SELF_UPDATE);
  expect((await search(dialog, 'mettre a jour claude code')).join(' ')).toContain(SELF_UPDATE);
  await dialog.locator('.s-head .close').click();
  await expect(dialog).toBeHidden();

  // Uninstalled in the view Plugins.
  await uninstallClaude(page);

  // Comptes et harness: neither its group nor an offer to install it; its account kept on the disk.
  dialog = await openSettingsPage(page, 'Comptes et harness');
  await expect(dialog.getByRole('region', { name: 'Codex', exact: true })).toBeVisible();
  await expect(dialog.getByRole('region', { name: 'Claude Code', exact: true })).toHaveCount(0);
  await expect(dialog.getByRole('list', { name: 'Comptes Claude Code' })).toHaveCount(0);
  await expect(dialog.locator('#settings-panel')).not.toContainText('Non installé');
  await expect(dialog.locator('#settings-panel').getByRole('button', { name: /^Installer/ })).toHaveCount(0);
  await openAdvanced(dialog);
  await expect(dialog.getByRole('list', { name: 'Ordre des harness' })).not.toContainText('Claude Code');
  await expect(dialog.locator('#settings-panel')).not.toContainText(SELF_UPDATE);
  expect(savedAccounts(app.data)).toEqual(kept);
  // The search follows: what Claude Code alone shows is no longer found.
  expect((await search(dialog, 'mettre a jour claude code')).join(' ')).not.toContain(SELF_UPDATE);

  // Installed again in Réglages › Plugins, from the seed: its group and its account are back, and found.
  await openSettingsPage(dialog, 'Plugins');
  await dialog.getByRole('region', { name: 'Proposés par Escouade' }).getByRole('button', { name: 'Installer Claude Code' }).click();
  await expect(page.locator('.toast').filter({ hasText: 'Plugin Claude Code installé' }).first()).toBeVisible();
  await openSettingsPage(dialog, 'Comptes et harness');
  await expect(dialog.getByRole('list', { name: 'Comptes Claude Code' }).getByRole('listitem')).toHaveCount(kept.length);
  expect((await search(dialog, 'mettre a jour claude code')).join(' ')).toContain(SELF_UPDATE);
});
