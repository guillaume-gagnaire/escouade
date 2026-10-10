import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import type { AccountView } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import IntegrationsTab from './IntegrationsTab.svelte';

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

// The tab alone, in English (the window around it is tested above, in French).
describe('IntegrationsTab in English', () => {
  /** The project, linked to a Jira source or not, as the settings open on it. */
  const open = (links = false) => {
    const integrations = {
      links: links ? [{ service: 'jira' as const, container: 'ATL', name: 'ATL — Atlas', states: {} }] : [],
      comments: ['review' as const],
    };
    resetApp({ projects: [project({ integrations })] });
    settingsForm.open({ tab: 'integrations', projectId: 'p1' });
  };

  beforeEach(() => setLang('en'));

  it('writes the accounts, what each one says, and the form of one to connect', async () => {
    backend();
    open();
    app.accounts = [{ ...ACCOUNTS[0], inFile: true }, off('trello'), { service: 'github', connected: true, label: '@work', unread: true }];
    render(IntegrationsTab, { project: app.projects[0] });
    const p = within(document.body);
    expect(p.getByRole('heading', { name: 'Connected accounts' })).toBeInTheDocument();
    expect(p.getByText('Connected · ada@atlas.dev · atlas.atlassian.net')).toBeInTheDocument();
    expect(p.getByText('Not connected')).toBeInTheDocument();
    expect(p.getByText('System keychain unavailable: the token stays in ~/.escouade/integrations.json.')).toBeInTheDocument();
    expect(p.getByText('System keychain unreadable: restart Escouade or reconnect the account.')).toBeInTheDocument();
    expect(p.getAllByRole('button', { name: 'Disconnect' })).toHaveLength(2);
    await userEvent.click(p.getByRole('button', { name: 'Connect…' }));
    const form = p.getByRole('form', { name: 'Connect to Trello' });
    expect(within(form).getByLabelText('API key')).toBeInTheDocument();
    expect(within(form).getByLabelText('Token')).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Get a token for this key' })).toBeDisabled();
    expect(within(form).getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });

  it('writes the token of GitHub with its hint inside the sentence', async () => {
    backend();
    open();
    app.accounts = [off('jira'), off('trello'), off('github')];
    render(IntegrationsTab, { project: app.projects[0] });
    await userEvent.click(screen.getAllByRole('button', { name: 'Connect…' })[2]);
    const form = screen.getByRole('form', { name: 'Connect to GitHub Issues' });
    expect(within(form).getByLabelText('Token (empty: the one from gh)')).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Create a GitHub token' })).toBeInTheDocument();
  });

  it('writes the sources, the mapping of the statuses with the columns of the Kanban, and the sync', async () => {
    open(true);
    app.accounts = ACCOUNTS;
    backend();
    render(IntegrationsTab, { project: app.projects[0] });
    const p = within(document.body);
    expect(p.getByRole('heading', { name: 'Sources linked to demo-api' })).toBeInTheDocument();
    expect(p.getByText('Project')).toBeInTheDocument();
    const source = p.getByRole('combobox', { name: 'Jira project' });
    expect(within(source).getByRole('option', { name: 'None' })).toBeInTheDocument();
    expect(p.getByRole('heading', { name: 'Status mapping' })).toBeInTheDocument();
    expect(p.getByText('Comment')).toBeInTheDocument();
    const review = p.getByRole('combobox', { name: 'Jira — To review' });
    expect(within(review).getByRole('option', { name: '— unchanged' })).toBeInTheDocument();
    expect(p.getByRole('combobox', { name: 'Jira — Done' })).toBeInTheDocument();
    expect(p.getByRole('switch', { name: 'Comment when a ticket reaches “To review”' })).toHaveAttribute('aria-checked', 'true');
    expect(p.getByRole('heading', { name: 'Sync' })).toBeInTheDocument();
    expect(p.getByRole('switch', { name: 'Post a summary on every loop' })).toBeInTheDocument();
    expect(p.getByRole('switch', { name: 'Extract the acceptance criteria' })).toBeInTheDocument();
  });

  it('writes the automatic import', () => {
    backend();
    open();
    app.accounts = [off('jira'), off('trello'), off('github')];
    render(IntegrationsTab, { project: app.projects[0] });
    const p = within(document.body);
    expect(p.getByText('Connect an account above to link a source to this project.')).toBeInTheDocument();
    expect(p.getByRole('heading', { name: 'Automatic import' })).toBeInTheDocument();
    expect(p.getByRole('switch', { name: 'Import labeled tickets' })).toBeInTheDocument();
    expect(p.getByRole('textbox', { name: 'Label' })).toHaveValue('claude-ready');
    expect(
      within(p.getByRole('group', { name: 'Check every' }))
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['5 min', '15 min', '60 min']);
  });
});
