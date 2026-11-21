// The filter of the box « À toi » by type of request (ESC-578, docs/INBOX.md § 10.2): which type a row is, how the types
// are gathered, how many of each wait, which rows a filter keeps, what it hides, and how the choice is kept on this
// machine. Plain functions with no state of the window's, so that the mobile app (MO-5, ESC-571) reads the same box the
// same way.

import type { Agent, AskKind } from '../types';
import type { InboxItem, ViewSource } from './views';

/**
 * The types the filter offers, in the order it shows them: what stops an agent first (permissions, questions, plans),
 * then what the agents leave without stopping (choices, go-aheads, tasks), then what Escouade puts in of its own
 * (tickets, decisions, reviews, docs). Gathered where the user would not tell them apart:
 * - « Autorisations »: an agent's permission, and a deletion an agent asks for (a yes or no on what it would do);
 * - « Questions »: an agent's blocking question, a request for a free answer (`answer`) and a plain yes or no
 *   (`confirm` without `gate`): what the agent asks, whatever the form of the reply;
 * - « Feux verts »: a `confirm` with `gate` stays apart, the only one the user alone may answer (publish, sign, spend);
 * - « Tickets »: a ticket to approve and a blocked ticket, both settled on the ticket.
 */
export const INBOX_TYPES = ['permission', 'question', 'plan', 'choice', 'gate', 'task', 'tickets', 'decision', 'review', 'docs'] as const;
export type InboxType = (typeof INBOX_TYPES)[number];

/** What a blocking request waits for, as its row says it. */
export function stoppedWhat(a: Pick<Agent, 'requests' | 'deleteAsk'>): 'permission' | 'question' | 'plan' | 'delete' {
  const r = a.requests?.[0];
  if (!r) return a.deleteAsk ? 'delete' : 'permission';
  if (r.kind === 'question') return 'question';
  return r.tool === 'ExitPlanMode' ? 'plan' : 'permission';
}

/** The type of an agent's request (`ask_user`). */
export function typeOfAsk(a: { kind: AskKind; gate: boolean }): InboxType {
  if (a.kind === 'choice') return 'choice';
  if (a.kind === 'task') return 'task';
  if (a.kind === 'confirm' && a.gate) return 'gate';
  return 'question';
}

const VIEW_TYPE: Record<ViewSource, InboxType> = {
  permission: 'permission',
  question: 'question',
  plan: 'plan',
  ticketReview: 'tickets',
  ticketBlocked: 'tickets',
  decision: 'decision',
  docs: 'docs',
  review: 'review',
};

/** The type of what Escouade puts in the box (a view). */
export const typeOfView = (v: Pick<InboxItem, 'source'>): InboxType => VIEW_TYPE[v.source];

/** A row of the box, whatever the window draws of it. */
export type FilterRow =
  | { kind: 'stopped'; agent: Pick<Agent, 'requests' | 'deleteAsk'> }
  | { kind: 'ask'; ask: { kind: AskKind; gate: boolean } }
  | { kind: 'view'; view: Pick<InboxItem, 'source'> };

/** The type of a row. */
export function typeOf(r: FilterRow): InboxType {
  if (r.kind === 'ask') return typeOfAsk(r.ask);
  if (r.kind === 'view') return typeOfView(r.view);
  const what = stoppedWhat(r.agent);
  return what === 'delete' ? 'permission' : what;
}

/** No type chosen: everything shows (« Tout »). */
export const isAll = (chosen: readonly InboxType[]) => chosen.length === 0;

/** Whether a row of that type shows under the filter. */
export const shows = (chosen: readonly InboxType[], type: InboxType) => isAll(chosen) || chosen.includes(type);

/** The rows a filter keeps, in their order. */
export const keep = <R extends FilterRow>(rows: readonly R[], chosen: readonly InboxType[]): R[] =>
  isAll(chosen) ? [...rows] : rows.filter((r) => chosen.includes(typeOf(r)));

/** The rows that wait and that the filter hides: what « N autres masquées par le filtre » counts. */
export const hiddenCount = (rows: readonly FilterRow[], chosen: readonly InboxType[]) =>
  isAll(chosen) ? 0 : rows.filter((r) => !chosen.includes(typeOf(r))).length;

export interface TypeCount {
  type: InboxType;
  /** The rows of that type that wait. */
  count: number;
  /** Chosen in the filter. */
  on: boolean;
}

/**
 * The types the filter shows, in their order, each with the number of rows of it that wait: a type with none is left
 * out, unless it is chosen (so that it can be taken off once its last request is answered).
 */
export function typeCounts(rows: readonly FilterRow[], chosen: readonly InboxType[]): TypeCount[] {
  const n = new Map<InboxType, number>();
  for (const r of rows) {
    const k = typeOf(r);
    n.set(k, (n.get(k) ?? 0) + 1);
  }
  return INBOX_TYPES.filter((k) => (n.get(k) ?? 0) > 0 || chosen.includes(k)).map((k) => ({
    type: k,
    count: n.get(k) ?? 0,
    on: chosen.includes(k),
  }));
}

/** A type put in the filter, or taken out; the result in the order of `INBOX_TYPES`. */
export function toggle(chosen: readonly InboxType[], type: InboxType): InboxType[] {
  const set = new Set(chosen);
  if (set.has(type)) set.delete(type);
  else set.add(type);
  return INBOX_TYPES.filter((k) => set.has(k));
}

/** Where the choice is kept, on this machine (the window's `localStorage`, the phone's own store). */
export const FILTER_KEY = 'escouade.inbox.filter';

/** The choice kept, its unknown types dropped; none kept or unreadable: « Tout ». */
export function readFilter(store: Pick<Storage, 'getItem'> | null | undefined): InboxType[] {
  try {
    const v: unknown = JSON.parse(store?.getItem(FILTER_KEY) ?? '[]');
    if (!Array.isArray(v)) return [];
    return INBOX_TYPES.filter((k) => v.includes(k));
  } catch {
    return [];
  }
}

/** Keeps the choice; « Tout » leaves nothing behind. */
export function writeFilter(store: Pick<Storage, 'setItem' | 'removeItem'> | null | undefined, chosen: readonly InboxType[]) {
  try {
    if (isAll(chosen)) store?.removeItem(FILTER_KEY);
    else store?.setItem(FILTER_KEY, JSON.stringify(chosen));
  } catch {
    // Not kept: « Tout » at the next opening.
  }
}

/** The requests a filter keeps (« Plus tard », « Répondu récemment »). */
export const keepAsks = <A extends { kind: AskKind; gate: boolean }>(asks: readonly A[], chosen: readonly InboxType[]): A[] =>
  asks.filter((a) => shows(chosen, typeOfAsk(a)));

/** The views a filter keeps (« Plus tard », « Le chef s'en occupe », the trace). */
export const keepViews = <V extends Pick<InboxItem, 'source'>>(views: readonly V[], chosen: readonly InboxType[]): V[] =>
  views.filter((v) => shows(chosen, typeOfView(v)));
