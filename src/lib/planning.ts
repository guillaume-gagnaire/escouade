// « Planifier » (ESC-262, docs/FACTORY.md « « Planifier » : de la spec aux tickets »): the changes of the spec without
// tickets, whether a discussion has some to plan, and the names of the pages a ticket implements.
import type { Discussion, DocsPlan, DocsPlanMade, DocsPlanning, DocsTree, Ticket } from './types';

/** How many changes of the spec have no tickets yet: the pages changed and the decisions proposed. */
export function unplannedCount(p: DocsPlanning | null | undefined): number {
  return p ? p.pages.length + p.decisions.length : 0;
}

/** Whether « Planifier » is offered on a discussion: it poured changes, none planned since. */
export function canPlan(d: Discussion): boolean {
  return !!d.poured && (!d.planned || d.poured.at > d.planned.at);
}

/** The title of a page of the spec, as the tree gives it; else the last part of its path. */
export function pageTitle(path: string, tree: DocsTree | undefined): string {
  for (const s of tree?.subjects ?? []) {
    const p = s.pages.find((x) => x.path === path);
    if (p?.title) return p.title;
  }
  return path.split('/').pop() || path;
}

/** The keys of the tickets of a subject's pages, in their order, each once. */
export function subjectTickets(pages: { tickets: string[] }[]): string[] {
  const out: string[] = [];
  for (const p of pages) for (const k of p.tickets) if (!out.includes(k)) out.push(k);
  return out;
}

/** Whether a ticket has started: as « Annuler » of a planning sees it, left in « À faire » with no agent and no start. */
export function ticketStarted(t: Pick<Ticket, 'column' | 'startedAt' | 'agentId'>): boolean {
  return t.column !== 'todo' || t.startedAt != null || t.agentId != null;
}

/**
 * Where the tickets of a planning stand (ESC-291): `none` made nothing, `cancelled` undone, `gone` all removed from the
 * board, `todo` none started, `partial` some, `started` all those left. `toCancel` are those « Annuler » still removes.
 */
export type PlanState = 'none' | 'cancelled' | 'gone' | 'todo' | 'partial' | 'started';
export function planProgress(
  plan: DocsPlan,
  tickets: Record<string, Ticket>,
): { state: PlanState; left: DocsPlanMade[]; started: DocsPlanMade[]; toCancel: DocsPlanMade[] } {
  const left = plan.made.filter((m) => tickets[m.id]);
  const started = left.filter((m) => ticketStarted(tickets[m.id]));
  const toCancel = plan.cancelled ? [] : left.filter((m) => !ticketStarted(tickets[m.id]));
  const state: PlanState = !plan.made.length
    ? 'none'
    : plan.cancelled
      ? 'cancelled'
      : !left.length
        ? 'gone'
        : !started.length
          ? 'todo'
          : toCancel.length
            ? 'partial'
            : 'started';
  return { state, left, started, toCancel };
}
