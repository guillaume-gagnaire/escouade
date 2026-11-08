import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, test } from './fixture';

// The view Plugins (PLG-4, ESC-176): the fake plugin (tests/fixtures/fake-plugin.mjs) installed from a folder through the
// install window — its origin, its fingerprint and what it may do shown before anything is installed —, switched off then
// on again for a project, its process started from its journal, which then tells its start; uninstalled, its copy and
// its journal gone. Réglages › Plugins (ESC-384) lists it, what it adds, on how many projects, and leads to its page.

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const ID = 'dev.test.fake';

/** The fake plugin in a folder of the test's own, as an author would have it; `changes` go over its manifest. */
function fakePlugin(dir: string, changes: Record<string, unknown> = {}) {
  fs.mkdirSync(dir, { recursive: true });
  for (const f of ['fake-plugin.mjs', 'fake-plugin.cmd', 'fake-plugin'])
    fs.copyFileSync(path.join(ROOT, 'tests', 'fixtures', f), path.join(dir, f));
  const manifest = {
    id: ID,
    name: 'Faux plugin',
    version: '1.0.0',
    description: { fr: 'Le plugin des tests d’Escouade.', en: 'The plugin of Escouade’s tests.' },
    author: { name: 'escouade-tests' },
    license: 'MIT',
    escouade: { api: '1', min: '2.1.0' },
    run: { command: 'fake-plugin', windows: 'fake-plugin.cmd' },
    activation: ['event:ticket.column'],
    permissions: { events: ['ticket.column'], network: ['hooks.example.com'], secrets: ['token'] },
    contributes: { secrets: [{ key: 'token', title: { fr: 'Jeton', en: 'Token' } }] },
    ...changes,
  };
  fs.writeFileSync(path.join(dir, 'escouade-plugin.json'), JSON.stringify(manifest, null, 2));
}

/** The folder dialog answered with `dir`, in the page, on the `fetch` of Tauri's IPC: no native window opens. */
async function pickAnswers(page: Page, dir: string) {
  await page.evaluate((answer) => {
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.includes('plugin:dialog|open'))
        return Promise.resolve(
          new Response(JSON.stringify(answer), { headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } }),
        );
      return fetch(input, init);
    };
  }, dir);
}

const projectId = (data: string) => JSON.parse(fs.readFileSync(path.join(data, 'state.json'), 'utf8')).projects[0].id as string;
const installed = (data: string) => {
  const file = path.join(data, 'plugins.json');
  return fs.existsSync(file)
    ? (JSON.parse(fs.readFileSync(file, 'utf8')).plugins as { id: string; origin: { kind: string }; projects?: Record<string, boolean> }[])
        // Those the app gives (INT-4) are in the file too, but are not installed.
        .filter((p) => p.origin.kind !== 'builtin')
    : [];
};

test('the fake plugin is installed from a folder, switched on for a project, its journal read, then uninstalled', async ({ app }) => {
  test.setTimeout(120_000);
  const { page } = app;
  await addProject(page, app.repo);
  const src = path.join(app.root, 'fake-plugin');
  fakePlugin(src);

  // The view, by its icon of the projects' rail.
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  const side = page.getByRole('complementary', { name: 'Plugins' });
  await expect(side.getByRole('button', { name: /Installés/ })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByText('Aucun plugin installé.')).toBeVisible();

  // « Depuis un dossier… »: the window says where it comes from and what it may do; nothing is installed before.
  await pickAnswers(page, src);
  await page.getByRole('button', { name: 'Depuis un dossier…' }).click();
  const dialog = page.getByRole('dialog', { name: 'Installer Faux plugin ?' });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText('Personne n’a relu ce plugin.');
  await expect(dialog).toContainText(`Dossier · ${src}`);
  await expect(dialog).toContainText(/sha256:[0-9a-f]{64} · fichiers copiés/);
  await expect(dialog.locator('.asks li .txt').first()).toContainText('Lance un programme sur ton ordinateur, avec tes droits');
  await expect(dialog).toContainText('Se connecte àhooks.example.com');
  await expect(dialog).toContainText('Garde un secret : Jeton');
  expect(installed(app.data)).toEqual([]);
  await dialog.getByRole('button', { name: 'Installer quand même' }).click();
  await expect(dialog).toBeHidden();
  expect(installed(app.data).map((p) => p.id)).toEqual([ID]);
  expect(fs.existsSync(path.join(app.data, 'plugins', ID, 'escouade-plugin.json'))).toBe(true);

  // Its page: switched off then on for the project, kept in the list of the installed plugins.
  await expect(page.getByRole('heading', { name: /Faux plugin/ })).toBeVisible();
  await page.getByRole('tab', { name: /Projets/ }).click();
  const sw = page.getByRole('tabpanel').getByRole('switch', { name: 'Actif dans demo-api' });
  await expect(sw).toHaveAttribute('aria-checked', 'true');
  await sw.click();
  await expect(sw).toHaveAttribute('aria-checked', 'false');
  const id = projectId(app.data);
  expect(installed(app.data)[0].projects).toEqual({ [id]: false });
  await sw.click();
  await expect(sw).toHaveAttribute('aria-checked', 'true');
  expect(installed(app.data)[0].projects).toEqual({ [id]: true });

  // Réglages › Plugins: its line says what it adds and on how many projects it is on; its switch turns it off and on in
  // Escouade (what « Réglages › Plugins le rallume » says); « Réglages » opens its page in the view, on its settings.
  let settings = await openSettingsPage(page, 'Plugins');
  const row = settings.getByRole('region', { name: 'Installés' }).getByRole('group', { name: 'Faux plugin' });
  await expect(row).toContainText('Faux pluginv1.0.0');
  await expect(row).toContainText('Actif sur 1 projet sur 1');
  const appSwitch = row.getByRole('switch', { name: 'Actif dans Escouade : Faux plugin' });
  await expect(appSwitch).toHaveAttribute('aria-checked', 'true');
  await appSwitch.click();
  await expect(appSwitch).toHaveAttribute('aria-checked', 'false');
  await expect.poll(() => installed(app.data)[0]).toMatchObject({ enabled: false });
  await appSwitch.click();
  await expect(appSwitch).toHaveAttribute('aria-checked', 'true');
  await expect.poll(() => installed(app.data)[0]).toMatchObject({ enabled: true });
  await row.getByRole('button', { name: 'Réglages de Faux plugin, dans la vue Plugins' }).click();
  await expect(settings).toBeHidden();
  await expect(page.getByRole('tab', { name: /^Réglages/ })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('tabpanel', { name: 'Réglages' })).toContainText('Jeton');
  // « Ouvrir la vue Plugins »: the installed ones.
  settings = await openSettingsPage(page, 'Plugins');
  await settings.getByRole('button', { name: 'Ouvrir la vue Plugins' }).click();
  await expect(settings).toBeHidden();
  await expect(side.getByRole('button', { name: /Installés/ })).toHaveAttribute('aria-current', 'page');
  await page
    .getByRole('table', { name: 'Ajoutés' })
    .getByRole('button', { name: /Faux plugin/ })
    .click();

  // Its journal: started from there, its start and its answer to `initialize` read.
  await page.getByRole('tab', { name: 'Journal' }).click();
  const journal = page.getByRole('tabpanel', { name: 'Journal' });
  await expect(journal).toContainText('Prêt, pas encore démarré');
  await journal.getByRole('button', { name: 'Démarrer' }).click();
  await expect(journal).toContainText('En marche', { timeout: 30_000 });
  const log = journal.getByRole('list', { name: 'Journal' });
  await expect(log).toContainText('Démarrage');
  await expect(log).toContainText('Prêt', { timeout: 15_000 });
  expect(fs.existsSync(path.join(app.data, 'plugin-logs', `${ID}.log`))).toBe(true);

  // Uninstalled: its process stopped, its copy and its journal gone, the folder it came from left.
  await page.getByRole('button', { name: 'Désinstaller' }).click();
  const confirm = page.getByRole('dialog', { name: 'Désinstaller Faux plugin ?' });
  await expect(
    confirm.getByRole('checkbox', { name: 'Garder ses réglages et ses secrets pour une prochaine installation' }),
  ).not.toBeChecked();
  await confirm.getByRole('button', { name: 'Désinstaller' }).click();
  await expect(page.getByText('Aucun plugin installé.')).toBeVisible();
  expect(installed(app.data)).toEqual([]);
  expect(fs.existsSync(path.join(app.data, 'plugins', ID))).toBe(false);
  expect(fs.existsSync(path.join(app.data, 'plugin-logs', `${ID}.log`))).toBe(false);
  expect(fs.existsSync(path.join(src, 'escouade-plugin.json'))).toBe(true);
});

// ESC-329: through the app's own commands, a plugin that decides runs before any request as soon as it is ready and on; one that
// does not decide waits for its first use.
test('a plugin that decides starts at its installation and when it is switched on again, another does not', async ({ app }) => {
  test.setTimeout(120_000);
  const { page } = app;
  const invoke = <T>(cmd: string, args: Record<string, unknown>) =>
    page.evaluate(
      ([cmd, args]) =>
        (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
          cmd as string,
          args,
        ),
      [cmd, args] as const,
    ) as Promise<T>;
  const phase = (id: string) => invoke<{ status: { phase: string } }>('plugin_run', { id }).then((r) => r.status.phase);
  const install = async (path: string) => {
    const { token } = await invoke<{ token: string }>('plugin_inspect_folder', { path });
    await invoke('plugin_install', { token });
  };

  const DECIDER = 'dev.test.decider';
  const decider = path.join(app.root, 'decider');
  fakePlugin(decider, {
    id: DECIDER,
    run: { command: 'fake-plugin', windows: 'fake-plugin.cmd', args: ['--decide', 'allow'] },
    activation: [],
    permissions: { decide: true },
    contributes: {},
  });
  const other = path.join(app.root, 'other');
  fakePlugin(other);

  await install(decider);
  await expect.poll(() => phase(DECIDER), { timeout: 30_000 }).toBe('running');
  await install(other);
  await expect.poll(() => phase(ID)).toBe('idle');

  await invoke('plugin_set_enabled', { id: DECIDER, on: false });
  await expect.poll(() => phase(DECIDER)).toBe('idle');
  await invoke('plugin_set_enabled', { id: DECIDER, on: true });
  await expect.poll(() => phase(DECIDER), { timeout: 30_000 }).toBe('running');

  await invoke('plugin_set_permission', { id: DECIDER, word: 'decide', granted: false });
  await expect.poll(() => phase(DECIDER)).toBe('idle');
  await invoke('plugin_set_permission', { id: DECIDER, word: 'decide', granted: true });
  await expect.poll(() => phase(DECIDER), { timeout: 30_000 }).toBe('running');

  await invoke('plugin_set_enabled', { id: ID, on: false });
  await invoke('plugin_set_enabled', { id: ID, on: true });
  expect(await phase(ID)).toBe('idle');
});
