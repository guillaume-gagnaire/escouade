import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { Account, AccountShare, AgentShare, StatsView, TicketShare } from '../lib/types';
import { fakeBackend, project, resetApp } from '../test/ipc';
import Stats from './Stats.svelte';

const view = (range: string): StatsView => ({
  range,
  buckets: [
    { label: '26/09', start: 1, input: 1000, cache: 50_000, output: 2000, cost: 1.5, prompts: 3 },
    { label: '27/09', start: 2, input: 500, cache: 20_000, output: 1000, cost: 0.5, prompts: 1 },
  ],
  tokens: 74_500,
  tokensPrev: 50_000,
  cost: 2,
  costAll: 12.4,
  firstTs: Date.UTC(2026, 5, 1),
  prompts: 4,
  byProject: [
    { key: 'p1', tokens: 60_000, cost: 1.6 },
    { key: 'gone', tokens: 14_500, cost: 0.4 },
  ],
  byModel: [{ key: 'claude-opus-5-5', tokens: 74_500, cost: 2 }],
  byAgent: [],
  byTicket: [],
  byAccount: [],
  accounts: ['principal'],
});

const agentShare = (i: number, over: Partial<AgentShare> = {}): AgentShare => ({
  agentId: `a${i}`,
  name: `agent-${i}`,
  projectId: 'p1',
  tokens: 1000 * i,
  cost: i / 10,
  ...over,
});

const ticketShare = (i: number, over: Partial<TicketShare> = {}): TicketShare => ({
  id: `t${i}`,
  key: `DEM-${i}`,
  title: `Ticket ${i}`,
  loops: 2,
  cost: i / 10,
  ...over,
});

describe('Stats', () => {
  beforeEach(() => {
    localStorage.clear();
    resetApp({ projects: [project()] });
  });

  it('shows the KPIs of the period', async () => {
    fakeBackend({ stats: (a: any) => view(a.range) });
    render(Stats);
    await screen.findAllByText('74,5 k');
    const tokens = within(screen.getByText('Tokens').parentElement!);
    expect(tokens.getByText('74,5 k')).toBeInTheDocument();
    expect(tokens.getByText('14 derniers jours · +49 %')).toBeInTheDocument();
    const kpi = (label: string) => within(screen.getByText(label).parentElement!);
    expect(kpi('Coût global').getByText(/^2,00/)).toBeInTheDocument();
    expect(kpi('Coût global').getByText(/12,40 .* depuis le 1 juin 2026/)).toBeInTheDocument();
    expect(kpi('Coût moyen / prompt').getByText(/^0,50/)).toBeInTheDocument();
    expect(kpi('Prompts').getByText('4')).toBeInTheDocument();
  });

  it('switches range and remembers it', async () => {
    const backend = fakeBackend({ stats: (a: any) => view(a.range) });
    render(Stats);
    await screen.findAllByText('74,5 k');
    await userEvent.click(screen.getByRole('button', { name: 'Mois' }));
    await waitFor(() => expect(backend.called('stats').at(-1)?.args.range).toBe('month'));
    expect(localStorage.getItem('escouade.statsRange')).toBe('month');
  });

  it('offers a table view of the buckets', async () => {
    fakeBackend({ stats: (a: any) => view(a.range) });
    render(Stats);
    await screen.findAllByText('74,5 k');
    await userEvent.click(screen.getByRole('button', { name: 'Tableau' }));
    const rows = screen.getAllByRole('row');
    expect(rows).toHaveLength(3);
    expect(rows[1].textContent).toContain('26/09');
    expect(rows[1].textContent).toContain('50,0 k');
  });

  it('names projects and models, including closed projects', async () => {
    fakeBackend({ stats: (a: any) => view(a.range) });
    render(Stats);
    expect(await screen.findByText('demo-api')).toBeInTheDocument();
    expect(screen.getByText('Projet fermé')).toBeInTheDocument();
    expect(screen.getByText('Opus 5.5')).toBeInTheDocument();
  });
});

describe('Stats by agent and by ticket', () => {
  beforeEach(() => {
    localStorage.clear();
    resetApp({ projects: [project()] });
  });

  /** The lines of a list, header apart, as the text of their cells. */
  const lines = (name: string) =>
    within(screen.getByRole('table', { name }))
      .getAllByRole('row')
      .slice(1)
      .map((r) =>
        within(r)
          .getAllByRole('cell')
          .map((c) => c.textContent),
      );

  it('lists the agents of the period with their project, tokens and cost', async () => {
    fakeBackend({
      stats: (a: any) => ({
        ...view(a.range),
        byAgent: [
          agentShare(3, { name: 'refacto-auth', tokens: 52_000, cost: 1.2 }),
          agentShare(2, { name: null, projectId: 'gone', tokens: 8000, cost: 0.31 }),
        ],
      }),
    });
    render(Stats);
    await screen.findByRole('table', { name: 'Par agent' });
    expect(lines('Par agent')).toEqual([
      ['refacto-auth', 'demo-api', '52,0 k', '1,20\u00a0$'],
      ['Agent supprimé', 'Projet fermé', '8,0 k', '0,31\u00a0$'],
    ]);
  });

  it('lists the tickets of the period with their loops and the cost of their agents', async () => {
    fakeBackend({
      stats: (a: any) => ({
        ...view(a.range),
        byTicket: [ticketShare(4, { loops: 3, cost: 2.5, title: 'Ajouter le login' }), ticketShare(1, { loops: 1, cost: 0.4 })],
      }),
    });
    render(Stats);
    await screen.findByRole('table', { name: 'Par ticket' });
    expect(lines('Par ticket')).toEqual([
      ['DEM-4', 'Ajouter le login', '3 boucles', '2,50\u00a0$'],
      ['DEM-1', 'Ticket 1', '1 boucle', '0,40\u00a0$'],
    ]);
  });

  it('says so when no agent and no ticket cost anything over the period', async () => {
    fakeBackend({ stats: (a: any) => view(a.range) });
    render(Stats);
    await screen.findByText('Aucun ticket sur la période.');
    // The projects and models of `view` have data: only the agents' section is empty.
    expect(screen.getByText('Aucune donnée sur la période.')).toBeInTheDocument();
    expect(screen.queryByRole('table', { name: 'Par agent' })).not.toBeInTheDocument();
  });

  it('shows 20 lines of each list, then everything on « Tout voir »', async () => {
    fakeBackend({
      stats: (a: any) => ({
        ...view(a.range),
        byAgent: Array.from({ length: 23 }, (_, i) => agentShare(i + 1)),
        byTicket: Array.from({ length: 21 }, (_, i) => ticketShare(i + 1)),
      }),
    });
    render(Stats);
    await screen.findByRole('table', { name: 'Par agent' });
    expect(lines('Par agent')).toHaveLength(20);
    expect(lines('Par ticket')).toHaveLength(20);
    const more = screen.getAllByRole('button', { name: /Tout voir/ });
    expect(more).toHaveLength(2);
    await userEvent.click(more[0]);
    expect(lines('Par agent')).toHaveLength(23);
    // The other list is not opened with it.
    expect(lines('Par ticket')).toHaveLength(20);
    expect(screen.getAllByRole('button', { name: /Tout voir/ })).toHaveLength(1);
  });

  it('tells assistive technologies whether a list is open, and closes it again with « Réduire »', async () => {
    fakeBackend({
      stats: (a: any) => ({
        ...view(a.range),
        byAgent: Array.from({ length: 23 }, (_, i) => agentShare(i + 1)),
        byTicket: Array.from({ length: 21 }, (_, i) => ticketShare(i + 1)),
      }),
    });
    render(Stats);
    await screen.findByRole('table', { name: 'Par agent' });
    const [agents, tickets] = screen.getAllByRole('button', { name: 'Tout voir' });
    expect(agents).toHaveAttribute('aria-expanded', 'false');
    expect(tickets).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(agents);
    const less = screen.getByRole('button', { name: 'Réduire' });
    expect(less).toHaveAttribute('aria-expanded', 'true');
    // The other list stays closed.
    expect(screen.getByRole('button', { name: 'Tout voir' })).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(less);
    expect(lines('Par agent')).toHaveLength(20);
    expect(screen.getAllByRole('button', { name: 'Tout voir' })).toHaveLength(2);
  });

  it('says the cost of a ticket is the one of the period, as its agents cost over it', async () => {
    fakeBackend({ stats: (a: any) => ({ ...view(a.range), byTicket: [ticketShare(1)] }) });
    render(Stats);
    const table = await screen.findByRole('table', { name: 'Par ticket' });
    expect(within(table).getByRole('columnheader', { name: 'Coût sur la période' })).toBeInTheDocument();
    expect(within(table).queryByRole('columnheader', { name: 'Coût' })).not.toBeInTheDocument();
  });

  it('offers no « Tout voir » for 20 lines or fewer', async () => {
    fakeBackend({
      stats: (a: any) => ({ ...view(a.range), byAgent: Array.from({ length: 20 }, (_, i) => agentShare(i + 1)) }),
    });
    render(Stats);
    await screen.findByRole('table', { name: 'Par agent' });
    expect(screen.queryByRole('button', { name: /Tout voir/ })).not.toBeInTheDocument();
  });

  it('follows the period like the other sections', async () => {
    fakeBackend({
      stats: (a: any) => ({
        ...view(a.range),
        byAgent: [agentShare(1, { name: a.range === 'month' ? 'agent-du-mois' : 'agent-du-jour' })],
      }),
    });
    render(Stats);
    await screen.findByText('agent-du-jour');
    await userEvent.click(screen.getByRole('button', { name: 'Mois' }));
    expect(await screen.findByText('agent-du-mois')).toBeInTheDocument();
    expect(screen.queryByText('agent-du-jour')).not.toBeInTheDocument();
  });

  it('reads the period again when the language changes: the backend names its steps', async () => {
    let lang = 'fr';
    fakeBackend({
      stats: (a: any) => {
        const v = view(a.range);
        if (lang === 'en') v.buckets[1].label = 'Sep 27';
        return v;
      },
    });
    render(Stats);
    await userEvent.click(await screen.findByRole('button', { name: 'Tableau' }));
    expect(await screen.findByText('27/09')).toBeInTheDocument();
    lang = 'en';
    setLang('en');
    expect(await screen.findByText('Sep 27')).toBeInTheDocument();
  });
});

const MAIN: Account = { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true };
const PRO: Account = { id: 'pro', name: 'Pro', configDir: 'C:\\claude\\pro', claudePath: '', active: true };

const accountShare = (account: string, over: Partial<AccountShare> = {}): AccountShare => ({
  account,
  tokens: 1000,
  cost: 1,
  turns: 3,
  agents: 2,
  ...over,
});

describe('Stats by account', () => {
  beforeEach(() => {
    localStorage.clear();
    resetApp({ projects: [project()] });
    app.settings.accounts = [MAIN, PRO];
  });

  /** Both accounts over the period: what the backend gives whichever one the view is for. */
  const together = (a: any): StatsView => ({
    ...view(a.range),
    accounts: ['principal', 'pro'],
    byAccount: [
      accountShare('principal', { tokens: 60_000, cost: 1.5, turns: 12, agents: 3 }),
      accountShare('pro', { tokens: 14_500, cost: 0.5, turns: 4, agents: 1 }),
    ],
  });

  /** The figures of the account asked for, the accounts side by side as ever. */
  const filtered = (a: any): StatsView =>
    a.account === 'pro'
      ? {
          ...together(a),
          tokens: 33_300,
          cost: 0.5,
          byProject: [{ key: 'p1', tokens: 33_300, cost: 0.5 }],
          byModel: [{ key: 'claude-sonnet-5', tokens: 33_300, cost: 0.5 }],
        }
      : together(a);

  const lines = (name: string) =>
    within(screen.getByRole('table', { name }))
      .getAllByRole('row')
      .slice(1)
      .map((r) =>
        within(r)
          .getAllByRole('cell')
          .map((c) => c.textContent),
      );

  const picker = () => screen.findByRole('combobox', { name: 'Compte' });

  it('offers every account and each one, every account being the choice until another is made', async () => {
    const backend = fakeBackend({ stats: together });
    render(Stats);
    const select = await picker();
    expect(
      within(select)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['Tous les comptes', 'Principal', 'Pro']);
    expect(select).toHaveValue('');
    await screen.findAllByText('74,5 k');
    expect(backend.called('stats').at(-1)?.args).toEqual({ range: 'day', account: null });
  });

  it('asks for the account chosen with the period, and for every account again when asked to', async () => {
    const backend = fakeBackend({ stats: together });
    render(Stats);
    const select = await picker();
    await userEvent.selectOptions(select, 'Pro');
    await waitFor(() => expect(backend.called('stats').at(-1)?.args).toEqual({ range: 'day', account: 'pro' }));
    // The period changes, the account stays.
    await userEvent.click(screen.getByRole('button', { name: 'Mois' }));
    await waitFor(() => expect(backend.called('stats').at(-1)?.args).toEqual({ range: 'month', account: 'pro' }));
    await userEvent.selectOptions(select, 'Tous les comptes');
    await waitFor(() => expect(backend.called('stats').at(-1)?.args).toEqual({ range: 'month', account: null }));
  });

  it('shows the figures of the account chosen', async () => {
    fakeBackend({ stats: filtered });
    render(Stats);
    await userEvent.selectOptions(await picker(), 'Pro');
    await screen.findAllByText('33,3 k');
    const kpis = within(document.querySelector<HTMLElement>('.kpis')!);
    expect(kpis.getByText('33,3 k')).toBeInTheDocument();
    expect(within(screen.getByText('Coût global').parentElement!).getByText(/^0,50/)).toBeInTheDocument();
    expect(screen.queryByText('74,5 k')).not.toBeInTheDocument();
  });

  it('puts the accounts side by side in « Par compte », cost, tokens, turns and agents, whichever account is chosen', async () => {
    fakeBackend({ stats: filtered });
    render(Stats);
    await screen.findByRole('table', { name: 'Par compte' });
    const head = within(screen.getByRole('table', { name: 'Par compte' }))
      .getAllByRole('columnheader')
      .map((h) => h.textContent);
    expect(head).toEqual(['Compte', 'Tokens', 'Coût', 'Tours', 'Agents']);
    const side = [
      ['Principal', '60,0 k', '1,50\u00a0$', '12', '3'],
      ['Pro', '14,5 k', '0,50\u00a0$', '4', '1'],
    ];
    expect(lines('Par compte')).toEqual(side);
    await userEvent.selectOptions(await picker(), 'Pro');
    await screen.findAllByText('33,3 k');
    expect(lines('Par compte')).toEqual(side);
    // The account the other views are for is marked.
    const rows = within(screen.getByRole('table', { name: 'Par compte' }))
      .getAllByRole('row')
      .slice(1);
    expect(rows.map((r) => r.getAttribute('aria-current'))).toEqual([null, 'true']);
  });

  it('says so when no account ran a turn in the period', async () => {
    fakeBackend({ stats: (a: any) => ({ ...together(a), byAccount: [] }) });
    render(Stats);
    await screen.findAllByText('74,5 k');
    expect(screen.queryByRole('table', { name: 'Par compte' })).not.toBeInTheDocument();
    expect(screen.getByText('Par compte')).toBeInTheDocument();
    // The agents' section, and now the accounts', hold nothing.
    expect(screen.getAllByText('Aucune donnée sur la période.')).toHaveLength(2);
  });

  it('shows neither the choice nor « Par compte » with a single account', async () => {
    app.settings.accounts = [MAIN];
    fakeBackend({ stats: (a: any) => ({ ...view(a.range), accounts: ['principal'], byAccount: [accountShare('principal')] }) });
    render(Stats);
    await screen.findAllByText('74,5 k');
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
    expect(screen.queryByText('Par compte')).not.toBeInTheDocument();
    expect(screen.queryByText('Tous les comptes')).not.toBeInTheDocument();
  });

  it('shows them for the turns of several accounts when the settings list one, naming the one that is gone', async () => {
    app.settings.accounts = [MAIN];
    fakeBackend({
      stats: (a: any) => ({
        ...view(a.range),
        accounts: ['ancien', 'principal'],
        byAccount: [accountShare('principal', { cost: 2 }), accountShare('ancien', { cost: 1 })],
      }),
    });
    render(Stats);
    const select = await picker();
    expect(
      within(select)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['Tous les comptes', 'Principal', 'Compte supprimé (ancien)']);
    await screen.findByRole('table', { name: 'Par compte' });
    expect(lines('Par compte').map((l) => l[0])).toEqual(['Principal', 'Compte supprimé (ancien)']);
  });

  it('writes it in English, Principal being Main while it keeps its name', async () => {
    fakeBackend({ stats: together });
    setLang('en');
    render(Stats);
    const select = await screen.findByRole('combobox', { name: 'Account' });
    expect(
      within(select)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['All accounts', 'Main', 'Pro']);
    await screen.findByRole('table', { name: 'By account' });
    const head = within(screen.getByRole('table', { name: 'By account' })).getAllByRole('columnheader');
    expect(head.map((h) => h.textContent)).toEqual(['Account', 'Tokens', 'Cost', 'Turns', 'Agents']);
    expect(lines('By account')[0]).toEqual(['Main', '60.0k', '$1.50', '12', '3']);
  });

  it('reads every account again when the one chosen cannot be any more', async () => {
    let gone = false;
    const backend = fakeBackend({
      stats: (a: any) => (gone ? { ...view(a.range), accounts: ['principal'], byAccount: [accountShare('principal')] } : together(a)),
    });
    render(Stats);
    await userEvent.selectOptions(await picker(), 'Pro');
    await waitFor(() => expect(backend.called('stats').at(-1)?.args.account).toBe('pro'));
    // Pro is removed, its history with it; the statistics are read again at the next turn.
    gone = true;
    app.settings.accounts = [MAIN];
    app.usage.todayCost = 5;
    await waitFor(() => expect(backend.called('stats').at(-1)?.args.account).toBeNull());
    await waitFor(() => expect(screen.queryByRole('combobox')).not.toBeInTheDocument());
  });

  it('shows the answer to the last question only', async () => {
    let answerPro: (v: StatsView) => void = () => {};
    const backend = fakeBackend({
      stats: (a: any) => (a.account === 'pro' ? new Promise<StatsView>((resolve) => (answerPro = resolve)) : together(a)),
    });
    render(Stats);
    const select = await picker();
    await screen.findAllByText('74,5 k');
    await userEvent.selectOptions(select, 'Pro');
    await waitFor(() => expect(backend.called('stats').at(-1)?.args.account).toBe('pro'));
    // Pro's answer is late: every account was asked for in the meantime.
    await userEvent.selectOptions(select, 'Tous les comptes');
    await waitFor(() => expect(backend.called('stats').at(-1)?.args.account).toBeNull());
    answerPro(filtered({ range: 'day', account: 'pro' }));
    await new Promise((r) => setTimeout(r, 40));
    expect(screen.queryByText('33,3 k')).not.toBeInTheDocument();
    expect(screen.getAllByText('74,5 k').length).toBeGreaterThan(0);
  });
});

describe('Stats errors', () => {
  it('says when the statistics cannot be read', async () => {
    resetApp({ projects: [project()] });
    fakeBackend({
      stats: () => {
        throw new Error('base verrouillée');
      },
    });
    render(Stats);
    expect(await screen.findByText(/base verrouillée/)).toBeInTheDocument();
    expect(screen.queryByText('Chargement…')).not.toBeInTheDocument();
  });
});

describe('Stats in English', () => {
  beforeEach(() => {
    localStorage.clear();
    resetApp({ projects: [project()] });
    setLang('en');
  });

  const lines = (name: string) =>
    within(screen.getByRole('table', { name }))
      .getAllByRole('row')
      .slice(1)
      .map((r) =>
        within(r)
          .getAllByRole('cell')
          .map((c) => c.textContent),
      );

  it('writes the title, the periods and the figures of the period in English', async () => {
    fakeBackend({ stats: (a: any) => view(a.range) });
    render(Stats);
    await screen.findAllByText('74.5k');
    expect(screen.getByText('Statistics')).toBeInTheDocument();
    // No agent yet: « 0 agents », the plural of English for zero.
    expect(screen.getByText('Agents started from the app · 1 project, 0 agents')).toBeInTheDocument();
    for (const name of ['Day', 'Week', 'Month']) expect(screen.getByRole('button', { name })).toBeInTheDocument();
    const kpi = (label: string) => within(screen.getByText(label).parentElement!);
    expect(kpi('Tokens').getByText('Last 14 days · +49%')).toBeInTheDocument();
    expect(kpi('Total cost').getByText('$2.00')).toBeInTheDocument();
    expect(kpi('Total cost').getByText(/\$12\.40 since June 1, 2026/)).toBeInTheDocument();
    expect(kpi('Average cost / prompt').getByText('$0.50')).toBeInTheDocument();
    expect(kpi('Average cost / prompt').getByText(/≈ 18\.6k tokens \/ prompt/)).toBeInTheDocument();
    expect(kpi('Prompts').getByText('2 per day on average')).toBeInTheDocument();
  });

  it('says when no prompt was sent in the period, in English', async () => {
    fakeBackend({ stats: (a: any) => ({ ...view(a.range), prompts: 0 }) });
    render(Stats);
    await screen.findAllByText('74.5k');
    const kpi = within(screen.getByText('Average cost / prompt').parentElement!);
    expect(kpi.getByText('—')).toBeInTheDocument();
    expect(kpi.getByText('no prompts in this period')).toBeInTheDocument();
  });

  it('names the step of the period, the series and the table in English', async () => {
    fakeBackend({ stats: (a: any) => view(a.range) });
    render(Stats);
    await screen.findAllByText('74.5k');
    expect(screen.getByText('Tokens per day')).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Tokens per day, stacked input, cache and output' })).toBeInTheDocument();
    for (const series of ['Input', 'Cache', 'Output']) expect(screen.getByText(series)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Month' }));
    expect(await screen.findByText('Tokens per month')).toBeInTheDocument();
    expect(await screen.findByText('Last 12 months · +49%')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Table' }));
    expect(screen.getAllByRole('columnheader').map((h) => h.textContent)).toEqual([
      'Period',
      'Input',
      'Cache',
      'Output',
      'Total',
      'Cost',
      'Prompts',
    ]);
    expect(screen.getByRole('button', { name: 'Chart' })).toBeInTheDocument();
  });

  it('writes the shares, the lists and their counts in English', async () => {
    fakeBackend({
      stats: (a: any) => ({
        ...view(a.range),
        byAgent: Array.from({ length: 23 }, (_, i) => agentShare(i + 1)).concat([agentShare(30, { name: null, projectId: 'gone' })]),
        byTicket: [ticketShare(4, { loops: 3, cost: 2.5 }), ticketShare(1, { loops: 1, cost: 0.4 })],
      }),
    });
    render(Stats);
    await screen.findByRole('table', { name: 'By agent' });
    expect(screen.getByText('By project')).toBeInTheDocument();
    expect(screen.getByText('By model')).toBeInTheDocument();
    expect(screen.getByText('Closed project', { selector: '.sname' })).toBeInTheDocument();
    expect(lines('By ticket')).toEqual([
      ['DEM-4', 'Ticket 4', '3 loops', '$2.50'],
      ['DEM-1', 'Ticket 1', '1 loop', '$0.40'],
    ]);
    expect(
      within(screen.getByRole('table', { name: 'By ticket' })).getByRole('columnheader', { name: 'Cost over the period' }),
    ).toBeInTheDocument();
    expect(lines('By agent')).toHaveLength(20);
    expect(screen.getByText('20 of 24')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Show all' }));
    expect(lines('By agent')).toHaveLength(24);
    expect(lines('By agent').at(-1)).toEqual(['Deleted agent', 'Closed project', '30.0k', '$3.00']);
    expect(screen.getByRole('button', { name: 'Show less' })).toHaveAttribute('aria-expanded', 'true');
  });

  it('says when a period holds nothing, and when the statistics cannot be read, in English', async () => {
    fakeBackend({ stats: (a: any) => ({ ...view(a.range), byProject: [], byModel: [] }) });
    const { unmount } = render(Stats);
    await screen.findByText('No tickets in this period.');
    expect(screen.getAllByText('No data for this period.')).toHaveLength(3);
    unmount();
    fakeBackend({
      stats: () => {
        throw new Error('database locked');
      },
    });
    render(Stats);
    expect(await screen.findByText(/Statistics unavailable: .*database locked/)).toBeInTheDocument();
  });
});
