// SEC-02 (ESC-590): a harness of a third party's plugin is never launched in a private project. The plugin of
// tests/fixtures/plugins/fake-acp/ (`dev.example.fake-acp`, a third party's) describes « Faux ACP »; its agent at rest
// stops as soon as its project is made private, said in its conversation; the settings say the limit; the menu no longer
// offers it there, and a new agent on it is refused with why. SEC-02b (ESC-623): while a private project exists, the same
// in a project that is not private, and given back once none is left.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, send, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const FIXTURE = path.join(ROOT, 'tests', 'fixtures', 'plugins', 'fake-acp');
const BIN = path.join(ROOT, 'tests', 'fixtures', 'acp-bin');
const ENGINE = 'plugin:dev.example.fake-acp:fake-acp';
const LIMIT = 'Un harness d’un plugin tiers ne peut pas garantir la clôture d’un projet privé';
// The page’s end of it, and the core’s and the card’s why (SEC-02b, ESC-623): while a private project exists, in no project.
const PAGE_TAIL = ' : tant qu’il en existe un sur ce poste, il n’est lancé dans aucun projet, et un agent qui en a un s’arrête.';
const WHY = 'Un projet privé existe sur ce poste : un harness d’un plugin tiers ne peut pas garantir sa clôture';

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

test.describe('a third party’s harness in a private project', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: {
      run: () => ({
        accounts: [{ id: `principal-${ENGINE}`, name: 'Principal', engine: ENGINE, configDir: '', claudePath: '', active: true }],
      }),
    },
  });

  test('its agent stops once the project is private, said; it is no longer offered there, and a new one is refused', async ({ app }) => {
    const { page } = app;
    const read = await invoke<{ token: string }>(page, 'plugin_inspect_folder', { path: FIXTURE });
    expect('ok' in read, JSON.stringify(read)).toBe(true);
    expect('ok' in (await invoke(page, 'plugin_install', { token: (read as { ok: { token: string } }).ok.token }))).toBe(true);

    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await page.getByRole('menuitemradio', { name: /Faux ACP/ }).click();
    // A harness of a third party: its whole command agreed to at its first launch (SEC-01, ESC-589).
    const consent = page.getByRole('dialog', { name: 'Lancer Faux ACP ?' });
    await consent.getByRole('button', { name: 'Autoriser et lancer' }).click();
    await expect(consent).toHaveCount(0);
    await send(page, 'bonjour');
    // Its plugin's module and its program start first: slow on a loaded machine.
    await expect(page.getByText('Faux ACP a reçu : bonjour')).toBeVisible({ timeout: 60_000 });

    // « Projet privé »: the page says the limit.
    const dialog = await openSettingsPage(page, 'Confidentialité');
    await dialog.getByRole('switch', { name: 'Projet privé' }).click();
    await expect(dialog.getByTestId('private-third-party')).toHaveText(`${LIMIT}${PAGE_TAIL}`);
    await closeSettings(dialog);

    // Its agent, at rest, stops at once: its conversation says why, and nothing to turn on.
    const conv = page.locator('main.conv');
    await expect(conv.getByText(new RegExp(`^${WHY}.*L’agent s’arrête\\.$`))).toBeVisible();
    const card = page.getByTestId('harness-third-party');
    await expect(card).toContainText(WHY);
    await expect(card.getByRole('button')).toHaveCount(0);

    // Not offered by the menu « Harness » of the project any more.
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await expect(page.getByRole('menuitemradio', { name: /Faux ACP/ })).toHaveCount(0);
    await page.keyboard.press('Escape');

    // A new agent on it is refused by the core, with why.
    const projectId = JSON.parse(fs.readFileSync(path.join(app.data, 'state.json'), 'utf8')).projects[0].id as string;
    const made = await invoke(page, 'create_agent', { projectId, model: null, engine: ENGINE, isolated: null, base: null });
    expect('err' in made).toBe(true);
    expect(String((made as { err: unknown }).err)).toContain(WHY);
  });

  // SEC-02b (ESC-623): another project made private stops the agent of a project that is not private; the menu of that
  // project leaves the harness out and the core refuses it there; the other project public again, it is offered again.
  test('another project made private stops it in a project that is not, and no private project left gives it back', async ({ app }) => {
    const { page } = app;
    const read = await invoke<{ token: string }>(page, 'plugin_inspect_folder', { path: FIXTURE });
    expect('ok' in read, JSON.stringify(read)).toBe(true);
    expect('ok' in (await invoke(page, 'plugin_install', { token: (read as { ok: { token: string } }).ok.token }))).toBe(true);

    await addProject(page, app.repo, { firstAgent: false });
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await page.getByRole('menuitemradio', { name: /Faux ACP/ }).click();
    const consent = page.getByRole('dialog', { name: 'Lancer Faux ACP ?' });
    await consent.getByRole('button', { name: 'Autoriser et lancer' }).click();
    await expect(consent).toHaveCount(0);
    await send(page, 'bonjour');
    await expect(page.getByText('Faux ACP a reçu : bonjour')).toBeVisible({ timeout: 60_000 });

    // A second project in a repository of its own (the first one's holds what its agent wrote), made private.
    const client = path.join(path.dirname(app.repo), 'client');
    fs.mkdirSync(client, { recursive: true });
    for (const args of [
      ['init', '-q', '-b', 'main'],
      ['config', 'user.email', 'e2e@test'],
      ['config', 'user.name', 'e2e'],
      ['commit', '-q', '--allow-empty', '-m', 'init'],
    ])
      execFileSync('git', args, { cwd: client, stdio: 'pipe' });
    await addProject(page, client, { firstAgent: false });
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    const dialog = page.getByRole('dialog', { name: 'Réglages' });
    await openSettingsPage(dialog, 'Confidentialité', { project: 'client' });
    await dialog.getByRole('switch', { name: 'Projet privé' }).click();
    await expect(dialog.getByTestId('private-third-party')).toHaveText(`${LIMIT}${PAGE_TAIL}`);
    await closeSettings(dialog);

    // Back to « demo-api », not private: its agent stopped at once, said; nothing to turn on.
    await page
      .locator('nav.rail')
      .getByRole('tab', { name: /demo-api/ })
      .click();
    const conv = page.locator('main.conv');
    await expect(conv.getByText(new RegExp(`^${WHY}.*L’agent s’arrête\\.$`))).toBeVisible();
    const card = page.getByTestId('harness-third-party');
    await expect(card).toContainText(WHY);
    await expect(card.getByRole('button')).toHaveCount(0);
    // Not offered by its menu « Harness », and refused by the core there.
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await expect(page.getByRole('menuitemradio', { name: /Faux ACP/ })).toHaveCount(0);
    // Closed by its backdrop: the menu « Harness » keeps no focus for Escape once its items are read.
    await page.locator('div.backdrop').click();
    await expect(page.locator('div.backdrop')).toHaveCount(0);
    const projects = JSON.parse(fs.readFileSync(path.join(app.data, 'state.json'), 'utf8')).projects as { id: string; name: string }[];
    const projectId = projects.find((p) => p.name === 'demo-api')!.id;
    const made = await invoke(page, 'create_agent', { projectId, model: null, engine: ENGINE, isolated: null, base: null });
    expect('err' in made).toBe(true);
    expect(String((made as { err: unknown }).err)).toContain(WHY);

    // « client » public again: the harness is offered in « demo-api » again; nothing started by itself.
    await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
    await openSettingsPage(dialog, 'Confidentialité', { project: 'client' });
    await dialog.getByRole('switch', { name: 'Projet privé' }).click();
    await closeSettings(dialog);
    await page
      .locator('nav.rail')
      .getByRole('tab', { name: /demo-api/ })
      .click();
    await expect(page.getByTestId('harness-third-party')).toHaveCount(0);
    await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
    await expect(page.getByRole('menuitemradio', { name: /Faux ACP/ })).toHaveCount(1);
    await page.keyboard.press('Escape');
  });
});
