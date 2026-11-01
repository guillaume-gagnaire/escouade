// PLG-6 (ESC-178): a harness described by a plugin without code, served by the generic ACP plugin `dev.escouade.acp`.
// The plugin of tests/fixtures/plugins/fake-acp/ is installed from its folder; its agent (tests/fixtures/fake-acp.mjs,
// found on the PATH as `fake-acp`) converses, asks its permissions, writes a file through Escouade with its diff, and
// ends its turns. Turned off, its agent stays, « harness absent ».

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const FIXTURE = path.join(ROOT, 'tests', 'fixtures', 'plugins', 'fake-acp');
const BIN = path.join(ROOT, 'tests', 'fixtures', 'acp-bin');
const ENGINE = 'plugin:dev.example.fake-acp:fake-acp';

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

test.describe('an agent on a harness an ACP plugin describes', () => {
  test.use({
    appEnv: {
      PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}`,
    },
    preStart: {
      run: () => ({
        accounts: [{ id: `principal-${ENGINE}`, name: 'Principal', engine: ENGINE, configDir: '', claudePath: '', active: true }],
      }),
    },
  });

  test('converses, asks its permissions, writes with its diff and ends its turns; turned off, its agent is « harness absent »', async ({
    app,
  }) => {
    const { page } = app;
    // Installed from its folder: one gesture, the program its agents run shown first.
    const read = await invoke<{ token: string; asks: string[] }>(page, 'plugin_inspect_folder', { path: FIXTURE });
    expect('ok' in read, JSON.stringify(read)).toBe(true);
    const preview = (read as { ok: { token: string; asks: string[] } }).ok;
    expect(preview.asks).toContain('harness:fake-acp acp');
    expect('ok' in (await invoke(page, 'plugin_install', { token: preview.token }))).toBe(true);

    await addProject(page, app.repo);
    // The menu « Harness » reads the registry again: the harness the plugin describes is in it.
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await page.getByRole('menuitemradio', { name: /Faux ACP/ }).click();
    await expect(page.locator('main.conv header.head')).toContainText('Faux ACP');

    // A message, and the end of its turn.
    await send(page, 'bonjour');
    await expect(page.getByText('Faux ACP a reçu : bonjour')).toBeVisible();
    // A command asks its permission; allowed, it runs.
    await send(page, 'commande');
    const card = page.getByTestId('permission-pending');
    await expect(card).toContainText('npm test');
    await card.getByRole('button', { name: 'Autoriser', exact: true }).click();
    await expect(page.getByText('Les tests passent.')).toBeVisible();

    // A write asks with its diff, then goes through Escouade (fs/write_text_file): the file is written, its diff shown.
    await send(page, 'fichier');
    await expect(card).toContainText('notes.txt');
    await expect(card.locator('.patch, .patch-view, pre').first()).toContainText('bonjour');
    await card.getByRole('button', { name: 'Autoriser', exact: true }).click();
    await expect(page.getByText('notes.txt écrit.')).toBeVisible();
    const repo = fs.existsSync(path.join(app.repo, 'notes.txt'));
    expect(repo).toBe(true);
    expect(fs.readFileSync(path.join(app.repo, 'notes.txt'), 'utf8')).toBe('bonjour\n');
    await expect(page.locator('main.conv').getByText('bonjour', { exact: true }).first()).toBeVisible();

    // The end of the turn: the agent is done, nothing waits.
    await expect(card).toHaveCount(0);
    await expect(page.locator('.card', { hasText: 'rapide' }).first()).toContainText('Terminé');

    // Its plugin turned off: its agent stays, « harness absent », its conversation kept.
    expect('ok' in (await invoke(page, 'plugin_set_enabled', { id: 'dev.example.fake-acp', on: false }))).toBe(true);
    const declared = await invoke<{ id: string }[]>(page, 'engines_declared');
    expect((declared as { ok: { id: string }[] }).ok.map((e) => e.id)).not.toContain(ENGINE);
    await send(page, 'encore là ?');
    await expect(page.getByText(/Harness absent/).first()).toBeVisible();
    await expect(page.getByText('Faux ACP a reçu : bonjour')).toBeVisible();
  });
});
