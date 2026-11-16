// « Projets de l'équipe » (TS-8) with two clients on the real app, against a real `escouade-team-server` started by the
// test. Alice, the administrator, adds two projects to the team's list: « Acme API » (github.com/acme/api) and « Site
// ACME », whose repository Bob has cloned already. Bob, a member on a data folder of his own, opens « Projets de
// l'équipe »: his clone of the site is known by its address and attached to the team without a new clone, its own name
// kept and said; « Cloner » on « Acme API » opens the wizard « Nouveau projet » with the address shown whole, clones it
// by his own git (a local repository served as the hosts' tests serve theirs: `insteadOf`), and the project arrives
// attached to the team.
//
// The team is made and joined by the backend's commands: the screens of TS-3 are `team.spec.ts`'s. The server's port:
// the worktree's range (`ESCOUADE_PORT_BASE` + 4) when the run has one, else one the system gives.

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

/** The address of the list, and the local repository git reaches it at. */
const API_URL = 'https://github.com/acme/api.git';
const SITE_URL = 'https://github.com/acme/site.git';
const ORIGIN = path.join(os.tmpdir(), `escouade-e2e-team-projects-${process.env.ESCOUADE_PORT_BASE ?? 'any'}`);
const BARE = path.join(ORIGIN, 'api.git');

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', ...args], { cwd, encoding: 'utf8' }).trim();

let server: ChildProcess | null = null;
let host = '';
let token = '';
let dir = '';

async function freePort(): Promise<number> {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  if (base) return base + 4;
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
  // The repository of « Acme API », as its host would serve it.
  fs.rmSync(ORIGIN, { recursive: true, force: true });
  const work = path.join(ORIGIN, 'work');
  fs.mkdirSync(work, { recursive: true });
  git(work, 'init', '-q', '-b', 'main');
  fs.writeFileSync(path.join(work, 'README.md'), '# Acme API\n');
  git(work, 'add', '.');
  git(work, 'commit', '-q', '-m', 'init');
  git(ORIGIN, 'clone', '-q', '--bare', work, BARE);

  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'team-projects-'));
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
  fs.rmSync(ORIGIN, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
});

// git, for the app alone: the list's address reaches the local repository (as the e2e of the hosts do).
test.use({
  appEnv: {
    ESCOUADE_TEAMS_EVERY_MS: '800',
    GIT_CONFIG_COUNT: '1',
    GIT_CONFIG_KEY_0: `url.${BARE.replace(/\\/g, '/')}.insteadOf`,
    GIT_CONFIG_VALUE_0: API_URL,
  },
});

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** Screenshots of the window in both themes (ESCOUADE_E2E_SHOTS=<folder>). */
async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['light', 'dark'] as const) {
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
      document.documentElement.style.colorScheme = t;
    }, t);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${t}.png`) });
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

/** « Projets de l'équipe » of the team ACME, from Réglages › Avancé › « Ouvrir les équipes… ». */
async function openTeamProjects(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  const teams = page.getByRole('dialog', { name: 'Équipes' });
  await teams.getByRole('region', { name: 'ACME' }).getByRole('button', { name: 'Projets de l’équipe' }).click();
  const dialog = page.getByRole('dialog', { name: 'Projets de l’équipe' });
  await expect(dialog.getByText(/La liste des projets de l’équipe, chiffrée/)).toBeVisible();
  return dialog;
}

test('an administrator lists a project, the other client clones it from the wizard and it arrives attached to the team', async ({
  app,
}) => {
  test.setTimeout(400_000);
  let page = app.page;

  // Alice makes the team and invites Bob.
  const created = await ipc<{ team: { id: string } }>(page, 'team_create', {
    server: host,
    token,
    name: 'ACME',
    escrowPublic: null,
    escrowHolder: null,
  });
  const team = created.team.id;
  const invite = await ipc<{ url: string }>(page, 'team_invite', { team, role: 'member', hours: 72 });

  // Bob, a data folder of his own, with his clone of the site; he asks to join.
  git(app.repo, 'remote', 'add', 'origin', 'git@github.com:Acme/site');
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  fs.writeFileSync(path.join(bobData, 'settings.json'), fs.readFileSync(path.join(app.data, 'settings.json')));
  page = await app.restartAs(bobData);
  await addProject(page, app.repo, { name: 'site-local' });
  await ipc(page, 'team_join', { link: invite.url, name: 'ACME' });

  // Alice, online, lets him in; then she adds the two projects to the list.
  page = await app.restartAs(app.data);
  await expect.poll(async () => (await ipc<{ members: number }[]>(page, 'team_list'))[0]?.members, { timeout: 60_000 }).toBe(2);
  let dialog = await openTeamProjects(page);
  await expect(dialog.getByText('La liste de l’équipe ne compte encore aucun projet : ajoute le premier.')).toBeVisible();
  for (const [name, url, branch] of [
    ['Acme API', API_URL, 'main'],
    ['Site ACME', SITE_URL, ''],
  ]) {
    await dialog.getByRole('button', { name: 'Ajouter un projet' }).click();
    const form = dialog.getByRole('region', { name: 'Ajouter un projet à la liste' });
    await form.getByRole('textbox', { name: 'Nom' }).fill(name);
    await form.getByRole('textbox', { name: 'Adresse du dépôt 1' }).fill(url);
    if (branch) await form.getByRole('textbox', { name: 'Branche par défaut' }).fill(branch);
    await form.getByRole('button', { name: 'Enregistrer' }).click();
    await expect(dialog.getByRole('article', { name })).toBeVisible({ timeout: 30_000 });
  }
  await shots(page, 'team-projects-admin');
  // A dangerous address never enters the list.
  await dialog.getByRole('button', { name: 'Ajouter un projet' }).click();
  const form = dialog.getByRole('region', { name: 'Ajouter un projet à la liste' });
  await form.getByRole('textbox', { name: 'Nom' }).fill('Piège');
  await form.getByRole('textbox', { name: 'Adresse du dépôt 1' }).fill('--upload-pack=calc.exe');
  await form.getByRole('button', { name: 'Enregistrer' }).click();
  await expect(dialog.getByRole('alert')).toContainText('git would read an option');
  await form.getByRole('button', { name: 'Annuler' }).click();
  // The server reads neither the names nor the addresses.
  const disk = fs
    .readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => fs.readFileSync(path.join(e.parentPath, e.name)).toString('latin1'))
    .join('');
  for (const clear of ['Acme API', 'Site ACME', 'acme/api', 'github.com']) expect(disk).not.toContain(clear);

  // Bob opens « Projets de l'équipe »: his clone of the site is there, known by its address.
  page = await app.restartAs(bobData);
  dialog = await openTeamProjects(page);
  const here = dialog.getByRole('region', { name: 'Sur ce poste' });
  const site = here.getByRole('article', { name: 'Site ACME' });
  await expect(site).toBeVisible({ timeout: 30_000 });
  await expect(site).toContainText('Ici « site-local », dans la liste « Site ACME » : ton nom et ton logo restent.');
  await shots(page, 'team-projects-member');
  await site.getByRole('button', { name: 'Rattacher à l’équipe' }).click();
  await expect(site.getByText('Projet de l’équipe', { exact: true })).toBeVisible();

  // « Cloner » on « Acme API »: the wizard, the address whole before the clone.
  const elsewhere = dialog.getByRole('region', { name: 'À cloner' });
  await elsewhere.getByRole('article', { name: 'Acme API' }).getByRole('button', { name: 'Cloner' }).click();
  const wizard = page.getByRole('dialog', { name: 'Nouveau projet' });
  const block = wizard.getByRole('region', { name: 'Cloner « Acme API » de l’équipe ACME' });
  await expect(block.getByTestId('clone-repos')).toContainText(API_URL);
  await expect(block).toContainText('le serveur d’équipe ne donne aucun accès');
  const parent = path.join(app.root, 'bob-src');
  fs.mkdirSync(parent);
  await block.getByRole('textbox', { name: 'Dans le dossier' }).fill(parent);
  await expect(block.getByRole('textbox', { name: 'Nom du dossier' })).toHaveValue('acme-api');
  await shots(page, 'team-projects-clone');
  await block.getByRole('button', { name: 'Cloner' }).click();
  await expect(wizard.getByText(/Cloné dans .*acme-api\. Le projet sera un projet de l’équipe ACME\./)).toBeVisible({ timeout: 60_000 });
  await expect(wizard.getByText(/Dépôt git détecté · branche main/)).toBeVisible();
  await wizard.getByRole('button', { name: /^Créer sans l.assistant$/ }).click();
  await expect(wizard).toBeHidden();
  expect(fs.existsSync(path.join(parent, 'acme-api', 'README.md'))).toBe(true);
  expect(git(path.join(parent, 'acme-api'), 'config', '--get', 'remote.origin.url')).toBe(API_URL);

  // The project arrives attached to the team: both are « Sur ce poste », projects of the team.
  dialog = await openTeamProjects(page);
  const api = dialog.getByRole('region', { name: 'Sur ce poste' }).getByRole('article', { name: 'Acme API' });
  await expect(api.getByText('Projet de l’équipe', { exact: true })).toBeVisible({ timeout: 30_000 });
  await expect(dialog.getByRole('region', { name: 'À cloner' })).toHaveCount(0);
  await shots(page, 'team-projects-attached');

  // The keychain keeps nothing of the test: each client forgets the team.
  const forget = (p: Page) =>
    p.evaluate(async () => {
      const ipc = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      const list = (await ipc.invoke('team_list')) as { id: string }[];
      for (const t of list) await ipc.invoke('team_forget', { team: t.id });
    });
  await page.keyboard.press('Escape');
  await forget(page);
  page = await app.restartAs(app.data);
  await forget(page);
});
