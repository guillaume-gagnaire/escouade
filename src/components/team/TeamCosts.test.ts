import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { fakeBackend, resetApp } from '../../test/ipc';
import { API, BOB, costsView, ME, record, ticketRecord, WEB } from '../../test/team-costs';
import TeamCosts from './TeamCosts.svelte';

// « Équipe › Coûts » (TS-16): the table by project, ticket, harness and model (by member when the team chose it), over
// the week, the month or a chosen period; the equivalent in API of a plan marked; the budgets; what goes up of the
// member, exactly, and the team's choice said.

const TODAY = new Date(2026, 10, 20);

function show(view = costsView()) {
  const backend = fakeBackend({ team_costs: () => view, team_costs_send: () => null });
  render(TeamCosts, { teamId: 'acme', onback: () => {}, today: TODAY });
  return backend;
}

function cells(row: HTMLElement): string[] {
  return within(row)
    .getAllByRole('cell')
    .map((c) => (c.textContent ?? '').replace(/\s+/g, ' ').trim());
}

describe('« Équipe › Coûts » (TS-16)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
  });

  it('shows the month by project, the equivalent in API marked, then the week and a chosen period', async () => {
    show(
      costsView({
        records: [
          record({ day: '2026-11-03', cost: 2, plan: true, quota: 71 }),
          record({ project: WEB, day: '2026-11-19', cost: 1 }),
          record({ day: '2026-10-10', cost: 50 }),
        ],
      }),
    );
    const table = await screen.findByRole('table');
    const api = within(table).getByRole('row', { name: /^api/ });
    expect(cells(api)).toEqual(['2,0 k', '≈ 2,00 $ équivalent API', '71 %']);
    expect(within(table).getByRole('row', { name: /^Total/ })).toHaveTextContent('≈ 3,00 $');
    // The week: api's 3 November is out.
    await userEvent.click(screen.getByRole('button', { name: 'Semaine' }));
    expect(within(screen.getByRole('table')).queryByRole('row', { name: /^api/ })).toBeNull();
    expect(within(screen.getByRole('table')).getByRole('row', { name: /^web/ })).toBeInTheDocument();
    // A chosen period: October.
    await userEvent.click(screen.getByRole('button', { name: 'Période choisie' }));
    const [from, to] = screen.getAllByDisplayValue('') as HTMLInputElement[];
    await userEvent.type(from, '2026-10-01');
    await userEvent.type(to, '2026-10-31');
    const october = within(screen.getByRole('table')).getByRole('row', { name: /^api/ });
    expect(cells(october)[1]).toBe('50,00 $');
  });

  it('groups by ticket and by harness and model; by member only when the team chose it', async () => {
    show(costsView({ records: [record()], tickets: [ticketRecord()] }));
    await screen.findByRole('table');
    expect(screen.queryByRole('button', { name: 'Par membre' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Par ticket' }));
    expect(screen.getByRole('row', { name: /^API-12 · api/ })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Par harness et modèle' }));
    expect(screen.getByRole('row', { name: /^claude · claude-opus-4/ })).toBeInTheDocument();
  });

  it('shows each member to an administrator of a team « par membre »', async () => {
    show(
      costsView({
        rules: { byMember: true, team: null, projects: {}, members: {}, stop: false },
        told: true,
        records: [record({ member: ME, mine: true, cost: 1 }), record({ member: BOB, cost: 4 })],
      }),
    );
    await screen.findByRole('table');
    await userEvent.click(screen.getByRole('button', { name: 'Par membre' }));
    expect(screen.getByRole('row', { name: new RegExp(`^Membre ${BOB.slice(0, 8)}`) })).toHaveTextContent('4,00 $');
    expect(screen.getByRole('row', { name: /^Toi/ })).toHaveTextContent('1,00 $');
  });

  it('shows a member itself, the summary of the projects, and what went of it, exactly', async () => {
    const sent = { hour: 1, days: [{ day: '2026-11-20', rows: [{ project: API, cost: 1 }] }] };
    const backend = show(
      costsView({
        admin: false,
        holds: false,
        records: [record({ member: ME, mine: true, day: '2026-11-20' })],
        summary: { month: '2026-11', at: 1, projects: { [API]: { tokens: 10, cost: 12 } }, team: { tokens: 10, cost: 12 } },
        sent,
        pending: ['2026-11-20'],
      }),
    );
    expect(await screen.findByText(/Tes propres totaux/)).toBeInTheDocument();
    expect(screen.getByText('Toute l’équipe')).toBeInTheDocument();
    const mine = screen.getByRole('region', { name: 'Ce qui remonte de toi' });
    expect(within(mine).getByText(/Tes totaux par jour, jamais ton travail/)).toBeInTheDocument();
    expect(within(mine).getByText('L’équipe ACME voit les totaux par projet : les tiens partent sans ton nom.')).toBeInTheDocument();
    expect(within(mine).getByText('1 jour attend d’être envoyé.')).toBeInTheDocument();
    await userEvent.click(within(mine).getByRole('button', { name: 'Voir ce qui est parti' }));
    expect(screen.getByTestId('team-costs-sent').textContent).toBe(JSON.stringify(sent, null, 2));
    await userEvent.click(within(mine).getByRole('button', { name: 'Envoyer maintenant' }));
    expect(backend.called('team_costs_send')).toHaveLength(1);
  });

  it('weighs the budgets of the month', async () => {
    show(
      costsView({
        rules: { byMember: false, team: 10, projects: { api: 5 }, members: {}, stop: true },
        budgets: [
          {
            scope: 'project',
            of: 'p1',
            name: 'api',
            spent: 5.2,
            budget: 5,
            pct: 104,
            text: 'Le projet api a dépensé 104 % de son budget du mois',
          },
          {
            scope: 'team',
            of: '',
            name: 'ACME',
            spent: 9.2,
            budget: 10,
            pct: 92,
            text: 'L’équipe ACME a dépensé 92 % de son budget du mois',
          },
        ],
      }),
    );
    expect(await screen.findByText('Le projet api a dépensé 104 % de son budget du mois')).toBeInTheDocument();
    expect(screen.getByText('9,20 $ sur 10,00 $')).toBeInTheDocument();
    expect(screen.getByText('Au-delà d’un budget, le pilote auto s’arrête (imposé par l’équipe).')).toBeInTheDocument();
  });

  it('says when no administrator drew the key yet', async () => {
    show(costsView({ admin: false, epoch: null }));
    expect(await screen.findByText(/Aucun administrateur n’a encore tiré la clé des coûts/)).toBeInTheDocument();
  });
});
