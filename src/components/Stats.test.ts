import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import type { AgentShare, StatsView, TicketShare } from '../lib/types';
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
      ['refacto-auth', 'demo-api', '52,0 k', '1,20 $'],
      ['Agent supprimé', 'Projet fermé', '8,0 k', '0,31 $'],
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
      ['DEM-4', 'Ajouter le login', '3 boucles', '2,50 $'],
      ['DEM-1', 'Ticket 1', '1 boucle', '0,40 $'],
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
