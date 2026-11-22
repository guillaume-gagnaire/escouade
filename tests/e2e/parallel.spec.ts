// « En parallèle » of the Kanban (ESC-620): from 1 to 16 on a desktop, beyond 6 a warning from the machine and the quota
// that refuses nothing, and a ceiling the device's policy imposes, said with where it comes from. The app is started
// with a source of policies of the tests (ESCOUADE_TEST_POLICIES, a build of development alone) for the ceiling.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';

test('the tickets in parallel go beyond 6 up to 16, with a warning that refuses nothing', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { worktrees: true, firstAgent: false });
  const settings = await openSettingsPage(page, 'Kanban');
  const field = settings.getByRole('spinbutton', { name: 'En parallèle' });
  await expect(field).toHaveValue('2');
  await expect(settings.getByText('De 1 à 16')).toBeVisible();
  await expect(settings.locator('[data-parallel-warning]')).toHaveCount(0);

  // 12: taken, and the warning says this machine, the room of an agent and the quota.
  await field.fill('12');
  const warning = settings.locator('[data-parallel-warning]');
  await expect(warning).toContainText('Au-delà de 6 : ce poste (');
  await expect(warning).toContainText('cœurs');
  await expect(warning).toContainText('mesuré sur le dépôt d’Escouade');
  await expect(warning).toContainText('Le quota du compte part aussi plus vite.');
  // Beyond the bound: brought back to 16 when the field is left.
  await field.fill('40');
  await field.blur();
  await expect(field).toHaveValue('16');
  await closeSettings(settings);

  // Kept: the Kanban's settings say 16 again.
  const again = await openSettingsPage(page, 'Kanban');
  await expect(again.getByRole('spinbutton', { name: 'En parallèle' })).toHaveValue('16');
  await expect(again.locator('[data-parallel-ceiling]')).toHaveCount(0);
});

test.describe('under a ceiling imposed by the device', () => {
  test.use({
    appEnv: async ({}, use) => {
      const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-parallel-'));
      const file = path.join(dir, 'policies.json');
      fs.writeFileSync(file, JSON.stringify({ 'projects.maxParallel': 8 }));
      await use({ ESCOUADE_TEST_POLICIES: file });
      fs.rmSync(dir, { recursive: true, force: true });
    },
  });

  test('the ceiling and where it comes from are said under the setting, which goes no higher', async ({ app }) => {
    const { page } = app;
    await addProject(page, app.repo, { worktrees: true, firstAgent: false });
    const settings = await openSettingsPage(page, 'Kanban');
    await expect(settings.locator('[data-parallel-ceiling]')).toHaveText('Plafond de 8, imposé par la stratégie de ce poste');
    await expect(settings.getByText('De 1 à 8')).toBeVisible();
    const field = settings.getByRole('spinbutton', { name: 'En parallèle' });
    // Not greyed: set below it.
    await expect(field).toBeEnabled();
    await field.fill('12');
    await field.blur();
    await expect(field).toHaveValue('8');
    await expect(settings.locator('[data-parallel-warning]')).toContainText('Au-delà de 6');
  });
});
