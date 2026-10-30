import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import type { Manifest, PluginPreview, PluginView } from '../../lib/plugins';
import { plugins } from '../../lib/plugins-view.svelte';
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
