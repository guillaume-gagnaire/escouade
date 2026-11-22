// ESC-489: the mode judged is the one a harness really runs in. A harness described over ACP that declares one mode,
// Bypass (tests/fixtures/plugins/bypass-only/, the agent tests/fixtures/fake-unharnessed.mjs found on the PATH as
// `unharnessed`, both taken from ESC-484's Unharnessed), installed from a folder under an id of anyone's. Its new agent
// shows Bypass, the mode it runs in, not « Demander ». Without « Autoriser le mode Bypass », it does not start, and says
// why and what to do; with it, it starts.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, send, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const MANIFEST = path.join(ROOT, 'tests', 'fixtures', 'plugins', 'bypass-only', 'escouade-plugin.json');
const BIN = path.join(ROOT, 'tests', 'fixtures', 'unharnessed-bin');
const ENGINE = 'plugin:dev.example.unharnessed:unharnessed';

type Result<T> = { ok: T } | { err: unknown };

async function invoke<T>(page: Page, cmd: string, args: Record<string, unknown> = {}): Promise<Result<T>> {
  await page.waitForFunction(() => '__TAURI_INTERNALS__' in window, null, { timeout: 30_000 });
  return page.evaluate(
    async ([cmd, args]) => {
      const t = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      try {
        return { ok: await t.invoke(cmd as string, args) };
      } catch (err) {
        return { err };
      }
    },
    [cmd, args] as const,
  ) as never;
}

/** The plugin installed from a copy of its folder (its preview says its agents run in Bypass alone), then a project and
 *  a new agent of its harness. */
async function installAndOpen(page: Page, repo: string) {
  const dir = path.join(path.dirname(repo), 'bypass-only-plugin');
  fs.mkdirSync(dir, { recursive: true });
  fs.copyFileSync(MANIFEST, path.join(dir, 'escouade-plugin.json'));
  const read = await invoke<{ token: string; asks: string[] }>(page, 'plugin_inspect_folder', { path: dir });
  expect('ok' in read, JSON.stringify(read)).toBe(true);
  const preview = (read as { ok: { token: string; asks: string[] } }).ok;
  expect(preview.asks).toContain('harnessBypass:Unharnessed');
  expect('ok' in (await invoke(page, 'plugin_install', { token: preview.token }))).toBe(true);
  await addProject(page, repo);
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitemradio', { name: /Unharnessed/ }).click();
  // A harness of a third party: its whole command agreed to at its first launch (SEC-01, ESC-589).
  await page.getByRole('dialog', { name: 'Lancer Unharnessed ?' }).getByRole('button', { name: 'Autoriser et lancer' }).click();
  await expect(page.getByRole('dialog', { name: 'Lancer Unharnessed ?' })).toHaveCount(0);
  await expect(page.locator('main.conv header.head')).toContainText('Unharnessed');
  // Its real mode, not « Demander ».
  await expect(page.getByRole('button', { name: /^Mode/ })).toContainText('Bypass');
}

/** An account of the harness, its folder of its own. */
function account(root: string) {
  return {
    accounts: [
      {
        id: `principal-${ENGINE}`,
        name: 'Principal',
        engine: ENGINE,
        configDir: path.join(root, 'unharnessed-home'),
        claudePath: '',
        active: true,
      },
    ],
  };
}

test.describe('a new agent of a harness whose one mode is Bypass, Bypass not allowed', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: { run: (root) => ({ ...account(root), allowBypass: false, agentMode: 'acceptEdits' }) },
  });

  test('shows Bypass, does not start, and says why and what to do', async ({ app }) => {
    const { page } = app;
    await installAndOpen(page, app.repo);
    await send(page, 'bonjour');
    await expect(
      page
        .getByText(
          /Unharnessed ne tourne qu’en Bypass .* active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre harness/,
        )
        .first(),
    ).toBeVisible();
    await expect(page.getByText('Unharnessed a reçu : bonjour')).toHaveCount(0);
    // The menu offers no other mode: each says why.
    await page.getByRole('button', { name: /^Mode/ }).click();
    const ask = page.getByRole('menuitemradio', { name: /^Demander/ });
    await expect(ask).toBeDisabled();
    await expect(ask).toContainText('Unharnessed ne tourne qu’en Bypass.');
    await page.keyboard.press('Escape');
  });
});

test.describe('a new agent of a harness whose one mode is Bypass, Bypass allowed', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: { run: (root) => ({ ...account(root), allowBypass: true, agentMode: 'acceptEdits' }) },
  });

  test('starts, in Bypass', async ({ app }) => {
    const { page } = app;
    await installAndOpen(page, app.repo);
    await send(page, 'bonjour');
    await expect(page.getByText('Unharnessed a reçu : bonjour')).toBeVisible();
    await expect(page.getByRole('button', { name: /^Mode/ })).toContainText('Bypass');
  });

  // ESC-521 (ESC-497 on the 2.2): « Autoriser le mode Bypass » turned off while its process runs in Bypass: the process
  // stops, its conversation says why, and its next message is refused.
  test('stops when Bypass is turned off, and says why', async ({ app }) => {
    const { page } = app;
    await installAndOpen(page, app.repo);
    await send(page, 'bonjour');
    await expect(page.getByText('Unharnessed a reçu : bonjour')).toBeVisible();
    const dialog = await openSettingsPage(page, 'Agents');
    const allow = dialog.getByRole('switch', { name: 'Autoriser le mode Bypass' });
    await expect(allow).toHaveAttribute('aria-checked', 'true');
    await allow.click();
    await expect(allow).toHaveAttribute('aria-checked', 'false');
    await closeSettings(dialog);
    await expect(page.getByText(/Unharnessed ne tourne qu’en Bypass .* L’agent s’arrête/).first()).toBeVisible();
    await send(page, 'encore');
    await expect(page.getByText(/Unharnessed ne tourne qu’en Bypass .*ou choisis un autre harness\.$/).first()).toBeVisible();
    await expect(page.getByText('Unharnessed a reçu : encore')).toHaveCount(0);
  });
});
