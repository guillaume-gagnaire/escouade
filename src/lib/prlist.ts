// The project's « Pull requests » view (ESC-65, direction A of design/esc-65): its open pull requests in a dense list,
// filtered by « Les miennes », « Relecture demandée » and « Toutes », each with its CI, its reviews, its conflict and its
// age; « Lancer un agent sur cette PR » puts an agent in a worktree on its branch (a fork's too). Read only: nothing is
// written on the host.

import { fSince } from './format';
import { t } from './i18n';
import type { IconName } from './icons';
import { readPref, writePref } from './prefs';
import { ownBranch } from './prs';
import type { Agent, PullRequest } from './types';

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
