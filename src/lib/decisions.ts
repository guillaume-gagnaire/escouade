// The register of the decisions (ESC-259, docs/FACTORY.md « Les décisions : un registre, pas un document »): what its
// rows and its detail say of each decision, worked out from what the backend reads in `.escouade/decisions/`.
import { fDayMonth } from './format';
import { t } from './i18n';
import type { DecisionRef, DecisionState, DocsDecision, DocsTree } from './types';

/** The filter of the register: every decision, or those of a state. */
export type DecisionFilter = 'all' | DecisionState;
export const DECISION_FILTERS: DecisionFilter[] = ['all', 'proposed', 'accepted', 'superseded'];

/** A decision's number as the register writes it: « 014 ». */
export const decisionNo = (n: number) => String(n).padStart(3, '0');

/** How a decision is cited (ESC-303): its number once accepted, else its id (a proposal has no number). */
export const decisionRef = (d: DocsDecision): DecisionRef => d.number ?? d.id;

/** A citation as it is shown: « 014 » for a number, the id of a proposal as it is. */
export const refLabel = (r: DecisionRef) => (typeof r === 'number' ? decisionNo(r) : r);

/** A decision as it is shown: its number, else its id. */
export const decisionLabel = (d: DocsDecision) => refLabel(decisionRef(d));

/** A key that tells a decision apart in the register (a number and an id never meet: an id is never digits alone). */
export const decisionKey = (d: DocsDecision) => String(decisionRef(d));

/** Whether `r` cites `d`: its number, or its id. */
export const cites = (r: DecisionRef, d: DocsDecision) => (typeof r === 'number' ? d.number === r : d.id === r);

/** What the head of the register counts. */
export function decisionCounts(list: DocsDecision[]) {
  const of = (s: DecisionState) => list.filter((d) => d.state === s).length;
  return { all: list.length, accepted: of('accepted'), proposed: of('proposed'), superseded: of('superseded') };
}

/** The rows the filter keeps, the latest first: the proposals without a number (the latest day first), then by number. */
export function filterDecisions(list: DocsDecision[], filter: DecisionFilter): DocsDecision[] {
  return list
    .filter((d) => filter === 'all' || d.state === filter)
    .sort((a, b) =>
      a.number == null || b.number == null
        ? (a.number == null ? 0 : 1) - (b.number == null ? 0 : 1) || b.date.localeCompare(a.date) || a.id.localeCompare(b.id)
        : b.number - a.number,
    );
}

/** A day `YYYY-MM-DD` (or a time that starts with one) at midnight, local; null when it is none. */
function dayOf(date: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(date);
  return m ? new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])) : null;
}

/** A decision's day, short (`now` in milliseconds, as `app.now`): « Auj. », « Hier », else « 24/10 » (with its year when it is not this one's). */
export function decisionDay(date: string, now: number): string {
  const d = dayOf(date);
  if (!d) return date;
  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  const days = Math.round((today.getTime() - d.getTime()) / 86_400_000);
  if (days === 0) return t('docs.decisions.today');
  if (days === 1) return t('docs.decisions.yesterday');
  const dm = fDayMonth(d.getTime());
  return d.getFullYear() === today.getFullYear() ? dm : `${dm}/${String(d.getFullYear()).slice(2)}`;
}

/** The same in words for the detail's head: « aujourd’hui », « hier », « le 24/10 ». */
export function decisionDayWords(date: string, now: number): string {
  const short = decisionDay(date, now);
  if (short === t('docs.decisions.today')) return t('docs.decisions.todayWords');
  if (short === t('docs.decisions.yesterday')) return t('docs.decisions.yesterdayWords');
  return t('docs.decisions.onDay', { day: short });
}

/** Where it comes from, as its row says it: « Brainstorm « Tarif famille » », « Ticket DEM-31 », a document's name. */
export function decisionOrigin(d: DocsDecision): string {
  const ref = d.originRef.trim();
  switch (d.origin.trim().toLowerCase()) {
    case 'brainstorm':
      return ref ? t('docs.decisions.origin.brainstorm', { ref }) : t('docs.decisions.origin.brainstormAlone');
    case 'discussion':
      return ref ? t('docs.decisions.origin.discussion', { ref }) : t('docs.decisions.origin.discussionAlone');
    case 'ticket':
      return ref ? t('docs.decisions.origin.ticket', { ref }) : t('docs.decisions.origin.ticketAlone');
    case 'document':
      return ref || t('docs.decisions.origin.documentAlone');
    case '':
      return ref;
    default:
      return ref ? `${d.origin} · ${ref}` : d.origin;
  }
}

/** The pages it changed: those of its `changes`, else those it names, each with its lines added and removed when known. */
export function decisionChanges(d: DocsDecision, tree: DocsTree | null | undefined) {
  const titles = new Map<string, { page: string; subject: string }>();
  for (const s of tree?.subjects ?? []) for (const p of s.pages) titles.set(p.path, { page: p.title, subject: s.title });
  const rows = d.changes.length
    ? d.changes.map((c) => ({ path: c.page, added: c.added as number | null, removed: c.removed as number | null }))
    : d.pages.map((p) => ({ path: p, added: null, removed: null }));
  return rows.map((r) => ({
    ...r,
    title: titles.get(r.path)?.page ?? r.path,
    subject: titles.get(r.path)?.subject ?? r.path.split('/')[0],
    known: titles.has(r.path),
  }));
}

/** « A changé » in its row: the subjects it touched, each once, and for a replaced one, by which. */
export function decisionChangedLine(d: DocsDecision, tree: DocsTree | null | undefined): string {
  const subjects = [...new Set(decisionChanges(d, tree).map((c) => c.subject))].join(', ');
  if (d.state === 'superseded' && d.supersededBy != null) {
    const by = t('docs.decisions.byNumber', { n: decisionNo(d.supersededBy) });
    return subjects ? `${subjects} · ${by}` : by;
  }
  return subjects;
}

/** The proposals in the register that would replace `d` once accepted. */
export function replacementsOf(d: DocsDecision, list: DocsDecision[]): DocsDecision[] {
  const n = d.number;
  return n == null ? [] : list.filter((x) => x.state === 'proposed' && x.supersedes.includes(n));
}
