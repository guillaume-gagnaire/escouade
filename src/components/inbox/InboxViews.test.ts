// What Escouade puts in the box « À toi » of its own (ESC-513), in the box's screen (ESC-510): its views in the queue's
// groups, one opened in the panel with the gestures of its source, sent on the view the user read (`inbox_view_act`);
// « Plus tard »; one count with the requests and the agents stopped; nothing while the MCP server is off.
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { groups, inbox, inboxCount } from '../../lib/inbox/inbox.svelte';
import type { InboxItem, InboxViews } from '../../lib/inbox/views';
import { inboxViews } from '../../lib/inbox/views.svelte';
import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import type { PendingRequest, WindowAsk } from '../../lib/types';
import { agent, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import { windowAsk } from '../../test/inbox';
import ContextMenu from '../ContextMenu.svelte';
import InboxQueue from './InboxQueue.svelte';
import InboxScreen from './InboxScreen.svelte';
import InboxViewPane from './InboxViewPane.svelte';

vi.mock('../../lib/previews/pdf', () => ({ PdfPasswordError: class extends Error {}, openPdf: async () => null }));

const NOW = Date.now();
const MIN = 60_000;

const request = (over: Partial<PendingRequest> = {}): PendingRequest => ({
  id: 'req_perm',
  kind: 'permission',
  tool: 'Bash',
  arg: 'rm -rf build',
  description: 'Supprime le dossier build',
  reason: 'Commande destructive',
  questions: [],
  cut: false,
  defaultNo: false,
  ...over,
});

function view(over: Partial<InboxItem> = {}): InboxItem {
  return {
    key: 'req:a1:req_perm',
    source: 'permission',
    group: 'stopped',
    projectId: 'p1',
    projectName: 'portail',
    private: false,
    title: 'rm -rf build',
    since: NOW - 40 * MIN,
    agentId: 'a1',
    agentName: 'design-tableau',
    ticketId: null,
    ticketKey: null,
    snoozedUntil: null,
    request: request(),
    column: null,
    blocked: null,
    conflict: false,
    partial: false,
    decision: null,
    files: 0,
    number: null,
    url: null,
    word: null,
    ...over,
  };
}

const ticketView = (over: Partial<InboxItem> = {}) =>
  view({
    key: 'ticket:t1',
    source: 'ticketReview',
    group: 'today',
    title: 'Le formulaire d’adhésion',
    request: null,
    agentId: 'a2',
    agentName: 'form',
    ticketId: 't1',
    ticketKey: 'DEM-12',
    column: 'review',
    ...over,
  });

const decisionView = (over: Partial<InboxItem> = {}) =>
  view({
    key: 'decision:p1:trois-echeances',
    source: 'decision',
    group: 'earlier',
    title: 'Trois échéances',
    request: null,
    agentId: null,
    agentName: null,
    decision: 'trois-echeances',
    ...over,
  });

const docsView = () => decisionView({ key: 'docs:p1', source: 'docs', title: '', decision: null, files: 3 });

let backend: ReturnType<typeof fakeBackend>;
let views: InboxViews;
let asks: WindowAsk[];

beforeEach(() => {
  resetApp();
  setLang('fr');
  inbox.reset();
  inboxViews.reset();
  menu.close();
  app.projects = [project({ id: 'p1', name: 'portail' })];
  views = { enabled: true, views: [], blocking: 0, total: 0 };
  asks = [];
  backend = fakeBackend({
    inbox_list: () => ({ enabled: true, asks, open: asks.length }),
    inbox_views: () => views,
    inbox_view_act: () => null,
    inbox_view_snooze: () => null,
  });
});

/** The box shown, with its queue and its panel, holding `vs` and `as`. */
async function show(vs: InboxItem[], as: WindowAsk[] = []) {
  views = { enabled: true, views: vs, blocking: vs.filter((v) => v.group === 'stopped').length, total: vs.length };
  asks = as;
  app.settings.mcpEnabled = true;
  app.ui.view = 'inbox';
  await inbox.load();
  await inboxViews.load();
  render(InboxQueue);
  render(InboxScreen);
  render(ContextMenu);
  await tick();
}

const queue = () => screen.getByTestId('inbox-queue');
const rows = () => within(queue()).getAllByTestId('ivw-row');
const pane = () => screen.getByTestId('ivw-pane');
const acts = () => backend.called('inbox_view_act').map((c) => [c.args.key, c.args.act]);
const headings = () =>
  within(queue())
    .getAllByRole('heading', { level: 2 })
    .map((h) => h.textContent?.replace(/\s*\d+\s*$/, '').trim());

describe('the views in the queue of the box', () => {
  it('go in its groups: the blocking among the stopped, the tickets today, the decisions and the docs earlier', async () => {
    await show([view(), ticketView(), decisionView(), docsView()], [windowAsk({ id: 'a-1' })]);
    expect(headings()).toEqual(['Tes agents se sont arrêtés', 'Aujourd’hui', 'Plus tôt']);
    expect(rows().map((r) => r.dataset.source)).toEqual(['permission', 'ticketReview', 'decision', 'docs']);
    // A row says what, who, and its kind.
    expect(rows()[1]).toHaveTextContent('Le formulaire d’adhésion');
    expect(rows()[1]).toHaveTextContent('form');
    expect(rows()[1]).toHaveTextContent('Ticket à valider');
    expect(rows()[3]).toHaveTextContent('3 fichiers des docs à partager');
    // The request of an agent stays a request, among them.
    expect(within(queue()).getAllByTestId('inbox-row')).toHaveLength(1);
  });

  it('takes the place of its agent’s row among the stopped; an agent with no view keeps its own', async () => {
    app.agents = {
      a1: agent({ id: 'a1', projectId: 'p1', status: 'waiting', requests: [request()] }),
      a9: agent({
        id: 'a9',
        projectId: 'p1',
        name: 'nettoyeur',
        status: 'running',
        deleteAsk: { by: 'a1', at: NOW, worktree: false } as never,
      }),
    };
    await show([view()]);
    const stopped = within(queue())
      .getAllByRole('button')
      .filter((b) => b.dataset.testid === 'ivw-row' || b.dataset.testid === 'inbox-stopped');
    expect(stopped.map((b) => b.dataset.testid)).toEqual(['ivw-row', 'inbox-stopped']);
    expect(within(queue()).getByTestId('inbox-stopped')).toHaveTextContent('nettoyeur');
  });

  it('opens the first one in the panel, with the gestures of its source, and the bar leads to its place', async () => {
    await show([view(), ticketView()]);
    await waitFor(() => expect(pane().dataset.key).toBe('req:a1:req_perm'));
    expect(pane()).toHaveTextContent('Autorisation · l’agent s’est arrêté');
    expect(pane()).toHaveTextContent('rm -rf build');
    expect(pane()).toHaveTextContent('Commande destructive');
    expect(screen.getByTestId('ivw-source')).toHaveTextContent('Aller à l’agent');
    expect(screen.getByTestId('inbox-nth')).toHaveTextContent('1 sur 2');
    await userEvent.click(within(pane()).getByRole('button', { name: 'Autoriser' }));
    await waitFor(() => expect(acts()).toEqual([['req:a1:req_perm', { act: 'allow' }]]));
    // Done, the next row opens.
    await waitFor(() => expect(pane().dataset.key).toBe('ticket:t1'));
    expect(screen.getByTestId('ivw-source')).toHaveTextContent('Voir dans le Kanban');
  });

  it('puts a view off from the panel at the times of the box, and shows it in « Plus tard »', async () => {
    await show([decisionView(), decisionView({ key: 'decision:p1:b', title: 'Tarif famille', snoozedUntil: NOW + 3_600_000 })]);
    await waitFor(() => expect(pane().dataset.key).toBe('decision:p1:trois-echeances'));
    await userEvent.click(within(pane()).getByRole('button', { name: 'Plus tard' }));
    const items = menu.open?.items ?? [];
    expect(items.map((i) => i.label)).toContain('Demain, 9:00');
    items.find((i) => i.label === 'Demain, 9:00')?.onClick?.();
    await waitFor(() => expect(backend.called('inbox_view_snooze')).toHaveLength(1));
    expect(backend.called('inbox_view_snooze')[0].args.key).toBe('decision:p1:trois-echeances');
    expect(acts()).toEqual([]);
    // The one put off is in « Plus tard », folded with the requests put off.
    await userEvent.click(within(queue()).getByTestId('inbox-later'));
    expect(within(queue()).getByText('Tarif famille')).toBeInTheDocument();
  });

  it('never puts off a request its agent stopped for', async () => {
    await show([view()]);
    await waitFor(() => expect(pane().dataset.key).toBe('req:a1:req_perm'));
    expect(within(pane()).queryByRole('button', { name: 'Plus tard' })).toBeNull();
  });
});

describe('a view opened in the panel', () => {
  it('refuses a permission with a word for the agent', async () => {
    render(InboxViewPane, { view: view() });
    await userEvent.click(within(pane()).getByRole('button', { name: 'Refuser' }));
    await userEvent.type(screen.getByLabelText('Un mot pour l’agent (facultatif)'), 'pas maintenant');
    await userEvent.click(within(pane()).getByRole('button', { name: 'Refuser' }));
    await waitFor(() => expect(acts()).toEqual([['req:a1:req_perm', { act: 'deny', message: 'pas maintenant' }]]));
  });

  it('offers « Toujours autoriser » only when the card does', () => {
    const { unmount } = render(InboxViewPane, { view: view() });
    expect(screen.queryByRole('button', { name: 'Toujours autoriser' })).toBeNull();
    unmount();
    render(InboxViewPane, { view: view({ request: request({ canAlways: true }) }) });
    expect(screen.getByRole('button', { name: 'Toujours autoriser' })).toBeInTheDocument();
  });

  it('approves a plan', async () => {
    render(InboxViewPane, {
      view: view({ key: 'req:a1:req_plan', source: 'plan', title: '', request: request({ tool: 'ExitPlanMode', arg: '# Plan' }) }),
    });
    expect(pane()).toHaveTextContent('Approuver le plan');
    await userEvent.click(within(pane()).getByRole('button', { name: 'Approuver le plan' }));
    await waitFor(() => expect(acts()).toEqual([['req:a1:req_plan', { act: 'allow' }]]));
  });

  it('answers a question by its options, or sends one it cannot answer to the conversation', async () => {
    const { unmount } = render(InboxViewPane, {
      view: view({
        key: 'req:a1:req_question',
        source: 'question',
        title: 'Quelle base de données ?',
        request: request({
          id: 'req_question',
          kind: 'question',
          tool: 'AskUserQuestion',
          arg: '',
          questions: [{ question: 'Quelle base de données ?', options: ['PostgreSQL', 'SQLite'] }],
        }),
      }),
    });
    const send = screen.getByRole('button', { name: 'Répondre' });
    expect(send).toBeDisabled();
    await userEvent.click(screen.getByLabelText('SQLite'));
    await userEvent.click(send);
    await waitFor(() =>
      expect(acts()).toEqual([['req:a1:req_question', { act: 'answer', answers: { 'Quelle base de données ?': 'SQLite' } }]]),
    );
    unmount();
    render(InboxViewPane, {
      view: view({
        source: 'question',
        request: request({
          kind: 'question',
          questions: [
            { question: 'Q1', options: ['a'] },
            { question: 'Q2', options: ['b'] },
          ],
        }),
      }),
    });
    expect(screen.getByRole('button', { name: 'Répondre dans la conversation' })).toBeInTheDocument();
  });

  it('approves a ticket as the board does, or sends it back with what to change', async () => {
    app.tickets = { t1: ticket({ id: 't1', projectId: 'p1', column: 'review' }) };
    const { unmount } = render(InboxViewPane, { view: ticketView() });
    expect(pane()).toHaveTextContent('Ticket à valider');
    expect(pane()).toHaveTextContent('DEM-12');
    await userEvent.click(within(pane()).getByRole('button', { name: 'Valider' }));
    await waitFor(() => expect(acts()).toEqual([['ticket:t1', { act: 'approve' }]]));
    unmount();
    render(InboxViewPane, { view: ticketView() });
    await userEvent.click(screen.getByRole('button', { name: 'Renvoyer à l’agent' }));
    await userEvent.type(screen.getByRole('textbox'), 'le bouton est trop petit');
    await userEvent.click(screen.getByRole('button', { name: /^Renvoyer$/ }));
    await waitFor(() => expect(acts().at(-1)).toEqual(['ticket:t1', { act: 'reject', comment: 'le bouton est trop petit' }]));
  });

  it('settles a blocked ticket with the gestures of its card', async () => {
    render(InboxViewPane, {
      view: ticketView({ key: 'blocked:t1', source: 'ticketBlocked', blocked: 'Conflit avec main', conflict: true }),
    });
    expect(pane()).toHaveTextContent('Conflit avec main');
    expect(screen.getByRole('button', { name: 'Annuler' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'L’agent résout' }));
    await waitFor(() => expect(acts()).toEqual([['blocked:t1', { act: 'resolve' }]]));
  });

  it('accepts a decision, shares the docs, and opens a review on its host', async () => {
    const { unmount } = render(InboxViewPane, { view: decisionView() });
    await userEvent.click(screen.getByRole('button', { name: 'Accepter' }));
    unmount();
    const docs = render(InboxViewPane, { view: docsView() });
    expect(pane()).toHaveTextContent('3 fichiers des docs à partager');
    await userEvent.click(screen.getByRole('button', { name: 'Partager' }));
    docs.unmount();
    await waitFor(() =>
      expect(acts()).toEqual([
        ['decision:p1:trois-echeances', { act: 'accept' }],
        ['docs:p1', { act: 'share' }],
      ]),
    );
    render(InboxViewPane, {
      view: decisionView({
        key: 'review:p1::7',
        source: 'review',
        group: 'today',
        title: 'Ajoute le paiement',
        decision: null,
        number: 7,
        url: 'https://example.com/pr/7',
        word: 'MR',
      }),
    });
    expect(pane()).toHaveTextContent('MR #7 · Ajoute le paiement');
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir la MR' }));
    expect(acts()).toHaveLength(2);
  });

  it('speaks English', () => {
    setLang('en');
    render(InboxViewPane, { view: view() });
    expect(pane()).toHaveTextContent('Permission · the agent has stopped');
    expect(screen.getByRole('button', { name: 'Allow' })).toBeInTheDocument();
  });
});

describe('one count for the box', () => {
  it('counts the requests, the agents stopped once, and the views that are not theirs, none put off', () => {
    const stopped = [agent({ id: 'a1', status: 'waiting' })];
    const c = inboxCount([windowAsk({ id: 'a-1' })], stopped, [
      view(),
      view({ key: 'req:a1:req_2' }),
      ticketView(),
      decisionView({ snoozedUntil: NOW + MIN }),
    ]);
    // The two blocking views are the agent's, counted once with it.
    expect(c).toEqual({ asks: 1, stopped: 1, views: 1, total: 3 });
    const g = groups([], stopped, NOW, [view(), ticketView()]);
    expect(g.stopped.map((i) => i.kind)).toEqual(['view']);
  });
});

describe('the MCP server off', () => {
  it('shows nothing of the views', async () => {
    views = { enabled: false, views: [], blocking: 0, total: 0 };
    backend = fakeBackend({ inbox_list: () => ({ enabled: false, asks: [], open: 0 }), inbox_views: () => views });
    app.settings.mcpEnabled = false;
    await inbox.load();
    await inboxViews.load();
    expect(inbox.views).toEqual([]);
    expect(inbox.count.total).toBe(0);
  });

  it('reads the views again only while it is on, not at each token of an agent', async () => {
    const n = () => backend.called('inbox_views').length;
    const at = n();
    inboxViews.take({ type: 'ticket', ticket: ticket({ id: 't1' }) });
    await waitFor(() => expect(n()).toBe(at + 1));
    const a = { id: 'a1', status: 'running', requests: [] } as never;
    inboxViews.take({ type: 'agent', agent: a });
    await waitFor(() => expect(n()).toBe(at + 2));
    inboxViews.take({ type: 'agent', agent: a });
    await new Promise((r) => setTimeout(r, 200));
    expect(n()).toBe(at + 2);
  });
});
