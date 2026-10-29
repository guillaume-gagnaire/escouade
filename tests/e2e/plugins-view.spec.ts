import fs from 'node:fs';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

// The view Plugins (PLG-4, ESC-176): the fake plugin (tests/fixtures/fake-plugin.mjs) installed from a folder through the
// install window — its origin, its fingerprint and what it may do shown before anything is installed —, switched off then
// on again for a project, its process started from its journal, which then tells its start; uninstalled, its copy and
// its journal gone.

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const ID = 'dev.test.fake';

/** The fake plugin in a folder of the test's own, as an author would have it. */
function fakePlugin(dir: string) {
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
    ? (JSON.parse(fs.readFileSync(file, 'utf8')).plugins as { id: string; projects?: Record<string, boolean> }[])
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
