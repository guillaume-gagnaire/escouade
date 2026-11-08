import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import type { AccountView } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import ServicesPage from './ServicesPage.svelte';
import { afterGesture } from '../../test/settings';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));
const opened = vi.hoisted(() => [] as string[]);
vi.mock('../../lib/opener', () => ({ openUrl: vi.fn(async (u: string) => void opened.push(u)), openPath: vi.fn() }));

const off = (service: AccountView['service']): AccountView => ({ service, connected: false, label: '' });
const ACCOUNTS: AccountView[] = [
  { service: 'jira', connected: true, label: 'ada@atlas.dev · atlas.atlassian.net' },
  off('trello'),
  off('github'),
];

const backend = (over: Record<string, (a: any) => unknown> = {}) =>
  fakeBackend({
    save_settings: () => [],
    integration_containers: () => [],
    integration_states: () => ({ states: [], defaults: {} }),
    ...over,
  });

const panel = () => document.getElementById('settings-panel')!;
const nav = () => within(screen.getByRole('navigation', { name: 'Réglages' }));

// « Services connectés », a page of Escouade: the code hosts, the accounts of the ticket sources, the sync and the
// automatic import (design/settings-2-2/PROPOSITION.md §2, page 4).
describe('ServicesPage', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.accounts = ACCOUNTS;
    app.modal = { kind: 'settings', page: 'services' };
    opened.length = 0;
  });

  it('is a page of Escouade, in its teal, and the old « integrations » tab leads to it', () => {
    backend();
    render(SettingsModal, { page: 'services' });
    expect(nav().getByRole('button', { name: 'Services connectés' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('dialog', { name: 'Réglages' })).toHaveAttribute('data-scope', 'app');
    expect(screen.getByRole('heading', { level: 1, name: 'Services connectés' })).toBeInTheDocument();
    // What a project links is on its own page.
    expect(within(panel()).queryByRole('combobox', { name: 'Projet Jira' })).toBeNull();
    expect(within(panel()).queryByText('Correspondance des statuts')).toBeNull();
  });

  it('shows with no project at all', () => {
    resetApp({ projects: [] });
    app.accounts = [off('jira'), off('trello'), off('github')];
    backend();
    render(SettingsModal, { page: 'services' });
    expect(within(panel()).getAllByText('Non connecté')).toHaveLength(7);
    expect(within(panel()).getByRole('switch', { name: 'Importer les tickets étiquetés' })).toBeInTheDocument();
  });

  it('says which token stays in integrations.json when the system keychain is unavailable', () => {
    backend();
    app.accounts = [{ ...ACCOUNTS[0], inFile: true }, off('trello'), { service: 'github', connected: true, label: '@ada', inFile: false }];
    render(SettingsModal, { page: 'services' });
    const notes = within(panel()).getAllByText('Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json.');
    expect(notes).toHaveLength(1);
    expect(notes[0].closest('.acct')).toHaveTextContent('Connecté · ada@atlas.dev · atlas.atlassian.net');
  });

  it('says which account the system keychain did not give its token at this start', () => {
    backend();
    app.accounts = [ACCOUNTS[0], off('trello'), { service: 'github', connected: true, label: '@work', unread: true }];
    render(SettingsModal, { page: 'services' });
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
    render(SettingsModal, { page: 'services' });
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
    // Its boards are offered on the project's page « Tickets importés ».
    await userEvent.click(nav().getByRole('button', { name: 'demo-api' }));
    await userEvent.click(within(nav().getByRole('group', { name: 'demo-api' })).getByRole('button', { name: 'Tickets importés' }));
    await waitFor(() => expect(b.called('integration_containers').some((c) => c.args.service === 'trello')).toBe(true));
  });

  it('offers the plugins that serve an account, and says which one serves it (INT-6)', async () => {
    const servers = [
      { plugin: 'dev.escouade.trello', name: 'Escouade', firstHand: true },
      { plugin: 'acme.trello', name: 'Trello maison', firstHand: false },
    ];
    app.accounts = [
      { service: 'jira', connected: true, label: 'ada@atlas.dev', plugin: 'acme.jira-dc' },
      { ...off('trello'), servers },
      { ...off('github'), servers: [servers[0]] },
    ];
    const b = backend({ integration_connect: (a: any) => ({ service: a.service, connected: true, label: '@ada', plugin: 'acme.trello' }) });
    render(SettingsModal, { page: 'services' });
    const p = within(panel());
    expect(p.getByText('Servi par le plugin acme.jira-dc')).toBeInTheDocument();
    await userEvent.click(p.getAllByRole('button', { name: 'Connecter…' })[0]);
    const form = p.getByRole('form', { name: 'Connexion à Trello' });
    const by = within(form).getByLabelText('Servi par');
    expect(
      within(by)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['Escouade', 'Trello maison']);
    await userEvent.selectOptions(by, 'acme.trello');
    await userEvent.type(within(form).getByLabelText("Clé d'API"), 'k1');
    await userEvent.type(within(form).getByLabelText('Jeton'), 'tok');
    await userEvent.click(within(form).getByRole('button', { name: 'Connecter' }));
    await waitFor(() => expect(b.called('integration_connect')).toHaveLength(1));
    expect(b.called('integration_connect')[0].args.account).toEqual({
      site: '',
      email: '',
      key: 'k1',
      token: 'tok',
      plugin: 'acme.trello',
    });
  });

  it('disconnects an account at once', async () => {
    const b = backend({ integration_disconnect: () => [off('jira'), off('trello'), off('github')] });
    render(SettingsModal, { page: 'services' });
    await userEvent.click(within(panel()).getByRole('button', { name: 'Déconnecter' }));
    expect(b.called('integration_disconnect')[0].args).toEqual({ service: 'jira' });
    await waitFor(() => expect(within(panel()).getAllByText('Non connecté')).toHaveLength(7));
  });

  it('names its two parts apart, the code hosts then the sources of the tickets with their sync and import', () => {
    backend();
    render(SettingsModal, { page: 'services' });
    const p = within(panel());
    const hosts = p.getByRole('heading', { name: 'Hébergeurs de code' });
    const sources = p.getByRole('heading', { name: 'Sources de tickets' });
    expect(hosts.parentElement).toHaveTextContent('PR et MR des agents, CI et déploiements');
    expect(sources.parentElement).toHaveTextContent('Import et synchro des tickets');
    const accounts = p.getByRole('heading', { name: 'Comptes connectés' });
    const sync = p.getByRole('heading', { name: 'Synchronisation' });
    const auto = p.getByRole('heading', { name: 'Import automatique' });
    const after = (a: Element, b: Element) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
    expect([after(hosts, sources), after(sources, accounts), after(accounts, sync), after(sync, auto)]).toEqual([true, true, true, true]);
  });

  it('connects GitLab Issues through the GitLab code host, and shows it connected with it', async () => {
    const gitlab = {
      kind: 'gitlab',
      name: 'GitLab',
      requestWord: 'MR',
      defaultHost: 'gitlab.com',
      needsEmail: false,
      cli: 'glab',
      scopes: ['api'],
      capabilities: {},
    };
    const github = { ...gitlab, kind: 'github', name: 'GitHub', requestWord: 'PR', defaultHost: 'github.com', cli: 'gh' };
    const b = backend({ forges: () => ({ providers: [github, gitlab], accounts: [], unknown: [], issues: app.accounts }) });
    render(SettingsModal, { page: 'services' });
    const p = within(panel());
    const row = p.getAllByText('GitLab Issues')[0].closest('.acct') as HTMLElement;
    expect(row).toHaveTextContent('Non connecté');
    expect(row).toHaveTextContent('Compte de l’hébergeur GitLab : il se connecte et se déconnecte dans Hébergeurs de code.');
    await waitFor(() => expect(b.called('forges').length).toBeGreaterThan(0));
    // The same form as the code host: one GitLab account for both uses.
    await userEvent.click(within(row).getByRole('button', { name: 'Connecter GitLab…' }));
    const form = await p.findByRole('form', { name: 'Connexion à un hébergeur' });
    expect(within(form).getByLabelText('Hébergeur')).toHaveValue('gitlab');
    expect(within(form).getByLabelText('Adresse')).toHaveFocus();
    expect(b.called('integration_connect')).toHaveLength(0);
  });

  it('shows GitLab Issues and Azure Boards connected with their code host, without a disconnect of their own', () => {
    backend();
    app.accounts = [
      off('jira'),
      off('trello'),
      off('github'),
      { service: 'gitlab', connected: true, label: '@ada, @ada.l · git.acme.dev' },
      { service: 'azure', connected: true, label: 'ada@acme.dev · acme' },
    ];
    render(SettingsModal, { page: 'services' });
    const p = within(panel());
    const gitlab = p.getAllByText('GitLab Issues')[0].closest('.acct') as HTMLElement;
    expect(gitlab).toHaveTextContent('Connecté · @ada, @ada.l · git.acme.dev');
    expect(within(gitlab).queryByRole('button')).toBeNull();
    const azure = p.getAllByText('Azure Boards')[0].closest('.acct') as HTMLElement;
    expect(azure).toHaveTextContent('Connecté · ada@acme.dev · acme');
    expect(within(azure).queryByRole('button')).toBeNull();
  });

  it('connects Azure Boards through the Azure DevOps code host, the Work Items right asked', async () => {
    const azure = {
      kind: 'azure',
      name: 'Azure DevOps',
      requestWord: 'PR',
      defaultHost: 'dev.azure.com',
      addressHint: 'dev.azure.com/acme',
      needsEmail: false,
      cli: 'az',
      scopes: ['vso.code_write', 'vso.build_execute', 'vso.work_write'],
      capabilities: {},
    };
    const github = { ...azure, kind: 'github', name: 'GitHub', defaultHost: 'github.com', cli: 'gh' };
    const b = backend({ forges: () => ({ providers: [github, azure], accounts: [], unknown: [], issues: app.accounts }) });
    render(SettingsModal, { page: 'services' });
    const p = within(panel());
    const row = p.getAllByText('Azure Boards')[0].closest('.acct') as HTMLElement;
    expect(row).toHaveTextContent('Non connecté');
    expect(row).toHaveTextContent('Compte de l’hébergeur Azure DevOps : il se connecte et se déconnecte dans Hébergeurs de code.');
    expect(row).toHaveTextContent('Son jeton doit avoir le droit Work Items (lecture et écriture).');
    await waitFor(() => expect(b.called('forges').length).toBeGreaterThan(0));
    // The same form as the code host: one Azure DevOps account for the PRs and the work items.
    await userEvent.click(within(row).getByRole('button', { name: 'Connecter Azure DevOps…' }));
    const form = await p.findByRole('form', { name: 'Connexion à un hébergeur' });
    expect(within(form).getByLabelText('Hébergeur')).toHaveValue('azure');
    expect(within(form).getByLabelText('Adresse')).toHaveFocus();
    expect(b.called('integration_connect')).toHaveLength(0);
  });

  it('sets the sync and the automatic import for every project, the same keys as before', async () => {
    const b = backend();
    render(SettingsModal, { page: 'services' });
    const p = within(panel());
    await userEvent.click(p.getByRole('switch', { name: 'Importer les tickets étiquetés' }));
    const label = p.getByRole('textbox', { name: 'Étiquette' });
    await userEvent.clear(label);
    await userEvent.type(label, 'escouade');
    await userEvent.click(p.getByRole('button', { name: '60 min' }));
    await userEvent.click(p.getByRole('switch', { name: 'Publier un résumé à chaque boucle' }));
    await afterGesture();
    expect(b.called('save_settings').at(-1)!.args.settings.integrations).toEqual({
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

// The page alone, in English (the window around it is tested above, in French).
describe('ServicesPage in English', () => {
  beforeEach(() => {
    setLang('en');
    resetApp({ projects: [project()] });
    settingsForm.open({ page: 'services' });
  });

  it('writes the accounts, what each one says, and the form of one to connect', async () => {
    backend();
    app.accounts = [{ ...ACCOUNTS[0], inFile: true }, off('trello'), { service: 'github', connected: true, label: '@work', unread: true }];
    render(ServicesPage);
    const p = within(document.body);
    expect(p.getByRole('heading', { name: 'Connected accounts' })).toBeInTheDocument();
    expect(p.getByRole('heading', { name: 'Code hosts' })).toBeInTheDocument();
    expect(p.getByRole('heading', { name: 'Ticket sources' }).parentElement).toHaveTextContent(
      'Import and sync of tickets: an imported ticket follows its column there (status, comments).',
    );
    expect(p.getByText('The GitLab code host’s account: it is connected and disconnected in Code hosts.')).toBeInTheDocument();
    expect(p.getByRole('button', { name: 'Connect GitLab…' })).toBeInTheDocument();
    expect(p.getByText('Connected · ada@atlas.dev · atlas.atlassian.net')).toBeInTheDocument();
    expect(p.getAllByText('Not connected')).toHaveLength(5);
    expect(p.getByText('System keychain unavailable: the token stays in ~/.escouade/integrations.json.')).toBeInTheDocument();
    expect(p.getByText('System keychain unreadable: restart Escouade or reconnect the account.')).toBeInTheDocument();
    expect(p.getAllByRole('button', { name: 'Disconnect' })).toHaveLength(2);
    // Trello's first, then Linear's and ClickUp's.
    const [trello, linear, clickup] = p.getAllByRole('button', { name: 'Connect…' });
    await userEvent.click(trello);
    const form = p.getByRole('form', { name: 'Connect to Trello' });
    expect(within(form).getByLabelText('API key')).toBeInTheDocument();
    expect(within(form).getByLabelText('Token')).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Get a token for this key' })).toBeDisabled();
    expect(within(form).getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    // Linear: a personal API key alone.
    await userEvent.click(linear);
    const lin = p.getByRole('form', { name: 'Connect to Linear' });
    expect(within(lin).getByLabelText('API key')).toHaveAttribute('type', 'password');
    expect(within(lin).queryByLabelText('Token')).toBeNull();
    expect(within(lin).getByRole('button', { name: 'Create a Linear API key' })).toBeInTheDocument();
    // ClickUp: a personal API key alone too.
    await userEvent.click(clickup);
    const cu = p.getByRole('form', { name: 'Connect to ClickUp' });
    expect(within(cu).getByLabelText('API key')).toHaveAttribute('type', 'password');
    expect(within(cu).getByLabelText('API key')).toHaveAttribute('placeholder', 'pk_…');
    await userEvent.click(within(cu).getByRole('button', { name: 'Create a ClickUp API key' }));
    expect(opened.at(-1)).toBe('https://app.clickup.com/settings/apps');
  });

  it('writes the token of GitHub with its hint inside the sentence', async () => {
    backend();
    app.accounts = [off('jira'), off('trello'), off('github')];
    render(ServicesPage);
    await userEvent.click(screen.getAllByRole('button', { name: 'Connect…' })[2]);
    const form = screen.getByRole('form', { name: 'Connect to GitHub Issues' });
    expect(within(form).getByLabelText('Token (empty: the one from gh)')).toBeInTheDocument();
    expect(within(form).getByRole('button', { name: 'Create a GitHub token' })).toBeInTheDocument();
  });

  it('writes the sync and the automatic import', () => {
    backend();
    app.accounts = [off('jira'), off('trello'), off('github')];
    render(ServicesPage);
    const p = within(document.body);
    expect(p.getByRole('heading', { name: 'Sync' })).toBeInTheDocument();
    expect(p.getByRole('switch', { name: 'Post a summary on every loop' })).toBeInTheDocument();
    expect(p.getByRole('switch', { name: 'Extract the acceptance criteria' })).toBeInTheDocument();
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
