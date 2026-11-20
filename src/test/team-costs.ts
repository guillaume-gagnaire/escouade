// What the tests of « Équipe › Coûts » (TS-16) build: a view of a team's costs and its lines.

import type { CostRecord, TeamCostsView, TicketRecord } from '../lib/team-costs';

export const ME = 'aa'.repeat(16);
export const BOB = 'bb'.repeat(16);
export const API = '11'.repeat(16);
export const WEB = '22'.repeat(16);

export function record(over: Partial<CostRecord> = {}): CostRecord {
  return {
    member: null,
    mine: false,
    project: API,
    day: '2026-11-18',
    harness: 'claude',
    model: 'claude-opus-4',
    plan: false,
    input: 1000,
    output: 200,
    cache: 800,
    cost: 1.5,
    quota: null,
    ...over,
  };
}

export function ticketRecord(over: Partial<TicketRecord> = {}): TicketRecord {
  return { member: null, project: API, day: '2026-11-18', ticket: 'API-12', tokens: 2000, cost: 1.5, ...over };
}

export function costsView(over: Partial<TeamCostsView> = {}): TeamCostsView {
  return {
    team: 'acme',
    name: 'ACME',
    admin: true,
    me: ME,
    rules: { byMember: false, team: null, projects: {}, members: {}, stop: false },
    told: false,
    epoch: 1,
    holds: true,
    stale: false,
    pending: [],
    sent: null,
    records: [],
    tickets: [],
    summary: null,
    projects: { [API]: 'api', [WEB]: 'web' },
    budgets: [],
    ...over,
  };
}
