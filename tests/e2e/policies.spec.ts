// The settings imposed by the policies of this device (TS-11): the app is started with a source of policies of the
// tests, a JSON file named by ESCOUADE_TEST_POLICIES, which only a build of development reads (never a release: SEC-13
// of the audit of 2.2.0, `locks::the_test_source_of_policies_exists_in_a_build_of_development_alone`). What they impose
// is greyed with its padlock and who imposes it, the choices a bound denies greyed with the reason, and the settings
// file holds what they lock.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { addProject, expect, test, openSettingsPage } from './fixture';

test.use({
  appEnv: async ({}, use) => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ccm-policies-'));
    const file = path.join(dir, 'policies.json');
    fs.writeFileSync(
      file,
      JSON.stringify({
        'telemetry.enabled': false,
        'permissions.allowBypass': false,
        'updates.channel': 'stable',
        'permissions.modes': ['plan', 'default'],
        // No account reserved to a project (ESC-564).
        'accounts.reservation': 'denied',
        'some.future': 1,
      }),
    );
    await use({ ESCOUADE_TEST_POLICIES: file });
    fs.rmSync(dir, { recursive: true, force: true });
  },
  // Ticked before the start: the policy brings it back within them.
  // The machine's account reserved to a project before the start: the policy empties its list.
  preStart: {
    run: () => ({
      betaUpdates: true,
      agentMode: 'auto',
      accounts: [
        { id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true, projects: ['ailleurs'] },
      ],
    }),
  },
});

test('a setting imposed by this device’s policy is greyed, with its padlock and who imposes it', async ({ app }) => {
  const { page, data } = app;
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Général');

  // « Recevoir les versions bêta », ticked before: off, greyed, said imposed.
  const beta = dialog.locator('.sr', { has: page.getByRole('switch', { name: 'Recevoir les versions bêta' }) });
  await expect(beta).toHaveClass(/locked/);
  await expect(beta.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
  await expect(beta).toContainText('Imposé par la stratégie de ce poste');
  await expect(beta.locator('.ctl')).toHaveAttribute('aria-disabled', 'true');
  // Inert: a click does not reach it.
  await beta.getByRole('switch').click({ force: true });
  await expect(beta.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
  const stats = dialog.locator('.sr', { has: page.getByRole('switch', { name: 'Statistiques d’usage' }) });
  await expect(stats).toHaveClass(/locked/);
  await expect(stats).toContainText('Imposé par la stratégie de ce poste');
  // A setting not imposed stays as it was.
  const auto = dialog.locator('.sr', { has: page.getByRole('switch', { name: 'Installer les mises à jour automatiquement' }) });
  await expect(auto).not.toHaveClass(/locked/);
  // What the policies said and Escouade could not apply: listed.
  await expect(dialog.getByRole('heading', { name: 'Stratégies de ce poste' })).toBeVisible();
  await expect(dialog.getByText(/some\.future/)).toBeVisible();

  // The modes a bound denies: greyed, with the reason; the mode of no question imposed off.
  await openSettingsPage(dialog, 'Agents');
  const modes = dialog.getByRole('group', { name: 'Mode par défaut des agents' });
  await expect(modes.getByRole('button', { name: /^Plan/ })).toBeEnabled();
  await expect(modes.getByRole('button', { name: /^Demander/ })).toBeEnabled();
  await expect(modes.getByRole('button', { name: /^Auto/ })).toBeDisabled();
  await expect(modes.getByRole('button', { name: /^Édits auto/ })).toBeDisabled();
  // The mode of new agents, auto before the start: the nearest allowed that never allows more.
  await expect(modes.getByRole('button', { name: /^Demander/ })).toHaveAttribute('aria-pressed', 'true');
  const bypass = dialog.locator('.sr', { has: page.getByRole('switch', { name: 'Autoriser le mode Bypass' }) });
  await expect(bypass).toHaveClass(/locked/);
  await expect(bypass).toContainText('Imposé par la stratégie de ce poste');

  // The settings file holds what they lock.
  const saved = JSON.parse(fs.readFileSync(path.join(data, 'settings.json'), 'utf8'));
  expect([saved.betaUpdates, saved.telemetry, saved.allowBypass, saved.agentMode]).toEqual([false, false, false, 'default']);
});

// ESC-564: `accounts.reservation` imposed to `denied`. The reservation made before the start is gone, « Réservé aux projets »
// is greyed with its padlock, and a private project's « Réserver … à ce projet » and « Ajouter un compte réservé… » too.
test('no account may be reserved to a project: the gestures are greyed with their padlock', async ({ app }) => {
  const { page, data } = app;
  const saved = () => JSON.parse(fs.readFileSync(path.join(data, 'settings.json'), 'utf8'));
  expect(saved().accounts.find((a: { id: string }) => a.id === 'principal').projects ?? []).toEqual([]);
  await addProject(page, app.repo, { name: 'client', firstAgent: false });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Comptes et harness');
  const reserved = dialog.getByRole('group', { name: 'Réservé aux projets' }).first();
  await expect(reserved.locator('.chips')).toHaveAttribute('aria-disabled', 'true');
  await expect(reserved.getByTestId('reservation-lock')).toHaveText(
    'Imposé par la stratégie de ce poste · Aucun compte n’est réservé à un projet : un projet privé part sur les comptes partagés, sa clôture protège ses sessions.',
  );
  // Inert: a click does not tick the project.
  await reserved.getByRole('checkbox').first().click({ force: true });
  await expect(reserved.getByRole('checkbox').first()).not.toBeChecked();

  await openSettingsPage(dialog, 'Confidentialité', { project: 'client' });
  await dialog.getByRole('switch', { name: 'Projet privé' }).click();
  await expect(dialog.getByTestId('reservation-lock')).toContainText('Imposé par la stratégie de ce poste');
  await expect(dialog.getByRole('button', { name: 'Réserver Principal à ce projet' })).toBeDisabled();
  await expect(dialog.getByRole('button', { name: 'Ajouter un compte réservé…' })).toBeDisabled();
  // Its agents go on the shared account, as in 2.2.5: said, the fence keeping its sessions.
  await expect(dialog.getByTestId('private-shared')).toContainText('Ses sessions restent à lui');
  expect(saved().accounts.flatMap((a: { projects?: string[] }) => a.projects ?? [])).toEqual([]);
});
