import fs from 'node:fs';
import path from 'node:path';
import { addProject, expect, FAKE, send, test, openSettingsPage, closeSettings } from './fixture';

// Choosing the harness (ESC-29): Réglages › Harness lists each one with its executable and its version; its accounts sign
// in from Réglages › Comptes (1.8.2). With none found, the first start says « Aucun harness trouvé » with the command that
// installs each.

test('Réglages › Harness shows Claude Code’s executable and version, and sends to Comptes, where its account signs in', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Comptes et harness');
  const claude = settings.getByRole('region', { name: 'Claude Code', exact: true });
  await expect(claude.getByRole('textbox', { name: "Chemin de l'exécutable" })).toBeVisible();
  await expect(claude.locator('[data-state]')).toHaveText('2.1.301');
  // The test's Claude Code folder has no sign-in: the version's line says where to sign in, the tab itself does not.
  await expect(claude).toContainText('Aucun compte connecté : ajoute ou connecte un compte de ce harness dans l’onglet Comptes.');
  await expect(claude.getByRole('button', { name: 'Se connecter…' })).toHaveCount(0);
  await openSettingsPage(settings, 'Comptes et harness');
  await settings.getByRole('listitem', { name: 'Principal' }).getByRole('button', { name: 'Se connecter…' }).click();
  const signIn = page.getByRole('dialog', { name: 'Se connecter au compte « Principal »' });
  await expect(signIn).toBeVisible();
  // Its × (Escape may go to the sign-in's terminal once it has the focus).
  await signIn.getByRole('button', { name: 'Fermer' }).click();
  await expect(signIn).toHaveCount(0);
});

test('Escouade runs claude update at its start, apart from its agents, whose own update is off; Réglages › Harness says what it said (ESC-133)', async ({
  app,
}) => {
  const { page } = app;
  await addProject(page, app.repo);
  await send(page, 'Bonjour');
  await expect(page.locator('.card .name').first()).toHaveText('bonjour-fake');
  // Every claude launched for it (the agent, the naming of the agent, the reading of the models): its own update off.
  expect(app.launches().length).toBeGreaterThan(0);
  expect(app.launches().map((l) => l.autoUpdater)).toEqual(app.launches().map(() => '1'));
  // `claude update`, a few seconds after the start, with nothing that turns it off.
  const updates = path.join(app.root, 'fake-claude.jsonl.update.jsonl');
  const said = () =>
    fs.existsSync(updates)
      ? fs
          .readFileSync(updates, 'utf8')
          .trim()
          .split('\n')
          .map((l) => JSON.parse(l))
      : [];
  await expect.poll(said, { timeout: 30_000 }).toContainEqual({ done: true });
  expect(said()[0]).toMatchObject({ started: true, autoUpdater: null });
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Comptes et harness');
  const claude = settings.getByRole('region', { name: 'Claude Code', exact: true });
  await expect(claude.getByRole('switch', { name: 'Mettre à jour Claude Code automatiquement' })).toHaveAttribute('aria-checked', 'true');
  await expect(claude.getByRole('status')).toHaveText('À jour (2.1.301).');
});

test.describe('with Claude Code’s own update turned off by the user', () => {
  test.use({
    preStart: {
      run: (root: string) => {
        // As Claude Code's own migration writes it, in Principal's settings.json.
        fs.mkdirSync(path.join(root, 'claude'), { recursive: true });
        fs.writeFileSync(path.join(root, 'claude', 'settings.json'), JSON.stringify({ env: { DISABLE_AUTOUPDATER: '1' } }));
      },
    },
  });

  test('Escouade does not update it either, and says so (ESC-133)', async ({ app }) => {
    const { page } = app;
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    const claude = settings.getByRole('region', { name: 'Claude Code', exact: true });
    await expect(claude.getByRole('status')).toHaveText(
      'Coupée dans Claude Code lui-même (DISABLE_AUTOUPDATER) : Escouade ne le met pas à jour non plus.',
    );
    // Past the start's few seconds: never run.
    await page.waitForTimeout(8000);
    expect(fs.existsSync(path.join(app.root, 'fake-claude.jsonl.update.jsonl'))).toBe(false);
  });
});

test('« Modèle des textes rédigés » offers Claude Haiku alone with Claude Code alone, and Haiku names a new agent', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Agents');
  const writer = settings.getByRole('group', { name: 'Modèle des textes rédigés' });
  await expect(writer.getByRole('button')).toHaveText(['Claude Haiku']);
  await expect(writer.getByRole('button', { name: 'Claude Haiku' })).toHaveAttribute('aria-pressed', 'true');
  await page.keyboard.press('Escape');
  await expect(settings).toHaveCount(0);
  // The question goes to Claude Code, as before (the fake CLI answers the naming call).
  await send(page, 'Bonjour');
  await expect(page.locator('.card .name').first()).toHaveText('bonjour-fake');
});

test.describe('with no harness found', () => {
  test.use({ preStart: { run: (root: string) => ({ claudePath: path.join(root, 'absent', 'claude.cmd') }) } });

  test('the first start says « Aucun harness trouvé » with the command that installs each, and goes once one is found', async ({ app }) => {
    const { page } = app;
    const none = page.getByRole('region', { name: 'Aucun harness trouvé' });
    await expect(none).toBeVisible();
    await expect(none.getByRole('listitem').filter({ hasText: 'Claude Code' })).toContainText('irm https://claude.ai/install.ps1 | iex');
    // Réglages › Harness: not found, with its installer.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const settings = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(settings, 'Comptes et harness');
    const claude = settings.getByRole('region', { name: 'Claude Code', exact: true });
    await expect(claude.locator('[data-engine-state="absent"]')).toHaveText('Introuvable');
    await expect(claude.getByText('irm https://claude.ai/install.ps1 | iex', { exact: true })).toBeVisible();
    // Said with what puts it back (ESC-133), by the installation's row and by its update's.
    const gone =
      'Claude Code est introuvable : réinstalle-le (`irm https://claude.ai/install.ps1 | iex`, ou `npm install -g @anthropic-ai/claude-code`), ou indique son chemin dans Réglages › Harness.';
    await expect(claude.getByText(gone, { exact: true })).toHaveCount(2);
    // No harness to write the texts: « Modèle des textes rédigés » is not offered.
    await openSettingsPage(settings, 'Agents');
    await expect(settings.getByRole('heading', { name: 'Nouveaux agents' })).toBeVisible();
    await expect(settings.getByRole('group', { name: 'Modèle des textes rédigés' })).toHaveCount(0);
    await openSettingsPage(settings, 'Comptes et harness');
    // Its path given and saved: it is found, Escouade works.
    await claude.getByRole('textbox', { name: "Chemin de l'exécutable" }).fill(FAKE);
    await closeSettings(settings);
    await expect(none).toHaveCount(0);
  });
});
