// The policies of this device read again while the app runs (TS-12): the e2e's source of policies (a JSON file named by
// ESCOUADE_TEST_POLICIES, read by a build of development alone) is changed with the app open, and the window follows
// without a restart: the setting it imposes greyed, then free again when it is removed.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { expect, test, openSettingsPage } from './fixture';

let file = '';

test.use({
  appEnv: async ({}, use) => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-policies-live-'));
    file = path.join(dir, 'policies.json');
    fs.writeFileSync(file, JSON.stringify({ 'telemetry.enabled': false }));
    await use({ ESCOUADE_TEST_POLICIES: file });
    fs.rmSync(dir, { recursive: true, force: true });
  },
  // On before the start (the e2e's settings have it off): what the policy turns off.
  preStart: { run: () => ({ autoUpdate: true }) },
});

test('a policy set while the app runs greys its setting at once, and frees it when removed', async ({ app }) => {
  const { page, data } = app;
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Général');
  const auto = dialog.locator('.sr', { has: page.getByRole('switch', { name: 'Installer les mises à jour automatiquement' }) });
  // Not imposed at the start, and on.
  await expect(auto).not.toHaveClass(/locked/);
  await expect(auto.getByRole('switch')).toHaveAttribute('aria-checked', 'true');

  // The IT department turns the automatic updates off: a 0, as a REG_DWORD of Group Policy would be.
  fs.writeFileSync(file, JSON.stringify({ 'telemetry.enabled': false, 'updates.automatic': 0 }));
  await expect(auto).toHaveClass(/locked/, { timeout: 15_000 });
  await expect(auto.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
  await expect(auto).toContainText('Imposé par la stratégie de ce poste');
  await expect
    .poll(() => JSON.parse(fs.readFileSync(path.join(data, 'settings.json'), 'utf8')).autoUpdate, { timeout: 10_000 })
    .toBe(false);

  // Removed: the setting is free again, as the policy left it; the other policy stays.
  fs.writeFileSync(file, JSON.stringify({ 'telemetry.enabled': false }));
  await expect(auto).not.toHaveClass(/locked/, { timeout: 15_000 });
  await expect(auto.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
  const stats = dialog.locator('.sr', { has: page.getByRole('switch', { name: 'Statistiques d’usage' }) });
  await expect(stats).toHaveClass(/locked/);
});
