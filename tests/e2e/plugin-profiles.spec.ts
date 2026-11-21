import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

// A plugin's profiles and the page Plugins of a project (ESC-530, issues #38 and #39, docs/PLUGINS-CONTRACT-STUDY.md §2.2,
// §2.3 and §3.1; renders `b-plugin`, `b-profil`, `b-projet` of design/plugins-contract-2-3 on release/2.3). A plugin of test
// declares a kind of profile and settings of each project: two profiles are made on its page, each project chooses its own
// in its settings, the plugin reads on a fact of each project the secret of the profile that project chose (its journal
// names the profile, never the value), and a profile deleted leaves its project with none. The project's `.escouade.yml`
// proposes a profile by its name and sets a route: a band, never a choice. Captures with `ESCOUADE_E2E_SHOTS`.

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const ID = 'dev.test.tunnel';
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;
const SECRET_A = 'mot-de-passe-client-a-41';
const SECRET_B = 'mot-de-passe-client-b-42';

const title = (fr: string, en: string) => ({ fr, en });

/** The plugin of test: the fake plugin, told to read, on each fact, the secret `password` of its project's profile. */
function tunnelPlugin(dir: string) {
  fs.mkdirSync(dir, { recursive: true });
  fs.copyFileSync(path.join(ROOT, 'tests', 'fixtures', 'plugins', 'fake', 'plugin.mjs'), path.join(dir, 'plugin.mjs'));
  fs.writeFileSync(path.join(dir, 'args.json'), JSON.stringify(['--fact-profile', 'profile', '--fact-secret', 'password']));
  fs.writeFileSync(
    path.join(dir, 'escouade-plugin.json'),
    JSON.stringify({
      id: ID,
      name: 'Tunnel',
      version: '1.4.0',
      author: { name: 'escouade-tests' },
      license: 'MIT',
      description: 'Un tunnel OpenVPN par projet.',
      escouade: { api: '1', min: '2.1.0' },
      run: { js: 'plugin.mjs' },
      activation: ['event:ticket.created'],
      permissions: { events: ['ticket.created'], secrets: ['profile:vpn'] },
      contributes: {
        profiles: [
          {
            id: 'vpn',
            title: title('Profils VPN', 'VPN profiles'),
            description: title(
              'Un profil réunit un serveur, ses identifiants et sa configuration. Chaque projet choisit le sien dans ses réglages.',
              'A profile brings a server, its credentials and its configuration together. Each project chooses its own in its settings.',
            ),
            nameField: 'name',
            fields: [
              { key: 'name', type: 'string', title: title('Nom du profil', 'Profile name') },
              { key: 'server', type: 'string', title: title('Serveur', 'Server') },
              { key: 'proto', type: 'select', title: title('Protocole', 'Protocol'), options: ['UDP', 'TCP'], default: 'UDP' },
              { key: 'port', type: 'number', title: 'Port' },
              { key: 'login', type: 'string', title: title('Identifiant', 'Login') },
              { key: 'password', secret: true, title: title('Mot de passe', 'Password') },
            ],
          },
        ],
        settings: [
          {
            key: 'profile',
            type: 'profile',
            of: 'vpn',
            scope: 'project',
            title: title('Profil VPN', 'VPN profile'),
            description: title('Celui que prennent les agents de ce projet.', 'The one this project’s agents take.'),
          },
          {
            key: 'routes',
            type: 'string',
            scope: 'project',
            title: title('Routes par le tunnel', 'Routes through the tunnel'),
            description: title(
              'Les réseaux que les agents de ce projet joignent par le tunnel ; le reste sort directement.',
              'The networks this project’s agents reach through the tunnel; the rest goes out directly.',
            ),
          },
          {
            key: 'autoStart',
            type: 'boolean',
            title: title('Tunnel au lancement d’un agent', 'Tunnel when an agent starts'),
            description: title(
              'Monte le tunnel du projet quand son premier agent démarre.',
              'Opens the project’s tunnel when its first agent starts.',
            ),
            default: true,
          },
          {
            key: 'fallback',
            type: 'profile',
            of: 'vpn',
            title: title('Profil par défaut', 'Default profile'),
            description: title('Pour un projet qui n’en choisit pas.', 'For a project that chooses none.'),
          },
        ],
      },
    }),
  );
}

/** A git repository with one commit, named `name`, in the test's folder; `file`: its `.escouade.yml`. */
function repoNamed(root: string, name: string, file?: string): string {
  const dir = path.join(root, name);
  fs.mkdirSync(dir);
  const git = (...args: string[]) => execFileSync('git', args, { cwd: dir, stdio: 'pipe' });
  git('init', '-q', '-b', 'main');
  git('config', 'user.email', 'e2e@test');
  git('config', 'user.name', 'e2e');
  git('config', 'core.autocrlf', 'false');
  fs.writeFileSync(path.join(dir, 'README.md'), `# ${name}\n`);
  if (file) fs.writeFileSync(path.join(dir, '.escouade.yml'), file);
  git('add', '-A');
  git('commit', '-qm', 'init');
  return dir;
}

const FILE = `# Les réglages partagés de l'équipe
version: 1
plugins:
  ${ID}:
    profile: client b # proposé, jamais choisi
    routes: 10.20.0.0/16
`;

const invokeOn =
  (page: Page) =>
  <T>(cmd: string, args: Record<string, unknown> = {}) =>
    page.evaluate(
      ([cmd, args]) =>
        (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__
          .invoke(cmd as string, args)
          // A refusal is an object: said whole, not as « Object ».
          .catch((e: unknown) => Promise.reject(new Error(`${cmd}: ${JSON.stringify(e)}`))),
      [cmd, args] as const,
    ) as Promise<T>;

const theme = (page: Page, t: 'dark' | 'light') =>
  page.evaluate((t) => {
    document.documentElement.dataset.theme = t;
    document.documentElement.style.colorScheme = t;
  }, t);

async function shoot(page: Page, name: string) {
  if (!SHOTS) return;
  fs.mkdirSync(SHOTS, { recursive: true });
  const width = await page.evaluate(() => window.innerWidth);
  // The toasts of the gestures before are not what the capture shows.
  await page.addStyleTag({ content: '.toasts { visibility: hidden !important; }' });
  for (const t of ['light', 'dark'] as const) {
    await theme(page, t);
    await page.mouse.move(2, 400);
    await page.screenshot({ path: path.join(SHOTS, `profiles-${name}-${width}-${t}.png`) });
  }
  await theme(page, 'dark');
}

/** Nothing of `panel` goes out of it sideways. */
async function holds(panel: Locator) {
  const over = await panel.evaluate((el) => {
    const box = el.getBoundingClientRect();
    return [...el.querySelectorAll<HTMLElement>('*')]
      .filter((e) => e.getClientRects().length)
      .filter((e) => {
        const r = e.getBoundingClientRect();
        return r.right > box.right + 1 || r.left < box.left - 1;
      })
      .map((e) => `${e.tagName}.${e.className}: ${e.textContent?.slice(0, 40)}`);
  });
  expect(over).toEqual([]);
  expect(await panel.page().evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
}

/** The ids of the projects by their names, as the app saved them. */
async function projectIds(data: string, names: string[]): Promise<Record<string, string>> {
  let ids: Record<string, string> = {};
  await expect
    .poll(() => {
      const state = JSON.parse(fs.readFileSync(path.join(data, 'state.json'), 'utf8')) as { projects: { id: string; name: string }[] };
      ids = Object.fromEntries(state.projects.map((p) => [p.name, p.id]));
      return names.every((n) => n in ids);
    })
    .toBe(true);
  return ids;
}

async function openPluginSettings(page: Page, installed: string, tab: RegExp) {
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .getByRole('table', { name: installed })
    .getByRole('button', { name: /Tunnel/ })
    .click();
  await page.getByRole('tab', { name: tab }).click();
}

/** Makes a profile from the plugin's page: its window, its fields, its password, « Enregistrer ». */
async function makeProfile(
  page: Page,
  words: Words,
  values: { name: string; server: string; proto: string; port: string; login: string; secret: string },
) {
  await page.getByRole('button', { name: words.add }).click();
  const dialog = page.getByRole('dialog', { name: words.newTitle });
  await dialog.getByLabel(words.name).fill(values.name);
  await dialog.getByLabel(words.server).fill(values.server);
  await dialog.getByLabel(words.proto).selectOption(values.proto);
  await dialog.getByLabel('Port').fill(values.port);
  await dialog.getByLabel(words.login).fill(values.login);
  await dialog.getByPlaceholder(words.paste).fill(values.secret);
  await dialog.getByRole('button', { name: words.save }).click();
  await expect(dialog).toBeHidden();
}

/** Opens the settings on the page Plugins of the project `name` (unfolded in the bar first). */
async function openProjectPlugins(page: Page, name: string) {
  let dialog = page.getByRole('dialog', { name: /^(Réglages|Settings)$/ });
  if (!(await dialog.isVisible())) await page.getByRole('button', { name: /^(Réglages|Settings) \(/ }).click();
  dialog = page.getByRole('dialog', { name: /^(Réglages|Settings)$/ });
  const nav = dialog.getByRole('navigation');
  const row = nav
    .locator('button[aria-expanded]')
    .filter({ has: page.locator('.lbl', { hasText: name }) })
    .first();
  if ((await row.getAttribute('aria-expanded')) !== 'true') await row.click();
  const entry = nav.getByRole('group', { name }).getByRole('button', { name: 'Plugins', exact: true });
  await entry.click();
  await expect(entry).toHaveAttribute('aria-current', 'page');
  return dialog;
}

type Words = typeof FR;
const FR = {
  installed: 'Ajoutés',
  settingsTab: /^Réglages/,
  add: 'Ajouter un profil…',
  newTitle: 'Nouveau profil · Profils VPN',
  name: 'Nom du profil',
  server: 'Serveur',
  proto: 'Protocole',
  login: 'Identifiant',
  paste: 'Colle la valeur',
  save: 'Enregistrer',
  group: 'Profils VPN',
  profile: 'Profil VPN',
};
const EN: Words = {
  installed: 'Added',
  settingsTab: /^Settings/,
  add: 'Add a profile…',
  newTitle: 'New profile · VPN profiles',
  name: 'Profile name',
  server: 'Server',
  proto: 'Protocol',
  login: 'Login',
  paste: 'Paste the value',
  save: 'Save',
  group: 'VPN profiles',
  profile: 'VPN profile',
};

test('two profiles, chosen by two projects: the plugin reads the right secret for each, a profile deleted leaves none', async ({ app }) => {
  test.setTimeout(300_000);
  const { page } = app;
  const invoke = invokeOn(page);
  const atlas = repoNamed(app.root, 'atlas-web', FILE);
  const portail = repoNamed(app.root, 'portail-adherents');
  await addProject(page, atlas, { name: 'atlas-web' });
  await addProject(page, portail, { name: 'portail-adherents' });
  const ids = await projectIds(app.data, ['atlas-web', 'portail-adherents']);
  const src = path.join(app.root, 'tunnel-plugin');
  tunnelPlugin(src);
  const { token, refusal } = await invoke<{ token: string; refusal: string | null }>('plugin_inspect_folder', { path: src });
  expect(refusal, 'profiles are served: no « ne sert pas encore »').toBeNull();
  await invoke('plugin_install', { token });

  // Two profiles, made on the plugin's page.
  await openPluginSettings(page, FR.installed, FR.settingsTab);
  const panel = page.getByRole('tabpanel', { name: 'Réglages' });
  const group = panel.getByRole('region', { name: FR.group });
  await expect(group).toContainText('Aucun profil encore');
  await makeProfile(page, FR, {
    name: 'Client A',
    server: 'vpn.client-a.example',
    proto: 'UDP',
    port: '1194',
    login: 'mdurand-clienta',
    secret: SECRET_A,
  });
  await makeProfile(page, FR, {
    name: 'Client B',
    server: 'gw.client-b.example',
    proto: 'TCP',
    port: '443',
    login: 'mdurand-clientb',
    secret: SECRET_B,
  });
  await makeProfile(page, FR, {
    name: 'Interne',
    server: 'vpn.interne.example',
    proto: 'UDP',
    port: '1194',
    login: 'mdurand',
    secret: 'interne-0003',
  });
  const a = group.getByRole('group', { name: 'Client A' });
  await expect(a).toContainText('vpn.client-a.example · UDP · 1194');
  await expect(a).toContainText('1 secret gardé');
  await expect(a).toContainText('Aucun projet');
  // A setting of each project is not on the plugin's page; the default profile is.
  await expect(panel.getByLabel('Routes par le tunnel')).toHaveCount(0);
  await expect(panel.getByLabel('Profil par défaut')).toBeVisible();

  // Each project chooses its own, in its settings.
  let dialog = await openProjectPlugins(page, 'atlas-web');
  const body = dialog.locator('#settings-panel');
  const band = body.getByRole('status').filter({ hasText: '.escouade.yml propose' });
  await expect(band).toContainText('.escouade.yml propose le profil « client b » pour Tunnel.');
  await expect(band).toContainText('Un dépôt ne choisit pas tes identifiants');
  const tunnel = body.getByRole('region', { name: 'Tunnel' });
  await expect(tunnel).toContainText(ID);
  await expect(tunnel.getByRole('group', { name: FR.profile })).toContainText('Aucun profil : Tunnel n’en a pas pour ce projet.');
  await expect(tunnel.getByRole('group', { name: FR.profile })).toContainText('Gardé sur cette machine, jamais dans .escouade.yml.');
  await tunnel.getByRole('combobox', { name: FR.profile }).selectOption({ label: 'Client A · vpn.client-a.example' });
  const routes = tunnel.getByRole('group', { name: 'Routes par le tunnel' });
  await expect(routes).toContainText('Défini par .escouade.yml');
  await expect(routes.getByLabel('Routes par le tunnel')).toHaveValue('10.20.0.0/16');
  await expect(band).toBeVisible();
  await shoot(page, 'b-projet');
  await holds(body);

  // A route set here goes ahead of the file, then is written into it, that key alone, comments kept.
  await routes.getByLabel('Routes par le tunnel').fill('10.0.0.0/8');
  await routes.getByLabel('Routes par le tunnel').press('Tab');
  await expect(routes).toContainText('Réglage local de cette machine : il passe devant .escouade.yml.');
  await routes.getByRole('button', { name: 'Écrire dans .escouade.yml' }).click();
  await expect(routes).toContainText('Défini par .escouade.yml');
  const written = fs.readFileSync(path.join(atlas, '.escouade.yml'), 'utf8');
  expect(written).toContain("# Les réglages partagés de l'équipe");
  expect(written).toContain('profile: client b # proposé, jamais choisi');
  expect(written).toContain('routes: 10.0.0.0/8');
  expect(written).not.toContain(SECRET_A);

  dialog = await openProjectPlugins(page, 'portail-adherents');
  await expect(dialog.locator('#settings-panel').getByRole('status').filter({ hasText: '.escouade.yml propose' })).toHaveCount(0);
  await dialog
    .locator('#settings-panel')
    .getByRole('combobox', { name: FR.profile })
    .selectOption({ label: 'Client B · gw.client-b.example' });
  await dialog.locator('.s-head .close').click();

  // The plugin's page names the projects that chose each.
  await expect(group.getByRole('group', { name: 'Client A' })).toContainText('atlas-web');
  await expect(group.getByRole('group', { name: 'Client B' })).toContainText('portail-adherents');
  await expect(group.getByRole('group', { name: 'Interne' })).toContainText('Aucun projet');
  await shoot(page, 'b-plugin');
  await holds(panel);
  await group.getByRole('group', { name: 'Client A' }).getByRole('button', { name: 'Modifier' }).click();
  const profile = page.getByRole('dialog', { name: 'Profil « Client A »' });
  await expect(profile).toContainText('Utilisé par atlas-web');
  await expect(profile.getByRole('group', { name: 'Mot de passe' })).toContainText('Gardé dans le trousseau');
  await expect(profile.getByLabel(FR.server)).toHaveValue('vpn.client-a.example');
  await shoot(page, 'b-profil');
  await profile.getByRole('button', { name: 'Annuler' }).click();

  // A fact of each project: the plugin reads the secret of the profile that project chose, its journal says which, never
  // the value.
  for (const name of ['atlas-web', 'portail-adherents'])
    await invoke('ticket_create', {
      projectId: ids[name],
      draft: { title: `Tunnel de ${name}`, description: '', criteria: [], maxLoops: 5, after: [] },
    });
  const journal = () =>
    invoke<{ journal: { text: string }[] }>('plugin_run', { id: ID }).then((r) => r.journal.map((e) => e.text).join('\n'));
  await expect.poll(journal, { timeout: 60_000 }).toContain('was given its secret "password" of profile "Client A"');
  await expect.poll(journal, { timeout: 60_000 }).toContain('was given its secret "password" of profile "Client B"');
  const said = await journal();
  expect(said).not.toContain(SECRET_A);
  expect(said).not.toContain(SECRET_B);
  expect(said).not.toContain('of profile "Interne"');
  // Not the lock the running app holds on its data folder (`instance.lock`, ESC-451: empty, and locked on Windows).
  for (const f of fs.readdirSync(app.data).filter((f) => f !== 'instance.lock')) {
    const p = path.join(app.data, f);
    if (fs.statSync(p).isFile()) expect(fs.readFileSync(p, 'utf8')).not.toContain(SECRET_A);
  }

  // « Supprimer le profil » names the project that took it; its secret is erased, that project has none.
  await group.getByRole('group', { name: 'Client B' }).getByRole('button', { name: 'Modifier' }).click();
  const b = page.getByRole('dialog', { name: 'Profil « Client B »' });
  await b.getByRole('button', { name: 'Supprimer le profil' }).click();
  await expect(b).toContainText('Supprimer « Client B » ? Ses secrets sont effacés. portail-adherents n’aura plus de profil.');
  await b.getByRole('button', { name: 'Supprimer', exact: true }).click();
  await expect(b).toBeHidden();
  await expect(group.getByRole('group', { name: 'Client B' })).toHaveCount(0);
  dialog = await openProjectPlugins(page, 'portail-adherents');
  await expect(dialog.locator('#settings-panel').getByRole('group', { name: FR.profile })).toContainText(
    'Aucun profil : Tunnel n’en a pas pour ce projet.',
  );
  await dialog.locator('.s-head .close').click();

  // The keychain of a sandboxed run outlives it: the other profiles deleted, their secrets with them.
  const views = await invoke<{ id: string; choices: { profiles?: Record<string, { id: string }[]> } }[]>('plugins_list');
  for (const p of views.find((v) => v.id === ID)?.choices.profiles?.vpn ?? [])
    await invoke('plugin_profile_delete', { id: ID, kind: 'vpn', profile: p.id });
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the profiles, their window and the page Plugins of a project speak English', async ({ app }) => {
    test.setTimeout(240_000);
    const { page } = app;
    const invoke = invokeOn(page);
    const atlas = repoNamed(app.root, 'atlas-web', FILE);
    await addProject(page, atlas, { name: 'atlas-web', lang: 'en' });
    const src = path.join(app.root, 'tunnel-plugin');
    tunnelPlugin(src);
    const { token } = await invoke<{ token: string }>('plugin_inspect_folder', { path: src });
    await invoke('plugin_install', { token });
    await openPluginSettings(page, EN.installed, EN.settingsTab);
    const panel = page.getByRole('tabpanel', { name: 'Settings' });
    const group = panel.getByRole('region', { name: EN.group });
    await makeProfile(page, EN, {
      name: 'Client A',
      server: 'vpn.client-a.example',
      proto: 'UDP',
      port: '1194',
      login: 'mdurand-clienta',
      secret: SECRET_A,
    });
    await makeProfile(page, EN, {
      name: 'Client B',
      server: 'gw.client-b.example',
      proto: 'TCP',
      port: '443',
      login: 'mdurand-clientb',
      secret: SECRET_B,
    });
    await expect(group.getByRole('group', { name: 'Client A' })).toContainText('1 secret kept');

    const dialog = await openProjectPlugins(page, 'atlas-web');
    const body = dialog.locator('#settings-panel');
    await expect(body.getByRole('status').filter({ hasText: '.escouade.yml proposes' })).toContainText(
      '.escouade.yml proposes the profile “client b” for Tunnel.',
    );
    await expect(body.getByRole('group', { name: EN.profile })).toContainText('Kept on this machine, never in .escouade.yml.');
    await body.getByRole('combobox', { name: EN.profile }).selectOption({ label: 'Client A · vpn.client-a.example' });
    await expect(body.getByRole('group', { name: 'Routes through the tunnel' })).toContainText('Defined by .escouade.yml');
    await shoot(page, 'b-projet-en');
    await holds(body);
    await dialog.locator('.s-head .close').click();

    await expect(group.getByRole('group', { name: 'Client A' })).toContainText('atlas-web');
    await shoot(page, 'b-plugin-en');
    await group.getByRole('group', { name: 'Client A' }).getByRole('button', { name: 'Edit' }).click();
    const profile = page.getByRole('dialog', { name: 'Profile “Client A”' });
    await expect(profile).toContainText('Used by atlas-web');
    await expect(profile.getByRole('group', { name: 'Password' })).toContainText('Kept in the keychain');
    await shoot(page, 'b-profil-en');
    await profile.getByRole('button', { name: 'Delete the profile' }).click();
    await expect(profile).toContainText('Delete “Client A”? Its secrets are erased. atlas-web will have no profile.');
    await profile.getByRole('button', { name: 'Delete', exact: true }).click();
    await expect(profile).toBeHidden();
    const views = await invoke<{ id: string; choices: { profiles?: Record<string, { id: string }[]> } }[]>('plugins_list');
    for (const p of views.find((v) => v.id === ID)?.choices.profiles?.vpn ?? [])
      await invoke('plugin_profile_delete', { id: ID, kind: 'vpn', profile: p.id });
  });
});
