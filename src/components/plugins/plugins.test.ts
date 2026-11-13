import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { Manifest, PluginPreview, PluginView } from '../../lib/plugins';
import { asksMore, plugins } from '../../lib/plugins-view.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import InstallModal from './InstallModal.svelte';
import PluginPage from './PluginPage.svelte';
import PluginProjects from './PluginProjects.svelte';
import PluginsView from './PluginsView.host.test.svelte';

// The view Plugins (PLG-4): the install window says where a plugin comes from, its fingerprint, what it adds and what it
// may do, the strongest first, and installs nothing before « Installer »; an update says what it asks for more; a plugin
// is switched on project by project, a private project off by default; a permission is taken back.

const MANIFEST: Manifest = {
  id: 'dev.exemple.slack',
  name: 'Slack',
  version: '1.2.0',
  description: { fr: 'Les tickets à tester dans un salon Slack.', en: 'Tickets to test in a Slack channel.' },
  author: 'lea-martin',
  license: 'MIT',
  escouade: { api: 1, min: '2.1.0' },
  run: { command: 'bin/escouade-slack', args: [] },
  activation: ['event:ticket.column'],
  permissions: {
    events: ['ticket.column', 'agent.waiting'],
    network: ['hooks.slack.com'],
    secrets: ['webhook'],
    commands: [],
    decide: true,
  },
  contributes: {
    webhooks: [{ id: 'post', name: 'Message Slack' }],
    secrets: [{ key: 'webhook', title: { fr: 'Adresse du webhook', en: 'Webhook URL' } }],
  },
};
const ASKS = ['run', 'decide', 'events:ticket.column', 'events:agent.waiting', 'network:hooks.slack.com', 'secrets:webhook'];

function preview(over: Partial<PluginPreview> = {}): PluginPreview {
  return {
    token: 'tok',
    icon: null,
    id: MANIFEST.id,
    name: 'Slack',
    version: '1.2.0',
    description: 'Les tickets à tester dans un salon Slack.',
    origin: { kind: 'git', url: 'https://github.com/lea-martin/escouade-slack', tag: 'v1.2.0' },
    fingerprint: 'git:3f9c2e1aa0b',
    declarative: false,
    refusal: null,
    asks: ASKS,
    previous: null,
    grown: [],
    manifest: MANIFEST,
    verified: false,
    unverified: null,
    ...over,
  };
}

function view(over: Partial<PluginView> = {}): PluginView {
  return {
    id: MANIFEST.id,
    name: 'Slack',
    version: '1.2.0',
    description: 'Les tickets à tester dans un salon Slack.',
    origin: { kind: 'git', url: 'https://github.com/lea-martin/escouade-slack', tag: 'v1.2.0' },
    fingerprint: 'git:3f9c2e1aa0b',
    folder: 'C:/data/plugins/dev.exemple.slack',
    declarative: false,
    state: { kind: 'ready' },
    ignored: [],
    manifest: MANIFEST,
    installedAt: '2026-10-29T10:00:00Z',
    verified: false,
    withdrawn: null,
    inStore: null,
    choices: { enabled: true, projects: {}, revoked: [], settings: {} },
    asks: ASKS,
    secretKeys: ['webhook'],
    update: null,
    commands: [],
    commandsApproved: true,
    ...over,
  };
}

beforeEach(() => {
  setLang('fr');
  resetApp({
    projects: [project({ id: 'p1', name: 'demo-api' }), project({ id: 'p2', name: 'compta-interne', private: true })],
  });
  plugins.list = [];
  plugins.install = null;
  plugins.error = null;
  plugins.page = { kind: 'installed' };
  plugins.icons = {};
  plugins.rows = {};
  plugins.ahead = [];
});

describe('the tile of a plugin', () => {
  const LOGO = 'data:image/png;base64,iVBORw0KGgo=';

  it('shows its logo once plugin_icon gives it, its letters for one without', async () => {
    const backend = fakeBackend({
      plugins_list: () => [view(), view({ id: 'dev.exemple.nord', name: 'Nord' })],
      plugin_icon: (a: { id: string }) => (a.id === MANIFEST.id ? LOGO : null),
    });
    render(PluginsView);
    const table = await screen.findByRole('table', { name: 'Ajoutés' });
    await expect.poll(() => table.querySelector('.tile img')?.getAttribute('src')).toBe(LOGO);
    const tiles = [...table.querySelectorAll('.tile')];
    expect(tiles.map((t) => (t.querySelector('img') ? 'logo' : t.textContent?.trim()))).toEqual(['logo', 'No']);
    // Read once by id, never again for the same plugin.
    expect(
      backend
        .called('plugin_icon')
        .map((c) => c.args.id)
        .sort(),
    ).toEqual([MANIFEST.id, 'dev.exemple.nord'].sort());
  });

  it('shows the logo of a plugin read before it is installed, in the install window', () => {
    plugins.install = { kind: 'preview', preview: preview({ icon: LOGO }), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    expect(screen.getByRole('dialog').querySelector('.who .tile img')?.getAttribute('src')).toBe(LOGO);
  });
});

describe('the install window', () => {
  it('says where it comes from, its fingerprint, what it adds and what it may do, the strongest first', () => {
    plugins.install = { kind: 'preview', preview: preview(), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Installer Slack ?' });
    const d = within(dialog);
    expect(d.getByText('Personne n’a relu ce plugin.')).toBeTruthy();
    expect(dialog.textContent).toContain('Dépôt git · github.com/lea-martin/escouade-slack · v1.2.0');
    expect(dialog.textContent).toContain('git:3f9c2e1aa0b');
    expect(dialog.textContent).toContain('Aucune : hors du store, non vérifié');
    expect(dialog.textContent).toContain('Un webhook : Message Slack');
    const lines = [...dialog.querySelectorAll('.asks li .txt')].map((l) => l.firstChild?.textContent);
    expect(lines).toEqual([
      'Lance un programme sur ton ordinateur, avec tes droits',
      'Décide des autorisations de tes agents',
      'Reçoit ces événements',
      'Reçoit ces événements',
      'Se connecte à',
      'Garde un secret : Adresse du webhook',
    ]);
    expect(dialog.querySelector('.asks li.red')?.textContent).toContain('Décide des autorisations');
    expect(d.getByRole('button', { name: 'Installer quand même' })).toBeTruthy();
  });

  it('says a JS/TS plugin runs inside Escouade, isolated, and that its hosts are enforced (ESC-320)', () => {
    const wasm = { ...MANIFEST, run: { command: '', js: 'dist/plugin.mjs', args: [] } };
    plugins.install = { kind: 'preview', preview: preview({ manifest: wasm }), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Installer Slack ?' });
    const first = dialog.querySelector('.asks li');
    expect(first?.textContent).toContain('Tourne dans Escouade, isolé');
    expect(first?.textContent).toContain('dist/plugin.mjs');
    expect(first?.classList.contains('strong')).toBe(false);
    expect(dialog.textContent).not.toContain('Lance un programme');
    expect(dialog.textContent).toContain(
      'appliqué par Escouade : il ne joint aucun autre hôte, ni une adresse de ta machine ou de ton réseau local',
    );
    expect(dialog.textContent).not.toContain('ne filtre pas encore');
  });

  it('says the folders and the commands of a plugin in WASM, writing in orange and commands in red (ESC-333)', () => {
    const wasm = {
      ...MANIFEST,
      run: { command: '', js: 'dist/plugin.mjs', args: [] },
      permissions: {
        ...MANIFEST.permissions,
        commands: ['isola ls --json'],
        files: [
          { root: 'agent', access: 'write' },
          { root: 'agentRepo', access: 'read' },
          { root: 'agentHome', access: 'read' },
        ],
      },
    };
    const asks = ['run', 'commands:isola ls --json', 'files:agent:write', 'files:agentRepo:read', 'files:agentHome:read'];
    plugins.install = { kind: 'preview', preview: preview({ manifest: wasm, asks }), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Installer Slack ?' });
    const line = (text: string) => [...dialog.querySelectorAll('.asks li')].find((l) => l.textContent?.includes(text));
    const command = line('Lance cette commande sur ton ordinateur');
    expect(command?.textContent).toContain('isola ls --json');
    expect(command?.textContent).toContain('appliqué par Escouade');
    expect(command?.classList.contains('red')).toBe(true);
    expect(line('Écrit dans les dossiers de tes agents')?.classList.contains('orange')).toBe(true);
    expect(line('Lit les fichiers de tes agents')?.classList.contains('orange')).toBe(false);
    expect(line('Lit le dossier de compte de tes agents')).toBeTruthy();
  });

  it('says a command of a process is only declared (ESC-333)', () => {
    const asks = ['run', 'commands:isola down'];
    plugins.install = { kind: 'preview', preview: preview({ asks }), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Installer Slack ?' });
    expect(dialog.textContent).toContain('déclaré par le plugin : Escouade ne filtre pas ce que lance son programme');
  });

  it('says Escouade holds the webhooks of a declarative plugin to its hosts (ESC-184)', () => {
    const declarative = { ...MANIFEST, run: undefined };
    plugins.install = { kind: 'preview', preview: preview({ manifest: declarative }), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Installer Slack ?' });
    expect(dialog.textContent).toContain('appliqué par Escouade : ses webhooks ne partent que vers cet hôte');
    expect(dialog.textContent).not.toContain('ne filtre pas encore');
  });

  it('names a webhook by what sets it off, and says where a store plugin comes from once (ESC-184)', () => {
    const contributes = { webhooks: [{ events: ['ticket.column'] }, { events: ['agent.status'] }] };
    const manifest = { ...MANIFEST, contributes } as unknown as typeof MANIFEST;
    const origin = {
      kind: 'store' as const,
      version: '1.0.0',
      sha256: 'abc',
      verified: true,
      repository: 'https://github.com/o/r',
      tag: 'v1.0.0',
      commit: 'a2480d724f8e',
    };
    plugins.install = { kind: 'preview', preview: preview({ manifest, origin }), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Installer Slack ?' });
    expect(dialog.textContent).toContain('2 webhooks : ticket.column, agent.status');
    expect(dialog.textContent).not.toContain('store.escouade.dev · store.escouade.dev');
    expect(dialog.textContent).toContain('store.escouade.dev');
  });

  it('installs nothing before « Installer », then the plugin; cancelled, what was set aside is discarded', async () => {
    const backend = fakeBackend({ plugin_install: () => view() });
    plugins.install = { kind: 'preview', preview: preview(), busy: false, error: null };
    const { unmount } = render(InstallModal, { step: plugins.install });
    expect(backend.called('plugin_install')).toHaveLength(0);
    await userEvent.click(screen.getByRole('button', { name: 'Installer quand même' }));
    expect(backend.called('plugin_install')[0].args).toEqual({ token: 'tok' });
    expect(plugins.list.map((p) => p.id)).toEqual([MANIFEST.id]);
    expect(plugins.page).toEqual({ kind: 'plugin', id: MANIFEST.id, tab: 'overview' });
    expect(plugins.install).toBeNull();
    unmount();

    plugins.install = { kind: 'preview', preview: preview({ token: 'other' }), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(backend.called('plugin_discard')[0].args).toEqual({ token: 'other' });
    expect(backend.called('plugin_install')).toHaveLength(1);
  });

  it('an update says what it asks for more, marked, and asks again', () => {
    plugins.install = {
      kind: 'preview',
      preview: preview({
        version: '1.3.0',
        previous: { version: '1.2.0', fingerprint: 'git:aaa', asks: ASKS.slice(0, 5) },
        grown: ['secrets:webhook'],
      }),
      busy: false,
      error: null,
    };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Mettre à jour Slack ?' });
    expect(dialog.textContent).toContain('Cette version demande plus que la v1.2.0 installée : ton accord est redemandé.');
    const marked = [...dialog.querySelectorAll('.asks li')].filter((l) => l.querySelector('.new'));
    expect(marked.map((l) => l.textContent)).toEqual([expect.stringContaining('Garde un secret : Adresse du webhook')]);
    expect(within(dialog).getByRole('button', { name: 'Mettre à jour' })).toBeTruthy();
  });

  it('a refusal stays in the window, and nothing is installed', async () => {
    fakeBackend({
      plugin_inspect_git: () => {
        throw { message: "Le dépôt x n'a pas pu être cloné à l'étiquette v9 : not found", problems: [] };
      },
    });
    plugins.fromGit('x', 'v9');
    render(InstallModal, { step: plugins.install! });
    await userEvent.click(screen.getByRole('button', { name: 'Lire le plugin' }));
    expect((await screen.findByRole('alert')).textContent).toContain("n'a pas pu être cloné à l'étiquette v9");
    expect(plugins.list).toEqual([]);
  });
});

describe('the projects of a plugin', () => {
  it('is on in a project unless private, switched project by project', async () => {
    const backend = fakeBackend({
      plugin_set_project: (a: { id: string; project: string; on: boolean }) =>
        view({ choices: { enabled: true, projects: { [a.project]: a.on }, revoked: [], settings: {} } }),
    });
    plugins.list = [view()];
    render(PluginProjects, { plugin: plugins.list[0] });
    const demo = screen.getByRole('switch', { name: 'Actif dans demo-api' });
    const compta = screen.getByRole('switch', { name: 'Actif dans compta-interne' });
    expect(demo.getAttribute('aria-checked')).toBe('true');
    expect(compta.getAttribute('aria-checked')).toBe('false');
    expect(screen.getByText(/projet privé : coupé d’office, rien ne lui parvient/)).toBeTruthy();

    await userEvent.click(compta);
    expect(backend.called('plugin_set_project')[0].args).toEqual({ id: MANIFEST.id, project: 'p2', on: true });
    expect(plugins.list[0].choices.projects).toEqual({ p2: true });
  });

  it('says it reaches no project when it is off for the whole app', () => {
    render(PluginProjects, { plugin: view({ choices: { enabled: false, projects: {}, revoked: [], settings: {} } }) });
    expect(screen.getByText('Désactivé pour toute l’app : aucun projet ne le reçoit.')).toBeTruthy();
    for (const s of screen.getAllByRole('switch')) expect((s as HTMLButtonElement).disabled).toBe(true);
  });
});

describe('the permissions of a plugin', () => {
  it('takes one back, and gives it again; the program itself has no switch', async () => {
    const backend = fakeBackend({
      plugins_list: () => plugins.list,
      plugin_set_permission: (a: { word: string; granted: boolean }) =>
        view({ choices: { enabled: true, projects: {}, revoked: a.granted ? [] : [a.word], settings: {} } }),
    });
    plugins.list = [view()];
    plugins.page = { kind: 'plugin', id: MANIFEST.id, tab: 'permissions' };
    render(PluginsView);
    const panel = await screen.findByRole('tabpanel', { name: 'Permissions' });
    const p = within(panel);
    expect(p.queryByRole('switch', { name: /Lance un programme/ })).toBeNull();
    expect(panel.textContent).toContain('Pour l’arrêter, désactive le plugin.');
    const network = p.getByRole('switch', { name: 'Se connecte à hooks.slack.com : permise' });
    await userEvent.click(network);
    expect(backend.called('plugin_set_permission')[0].args).toEqual({ id: MANIFEST.id, word: 'network:hooks.slack.com', granted: false });
    expect(plugins.list[0].choices.revoked).toEqual(['network:hooks.slack.com']);
    const again = within(screen.getByRole('tabpanel', { name: 'Permissions' })).getByRole('switch', {
      name: 'Se connecte à hooks.slack.com : permise',
    });
    expect(again.getAttribute('aria-checked')).toBe('false');
    await userEvent.click(again);
    expect(backend.called('plugin_set_permission')[1].args).toMatchObject({ word: 'network:hooks.slack.com', granted: true });
  });
});

describe('the view', () => {
  it('lists the installed plugins with their projects in pills, a lock for a private one, and an update offered', async () => {
    fakeBackend({ plugins_list: () => [view({ update: { version: '1.3.0', tag: 'v1.3.0' } })] });
    render(PluginsView);
    const table = await screen.findByRole('table', { name: 'Ajoutés' });
    expect(table.textContent).toContain('Slack');
    expect(table.textContent).toContain('v1.3.0 disponible');
    expect(within(table).getByRole('img', { name: 'demo-api : actif' })).toBeTruthy();
    expect(within(table).getByRole('img', { name: 'compta-interne : coupé' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Tout mettre à jour · 1' })).toBeTruthy();
    // The sidebar counts the updates, in amber.
    const nav = screen.getByRole('complementary', { name: 'Plugins' });
    expect(within(nav).getByRole('button', { name: /Mises à jour/ }).className).toContain('upd');
  });

  it('puts a dev.escouade.* of the store with the plugins added, not with those the app gives (ESC-314)', async () => {
    const store = {
      kind: 'store' as const,
      version: '1.1.0',
      sha256: 'a'.repeat(64),
      verified: true,
      repository: 'https://github.com/guillaume-gagnaire/escouade',
      path: 'plugins/slack',
      tag: 'plugins/slack/v1.1.0',
      commit: 'c'.repeat(40),
    };
    fakeBackend({
      plugins_list: () => [
        view({ id: 'dev.escouade.slack', origin: store, folder: 'C:/data/plugins/dev.escouade.slack' }),
        view({ id: 'dev.escouade.github', name: 'GitHub', origin: { kind: 'workspace' } }),
      ],
    });
    render(PluginsView);
    const added = await screen.findByRole('table', { name: 'Ajoutés' });
    expect(added.textContent).toContain('Slack');
    expect(added.textContent).not.toContain('GitHub');
    const shipped = screen.getByRole('table', { name: 'Du dépôt' });
    expect(shipped.textContent).toContain('GitHub');
    expect(shipped.textContent).not.toContain('Slack');
  });

  it('proposes what the seed holds and is not installed, and installs it in one click only (ESC-338, ESC-373)', async () => {
    const calls: string[] = [];
    let offers = [
      { id: 'dev.escouade.trello', name: 'Trello', version: '2.2.0', removed: '2.2.0', refused: null },
      { id: 'dev.escouade.jira', name: 'Jira', version: '2.2.0', removed: null, refused: 'La signature ne tient pas.' },
      { id: 'dev.escouade.linear', name: 'Linear', version: '2.2.0', removed: null, refused: null },
    ];
    fakeBackend({
      plugins_list: () => [],
      plugin_seed_offers: () => offers,
      plugin_install_seed: ({ id }: { id: string }) => {
        calls.push(id);
        offers = offers.filter((o) => o.id !== id);
        return view({ id, name: 'Trello' });
      },
    });
    render(PluginsView);
    const proposed = await screen.findByRole('region', { name: 'Proposés par Escouade' });
    expect(proposed.textContent).toContain('Tu l’as désinstallé (v2.2.0)');
    expect(proposed.textContent).toContain('Non installé : La signature ne tient pas.');
    expect(proposed.textContent).toContain('Dans l’app, sans téléchargement');
    expect(calls).toEqual([]);
    await userEvent.click(within(proposed).getAllByRole('button', { name: 'Installer' })[0]!);
    expect(calls).toEqual(['dev.escouade.trello']);
    // No window: installed at once, and no longer proposed.
    expect(plugins.install).toBeNull();
    await waitFor(() => expect(plugins.byId('dev.escouade.trello')).toBeTruthy());
    expect(plugins.offers.map((o) => o.id)).toEqual(['dev.escouade.jira', 'dev.escouade.linear']);
  });

  it('takes a proposed plugin from the store when the store has a newer version (ESC-338)', async () => {
    plugins.offers = [];
    const calls: string[] = [];
    fakeBackend({
      plugins_list: () => [],
      plugin_seed_offers: () => [{ id: 'dev.escouade.trello', name: 'Trello', version: '2.2.0', removed: '2.2.0', refused: null }],
      store_view: () => ({
        url: '',
        catalog: null,
        readAt: '2026-11-05T00:00:00Z',
        error: null,
        best: { 'dev.escouade.trello': '2.10.0' },
      }),
      plugin_inspect_store: ({ id }: { id: string }) => {
        calls.push(`store:${id}`);
        return preview({ id });
      },
      plugin_install_seed: ({ id }: { id: string }) => {
        calls.push(`seed:${id}`);
        return null;
      },
    });
    render(PluginsView);
    const proposed = await screen.findByRole('region', { name: 'Proposés par Escouade' });
    await waitFor(() => expect(plugins.store?.best).toBeTruthy());
    await userEvent.click(within(proposed).getByRole('button', { name: 'Installer' }));
    expect(calls).toEqual(['store:dev.escouade.trello']);
  });

  it('installs an update the seed holds from the seed, with no network (ESC-338)', async () => {
    const calls: string[] = [];
    fakeBackend({
      plugin_inspect_seed: ({ id }: { id: string }) => {
        calls.push(id);
        return preview({ id });
      },
    });
    await plugins.update(view({ id: 'dev.escouade.trello', update: { version: '2.3.0', verified: true, seed: true } }));
    expect(calls).toEqual(['dev.escouade.trello']);
  });

  it('a plugin is uninstalled after its confirmation, its data kept if asked', async () => {
    const backend = fakeBackend({ plugin_remove: () => true });
    plugins.list = [view()];
    render(PluginPage, { plugin: plugins.list[0], tab: 'overview' });
    await userEvent.click(screen.getByRole('button', { name: 'Désinstaller' }));
    const modal = app.modal as { kind: 'confirm'; title: string; option?: { label: string }; onConfirm: (o: boolean) => Promise<void> };
    expect(modal.title).toBe('Désinstaller Slack ?');
    expect(modal.option?.label).toBe('Garder ses réglages et ses secrets pour une prochaine installation');
    await modal.onConfirm(true);
    expect(backend.called('plugin_remove')[0].args).toEqual({ id: MANIFEST.id, keep: true });
    expect(plugins.list).toEqual([]);
  });

  it('is in English too', async () => {
    setLang('en');
    plugins.install = { kind: 'preview', preview: preview(), busy: false, error: null };
    render(InstallModal, { step: plugins.install });
    const dialog = screen.getByRole('dialog', { name: 'Install Slack?' });
    expect(dialog.textContent).toContain('Runs a program on your computer, with your rights');
    expect(dialog.textContent).toContain('Keeps a secret: Webhook URL');
    expect(within(dialog).getByRole('button', { name: 'Install anyway' })).toBeTruthy();
  });
});

describe('the commands a plugin runs on facts (PLG-5)', () => {
  it('are shown with its permissions and run nothing before « Accepter », which can be taken back', async () => {
    const backend = fakeBackend({
      plugin_approve_commands: (a: { on: boolean }) => view({ commands: ['say "Claude a fini"'], commandsApproved: a.on }),
    });
    plugins.list = [view({ commands: ['say "Claude a fini"'], commandsApproved: false })];
    plugins.page = { kind: 'plugin', id: MANIFEST.id, tab: 'permissions' };
    const { rerender } = render(PluginPage, { plugin: plugins.list[0], tab: 'permissions' });
    const box = screen.getByRole('region', { name: 'Lance ces commandes' });
    expect(box.textContent).toContain('say "Claude a fini"');
    expect(box.textContent).toContain('Rien ne tourne avant ton accord');
    await userEvent.click(within(box).getByRole('button', { name: 'Accepter ces commandes' }));
    expect(backend.called('plugin_approve_commands')[0].args).toEqual({ id: MANIFEST.id, on: true });
    await rerender({ plugin: plugins.list[0], tab: 'permissions' });
    const agreed = screen.getByRole('region', { name: 'Lance ces commandes' });
    await userEvent.click(within(agreed).getByRole('button', { name: 'Retirer l’accord' }));
    expect(backend.called('plugin_approve_commands')[1].args).toEqual({ id: MANIFEST.id, on: false });
  });

  it('are not shown for a plugin that runs none', () => {
    plugins.list = [view()];
    render(PluginPage, { plugin: plugins.list[0], tab: 'permissions' });
    expect(screen.queryByRole('region', { name: 'Lance ces commandes' })).toBeNull();
  });
});

describe('« Rechercher des mises à jour » (ESC-405)', () => {
  // Local times: the line says the hour of this machine.
  const READ = '2026-11-09T10:42:00';
  const storeView = (error: string | null) => ({ url: '', catalog: null, readAt: READ, error, best: {} });

  it('shows that it looks while the store is read, then when the store was read and the version found', async () => {
    let answer: (v: unknown) => void = () => {};
    const backend = fakeBackend({
      plugins_list: () => [view()],
      plugins_check_updates: () => new Promise((done) => (answer = done)),
    });
    plugins.page = { kind: 'updates' };
    render(PluginsView);
    await userEvent.click(await screen.findByRole('button', { name: 'Chercher maintenant' }));
    const busy = await screen.findByRole('button', { name: 'Recherche…' });
    expect(busy.hasAttribute('disabled')).toBe(true);
    expect(busy.getAttribute('aria-busy')).toBe('true');
    expect(busy.querySelector('.spin')).toBeTruthy();
    // Clicked again while it looks: nothing more is asked.
    await userEvent.click(busy);
    expect(backend.called('plugins_check_updates')).toHaveLength(1);

    answer({ plugins: [view({ update: { version: '1.3.0', verified: true, seed: false } })], store: storeView(null) });
    const status = await screen.findByRole('status');
    expect(status.textContent).toBe('Store lu le 09/11 10:42');
    expect(status.className).not.toContain('warn');
    expect(screen.getByRole('button', { name: 'Chercher maintenant' }).getAttribute('aria-busy')).toBe('false');
    expect(document.body.textContent).toContain('v1.2.0 → v1.3.0');
  });

  it('says a store that does not answer, and of when the offers looked at are', async () => {
    fakeBackend({
      plugins_list: () => [view()],
      plugins_check_updates: () => ({
        plugins: [view()],
        store: storeView('Le store ne répond pas (HTTP 503) : le catalogue gardé sur ce poste est montré.'),
      }),
    });
    plugins.page = { kind: 'updates' };
    render(PluginsView);
    await userEvent.click(await screen.findByRole('button', { name: 'Chercher maintenant' }));
    const status = await screen.findByRole('status');
    expect(status.textContent).toBe('Store injoignable : offres du 09/11 10:42');
    expect(status.className).toContain('warn');
  });

  it('says it on the tab Versions of a plugin too', async () => {
    fakeBackend({ plugins_check_updates: () => ({ plugins: [view()], store: storeView(null) }) });
    plugins.store = null;
    render(PluginPage, { plugin: view(), tab: 'versions' });
    await userEvent.click(screen.getByRole('button', { name: 'Chercher une mise à jour' }));
    expect((await screen.findByRole('status')).textContent).toBe('Store lu le 09/11 10:42');
  });
});

describe('« Tout mettre à jour », and an update that leaves the page where it was (ESC-436)', () => {
  // Alpha, of the store: its 1.3.0 asks for nothing more. Slack, of a repository: its 1.3.0 connects to one more host.
  const STORE = {
    kind: 'store',
    version: '1.2.0',
    sha256: 'a'.repeat(64),
    verified: true,
    repository: 'https://github.com/x/alpha',
    tag: 'v1.2.0',
    commit: 'c'.repeat(40),
  } as const;
  const alpha = (over: Partial<PluginView> = {}) =>
    view({ id: 'dev.exemple.alpha', name: 'Alpha', origin: STORE, update: { version: '1.3.0', verified: true }, ...over });
  const slack = (over: Partial<PluginView> = {}) => view({ update: { version: '1.3.0', tag: 'v1.3.0' }, ...over });
  const PREVIOUS = { version: '1.2.0', fingerprint: 'x', asks: ASKS };
  const alphaPreview = () =>
    preview({ token: 'tok-alpha', id: 'dev.exemple.alpha', name: 'Alpha', version: '1.3.0', previous: PREVIOUS, verified: true });
  const slackPreview = () =>
    preview({
      token: 'tok-slack',
      version: '1.3.0',
      previous: PREVIOUS,
      grown: ['network:api.slack.com'],
      asks: [...ASKS, 'network:api.slack.com'],
    });

  function backend(over: Record<string, (a: any) => unknown> = {}) {
    return fakeBackend({
      plugins_list: () => plugins.list,
      plugin_inspect_store: () => alphaPreview(),
      plugin_inspect_git: () => slackPreview(),
      plugin_install: ({ token }: { token: string }) =>
        token === 'tok-alpha' ? alpha({ version: '1.3.0', update: null }) : slack({ version: '1.3.0', update: null }),
      ...over,
    });
  }

  it('is at the head of the updates page, reads them all, then asks once for those that ask for nothing more', async () => {
    const calls = backend();
    plugins.list = [alpha(), slack()];
    plugins.page = { kind: 'updates' };
    render(PluginsView);
    await userEvent.click(await screen.findByRole('button', { name: 'Tout mettre à jour · 2' }));

    // One window for Alpha, which asks for nothing more: nothing installed before the gesture.
    const batch = await screen.findByRole('dialog', { name: 'Mettre à jour 1 plugin ?' });
    expect(batch.textContent).toContain('Alpha');
    expect(batch.textContent).toContain('v1.2.0 → v1.3.0');
    expect(batch.textContent).toContain('Vérifié par Escouade');
    expect(batch.textContent).toContain('Ensuite, 1 mise à jour qui demande plus');
    expect(calls.called('plugin_install')).toHaveLength(0);
    // While it waits, « Tout mettre à jour » cannot be clicked again.
    expect(screen.getByRole('button', { name: 'Tout mettre à jour · 2', hidden: true }).hasAttribute('disabled')).toBe(true);
    await userEvent.click(within(batch).getByRole('button', { name: 'Mettre à jour' }));

    // Then Slack, in its own window, what it adds marked.
    const own = await screen.findByRole('dialog', { name: 'Mettre à jour Slack ?' });
    expect(own.textContent).toContain('Cette version demande plus que la v1.2.0 installée');
    expect(own.textContent).toContain('nouveau');
    expect(calls.called('plugin_install').map((c) => c.args.token)).toEqual(['tok-alpha']);
    await userEvent.click(within(own).getByRole('button', { name: 'Mettre à jour' }));

    await waitFor(() => expect(calls.called('plugin_install').map((c) => c.args.token)).toEqual(['tok-alpha', 'tok-slack']));
    // The page stays the updates', and says it is empty; the lines say what was done.
    expect(plugins.page).toEqual({ kind: 'updates' });
    expect(await screen.findByText('Tes plugins sont à jour.')).toBeTruthy();
    expect(screen.getAllByText('À jour en v1.3.0')).toHaveLength(2);
    expect(screen.queryByRole('button', { name: /Tout mettre à jour/ })).toBeNull();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('refusing one cancels none of the others, and installs nothing', async () => {
    const calls = backend();
    plugins.list = [alpha(), slack()];
    await plugins.updateAll();
    expect(plugins.install?.kind).toBe('batch');
    plugins.cancel();
    // The one that asks for more is shown all the same.
    expect(plugins.install?.kind === 'preview' && plugins.install.preview.id).toBe(MANIFEST.id);
    plugins.cancel();
    expect(plugins.install).toBeNull();
    expect(calls.called('plugin_install')).toHaveLength(0);
    expect(calls.called('plugin_discard').map((c) => c.args.token)).toEqual(['tok-alpha', 'tok-slack']);
    // Both still offered, their buttons free again.
    expect(plugins.rows).toEqual({});
    expect(plugins.updates.map((p) => p.id)).toEqual(['dev.exemple.alpha', MANIFEST.id]);
  });

  it('an error on one is said on its line and stops none of the others', async () => {
    const beta = alpha({ id: 'dev.exemple.beta', name: 'Beta' });
    const calls = backend({
      plugin_inspect_store: ({ id }: { id: string }) => {
        if (id === 'dev.exemple.alpha') throw { message: 'Cette version n’est pas vérifiée.', problems: [] };
        return preview({ ...alphaPreview(), token: 'tok-beta', id, name: 'Beta' });
      },
      plugin_install: ({ token }: { token: string }) => {
        if (token === 'tok-beta') throw { message: 'Le store ne répond pas.', problems: [] };
        return slack({ version: '1.3.0', update: null });
      },
    });
    plugins.list = [alpha(), beta, slack()];
    plugins.page = { kind: 'updates' };
    render(PluginsView);
    await userEvent.click(await screen.findByRole('button', { name: 'Tout mettre à jour · 3' }));
    const batch = await screen.findByRole('dialog', { name: 'Mettre à jour 1 plugin ?' });
    expect(batch.textContent).toContain('Beta');
    await userEvent.click(within(batch).getByRole('button', { name: 'Mettre à jour' }));
    const own = await screen.findByRole('dialog', { name: 'Mettre à jour Slack ?' });
    await userEvent.click(within(own).getByRole('button', { name: 'Mettre à jour' }));
    await waitFor(() => expect(calls.called('plugin_install')).toHaveLength(2));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    const alerts = screen.getAllByRole('alert').map((a) => a.textContent);
    expect(alerts).toContain('Cette version n’est pas vérifiée.');
    expect(alerts).toContain('Le store ne répond pas.');
    expect(await screen.findByText('À jour en v1.3.0')).toBeTruthy();
    // Those that failed stay offered, to try again.
    expect(screen.getByRole('button', { name: 'Tout mettre à jour · 2' }).hasAttribute('disabled')).toBe(false);
  });

  it('one update leaves the page where it was; its line shows it under way, its button not clickable twice', async () => {
    let read: (v: PluginPreview) => void = () => {};
    const calls = backend({ plugin_inspect_git: () => new Promise((done) => (read = done)) });
    plugins.list = [slack()];
    plugins.page = { kind: 'updates' };
    render(PluginsView);
    const button = await screen.findByRole('button', { name: 'Mettre à jour' });
    await userEvent.click(button);
    expect((await screen.findByText('Lecture de la nouvelle version…')).closest('.up')).toBeTruthy();
    expect(button.hasAttribute('disabled')).toBe(true);
    await userEvent.click(button);
    expect(calls.called('plugin_inspect_git')).toHaveLength(1);
    read(slackPreview());
    const own = await screen.findByRole('dialog', { name: 'Mettre à jour Slack ?' });
    expect(screen.getByText('En attente de ton accord')).toBeTruthy();
    await userEvent.click(within(own).getByRole('button', { name: 'Mettre à jour' }));
    expect(await screen.findByText('À jour en v1.3.0')).toBeTruthy();
    expect(plugins.page).toEqual({ kind: 'updates' });

    // From the installed ones, or a plugin's page: the page stays too.
    for (const page of [{ kind: 'installed' }, { kind: 'plugin', id: MANIFEST.id, tab: 'versions' }] as const) {
      plugins.list = [slack()];
      plugins.page = page;
      plugins.rows = {};
      backend();
      await plugins.update(plugins.list[0]);
      await plugins.confirm();
      expect(plugins.page).toEqual(page);
    }
  });

  it('a version that will be loaded switched off is said in its own window, never in the one for all', async () => {
    backend({ plugin_inspect_store: () => ({ ...alphaPreview(), refusal: 'Escouade ne lance plus run.command.' }) });
    plugins.list = [alpha()];
    await plugins.updateAll();
    expect(plugins.install?.kind === 'preview' && plugins.install.preview.refusal).toBe('Escouade ne lance plus run.command.');
    expect(plugins.ahead).toEqual([]);
  });

  it('a new install alone opens the plugin’s page', async () => {
    backend({ plugin_install: () => view() });
    plugins.page = { kind: 'store' };
    plugins.install = { kind: 'preview', preview: preview(), busy: false, error: null };
    await plugins.confirm();
    expect(plugins.page).toEqual({ kind: 'plugin', id: MANIFEST.id, tab: 'overview' });
  });

  it('an update that adds a contribution, or that cannot be compared, asks in its own window', () => {
    const more: Manifest = { ...MANIFEST, contributes: { ...MANIFEST.contributes, themes: [{ id: 'nuit', name: 'Nuit' }] } };
    expect(asksMore(alphaPreview(), view())).toBe(false);
    expect(asksMore(slackPreview(), view())).toBe(true);
    expect(asksMore(preview({ ...alphaPreview(), manifest: more }), view())).toBe(true);
    expect(asksMore(alphaPreview(), view({ manifest: null }))).toBe(true);
  });

  it('sorts by the seal too (SEC-29): only a verified version that adds nothing goes in the window for all', () => {
    const folder = { kind: 'folder', path: 'C:/dev/slack' } as const;
    // Verified, nothing added: the window for all.
    expect(asksMore(alphaPreview(), alpha())).toBe(false);
    // Not verified, nothing added: its own window.
    expect(asksMore({ ...alphaPreview(), verified: false }, alpha({ verified: false }))).toBe(true);
    // The seal lost (installed verified, the new one not, or its seal taken off): its own window.
    expect(asksMore({ ...alphaPreview(), verified: false }, alpha({ verified: true }))).toBe(true);
    expect(asksMore({ ...alphaPreview(), verified: false, unverified: 'retirée' }, alpha({ verified: true }))).toBe(true);
    // A plugin of a repository or a folder, never verified, even when it adds nothing: its own window.
    expect(asksMore({ ...alphaPreview(), verified: false, origin: preview().origin, grown: [] }, view())).toBe(true);
    expect(asksMore({ ...alphaPreview(), verified: false, origin: folder }, view({ origin: folder }))).toBe(true);
  });

  it('a version not verified of a plugin that was gets its own window, its warning said (B-16)', async () => {
    // Nuit, of the store: its 1.0.0 verified, its 1.1.0 not, nothing added; Alpha's 1.3.0 verified, nothing added.
    const nuit = (over: Partial<PluginView> = {}) =>
      alpha({ id: 'dev.exemple.nuit', name: 'Nuit', verified: true, update: { version: '1.1.0', verified: false }, ...over });
    const nuitPreview = () =>
      preview({
        ...alphaPreview(),
        token: 'tok-nuit',
        id: 'dev.exemple.nuit',
        name: 'Nuit',
        version: '1.1.0',
        verified: false,
        origin: STORE,
      });
    const calls = backend({
      plugin_inspect_store: ({ id }: { id: string }) => (id === 'dev.exemple.nuit' ? nuitPreview() : alphaPreview()),
      plugin_install: ({ token }: { token: string }) =>
        token === 'tok-nuit' ? nuit({ version: '1.1.0', verified: false, update: null }) : alpha({ version: '1.3.0', update: null }),
    });
    plugins.list = [nuit(), alpha()];
    plugins.page = { kind: 'updates' };
    render(PluginsView);
    await userEvent.click(await screen.findByRole('button', { name: 'Tout mettre à jour · 2' }));

    // The window for all holds Alpha alone, and says its versions are verified.
    const batch = await screen.findByRole('dialog', { name: 'Mettre à jour 1 plugin ?' });
    expect(batch.textContent).toContain('Alpha');
    expect(batch.textContent).not.toContain('Nuit');
    expect(batch.textContent).toContain('Toutes ces versions sont vérifiées par Escouade');
    expect(batch.textContent).not.toContain('Non vérifié');
    await userEvent.click(within(batch).getByRole('button', { name: 'Mettre à jour' }));

    // Then Nuit, in its own window, with the warning of a version nobody reviewed.
    const own = await screen.findByRole('dialog', { name: 'Mettre à jour Nuit ?' });
    expect(own.textContent).toContain('Personne n’a relu cette version.');
    expect(own.textContent).toContain('Aucune : version non vérifiée');
    expect(calls.called('plugin_install').map((c) => c.args.token)).toEqual(['tok-alpha']);
    await userEvent.click(within(own).getByRole('button', { name: 'Mettre à jour' }));
    await waitFor(() => expect(calls.called('plugin_install').map((c) => c.args.token)).toEqual(['tok-alpha', 'tok-nuit']));
  });

  it('with no verified version that adds nothing, no window for all: each its own', async () => {
    backend({ plugin_inspect_store: () => ({ ...alphaPreview(), verified: false }) });
    plugins.list = [alpha({ verified: true })];
    await plugins.updateAll();
    expect(plugins.install?.kind).toBe('preview');
    expect(plugins.ahead).toEqual([]);
  });

  it('is in English too', async () => {
    setLang('en');
    backend();
    plugins.list = [alpha(), slack()];
    plugins.page = { kind: 'updates' };
    render(PluginsView);
    await userEvent.click(await screen.findByRole('button', { name: 'Update all · 2' }));
    const batch = await screen.findByRole('dialog', { name: 'Update 1 plugin?' });
    expect(batch.textContent).toContain('Then 1 update that asks for more, or is not verified: it gets its own window.');
  });
});
