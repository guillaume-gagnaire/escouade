// The team's signed `.escouade.yml` (TS-13) with two clients on the real app, against a real `escouade-team-server`
// started by the test: Alice, the administrator, writes a version (the modes allowed, an exclusion of the proxy),
// reads its change and signs it. Bob, a member on a data folder of his own, sees the band (« L'équipe ACME a changé 5
// réglages · Claire Martin »), opens it, makes his project one of the team's: its Kanban's mode greyed with who
// imposes it; the proxy's exclusion waits for his consent, said with what it allows, then applies once accepted.
// A plugin's setting (ESC-532): the team imposes the routes of a plugin of test and recommends its MTU; the port its
// author keeps off the files is refused, said with why in Alice's reading. Bob's project shows the routes greyed with
// « Imposé par l'équipe ACME », the MTU with its provenance, and the plugin reads the imposed routes on a fact.
//
// The team is made and joined by the backend's commands: the screens of TS-3 are `team.spec.ts`'s. The server's port:
// the worktree's range (`ESCOUADE_PORT_BASE` + 8) when the run has one, else one the system gives.

import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { addProject, expect, openSettingsPage, test } from './fixture';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const SERVER = path.join(ROOT, 'src-tauri', 'target', 'debug', `escouade-team-server${process.platform === 'win32' ? '.exe' : ''}`);

let server: ChildProcess | null = null;
let host = '';
let token = '';
let dir = '';

async function freePort(): Promise<number> {
  const base = Number(process.env.ESCOUADE_PORT_BASE ?? 0);
  if (base) return base + 8;
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
  dir = fs.mkdtempSync(path.join(process.env.TEMP ?? process.env.TMPDIR ?? '/tmp', 'team-settings-'));
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
});

test.use({ appEnv: { ESCOUADE_TEAMS_EVERY_MS: '800' } });

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

/** « Ouvrir les équipes… » of Réglages › Avancé. */
async function openTeams(page: Page) {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Avancé');
  await settings.getByRole('button', { name: 'Ouvrir les équipes…' }).click();
  await expect(page.getByRole('dialog', { name: 'Équipes' })).toBeVisible();
}

const PLUGIN = 'dev.test.tunnel';

/** The plugin of test (the fake plugin): settings of each project, the routes said in its journal on each fact. */
async function installTunnel(page: Page, root: string) {
  const dir = path.join(root, `tunnel-${Date.now()}`);
  fs.mkdirSync(dir, { recursive: true });
  fs.copyFileSync(path.join(ROOT, 'tests', 'fixtures', 'plugins', 'fake', 'plugin.mjs'), path.join(dir, 'plugin.mjs'));
  fs.writeFileSync(path.join(dir, 'args.json'), JSON.stringify(['--fact-setting', 'routes']));
  fs.writeFileSync(
    path.join(dir, 'escouade-plugin.json'),
    JSON.stringify({
      id: PLUGIN,
      name: 'Tunnel',
      version: '1.4.0',
      author: { name: 'escouade-tests' },
      license: 'MIT',
      description: 'Un tunnel par projet.',
      escouade: { api: '1', min: '2.1.0' },
      run: { js: 'plugin.mjs' },
      activation: ['event:ticket.created'],
      permissions: { events: ['ticket.created'] },
      contributes: {
        settings: [
          { key: 'routes', type: 'string', scope: 'project', title: 'Routes par le tunnel' },
          { key: 'mtu', type: 'number', scope: 'project', title: 'MTU' },
          { key: 'port', type: 'number', scope: 'project', title: 'Port', repo: false },
        ],
      },
    }),
  );
  const { token } = await ipc<{ token: string }>(page, 'plugin_inspect_folder', { path: dir });
  await ipc(page, 'plugin_install', { token });
}

const YAML = `version: 1
defaults:
  plugins:
    ${PLUGIN}:
      mtu: 1400
enforced:
  permissions:
    modes: [plan, default]
  proxy.exclusions: ["*.acme.fr"]
  plugins:
    ${PLUGIN}:
      routes: 172.16.0.0/12
      port: 4444
reasons:
  permissions.modes: "Pas de mode auto sur les projets du client."
`;

test('an administrator imposes a setting, the other client sees it greyed with who imposes it, a sensitive key waits for its consent', async ({
  app,
}) => {
  test.setTimeout(300_000);
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
  await installTunnel(page, app.root);
  const invite = await ipc<{ url: string }>(page, 'team_invite', { team, role: 'member', hours: 72 });

  // Bob, a data folder of his own, with a project; he asks to join.
  const bobData = path.join(app.root, 'bob-data');
  fs.mkdirSync(bobData);
  fs.writeFileSync(path.join(bobData, 'settings.json'), fs.readFileSync(path.join(app.data, 'settings.json')));
  page = await app.restartAs(bobData);
  await addProject(page, app.repo, { name: 'site-acme' });
  await installTunnel(page, app.root);
  await ipc(page, 'team_join', { link: invite.url, name: 'ACME' });

  // Alice, online, lets him in; then she writes a version, reads its change, and signs it.
  page = await app.restartAs(app.data);
  await expect.poll(async () => (await ipc<{ members: number }[]>(page, 'team_list'))[0]?.members, { timeout: 60_000 }).toBe(2);
  await openTeams(page);
  await page.getByRole('region', { name: 'ACME' }).getByRole('button', { name: 'Réglages de l’équipe' }).click();
  const alice = page.getByRole('dialog', { name: 'Réglages de l’équipe' });
  await expect(alice.getByText('L’équipe n’a encore aucun réglage.').first()).toBeVisible();
  const propose = alice.getByRole('region', { name: 'Proposer une version' });
  await propose.getByRole('textbox', { name: 'Proposer une version' }).fill(YAML);
  await propose.getByLabel('Tu signes sous le nom').fill('Claire Martin');
  await propose.getByLabel(/Ce que tu changes/).fill('Les modes du contrat.');
  await propose.getByRole('button', { name: 'Relire le changement' }).click();
  const diff = propose.getByTestId('team-settings-diff');
  await expect(diff).toContainText('permissions.modes');
  await expect(diff).toContainText('proxy.exclusions');
  await expect(diff).toContainText(`plugins.${PLUGIN}.routes`);
  // The port its author keeps off the files: refused, with why, as the other keys that will not apply.
  await expect(
    propose.getByText(
      `enforced.plugins.${PLUGIN}.port refusé : son auteur le garde hors des fichiers partagés (repo: false). Il ne s’appliquera pas.`,
    ),
  ).toBeVisible();
  await expect(propose.getByText(/Une signature suffit/)).toBeVisible();
  await propose.getByRole('button', { name: 'Signer et publier' }).click();
  await expect(alice.getByTestId('team-settings-force')).toContainText('Version 1 · signée par Claire Martin', { timeout: 30_000 });
  await expect(alice.getByTestId('team-settings-history')).toContainText('En vigueur');
  // On her own device too, the proxy's exclusion waits for her consent.
  await expect(alice.getByTestId('team-consent')).toContainText('proxy.exclusions');

  // Bob: the band says the change, who signed it.
  page = await app.restartAs(bobData);
  const band = page.getByTestId('team-settings-band');
  await expect(band).toContainText('L’équipe ACME a changé 5 réglages · Claire Martin', { timeout: 60_000 });
  await expect(band).toContainText('ACME demande ton accord pour 1 réglage');
  await band.getByRole('button', { name: 'Voir' }).click();
  const bob = page.getByRole('dialog', { name: 'Réglages de l’équipe' });
  await expect(bob.getByTestId('team-settings-force')).toContainText('Version 1 · signée par Claire Martin');
  await expect(bob.getByText('Les modes du contrat.').first()).toBeVisible();
  // The sensitive key: waiting, with what it allows; nothing of it applies yet.
  const ask = bob.getByTestId('team-consent');
  await expect(ask).toContainText('proxy.exclusions');
  await expect(ask).toContainText('Ces adresses ne passeront plus par le proxy');
  // A member proposes nothing.
  await expect(bob.getByRole('region', { name: 'Proposer une version' })).toHaveCount(0);
  // His project becomes one of the team's.
  await bob.getByRole('checkbox', { name: 'site-acme' }).check();
  await expect(bob.getByRole('checkbox', { name: 'site-acme' })).toBeChecked();
  // He accepts the exclusion on his device: it applies, his consent can be withdrawn.
  await ask.getByRole('button', { name: 'Accepter sur ce poste' }).click();
  await expect(bob.getByTestId('team-consent')).toHaveCount(0);
  const imposed = bob.getByRole('region', { name: 'Ce que l’équipe impose' });
  await expect(imposed).toContainText('proxy.exclusions');
  await expect(imposed.getByRole('button', { name: 'Retirer mon accord' })).toBeVisible();
  await page.keyboard.press('Escape');

  // The Kanban of the team's project: the modes the team does not allow greyed, with who imposes it.
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await openSettingsPage(settings, 'Kanban');
  const mode = settings.getByRole('combobox', { name: 'Mode' });
  await expect(mode.locator('option', { hasText: /non permis par l’équipe ACME/ }).first()).toBeAttached();
  await expect(mode.locator('option[value="auto"]')).toBeDisabled();
  await expect(mode.locator('option[value="plan"]')).toBeEnabled();

  // The page Plugins of the team's project: the routes imposed, greyed, with who imposes them; the MTU recommended, with
  // where it comes from; the port refused, with why.
  const nav = settings.getByRole('navigation');
  const entry = nav.getByRole('group', { name: 'site-acme' }).getByRole('button', { name: 'Plugins', exact: true });
  if (!(await entry.isVisible())) {
    await nav
      .locator('button[aria-expanded]')
      .filter({ has: page.locator('.lbl', { hasText: 'site-acme' }) })
      .first()
      .click();
  }
  await entry.click();
  const tunnel = settings.locator('#settings-panel').getByRole('region', { name: 'Tunnel' });
  const routes = tunnel.getByRole('group', { name: 'Routes par le tunnel' });
  await expect(routes).toContainText('Imposé par l’équipe ACME');
  await expect(routes.getByLabel('Routes par le tunnel')).toHaveValue('172.16.0.0/12');
  await expect(routes.getByLabel('Routes par le tunnel')).toBeDisabled();
  await expect(routes.getByRole('button', { name: 'Écrire dans .escouade.yml' })).toHaveCount(0);
  const mtu = tunnel.getByRole('group', { name: 'MTU' });
  await expect(mtu).toContainText('Recommandé par l’équipe ACME');
  await expect(mtu.getByLabel('MTU')).toHaveValue('1400');
  await expect(tunnel).toContainText('L’équipe ACME règle port : refusé, son auteur le garde hors des fichiers partagés.');
  // This machine's value never goes ahead of it.
  let projectId = '';
  await expect
    .poll(() => {
      const state = JSON.parse(fs.readFileSync(path.join(bobData, 'state.json'), 'utf8')) as { projects: { id: string; name: string }[] };
      projectId = state.projects.find((p) => p.name === 'site-acme')?.id ?? '';
      return projectId;
    })
    .not.toBe('');
  const refused = await page.evaluate(
    ([id, project]) =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__
        .invoke('plugin_project_set', { id, project, key: 'routes', value: '10.0.0.0/8', clear: false })
        .then(
          () => 'accepted',
          (e: unknown) => String(e),
        ),
    [PLUGIN, projectId] as const,
  );
  expect(refused).toContain('imposé par l’équipe ACME');
  await page.keyboard.press('Escape');

  // The plugin reads the imposed routes in the team's project, on a fact of it.
  await ipc(page, 'ticket_create', {
    projectId,
    draft: { title: 'Tunnel du site', description: '', criteria: [], maxLoops: 5, after: [] },
  });
  const journal = () =>
    ipc<{ journal: { text: string }[] }>(page, 'plugin_run', { id: PLUGIN }).then((r) => r.journal.map((e) => e.text).join('\n'));
  await expect.poll(journal, { timeout: 60_000 }).toContain('setting routes = "172.16.0.0/12"');

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
