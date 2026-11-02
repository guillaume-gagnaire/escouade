// The pull requests of the tickets (ESC-59): the column « En revue » of the board, and what a card there says of its pull
// request as its host last told it (state, CI and its jobs, approvals, open threads, conflict, an error of the host).

import { fSince } from './format';
import { t } from './i18n';
import type { BoardSettings, PullRequest, Ticket, TicketPr } from './types';

/** The column « En revue » shows when the board opens pull requests, or while a ticket is in it. */
export const prColumnShown = (s: Pick<BoardSettings, 'action'>, tickets: Ticket[]) =>
  s.action === 'pr' || tickets.some((x) => x.column === 'pr');

/** « Relecteurs » and « Étiquettes » of the board's settings: names by commas, as typed (a label may hold a space). */
export const splitNames = (s: string) =>
  s
    .split(',')
    .map((x) => x.trim().replace(/^@/, ''))
    .filter(Boolean);

export const joinNames = (list: string[] | undefined) => (list ?? []).join(', ');

/** Its state in a word: closed, a draft, changes requested, approved, else open. */
export function prStateKey(p: TicketPr): 'closed' | 'draft' | 'changes' | 'approved' | 'open' {
  if (p.closed || p.state === 'closed') return 'closed';
  if (p.draft) return 'draft';
  if (p.changesRequested) return 'changes';
  if (p.approvals > 0) return 'approved';
  return 'open';
}

export const prStateWord = (p: TicketPr) => t(`board.pr.state.${prStateKey(p)}`);

/** Its CI with the count of its jobs; null when it has none. */
export function ciLine(p: TicketPr): string | null {
  if (!p.checks) return null;
  const count = p.checksTotal;
  if (p.checks === 'failure') return count ? t('board.pr.ci.failure', { failed: p.checksFailed, count }) : t('prs.checks.failure');
  if (p.checks === 'skipped') return t('board.pr.ci.skipped');
  return count ? t(`board.pr.ci.${p.checks}`, { count }) : t(`prs.checks.${p.checks}`);
}

/** What else to know of it: its approvals, its open threads, a conflict with its target. */
export function prFacts(p: TicketPr): string[] {
  const out: string[] = [];
  if (p.approvals) out.push(t('prs.approvals', { count: p.approvals }));
  if (p.openThreads) out.push(t('prs.threads', { count: p.openThreads }));
  if (p.mergeable === false) out.push(t('prs.conflict', { base: p.target }));
  return out;
}

/** Its host could not be read (out of reach, token expired): why, and since when. Null while it reads well. */
export function prErrorLine(p: TicketPr, now: number): string | null {
  if (!p.error) return null;
  return t('board.pr.error', { error: p.error, since: fSince(p.errorSince ?? now, now) });
}

/** As the agents' pull requests are (`prs.ts`), for their tone and icon. */
export function asPull(p: TicketPr): PullRequest {
  return {
    number: p.number,
    url: p.url,
    title: '',
    state: p.closed ? 'closed' : p.state,
    draft: p.draft,
    head: p.branch,
    base: p.target,
    mergeable: p.mergeable,
    approvals: p.approvals,
    changesRequested: p.changesRequested,
    openThreads: p.openThreads,
    autoMerge: false,
    checks: p.checks,
    fork: false,
  };
}
