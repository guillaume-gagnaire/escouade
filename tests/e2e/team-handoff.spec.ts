// Passing an agent to a teammate (TS-17) on the real app, two clients at the same time against a real
// `escouade-team-server` started by the test, and a bare repository as their host. Alice launches a ticket of the team;
// its turn runs ([lent:600]) and she leaves work uncommitted in its worktree. From the card's menu, « Passer à un
// collègue… »: the window says the branch will be pushed on origin, the turn interrupted, the work as a patch; she passes
// it to Bob. Bob's card says « Alice Martin te passe DEM-1 · Prendre », and so does his box « À toi »; he takes it: his
// worktree has the branch and the work, his agent's first message on the fake harness is the handover file, the
// conversation of before above it. With `ESCOUADE_E2E_SHOTS=<folder>`, the window and the card, dark and light, at the
// size of the window (`ESCOUADE_E2E_VIEWPORT`).
//
// Its server's port: the worktree's `ESCOUADE_PORT_BASE` + 5, else one the system gives. The second app's DevTools port:
// `ESCOUADE_E2E_SECOND_CDP_PORT`, else the first one's + 1.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

const ALICE = { userName: 'Alice Martin', userId: 'a91f03c2', nameAsked: true, mcpEnabled: true };
const BOB = { userName: 'Bob Durand', userId: '5d0e77b1', nameAsked: true, mcpEnabled: true };

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
  dir = fs.mkdtempSync(path.join(process.env.TEMP ?? process.env.TMPDIR ?? '/tmp', 'team-handoff-'));
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

/** The project « demo » made the team's, then its Kanban the team's: a new one (`join` null) or a teammate's. */
async function teamBoard(page: Page, join: string | null): Promise<string> {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  await page.getByRole('region', { name: 'ACME' }).getByRole('button', { name: 'Réglages de l’équipe' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages de l’équipe' });
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
  const journal = (await id.getAttribute('data-team-board-id'))!;
  await page.keyboard.press('Escape');
  await page.keyboard.press('Escape');
  return journal;
}

async function board(page: Page) {
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByRole('region', { name: 'À faire' })).toBeVisible();
}

/** The id of the project of `repo` in the data folder `data`, once saved. */
async function projectId(data: string, repo: string): Promise<string> {
  let id = '';
  await expect
    .poll(
      () => {
        try {
          const state = JSON.parse(fs.readFileSync(path.join(data, 'state.json'), 'utf8')) as { projects?: { id: string; path: string }[] };
          const same = (a: string) => path.resolve(a).toLowerCase() === path.resolve(repo).toLowerCase();
          id = state.projects?.find((p) => same(p.path))?.id ?? '';
        } catch {
          id = '';
        }
        return id;
      },
      { timeout: 30_000 },
    )
    .not.toBe('');
  return id;
}

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

/** The window (or `what`) in both themes, named with the size of the window. */
async function shoot(page: Page, name: string, what?: Locator) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const size = page.viewportSize();
  const tag = size ? `${size.width}x${size.height}` : 'window';
  for (const t of ['dark', 'light'] as const) {
    await theme(page, t);
    await page.waitForTimeout(250);
    await (what ?? page).screenshot({ path: path.join(SHOTS, `${name}-${tag}-${t}.png`), animations: 'disabled' });
  }
  await theme(page, 'dark');
}

const git = (cwd: string, ...args: string[]) =>
  execFileSync('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', ...args], { cwd, encoding: 'utf8' }).trim();

test('an agent passed to a teammate with its uncommitted work goes on on his machine, with the handover file first', async ({ app }) => {
  test.setTimeout(480_000);
  const alice = app.page;

  // Their host: a bare repository; Alice's repository pushes there, Bob clones it.
  const origin = path.join(app.root, 'origin.git');
  git(app.root, 'clone', '-q', '--bare', app.repo, origin);
  git(app.repo, 'remote', 'add', 'origin', origin);
  git(app.repo, 'fetch', '-q', 'origin');
  const bobRepo = path.join(app.root, 'bob-repo');
  git(app.root, 'clone', '-q', origin, bobRepo);

  // Alice makes the team and invites; Bob, a second app at the same time, joins.
  const created = await ipc<{ team: { id: string } }>(alice, 'team_create', {
    server: host,
    token,
    name: 'ACME',
    escrowPublic: null,
    escrowHolder: null,
  });
  const team = created.team.id;
  const invite = await ipc<{ url: string }>(alice, 'team_invite', { team, role: 'member', hours: 72 });
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  const settings = JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
  fs.writeFileSync(path.join(bobData, 'settings.json'), JSON.stringify({ ...settings, ...BOB }));
  const bob = await app.second(bobData);
  const size = alice.viewportSize();
  if (size) await bob.setViewportSize(size);
  await ipc(bob, 'team_join', { link: invite.url, name: 'ACME' });
  await expect.poll(async () => (await ipc<{ joined: boolean }[]>(bob, 'team_list'))[0]?.joined, { timeout: 60_000 }).toBe(true);

  await addProject(alice, app.repo, { firstAgent: false, name: 'demo' });
  await addProject(bob, bobRepo, { firstAgent: false, name: 'demo' });
  const journal = await teamBoard(alice, null);
  await teamBoard(bob, journal);
  await board(alice);
  await board(bob);

  // Alice's ticket, its turn long ([lent:600]): she launches it, then leaves work uncommitted in its worktree.
  const pa = await projectId(app.data, app.repo);
  await ipc(alice, 'ticket_create', {
    projectId: pa,
    draft: { title: 'La passation', description: '[lent:600]', criteria: [], maxLoops: 3, teamLocal: false },
  });
  const aliceCard = alice.locator('.card', { hasText: 'La passation' });
  await expect(aliceCard).toBeVisible({ timeout: 15_000 });
  await expect(bob.locator('.card', { hasText: 'La passation' })).toBeVisible({ timeout: 15_000 });
  await aliceCard.getByRole('button', { name: 'Lancer' }).click({ timeout: 15_000 });
  await expect(aliceCard.locator('[data-team-holder="here"]')).toBeVisible({ timeout: 30_000 });
  const aliceTree = path.join(app.repo, '.claude', 'worktrees', 'dem-1');
  await expect.poll(() => fs.existsSync(aliceTree), { timeout: 60_000 }).toBe(true);
  fs.writeFileSync(path.join(aliceTree, 'passation.txt'), 'le travail en cours\n');

  // « Passer à un collègue… » from the card's menu, once the card has its agent: the window says what goes before the
  // gesture.
  // The card is drawn again as its agent works: the menu is opened and its item clicked in one try, again if it closed.
  const dialog = alice.getByRole('dialog', { name: 'Passer DEM-1' });
  await expect(async () => {
    await alice.keyboard.press('Escape');
    await aliceCard.click({ button: 'right' });
    await alice.getByRole('menuitem', { name: 'Passer à un collègue…' }).click({ timeout: 2000 });
    await expect(dialog).toBeVisible({ timeout: 3000 });
  }).toPass({ timeout: 60_000 });
  await expect(dialog.getByTestId('team-handoff-push')).toContainText('La branche ticket/dem-1 sera poussée sur origin.');
  await expect(dialog.getByTestId('team-handoff-push')).toContainText('Rien n’est poussé avant « Passer ».');
  await expect(dialog.getByTestId('team-handoff-wip')).toContainText('1 fichier non commité');
  await expect(dialog.getByTestId('team-handoff-target').filter({ hasText: 'Bob Durand' })).toBeVisible();
  await expect(dialog.getByTestId('team-handoff-patch')).toBeChecked();
  await shoot(alice, 'equipe-passer-agent', dialog);
  // Nothing pushed before the gesture.
  expect(git(origin, 'branch', '--list', 'ticket/dem-1')).toBe('');
  await dialog.getByTestId('team-handoff-submit').click();
  await expect(dialog.getByTestId('team-handoff-done')).toContainText('Passé à Bob Durand', { timeout: 60_000 });
  await alice.keyboard.press('Escape');
  expect(git(origin, 'branch', '--list', 'ticket/dem-1')).toContain('ticket/dem-1');
  await expect(aliceCard.locator('[data-team-handoff="outgoing"]')).toContainText('Passé à Bob Durand');
  await expect(aliceCard.locator('[data-team-holder]')).toHaveCount(0);
  // The work stays here too: nothing lost.
  expect(fs.readFileSync(path.join(aliceTree, 'passation.txt'), 'utf8')).toBe('le travail en cours\n');

  // Bob: the proposal on the card and in his box « À toi ».
  const bobCard = bob.locator('.card', { hasText: 'La passation' });
  const offer = bobCard.locator('[data-team-handoff="incoming"]');
  await expect(offer).toContainText('Alice Martin te passe DEM-1', { timeout: 30_000 });
  await expect(bobCard.locator('[data-team-holder]')).toHaveCount(0);
  await shoot(bob, 'equipe-prendre-agent');
  const views = await ipc<{ views: { key: string; source: string; by: string }[] }>(bob, 'inbox_views', { projectId: null });
  expect(views.views.find((v) => v.source === 'handoff')).toMatchObject({ by: 'Alice Martin' });

  // « Prendre »: the branch and the work in his worktree, his agent's first message the handover file.
  await offer.getByRole('button', { name: 'Prendre' }).click();
  await expect(bobCard.locator('[data-team-holder="here"]')).toBeVisible({ timeout: 60_000 });
  const bobTree = path.join(bobRepo, '.claude', 'worktrees', 'dem-1');
  await expect.poll(() => fs.existsSync(path.join(bobTree, 'passation.txt')), { timeout: 60_000 }).toBe(true);
  // As a checkout writes it on this machine (its git may turn the line ends to CRLF).
  expect(fs.readFileSync(path.join(bobTree, 'passation.txt'), 'utf8').replace(/\r\n/g, '\n')).toBe('le travail en cours\n');
  expect(git(bobTree, 'branch', '--show-current')).toBe('ticket/dem-1');
  // Published, as on the machine that pushed it: it follows its remote.
  expect(git(bobTree, 'rev-parse', '--abbrev-ref', '@{upstream}')).toBe('origin/ticket/dem-1');
  await expect(aliceCard.locator('[data-team-holder="held"]')).toContainText('Pris par Bob Durand', { timeout: 30_000 });
  await expect(aliceCard.locator('[data-team-handoff]')).toHaveCount(0);
  await bobCard.click();
  await expect(bob.getByTestId('handover')).toContainText('Passé par Alice Martin · suite sur Claude Code', { timeout: 30_000 });
  await expect(bob.getByRole('button', { name: /Dossier de passation/ })).toBeVisible();
  await bob.getByRole('button', { name: /Dossier de passation/ }).click();
  await expect(bob.locator('.file-text')).toContainText('Passation : suite d’un travail que Alice Martin t’a passé');
  await shoot(bob, 'equipe-agent-repris');

  for (const p of [alice, bob]) {
    await p.evaluate(async () => {
      const ipc = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      const list = (await ipc.invoke('team_list')) as { id: string }[];
      for (const t of list) await ipc.invoke('team_forget', { team: t.id }).catch(() => {});
    });
  }
});
