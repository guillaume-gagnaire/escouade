// The Kanban of a team (TS-6) on the real app, two clients at the same time against a real `escouade-team-server`
// started by the test: Alice makes the team, Bob joins it, their projects become the team's Kanban. Alice makes a
// ticket (its key from the server's counter) and launches it: Bob sees it taken by her within a second, and cannot
// launch it. Then the server goes away; each one changes the ticket's title meanwhile; back, the last writer wins and the
// loser's card says so (« Alice Martin a changé le titre en même temps · Reprendre le mien »). With
// `ESCOUADE_TEAM_SHOTS=<folder>`, Bob's board is kept there, dark and light.
//
// Its server's port: the worktree's `ESCOUADE_PORT_BASE` + 7 (team.spec.ts takes + 9, team-settings.spec.ts + 8), else
// one the system gives. The second app's DevTools port: `ESCOUADE_E2E_SECOND_CDP_PORT`, else the first one's + 1.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);
const SHOTS = process.env.ESCOUADE_TEAM_SHOTS;

const ALICE = { userName: 'Alice Martin', userId: 'a91f03c2', nameAsked: true };
const BOB = { userName: 'Bob Durand', userId: '5d0e77b1', nameAsked: true };

let server: ChildProcess | null = null;
let host = '';
let token = '';
let dir = '';

async function freePort(): Promise<number> {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  if (base) return base + 7;
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
  dir = fs.mkdtempSync(path.join(process.env.TEMP ?? process.env.TMPDIR ?? '/tmp', 'team-board-'));
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

/** « Réglages de l’équipe » of the team ACME, from Réglages › Avancé › « Ouvrir les équipes… ». */
async function teamSettings(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  await page.getByRole('region', { name: 'ACME' }).getByRole('button', { name: 'Réglages de l’équipe' }).click();
  return page.getByRole('dialog', { name: 'Réglages de l’équipe' });
}

/** The project « demo » made the team's (TS-13), then its Kanban the team's: a new one (`join` null) or a teammate's. */
async function teamBoard(page: Page, join: string | null): Promise<string> {
  const dialog = await teamSettings(page);
  await dialog.getByRole('checkbox', { name: 'demo' }).check();
  const link = dialog.locator('[data-team-board-link]');
  await expect(link).toBeVisible();
  if (join) {
    await link.getByRole('textbox', { name: 'Identifiant du Kanban d’un collègue' }).fill(join);
    await link.getByRole('button', { name: 'Rejoindre' }).click();
  } else {
    await link.getByRole('button', { name: 'Partager son Kanban avec l’équipe' }).click();
  }
  const id = dialog.locator('[data-team-board-id]');
  await expect(id).toBeVisible({ timeout: 30_000 });
  await shoot(page, join ? 'equipe-kanban-rejoint' : 'equipe-kanban-partage', dialog);
  const journal = (await id.getAttribute('data-team-board-id'))!;
  await page.keyboard.press('Escape');
  await page.keyboard.press('Escape');
  return journal;
}

async function board(page: Page) {
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByRole('region', { name: 'À faire' })).toBeVisible();
}

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, name: string, what?: ReturnType<Page['locator']>) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.waitForTimeout(200);
    await (what ?? page).screenshot({ path: path.join(SHOTS, `${name}-${t}.png`), animations: 'disabled' });
  }
  await theme(page, 'dark');
}

test('a ticket of the team taken on one client is seen taken on the other within a second, then a title in conflict', async ({ app }) => {
  test.setTimeout(400_000);
  const alice = app.page;
  await alice.setViewportSize({ width: 1024, height: 728 });

  // Alice makes the team and invites; Bob, a second app at the same time on a clone of the repository, joins.
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
  await bob.setViewportSize({ width: 1024, height: 728 });
  await ipc(bob, 'team_join', { link: invite.url, name: 'ACME' });
  await expect.poll(async () => (await ipc<{ joined: boolean }[]>(bob, 'team_list'))[0]?.joined, { timeout: 60_000 }).toBe(true);

  // Their projects, the team's (TS-13); Alice's Kanban shared with the team, Bob's joins it by its id, in the team's
  // screen.
  await addProject(alice, app.repo, { firstAgent: false, name: 'demo' });
  await addProject(bob, bobRepo, { firstAgent: false, name: 'demo' });
  const journal = await teamBoard(alice, null);
  expect(journal).toMatch(/^[0-9a-f]{32}$/);
  await teamBoard(bob, journal);
  await board(alice);
  await board(bob);

  // Alice makes a ticket from its form: its key comes from the server's counter; Bob's board shows it.
  await alice.getByRole('button', { name: 'Nouveau ticket' }).click();
  const form = alice.getByRole('dialog', { name: 'Nouveau ticket' });
  await expect(form.getByRole('switch', { name: 'Ne pas partager' })).toHaveAttribute('aria-checked', 'false');
  await form.getByRole('textbox', { name: 'Titre du ticket' }).fill('La prise atomique');
  await form.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(form).toBeHidden();
  const key = 'DEM-1';
  const aliceCard = alice.getByRole('region', { name: 'À faire' }).getByRole('button', { name: new RegExp(`${key} La prise atomique`) });
  await expect(aliceCard).toBeVisible({ timeout: 15_000 });
  const bobTodo = bob.getByRole('region', { name: 'À faire' }).getByRole('button', { name: new RegExp(`${key} La prise atomique`) });
  await expect(bobTodo).toBeVisible({ timeout: 15_000 });

  // Alice launches it: the server's claim is hers. Bob sees it taken by her within a second, and has no « Lancer ».
  await aliceCard.getByRole('button', { name: 'Lancer' }).click({ timeout: 15_000 });
  await alice.waitForSelector('[data-team-holder="here"]', { timeout: 30_000 });
  const held = Date.now();
  await bob.waitForSelector('[data-team-holder="held"]', { timeout: 10_000 });
  const seen = Date.now() - held;
  expect(seen, `seen taken after ${seen} ms`).toBeLessThan(1000);
  const bobCard = bob.locator('.card', { hasText: 'La prise atomique' });
  await expect(bobCard.locator('[data-team-holder="held"]')).toContainText('Pris par Alice Martin · depuis');
  await expect(bobCard.getByRole('button', { name: 'Lancer' })).toHaveCount(0);
  await shoot(bob, 'kanban-equipe-pris');

  // A second ticket, « À faire » on both boards.
  await alice.getByRole('button', { name: 'Nouveau ticket' }).click();
  const second = alice.getByRole('dialog', { name: 'Nouveau ticket' });
  await second.getByRole('textbox', { name: 'Titre du ticket' }).fill('Les clés communes');
  await second.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(second).toBeHidden();
  await expect(bob.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /DEM-2 Les clés communes/ })).toBeVisible({
    timeout: 15_000,
  });

  // The server goes away; each one changes its title meanwhile (Bob first: Alice's is the last write).
  await stopServer();
  const ticketOf = async (p: Page) => (await p.locator('.card', { hasText: 'Les clés communes' }).getAttribute('data-ticket-id'))!;
  const tb = await ticketOf(bob);
  const ta = await ticketOf(alice);
  const draft = (title: string) => ({ title, description: '', criteria: [], maxLoops: 3 });
  const waiting = (p: Page) => async () => (await ipc<{ pending: number }>(p, 'team_sync_status', { team })).pending;
  await ipc(bob, 'ticket_update', { id: tb, draft: draft('Les clés communes (Bob)') });
  await expect.poll(waiting(bob), { timeout: 30_000 }).toBeGreaterThan(0);
  await bob.waitForTimeout(50);
  await ipc(alice, 'ticket_update', { id: ta, draft: draft('Les clés communes (Alice)') });
  await expect.poll(waiting(alice), { timeout: 30_000 }).toBeGreaterThan(0);
  await startServer();

  // Back: the outboxes leave; the title is Alice's on both boards, and Bob's card says he lost it, with her name.
  const conflict = bob.locator('[data-team-conflict="title"]');
  await expect(conflict).toContainText('Alice Martin a changé le titre en même temps', { timeout: 60_000 });
  await expect(bob.locator('.card', { hasText: 'Les clés communes (Alice)' })).toBeVisible();
  await expect(alice.locator('.card', { hasText: 'Les clés communes (Alice)' })).toBeVisible();
  await expect(alice.locator('[data-team-conflict]')).toHaveCount(0);
  await shoot(bob, 'kanban-equipe-conflit');
  // « Reprendre le mien »: Bob's title goes to Alice.
  await conflict.getByRole('button', { name: 'Reprendre le mien' }).click();
  await expect(alice.locator('.card', { hasText: 'Les clés communes (Bob)' })).toBeVisible({ timeout: 15_000 });
  await expect(bob.locator('[data-team-conflict]')).toHaveCount(0);

  for (const p of [alice, bob]) {
    await p.evaluate(async () => {
      const ipc = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      const list = (await ipc.invoke('team_list')) as { id: string }[];
      for (const t of list) await ipc.invoke('team_forget', { team: t.id }).catch(() => {});
    });
  }
});
