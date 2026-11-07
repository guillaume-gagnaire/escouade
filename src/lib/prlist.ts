// The project's « Pull requests » view (ESC-65, direction A of design/esc-65): its open pull requests in a dense list,
// filtered by « Les miennes », « Relecture demandée » and « Toutes », each with its CI, its reviews, its conflict and its
// age; « Lancer un agent sur cette PR » puts an agent in a worktree on its branch (a fork's too). Read only: nothing is
// written on the host.

import { fSince } from './format';
import { t } from './i18n';
import type { IconName } from './icons';
import { readPref, writePref } from './prefs';
import { ownBranch } from './prs';
import type { Agent, OverviewPrs, PullRequest, Ticket, TicketPr } from './types';

export type PrFilter = 'mine' | 'review' | 'all';
export const FILTERS: PrFilter[] = ['mine', 'review', 'all'];

/** How often the view reads them again while it is shown and the window visible. */
export const LIST_EVERY_MS = 60_000;

const same = (a: string, b: string) => a.localeCompare(b, undefined, { sensitivity: 'accent' }) === 0;

/** Opened by the account the host was read with (`me`, its login). */
export const isMine = (p: PullRequest, me: string) => !!me && !!p.author && same(p.author, me);

/** Its review is asked of `me` and not given yet. */
export const asksMe = (p: PullRequest, me: string) => !!me && (p.reviewers ?? []).some((r) => same(r, me));

export function filterPrs(prs: PullRequest[], f: PrFilter, me: string): PullRequest[] {
  if (f === 'mine') return prs.filter((p) => isMine(p, me));
  if (f === 'review') return prs.filter((p) => asksMe(p, me));
  return prs;
}

/** How many each filter shows. */
export const filterCounts = (prs: PullRequest[], me: string): Record<PrFilter, number> => ({
  mine: filterPrs(prs, 'mine', me).length,
  review: filterPrs(prs, 'review', me).length,
  all: prs.length,
});

/** The filter chosen last (kept on this machine), « Toutes » at first. */
export function savedFilter(): PrFilter {
  const f = readPref('prsFilter');
  return f === 'mine' || f === 'review' || f === 'all' ? f : 'all';
}
export const saveFilter = (f: PrFilter) => writePref('prsFilter', f);

/** The local branch of a pull request opened from a fork, as the backend names it (`pragent::fork_branch`). */
export const forkBranch = (word: string, number: number) => `${word.toLowerCase()}-${number}`;

/**
 * The agent of the project already on it: one that read it, one on its branch, or for a fork's one on the branch the
 * backend fetched it into (`pr-<n>`).
 */
export function ownerOf(agents: Agent[], p: PullRequest, word: string): Agent | null {
  const live = agents.filter((a) => !a.archived);
  const fork = forkBranch(word, p.number);
  return (
    live.find((a) => a.prs.some((q) => q.number === p.number && !q.hostRepo)) ??
    live.find((a) => (p.fork ? ownBranch(a) === fork : ownBranch(a) === p.head)) ??
    null
  );
}

export type Tone = 'ok' | 'wait' | 'fail' | 'dim';

/** Its CI in words and in a tone: « CI verte », « CI en échec », « CI en cours », « CI ignorée », « Pas de CI ». */
export function ciOf(p: PullRequest): { text: string; tone: Tone; icon: IconName } {
  if (!p.checks) return { text: t('prs.list.noCi'), tone: 'dim', icon: 'pr' };
  const tone: Tone = p.checks === 'success' ? 'ok' : p.checks === 'failure' ? 'fail' : p.checks === 'pending' ? 'wait' : 'dim';
  const icon = ({ failure: 'close', success: 'check', pending: 'clock', skipped: 'pr' } as const)[p.checks];
  return { text: t(`prs.checks.${p.checks}`), tone, icon };
}

/** The glyph at the start of its row: its CI's, a draft's grey. */
export function rowTone(p: PullRequest): Tone {
  if (p.draft) return 'dim';
  if (p.checks === 'failure' || p.mergeable === false) return 'fail';
  if (p.changesRequested || p.checks === 'pending') return 'wait';
  return p.checks ? 'ok' : 'dim';
}

export interface Chip {
  key: string;
  text: string;
  tone: Tone | 'muted' | 'ask';
  icon: IconName;
}

/** What its row says of its reviews and its conflict, the most pressing first. */
export function chipsOf(p: PullRequest, me: string): Chip[] {
  const chips: Chip[] = [];
  if (p.mergeable === false) chips.push({ key: 'conflict', text: t('prs.list.conflict'), tone: 'fail', icon: 'swap' });
  if (p.changesRequested) chips.push({ key: 'changes', text: t('prs.list.changesRequested'), tone: 'wait', icon: 'edit' });
  if (p.approvals) chips.push({ key: 'approvals', text: t('prs.approvals', { count: p.approvals }), tone: 'ok', icon: 'check' });
  if (p.openThreads) chips.push({ key: 'threads', text: t('prs.threads', { count: p.openThreads }), tone: 'muted', icon: 'read' });
  if (asksMe(p, me)) chips.push({ key: 'asked', text: t('prs.list.yourReview'), tone: 'ask', icon: 'user' });
  return chips;
}

/** Why a pull request is in the group of the overview: what the user has to do about it. */
export type PrNeed = 'fix' | 'merge' | 'review';
export const NEEDS: PrNeed[] = ['fix', 'merge', 'review'];

/** A line of the group « Pull requests » of the overview (ESC-68). */
export interface OverviewPr {
  key: string;
  projectId: string;
  word: string;
  pr: PullRequest;
  need: PrNeed;
  me: string;
  /** What a click opens: its agent, else its ticket's board, else the project's pull requests. */
  agent: Agent | null;
  ticket: Ticket | null;
}

/** What a pull request of the user's waits for: to be fixed (its CI, changes asked, a conflict), or merged (approved, its CI done). */
export function needOf(p: PullRequest): 'fix' | 'merge' | null {
  if (p.state !== 'open' || p.draft) return null;
  if (p.checks === 'failure' || p.changesRequested || p.mergeable === false) return 'fix';
  return p.approvals > 0 && p.checks !== 'pending' ? 'merge' : null;
}

/** A ticket's pull request, as a list has it. Its title is the ticket's. */
const ofTicket = (t: Ticket, tp: TicketPr): PullRequest => ({
  number: tp.number,
  url: tp.url,
  title: t.title,
  state: tp.state,
  draft: tp.draft,
  head: tp.branch,
  base: tp.target,
  mergeable: tp.mergeable ?? null,
  approvals: tp.approvals,
  changesRequested: tp.changesRequested,
  openThreads: tp.openThreads,
  autoMerge: !!tp.autoMerge,
  checks: tp.checks,
  fork: false,
});

/**
 * The pull requests the user has something to do about, all projects together: those of the tickets « En revue » and of
 * the agents (read by their follow), then those of the lists (the user's own, and those whose review is asked of them).
 * Each once: the ticket's reading first, then the agent's, the list's last (which on GitLab says neither its CI nor its
 * approvals). The most pressing need first, the newest first in each.
 */
export function overviewPrs(lists: OverviewPrs[], agents: Agent[], tickets: Ticket[], projectIds: string[]): OverviewPr[] {
  const known = new Set(projectIds);
  const live = agents.filter((a) => !a.archived && known.has(a.projectId));
  const byKey = new Map<string, OverviewPr>();
  const keyOf = (projectId: string, p: PullRequest) => `pr:${projectId}:${(p.hostRepo ?? '').toLowerCase()}:${p.number}`;
  const wordOf = (projectId: string, fallback?: string | null) => lists.find((l) => l.projectId === projectId)?.word ?? fallback ?? 'PR';
  const meOf = (projectId: string) => lists.find((l) => l.projectId === projectId)?.me ?? '';
  for (const t of tickets) {
    if (t.column !== 'pr' || !t.pr || t.pr.closed || !known.has(t.projectId)) continue;
    const pr = ofTicket(t, t.pr);
    const need = needOf(pr);
    const agent = live.find((a) => a.id === t.agentId) ?? null;
    if (need)
      byKey.set(keyOf(t.projectId, pr), {
        key: keyOf(t.projectId, pr),
        projectId: t.projectId,
        word: t.pr.word,
        pr,
        need,
        me: meOf(t.projectId),
        agent,
        ticket: t,
      });
  }
  for (const a of live) {
    for (const pr of a.prs) {
      const key = keyOf(a.projectId, pr);
      const need = needOf(pr);
      const seen = byKey.get(key);
      if (seen) seen.agent ??= a;
      else if (need)
        byKey.set(key, {
          key,
          projectId: a.projectId,
          word: wordOf(a.projectId, a.prWord),
          pr,
          need,
          me: meOf(a.projectId),
          agent: a,
          ticket: null,
        });
    }
  }
  for (const l of lists) {
    if (!known.has(l.projectId)) continue;
    for (const pr of l.prs) {
      const key = keyOf(l.projectId, pr);
      if (byKey.has(key)) continue;
      // A ticket or an agent that read it has said all it has to say: a ticket's never goes by its list.
      const held = tickets.some((t) => t.projectId === l.projectId && t.pr?.number === pr.number);
      const need = asksMe(pr, l.me) && !isMine(pr, l.me) ? 'review' : isMine(pr, l.me) ? needOf(pr) : null;
      if (!need || held) continue;
      const agent = ownerOf(
        live.filter((a) => a.projectId === l.projectId),
        pr,
        l.word,
      );
      byKey.set(key, { key, projectId: l.projectId, word: l.word, pr, need, me: l.me, agent, ticket: null });
    }
  }
  const rank = (o: OverviewPr) => NEEDS.indexOf(o.need);
  return [...byKey.values()].sort((a, b) => rank(a) - rank(b) || b.pr.number - a.pr.number);
}

/** How old it is, short: « 3 min », « 2 h », « 3 j »; empty when the host does not say. */
export const ageOf = (p: PullRequest, now: number) => (p.created ? fSince(p.created * 1000, now) : '');

/** The fork it comes from (`marta` of `marta/demo`), for its mark. */
export const forkOwner = (p: PullRequest) => (p.fork ? (p.headRepo ?? '').split('/')[0] || '?' : null);

/** The next row on ↑ / ↓ (Home, End), within the rows shown; -1 without any. */
export function moveIn(len: number, at: number, key: string): number {
  if (!len) return -1;
  if (key === 'Home') return 0;
  if (key === 'End') return len - 1;
  if (at < 0) return key === 'ArrowUp' ? len - 1 : 0;
  if (key === 'ArrowDown') return Math.min(len - 1, at + 1);
  if (key === 'ArrowUp') return Math.max(0, at - 1);
  return at;
}
