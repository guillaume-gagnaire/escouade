// The pull requests of the tickets (ESC-59): the column « En revue » of the board, and what a card there says of its pull
// request as its host last told it (state, CI and its jobs, approvals, open threads, conflict, an error of the host).

import { fSince } from './format';
import { t } from './i18n';
import type { BoardSettings, MergeStrategy, PullRequest, Ticket, TicketPr } from './types';

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

/** Its CI passed, or it has none. */
export const isGreen = (p: TicketPr) => p.checks === null || p.checks === 'success';

/** Open, followed, not a draft, without a conflict with its target, and its agent not fixing its CI (ESC-60). */
const isOpen = (p: TicketPr) => p.state === 'open' && !p.closed && !p.draft && p.mergeable !== false && !p.fixing;

/** « Merger » may be offered (as `ticketprs::merge_ready`): approved, no change asked for, green, without a conflict. */
export const mergeReady = (p: TicketPr) => isOpen(p) && p.approvals > 0 && !p.changesRequested && isGreen(p);

/** « Merger quand c'est vert » (as `ticketprs::can_wait_green`): its CI runs, and nothing merges it by itself yet. */
export const canWaitGreen = (p: TicketPr) =>
  isOpen(p) && !p.changesRequested && p.checks === 'pending' && !p.autoMerge && !p.mergeWhenGreen;

/** The strategy « Merger » uses: the Kanban's when the repository allows it, else the first it allows; null when it allows
 * none, or is not read yet (as `ticketprs::pick_strategy`). */
export function mergeStrategy(p: TicketPr, wanted: BoardSettings['strategy']): MergeStrategy | null {
  const allowed = p.mergeOptions?.strategies;
  if (!allowed) return null;
  return allowed.includes(wanted) ? wanted : (allowed[0] ?? null);
}

const HOSTS: Record<string, string> = {
  github: 'GitHub',
  gitlab: 'GitLab',
  gitea: 'Gitea',
  bitbucket: 'Bitbucket',
  azure: 'Azure DevOps',
};

/** Its host's name (`GitHub`), its address for one Escouade does not know. */
export const hostName = (p: TicketPr) => HOSTS[p.kind] ?? p.host;

/** What merges it by itself, in words: the host's auto-merge, or Escouade once it is approved and green; null when nothing. */
export function mergeWaitLine(p: TicketPr): string | null {
  if (p.state !== 'open' || p.closed) return null;
  if (p.autoMerge) return t('board.pr.merge.hostOn', { host: hostName(p) });
  if (p.mergeWhenGreen) return t('board.pr.merge.escouadeWaits', { strategy: t(`board.pr.merge.strategies.${p.mergeWhenGreen}`) });
  return null;
}

/** What the merge buttons of its card are: « Merger (strategy) » approved and green, « Merger quand c'est vert » while
 * its CI runs; `none` says why « Merger » is not there though it is approved and green (the repository allows none). */
export function mergeActions(
  p: TicketPr,
  wanted: BoardSettings['strategy'],
): { merge: MergeStrategy | null; whenGreen: boolean; none: boolean } {
  const ready = mergeReady(p);
  const strategy = mergeStrategy(p, wanted);
  return {
    merge: ready ? strategy : null,
    whenGreen: canWaitGreen(p),
    none: ready && !!p.mergeOptions && !strategy,
  };
}

/** What « Merger quand c'est vert » did, for its toast. */
export const mergeHowText = (how: string, p: TicketPr) =>
  how === 'host'
    ? t('board.pr.merge.hostOn', { host: hostName(p) })
    : how === 'escouade'
      ? t('board.pr.merge.escouadeTold', { host: hostName(p) })
      : t('board.pr.merge.merged', { word: p.word, number: p.number });
/** « L'agent corrige » (ESC-60): its CI failed, its pull request is open, it has an agent, and no fix is under way. */
export const canFixCi = (x: Ticket) =>
  x.column === 'pr' && !!x.pr && !!x.agentId && !x.pr.closed && x.pr.state === 'open' && x.pr.checks === 'failure' && !x.pr.fixing;

/** The failed checks whose host keeps no log (a CI of another kind: Jenkins, CircleCI…): said on the card, their name alone goes to the agent. */
export function noLogLine(p: TicketPr): string | null {
  const names = (p.failed ?? []).filter((c) => !c.log).map((c) => c.name);
  if (!names.length) return null;
  return t('board.pr.noLog', { count: names.length, names: names.join(', ') });
}

/** Its agent is free to be given something on its pull request: it has one, its pull request is open, and no fix is under way. */
const free = (x: Ticket) => x.column === 'pr' && !!x.pr && !!x.agentId && !x.pr.closed && x.pr.state === 'open' && !x.pr.fixing;

/** « L'agent corrige les retours » (ESC-61): new review comments wait for its agent. On a click only, the autopilot's too. */
export const canFixReviews = (x: Ticket) => free(x) && (x.pr?.feedback ?? 0) > 0;

/** « L'agent met à jour la branche » (ESC-61): its pull request conflicts with its target. */
export const canUpdateBranch = (x: Ticket) => free(x) && x.pr?.mergeable === false;

/** What its agent does now on its pull request, in words. */
export const fixingLine = (p: TicketPr) =>
  p.fixKind === 'rebase'
    ? t('board.pr.fixingKind.rebase', { target: p.target })
    : t(`board.pr.fixingKind.${p.fixKind === 'reviews' || p.fixKind === 'merge' ? p.fixKind : 'ci'}`);

/** With the autopilot, its CI no longer goes to its agent by itself once its loops are spent: « L'agent corrige » still does. */
export const fixesSpent = (x: Ticket, autopilot: boolean) => autopilot && !!x.pr && x.pr.fixes >= x.maxLoops;

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
    autoMerge: !!p.autoMerge,
    checks: p.checks,
    fork: false,
  };
}
