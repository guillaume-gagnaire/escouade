// « La mémoire d'équipe » (TS-10) with two clients on the real app, against a real `escouade-team-server` started by the
// test. Alice's project and Bob's are the team's, their docs (and so their memory) on the team server. Alice's agent (the
// fake `claude`) notes a memory by Escouade's write_memory: it is born proposed, her agents read it marked as data, and
// Bob's machine does not see it. Alice keeps it in Réglages › Projets › Équipe › Mémoire partagée: Bob's machine reads it
// then, with its provenance (« par l'agent … »). Alice, an administrator, puts what her device wrote in quarantine after
// its preview: the memory leaves both machines. The server keeps neither its name nor its text.
//
// The team is made and joined by the backend's commands (the screens of TS-3 are `team.spec.ts`'s), the docs put on the
// server by `team_docs_choose` (the screen of TS-9 is `team-docs.spec.ts`'s). The server's port: the worktree's range
// (`ESCOUADE_PORT_BASE` + 6, as `team-docs.spec.ts`: the specs run one at a time) when the run has one, else one the
// system gives.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, send, test } from './fixture';
import { letIn } from './team-admit';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);
const NAME = 'tests-avant-commit';

let server: ChildProcess | null = null;
let host = '';
let token = '';
let dir = '';

async function freePort(): Promise<number> {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  if (base) return base + 6;
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
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'team-memory-'));
  const out = execFileSync(SERVER, ['init', '--data', dir], { encoding: 'utf8' });
  token = /^\s{4}(\S+)\s*$/m.exec(out)![1];
  host = `127.0.0.1:${await freePort()}`;
  await startServer();
});

test.afterAll(async () => {
  await stopServer();
  if (dir) fs.rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
});

test.use({ appEnv: { ESCOUADE_TEAMS_EVERY_MS: '800' } });
// « Claude et les agents peuvent piloter Escouade » : the fake `claude` reaches write_memory by Escouade's MCP server.
test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

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

/** The names of the memories the agents of `project` read on this machine. */
const memories = async (page: Page, project: string) =>
  (await ipc<{ name: string }[]>(page, 'memory_list', { projectId: project })).map((m) => m.name);

/** Nothing of this machine waits for the server. */
async function sent(page: Page, project: string) {
  await expect
    .poll(async () => (await ipc<{ pending: number }>(page, 'team_docs_status', { id: project })).pending, { timeout: 60_000 })
    .toBe(0);
}

/** Réglages › Projets › Équipe, at « Mémoire partagée ». */
async function openMemory(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Équipe');
  const box = dialog.getByTestId('team-memory');
  await box.scrollIntoViewIfNeeded();
  return { dialog, box };
}

test('an agent’s memory is read by the other machine only once kept, then a quarantine takes it off both', async ({ app }) => {
  test.setTimeout(400_000);
  const alice = app.page;

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
  // Bob's has no agent: no MCP server of its own (it would take Alice's port).
  fs.writeFileSync(path.join(bobData, 'settings.json'), JSON.stringify({ ...settings, mcpEnabled: false, userName: 'Bob' }));
  const bob = await app.second(bobData);
  const asked = await ipc<{ joinWords: string[] }>(bob, 'team_join', { link: invite.url, name: 'ACME' });
  // Alice compares the six words of the request with Bob's screen, and lets him in (SR2-08).
  await letIn(alice, asked.joinWords);
  await expect.poll(async () => (await ipc<{ joined: boolean }[]>(bob, 'team_list'))[0]?.joined, { timeout: 60_000 }).toBe(true);

  // Their projects, the team's; the docs of Alice's, and its memory, on the team server; Bob follows its journal.
  await addProject(alice, app.repo, { name: 'paiements', firstAgent: false });
  await addProject(bob, bobRepo, { name: 'paiements', firstAgent: false });
  const a = await projectId(app.data, app.repo);
  const b = await projectId(bobData, bobRepo);
  await ipc(alice, 'project_set_team', { project: a, team });
  await ipc(bob, 'project_set_team', { project: b, team });
  const view = await ipc<{ journal: string; onServer: boolean }>(alice, 'team_docs_choose', { id: a, server: true, journal: null });
  expect(view.onServer).toBe(true);
  await ipc(bob, 'team_docs_choose', { id: b, server: true, journal: view.journal });

  // Alice's agent notes a memory: proposed, read by her agents alone.
  await ipc(alice, 'create_agent', { projectId: a, model: null, engine: null, isolated: false, base: null });
  await send(alice, `Note ce que tu as appris [souvenir:${NAME}]`);
  await expect(alice.getByText('Souvenir noté : proposed.')).toBeVisible({ timeout: 60_000 });
  expect(await memories(alice, a)).toEqual([NAME]);
  await sent(alice, a);
  let { dialog, box } = await openMemory(alice);
  const proposal = box.getByTestId('team-memory-proposal');
  await expect(proposal).toHaveAttribute('data-name', NAME);
  await expect(proposal).toContainText('par l’agent');
  // Written before any third party's data: proposed, not said after one.
  await expect(proposal).not.toContainText('après des données de tiers');
  await expect(box.getByTestId('team-memory-item')).toHaveAttribute('data-standing', 'proposed');
  await shots(alice, 'team-memory-proposed');

  // Bob's machine does not see it, after several readings of the team.
  await bob.waitForTimeout(4000);
  expect(await memories(bob, b)).toEqual([]);

  // Alice keeps it: the team's; Bob's machine reads it then, with its provenance.
  await box.getByTestId('team-memory-keep').click();
  await expect(box.getByTestId('team-memory-proposal')).toHaveCount(0, { timeout: 30_000 });
  await expect(box.getByTestId('team-memory-item')).toHaveAttribute('data-standing', 'accepted');
  await sent(alice, a);
  await expect.poll(() => memories(bob, b), { timeout: 60_000 }).toEqual([NAME]);
  const bobs = await openMemory(bob);
  const item = bobs.box.getByTestId('team-memory-item');
  await expect(item).toHaveAttribute('data-name', NAME);
  await expect(item).toContainText('par l’agent');
  await shots(bob, 'team-memory-read');
  await closeSettings(bobs.dialog);

  // Alice, an administrator, puts in quarantine what her device wrote, after its preview: gone from both.
  await box.getByTestId('team-memory-q-author').selectOption({ index: 1 });
  await box.getByTestId('team-memory-q-preview').click();
  const list = box.getByTestId('team-memory-q-list');
  await expect(list).toContainText('1 souvenir part');
  await expect(list).toContainText(NAME);
  await shots(alice, 'team-memory-quarantine');
  await box.getByTestId('team-memory-q-apply').click();
  await expect(box.getByTestId('team-memory-rule')).toBeVisible({ timeout: 30_000 });
  await expect(box.getByTestId('team-memory-item')).toHaveCount(0);
  await closeSettings(dialog);
  expect(await memories(alice, a)).toEqual([]);
  await sent(alice, a);
  await expect.poll(() => memories(bob, b), { timeout: 60_000 }).toEqual([]);

  // The server reads neither its name nor its text.
  const disk = fs
    .readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => fs.readFileSync(path.join(e.parentPath, e.name)).toString('latin1'))
    .join('');
  for (const clear of ['tests-avant', 'commiter', 'relire le diff']) expect(disk).not.toContain(clear);

  // The keychain keeps nothing of the test: each client forgets the team.
  const forget = (p: Page) =>
    p.evaluate(async () => {
      const ipc = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      const list = (await ipc.invoke('team_list')) as { id: string }[];
      for (const t of list) await ipc.invoke('team_forget', { team: t.id });
    });
  await forget(bob);
  await forget(alice);
});
