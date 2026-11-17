// « Les docs d'un projet d'équipe » (TS-9) with two clients on the real app, against a real `escouade-team-server`
// started by the test. Alice puts her project's docs on the team server from its settings (« Docs du projet
// d'équipe »), writes a page and proposes a decision; Bob, on a data folder of his own, follows the project's journal
// from the same settings and reads both in his Docs tab, from his machine's copy. The server keeps neither the page's
// title nor its text. Then both write the same line of the page while the server is away: back, Bob's Docs show the
// conflict (« Garder la mienne », « Prendre la sienne », « Donner à l'agent rédacteur »), and « Garder la mienne »
// settles it.
//
// The team is made and joined by the backend's commands (the screens of TS-3 are `team.spec.ts`'s), the page and the
// decision written by the window's commands. The server's port: the worktree's range (`ESCOUADE_PORT_BASE` + 6) when the
// run has one, else one the system gives.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, closeSettings, expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);

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
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'team-docs-'));
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

/** « Docs du projet d'équipe » in the project's « Git et worktrees ». */
async function openPlace(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const dialog = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(dialog, 'Git et worktrees');
  const place = dialog.locator('[data-team-docs-place]');
  await place.scrollIntoViewIfNeeded();
  return { dialog, place };
}

async function openDocs(page: Page) {
  await page.getByRole('group', { name: 'Vue du projet' }).getByRole('button', { name: 'Docs' }).click();
}

const PAGE = 'paiement/echeances';

test('a page and a decision written on one client are read on the other, then a page written on both sides shows its conflict', async ({
  app,
}) => {
  test.setTimeout(400_000);
  let page = app.page;

  // Alice makes the team, invites Bob, and makes her project the team's.
  const created = await ipc<{ team: { id: string } }>(page, 'team_create', {
    server: host,
    token,
    name: 'ACME',
    escrowPublic: null,
    escrowHolder: null,
  });
  const team = created.team.id;
  const invite = await ipc<{ url: string }>(page, 'team_invite', { team, role: 'member', hours: 72 });
  await addProject(page, app.repo, { name: 'paiements' });
  const alice = await projectId(app.data, app.repo);
  await ipc(page, 'project_set_team', { project: alice, team });
  // The attachment wrote the key `team` of `.escouade.yml` (TS-7): Alice commits it, as the toast asks.
  await expect.poll(() => fs.existsSync(path.join(app.repo, '.escouade.yml'))).toBe(true);
  execFileSync('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', 'add', '.escouade.yml'], { cwd: app.repo });
  execFileSync('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', 'commit', '-q', '-m', 'team'], { cwd: app.repo });

  // Bob, a data folder of his own, with the project; he asks to join, Alice lets him in.
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  fs.writeFileSync(path.join(bobData, 'settings.json'), fs.readFileSync(path.join(app.data, 'settings.json')));
  page = await app.restartAs(bobData);
  await addProject(page, app.repo, { name: 'paiements' });
  const bob = await projectId(bobData, app.repo);
  await ipc(page, 'team_join', { link: invite.url, name: 'ACME' });
  page = await app.restartAs(app.data);
  await expect.poll(async () => (await ipc<{ members: number }[]>(page, 'team_list'))[0]?.members, { timeout: 60_000 }).toBe(2);

  // Alice: « Sur le serveur d'équipe », in the project's settings. Its journal is shown.
  let { dialog, place } = await openPlace(page);
  await expect(place).toHaveAttribute('data-team-docs-place', 'repo');
  await expect(place.getByText(/Passer sur le serveur importe la spec/)).toBeVisible();
  await place.getByRole('button', { name: 'Sur le serveur d’équipe' }).click();
  await expect(place).toHaveAttribute('data-team-docs-place', 'server', { timeout: 30_000 });
  const journal = (await place.locator('[data-journal]').textContent())!.trim();
  expect(journal).toMatch(/^[0-9a-f]{32}$/);
  await expect(place.getByText(/Revenir au dépôt y écrit les docs du serveur/)).toBeVisible();
  await shots(page, 'team-docs-place');
  await closeSettings(dialog);

  // She writes a page and proposes a decision: they leave encrypted.
  await ipc(page, 'docs_spec_write', { id: alice, path: PAGE, title: 'Échéances secrètes', body: 'Trois échéances au plus.\n' });
  await ipc(page, 'docs_decision_propose', {
    id: alice,
    reference: null,
    decision: { title: 'Payer en trois fois', decision: 'Trois échéances au plus.' },
  });
  await expect
    .poll(async () => (await ipc<{ pending: number; online: boolean | null }>(page, 'team_docs_status', { id: alice })).pending, {
      timeout: 60_000,
    })
    .toBe(0);
  await openDocs(page);
  await expect(page.locator('[data-sync="synced"]')).toContainText('Docs sur le serveur d’équipe');
  // The server reads neither the title nor the text nor the decision.
  const disk = fs
    .readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => fs.readFileSync(path.join(e.parentPath, e.name)).toString('latin1'))
    .join('');
  for (const clear of ['secr', 'Trois', 'paiement', 'echeances', 'Payer']) expect(disk).not.toContain(clear);

  // Bob follows the project's journal from his settings, and reads both from his copy.
  page = await app.restartAs(bobData);
  await ipc(page, 'project_set_team', { project: bob, team });
  ({ dialog, place } = await openPlace(page));
  await place.getByRole('textbox').fill(journal);
  await place.getByRole('button', { name: 'Suivre' }).click();
  await expect(place).toHaveAttribute('data-team-docs-place', 'server', { timeout: 60_000 });
  await expect(place.locator('[data-journal]')).toHaveText(journal);
  await closeSettings(dialog);
  await openDocs(page);
  await expect(page.getByText('Échéances secrètes').first()).toBeVisible({ timeout: 30_000 });
  const read = await ipc<{ body: string }>(page, 'docs_spec_page', { id: bob, path: PAGE });
  expect(read.body).toContain('Trois échéances au plus.');
  const decisions = await ipc<{ id: string; state: string; title: string }[]>(page, 'docs_decisions', { id: bob });
  expect(decisions.map((d) => [d.title, d.state])).toEqual([['Payer en trois fois', 'proposed']]);
  await shots(page, 'team-docs-read');

  // The server away: Bob, then Alice, write the same line of the page.
  await stopServer();
  await ipc(page, 'docs_spec_write', { id: bob, path: PAGE, body: 'Quatre échéances (Bob).\n' });
  page = await app.restartAs(app.data);
  await ipc(page, 'docs_spec_write', { id: alice, path: PAGE, body: 'Quatre échéances (Alice).\n' });
  await openDocs(page);
  await expect(page.locator('[data-sync="offline"]')).toBeVisible({ timeout: 30_000 });
  await shots(page, 'team-docs-offline');
  // Back: Alice's leaves first.
  await startServer();
  await expect
    .poll(async () => (await ipc<{ pending: number }>(page, 'team_docs_status', { id: alice })).pending, { timeout: 60_000 })
    .toBe(0);

  // Bob back too: his Docs show the conflict, his version kept meanwhile.
  page = await app.restartAs(bobData);
  await openDocs(page);
  const band = page.locator(`[data-conflict="${PAGE}"]`);
  await expect(band).toContainText('La page « Échéances secrètes » a été écrite ici et par un collègue en même temps', { timeout: 60_000 });
  await band.getByRole('button', { name: 'Comparer' }).click();
  const sides = page.locator(`[data-sides="${PAGE}"]`);
  await expect(sides).toContainText('Quatre échéances (Bob).');
  await expect(sides).toContainText('Quatre échéances (Alice).');
  await shots(page, 'team-docs-conflict');
  await expect(band.getByRole('button', { name: 'Prendre la sienne' })).toBeVisible();
  await expect(band.getByRole('button', { name: 'Donner à l’agent rédacteur' })).toBeVisible();
  await band.getByRole('button', { name: 'Garder la mienne' }).click();
  await expect(band).toHaveCount(0, { timeout: 30_000 });
  expect((await ipc<{ body: string }>(page, 'docs_spec_page', { id: bob, path: PAGE })).body).toContain('Quatre échéances (Bob).');

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
