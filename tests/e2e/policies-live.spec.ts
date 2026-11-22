// The policies of this device read again while the app runs (TS-12): the e2e's source of policies (a JSON file named by
// ESCOUADE_TEST_POLICIES, read by a build of development alone) is changed with the app open, and the window follows
// without a restart: the setting it imposes greyed, then free again when it is removed.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { addProject, expect, test, openSettingsPage } from './fixture';

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

// ESC-564: `accounts.reservation` set to `required` while the app runs. A private project allowed to go on a shared account
// at the limit has it turned off at once, locked with who imposes it, and free again once the policy is removed.
test('a reservation required while the app runs locks « Comptes partagés à la limite » of a private project', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo, { name: 'client', firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Confidentialité', { project: 'client' });
  await dialog.getByRole('switch', { name: 'Projet privé' }).click();
  const limit = dialog.locator('.sr', { has: page.getByRole('switch', { name: 'Comptes partagés à la limite' }) });
  await limit.getByRole('switch').click();
  await expect(limit.getByRole('switch')).toHaveAttribute('aria-checked', 'true');
  await expect(limit).not.toHaveClass(/locked/);
  await expect(dialog.getByTestId('reservation-lock')).toHaveCount(0);

  fs.writeFileSync(file, JSON.stringify({ 'telemetry.enabled': false, 'accounts.reservation': 'required' }));
  await expect(limit).toHaveClass(/locked/, { timeout: 15_000 });
  await expect(limit.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
  await expect(limit).toContainText('Imposé par la stratégie de ce poste');
  await expect(dialog.getByTestId('reservation-lock')).toHaveText(
    'Imposé par la stratégie de ce poste · Ses agents ne partent que sur un compte réservé à lui seul, jamais sur un compte partagé, même à la limite.',
  );

  // Removed: free again, as the policy left it.
  fs.writeFileSync(file, JSON.stringify({ 'telemetry.enabled': false }));
  await expect(limit).not.toHaveClass(/locked/, { timeout: 15_000 });
  await expect(dialog.getByTestId('reservation-lock')).toHaveCount(0);
  await expect(limit.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
});

// ESC-564: `plugins.projectSettings`, a setting of each project of a plugin imposed by this device while the app runs.
// The page Plugins of the project greys it with its padlock and the value imposed; a secret it names is refused, said;
// removed, the setting is free again.
test('a plugin’s setting of each project imposed while the app runs is greyed with its padlock, never a secret', async ({ app }) => {
  const { page } = app;
  const ID = 'dev.test.routes';
  const src = path.join(app.root, 'routes-plugin');
  fs.mkdirSync(src, { recursive: true });
  fs.copyFileSync(path.resolve(import.meta.dirname, '..', 'fixtures', 'plugins', 'fake', 'plugin.mjs'), path.join(src, 'plugin.mjs'));
  fs.writeFileSync(
    path.join(src, 'escouade-plugin.json'),
    JSON.stringify({
      id: ID,
      name: 'Routes',
      version: '1.0.0',
      author: { name: 'escouade-tests' },
      license: 'MIT',
      description: 'Des routes par projet.',
      escouade: { api: '1', min: '2.1.0' },
      run: { js: 'plugin.mjs' },
      activation: ['event:ticket.created'],
      permissions: { events: ['ticket.created'], secrets: ['token'] },
      contributes: {
        secrets: [{ key: 'token', title: 'Jeton' }],
        settings: [{ key: 'routes', type: 'string', scope: 'project', title: 'Routes du projet' }],
      },
    }),
  );
  await addProject(page, app.repo, { name: 'client', firstAgent: false });
  const invoke = <T>(cmd: string, args: Record<string, unknown> = {}) =>
    page.evaluate(
      ([cmd, args]) =>
        (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
          cmd as string,
          args,
        ),
      [cmd, args] as const,
    ) as Promise<T>;
  const { token } = await invoke<{ token: string }>('plugin_inspect_folder', { path: src });
  await invoke('plugin_install', { token });

  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  const nav = dialog.getByRole('navigation');
  const row = nav
    .locator('button[aria-expanded]')
    .filter({ has: page.locator('.lbl', { hasText: 'client' }) })
    .first();
  if ((await row.getAttribute('aria-expanded')) !== 'true') await row.click();
  await nav.getByRole('group', { name: 'client' }).getByRole('button', { name: 'Plugins', exact: true }).click();
  const routes = dialog.getByRole('group', { name: 'Routes du projet' });
  await expect(routes.getByLabel('Routes du projet')).toBeEnabled();
  await expect(routes.getByTestId('plugin-team-lock')).toHaveCount(0);

  fs.writeFileSync(
    file,
    JSON.stringify({ 'telemetry.enabled': false, 'plugins.projectSettings': { [ID]: { routes: '10.8.0.0/16', token: 'pas-ici' } } }),
  );
  await expect(routes.getByTestId('plugin-team-lock')).toHaveText('Imposé par la stratégie de ce poste', { timeout: 15_000 });
  await expect(routes.getByLabel('Routes du projet')).toHaveValue('10.8.0.0/16');
  await expect(routes.getByLabel('Routes du projet')).toBeDisabled();
  await expect(dialog.getByRole('region', { name: 'Routes' })).toContainText(
    'La stratégie de ce poste règle token : refusé, c’est un secret : il reste dans le trousseau de chaque poste. Ce réglage ne s’applique pas.',
  );
  // Refused by the core too, as a window or a client at a distance would ask.
  await expect(
    invoke('plugin_project_set', { id: ID, project: (await projectId(app.data, 'client')) ?? '', key: 'routes', value: '1.1.1.1/32' }),
  ).rejects.toThrow(/stratégie de ce poste/);

  fs.writeFileSync(file, JSON.stringify({ 'telemetry.enabled': false }));
  await expect(routes.getByTestId('plugin-team-lock')).toHaveCount(0, { timeout: 15_000 });
  await expect(routes.getByLabel('Routes du projet')).toBeEnabled();
});

/** The id of the project named `name`, as the app saved it. */
async function projectId(data: string, name: string): Promise<string | undefined> {
  let id: string | undefined;
  await expect
    .poll(() => {
      try {
        const state = JSON.parse(fs.readFileSync(path.join(data, 'state.json'), 'utf8')) as { projects: { id: string; name: string }[] };
        id = state.projects.find((p) => p.name === name)?.id;
      } catch {
        id = undefined;
      }
      return id;
    })
    .toBeTruthy();
  return id;
}
