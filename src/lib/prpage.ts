// The view of a pull request (ESC-457, PR-2 of the PR in Escouade, direction A of design/pr-2-3): what it says of a pull
// request read whole (`PrContents`) and of its checks and reviews (`PrDetail`). The component reads; these say.

import { t } from './i18n';
import { failureTitle } from './prview';
import type { Agent, PrContents, PrDetail, PrFailure, PrReview, PrThread, PullRequest, Ticket } from './types';

/** The tabs of the view, in their order; « Fichiers » is PR-3's. */
export const PR_TABS = ['conversation', 'commits', 'checks', 'files'] as const;
export type PrTab = (typeof PR_TABS)[number];

/** A commit as the timeline and the tab « Commits » show it. */
export type PrCommitRow = PrContents['commits'][number];

/** An item of the conversation's timeline, after the description. */
export type TimelineItem =
  | { kind: 'commits'; key: string; author: string; commits: PrCommitRow[]; at?: number }
  | { kind: 'thread'; key: string; thread: PrThread; at?: number }
  | { kind: 'comment'; key: string; thread: PrThread; at?: number }
  | { kind: 'review'; key: string; review: PrReview; at?: number };

/** When a thread started: its first comment's time. */
const startOf = (th: PrThread) => th.comments[0]?.created;

/**
 * The conversation's timeline: the commits pushed (the consecutive ones of an author together), the threads on lines and
 * the comments of the conversation, by their time; then the reviews, which the host gives without one, in its order.
 * An item without a time keeps its place after the dated ones.
 */
export function timeline(c: PrContents | null, d: PrDetail | null): TimelineItem[] {
  const dated: TimelineItem[] = [];
  for (const [i, k] of (c?.commits ?? []).entries()) {
    const last = dated[dated.length - 1];
    if (last?.kind === 'commits' && last.author === k.author) {
      last.commits.push(k);
      continue;
    }
    dated.push({ kind: 'commits', key: `c${i}-${k.sha}`, author: k.author, commits: [k], at: k.created });
  }
  for (const [i, th] of (c?.threads ?? []).entries()) {
    if (!th.comments.length) continue;
    const key = `t${i}-${th.id}`;
    dated.push(th.path ? { kind: 'thread', key, thread: th, at: startOf(th) } : { kind: 'comment', key, thread: th, at: startOf(th) });
  }
  // A stable sort: those of the same time, or without one, keep the host's order.
  const order = dated.map((x, i) => ({ x, i }));
  order.sort((a, b) => {
    const [ta, tb] = [a.x.at, b.x.at];
    if (ta === undefined && tb === undefined) return a.i - b.i;
    if (ta === undefined) return 1;
    if (tb === undefined) return -1;
    return ta - tb || a.i - b.i;
  });
  const reviews = (d?.reviews ?? [])
    .filter((r) => r.state !== 'commented' || r.body.trim())
    .map((review, i): TimelineItem => ({ kind: 'review', key: `r${i}`, review }));
  return [...order.map((o) => o.x), ...reviews];
}

/** A thread's state in words. */
export const threadState = (th: PrThread) => t(`prs.page.thread.${th.state}`);

/** Where a thread is: « src/a.ts:12 », a range « src/a.ts:10-12 ». */
export function threadPlace(th: PrThread): string {
  if (!th.path) return '';
  const line = th.line ?? th.originalLine;
  if (!line) return th.path;
  return th.startLine && th.startLine !== line ? `${th.path}:${th.startLine}-${line}` : `${th.path}:${line}`;
}

/** A reviewer of the aside: its last word (approved, changes asked, commented), or a review asked and not given. */
export interface Reviewer {
  name: string;
  state: 'approved' | 'changesRequested' | 'commented' | 'requested';
}

/** The reviewers: each author's last review (the host gives them oldest first), then those asked who have not answered. */
export function reviewers(pr: PullRequest | null, d: PrDetail | null): Reviewer[] {
  const last = new Map<string, Reviewer['state']>();
  for (const r of d?.reviews ?? []) {
    // A comment after an approval does not take it back.
    if (r.state === 'commented' && last.has(r.author)) continue;
    last.set(r.author, r.state);
  }
  const out: Reviewer[] = [...last].map(([name, state]) => ({ name, state }));
  for (const name of pr?.reviewers ?? []) if (!last.has(name)) out.push({ name, state: 'requested' });
  return out;
}

/** What the host does not read of a pull request (its capabilities), in words: the view says it, empty is not missing. */
export function missingParts(c: PrContents | null): string[] {
  if (!c) return [];
  const parts: string[] = [];
  if (!c.reading.description) parts.push(t('prs.page.part.description'));
  if (!c.reading.commits) parts.push(t('prs.page.part.commits'));
  if (!c.reading.threads) parts.push(t('prs.page.part.threads'));
  return parts;
}

/** The checks in a tab's count: « 4/5 » passed of all; none without a check. */
export function checksCount(d: PrDetail | null): string {
  if (!d?.checks.length) return '';
  return `${d.checks.filter((c) => c.state === 'success').length}/${d.checks.length}`;
}

/** The conversation's count: its comments, as GitHub counts them (those of the threads and of the conversation). */
export const conversationCount = (c: PrContents | null) => (c ? c.threads.reduce((n, th) => n + th.comments.length, 0) : 0);

/** The state's pill: its words and its tone. */
export function statePill(pr: PullRequest): { text: string; tone: 'open' | 'draft' | 'merged' | 'closed' } {
  const tone = pr.state === 'open' ? (pr.draft ? 'draft' : 'open') : pr.state;
  return { text: t(`prs.page.state.${tone}`), tone };
}

/** The first line of a commit's message; the rest is shown on a click. */
export const subjectOf = (message: string) => message.split('\n', 1)[0] ?? '';
export const hasBody = (message: string) => message.trim().includes('\n');

/** A commit's short hash. */
export const shortSha = (sha: string) => sha.slice(0, 7);

/** The two letters of an avatar: of two words, their first ones; of one, its first two (« marta », MA). */
export function initials(name: string): string {
  const words = name
    .replace(/[^\p{L}\p{N}]+/gu, ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (!words.length) return '?';
  const two = words.length > 1 ? words[0][0] + words[1][0] : words[0].slice(0, 2);
  return two.toUpperCase();
}

/** The ticket whose pull request it is, in the project. */
export const ticketOf = (tickets: Ticket[], projectId: string, number: number, hostRepo?: string) =>
  tickets.find(
    (tk) =>
      tk.projectId === projectId &&
      tk.pr?.number === number &&
      (!hostRepo || `${tk.pr.namespace}/${tk.pr.name}`.toLowerCase() === hostRepo.toLowerCase()),
  ) ?? null;

/** The agent of the project whose pull request it is (one that read it, by its repository too). */
export const agentWithPr = (agents: Agent[], projectId: string, number: number, hostRepo?: string) =>
  agents.find(
    (a) =>
      a.projectId === projectId &&
      !a.archived &&
      a.prs.some((p) => p.number === number && (p.hostRepo ?? '').toLowerCase() === (hostRepo ?? '').toLowerCase()),
  ) ?? null;

/** A failure of the view in words: a pull request the host does not find says so, the rest as the tab says it. */
export function pageFailure(f: PrFailure, word: string, number: number, online = true): string {
  if (f.kind === 'unreachable' && !online) return t('prs.page.offline', { word });
  if (f.kind === 'other' && /not found|introuvable|404/i.test(f.message)) return t('prs.page.notFound', { word, number });
  return failureTitle(f);
}
