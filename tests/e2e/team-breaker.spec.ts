// « Le coupe-circuit de l'administrateur » (TS-15) with two clients on the real app, against a real `escouade-team-server`
// started by the test. Alice (the team's administrator) and Bob (a member) each have the team's project; Bob's agent (the
// fake `claude`) ends a turn. Alice, in the team's window, takes the harness Claude Code out of the whole team, confirmed
// with what stops and on how many devices online: Bob's agent stops between two turns and does not start again, and the
// band of Bob's window says who cut, what and why. Alice lifts it: the band goes, and Bob's agent answers again.
//
// The team is made and joined by the backend's commands (the screens of TS-3 are `team.spec.ts`'s). The server's port:
// the worktree's range (`ESCOUADE_PORT_BASE` + 6, as `team-docs.spec.ts`: the specs run one at a time) when the run has
// one, else one the system gives. ESCOUADE_E2E_SHOTS=<folder> keeps the band and the screen in both themes.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, send, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);
const WHY = 'fuite de clé chez le fournisseur X';

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
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'team-breaker-'));
  const out = execFileSync(SERVER, ['init', '--data', dir], { encoding: 'utf8' });
  token = /^\s{4}(\S+)\s*$/m.exec(out)![1];
  host = `127.0.0.1:${await freePort()}`;
  await startServer();
});

test.afterAll(async () => {
  await stopServer();
  if (dir) fs.rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
});

// The watch is what is left when the live feed does not connect: long, so that the live feed is what the test sees.
test.use({ appEnv: { ESCOUADE_TEAMS_EVERY_MS: '20000' } });

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** Screenshots of the window in both themes (ESCOUADE_E2E_SHOTS=<folder>), named by the size of the run. */
async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const size = process.env.ESCOUADE_E2E_VIEWPORT ?? '1440x900';
  for (const t of ['light', 'dark'] as const) {
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
      document.documentElement.style.colorScheme = t;
    }, t);
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${size}-${t}.png`) });
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

/** Réglages › Avancé › « Ouvrir les équipes… ». */
async function openTeams(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  const teams = page.getByRole('dialog', { name: 'Équipes' });
  await expect(teams).toBeVisible();
  return teams;
}

test('an administrator cuts a harness: the other client’s agent stops between two turns and does not start again, its band says it, the lifting gives it back', async ({
  app,
}) => {
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
  fs.writeFileSync(path.join(bobData, 'settings.json'), JSON.stringify({ ...settings, mcpEnabled: false, userName: 'Bob' }));
  const bob = await app.second(bobData);
  await ipc(bob, 'team_join', { link: invite.url, name: 'ACME' });
  await expect.poll(async () => (await ipc<{ joined: boolean }[]>(bob, 'team_list'))[0]?.joined, { timeout: 60_000 }).toBe(true);
  await ipc(alice, 'team_sync', { team });

  // Their projects, the team's. Bob's agent ends a turn.
  await addProject(alice, app.repo, { name: 'paiements', firstAgent: false });
  await addProject(bob, bobRepo, { name: 'paiements', firstAgent: false });
  const a = await projectId(app.data, app.repo);
  const b = await projectId(bobData, bobRepo);
  await ipc(alice, 'project_set_team', { project: a, team });
  await ipc(bob, 'project_set_team', { project: b, team });
  const agent = await ipc<{ id: string }>(bob, 'create_agent', { projectId: b, model: null, engine: null, isolated: false, base: null });
  await send(bob, 'Bonjour');
  await expect(bob.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible({ timeout: 60_000 });
  await expect(bob.getByTestId('breaker-band')).toHaveCount(0);

  // Bob's device online for Alice (its live feed).
  await expect.poll(() => ipc<number>(alice, 'team_breaker_online', { team }), { timeout: 60_000 }).toBeGreaterThanOrEqual(1);

  // Alice cuts Claude Code for the whole team, from the team's window.
  const teams = await openTeams(alice);
  await teams.getByRole('button', { name: 'Coupe-circuit' }).click();
  const screen = alice.getByTestId('team-breakers');
  const form = screen.getByRole('region', { name: 'Poser un coupe-circuit' });
  await expect(form).toBeVisible();
  await form.getByRole('combobox', { name: 'Un harness' }).selectOption('claude');
  await form.getByRole('textbox', { name: 'Raison' }).fill(WHY);
  await form.getByRole('textbox', { name: 'Signé par' }).fill('Claire Martin');
  await shots(alice, 'team-breaker-form');
  await form.getByRole('button', { name: 'Continuer' }).click();
  const confirm = screen.getByTestId('breaker-confirm');
  await expect(confirm).toContainText('Plus aucun agent des projets de l’équipe ne démarre sur Claude Code');
  await expect(confirm).toContainText('Sur 1 poste en ligne, dans la seconde');
  await shots(alice, 'team-breaker-confirm');
  await confirm.getByRole('button', { name: 'Couper' }).click();
  const active = screen.getByTestId('breaker-active');
  await expect(active).toContainText(`Arrêt demandé par Claire Martin : ${WHY}`);
  await shots(alice, 'team-breaker-active');

  // Bob: his agent stops (it is between two turns), said in its conversation; the band says who cut, what and why.
  const band = bob.getByTestId('breaker-band');
  await expect(band).toContainText(`Arrêt demandé par Claire Martin : ${WHY}`, { timeout: 30_000 });
  await expect(band).toContainText('Retiré : le harness Claude Code · toute l’équipe ACME');
  await expect(bob.getByText(/Le harness « claude » est retiré par le coupe-circuit de Claire Martin/).first()).toBeVisible({
    timeout: 30_000,
  });
  await shots(bob, 'team-breaker-band');
  // It does not start again: its next message is refused, with the reason.
  const refused = await ipc(bob, 'send_message', { id: agent.id, text: 'Encore', attachments: [], files: [] }).then(
    () => '',
    (e: unknown) => String(e),
  );
  expect(refused).toContain('Claire Martin');
  expect(refused).toContain(WHY);
  // The band opens the team's breakers, where a member lifts nothing.
  await band.getByRole('button', { name: 'Voir' }).click();
  const bobScreen = bob.getByTestId('team-breakers');
  await expect(bobScreen.getByTestId('breaker-active')).toContainText(WHY);
  await expect(bobScreen.getByRole('button', { name: 'Lever' })).toHaveCount(0);
  await shots(bob, 'team-breaker-member');
  await bob.keyboard.press('Escape');

  // Alice lifts it: Bob's band goes, and his agent answers again.
  await active.getByRole('button', { name: 'Lever' }).click();
  await expect(screen.getByText('Aucun coupe-circuit en cours.')).toBeVisible({ timeout: 30_000 });
  await expect(band).toHaveCount(0, { timeout: 30_000 });
  await alice.keyboard.press('Escape');
  await send(bob, 'Rebonjour');
  await expect(bob.getByText('Bonjour, tu as dit : Rebonjour')).toBeVisible({ timeout: 60_000 });

  // The server reads neither what was cut nor why.
  const disk = fs
    .readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => fs.readFileSync(path.join(e.parentPath, e.name)).toString('latin1'))
    .join('');
  for (const clear of ['fournisseur', 'Claire Martin']) expect(disk).not.toContain(clear);

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
