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
vi.mock('../../lib/opener', () => ({ openUrl: vi.fn(async (u: string) => void opened.push(u)), openPath: vi.fn() }));

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
    expect(within(panel()).getAllByText('Non connecté')).toHaveLength(7);
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
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
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
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    await userEvent.click(within(panel()).getByRole('button', { name: 'Déconnecter' }));
    expect(b.called('integration_disconnect')[0].args).toEqual({ service: 'jira' });
    await waitFor(() => expect(within(panel()).getAllByText('Non connecté')).toHaveLength(7));
  });

  it('names its two parts apart, the code hosts and the sources of the Kanban, and opens on the sources when asked', async () => {
    backend();
    const scrolled = vi.spyOn(Element.prototype, 'scrollIntoView');
    render(SettingsModal, { tab: 'integrations', projectId: 'p1', section: 'sources' });
    const p = within(panel());
    const hosts = p.getByRole('heading', { name: 'Hébergeurs de code' });
    const sources = p.getByRole('heading', { name: 'Sources du Kanban' });
    expect(hosts.parentElement).toHaveTextContent('PR et MR des agents, CI et déploiements');
    expect(sources.parentElement).toHaveTextContent('Import et synchro des tickets');
    // The accounts of the sources come under their own part.
    const accounts = p.getByRole('heading', { name: 'Comptes connectés' });
    expect(sources.compareDocumentPosition(accounts) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(hosts.compareDocumentPosition(sources) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    await expect.poll(() => scrolled.mock.contexts.length).toBeGreaterThan(0);
    expect(scrolled.mock.contexts.at(-1)).toBe(sources.parentElement);
    scrolled.mockRestore();
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
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
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

  it('lists the GitLab projects of every instance, without a disconnect of its own', async () => {
    const b = backend({
      integration_containers: (a: any) =>
        a.service === 'gitlab'
          ? [
              { id: 'gitlab.com/acme/back/api', name: 'acme/back/api (dépôt du projet)', own: true },
              { id: 'git.acme.dev/infra/deploy', name: 'infra/deploy', group: 'git.acme.dev' },
            ]
          : [],
    });
    app.accounts = [
      off('jira'),
      off('trello'),
      off('github'),
      { service: 'gitlab', connected: true, label: '@ada, @ada.l · git.acme.dev' },
    ];
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const p = within(panel());
    const row = p.getAllByText('GitLab Issues')[0].closest('.acct') as HTMLElement;
    expect(row).toHaveTextContent('Connecté · @ada, @ada.l · git.acme.dev');
    expect(within(row).queryByRole('button')).toBeNull();
    const source = p.getByRole('combobox', { name: 'Projet GitLab Issues' });
    await waitFor(() => expect(within(source).getByRole('option', { name: 'acme/back/api (dépôt du projet)' })).toBeInTheDocument());
    expect(within(source).getByRole('group', { name: 'git.acme.dev' })).toHaveTextContent('infra/deploy');
    await userEvent.selectOptions(source, 'gitlab.com/acme/back/api');
    expect(b.called('integration_states').at(-1)!.args).toEqual({ service: 'gitlab', container: 'gitlab.com/acme/back/api' });
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
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
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

  it('lists the projects of the Azure DevOps organization, the remote’s first, and links one with its states', async () => {
    const b = backend({
      integration_containers: (a: any) =>
        a.service === 'azure'
          ? [
              { id: 'acme/Web Shop', name: 'Web Shop (dépôt du projet)', own: true },
              { id: 'acme/Mobile', name: 'Mobile', group: 'acme' },
            ]
          : [],
      integration_states: () => ({
        states: [
          { id: 'New', name: 'New' },
          { id: 'Active', name: 'Active' },
          { id: 'Resolved', name: 'Resolved' },
          { id: 'Closed', name: 'Closed' },
        ],
        defaults: {
          doing: { id: 'Active', name: 'Active' },
          review: { id: 'Active', name: 'Active' },
          done: { id: 'Resolved', name: 'Resolved' },
        },
      }),
    });
    app.accounts = [off('jira'), off('trello'), off('github'), { service: 'azure', connected: true, label: 'ada@acme.dev · acme' }];
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const p = within(panel());
    const row = p.getAllByText('Azure Boards')[0].closest('.acct') as HTMLElement;
    expect(row).toHaveTextContent('Connecté · ada@acme.dev · acme');
    // Disconnected with its code host only.
    expect(within(row).queryByRole('button')).toBeNull();
    const source = p.getByRole('combobox', { name: 'Projet Azure Boards' });
    await waitFor(() => expect(within(source).getByRole('option', { name: 'Web Shop (dépôt du projet)' })).toBeInTheDocument());
    expect(within(source).getByRole('group', { name: 'acme' })).toHaveTextContent('Mobile');
    await userEvent.selectOptions(source, 'acme/Web Shop');
    expect(b.called('integration_states').at(-1)!.args).toEqual({ service: 'azure', container: 'acme/Web Shop' });
    expect(await p.findByRole('combobox', { name: 'Azure Boards — En cours' })).toHaveValue('Active');
    expect(p.getByRole('combobox', { name: 'Azure Boards — Terminé' })).toHaveValue('Resolved');
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

  it('lists the ClickUp lists by space and folder, and links one with the statuses of its own', async () => {
    app.accounts = [
      off('jira'),
      off('trello'),
      off('github'),
      off('linear'),
      { service: 'clickup', connected: true, label: 'Ada Lovelace' },
    ];
    const b = backend({
      integration_containers: () => [
        { id: 'l0', name: 'Backlog', group: 'Produit' },
        { id: 'l1', name: 'Sprint 12', group: 'Produit › Sprints' },
        { id: 'l2', name: 'Sprint 13', group: 'Produit › Sprints' },
      ],
      integration_states: () => ({
        states: [
          { id: 'to do', name: 'to do' },
          { id: 'in progress', name: 'in progress' },
          { id: 'review', name: 'review' },
          { id: 'complete', name: 'complete' },
        ],
        defaults: {
          doing: { id: 'in progress', name: 'in progress' },
          review: { id: 'in progress', name: 'in progress' },
          done: { id: 'review', name: 'review' },
        },
      }),
    });
    render(SettingsModal, { tab: 'integrations', projectId: 'p1' });
    const p = within(panel());
    expect(p.getByText('Connecté · Ada Lovelace')).toBeInTheDocument();
    const source = p.getByRole('combobox', { name: 'Liste ClickUp' });
    await waitFor(() => expect(within(source).getByRole('option', { name: 'Sprint 12' })).toBeInTheDocument());
    const groups = [...source.querySelectorAll('optgroup')].map((g) => [
      g.label,
      [...g.querySelectorAll('option')].map((o) => o.textContent),
    ]);
    expect(groups).toEqual([
      ['Produit', ['Backlog']],
      ['Produit › Sprints', ['Sprint 12', 'Sprint 13']],
    ]);
    await userEvent.selectOptions(source, 'l1');
    expect(b.called('integration_states')[0].args).toEqual({ service: 'clickup', container: 'l1' });
    expect(await p.findByRole('combobox', { name: 'ClickUp — Terminé' })).toHaveValue('review');
    expect(p.getByRole('combobox', { name: 'ClickUp — En cours' })).toHaveValue('in progress');
    await save();
    expect(b.called('update_project')[0].args.project.integrations.links[0]).toMatchObject({
      service: 'clickup',
      container: 'l1',
      name: 'Sprint 12',
    });
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
    expect(p.getByRole('heading', { name: 'Code hosts' })).toBeInTheDocument();
    expect(p.getByRole('heading', { name: 'Kanban sources' }).parentElement).toHaveTextContent(
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
