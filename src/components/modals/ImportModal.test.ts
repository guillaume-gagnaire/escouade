import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { AccountView, ExternalIssue, IssuePage, ProjectIntegrations, Ticket } from '../../lib/types';
import { fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import ImportModal from './ImportModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const ACCOUNTS: AccountView[] = [
  { service: 'jira', connected: true, label: 'ada@atlas.dev · atlas.atlassian.net' },
  { service: 'trello', connected: true, label: '@ada' },
  { service: 'github', connected: false, label: '' },
];

const LINKED: ProjectIntegrations = {
  links: [
    { service: 'jira', container: 'ATL', name: 'ATL — Atlas', states: {} },
    { service: 'trello', container: 'b1', name: 'Atlas — Backlog', states: {} },
    // Its account is not connected: not offered.
    { service: 'github', container: 'acme/api', name: 'acme/api', states: {} },
  ],
  comments: [],
};

function issue(over: Partial<ExternalIssue> = {}): ExternalIssue {
  return {
    service: 'jira',
    id: 'ATL-1287',
    key: 'ATL-1287',
    title: 'Rafraîchir le token',
    kind: 'Story',
    meta: ['Haute', 'Ada L.', 'To Do'],
    url: 'https://atlas.atlassian.net/browse/ATL-1287',
    description: '',
    criteria: ['Refresh avant expiration', 'Tests unitaires'],
    container: 'ATL',
    imported: false,
    ...over,
  };
}

const JIRA: IssuePage = {
  issues: [
    issue(),
    issue({ id: 'ATL-1290', key: 'ATL-1290', title: 'Erreur 500 avec un +', kind: 'Bug', criteria: [] }),
    issue({ id: 'ATL-1301', key: 'ATL-1301', title: 'Masquer les tokens', imported: true }),
  ],
  filters: [
    { id: 'mine', label: 'Assignés à moi' },
    { id: 'sprint', label: 'Sprint actif' },
  ],
  next: null,
  total: null,
};
const TRELLO: IssuePage = {
  issues: [issue({ service: 'trello', id: 'c1', key: '#151', title: 'Exporter le journal', kind: 'Carte', container: 'b1' })],
  filters: [{ id: 'mine', label: 'Mes cartes' }],
  next: null,
  total: null,
};

const dialog = () => screen.getByRole('dialog', { name: 'Importer des tickets' });
const row = (name: RegExp) => within(dialog()).getByRole('checkbox', { name });

describe('ImportModal', () => {
  beforeEach(() => {
    resetApp({ projects: [project({ integrations: LINKED })] });
    app.accounts = ACCOUNTS;
    app.modal = { kind: 'import', projectId: 'p1' };
  });

  it('says there is no source to import from, and leads to the settings that link one', async () => {
    resetApp({ projects: [project()] });
    app.accounts = ACCOUNTS;
    fakeBackend();
    render(ImportModal, { projectId: 'p1' });
    expect(within(dialog()).getByText('Aucune source liée à ce projet')).toBeInTheDocument();
    await userEvent.click(within(dialog()).getByRole('button', { name: 'Lier une source' }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'integrations', projectId: 'p1' });
  });

  it('lists the first source’s tickets, with their kind, meta and criteria, those already there greyed', async () => {
    const backend = fakeBackend({ integration_issues: (a: any) => (a.service === 'jira' ? JIRA : TRELLO) });
    render(ImportModal, { projectId: 'p1' });
    const d = within(dialog());
    expect(d.getByText('Dans « À faire » du Kanban de demo-api')).toBeInTheDocument();
    const tabs = d.getAllByRole('tab');
    expect(tabs).toHaveLength(2);
    expect(tabs[0]).toHaveTextContent('JiraATL — Atlas');
    expect(tabs[1]).toHaveTextContent('TrelloAtlas — Backlog');
    await waitFor(() => expect(d.getByText('3 résultats')).toBeInTheDocument());
    expect(backend.called('integration_issues')[0].args).toEqual({ projectId: 'p1', service: 'jira', text: '', filters: [] });
    const first = row(/ATL-1287/);
    expect(first).toHaveTextContent('Story Haute · Ada L. · To Do');
    expect(within(first).getByText('✓ 2 critères détectés')).toHaveAttribute('title', 'Refresh avant expiration · Tests unitaires');
    expect(within(row(/ATL-1290/)).queryByText(/critère/)).not.toBeInTheDocument();
    expect(d.getByText('ada@atlas.dev · ATL — Atlas')).toBeInTheDocument();
    const already = row(/ATL-1301/);
    expect(already).toHaveAttribute('aria-disabled', 'true');
    expect(within(already).getByText('Déjà dans le Kanban')).toBeInTheDocument();
  });

  it('searches as typed and filters by its chips', async () => {
    const backend = fakeBackend({ integration_issues: () => JIRA });
    render(ImportModal, { projectId: 'p1' });
    const d = within(dialog());
    await waitFor(() => expect(d.getByText('3 résultats')).toBeInTheDocument());
    await userEvent.type(d.getByRole('textbox', { name: 'Rechercher' }), 'token');
    await waitFor(() => expect(backend.called('integration_issues').at(-1)!.args.text).toBe('token'));
    // Typed at once, searched once.
    expect(backend.called('integration_issues').filter((c) => c.args.text.startsWith('t'))).toHaveLength(1);
    await userEvent.click(d.getByRole('button', { name: 'Assignés à moi' }));
    await waitFor(() => expect(backend.called('integration_issues').at(-1)!.args.filters).toEqual(['mine']));
    expect(d.getByRole('button', { name: 'Assignés à moi' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('chooses tickets across sources and imports them with their loops', async () => {
    const made: Ticket[] = [
      ticket({
        id: 'n1',
        key: 'DEM-4',
        external: { service: 'jira', id: 'ATL-1287', key: 'ATL-1287', container: 'ATL', url: '', error: null },
      }),
      ticket({ id: 'n2', key: 'DEM-5', external: { service: 'trello', id: 'c1', key: '#151', container: 'b1', url: '', error: null } }),
    ];
    const backend = fakeBackend({
      integration_issues: (a: any) => (a.service === 'jira' ? JIRA : TRELLO),
      integration_import: () => made,
    });
    render(ImportModal, { projectId: 'p1' });
    const d = within(dialog());
    await waitFor(() => expect(d.getByText('3 résultats')).toBeInTheDocument());
    expect(d.getByRole('button', { name: 'Importer' })).toBeDisabled();
    // « Tout sélectionner » takes those not imported yet.
    await userEvent.click(d.getByRole('checkbox', { name: 'Tout sélectionner' }));
    expect(d.getByText('2 tickets sélectionnés')).toBeInTheDocument();
    await userEvent.click(row(/ATL-1290/));
    await userEvent.click(row(/ATL-1301/));
    expect(d.getByText('1 ticket sélectionné')).toBeInTheDocument();
    await userEvent.click(d.getByRole('tab', { name: /Trello/ }));
    await waitFor(() => expect(d.getByText('1 résultat')).toBeInTheDocument());
    row(/#151/).focus();
    await userEvent.keyboard(' ');
    expect(d.getByRole('tab', { name: /Jira/ })).toHaveTextContent('1');
    await userEvent.click(d.getByRole('button', { name: '8' }));
    await userEvent.click(d.getByRole('button', { name: 'Importer 2 tickets' }));
    const call = backend.called('integration_import')[0].args;
    expect(call.projectId).toBe('p1');
    expect(call.maxLoops).toBe(8);
    expect(call.issues.map((i: ExternalIssue) => i.key)).toEqual(['ATL-1287', '#151']);
    expect(app.modal).toBeNull();
    expect(app.tickets.n1.key).toBe('DEM-4');
    expect(app.toasts.at(-1)?.text).toBe('2 tickets importés depuis Jira et Trello');
  });

  it('shows more of a long list with « Afficher plus », and how many there are in all', async () => {
    const more: IssuePage = {
      ...JIRA,
      issues: [issue({ id: 'ATL-1400', key: 'ATL-1400', title: 'Paginer les exports' })],
      next: null,
      total: null,
    };
    const backend = fakeBackend({
      integration_issues: (a: any) => (a.page === 'p2' ? more : { ...JIRA, next: 'p2', total: 312 }),
    });
    render(ImportModal, { projectId: 'p1' });
    const d = within(dialog());
    await waitFor(() => expect(d.getByText('3 affichés sur 312')).toBeInTheDocument());
    await userEvent.click(d.getByRole('button', { name: 'Afficher plus' }));
    await waitFor(() => expect(row(/ATL-1400/)).toBeInTheDocument());
    expect(backend.called('integration_issues').at(-1)!.args).toEqual({
      projectId: 'p1',
      service: 'jira',
      text: '',
      filters: [],
      page: 'p2',
    });
    // The first ones stay, the next ones follow; all there, nothing more to ask for.
    expect(d.getAllByRole('checkbox', { name: /ATL-/ })).toHaveLength(4);
    expect(d.getByText('4 résultats')).toBeInTheDocument();
    expect(d.queryByRole('button', { name: 'Afficher plus' })).not.toBeInTheDocument();
    // Its button gone, the focus goes to the first ticket it brought.
    expect(row(/ATL-1400/)).toHaveFocus();
    // A new search starts again from the first page.
    await userEvent.type(d.getByRole('textbox', { name: 'Rechercher' }), 'token');
    await waitFor(() => expect(backend.called('integration_issues').at(-1)!.args.text).toBe('token'));
    expect(backend.called('integration_issues').at(-1)!.args.page).toBeUndefined();
    await waitFor(() => expect(d.queryByRole('checkbox', { name: /ATL-1400/ })).not.toBeInTheDocument());
  });

  it('says how many are shown when the service does not say how many there are', async () => {
    fakeBackend({ integration_issues: () => ({ ...JIRA, next: '2', total: null }) });
    render(ImportModal, { projectId: 'p1' });
    const d = within(dialog());
    await waitFor(() => expect(d.getByText('3 affichés')).toBeInTheDocument());
    expect(d.getByRole('button', { name: 'Afficher plus' })).toBeInTheDocument();
  });

  it('says what the service answered when it fails', async () => {
    fakeBackend({
      integration_issues: () => {
        throw 'Jira refuse ces identifiants (401)';
      },
    });
    render(ImportModal, { projectId: 'p1' });
    expect(await within(dialog()).findByRole('alert')).toHaveTextContent('Jira refuse ces identifiants (401)');
  });
});
