import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import type { AccountView } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import TicketsPage from './TicketsPage.svelte';
import { afterGesture } from '../../test/settings';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));
vi.mock('../../lib/opener', () => ({ openUrl: vi.fn(), openPath: vi.fn() }));

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

const panel = () => document.getElementById('settings-panel')!;
const nav = () => within(screen.getByRole('navigation', { name: 'Réglages' }));

// « Tickets importés », a page of a project: the sources it links and the mapping of their statuses
// (design/settings-2-2/PROPOSITION.md §2, page 5 of a project).
describe('TicketsPage', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.accounts = ACCOUNTS;
    app.modal = { kind: 'settings', page: 'tickets', projectId: 'p1' };
  });

  it('is a page of the project, in its colour, and the old link to the sources leads to it', () => {
    backend();
    render(SettingsModal, { page: 'tickets', projectId: 'p1' });
    const sub = nav().getByRole('group', { name: 'demo-api' });
    expect(within(sub).getByRole('button', { name: 'Tickets importés' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('dialog', { name: 'Réglages' })).toHaveAttribute('data-scope', 'project');
    // The accounts, the sync and the import are Escouade's.
    const p = within(panel());
    expect(p.queryByRole('heading', { name: 'Comptes connectés' })).toBeNull();
    expect(p.queryByRole('heading', { name: 'Synchronisation' })).toBeNull();
    expect(p.queryByRole('switch', { name: 'Importer les tickets étiquetés' })).toBeNull();
    expect(p.getByRole('heading', { name: 'Sources liées à demo-api' })).toBeInTheDocument();
  });

  it('says to connect an account first, and leads to « Services connectés »', async () => {
    backend();
    app.accounts = [off('jira'), off('trello'), off('github')];
    render(SettingsModal, { page: 'tickets', projectId: 'p1' });
    const p = within(panel());
    expect(p.getByText('Connecte un compte dans Services connectés pour lier une source à ce projet.')).toBeInTheDocument();
    expect(p.queryByText('Correspondance des statuts')).not.toBeInTheDocument();
    await userEvent.click(p.getByRole('button', { name: 'Ouvrir Services connectés' }));
    expect(nav().getByRole('button', { name: 'Services connectés' })).toHaveAttribute('aria-current', 'page');
    expect(within(panel()).getByRole('heading', { name: 'Comptes connectés' })).toBeInTheDocument();
  });

  it('links a source with its default states, sets them by column, and saves them with the project', async () => {
    const b = backend();
    render(SettingsModal, { page: 'tickets', projectId: 'p1' });
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
    await afterGesture();
    const sent = b.called('update_project').at(-1)!.args.project;
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
    expect(b.called('save_settings')).toHaveLength(0);
  });

  it('links the source to the project it was chosen for, even if another is shown meanwhile', async () => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.accounts = ACCOUNTS;
    let arrive: (v: unknown) => void = () => {};
    const b = backend({ integration_states: () => new Promise((r) => (arrive = r)) });
    render(SettingsModal, { page: 'tickets', projectId: 'p1' });
    const source = within(panel()).getByRole('combobox', { name: 'Projet Jira' });
    await waitFor(() => expect(within(source).getByRole('option', { name: 'ATL — Atlas' })).toBeInTheDocument());
    await userEvent.selectOptions(source, 'ATL');
    // Another project shown meanwhile: the source still goes to the first.
    await userEvent.click(nav().getByRole('button', { name: 'site' }));
    arrive(JIRA_STATES);
    await afterGesture();
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
    render(SettingsModal, { page: 'tickets', projectId: 'p1' });
    const p = within(panel());
    expect(p.getByRole('combobox', { name: 'Projet Jira' })).toHaveValue('ATL');
    expect(p.getByText('Correspondance des statuts')).toBeInTheDocument();
    await userEvent.selectOptions(p.getByRole('combobox', { name: 'Projet Jira' }), '');
    expect(p.queryByText('Correspondance des statuts')).not.toBeInTheDocument();
    await afterGesture();
    expect(b.called('update_project')[0].args.project.integrations.links).toEqual([]);
  });

  it('lists the GitLab projects of every instance', async () => {
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
    render(SettingsModal, { page: 'tickets', projectId: 'p1' });
    const p = within(panel());
    const source = p.getByRole('combobox', { name: 'Projet GitLab Issues' });
    await waitFor(() => expect(within(source).getByRole('option', { name: 'acme/back/api (dépôt du projet)' })).toBeInTheDocument());
    expect(within(source).getByRole('group', { name: 'git.acme.dev' })).toHaveTextContent('infra/deploy');
    await userEvent.selectOptions(source, 'gitlab.com/acme/back/api');
    expect(b.called('integration_states').at(-1)!.args).toEqual({ service: 'gitlab', container: 'gitlab.com/acme/back/api' });
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
    render(SettingsModal, { page: 'tickets', projectId: 'p1' });
    const p = within(panel());
    const source = p.getByRole('combobox', { name: 'Projet Azure Boards' });
    await waitFor(() => expect(within(source).getByRole('option', { name: 'Web Shop (dépôt du projet)' })).toBeInTheDocument());
    expect(within(source).getByRole('group', { name: 'acme' })).toHaveTextContent('Mobile');
    await userEvent.selectOptions(source, 'acme/Web Shop');
    expect(b.called('integration_states').at(-1)!.args).toEqual({ service: 'azure', container: 'acme/Web Shop' });
    expect(await p.findByRole('combobox', { name: 'Azure Boards — En cours' })).toHaveValue('Active');
    expect(p.getByRole('combobox', { name: 'Azure Boards — Terminé' })).toHaveValue('Resolved');
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
    render(SettingsModal, { page: 'tickets', projectId: 'p1' });
    const p = within(panel());
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
    await afterGesture();
    expect(b.called('update_project')[0].args.project.integrations.links[0]).toMatchObject({
      service: 'clickup',
      container: 'l1',
      name: 'Sprint 12',
    });
  });
});

// The page alone, in English (the window around it is tested above, in French).
describe('TicketsPage in English', () => {
  /** The project, linked to a Jira source or not, as the settings open on it. */
  const open = (links = false) => {
    const integrations = {
      links: links ? [{ service: 'jira' as const, container: 'ATL', name: 'ATL — Atlas', states: {} }] : [],
      comments: ['review' as const],
    };
    resetApp({ projects: [project({ integrations })] });
    settingsForm.open({ page: 'tickets', projectId: 'p1' });
  };

  beforeEach(() => setLang('en'));

  it('writes the sources and the mapping of the statuses with the columns of the Kanban', () => {
    open(true);
    app.accounts = ACCOUNTS;
    backend();
    render(TicketsPage, { project: app.projects[0] });
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
  });

  it('says to connect an account in Connected services first', () => {
    open();
    app.accounts = [off('jira'), off('trello'), off('github')];
    backend();
    render(TicketsPage, { project: app.projects[0] });
    expect(screen.getByText('Connect an account in Connected services to link a source to this project.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open Connected services' })).toBeInTheDocument();
  });
});
