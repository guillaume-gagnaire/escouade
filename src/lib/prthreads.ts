import { fTime } from './format';
import { t } from './i18n';
import type { PrContentsFile, PrFailure, PrLine, PrThread } from './types';

// The threads of a pull request in its tab « Fichiers » (ESC-474, PR-4 of the PR in Escouade): each under its line (or its
// range) in the diff of the whole pull request, the others apart (« périmés »: their line is no longer in the diff, or
// the host says they speak of a former version), « Suivante » from an open one to the next, and what is written on the
// host on a click: its drafts, kept until sent, and its failures in words.

/** What a host writes in the user's name (`forge::Writing`). */
export interface PrWriting {
  comment: boolean;
  lineComment: boolean;
  reply: boolean;
  resolve: boolean;
  reopen: boolean;
  /** The control of the pull request (ESC-488): a draft made ready, ready made a draft. */
  draft?: boolean;
  reviewers?: boolean;
  labels?: boolean;
  updateMerge?: boolean;
  updateRebase?: boolean;
  rerunCheck?: boolean;
  cancelAutoMerge?: boolean;
  deleteBranch?: boolean;
  /** It reads where the pull request stands for its merge. */
  control?: boolean;
}

export const NO_WRITING: PrWriting = { comment: false, lineComment: false, reply: false, resolve: false, reopen: false };

/** A write the user clicked (`prwrite::PrWrite`). */
export type PrWrite =
  | { kind: 'comment'; body: string }
  | { kind: 'lineComment'; path: string; line: number; startLine?: number; side: 'left' | 'right'; body: string }
  | { kind: 'reply'; comment: string; body: string }
  | { kind: 'resolve'; thread: string }
  | { kind: 'reopen'; thread: string }
  // The control of the pull request (ESC-488).
  | { kind: 'setDraft'; draft: boolean }
  | { kind: 'requestReviewers'; logins: string[] }
  | { kind: 'removeReviewer'; login: string }
  | { kind: 'addLabels'; labels: string[] }
  | { kind: 'removeLabel'; label: string }
  | { kind: 'updateBranch'; method: 'merge' | 'rebase' }
  | { kind: 'rerunCheck'; job: string }
  | { kind: 'autoMerge'; strategy: import('./prcontrol').MergeStrategy }
  | { kind: 'cancelAutoMerge' };

/** A thread's key in the view: the host's id, else its place and its first comment. */
export const threadKey = (th: PrThread) => th.id || `${th.path ?? ''}:${th.line ?? ''}:${th.comments[0]?.id ?? ''}`;

/** Not resolved: open, or outdated (the host still waits for it). */
export const isOpen = (th: PrThread) => th.state !== 'resolved';

/** The index of a line of the diff a thread is on: the tip's number on its side, the base's for a line removed. */
export function lineIndex(lines: PrLine[], line: number, side: 'left' | 'right'): number {
  return lines.findIndex((l) =>
    side === 'left' ? l.kind === '-' && l.old === line : (l.kind === '+' || l.kind === ' ') && l.new === line,
  );
}

/** Where the threads on lines go in the tab: under their line, by file and by index; apart in their file when their line
 * is no longer in the diff (or the host says they are outdated); apart from the files when their file is not changed. */
export interface Placement {
  under: Record<string, Record<number, PrThread[]>>;
  stale: Record<string, PrThread[]>;
  elsewhere: PrThread[];
}

export function placeThreads(files: PrContentsFile[], threads: PrThread[]): Placement {
  const out: Placement = { under: {}, stale: {}, elsewhere: [] };
  const byPath = new Map(files.map((f) => [f.path, f]));
  for (const th of threads) {
    if (!th.path) continue;
    const f = byPath.get(th.path);
    if (!f) {
      out.elsewhere.push(th);
      continue;
    }
    const i = th.state === 'outdated' || !th.line ? -1 : lineIndex(f.lines, th.line, th.side);
    if (i < 0) (out.stale[f.path] ??= []).push(th);
    else ((out.under[f.path] ??= {})[i] ??= []).push(th);
  }
  return out;
}

/** A thread's place in the order of the tab: its file, then where it is in it (apart first). */
export interface ThreadStop {
  key: string;
  path: string | null;
  /** Its line's index; -1 apart in its file or from the files. */
  index: number;
}

/** The open threads in the order the tab shows them: the files' order (`ordered`), apart from the files first, then in
 * each file those apart, then by line. */
export function openOrder(ordered: PrContentsFile[], placed: Placement): ThreadStop[] {
  const out: ThreadStop[] = placed.elsewhere.filter(isOpen).map((th) => ({ key: threadKey(th), path: null, index: -1 }));
  for (const f of ordered) {
    for (const th of placed.stale[f.path] ?? []) if (isOpen(th)) out.push({ key: threadKey(th), path: f.path, index: -1 });
    const under = placed.under[f.path] ?? {};
    for (const i of Object.keys(under)
      .map(Number)
      .sort((a, b) => a - b))
      for (const th of under[i]) if (isOpen(th)) out.push({ key: threadKey(th), path: f.path, index: i });
  }
  return out;
}

/** The next open thread after `current` (the first when none is current, again from the start after the last). */
export function nextStop(stops: ThreadStop[], current: string | null, step: 1 | -1 = 1): ThreadStop | null {
  if (!stops.length) return null;
  const at = current ? stops.findIndex((s) => s.key === current) : -1;
  if (at < 0) return step > 0 ? stops[0] : stops[stops.length - 1];
  return stops[(at + step + stops.length) % stops.length];
}

/** The threads on lines, open and resolved: what the tab's head counts. */
export function threadCounts(threads: PrThread[]): { open: number; resolved: number } {
  let open = 0;
  let resolved = 0;
  for (const th of threads) {
    if (!th.path) continue;
    if (isOpen(th)) open++;
    else resolved++;
  }
  return { open, resolved };
}

/** Where a draft is kept: the pull request, then what it answers (a thread), comments (lines) or the conversation. */
export const draftKey = (projectId: string, hostRepo: string | undefined, number: number, target: string) =>
  `${projectId}#${(hostRepo ?? '').toLowerCase()}#${number}#${target}`;

export const lineTarget = (path: string, side: 'left' | 'right', line: number) => `line:${side}:${line}:${path}`;

/** The lines a comment is on: one, or a range on the same side. */
export interface LinePick {
  path: string;
  side: 'left' | 'right';
  line: number;
  startLine?: number;
  /** The index of its last line in the diff: the composer goes under it. */
  index: number;
}

/** The line of the diff a comment can be written on, with its side and number; none for a hunk's head. */
export function pickOf(path: string, l: PrLine, index: number): LinePick | null {
  if (l.kind === '@') return null;
  if (l.kind === '-') return l.old ? { path, side: 'left', line: l.old, index } : null;
  return l.new ? { path, side: 'right', line: l.new, index } : null;
}

/** A range from `from` to `to`, on the same file and side: else `to` alone. */
export function extendPick(from: LinePick | null, to: LinePick): LinePick {
  if (!from || from.path !== to.path || from.side !== to.side || from.line === to.line) return to;
  const [a, b] = from.line < to.line ? [from, to] : [to, from];
  return { path: to.path, side: to.side, line: b.line, startLine: a.line, index: b.index };
}

/** A pick in words: « ligne 12 », « lignes 10 à 12 ». */
export const pickPlace = (p: LinePick) =>
  p.startLine ? t('prs.page.threads.lines', { from: p.startLine, to: p.line }) : t('prs.page.threads.line', { line: p.line });

/** Why a write did not go, as the thread says it; the draft is kept. */
export function writeFailure(f: PrFailure, host: string, online = true): string {
  switch (f.kind) {
    case 'refused':
      return t('prs.page.threads.failure.refused', { host });
    case 'forbidden':
      return t('prs.page.threads.failure.forbidden', { host });
    case 'gone':
      return t('prs.page.threads.failure.gone', { host });
    case 'rateLimit':
      return f.reset
        ? t('prs.page.threads.failure.rateLimitUntil', { host, time: fTime(f.reset * 1000) })
        : t('prs.page.threads.failure.rateLimit', { host });
    case 'unreachable':
      return online ? t('prs.page.threads.failure.unreachable', { host }) : t('prs.page.threads.failure.offline');
    default:
      return f.message;
  }
}
