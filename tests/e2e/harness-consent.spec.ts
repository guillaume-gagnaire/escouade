// SEC-01 (ESC-589): what a plugin of a third party may do without a review. « N'installer que des versions vérifiées »
// holds a folder to the store (the fake ACP plugin of tests/fixtures/plugins/fake-acp/ is no version of the store: it is
// refused, the gesture that allows it said), and development mode stays apart. The harness that plugin describes
// (tests/fixtures/fake-acp.mjs, found on the PATH as `fake-acp`) asks at its first launch, its whole command said;
// agreed, its agent starts, and a second agent asks nothing more.

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

const account = () => ({
  accounts: [{ id: `principal-${ENGINE}`, name: 'Principal', engine: ENGINE, configDir: '', claudePath: '', active: true }],
});

/** A new agent of the fake ACP harness, by the menu « Autres façons de créer un agent ». */
async function newAgent(page: Page) {
  await page.getByRole('button', { name: 'Autres façons de créer un agent' }).click();
  await page.getByRole('menuitemradio', { name: /Faux ACP/ }).click();
}

test.describe('« N’installer que des versions vérifiées » on a folder', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: { run: () => ({ ...account(), pluginsVerifiedOnly: true }) },
  });

  test('refuses a folder that is no verified version of the store, and leaves development apart', async ({ app }) => {
    const { page } = app;
    const read = await invoke<{ token: string }>(page, 'plugin_inspect_folder', { path: FIXTURE });
    expect('err' in read, JSON.stringify(read)).toBe(true);
    const why = JSON.stringify((read as { err: unknown }).err);
    expect(why).toContain('Faux ACP v1.0.0 n');
    expect(why).toContain('est pas install');
    expect(why).toContain('coupe-le pour installer ce plugin quand même');
    // In development: another gesture, which the setting does not hold.
    const dev = await invoke<{ token: string; origin: { kind: string } }>(page, 'plugin_inspect_dev', { path: FIXTURE });
    expect('ok' in dev, JSON.stringify(dev)).toBe(true);
    expect((dev as { ok: { origin: { kind: string } } }).ok.origin.kind).toBe('dev');
    expect('ok' in (await invoke(page, 'plugin_discard', { token: (dev as { ok: { token: string } }).ok.token }))).toBe(true);
  });
});

test.describe('a harness of a third party', () => {
  test.use({
    appEnv: { PATH: `${BIN}${path.delimiter}${process.env.PATH ?? ''}` },
    preStart: { run: () => account() },
  });

  test('asks at its first launch with its whole command, then nothing more at the second', async ({ app }) => {
    const { page } = app;
    // Installed from its folder, the setting off: the harness is in the registry, its command not agreed to yet.
    const read = await invoke<{ token: string }>(page, 'plugin_inspect_folder', { path: FIXTURE });
    expect('ok' in read, JSON.stringify(read)).toBe(true);
    expect('ok' in (await invoke(page, 'plugin_install', { token: (read as { ok: { token: string } }).ok.token }))).toBe(true);
    const held = await invoke<Record<string, string>>(page, 'harness_launches');
    expect((held as { ok: Record<string, string> }).ok[ENGINE]).toBe('asks');
    await addProject(page, app.repo);

    // Its first agent: the window says the whole command, the program found on the PATH among it.
    await newAgent(page);
    const ask = page.getByRole('dialog', { name: 'Lancer Faux ACP ?' });
    await expect(ask).toBeVisible();
    await expect(ask).toContainText('vient du plugin tiers Faux ACP (dev.example.fake-acp)');
    await expect(ask.getByTestId('launch-line')).toContainText(/acp-bin.fake-acp(\.cmd)? acp$/);
    await expect(ask.getByTestId('launch-program')).toContainText('acp-bin');
    await expect(ask).toContainText('FAKE_ACP_HOME');
    await ask.getByRole('button', { name: 'Autoriser et lancer' }).click();
    await expect(ask).toHaveCount(0);
    await expect(page.getByTestId('harness-launch-held')).toHaveCount(0);
    await send(page, 'bonjour');
    await expect(page.locator('main.conv').getByText('Faux ACP a reçu : bonjour')).toBeVisible();

    // The second: nothing asked, it starts.
    await newAgent(page);
    await expect(page.locator('main.conv header.head')).toContainText('Faux ACP');
    await send(page, 'encore');
    await expect(page.locator('main.conv').getByText('Faux ACP a reçu : encore')).toBeVisible();
    await expect(page.getByRole('dialog', { name: 'Lancer Faux ACP ?' })).toHaveCount(0);
    const after = await invoke<Record<string, string>>(page, 'harness_launches');
    expect((after as { ok: Record<string, string> }).ok[ENGINE]).toBeUndefined();
  });
});
