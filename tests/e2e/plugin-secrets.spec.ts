import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, test } from './fixture';

// A plugin's secret on a machine without a keychain (issue #37, ESC-499), the app told so by its sandbox
// (`ESCOUADE_TEST_KEYCHAIN=absent`, as WSL without Secret Service): « Garder » says why beside its field, the file readable by
// the user alone is offered and kept once chosen, the plugin reads its secret from it (`secrets/get`, its journal masking it),
// Réglages › Plugins shows the file and takes it back. Captures in both themes with `ESCOUADE_E2E_SHOTS`.

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const ID = 'dev.test.fake';
const SECRET = 'hunter2-openvpn-e2e';
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

test.use({ appEnv: { ESCOUADE_TEST_KEYCHAIN: 'absent' } });

/** The fake plugin, told `--leak`: on each fact it asks for its secret `token` and writes on its journal what came. */
function fakePlugin(dir: string) {
  fs.mkdirSync(dir, { recursive: true });
  fs.copyFileSync(path.join(ROOT, 'tests', 'fixtures', 'plugins', 'fake', 'plugin.mjs'), path.join(dir, 'plugin.mjs'));
  fs.writeFileSync(path.join(dir, 'args.json'), JSON.stringify(['--leak']));
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

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const size = page.viewportSize();
  for (const t of ['light', 'dark'] as const) {
    await theme(page, t);
    await page.mouse.move(2, 400);
    await page.screenshot({ path: path.join(SHOTS, `secrets-${name}-${size?.width ?? 0}-${t}.png`) });
  }
  await theme(page, 'dark');
}

/** Every file under `dir`. */
const walk = (dir: string): string[] =>
  fs.existsSync(dir)
    ? fs
        .readdirSync(dir, { withFileTypes: true })
        .flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]))
    : [];

test('without a keychain, « Garder » says why, the file chosen keeps the secret the plugin reads, Réglages takes it back', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  const invoke = invokeOn(page);
  await addProject(page, app.repo);
  const src = path.join(app.root, 'fake-plugin');
  fakePlugin(src);
  const { token } = await invoke<{ token: string }>('plugin_inspect_folder', { path: src });
  await invoke('plugin_install', { token });

  // Its page, on its settings.
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .getByRole('table', { name: 'Ajoutés' })
    .getByRole('button', { name: /Faux plugin/ })
    .click();
  await page.getByRole('tab', { name: /^Réglages/ }).click();
  const panel = page.getByRole('tabpanel', { name: 'Réglages' });
  const row = panel.getByRole('group', { name: 'Jeton' });
  await expect(row).toContainText('aucun');

  // « Garder »: nothing kept, said beside the field with its cause, the file offered with what it means.
  await row.getByLabel('Jeton').fill(SECRET);
  await row.getByRole('button', { name: 'Garder', exact: true }).click();
  await expect(row.getByRole('alert')).toHaveText('Pas de trousseau système sur ce poste : le secret n’a pas été gardé.');
  await expect(row).toContainText('aucun');
  const file = path.join(app.data, 'plugin-secrets.json');
  await expect(row).toContainText(`en clair sur le disque, dans ${file}`);
  expect(fs.existsSync(file)).toBe(false);
  await shoot(page, 'refused');

  // Chosen: kept in the file, said so.
  await row.getByRole('button', { name: 'Garder dans un fichier lisible de toi seul' }).click();
  await expect(row).toContainText('gardé dans un fichier lisible de toi seul');
  await expect(row.getByRole('alert')).toHaveCount(0);
  expect(fs.readFileSync(file, 'utf8')).toContain(SECRET);
  await shoot(page, 'in-file');

  // The plugin reads it by `secrets/get` on a fact; its journal masks it, the app's log never has it.
  const projectId = JSON.parse(fs.readFileSync(path.join(app.data, 'state.json'), 'utf8')).projects[0].id as string;
  await invoke('ticket_create', { projectId, draft: { title: 'Monter le tunnel', description: '', criteria: [], maxLoops: 5, after: [] } });
  const journal = () =>
    invoke<{ journal: { text: string }[] }>('plugin_run', { id: ID }).then((r) => r.journal.map((e) => e.text).join('\n'));
  await expect.poll(journal, { timeout: 60_000 }).toContain('fake-plugin: the secret is ••••');
  expect(await journal()).not.toContain(SECRET);
  expect(await journal()).not.toContain('the secret is null');
  for (const f of walk(app.data).filter((f) => /\.(log|json|gz)$/.test(f) && path.resolve(f) !== path.resolve(file))) {
    expect(fs.readFileSync(f).includes(SECRET), f).toBe(false);
  }

  // Réglages › Plugins: the file, where it is and what it holds; taken back, it goes.
  const settings = await openSettingsPage(page, 'Plugins');
  const kept = settings.getByRole('region', { name: 'Secrets gardés dans un fichier' });
  await expect(kept).toContainText('Ce poste n’a pas de trousseau système');
  await expect(kept).toContainText(file);
  await expect(kept).toContainText('token, de Faux plugin');
  await kept.scrollIntoViewIfNeeded();
  await shoot(page, 'settings');
  await kept.getByRole('button', { name: 'Retirer le fichier' }).click();
  await kept.getByRole('button', { name: 'Retirer et effacer' }).click();
  await expect(kept).toHaveCount(0);
  expect(fs.existsSync(file)).toBe(false);
});
