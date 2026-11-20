// The costs and the quotas of a team (TS-16, `src-tauri/src/teamcosts.rs`): what the table of « Équipe › Coûts » reads,
// summed by member, project, ticket or harness and model over a period. Pure functions here, the screen in
// `components/team/TeamCosts.svelte`. A cost on a plan (Claude Max, ChatGPT Pro…) is no bill: an equivalent in API,
// marked as such.

import { invoke } from './transport';
import { fUsd } from './format';
import { t } from './i18n';

export interface CostRules {
  byMember: boolean;
  team: number | null;
  projects: Record<string, number>;
  members: Record<string, number>;
  stop: boolean;
}

export interface CostRecord {
  member: string | null;
  mine: boolean;
  project: string;
  /** `2026-11-20`. */
  day: string;
  harness: string;
  model: string;
  plan: boolean;
  input: number;
  output: number;
  cache: number;
  cost: number;
  quota: number | null;
}

export interface TicketRecord {
  member: string | null;
  project: string;
  day: string;
  ticket: string;
  tokens: number;
  cost: number;
}

export interface BudgetState {
  scope: 'team' | 'project' | 'member';
  of: string;
  name: string;
  spent: number;
  budget: number;
  pct: number;
  text: string;
}

export interface Spent {
  tokens: number;
  cost: number;
}

export interface Summary {
  month: string;
  at: number;
  projects: Record<string, Spent>;
  team: Spent;
}

/** The totals as they went, exactly (`escouade_team::costs::Totals`). */
export interface Totals {
  member?: string;
  hour: number;
  days: { day: string; rows: unknown[]; tickets?: unknown[] }[];
}

export interface TeamCostsView {
  team: string;
  name: string;
  admin: boolean;
  me: string;
  rules: CostRules;
  told: boolean;
  epoch: number | null;
  holds: boolean;
  stale: boolean;
  pending: string[];
  sent: Totals | null;
  records: CostRecord[];
  tickets: TicketRecord[];
  summary: Summary | null;
  projects: Record<string, string>;
  budgets: BudgetState[];
}

export interface CostNotice {
  text: string;
  pct: number;
  over: boolean;
  stops: boolean;
  team: string;
}

export const teamCostsApi = {
  get: (team: string) => invoke<TeamCostsView>('team_costs', { team }),
  ack: (team: string) => invoke<void>('team_costs_ack', { team }),
  untold: () => invoke<[string, string][]>('team_costs_untold'),
  notice: (project: string) => invoke<CostNotice | null>('team_cost_notice', { project }),
  send: () => invoke<void>('team_costs_send'),
};

// ---------- the periods ----------

export type Period = 'week' | 'month' | 'custom';

function iso(d: Date): string {
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** The days of a period, both ends in: the week (from Monday), the month (from the 1st), or `from`–`to`. */
export function periodBounds(period: Period, today: Date, from = '', to = ''): [string, string] {
  if (period === 'custom') {
    const a = from || iso(today);
    const b = to || iso(today);
    return a <= b ? [a, b] : [b, a];
  }
  if (period === 'month') return [iso(new Date(today.getFullYear(), today.getMonth(), 1)), iso(today)];
  const monday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - ((today.getDay() + 6) % 7));
  return [iso(monday), iso(today)];
}

export function within(day: string, [a, b]: [string, string]): boolean {
  return day >= a && day <= b;
}

// ---------- the sums ----------

export type By = 'member' | 'project' | 'ticket' | 'model';

export interface Line {
  key: string;
  label: string;
  tokens: number;
  cost: number;
  /** Some of it is on a plan: an equivalent in API, no bill. */
  plan: boolean;
  /** All of it is on a plan. */
  planOnly: boolean;
  /** The highest quota reached, when known. */
  quota: number | null;
  mine: boolean;
}

/** The dimensions a view offers: by member only when the team chose it and the device sees others. */
export function dimensions(v: Pick<TeamCostsView, 'admin' | 'rules'>): By[] {
  return v.admin && v.rules.byMember ? ['member', 'project', 'ticket', 'model'] : ['project', 'ticket', 'model'];
}

export function memberLabel(member: string | null, me: string): string {
  if (!member) return t('teamCosts.member.unnamed');
  return member === me ? t('teamCosts.member.me') : t('teamCosts.member.other', { id: member.slice(0, 8) });
}

/** The lines of the table for a dimension over a period, the most costly first. */
export function lines(v: TeamCostsView, by: By, bounds: [string, string]): Line[] {
  const out = new Map<string, Line & { paid: number }>();
  const add = (key: string, label: string, tokens: number, cost: number, plan: boolean, quota: number | null, mine: boolean) => {
    const l = out.get(key) ?? { key, label, tokens: 0, cost: 0, plan: false, planOnly: true, quota: null, mine: false, paid: 0 };
    l.tokens += tokens;
    l.cost += cost;
    l.plan ||= plan;
    l.planOnly &&= plan;
    if (quota !== null) l.quota = Math.max(l.quota ?? 0, quota);
    l.mine ||= mine;
    out.set(key, l);
  };
  if (by === 'ticket') {
    const plans = planned(v, bounds);
    for (const r of v.tickets.filter((r) => within(r.day, bounds))) {
      const label = `${r.ticket} · ${projectName(v, r.project)}`;
      add(`${r.project}|${r.ticket}`, label, r.tokens, r.cost, plans.has(r.project), null, r.member === v.me);
    }
  } else {
    for (const r of v.records.filter((r) => within(r.day, bounds))) {
      const tokens = r.input + r.output + r.cache;
      const [key, label] =
        by === 'member'
          ? [r.member ?? '', memberLabel(r.member, v.me)]
          : by === 'project'
            ? [r.project, projectName(v, r.project)]
            : [`${r.harness}|${r.model}`, `${r.harness} · ${r.model}`];
      add(key, label, tokens, r.cost, r.plan, r.quota, r.mine);
    }
  }
  return [...out.values()].map(({ paid: _paid, ...l }) => l).sort((a, b) => b.cost - a.cost || a.label.localeCompare(b.label));
}

/** The projects whose turns of the period ran on a plan (a ticket's total is marked by its project). */
function planned(v: TeamCostsView, bounds: [string, string]): Set<string> {
  return new Set(v.records.filter((r) => r.plan && within(r.day, bounds)).map((r) => r.project));
}

export function projectName(v: Pick<TeamCostsView, 'projects'>, journal: string): string {
  return v.projects[journal] ?? journal.slice(0, 8);
}

export function total(list: Line[]): { tokens: number; cost: number; plan: boolean } {
  const zero: { tokens: number; cost: number; plan: boolean } = { tokens: 0, cost: 0, plan: false };
  return list.reduce((s, l) => ({ tokens: s.tokens + l.tokens, cost: s.cost + l.cost, plan: s.plan || l.plan }), zero);
}

/** A cost as the table writes it: « 12,40 $ », or « ≈ 12,40 $ » for what ran on a plan, with its mark beside. */
export function fCostOf(cost: number, plan: boolean): string {
  return (plan ? '≈ ' : '') + fUsd(cost);
}

/** The mark beside a cost on a plan. */
export function planMark(line: Pick<Line, 'plan' | 'planOnly'>): string | null {
  if (!line.plan) return null;
  return line.planOnly ? t('teamCosts.plan.only') : t('teamCosts.plan.some');
}

/** « Par projet » or « par membre », as each member is told. */
export function grainLine(v: Pick<TeamCostsView, 'rules' | 'told' | 'name'>): string {
  if (!v.rules.byMember) return t('teamCosts.grain.project', { team: v.name });
  return v.told ? t('teamCosts.grain.member', { team: v.name }) : t('teamCosts.grain.memberUntold', { team: v.name });
}

/** What a start in a team's project is told of its budgets, as a sentence that asks (none under 80 %). */
export function noticeLine(n: CostNotice | null): string | null {
  if (!n) return null;
  return n.stops ? t('teamCosts.notice.launchStops', { text: n.text, team: n.team }) : t('teamCosts.notice.launchBody', { text: n.text });
}

/** The notice of a project read from the core, as a sentence; none when it cannot be read. */
export async function budgetAsk(project: string): Promise<string | null> {
  try {
    return noticeLine(await teamCostsApi.notice(project));
  } catch {
    return null;
  }
}
