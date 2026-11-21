import fs from 'node:fs';
import path from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';

// The page of a plugin's settings, issue #40 (ESC-526, docs/PLUGINS-CONTRACT-STUDY.md §4): a plugin of test declares groups
// (one folded), a secret with its description, a secret on several lines that imports a file. The groups come in order,
// settings and secrets mixed; the folded one opens by the keyboard; « Importer un fichier… » reads a real file of the disk
// (handed to the input as the system's dialog does) and fills the numbered field; kept, the secret is never read back, it
// says its lines and size; the file's path and name are nowhere in the app's data, its log nor the plugin's journal; the
// plugin reads the secret by `secrets/get`. Captures in both themes with `ESCOUADE_E2E_SHOTS`; nothing overflows at 1024 × 728.

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const ID = 'dev.test.tunnel';
const SHOTS = process.env.ESCOUADE_E2E_SHOTS;

/** An OpenVPN configuration of `lines` lines, under the 1 280 characters the keychain of Windows takes at most. */
const ovpn = (lines: number) =>
  [
    'client',
    'dev tun',
    'proto udp',
    'remote vpn.acme.example 1194',
    'resolv-retry infinite',
    'nobind',
    'persist-key',
    'persist-tun',
    'remote-cert-tls server',
    'cipher AES-256-GCM',
    'verb 3',
    ...Array.from({ length: lines }, (_, i) => `# ligne de commentaire ${i + 1}`),
  ].join('\n') + '\n';

/** The plugin of test: the fake plugin, told to ask for its secret `ovpn` on each fact and to write what came (masked). */
function tunnelPlugin(dir: string) {
  fs.mkdirSync(dir, { recursive: true });
  fs.copyFileSync(path.join(ROOT, 'tests', 'fixtures', 'plugins', 'fake', 'plugin.mjs'), path.join(dir, 'plugin.mjs'));
  fs.writeFileSync(path.join(dir, 'args.json'), JSON.stringify(['--leak', '--fact-secret', 'ovpn']));
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
      permissions: { events: ['ticket.created'], secrets: ['password', 'ovpn'] },
      contributes: {
        groups: [
          {
            id: 'connection',
            title: { fr: 'Connexion', en: 'Connection' },
            description: {
              fr: 'Pour tous les projets, sauf ceux qui donnent les leurs.',
              en: 'For every project, but those that give their own.',
            },
            fields: ['server', 'proto', 'autoStart'],
          },
          { id: 'credentials', title: { fr: 'Identifiants', en: 'Credentials' }, fields: ['login', 'password', 'ovpn'] },
          { id: 'advanced', title: { fr: 'Avancé', en: 'Advanced' }, folded: true, fields: ['mtu', 'verbosity', 'keepalive'] },
        ],
        settings: [
          {
            key: 'server',
            type: 'string',
            title: { fr: 'Serveur', en: 'Server' },
            description: { fr: 'Le nom ou l’adresse du serveur OpenVPN.', en: 'The name or address of the OpenVPN server.' },
          },
          { key: 'proto', type: 'select', title: { fr: 'Protocole', en: 'Protocol' }, options: ['udp', 'tcp'], default: 'udp' },
          {
            key: 'autoStart',
            type: 'boolean',
            title: { fr: 'Tunnel au lancement d’un agent', en: 'Tunnel when an agent starts' },
            description: {
              fr: 'Monte le tunnel du projet quand son premier agent démarre.',
              en: 'Opens the project’s tunnel when its first agent starts.',
            },
            default: true,
          },
          { key: 'login', type: 'string', title: { fr: 'Identifiant', en: 'Login' } },
          { key: 'mtu', type: 'number', title: 'MTU', default: 1500 },
          { key: 'verbosity', type: 'select', title: { fr: 'Verbosité', en: 'Verbosity' }, options: ['1', '3', '5'], default: '3' },
          { key: 'keepalive', type: 'string', title: 'Keepalive', default: '10 60' },
        ],
        secrets: [
          { key: 'password', title: { fr: 'Mot de passe', en: 'Password' } },
          {
            key: 'ovpn',
            title: { fr: 'Configuration OpenVPN', en: 'OpenVPN configuration' },
            description: {
              fr: 'Le fichier .ovpn de ton fournisseur, certificats compris.',
              en: 'Your provider’s .ovpn file, certificates included.',
            },
            multiline: true,
            import: ['.ovpn', '.conf'],
          },
        ],
      },
    }),
  );
}

const invokeOn =
  (page: Page) =>
  <T>(cmd: string, args: Record<string, unknown> = {}) =>
    page.evaluate(
      ([cmd, args]) =>
        (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(
          cmd as string,
          args,
        ),
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
  for (const t of ['light', 'dark'] as const) {
    await theme(page, t);
    await page.mouse.move(2, 400);
    await page.screenshot({ path: path.join(SHOTS, `groups-${name}-${width}-${t}.png`) });
  }
  await theme(page, 'dark');
}

/** Hands a real file of the disk to the field's file input, as the system's dialog does (see attachments.spec.ts). */
async function chooseFile(page: Page, key: string, file: string) {
  const cdp = await page.context().newCDPSession(page);
  try {
    const { root } = await cdp.send('DOM.getDocument', { depth: -1 });
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector: `input.import-file[data-field="${key}"]` });
    expect(nodeId, `no file input for ${key}`).toBeGreaterThan(0);
    await cdp.send('DOM.setFileInputFiles', { files: [file], nodeId });
  } finally {
    await cdp.detach();
  }
}

/** What a kept secret on several lines says: « Gardé · 31 lignes · 0,7 Ko », « Kept · 15 lines · 0.3 KB ». */
function kept(text: string, locale: 'fr-FR' | 'en-US') {
  const lines = text.replace(/\n$/, '').split('\n').length;
  const kb = Math.max(0.1, Buffer.byteLength(text) / 1024).toLocaleString(locale, { maximumFractionDigits: 1 });
  return locale === 'fr-FR' ? `Gardé · ${lines} lignes · ${kb} Ko` : `Kept · ${lines} lines · ${kb} KB`;
}

/** Every file under `dir`. */
const walk = (dir: string): string[] =>
  fs.existsSync(dir)
    ? fs
        .readdirSync(dir, { withFileTypes: true })
        .flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]))
    : [];

/** Nothing of the panel goes out of it sideways: it holds at 1024 × 728. */
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

/** Forgets the secrets of the plugin of test, in the keychain a sandboxed run shares with the runs before and after it. */
async function forget(invoke: ReturnType<typeof invokeOn>) {
  for (const key of ['password', 'ovpn']) await invoke('plugin_set_secret', { id: ID, key, value: '' });
}

async function openSettings(page: Page, tab: RegExp, installed: string) {
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page
    .getByRole('table', { name: installed })
    .getByRole('button', { name: /Tunnel/ })
    .click();
  await page.getByRole('tab', { name: tab }).click();
}

test('a plugin’s page shows its groups, imports a file into a secret on several lines that is never read back, keeps no path', async ({
  app,
}) => {
  test.setTimeout(240_000);
  const { page } = app;
  const invoke = invokeOn(page);
  await addProject(page, app.repo);
  const src = path.join(app.root, 'tunnel-plugin');
  tunnelPlugin(src);
  const { token } = await invoke<{ token: string }>('plugin_inspect_folder', { path: src });
  await invoke('plugin_install', { token });
  // The keychain of a sandboxed run outlives it: nothing kept by a run before.
  await forget(invoke);

  await openSettings(page, /^Réglages/, 'Ajoutés');
  const panel = page.getByRole('tabpanel', { name: 'Réglages' });

  // The groups, in order; settings and secrets mixed as `fields` says; « Avancé » folded.
  const connection = panel.getByRole('region', { name: 'Connexion' });
  await expect(connection).toContainText('Pour tous les projets, sauf ceux qui donnent les leurs.');
  await expect(connection.getByLabel('Serveur')).toBeVisible();
  const credentials = panel.getByRole('region', { name: 'Identifiants' });
  await expect(credentials.getByLabel('Identifiant')).toBeVisible();
  await expect(credentials.getByRole('group', { name: 'Mot de passe' })).toBeVisible();
  const conf = credentials.getByRole('group', { name: 'Configuration OpenVPN' });
  await expect(conf).toContainText('Le fichier .ovpn de ton fournisseur, certificats compris.');
  const labels = await panel.locator('section > h4, section > .fold .ft, .row .lab b').allTextContents();
  expect(labels).toEqual([
    'Connexion',
    'Serveur',
    'Protocole',
    'Tunnel au lancement d’un agent',
    'Identifiants',
    'Identifiant',
    'Mot de passe',
    'Configuration OpenVPN',
    'Avancé',
  ]);
  const fold = panel.getByRole('button', { name: /^Avancé/ });
  await expect(fold).toHaveAttribute('aria-expanded', 'false');
  await expect(fold).toContainText('3 réglages');
  await connection.getByLabel('Serveur').fill('vpn.acme.example');
  await connection.getByLabel('Serveur').press('Tab');
  await credentials.getByLabel('Identifiant').fill('m.durand');
  await credentials.getByLabel('Identifiant').press('Tab');

  // « Importer un fichier… »: a real file of the disk, filtered on its kinds; its text fills the numbered field.
  const area = conf.getByRole('textbox', { name: 'Configuration OpenVPN' });
  await expect(area).toHaveAttribute('rows', '6');
  await expect(panel.locator('input.import-file[data-field="ovpn"]')).toHaveAttribute('accept', '.ovpn,.conf');
  const folder = path.join(app.root, 'dossier-secret-du-client');
  fs.mkdirSync(folder, { recursive: true });
  const text = ovpn(20);
  expect(text.length).toBeLessThan(1280);
  const file = path.join(folder, 'client-acme-profil.ovpn');
  fs.writeFileSync(file, text);
  await chooseFile(page, 'ovpn', file);
  await expect(area).toHaveValue(text);
  await expect(conf.locator('.gutter')).toContainText('31');
  await area.evaluate((el) => (el.scrollTop = 0));
  await conf.scrollIntoViewIfNeeded();
  await shoot(page, 'typed');
  await holds(panel);

  // Kept: never read back; its lines and size, « Remplacer… », « Oublier ».
  await conf.getByRole('button', { name: 'Garder', exact: true }).click();
  await expect(conf).toContainText(kept(text, 'fr-FR'));
  await expect(conf.getByRole('textbox')).toHaveCount(0);
  await expect(conf.getByRole('button', { name: 'Remplacer…' })).toBeVisible();
  await expect(conf.getByRole('button', { name: 'Oublier' })).toBeVisible();
  await shoot(page, 'kept');
  await holds(panel);

  // « Avancé », opened by the keyboard.
  await fold.focus();
  await page.keyboard.press('Enter');
  await expect(fold).toHaveAttribute('aria-expanded', 'true');
  await expect(panel.getByLabel('MTU')).toBeVisible();
  await page.keyboard.press('Enter');
  await expect(panel.getByLabel('MTU')).toHaveCount(0);

  // A file over 64 KiB, and one that is not text: refused, said, nothing taken.
  await conf.getByRole('button', { name: 'Remplacer…' }).click();
  const big = path.join(folder, 'gros.ovpn');
  fs.writeFileSync(big, 'x'.repeat(64 * 1024 + 1));
  await chooseFile(page, 'ovpn', big);
  await expect(conf.getByRole('alert')).toHaveText('Plus de 64 Kio : rien n’a été pris, rien n’est coupé.');
  const binary = path.join(folder, 'binaire.conf');
  fs.writeFileSync(binary, Buffer.from([0x00, 0xff, 0xfe, 0x41]));
  await chooseFile(page, 'ovpn', binary);
  await expect(conf.getByRole('alert')).toHaveText('Ce fichier n’est pas du texte en UTF-8 : rien n’a été pris.');
  await expect(conf.getByRole('textbox')).toHaveValue('');

  // « Annuler » goes back to the one kept.
  await conf.getByRole('button', { name: 'Annuler' }).click();
  await expect(conf).toContainText(kept(text, 'fr-FR'));

  // A configuration of 6 KB, over the 1 280 characters the keychain of Windows takes in one entry: kept (in pieces, ESC-527).
  await conf.getByRole('button', { name: 'Remplacer…' }).click();
  const long = path.join(folder, 'client-acme-profil-long.ovpn');
  const config = ovpn(230);
  expect(Buffer.byteLength(config)).toBeGreaterThan(6 * 1024);
  fs.writeFileSync(long, config);
  await chooseFile(page, 'ovpn', long);
  await expect(conf.getByRole('textbox')).toHaveValue(config);
  await conf.getByRole('button', { name: 'Garder', exact: true }).click();
  await expect(conf.getByRole('alert')).toHaveCount(0);
  await expect(conf).toContainText(kept(config, 'fr-FR'));

  // The plugin reads it by `secrets/get` on a fact (its journal masks it); nothing of the file's place anywhere.
  const projectId = JSON.parse(fs.readFileSync(path.join(app.data, 'state.json'), 'utf8')).projects[0].id as string;
  await invoke('ticket_create', { projectId, draft: { title: 'Monter le tunnel', description: '', criteria: [], maxLoops: 5, after: [] } });
  const journal = () =>
    invoke<{ journal: { text: string }[] }>('plugin_run', { id: ID }).then((r) => r.journal.map((e) => e.text).join('\n'));
  await expect.poll(journal, { timeout: 60_000 }).toContain('was given its secret "ovpn"');
  await expect.poll(journal).toContain('••••');
  const said = await journal();
  // Written line by line, each line of the configuration longer than 8 characters is masked (study §4.4).
  for (const line of text.split('\n').filter((l) => l.length > 8)) expect(said).not.toContain(line);
  for (const where of [said, ...walk(app.data).map((f) => fs.readFileSync(f).toString('utf8'))]) {
    expect(where).not.toContain('dossier-secret-du-client');
    expect(where).not.toContain('client-acme-profil');
  }
  // Nor any right on the files given to the plugin: its permissions are the manifest's.
  const views = await invoke<{ id: string; manifest: { permissions: { files?: unknown[] } } }[]>('plugins_list');
  expect(views.find((v) => v.id === ID)?.manifest.permissions.files ?? []).toEqual([]);
  await forget(invoke);
});

test.describe('in English', () => {
  test.use({ language: 'en' });

  test('the groups, the import and the kept secret speak English', async ({ app }) => {
    test.setTimeout(180_000);
    const { page } = app;
    const invoke = invokeOn(page);
    await addProject(page, app.repo, { lang: 'en' });
    const src = path.join(app.root, 'tunnel-plugin');
    tunnelPlugin(src);
    const { token } = await invoke<{ token: string }>('plugin_inspect_folder', { path: src });
    await invoke('plugin_install', { token });
    // The keychain of a sandboxed run outlives it: nothing kept by a run before.
    await forget(invoke);
    await openSettings(page, /^Settings/, 'Added');
    const panel = page.getByRole('tabpanel', { name: 'Settings' });
    await expect(panel.getByRole('region', { name: 'Connection' })).toContainText('For every project');
    await expect(panel.getByRole('button', { name: /^Advanced/ })).toContainText('3 settings');
    const conf = panel.getByRole('group', { name: 'OpenVPN configuration' });
    await expect(conf.getByRole('button', { name: 'Import a file…' })).toBeVisible();
    const file = path.join(app.root, 'profile.conf');
    fs.writeFileSync(file, ovpn(4));
    await chooseFile(page, 'ovpn', file);
    await expect(conf.getByRole('textbox', { name: 'OpenVPN configuration' })).toHaveValue(ovpn(4));
    await conf.getByRole('button', { name: 'Keep', exact: true }).click();
    await expect(conf).toContainText(kept(ovpn(4), 'en-US'));
    await expect(conf.getByRole('button', { name: 'Replace…' })).toBeVisible();
    await shoot(page, 'en');
    await holds(panel);
    await forget(invoke);
  });
});
