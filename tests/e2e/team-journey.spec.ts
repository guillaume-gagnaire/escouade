// The team server from end to end (TS-18), two clients at the same time on the real app against a real
// `escouade-team-server` started by the test: Alice makes the team and invites, Bob joins; their project becomes the
// team's, its Kanban shared; Alice makes a ticket and launches it, Bob sees it taken by her within a second; Alice pours
// the repository's spec to the team (« Verser à l'équipe »), Bob reads it from his machine's copy. Then the server goes
// away: Bob's Docs say « Hors ligne », he changes a ticket's title and a page meanwhile; back, both reach Alice. The
// other specs `team*.spec.ts` each take a part of it further (the screens of TS-3, the settings of TS-13, the list of
// TS-8, the docs of TS-9, the attachment of TS-7, the conflicts of TS-6).
//
// Not here: the revocation of a device, which has no screen nor command in the app yet (TS-4, Réglages › Équipe); the
// crate's tests play it against the real server, on SQLite and PostgreSQL (`crates/escouade-team/tests/sync.rs`,
// `a_member_gone_reads_nothing_new`; `devices.rs`, the stolen device).
//
// The team is made and joined by the backend's commands (the screens of TS-3 are `team.spec.ts`'s). The server's port:
// the worktree's range (`ESCOUADE_PORT_BASE` + 9, as `team.spec.ts`: the specs run one at a time) when the run has one,
// else one the system gives. The second app's DevTools port: `ESCOUADE_E2E_SECOND_CDP_PORT`, else the first one's + 1.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { letIn } from './team-admit';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);

const ALICE = { userName: 'Alice Martin', userId: 'a91f03c2', nameAsked: true };
const BOB = { userName: 'Bob Durand', userId: '5d0e77b1', nameAsked: true };
const PAGE = 'general/accueil';

let server: ChildProcess | null = null;
let host = '';
let token = '';
let dir = '';

async function freePort(): Promise<number> {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  if (base) return base + 9;
  return new Promise((resolve) => {
    const s = net.createServer();
    s.listen(0, '127.0.0.1', () => {
      const port = (s.address() as net.AddressInfo).port;
      s.close(() => resolve(port));
    });
  });
}

async function startServer() {
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
}

async function stopServer() {
  if (!server?.pid) return;
  const gone = new Promise((r) => server!.once('exit', r));
  if (process.platform === 'win32') {
    try {
      execFileSync('taskkill', ['/PID', String(server.pid), '/T', '/F'], { stdio: 'ignore' });
    } catch {
      // already gone
    }
  } else server.kill();
  await Promise.race([gone, new Promise((r) => setTimeout(r, 5000))]);
  server = null;
}

test.beforeAll(async () => {
  test.setTimeout(600_000);
  if (!fs.existsSync(SERVER)) {
    execFileSync('cargo', ['build', '-p', 'escouade-team-server'], { cwd: path.join(ROOT, 'src-tauri'), stdio: 'inherit' });
  }
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'team-journey-'));
  const out = execFileSync(SERVER, ['init', '--data', dir], { encoding: 'utf8' });
  token = /^\s{4}(\S+)\s*$/m.exec(out)![1];
  host = `127.0.0.1:${await freePort()}`;
  await startServer();
});

test.afterAll(async () => {
  await stopServer();
  if (dir) fs.rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
});

test.use({ preStart: { run: () => ALICE }, appEnv: { ESCOUADE_TEAMS_EVERY_MS: '800' } });

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

function git(cwd: string, ...args: string[]) {
  return execFileSync('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', ...args], { cwd, encoding: 'utf8' });
}

async function board(page: Page) {
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByRole('region', { name: 'À faire' })).toBeVisible();
}

async function openDocs(page: Page) {
  await page.getByRole('group', { name: 'Vue du projet' }).getByRole('button', { name: 'Docs' }).click();
}

/** A ticket made from the form of the board: its key comes from the server's counter. */
async function newTicket(page: Page, title: string) {
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  const form = page.getByRole('dialog', { name: 'Nouveau ticket' });
  await expect(form.getByRole('switch', { name: 'Ne pas partager' })).toHaveAttribute('aria-checked', 'false');
  await form.getByRole('textbox', { name: 'Titre du ticket' }).fill(title);
  await form.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(form).toBeHidden();
}

test('a team from end to end: invite, join, share a Kanban, a ticket taken, the spec poured, offline and back', async ({ app }) => {
  test.setTimeout(480_000);
  const alice = app.page;

  // The repository has a page of spec, committed: Alice will pour it to the team.
  fs.mkdirSync(path.join(app.repo, '.escouade', 'spec', 'general'), { recursive: true });
  fs.writeFileSync(path.join(app.repo, '.escouade', 'spec', 'general', 'accueil.md'), '# Accueil\n\nLe produit en une phrase.\n');
  git(app.repo, 'add', '.escouade');
  git(app.repo, 'commit', '-q', '-m', 'spec');

  // Alice makes the team and invites; Bob, a second app at the same time on a clone of the repository, joins and is let
  // in by Alice's app.
  const created = await ipc<{ team: { id: string } }>(alice, 'team_create', {
    server: host,
    token,
    name: 'ACME',
    escrowPublic: null,
    escrowHolder: null,
  });
  const team = created.team.id;
  const invite = await ipc<{ url: string }>(alice, 'team_invite', { team, role: 'member', hours: 72 });
  const bobRepo = path.join(app.root, 'bob-repo');
  execFileSync('git', ['clone', '-q', app.repo, bobRepo]);
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  const settings = JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
  fs.writeFileSync(path.join(bobData, 'settings.json'), JSON.stringify({ ...settings, ...BOB }));
  const bob = await app.second(bobData);
  const asked = await ipc<{ joinWords: string[] }>(bob, 'team_join', { link: invite.url, name: 'ACME' });
  // Alice compares the six words of the request with Bob's screen, and lets him in (SR2-08).
  await letIn(alice, asked.joinWords);
  await expect.poll(async () => (await ipc<{ joined: boolean }[]>(bob, 'team_list'))[0]?.joined, { timeout: 60_000 }).toBe(true);
  await expect.poll(async () => (await ipc<{ members: number }[]>(alice, 'team_list'))[0]?.members, { timeout: 60_000 }).toBe(2);

  // The project, the team's on both; Alice's Kanban shared with the team, Bob's joins it by its id.
  await addProject(alice, app.repo, { firstAgent: false, name: 'demo' });
  await addProject(bob, bobRepo, { firstAgent: false, name: 'demo' });
  const alicePid = await projectId(app.data, 'demo');
  const bobPid = await projectId(bobData, 'demo');
  await ipc(alice, 'project_set_team', { project: alicePid, team });
  await ipc(bob, 'project_set_team', { project: bobPid, team });
  const linked = await ipc<{ teamProject: string }>(alice, 'project_link_team_board', { project: alicePid, teamProject: null });
  expect(linked.teamProject).toMatch(/^[0-9a-f]{32}$/);
  await ipc(bob, 'project_link_team_board', { project: bobPid, teamProject: linked.teamProject });
  await board(alice);
  await board(bob);

  // Alice makes a ticket; Bob's board shows it. She launches it: Bob sees it taken by her within a second, without
  // « Lancer ».
  await newTicket(alice, 'La prise atomique');
  const aliceCard = alice.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /DEM-1 La prise atomique/ });
  await expect(aliceCard).toBeVisible({ timeout: 15_000 });
  await expect(bob.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /DEM-1 La prise atomique/ })).toBeVisible({
    timeout: 15_000,
  });
  await aliceCard.getByRole('button', { name: 'Lancer' }).click({ timeout: 15_000 });
  await alice.waitForSelector('[data-team-holder="here"]', { timeout: 30_000 });
  const held = Date.now();
  await bob.waitForSelector('[data-team-holder="held"]', { timeout: 10_000 });
  const seen = Date.now() - held;
  expect(seen, `seen taken after ${seen} ms`).toBeLessThan(1000);
  const bobCard = bob.locator('.card', { hasText: 'La prise atomique' });
  await expect(bobCard.locator('[data-team-holder="held"]')).toContainText('Pris par Alice Martin');
  await expect(bobCard.getByRole('button', { name: 'Lancer' })).toHaveCount(0);

  // A second ticket, for the time offline.
  await newTicket(alice, 'Les clés communes');
  await expect(bob.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /DEM-2 Les clés communes/ })).toBeVisible({
    timeout: 15_000,
  });

  // « Verser à l'équipe »: the spec page goes to the server, named in the preview; Bob reads it from his copy.
  await alice.getByTestId('team-pour-open').click();
  const pour = alice.getByRole('dialog', { name: 'Verser à l’équipe' });
  await expect(pour.getByTestId('team-pour-docs')).toContainText('spec/general/accueil.md');
  await expect(pour.getByTestId('team-pour-submit')).toContainText('1 fichier de docs');
  await pour.getByTestId('team-pour-submit').click();
  await expect(pour.getByTestId('team-pour-docs-done')).toContainText('Docs versées : 1 fichier', { timeout: 30_000 });
  await pour.getByRole('button', { name: 'Fermer' }).last().click();
  const bobCopy = path.join(bobData, 'projects', bobPid, 'team-docs', '.escouade', 'spec', 'general', 'accueil.md');
  await expect.poll(() => fs.existsSync(bobCopy), { timeout: 60_000 }).toBe(true);
  await expect
    .poll(async () => (await ipc<{ body: string }>(bob, 'docs_spec_page', { id: bobPid, path: PAGE })).body, { timeout: 30_000 })
    .toContain('Le produit en une phrase.');
  // The server keeps neither the page nor the tickets' titles.
  const disk = fs
    .readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => fs.readFileSync(path.join(e.parentPath, e.name)).toString('latin1'))
    .join('');
  for (const clear of ['Le produit', 'prise atomique', 'clés communes', 'accueil']) expect(disk).not.toContain(clear);

  // The server goes away: Bob's Docs say so; meanwhile he changes the second ticket's title and the page.
  const ticket = (await bob.locator('.card', { hasText: 'Les clés communes' }).getAttribute('data-ticket-id', { timeout: 15_000 }))!;
  await stopServer();
  const status = (p: Page) => ipc<{ online: boolean | null; pending: number }>(p, 'team_sync_status', { team });
  await expect.poll(async () => (await status(bob)).online, { timeout: 30_000 }).toBe(false);
  await openDocs(bob);
  await expect(bob.locator('[data-sync="offline"]')).toBeVisible({ timeout: 30_000 });
  await ipc(bob, 'ticket_update', {
    id: ticket,
    draft: { title: 'Les clés communes (hors ligne)', description: '', criteria: [], maxLoops: 3 },
  });
  await ipc(bob, 'docs_spec_write', { id: bobPid, path: PAGE, body: '# Accueil\n\nÉcrit hors ligne.\n' });
  await expect.poll(async () => (await status(bob)).pending, { timeout: 30_000 }).toBeGreaterThan(0);

  // Back: Bob's changes leave, and reach Alice.
  await startServer();
  await expect.poll(async () => (await status(bob)).online, { timeout: 60_000 }).toBe(true);
  await expect.poll(async () => (await status(bob)).pending, { timeout: 60_000 }).toBe(0);
  await expect(bob.locator('[data-sync="synced"]')).toBeVisible({ timeout: 60_000 });
  await expect(alice.locator('.card', { hasText: 'Les clés communes (hors ligne)' })).toBeVisible({ timeout: 60_000 });
  await expect
    .poll(async () => (await ipc<{ body: string }>(alice, 'docs_spec_page', { id: alicePid, path: PAGE })).body, { timeout: 60_000 })
    .toContain('Écrit hors ligne.');

  // The keychain keeps nothing of the test: each client forgets the team.
  for (const p of [alice, bob]) {
    await p.evaluate(async () => {
      const ipc = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      const list = (await ipc.invoke('team_list')) as { id: string }[];
      for (const t of list) await ipc.invoke('team_forget', { team: t.id }).catch(() => {});
    });
  }
});
