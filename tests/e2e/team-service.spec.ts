// A machine of the team (OE-1) on the real app, two clients at the same time against a real `escouade-team-server`
// started by the test: Alice makes the team and, in « Machines de l'équipe », enrolls a machine by a short code; the
// second client types the code as `escouade-server team pair` does (`team_pair_join`), shows its six words, and Alice
// lets it in: it is in under the role of service, ranked apart. Their project becomes the team's Kanban; Alice makes a
// ticket, the machine takes it (« Lancer »), and Alice sees it held by the machine. The machine's try to sign a setting
// of the team is refused. With `ESCOUADE_TEAM_SHOTS=<folder>`, Alice's team card is kept there, dark and light.
//
// Its server's port: the worktree's `ESCOUADE_PORT_BASE` + 7 (as team-board.spec.ts: the specs run one at a time), else
// one the system gives. The second app's DevTools port: `ESCOUADE_E2E_SECOND_CDP_PORT`, else the first one's + 1.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, test } from './fixture';
import { letIn } from './team-admit';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);
const SHOTS = process.env.ESCOUADE_TEAM_SHOTS;

const ALICE = { userName: 'Alice Martin', userId: 'a91f03c2', nameAsked: true };
const MACHINE = { userName: 'Machine de nuit', userId: '7c1e4b20', nameAsked: true };

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
  dir = fs.mkdtempSync(path.join(process.env.TEMP ?? process.env.TMPDIR ?? '/tmp', 'team-service-'));
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

/** The window of the teams, from Réglages › Avancé › « Ouvrir les équipes… ». */
async function openTeams(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  return page.getByRole('dialog', { name: 'Équipes' });
}

/** « Réglages de l’équipe » of the team ACME. */
async function teamSettings(page: Page) {
  await openTeams(page);
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

test('an administrator enrolls a machine of the team, it takes a ticket, and its try to sign a setting is refused', async ({ app }) => {
  test.setTimeout(400_000);
  const alice = app.page;
  await alice.setViewportSize({ width: 1024, height: 728 });
  const created = await ipc<{ team: { id: string } }>(alice, 'team_create', {
    server: host,
    token,
    name: 'ACME',
    escrowPublic: null,
    escrowHolder: null,
  });
  const team = created.team.id;

  // The machine: a second app at the same time, on a clone of the repository.
  const machineRepo = path.join(app.root, 'machine-repo');
  execFileSync('git', ['clone', '-q', app.repo, machineRepo]);
  const machineData = path.join(app.root, 'machine-data');
  fs.mkdirSync(machineData);
  const settings = JSON.parse(fs.readFileSync(path.join(app.data, 'settings.json'), 'utf8'));
  fs.writeFileSync(path.join(machineData, 'settings.json'), JSON.stringify({ ...settings, ...MACHINE }));
  const machine = await app.second(machineData);
  await machine.setViewportSize({ width: 1024, height: 728 });

  // Alice, in « Machines de l’équipe »: « Inscrire une machine de l’équipe » shows the code and the command.
  await openTeams(alice);
  const card = alice.getByRole('region', { name: 'ACME' });
  const machines = card.getByRole('group', { name: 'Machines de l’équipe' });
  await expect(machines).toContainText('Aucune machine dans cette équipe.');
  await machines.getByRole('button', { name: 'Inscrire une machine de l’équipe' }).click();
  const code = (await alice.getByTestId('machine-code').textContent())!.trim();
  expect(code).toMatch(/^[0-9A-Z]{5}-[0-9A-Z]{5}$/);
  await expect(alice.getByTestId('machine-command')).toHaveText(`escouade-server team pair ${host} ${code}`);
  // The machine types it (as `escouade-server team pair` does); the PAKE runs while Alice's window steps it.
  const asked = await ipc<{ joinWords: string[]; joined: boolean }>(machine, 'team_pair_join', { server: host, code, name: 'ACME' });
  expect(asked.joined).toBe(false);
  await expect(alice.getByText(/La machine a reçu son invitation/)).toBeVisible({ timeout: 30_000 });
  await letIn(alice, asked.joinWords);
  await expect
    .poll(async () => (await ipc<{ joined: boolean; role: string }[]>(machine, 'team_list'))[0], { timeout: 60_000 })
    .toMatchObject({ joined: true, role: 'service' });
  await alice.keyboard.press('Escape');

  // Ranked apart on Alice's card: one person, one machine.
  await expect
    .poll(() => ipc<{ members: number; machines: unknown[] }>(alice, 'team_sync', { team }), { timeout: 30_000 })
    .toMatchObject({ members: 1, machines: [expect.objectContaining({ devices: [expect.any(String)] })] });
  await openTeams(alice);
  const ranked = alice.getByRole('region', { name: 'ACME' }).getByRole('group', { name: 'Machines de l’équipe' });
  await expect(ranked.getByTestId('team-machine')).toHaveCount(1);
  await expect(alice.getByRole('region', { name: 'ACME' })).toContainText('1 membre');
  await expect(ranked).toContainText('Elle ne signe jamais');
  await shoot(alice, 'equipe-machines', alice.getByRole('region', { name: 'ACME' }));
  await alice.keyboard.press('Escape');
  await alice.keyboard.press('Escape');

  // The machine's try to sign a setting of the team: refused.
  const refused = await ipc(machine, 'team_settings_propose', {
    team,
    yaml: 'version: 1\nenforced:\n  telemetry:\n    enabled: false\n',
    note: 'La machine essaie',
    by: 'Machine de nuit',
  }).then(
    () => 'signed',
    (e) => String(e),
  );
  expect(refused).toContain('Seul un administrateur de l’équipe peut le faire.');

  // Their project, the team's Kanban: Alice shares hers, the machine joins it.
  await addProject(alice, app.repo, { firstAgent: false, name: 'demo' });
  await addProject(machine, machineRepo, { firstAgent: false, name: 'demo' });
  const journal = await teamBoard(alice, null);
  await teamBoard(machine, journal);
  await board(alice);
  await board(machine);

  // Alice makes a ticket; the machine reads it and takes it.
  await alice.getByRole('button', { name: 'Nouveau ticket' }).click();
  const form = alice.getByRole('dialog', { name: 'Nouveau ticket' });
  await form.getByRole('textbox', { name: 'Titre du ticket' }).fill('Le rapport de nuit');
  await form.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(form).toBeHidden();
  const machineCard = machine.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /DEM-1 Le rapport de nuit/ });
  await expect(machineCard).toBeVisible({ timeout: 15_000 });
  await machineCard.getByRole('button', { name: 'Lancer' }).click({ timeout: 15_000 });
  await machine.waitForSelector('[data-team-holder="here"]', { timeout: 30_000 });
  const aliceCard = alice.locator('.card', { hasText: 'Le rapport de nuit' });
  await expect(aliceCard.locator('[data-team-holder="held"]')).toContainText('Pris par Machine de nuit', { timeout: 15_000 });
  await expect(aliceCard.getByRole('button', { name: 'Lancer' })).toHaveCount(0);

  for (const p of [alice, machine]) {
    await p.evaluate(async () => {
      const ipc = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
      const list = (await ipc.invoke('team_list')) as { id: string }[];
      for (const t of list) await ipc.invoke('team_forget', { team: t.id }).catch(() => {});
    });
  }
});
