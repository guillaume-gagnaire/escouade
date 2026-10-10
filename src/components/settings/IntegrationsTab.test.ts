import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { AccountView } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));
const opened = vi.hoisted(() => [] as string[]);
vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl: vi.fn(async (u: string) => void opened.push(u)), openPath: vi.fn() }));

const off = (service: AccountView['service']): AccountView => ({ service, connected: false, label: '' });
const ACCOUNTS: AccountView[] = [
  { service: 'jira', connected: true, label: 'ada@atlas.dev · atlas.atlassian.net' },
  off('trello'),
  off('github'),
];

const JIRA_STATES = {
  states: [
    { id: '1', name: 'To Do' },
    { id: '3', name: 'In Progress' },
    { id: '10', name: 'In Review' },
  ],
  defaults: { doing: { id: '3', name: 'In Progress' }, review: { id: '3', name: 'In Progress' }, done: { id: '10', name: 'In Review' } },
};

const backend = (over: Record<string, (a: any) => unknown> = {}) =>
  fakeBackend({
    save_settings: () => [],
    integration_containers: (a: any) =>
      a.service === 'jira'
        ? [
            { id: 'ATL', name: 'ATL — Atlas' },
            { id: 'MOB', name: 'MOB — Mobile' },
          ]
        : [{ id: 'b1', name: 'Atlas — Backlog' }],
    integration_states: () => JIRA_STATES,
    ...over,
  });

const panel = () => screen.getByRole('tabpanel');
const save = () => userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));

describe('IntegrationsTab', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.accounts = ACCOUNTS;
    app.modal = { kind: 'settings', tab: 'integrations', projectId: 'p1' };
    opened.length = 0;
  });

  it('shows who is connected, and says what to do when no account is', async () => {
    backend();
    app.accounts = [off('jira'), off('trello'), off('github')];
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    expect(screen.getByRole('tab', { name: 'Intégrations' })).toHaveAttribute('aria-selected', 'true');
    expect(within(panel()).getAllByText('Non connecté')).toHaveLength(3);
    expect(within(panel()).getByText('Connecte un compte ci-dessus pour lier une source à ce projet.')).toBeInTheDocument();
    expect(within(panel()).queryByText('Correspondance des statuts')).not.toBeInTheDocument();
  });

  it('says which token stays in integrations.json when the system keychain is unavailable', () => {
    backend();
    app.accounts = [{ ...ACCOUNTS[0], inFile: true }, off('trello'), { service: 'github', connected: true, label: '@ada', inFile: false }];
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const notes = within(panel()).getAllByText('Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json.');
    expect(notes).toHaveLength(1);
    expect(notes[0].closest('.acct')).toHaveTextContent('Connecté · ada@atlas.dev · atlas.atlassian.net');
  });

  it('says which account the system keychain did not give its token at this start', () => {
    backend();
    app.accounts = [ACCOUNTS[0], off('trello'), { service: 'github', connected: true, label: '@work', unread: true }];
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const notes = within(panel()).getAllByText('Trousseau du système illisible : relance Escouade ou reconnecte le compte.');
    expect(notes).toHaveLength(1);
    expect(notes[0].closest('.acct')).toHaveTextContent('Connecté · @work');
  });

  it('connects an account once the service accepts it, and says why it refused', async () => {
    let refuse = true;
    const b = backend({
      integration_connect: (a: any) => {
        if (refuse) throw 'Trello refuse ces identifiants (401)';
        return { service: a.service, connected: true, label: '@ada' };
      },
    });
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const p = within(panel());
    const buttons = p.getAllByRole('button', { name: 'Connecter…' });
    await userEvent.click(buttons[0]);
    const form = p.getByRole('form', { name: 'Connexion à Trello' });
    await userEvent.type(within(form).getByLabelText("Clé d'API"), 'k1');
    await userEvent.type(within(form).getByLabelText('Jeton'), 'tok');
    await userEvent.click(within(form).getByRole('button', { name: 'Obtenir un jeton pour cette clé' }));
    expect(opened.at(-1)).toContain('key=k1');
    await userEvent.click(within(form).getByRole('button', { name: 'Connecter' }));
    expect(await within(form).findByRole('alert')).toHaveTextContent('Trello refuse ces identifiants (401)');
    refuse = false;
    await userEvent.click(within(form).getByRole('button', { name: 'Connecter' }));
    await waitFor(() => expect(p.getByText('Connecté · @ada')).toBeInTheDocument());
    expect(b.called('integration_connect').at(-1)!.args).toEqual({
      service: 'trello',
      account: { site: '', email: '', key: 'k1', token: 'tok' },
    });
    expect(app.accounts.find((a) => a.service === 'trello')?.connected).toBe(true);
    // Its boards are listed for the project.
    await waitFor(() => expect(b.called('integration_containers').some((c) => c.args.service === 'trello')).toBe(true));
  });

  it('disconnects an account at once', async () => {
    const b = backend({ integration_disconnect: () => [off('jira'), off('trello'), off('github')] });
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    await userEvent.click(within(panel()).getByRole('button', { name: 'Déconnecter' }));
    expect(b.called('integration_disconnect')[0].args).toEqual({ service: 'jira' });
    await waitFor(() => expect(within(panel()).getAllByText('Non connecté')).toHaveLength(3));
  });

  it('links a source with its default states, sets them by column, and saves them with the project', async () => {
    const b = backend();
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const p = within(panel());
    const source = p.getByRole('combobox', { name: 'Projet Jira' });
    await waitFor(() => expect(within(source).getByRole('option', { name: 'ATL — Atlas' })).toBeInTheDocument());
    await userEvent.selectOptions(source, 'ATL');
    expect(b.called('integration_states')[0].args).toEqual({ service: 'jira', container: 'ATL' });
    const done = await p.findByRole('combobox', { name: 'Jira — Terminé' });
    expect(done).toHaveValue('10');
    expect(p.getByRole('combobox', { name: 'Jira — En cours' })).toHaveValue('3');
    expect(p.getByRole('combobox', { name: 'Jira — À faire' })).toHaveValue('');
    await userEvent.selectOptions(p.getByRole('combobox', { name: 'Jira — À faire' }), '1');
    await userEvent.selectOptions(done, '');
    // Commented on arrival in « À tester » and « Terminé » by default.
    expect(p.getByRole('switch', { name: "Commenter à l'arrivée dans « À tester »" })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(p.getByRole('switch', { name: "Commenter à l'arrivée dans « En cours »" }));
    expect(screen.getByRole('tab', { name: /Intégrations/ })).toHaveClass('changed');
    await save();
    const sent = b.called('update_project')[0].args.project;
    expect(sent.integrations).toEqual({
      links: [
        {
          service: 'jira',
          container: 'ATL',
          name: 'ATL — Atlas',
          states: { todo: { id: '1', name: 'To Do' }, doing: { id: '3', name: 'In Progress' }, review: { id: '3', name: 'In Progress' } },
        },
      ],
      comments: ['review', 'done', 'doing'],
    });
  });

  it('links the source to the project it was chosen for, even if another is shown meanwhile', async () => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.accounts = ACCOUNTS;
    let arrive: (v: unknown) => void = () => {};
    const b = backend({ integration_states: () => new Promise((r) => (arrive = r)) });
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const source = within(panel()).getByRole('combobox', { name: 'Projet Jira' });
    await waitFor(() => expect(within(source).getByRole('option', { name: 'ATL — Atlas' })).toBeInTheDocument());
    await userEvent.selectOptions(source, 'ATL');
    await userEvent.click(screen.getByRole('group', { name: 'Projet' }).querySelector('button:nth-child(2)')!);
    arrive(JIRA_STATES);
    await waitFor(() => expect(screen.getByRole('tab', { name: /Intégrations/ })).toHaveClass('changed'));
    await save();
    const saved = b
      .called('update_project')
      .map((c) => [c.args.project.id, c.args.project.integrations.links.map((l: any) => l.container)]);
    expect(saved).toEqual([['p1', ['ATL']]]);
  });

  it('unlinks a source with « Aucun »', async () => {
    resetApp({
      projects: [
        project({ integrations: { links: [{ service: 'jira', container: 'ATL', name: 'ATL — Atlas', states: {} }], comments: [] } }),
      ],
    });
    app.accounts = ACCOUNTS;
    const b = backend();
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const p = within(panel());
    expect(p.getByRole('combobox', { name: 'Projet Jira' })).toHaveValue('ATL');
    expect(p.getByText('Correspondance des statuts')).toBeInTheDocument();
    await userEvent.selectOptions(p.getByRole('combobox', { name: 'Projet Jira' }), '');
    expect(p.queryByText('Correspondance des statuts')).not.toBeInTheDocument();
    await save();
    expect(b.called('update_project')[0].args.project.integrations.links).toEqual([]);
  });

  it('sets the sync and the automatic import for every project', async () => {
    const b = backend();
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const p = within(panel());
    await userEvent.click(p.getByRole('switch', { name: 'Importer les tickets étiquetés' }));
    const label = p.getByRole('textbox', { name: 'Étiquette' });
    await userEvent.clear(label);
    await userEvent.type(label, 'escouade');
    await userEvent.click(p.getByRole('button', { name: '60 min' }));
    await userEvent.click(p.getByRole('switch', { name: 'Publier un résumé à chaque boucle' }));
    await save();
    expect(b.called('save_settings')[0].args.settings.integrations).toEqual({
      syncStates: true,
      loopComments: true,
      extractCriteria: true,
      autoImport: true,
      importLabel: 'escouade',
      importEvery: 60,
    });
    expect(b.called('update_project')).toHaveLength(0);
  });
});
