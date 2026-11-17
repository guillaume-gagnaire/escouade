// « Rattacher à l'équipe » and « Verser à l'équipe » (TS-7) with two clients on the real app, against a real
// `escouade-team-server` started by the test. Alice, the administrator, has a clone of « Acme API » whose
// `.escouade/tickets/` holds two tickets shared by files (ESC-190). She lists it, attaches it, shares its Kanban (the
// list learns it: she is an administrator) and pours its tickets after the preview: the files are frozen, named to be
// removed by hand, nothing removed. Bob, a member on a data folder of his own, clones « Acme API » from the list: his
// project is the team's, its Kanban joined without an id to copy, and the two tickets are on it once each (his clone has
// the files too: the same ids, never a double), with the keys of the team. The repository's spec page (TS-9) goes to the
// server with the same gesture, named in the preview, never before it: Bob reads it from the server.
//
// The team is made and joined by the backend's commands (the screens of TS-3 are `team.spec.ts`'s). The server's port:
// the worktree's range (`ESCOUADE_PORT_BASE` + 4, as `team-projects.spec.ts`: the specs run one at a time) when the run
// has one, else one the system gives.

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

const API_URL = 'https://github.com/acme/api.git';
const ORIGIN = path.join(os.tmpdir(), `escouade-e2e-team-attach-${process.env.ESCOUADE_PORT_BASE ?? 'any'}`);
const BARE = path.join(ORIGIN, 'api.git');
const BOB = { userName: 'Bob Durand', userId: '5d0e77b1', nameAsked: true };

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', ...args], { cwd, encoding: 'utf8' }).trim();

/** A ticket's file as ESC-190 writes it. */
const ticketFile = (id: string, key: string, title: string) =>
  [
    '# A ticket of the Kanban shared by Escouade: https://escouade.dev/en/docs/shared-board',
    'version: 1',
    `id: ${id}`,
    `key: ${key}`,
    `title: ${title}`,
    'status: todo',
    'author: { name: Carol Petit, id: c0c0c0c0 }',
    `description: ${title}, décrit`,
    'criteria:',
    '  - Fait',
    '',
  ].join('\n');

/** A page of the spec, as the repository holds it before the team. */
const PAGE = ['---', 'title: "Accueil"', 'subject: "Général"', '---', '', 'Bonjour l’équipe.', ''].join('\n');

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
  // « Acme API » as its host serves it: two tickets shared by files, committed.
  fs.rmSync(ORIGIN, { recursive: true, force: true });
  const work = path.join(ORIGIN, 'work');
  fs.mkdirSync(path.join(work, '.escouade', 'tickets'), { recursive: true });
  git(work, 'init', '-q', '-b', 'main');
  fs.writeFileSync(path.join(work, 'README.md'), '# Acme API\n');
  fs.writeFileSync(path.join(work, '.escouade', 'tickets', 'k7m2pq9x.yml'), ticketFile('k7m2pq9x', 'DEM-12', 'Le bail de dix minutes'));
  fs.writeFileSync(path.join(work, '.escouade', 'tickets', 'r4tz8wn3.yml'), ticketFile('r4tz8wn3', 'DEM-13', 'Les clés communes'));
  fs.mkdirSync(path.join(work, '.escouade', 'spec', 'general'), { recursive: true });
  fs.writeFileSync(path.join(work, '.escouade', 'spec', 'general', 'accueil.md'), PAGE);
  git(work, 'add', '.');
  git(work, 'commit', '-q', '-m', 'init');
  git(ORIGIN, 'clone', '-q', '--bare', work, BARE);

  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'team-attach-'));
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

// git, for the app alone: the list's address reaches the local repository.
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

/** The project's id, as the data folder keeps it once it is saved (a moment after its creation). */
async function projectId(data: string, name: string): Promise<string> {
  const read = () => {
    try {
      const state = JSON.parse(fs.readFileSync(path.join(data, 'state.json'), 'utf8')) as { projects: { id: string; name: string }[] };
      return state.projects.find((p) => p.name === name)?.id ?? '';
    } catch {
      return '';
    }
  };
  await expect.poll(read, { timeout: 30_000 }).not.toBe('');
  return read();
}

const cards = (page: Page, title: string) => page.locator('.card', { hasText: title });

async function board(page: Page) {
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByRole('region', { name: 'À faire' })).toBeVisible();
}

test('a project attached pours its tickets of files, and a teammate who clones it sees its Kanban without an id to copy', async ({
  app,
}) => {
  test.setTimeout(400_000);
  let page = app.page;

  const created = await ipc<{ team: { id: string } }>(page, 'team_create', {
    server: host,
    token,
    name: 'ACME',
    escrowPublic: null,
    escrowHolder: null,
  });
  const team = created.team.id;
  const invite = await ipc<{ url: string }>(page, 'team_invite', { team, role: 'member', hours: 72 });

  // Bob asks to join, from a data folder of his own.
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  const settings = JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
  fs.writeFileSync(path.join(bobData, 'settings.json'), JSON.stringify({ ...settings, ...BOB }));
  page = await app.restartAs(bobData);
  await ipc(page, 'team_join', { link: invite.url, name: 'ACME' });

  // Alice lets him in; her clone of « Acme API », its two tickets of files on her board.
  page = await app.restartAs(app.data);
  await expect.poll(async () => (await ipc<{ members: number }[]>(page, 'team_list'))[0]?.members, { timeout: 60_000 }).toBe(2);
  const aliceRepo = path.join(app.root, 'alice-api');
  git(app.root, 'clone', '-q', BARE, aliceRepo);
  git(aliceRepo, 'remote', 'set-url', 'origin', API_URL);
  await addProject(page, aliceRepo, { name: 'Acme API', firstAgent: false });
  await board(page);
  await expect(cards(page, 'Le bail de dix minutes')).toHaveCount(1, { timeout: 30_000 });
  await ipc(page, 'team_project_put', {
    team,
    project: null,
    listed: {
      name: 'Acme API',
      color: '',
      logo: null,
      repos: [{ url: API_URL, folder: '' }],
      branch: '',
      access: { kind: 'team' },
      board: '',
    },
  });

  // « Rattacher à l'équipe » in « Projets de l'équipe »: the key `team` of `.escouade.yml` is written.
  let dialog = await openTeamProjects(page);
  const here = dialog.getByRole('region', { name: 'Sur ce poste' }).getByRole('article', { name: 'Acme API' });
  await expect(here.getByTestId('team-project-board')).toHaveText('Pas encore de Kanban de l’équipe');
  await here.getByRole('button', { name: 'Rattacher à l’équipe' }).click();
  await expect(here.getByText('Projet de l’équipe', { exact: true })).toBeVisible({ timeout: 30_000 });
  const yml = fs.readFileSync(path.join(aliceRepo, '.escouade.yml'), 'utf8');
  expect(yml).toContain(`id: ${team}`);
  expect(yml).toContain(`server: http://${host}`);
  expect(yml).toContain('name: ACME');
  await page.keyboard.press('Escape');
  await page.keyboard.press('Escape');
  await board(page);

  // Her Kanban, the team's (an administrator: the list learns it); the files frozen, still there.
  const alicePid = await projectId(app.data, 'Acme API');
  const linked = await ipc<{ teamProject: string }>(page, 'project_link_team_board', { project: alicePid, teamProject: null });
  expect(linked.teamProject).toMatch(/^[0-9a-f]{32}$/);
  await expect
    .poll(async () => (await ipc<{ projects: { board: string }[] }>(page, 'team_projects', { team })).projects[0]?.board, {
      timeout: 30_000,
    })
    .toBe(linked.teamProject);
  const frozen = fs.readFileSync(path.join(aliceRepo, '.escouade', 'tickets', 'k7m2pq9x.yml'), 'utf8');
  expect(frozen.split('\n')[0]).toMatch(/^# Frozen: this project joined a team server/);

  // « Verser à l'équipe »: the preview, then the gesture.
  await page.getByTestId('team-pour-open').click();
  const pour = page.getByRole('dialog', { name: 'Verser à l’équipe' });
  await expect(pour.getByTestId('team-pour-file')).toHaveCount(2);
  await expect(pour).toContainText('Ce qui reste sur ce poste : les images');
  // The spec page goes with the same gesture, named; nothing of it left before.
  const teamDocs = (data: string, pid: string) =>
    path.join(data, 'projects', pid, 'team-docs', '.escouade', 'spec', 'general', 'accueil.md');
  expect(fs.existsSync(teamDocs(app.data, alicePid))).toBe(false);
  await expect(pour.getByTestId('team-pour-docs')).toHaveAttribute('data-state', 'pending');
  await expect(pour.getByTestId('team-pour-docs')).toContainText('spec/general/accueil.md');
  await expect(pour.getByTestId('team-pour-submit')).toHaveText('Verser 2 tickets et 1 fichier de docs');
  await shots(page, 'team-pour-preview');
  await pour.getByTestId('team-pour-submit').click();
  await expect(pour.getByTestId('team-pour-done')).toContainText('Versé : 2 depuis les fichiers', { timeout: 30_000 });
  await expect(pour.getByTestId('team-pour-frozen')).toContainText('.escouade/tickets/k7m2pq9x.yml');
  await expect(pour).toContainText('git rm -r .escouade/tickets');
  await expect(pour.getByTestId('team-pour-docs-done')).toContainText('Docs versées : 1 fichier');
  expect(fs.existsSync(path.join(aliceRepo, '.escouade', 'spec', 'general', 'accueil.md'))).toBe(true);
  await shots(page, 'team-pour-done');
  await pour.getByRole('button', { name: 'Fermer' }).last().click();
  // Once each on her board, with a key of the team, its former key said; nothing removed nor committed.
  await expect(cards(page, 'Le bail de dix minutes')).toHaveCount(1);
  const bail = cards(page, 'Le bail de dix minutes');
  await expect(bail.locator('[data-team-former]')).toContainText('DEM-12', { timeout: 30_000 });
  expect(fs.readdirSync(path.join(aliceRepo, '.escouade', 'tickets'))).toHaveLength(2);
  expect(git(aliceRepo, 'log', '--oneline').split('\n')).toHaveLength(1);
  const aliceKey = /\b([A-Z]+-\d+)\b/.exec(await bail.innerText())?.[1] ?? '';
  expect(aliceKey).not.toBe('DEM-12');

  // Bob clones « Acme API » from the list: the project is the team's, its Kanban joined, its tickets there once each.
  page = await app.restartAs(bobData);
  dialog = await openTeamProjects(page);
  await dialog
    .getByRole('region', { name: 'À cloner' })
    .getByRole('article', { name: 'Acme API' })
    .getByRole('button', { name: 'Cloner' })
    .click();
  const wizard = page.getByRole('dialog', { name: 'Nouveau projet' });
  const block = wizard.getByRole('region', { name: 'Cloner « Acme API » de l’équipe ACME' });
  const parent = path.join(app.root, 'bob-src');
  fs.mkdirSync(parent);
  await block.getByRole('textbox', { name: 'Dans le dossier' }).fill(parent);
  await block.getByRole('button', { name: 'Cloner' }).click();
  await expect(wizard.getByText(/Le projet sera un projet de l’équipe ACME\./)).toBeVisible({ timeout: 60_000 });
  await wizard.getByRole('button', { name: /^Créer sans l.assistant$/ }).click();
  await expect(wizard).toBeHidden();
  await board(page);
  const bobPid = await projectId(bobData, 'Acme API');
  await expect
    .poll(
      () =>
        (
          JSON.parse(fs.readFileSync(path.join(bobData, 'state.json'), 'utf8')) as {
            projects: { id: string; team?: string; teamProject?: string }[];
          }
        ).projects.find((p) => p.id === bobPid)?.teamProject ?? null,
      { timeout: 30_000 },
    )
    .toBe(linked.teamProject);
  // Bob reads the team's spec from the server: its page in his machine's copy.
  await expect.poll(() => fs.existsSync(teamDocs(bobData, bobPid)), { timeout: 30_000 }).toBe(true);
  for (const title of ['Le bail de dix minutes', 'Les clés communes']) {
    await expect(cards(page, title)).toHaveCount(1, { timeout: 30_000 });
    await expect(cards(page, title).locator('[data-team-former]')).toBeVisible();
  }
  await expect(cards(page, 'Le bail de dix minutes')).toContainText(aliceKey);
  await shots(page, 'team-attach-bob-board');

  // The keychain keeps nothing of the test.
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
