import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, sandboxKeychain, test } from './fixture';

// The keychain's entries of a sandboxed run (ESC-584): a test that keeps a plugin's secret in the real keychain and then
// fails leaves none once it ended, the end of the fixture taking back what its app wrote, under names of its run only.

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const ID = 'dev.test.fake';

test.describe.configure({ mode: 'serial' });

/** The fake plugin, its secret `token` declared. */
function fakePlugin(dir: string) {
  fs.mkdirSync(dir, { recursive: true });
  fs.copyFileSync(path.join(ROOT, 'tests', 'fixtures', 'plugins', 'fake', 'plugin.mjs'), path.join(dir, 'plugin.mjs'));
  fs.writeFileSync(
    path.join(dir, 'escouade-plugin.json'),
    JSON.stringify({
      id: ID,
      name: 'Faux plugin',
      version: '1.0.0',
      author: { name: 'escouade-tests' },
      license: 'MIT',
      escouade: { api: '1', min: '2.1.0' },
      run: { js: 'plugin.mjs' },
      activation: ['event:ticket.created'],
      permissions: { events: ['ticket.created'], secrets: ['token'] },
      contributes: { secrets: [{ key: 'token', title: { fr: 'Jeton', en: 'Token' } }] },
    }),
  );
}

const invokeOn =
  (page: Page) =>
  <T>(cmd: string, args: Record<string, unknown> = {}) =>
    page.evaluate(
      ([cmd, args]) =>
        (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
          cmd as string,
          args,
        ),
      [cmd, args] as const,
    ) as Promise<T>;

/** The data folder of the test that kept a secret, for the one after it. */
let kept = '';

test('a test that keeps a secret in the keychain, then fails', async ({ app }) => {
  test.fail(true, 'it fails on purpose, after the secret is kept: the end of the fixture must take it back all the same');
  const invoke = invokeOn(app.page);
  const src = path.join(app.root, 'fake-plugin');
  fakePlugin(src);
  const { token } = await invoke<{ token: string }>('plugin_inspect_folder', { path: src });
  await invoke('plugin_install', { token });
  expect(sandboxKeychain('list', app.data)).toBe('');
  await invoke('plugin_set_secret', { id: ID, key: 'token', value: 'esc584-trial-value' });
  const names = sandboxKeychain('list', app.data).split('\n').filter(Boolean);
  // Under a name of its run, ending with the sandbox's service.
  expect(names.length).toBeGreaterThan(0);
  for (const n of names) expect(n).toMatch(/\.run-[0-9a-f]{16}\.dev\.gagnaire\.escouade\.sandbox$/);
  kept = app.data;
  throw new Error('failed on purpose, the secret kept');
});

test('once it ended, nothing it wrote is left in the keychain', () => {
  expect(kept).not.toBe('');
  expect(sandboxKeychain('list', kept)).toBe('');
});
