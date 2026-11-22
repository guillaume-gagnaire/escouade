// The team's private store of plugins (TS-14) with two clients on the real app, against a real `escouade-team-server`
// started by the test. Alice, the administrator, makes the team's key on her device (a minisign pair and its backup,
// sealed by a password, the backup written in a folder of her choice) and declares the namespace `fr.acme.*`; she
// proposes « Règles d'Acme » from its folder (the store's checks played on her device), plays them again and signs it;
// a plugin out of the namespace is refused. Bob, a member on a data folder of his own, finds it in the view Plugins,
// « ACME · privé », and installs it by his own gesture, after the window of its permissions.
//
// The team is made and joined by the backend's commands (the screens of TS-3 are `team.spec.ts`'s). The server's port:
// the worktree's range (`ESCOUADE_PORT_BASE` + 5: the specs run one at a time) when the run has one, else one the system
// gives. ESCOUADE_E2E_SHOTS=<folder> keeps the screenshots, light and dark, at the run's size. No key of the repository:
// the test's are drawn by the app.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);
const PASSWORD = 'un mot de passe de test assez long';

let server: ChildProcess | null = null;
let host = '';
let token = '';
let dir = '';

async function freePort(): Promise<number> {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  if (base) return base + 5;
  return new Promise((resolve) => {
    const s = net.createServer();
    s.listen(0, '127.0.0.1', () => {
      const port = (s.address() as net.AddressInfo).port;
      s.close(() => resolve(port));
    });
  });
}

test.beforeAll(async () => {
  test.setTimeout(600_000);
  if (!fs.existsSync(SERVER)) {
    execFileSync('cargo', ['build', '-p', 'escouade-team-server'], { cwd: path.join(ROOT, 'src-tauri'), stdio: 'inherit' });
  }
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'team-plugins-'));
  const out = execFileSync(SERVER, ['init', '--data', dir], { encoding: 'utf8' });
  token = /^\s{4}(\S+)\s*$/m.exec(out)![1];
  host = `127.0.0.1:${await freePort()}`;
  server = spawn(SERVER, ['serve', '--data', dir, '--listen', host], { stdio: 'ignore' });
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch(`http://${host}/healthz`)).ok) return;
    } catch {
      // not yet
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`escouade-team-server did not answer on ${host}`);
});

test.afterAll(async () => {
  if (server?.pid) {
    const gone = new Promise((r) => server!.once('exit', r));
    if (process.platform === 'win32') {
      try {
        execFileSync('taskkill', ['/PID', String(server.pid), '/T', '/F'], { stdio: 'ignore' });
      } catch {
        // already gone
      }
    } else server.kill();
    await Promise.race([gone, new Promise((r) => setTimeout(r, 5000))]);
  }
  if (dir) fs.rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
});

test.use({ appEnv: { ESCOUADE_TEAMS_EVERY_MS: '800' } });

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZE = (process.env.ESCOUADE_E2E_VIEWPORT ?? '1440x900').split('x')[0];

/** Screenshots of the window in both themes, at the run's size. */
async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const [t, word] of [
    ['light', 'clair'],
    ['dark', 'sombre'],
  ] as const) {
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
      document.documentElement.style.colorScheme = t;
    }, t);
    await page.waitForTimeout(200);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${word}-${SIZE}.png`), animations: 'disabled' });
  }
  await page.evaluate(() => {
    delete document.documentElement.dataset.theme;
    document.documentElement.style.colorScheme = '';
  });
}

/** A command of the backend, as the window calls it. */
function ipc<T>(page: Page, cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  return page.evaluate(
    ([c, a]) =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
        c as string,
        a,
      ),
    [cmd, args] as const,
  ) as Promise<T>;
}

/** A declarative plugin `id`@`version` in `folder`. */
function plugin(folder: string, id: string, version: string) {
  fs.mkdirSync(folder, { recursive: true });
  fs.writeFileSync(
    path.join(folder, 'escouade-plugin.json'),
    JSON.stringify(
      {
        id,
        name: { fr: 'Règles d’Acme', en: 'Acme’s rules' },
        description: { fr: 'Les règles internes de l’équipe Acme.', en: 'The Acme team’s own rules.' },
        version,
        escouade: { api: '1', min: '2.0.0' },
        contributes: {},
      },
      null,
      2,
    ),
  );
}

/** « Plugins de l'équipe » of the team ACME, from Réglages › Avancé › « Ouvrir les équipes… ». */
async function openTeamPlugins(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  const teams = page.getByRole('dialog', { name: 'Équipes' });
  await teams.getByRole('region', { name: 'ACME' }).getByRole('button', { name: 'Plugins', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Plugins de l’équipe' });
  await expect(dialog.getByText(/Les plugins internes de l’équipe, signés par sa clé/)).toBeVisible();
  return dialog;
}

test('an administrator publishes a plugin of the team, a member sees it and installs it, a plugin out of the namespace is refused', async ({
  app,
}) => {
  test.setTimeout(540_000);
  let page = app.page;

  // Alice makes the team and invites Bob; Bob, a data folder of his own, asks to join; Alice lets him in.
  const created = await ipc<{ team: { id: string } }>(page, 'team_create', {
    server: host,
    token,
    name: 'ACME',
    escrowPublic: null,
    escrowHolder: null,
  });
  const team = created.team.id;
  const invite = await ipc<{ url: string }>(page, 'team_invite', { team, role: 'member', hours: 72 });
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  fs.writeFileSync(path.join(bobData, 'settings.json'), fs.readFileSync(path.join(app.data, 'settings.json')));
  page = await app.restartAs(bobData);
  await addProject(page, app.repo, { name: 'site-local' });
  await ipc(page, 'team_join', { link: invite.url, name: 'ACME' });
  page = await app.restartAs(app.data);
  await expect.poll(async () => (await ipc<{ members: number }[]>(page, 'team_list'))[0]?.members, { timeout: 60_000 }).toBe(2);

  // Alice makes the team's key on her device: Escouade's own namespace refused, then fr.acme.*.
  let dialog = await openTeamPlugins(page);
  const form = dialog.getByTestId('team-plugin-key-form');
  await form.getByRole('textbox', { name: 'Espace de noms' }).fill('dev.escouade.*');
  await expect(form.getByRole('alert')).toContainText('dev.escouade.* est à Escouade');
  await form.getByRole('textbox', { name: 'Espace de noms' }).fill('fr.acme.*');
  await form.getByLabel('Mot de passe', { exact: true }).fill(PASSWORD);
  await form.getByLabel('Le même, encore').fill(PASSWORD);
  const usb = path.join(app.root, 'usb');
  await form.getByRole('textbox', { name: 'Dossier de la clé de secours' }).fill(usb);
  await shots(page, 'equipe-plugins-cle');
  await form.getByRole('button', { name: 'Créer la clé et déclarer l’espace' }).click();
  await expect(dialog.getByTestId('team-plugin-key')).toContainText('Espace fr.acme.*', { timeout: 30_000 });
  await expect(dialog.getByText(/La clé de secours est dans/)).toBeVisible();
  // The backup key is in the folder chosen, sealed: no secret half in the clear, anywhere.
  const backup = fs.readdirSync(usb).find((f) => f.endsWith('-backup.key'))!;
  expect(fs.readFileSync(path.join(usb, backup), 'utf8')).toMatch(/^untrusted comment: escouade team signing key/);

  // A plugin out of the namespace: refused.
  const other = path.join(app.root, 'other');
  plugin(other, 'com.other.rules', '1.0.0');
  await dialog.getByRole('textbox', { name: 'Dossier du plugin' }).fill(other);
  await dialog.getByPlaceholder('Étiquette').fill('v1.0.0');
  await dialog.getByRole('button', { name: 'Contrôler et proposer' }).click();
  await expect(dialog.getByRole('alert').filter({ hasText: 'com.other.rules' })).toBeVisible({ timeout: 30_000 });

  // « Règles d'Acme »: its checks played here, then again before the signature.
  const rules = path.join(app.root, 'rules');
  plugin(rules, 'fr.acme.rules', '1.0.0');
  await dialog.getByRole('textbox', { name: 'Dossier du plugin' }).fill(rules);
  await dialog.getByRole('button', { name: 'Contrôler et proposer' }).click();
  await expect(dialog.getByText(/Proposé\s: fr\.acme\.rules v1\.0\.0, empreinte/)).toBeVisible({ timeout: 60_000 });
  // The six checks, passed: under the form, and again on the proposal.
  await expect(dialog.getByTestId('team-plugin-check')).toHaveCount(12);
  await expect(dialog.locator('[data-testid="team-plugin-check"][data-ok="false"]')).toHaveCount(0);
  const proposed = dialog.getByTestId('team-plugin-proposed');
  await expect(proposed).toContainText('fr.acme.rules');
  await proposed.getByRole('button', { name: 'Contrôler à nouveau et signer' }).click();
  await proposed.getByLabel('Mot de passe').fill(PASSWORD);
  await shots(page, 'equipe-plugins-signer');
  await proposed.getByRole('button', { name: 'Contrôler à nouveau et signer' }).click();
  await expect(dialog.getByText(/Signé\s: les deux empreintes sont les mêmes/)).toBeVisible({ timeout: 60_000 });
  await expect(dialog.getByTestId('team-plugin-published')).toContainText('ACME · privé');
  await shots(page, 'equipe-plugins-publie');
  // The server reads neither the plugin's id nor its name.
  const disk = fs
    .readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => fs.readFileSync(path.join(e.parentPath, e.name)).toString('latin1'))
    .join('');
  for (const clear of ['fr.acme.rules', 'fr.acme.*', 'Règles']) expect(disk).not.toContain(clear);
  await page.keyboard.press('Escape');

  // Bob finds it in the view Plugins, « ACME · privé », and installs it by his gesture.
  page = await app.restartAs(bobData);
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  const side = page.getByRole('complementary', { name: 'Plugins' });
  await side.getByTestId('plugins-nav-team').click();
  const store = page.getByTestId('team-store');
  await expect(store.getByRole('heading', { name: 'ACME · privé' })).toBeVisible({ timeout: 30_000 });
  const line = store.getByTestId('team-store-plugin').filter({ hasText: 'fr.acme.rules' });
  await expect(line).toContainText('Règles d’Acme');
  await shots(page, 'plugins-equipe');
  await line.getByRole('button', { name: 'Installer' }).click();
  const modal = page.getByRole('dialog', { name: /^Installer Règles d’Acme/ });
  await expect(modal.getByText('Signé par la clé de l’équipe ACME.')).toBeVisible();
  await expect(modal).toContainText('ACME · privé');
  await shots(page, 'plugins-equipe-installer');
  // Nothing is installed before he says so.
  expect((await ipc<{ id: string }[]>(page, 'plugins_list')).some((p) => p.id === 'fr.acme.rules')).toBe(false);
  await modal.getByRole('button', { name: 'Installer', exact: true }).click();
  await expect(modal).toBeHidden({ timeout: 30_000 });
  const installed = await ipc<{ id: string; origin: { kind: string; name: string } }[]>(page, 'plugins_list');
  expect(installed.find((p) => p.id === 'fr.acme.rules')?.origin).toMatchObject({ kind: 'team', name: 'ACME' });
  await side.getByTestId('plugins-nav-team').click();
  await expect(page.getByTestId('team-store-plugin').filter({ hasText: 'fr.acme.rules' })).toContainText('Installé · v1.0.0', {
    timeout: 30_000,
  });

  // The keychain keeps nothing of the test: each client forgets the team.
  const forget = (p: Page) =>
    p.evaluate(async () => {
      const ipc = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      const list = (await ipc.invoke('team_list')) as { id: string }[];
      for (const t of list) await ipc.invoke('team_forget', { team: t.id });
    });
  await forget(page);
  page = await app.restartAs(app.data);
  await forget(page);
});
