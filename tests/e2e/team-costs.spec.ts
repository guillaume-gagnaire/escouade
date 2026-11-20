// « Équipe › Coûts » (TS-16) with two clients on the real app, against a real `escouade-team-server` started by the test.
// Alice (an administrator) and Bob (a member) each run a turn of the fake `claude` (0,05 $ each) on their project of the
// team. Their totals go once an hour (every half second here), sealed to the administrators' key: Alice sees the totals
// of the project, both devices summed, in the table; Bob sees himself, and what went of him, exactly. The team's file
// gives the project a budget of 0,08 $ and « le pilote auto s'arrête »: before Bob launches a ticket, the window tells
// him the project spent 125 % of its budget, its autopilot is stopped, and Alice's box « À toi » holds the warning. The
// server keeps neither the model nor a ticket's key.
//
// The team is made and joined by the backend's commands (the screens of TS-3 are `team.spec.ts`'s), the docs (and the
// journal of the project) put on the server by `team_docs_choose`. The server's port: the worktree's range
// (`ESCOUADE_PORT_BASE` + 6, as `team-memory.spec.ts`: the specs run one at a time) when the run has one, else one the
// system gives. ESCOUADE_E2E_SHOTS=<folder> keeps the screenshots of the table, light and dark.

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
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'team-costs-'));
  const out = execFileSync(SERVER, ['init', '--data', dir], { encoding: 'utf8' });
  token = /^\s{4}(\S+)\s*$/m.exec(out)![1];
  host = `127.0.0.1:${await freePort()}`;
  await startServer();
});

test.afterAll(async () => {
  await stopServer();
  if (dir) fs.rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
});

// The team read every 1,5 s and the costs every second: often enough for the test, not so often the server refuses (429).
test.use({ appEnv: { ESCOUADE_TEAMS_EVERY_MS: '1500', ESCOUADE_TEAM_COSTS_EVERY_MS: '1000' } });
// The box « À toi » rests on the MCP server: Alice's is on, to hold the warning.
test.use({ preStart: { run: () => ({ mcpEnabled: true }) } });

const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SIZE = process.env.ESCOUADE_E2E_VIEWPORT ?? '1440x900';

/** Screenshots of the window in both themes, at the run's size. */
async function shots(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const t of ['light', 'dark'] as const) {
    await page.evaluate((t) => {
      document.documentElement.dataset.theme = t;
      document.documentElement.style.colorScheme = t;
    }, t);
    await page.waitForTimeout(200);
    await page.screenshot({ path: path.join(SHOTS, `${name}-${SIZE}-${t}.png`), animations: 'disabled' });
  }
  await page.evaluate(() => {
    delete document.documentElement.dataset.theme;
    document.documentElement.style.colorScheme = '';
  });
}

/** « Ouvrir les équipes… » of Réglages › Avancé, then « Coûts » of the team. */
async function openCosts(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  await page.getByRole('dialog', { name: 'Équipes' }).getByRole('button', { name: 'Coûts', exact: true }).click();
  await expect(page.getByTestId('team-costs')).toBeVisible({ timeout: 15_000 });
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

interface Costs {
  admin: boolean;
  records: { cost: number; mine: boolean; model: string }[];
  sent: unknown;
  budgets: { pct: number; text: string }[];
}

const spent = (c: Costs) => Math.round(c.records.reduce((s, r) => s + r.cost, 0) * 100) / 100;

const BUDGET = `version: 1
enforced:
  costs:
    budgets:
      projects:
        paiements: 0.08
    stopAutopilot: true
`;

test('two clients’ totals reach the administrator by project, and a budget gone beyond is told before a start', async ({ app }) => {
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
  // Bob's has no MCP server of its own (it would take Alice's port).
  fs.writeFileSync(path.join(bobData, 'settings.json'), JSON.stringify({ ...settings, mcpEnabled: false, userName: 'Bob' }));
  const bob = await app.second(bobData);
  await ipc(bob, 'team_join', { link: invite.url, name: 'ACME' });
  await expect.poll(async () => (await ipc<{ joined: boolean }[]>(bob, 'team_list'))[0]?.joined, { timeout: 60_000 }).toBe(true);

  // Their projects, the team's, on one journal.
  await addProject(alice, app.repo, { name: 'paiements', firstAgent: false });
  await addProject(bob, bobRepo, { name: 'paiements', firstAgent: false });
  const a = await projectId(app.data, app.repo);
  const b = await projectId(bobData, bobRepo);
  await ipc(alice, 'project_set_team', { project: a, team });
  await ipc(bob, 'project_set_team', { project: b, team });
  const view = await ipc<{ journal: string }>(alice, 'team_docs_choose', { id: a, server: true, journal: null });
  await ipc(bob, 'team_docs_choose', { id: b, server: true, journal: view.journal });

  // A turn each, on the fake `claude`.
  await ipc(alice, 'create_agent', { projectId: a, model: null, engine: null, isolated: false, base: null });
  await send(alice, 'Un tour pour les coûts');
  await ipc(bob, 'create_agent', { projectId: b, model: null, engine: null, isolated: false, base: null });
  await send(bob, 'Un tour pour les coûts');

  // The administrator sees both devices' totals, by project, in « Équipe › Coûts ».
  await expect
    .poll(
      async () => {
        const c = await ipc<Costs>(alice, 'team_costs', { team });
        return [spent(c), c.records.some((r) => r.mine), c.records.some((r) => !r.mine)];
      },
      { timeout: 90_000 },
    )
    .toEqual([0.1, true, true]);
  // Bob, a member, sees himself alone, and what went of him.
  const mine = await ipc<Costs>(bob, 'team_costs', { team });
  expect(mine.admin).toBe(false);
  expect(spent(mine)).toBe(0.05);
  expect(mine.records.every((r) => r.mine)).toBe(true);
  expect(mine.sent).not.toBeNull();

  // A budget of 0,08 $ for the project, and « le pilote auto s'arrête », signed by Alice.
  // Signed again if the server asks to wait a little (429).
  await expect
    .poll(
      () =>
        ipc(alice, 'team_settings_propose', { team, yaml: BUDGET, note: 'Budget du mois', by: 'Alice' }).then(
          () => 'signed',
          (e: unknown) => String(e),
        ),
      { timeout: 60_000, intervals: [2000] },
    )
    .toBe('signed');
  await expect
    .poll(async () => (await ipc<{ text: string; stops: boolean } | null>(bob, 'team_cost_notice', { project: b }))?.stops ?? false, {
      timeout: 60_000,
    })
    .toBe(true);

  // Alice's table: the project by its name, both devices summed, the equivalent in API marked, the budget gone beyond.
  await openCosts(alice);
  const table = alice.getByTestId('team-costs');
  const row = table.getByRole('row', { name: /^paiements/ });
  await expect(row).toContainText('0,10', { timeout: 15_000 });
  await expect(table).toContainText('Les totaux de chaque poste de l’équipe');
  await expect(table).toContainText('Le projet paiements a dépensé 125 % de son budget du mois', { timeout: 30_000 });
  await shots(alice, 'team-costs-admin');
  await alice.keyboard.press('Escape');

  // Bob's: himself, the month of the team, and what goes up of him.
  await openCosts(bob);
  const own = bob.getByTestId('team-costs');
  await expect(own).toContainText('Tes propres totaux');
  await expect(own).toContainText('L’équipe ACME voit les totaux par projet : les tiens partent sans ton nom.');
  await own.getByRole('button', { name: 'Voir ce qui est parti' }).click();
  await expect(bob.getByTestId('team-costs-sent')).toContainText('"hour"');
  await shots(bob, 'team-costs-member');
  await bob.keyboard.press('Escape');

  // Before Bob launches a ticket, the window tells him.
  await bob.getByRole('button', { name: /^Kanban/ }).click();
  await expect(bob.getByTestId('budget-stop')).toContainText(
    'Pilote auto arrêté : Le projet paiements a dépensé 125 % de son budget du mois',
    {
      timeout: 30_000,
    },
  );
  await bob.getByRole('button', { name: 'Nouveau ticket' }).click();
  const form = bob.getByRole('dialog', { name: 'Nouveau ticket' });
  await form.getByRole('textbox', { name: 'Titre du ticket' }).fill('Le budget avant tout');
  await form.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(form).toBeHidden();
  const card = bob.getByRole('region', { name: 'À faire' }).getByRole('button', { name: /Le budget avant tout/ });
  await expect(card).toBeVisible({ timeout: 15_000 });
  await card.getByRole('button', { name: 'Lancer' }).click();
  const ask = bob.getByRole('dialog', { name: /Lancer .* \?/ });
  await expect(ask).toContainText(
    'Le projet paiements a dépensé 125 % de son budget du mois : le pilote auto de ce projet est arrêté par l’équipe ACME. Le lancer quand même ?',
  );
  // The tip of « À découvrir » on the autopilot is not in the picture.
  await bob
    .getByRole('button', { name: 'Compris' })
    .click({ timeout: 2000 })
    .catch(() => {});
  await shots(bob, 'team-costs-launch');
  await ask.getByRole('button', { name: 'Annuler' }).click();

  // Alice's box « À toi » holds the warning.
  await expect
    .poll(
      async () => (await ipc<{ asks: { title: string }[] }>(alice, 'inbox_list', { projectId: null })).asks.map((x) => x.title).join(' | '),
      {
        timeout: 60_000,
      },
    )
    .toContain('Le projet paiements a dépensé 125 % de son budget du mois');

  // The server reads neither the model nor a ticket's key.
  const disk = fs
    .readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => fs.readFileSync(path.join(e.parentPath, e.name)).toString('latin1'))
    .join('');
  for (const clear of ['claude-sonnet-test', 'paiements', 'Un tour pour les co']) expect(disk).not.toContain(clear);

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
