import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { setLocks } from '../../lib/locks.svelte';
import {
  kindsOf,
  projectFields,
  projectPlugins,
  projectValue,
  proposedProfile,
  summaryOf,
  teamRefusal,
  teamRefusals,
  usedBy,
} from '../../lib/plugin-profiles';
import type { PluginView } from '../../lib/plugins';
import { plugins } from '../../lib/plugins-view.svelte';
import { app } from '../../lib/state.svelte';
import type { Project } from '../../lib/types';
import { installedPlugin } from '../../test/plugins';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import PluginProjects from './PluginProjects.svelte';
import PluginSettings from './PluginSettings.svelte';

// A plugin's profiles and its settings of each project (ESC-530, issues #38 and #39): the list on the plugin's page, the
// window of a profile, the page Plugins of a project and its marks.

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const A = '0123456789abcdef';
const B = 'fedcba9876543210';

function tunnel(over: Partial<PluginView['choices']> = {}): PluginView {
  const p = installedPlugin({
    id: 'dev.exemple.tunnel',
    name: 'Tunnel',
    version: '1.4.0',
    escouade: { api: '1', min: '2.2.0' },
    permissions: { secrets: ['profile:vpn'] },
    contributes: {
      profiles: [
        {
          id: 'vpn',
          title: { fr: 'Profils VPN', en: 'VPN profiles' },
          nameField: 'name',
          fields: [
            { key: 'name', type: 'string', title: { fr: 'Nom du profil', en: 'Profile name' } },
            { key: 'server', type: 'string', title: { fr: 'Serveur', en: 'Server' } },
            { key: 'proto', type: 'select', title: 'Protocole', options: ['udp', 'tcp'], default: 'udp' },
            { key: 'password', secret: true, title: { fr: 'Mot de passe', en: 'Password' } },
          ],
        },
      ],
      settings: [
        { key: 'profile', type: 'profile', of: 'vpn', scope: 'project', title: { fr: 'Profil VPN', en: 'VPN profile' } },
        { key: 'routes', type: 'string', scope: 'project', title: { fr: 'Routes par le tunnel', en: 'Routes' } },
        { key: 'port', type: 'number', scope: 'project', title: 'Port', repo: false },
        { key: 'fallback', type: 'profile', of: 'vpn', title: { fr: 'Profil par défaut', en: 'Default profile' } },
        { key: 'autoStart', type: 'boolean', title: { fr: 'Tunnel au lancement d’un agent', en: 'Tunnel' } },
      ],
    },
  });
  return {
    ...p,
    name: 'Tunnel',
    choices: {
      ...p.choices,
      profiles: {
        vpn: [
          { id: A, values: { name: 'Client A', server: 'vpn.client-a.example', proto: 'udp' } },
          { id: B, values: { name: 'Client B', server: 'gw.client-b.example', proto: 'tcp' } },
        ],
      },
      projectSettings: { atlas: { profile: A } },
      ...over,
    },
  };
}

function atlas(file: Record<string, unknown> = {}, over: Partial<Project> = {}): Project {
  return project({
    id: 'atlas',
    name: 'atlas-web',
    ...over,
    shared: {
      local: [],
      trusted: '',
      file: {
        keys: Object.keys(file).length ? ['plugins.dev.exemple.tunnel'] : [],
        values: Object.keys(file).length ? { 'plugins.dev.exemple.tunnel': file } : {},
        warnings: [],
        error: null,
        version: 1,
        pending: null,
      },
    },
  });
}

const SECRETS = [
  { kind: 'vpn', profile: A, secrets: [{ key: 'password', kept: true, inFile: false }] },
  { kind: 'vpn', profile: B, secrets: [{ key: 'password', kept: false, inFile: false }] },
];

beforeEach(() => {
  setLang('fr');
  plugins.list = [];
  plugins.loaded = false;
  plugins.page = { kind: 'installed' };
});

describe('what the window reads of the profiles and the settings of each project', () => {
  it('ranks a value: this machine, then the file, then the default; never the file for repo: false', () => {
    const p = tunnel({ projectSettings: { atlas: { port: 1194 } } });
    const routes = projectFields(p.manifest).find((f) => f.key === 'routes')!;
    const port = projectFields(p.manifest).find((f) => f.key === 'port')!;
    expect(projectValue(p, atlas(), routes)).toEqual({ value: '', source: 'default' });
    expect(projectValue(p, atlas({ routes: '10.20.0.0/16' }), routes)).toEqual({
      value: '10.20.0.0/16',
      source: 'file',
      file: '10.20.0.0/16',
    });
    expect(projectValue(p, atlas({ port: 6666 }), port)).toEqual({ value: 1194, source: 'local', file: undefined });
    const q = tunnel({ projectSettings: { atlas: { routes: '10.0.0.0/8' } } });
    expect(projectValue(q, atlas({ routes: '10.20.0.0/16' }), routes)).toEqual({
      value: '10.0.0.0/8',
      source: 'local',
      file: '10.20.0.0/16',
    });
  });

  it('names who uses a profile, sums it up, and finds the one a file proposes by its name', () => {
    const p = tunnel();
    const [kind] = kindsOf(p.manifest);
    expect(usedBy(p, 'vpn', A)).toEqual(['atlas']);
    expect(usedBy(p, 'vpn', B)).toEqual([]);
    expect(summaryOf(kind, p.choices.profiles!.vpn[0])).toBe('vpn.client-a.example · udp');
    expect(proposedProfile(p, kind, ' client a ')?.id).toBe(A);
    expect(proposedProfile(p, kind, 'client-c')).toBeNull();
  });

  it('shows the page Plugins of a project only for a plugin installed, on there, with a setting of each project', () => {
    const p = tunnel();
    expect(projectPlugins([p], atlas())).toEqual([p]);
    expect(projectPlugins([tunnel({ projects: { atlas: false } })], atlas())).toEqual([]);
    expect(projectPlugins([p], atlas({}, { private: true }))).toEqual([]);
    expect(projectPlugins([{ ...p, choices: { ...p.choices, projects: { atlas: true } } }], atlas({}, { private: true }))).toHaveLength(1);
    expect(projectPlugins([{ ...p, state: { kind: 'disabled', reason: 'x' } }], atlas())).toEqual([]);
    const plain = installedPlugin({ id: 'dev.exemple.slack', version: '1.0.0', contributes: { settings: [{ key: 'c', type: 'string' }] } });
    expect(projectPlugins([plain], atlas())).toEqual([]);
  });
});

describe('the profiles on the plugin’s page', () => {
  it('lists them with the projects that use them and their secrets kept; a setting of each project is not there', async () => {
    resetApp({ projects: [atlas()] });
    fakeBackend({ plugin_profile_secrets: () => SECRETS, plugin_secrets: () => [], secrets_file: () => null });
    render(PluginSettings, { plugin: tunnel() });
    const group = await screen.findByRole('region', { name: 'Profils VPN' });
    const a = within(group).getByRole('group', { name: 'Client A' });
    expect(a).toHaveTextContent('vpn.client-a.example · udp');
    expect(a).toHaveTextContent('atlas-web');
    expect(await within(a).findByText('1 secret gardé')).toBeInTheDocument();
    const b = within(group).getByRole('group', { name: 'Client B' });
    expect(b).toHaveTextContent('Aucun projet');
    expect(b).toHaveTextContent('Aucun secret gardé');
    // The default profile, an app's setting, offers the profiles; the project's are not on this page.
    const fallback = screen.getByLabelText('Profil par défaut') as HTMLSelectElement;
    expect([...fallback.options].map((o) => o.text)).toEqual(['Aucun profil', 'Client A', 'Client B']);
    expect(screen.queryByLabelText('Routes par le tunnel')).toBeNull();
    expect(screen.queryByLabelText('Profil VPN')).toBeNull();
  });

  it('makes one in its window, its secret kept under its new id', async () => {
    resetApp({ projects: [atlas()] });
    const NEW = 'aaaaaaaaaaaaaaaa';
    const ipc = fakeBackend({
      plugin_profile_secrets: () => SECRETS,
      plugin_secrets: () => [],
      secrets_file: () => null,
      plugin_profile_save: () => [tunnel(), NEW],
      plugin_profile_secret: () => null,
    });
    render(PluginSettings, { plugin: tunnel() });
    await userEvent.click(await screen.findByRole('button', { name: 'Ajouter un profil…' }));
    const dialog = await screen.findByRole('dialog', { name: 'Nouveau profil · Profils VPN' });
    const save = within(dialog).getByRole('button', { name: 'Enregistrer' });
    expect(save).toBeDisabled();
    await userEvent.type(within(dialog).getByLabelText('Nom du profil'), 'Interne');
    await userEvent.type(within(dialog).getByLabelText('Serveur'), 'vpn.interne.example');
    await userEvent.type(within(dialog).getByPlaceholderText('Colle la valeur'), 'hunter2');
    await userEvent.click(save);
    expect(ipc.called('plugin_profile_save')[0].args).toEqual({
      id: 'dev.exemple.tunnel',
      kind: 'vpn',
      profile: null,
      values: { name: 'Interne', server: 'vpn.interne.example', proto: 'udp' },
    });
    expect(ipc.called('plugin_profile_secret')[0].args).toMatchObject({ profile: NEW, key: 'password', value: 'hunter2' });
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('deletes one after naming the projects that took it', async () => {
    resetApp({ projects: [atlas()] });
    const ipc = fakeBackend({
      plugin_profile_secrets: () => SECRETS,
      plugin_secrets: () => [],
      secrets_file: () => null,
      plugin_profile_delete: () => [tunnel(), { projects: ['atlas'] }],
    });
    render(PluginSettings, { plugin: tunnel() });
    const a = await screen.findByRole('group', { name: 'Client A' });
    await userEvent.click(within(a).getByRole('button', { name: 'Modifier' }));
    const dialog = await screen.findByRole('dialog', { name: 'Profil « Client A »' });
    expect(dialog).toHaveTextContent('Utilisé par atlas-web');
    // A secret kept is never shown: where it is, and « Remplacer… ».
    const pw = within(dialog).getByRole('group', { name: 'Mot de passe' });
    expect(pw).toHaveTextContent('Gardé dans le trousseau');
    expect(within(pw).queryByLabelText('Mot de passe')).toBeNull();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Supprimer le profil' }));
    expect(dialog).toHaveTextContent('Supprimer « Client A » ? Ses secrets sont effacés. atlas-web n’aura plus de profil.');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Supprimer' }));
    expect(ipc.called('plugin_profile_delete')[0].args).toEqual({ id: 'dev.exemple.tunnel', kind: 'vpn', profile: A });
  });
});

describe('the page Plugins of a project', () => {
  it('is in the bar only with a plugin on there that has a setting of each project', async () => {
    resetApp({ projects: [atlas()] });
    fakeBackend({ plugins_list: () => [tunnel()] });
    render(SettingsModal, { page: 'project', projectId: 'atlas' });
    expect(await screen.findByRole('button', { name: 'Plugins', current: undefined })).toBeInTheDocument();
  });

  it('chooses the profile, says the file’s values and offers to write one key into .escouade.yml', async () => {
    resetApp({ projects: [atlas({ routes: '10.20.0.0/16', profile: 'client b', port: 6666 })] });
    let view = tunnel({ projectSettings: { atlas: { profile: A, port: 1194 } } });
    const ipc = fakeBackend({
      plugins_list: () => [view],
      plugin_project_set: () => view,
      plugin_project_write: () => [view, atlas({ routes: '10.20.0.0/16' })],
    });
    render(SettingsModal, { page: 'projectPlugins', projectId: 'atlas' });
    const group = await screen.findByRole('region', { name: 'Tunnel' });
    expect(group).toHaveTextContent('dev.exemple.tunnel');
    // The profile: chosen here, kept on this machine.
    const profile = within(group).getByRole('group', { name: 'Profil VPN' });
    expect(profile).toHaveTextContent('Gardé sur cette machine, jamais dans .escouade.yml.');
    const select = within(profile).getByLabelText('Profil VPN') as HTMLSelectElement;
    expect(select.value).toBe(A);
    // The file proposes « client b »: a band, never a choice; the user takes it.
    const band = screen.getByRole('status', { name: '' });
    expect(band).toHaveTextContent('.escouade.yml propose le profil « client b » pour Tunnel.');
    expect(band).toHaveTextContent('Un dépôt ne choisit pas tes identifiants');
    await userEvent.click(within(band).getByRole('button', { name: 'Prendre « Client B »' }));
    expect(ipc.called('plugin_project_set').at(-1)!.args).toMatchObject({ project: 'atlas', key: 'profile', value: B, clear: false });
    // A simple setting the file defines.
    const routes = within(group).getByRole('group', { name: 'Routes par le tunnel' });
    expect(routes).toHaveTextContent('Défini par .escouade.yml');
    expect((within(routes).getByLabelText('Routes par le tunnel') as HTMLInputElement).value).toBe('10.20.0.0/16');
    // One its author keeps off the file: this machine's, no « Écrire dans .escouade.yml ».
    const port = within(group).getByRole('group', { name: 'Port' });
    expect(port).toHaveTextContent('Gardé sur cette machine, jamais dans .escouade.yml.');
    expect(within(port).queryByRole('button', { name: 'Écrire dans .escouade.yml' })).toBeNull();
    // Set here: ahead of the file, written key by key, or the file's taken back.
    await userEvent.clear(within(routes).getByLabelText('Routes par le tunnel'));
    await userEvent.type(within(routes).getByLabelText('Routes par le tunnel'), '10.0.0.0/8');
    await userEvent.tab();
    expect(ipc.called('plugin_project_set').at(-1)!.args).toMatchObject({ key: 'routes', value: '10.0.0.0/8' });
    app.projects = [atlas({ routes: '10.20.0.0/16' })];
    view = tunnel({ projectSettings: { atlas: { profile: A, routes: '10.0.0.0/8' } } });
    plugins.list = [view];
    const local = await within(screen.getByRole('region', { name: 'Tunnel' })).findByRole('group', { name: 'Routes par le tunnel' });
    expect(local).toHaveTextContent('Réglage local de cette machine : il passe devant .escouade.yml.');
    await userEvent.click(within(local).getByRole('button', { name: 'Reprendre la valeur du fichier' }));
    expect(ipc.called('plugin_project_set').at(-1)!.args).toMatchObject({ key: 'routes', clear: true });
    await userEvent.click(within(local).getByRole('button', { name: 'Écrire dans .escouade.yml' }));
    expect(ipc.called('plugin_project_write')[0].args).toEqual({ id: 'dev.exemple.tunnel', project: 'atlas', key: 'routes' });
  });

  it('says « Aucun profil » when the project has none', async () => {
    resetApp({ projects: [atlas()] });
    fakeBackend({ plugins_list: () => [tunnel({ projectSettings: {} })] });
    render(SettingsModal, { page: 'projectPlugins', projectId: 'atlas' });
    const profile = await screen.findByRole('group', { name: 'Profil VPN' });
    expect(profile).toHaveTextContent('Aucun profil : Tunnel n’en a pas pour ce projet.');
  });
});

describe('the tab Projets of the plugin', () => {
  it('links each project where it is on to its page Plugins', async () => {
    resetApp({ projects: [atlas(), project({ id: 'compta', name: 'compta-interne', private: true })] });
    fakeBackend({});
    render(PluginProjects, { plugin: tunnel() });
    const links = screen.getAllByRole('button', { name: 'Réglages du projet' });
    expect(links).toHaveLength(1);
    await userEvent.click(links[0]);
    expect(app.modal).toEqual({ kind: 'settings', page: 'projectPlugins', projectId: 'atlas' });
  });
});

// What the project's team sets on its plugins (TS-13, ESC-532): imposed (rank 5), greyed with the padlock and who imposes
// it; recommended (rank 2), with where it comes from; refused by the plugin, said with why.
function teamAtlas(file: Record<string, unknown>, defaults: Record<string, unknown>, enforced: Record<string, unknown>): Project {
  const p = atlas(file);
  const prefix = (m: Record<string, unknown>) =>
    Object.fromEntries(Object.entries(m).map(([k, v]) => [`plugins.dev.exemple.tunnel.${k}`, v]));
  return {
    ...p,
    team: 'acme',
    shared: { ...p.shared!, teamPlugins: { team: 'ACME', defaults: prefix(defaults), enforced: prefix(enforced) } },
  };
}

describe('what the project’s team sets on a plugin', () => {
  it('ranks it: imposed over this machine, recommended under the file; a key the plugin refuses never', () => {
    const routes = projectFields(tunnel().manifest).find((f) => f.key === 'routes')!;
    const local = tunnel({ projectSettings: { atlas: { routes: '10.0.0.0/8' } } });
    expect(projectValue(local, teamAtlas({ routes: '10.20.0.0/16' }, {}, { routes: '172.16.0.0/12' }), routes)).toMatchObject({
      value: '172.16.0.0/12',
      source: 'enforced',
      team: 'ACME',
    });
    expect(projectValue(tunnel(), teamAtlas({}, { routes: '10.1.0.0/16' }, {}), routes)).toMatchObject({
      value: '10.1.0.0/16',
      source: 'recommended',
      team: 'ACME',
    });
    expect(projectValue(tunnel(), teamAtlas({ routes: '10.20.0.0/16' }, { routes: '10.1.0.0/16' }, {}), routes)).toMatchObject({
      source: 'file',
      recommended: '10.1.0.0/16',
    });
    expect(projectValue(local, teamAtlas({}, { routes: '10.1.0.0/16' }, {}), routes)).toMatchObject({
      source: 'local',
      value: '10.0.0.0/8',
    });
    // A wrong type is not taken.
    expect(projectValue(tunnel(), teamAtlas({}, {}, { routes: 12 }), routes)).toMatchObject({ source: 'default' });
    const p = tunnel();
    expect(teamRefusal(p, 'routes', 'x')).toBeNull();
    expect(teamRefusal(p, 'routes', 12)).toBe('type');
    expect(teamRefusal(p, 'profile', A)).toBe('profile');
    expect(teamRefusal(p, 'port', 1)).toBe('keptOff');
    expect(teamRefusal(p, 'autoStart', true)).toBe('appScope');
    expect(teamRefusal(p, 'nope', 1)).toBe('unknown');
    expect(
      teamRefusal(
        { ...p, manifest: { ...p.manifest!, contributes: { ...p.manifest!.contributes, secrets: [{ key: 'token' }] } } },
        'token',
        'x',
      ),
    ).toBe('secret');
    const imported = installedPlugin({
      id: 'dev.exemple.conf',
      version: '1.0.0',
      contributes: { settings: [{ key: 'conf', type: 'string', scope: 'project', multiline: true, import: ['.conf'] }] },
    });
    expect(teamRefusal(imported, 'conf', 'x')).toBe('imported');
    expect(teamRefusals(p, teamAtlas({}, { nope: 1 }, { port: 4444, routes: 'ok' }))).toEqual([
      { key: 'port', part: 'enforced', why: 'keptOff' },
      { key: 'nope', part: 'defaults', why: 'unknown' },
    ]);
  });

  it('shows an imposed setting greyed with « Imposé par l’équipe ACME », a recommended one with its provenance, and what it refuses', async () => {
    resetApp({ projects: [teamAtlas({ routes: '10.20.0.0/16' }, { routes: 'x' }, { routes: '172.16.0.0/12', port: 4444 })] });
    const view = tunnel({ projectSettings: { atlas: { profile: A, routes: '10.0.0.0/8' } } });
    fakeBackend({ plugins_list: () => [view], plugin_project_set: () => view });
    render(SettingsModal, { page: 'projectPlugins', projectId: 'atlas' });
    const group = await screen.findByRole('region', { name: 'Tunnel' });
    const routes = within(group).getByRole('group', { name: 'Routes par le tunnel' });
    expect(routes).toHaveTextContent('Imposé par l’équipe ACME');
    const input = within(routes).getByLabelText('Routes par le tunnel') as HTMLInputElement;
    expect(input.value).toBe('172.16.0.0/12');
    expect(input).toBeDisabled();
    // Nothing goes ahead of it: neither this machine's value, nor the file's, nor « Écrire dans .escouade.yml ».
    expect(routes).not.toHaveTextContent('Réglage local de cette machine');
    expect(within(routes).queryByRole('button', { name: 'Écrire dans .escouade.yml' })).toBeNull();
    expect(within(routes).queryByRole('button', { name: 'Reprendre la valeur du fichier' })).toBeNull();
    // A key its author keeps off the files: refused, with why.
    expect(group).toHaveTextContent('L’équipe ACME règle port : refusé, son auteur le garde hors des fichiers partagés.');
    const port = within(group).getByRole('group', { name: 'Port' });
    expect(port).not.toHaveTextContent('Imposé par l’équipe');
  });

  it('says a recommended value comes from the team, and takes it back after a value of this machine', async () => {
    resetApp({ projects: [teamAtlas({}, { routes: '10.1.0.0/16' }, {})] });
    let view = tunnel();
    const ipc = fakeBackend({ plugins_list: () => [view], plugin_project_set: () => view });
    render(SettingsModal, { page: 'projectPlugins', projectId: 'atlas' });
    const routes = await screen.findByRole('group', { name: 'Routes par le tunnel' });
    expect(routes).toHaveTextContent('Recommandé par l’équipe ACME');
    expect((within(routes).getByLabelText('Routes par le tunnel') as HTMLInputElement).value).toBe('10.1.0.0/16');
    expect(within(routes).getByLabelText('Routes par le tunnel')).toBeEnabled();
    view = tunnel({ projectSettings: { atlas: { routes: '10.0.0.0/8' } } });
    plugins.list = [view];
    const local = await within(screen.getByRole('region', { name: 'Tunnel' })).findByRole('button', {
      name: 'Reprendre la valeur de l’équipe ACME',
    });
    await userEvent.click(local);
    expect(ipc.called('plugin_project_set').at(-1)!.args).toMatchObject({ key: 'routes', clear: true });
  });
});

// What the managed device imposes on the settings of each project (rank 6, `plugins.projectSettings`, ESC-564): ahead of
// the team's, greyed with « Imposé par la stratégie de ce poste »; never a secret, a profile or what is kept off the files.
describe('what the managed device imposes on a plugin’s setting of each project', () => {
  const device = (values: Record<string, unknown>) =>
    setLocks({
      locks: {
        'plugins.projectSettings': {
          key: 'plugins.projectSettings',
          value: { 'dev.exemple.tunnel': values },
          by: { kind: 'device', source: 'HKLM' },
          bounds: [],
        },
      },
      rows: {},
      warnings: [],
    });

  it('ranks it over the team and this machine, never a value the plugin refuses', () => {
    const routes = projectFields(tunnel().manifest).find((f) => f.key === 'routes')!;
    const local = tunnel({ projectSettings: { atlas: { routes: '10.0.0.0/8' } } });
    device({ routes: '192.168.0.0/16' });
    expect(projectValue(local, teamAtlas({}, {}, { routes: '172.16.0.0/12' }), routes)).toMatchObject({
      value: '192.168.0.0/16',
      source: 'enforced',
      by: { kind: 'device' },
    });
    device({ routes: 12 });
    expect(projectValue(local, atlas(), routes)).toMatchObject({ source: 'local', value: '10.0.0.0/8' });
    setLocks(undefined);
    expect(projectValue(local, atlas(), routes)).toMatchObject({ source: 'local' });
  });

  it('shows the setting greyed with its padlock and « Imposé par la stratégie de ce poste », and what it refuses', async () => {
    device({ routes: '192.168.0.0/16', port: 4444 });
    resetApp({ projects: [atlas({ routes: '10.20.0.0/16' })] });
    const view = tunnel({ projectSettings: { atlas: { profile: A, routes: '10.0.0.0/8' } } });
    fakeBackend({ plugins_list: () => [view], plugin_project_set: () => view });
    render(SettingsModal, { page: 'projectPlugins', projectId: 'atlas' });
    const group = await screen.findByRole('region', { name: 'Tunnel' });
    const routes = within(group).getByRole('group', { name: 'Routes par le tunnel' });
    expect(within(routes).getByTestId('plugin-team-lock')).toHaveTextContent('Imposé par la stratégie de ce poste');
    const input = within(routes).getByLabelText('Routes par le tunnel') as HTMLInputElement;
    expect(input.value).toBe('192.168.0.0/16');
    expect(input).toBeDisabled();
    expect(within(routes).queryByRole('button', { name: 'Écrire dans .escouade.yml' })).toBeNull();
    expect(group).toHaveTextContent('La stratégie de ce poste règle port : refusé, son auteur le garde hors des fichiers partagés.');
    setLocks(undefined);
  });
});
