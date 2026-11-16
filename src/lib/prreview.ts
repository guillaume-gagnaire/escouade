// « Faire relire par un agent » (ESC-66, ESC-490 for PR-5 of the PR in Escouade): what the window makes of a review written
// by an agent. Its remarks are threads of the agent under their line in the tab « Fichiers » of the pull request (or on
// its file, or on the pull request), marked « agent, pas publié », each of a severity; the user drops one (resolved here)
// or publishes it on the host as a comment of theirs, one by one or those kept at once. See src-tauri/src/prreview.rs.

import { t } from './i18n';
import { lineIndex, type ThreadStop } from './prthreads';
import type { AgentReview, PrContentsFile, ReviewNote, Severity } from './types';

/** The severities, the heaviest first. */
export const SEVERITIES: Severity[] = ['security', 'bug', 'spec', 'detail'];

/** A remark's key among the threads of the view (never a host's id). */
export const noteKey = (n: ReviewNote) => `agent:${n.id}`;

/** Still waits for the user: open or kept. */
export const isLive = (n: ReviewNote) => n.choice === 'open' || n.choice === 'kept';

/** The user wrote over what the agent proposed. */
export const edited = (n: ReviewNote) => n.body !== n.proposed;

/** The remarks still waiting, by severity: what the tab's head counts. */
export function severityCounts(notes: ReviewNote[]): Record<Severity, number> {
  const out: Record<Severity, number> = { security: 0, bug: 0, spec: 0, detail: 0 };
  for (const n of notes) if (isLive(n)) out[n.severity in out ? n.severity : 'detail']++;
  return out;
}

/** The severities shown: all when none is chosen. */
export type SeverityFilter = Partial<Record<Severity, boolean>>;

export const shownBy = (filter: SeverityFilter) => (n: ReviewNote) => !Object.values(filter).some(Boolean) || !!filter[n.severity];

/** The remarks in the order asked: the heaviest first (then by place), or by place (the pull request first, then file by
 * file in the order of `paths`, by line). */
export function sortNotes(notes: ReviewNote[], by: 'severity' | 'place', paths: string[] = []): ReviewNote[] {
  const file = (p: string) => (p ? paths.indexOf(p) + 1 || paths.length + 1 : 0);
  const place = (a: ReviewNote, b: ReviewNote) => file(a.path) - file(b.path) || a.line - b.line || a.id - b.id;
  const weight = (n: ReviewNote) => SEVERITIES.indexOf(n.severity);
  return [...notes].sort(by === 'severity' ? (a, b) => weight(a) - weight(b) || place(a, b) : place);
}

/** Where a remark is, in words: « src/a.ts:12 », « src/a.ts:10-12 », « src/a.ts », « la PR ». */
export function notePlace(n: ReviewNote, word: string): string {
  if (!n.path) return t('prs.review.onPr', { word });
  if (!n.line) return n.path;
  return n.startLine ? `${n.path}:${n.startLine}-${n.line}` : `${n.path}:${n.line}`;
}

/** Where the remarks go in the tab « Fichiers »: under their line in the diff read now, on their file (above its lines),
 * apart in their file when their code is no longer there (stale), on the pull request or on a file the diff does not
 * change (above the files). Those published are the host's threads now: not here. */
export interface NotePlacement {
  under: Record<string, Record<number, ReviewNote[]>>;
  onFile: Record<string, ReviewNote[]>;
  stale: Record<string, ReviewNote[]>;
  top: ReviewNote[];
}

/** The index of a remark's line in a file's diff, when its code is still there; -1 else. */
export function noteIndex(f: PrContentsFile, n: ReviewNote): number {
  if (n.stale || n.outside || !n.line) return -1;
  const i = lineIndex(f.lines, n.line, 'right');
  if (i < 0) return -1;
  const was = n.code.at(-1);
  return was === undefined || f.lines[i].text === was ? i : -1;
}

export function placeNotes(files: PrContentsFile[], notes: ReviewNote[]): NotePlacement {
  const out: NotePlacement = { under: {}, onFile: {}, stale: {}, top: [] };
  const byPath = new Map(files.map((f) => [f.path, f]));
  for (const n of notes) {
    if (n.choice === 'published') continue;
    const f = n.path ? byPath.get(n.path) : undefined;
    if (!f) {
      out.top.push(n);
      continue;
    }
    if (!n.line) {
      (out.onFile[f.path] ??= []).push(n);
      continue;
    }
    const i = noteIndex(f, n);
    if (i < 0) (out.stale[f.path] ??= []).push(n);
    else ((out.under[f.path] ??= {})[i] ??= []).push(n);
  }
  return out;
}

/** Whether a remark is published on its line: it sits under it in the diff read now. */
export function onItsLine(files: PrContentsFile[], n: ReviewNote): boolean {
  const f = files.find((x) => x.path === n.path);
  return !!f && noteIndex(f, n) >= 0;
}

/** The remarks still waiting, in the order of the tab (`ordered`: the files' order): those above the files first, then in
 * each file those on it or apart, then by line. Each stop's key is `noteKey`. */
export function noteOrder(ordered: PrContentsFile[], placed: NotePlacement): ThreadStop[] {
  const stop = (n: ReviewNote, path: string | null, index: number): ThreadStop => ({ key: noteKey(n), path, index });
  const out = placed.top.filter(isLive).map((n) => stop(n, null, -1));
  for (const f of ordered) {
    for (const n of [...(placed.onFile[f.path] ?? []), ...(placed.stale[f.path] ?? [])]) if (isLive(n)) out.push(stop(n, f.path, -1));
    const under = placed.under[f.path] ?? {};
    for (const i of Object.keys(under)
      .map(Number)
      .sort((a, b) => a - b))
      for (const n of under[i]) if (isLive(n)) out.push(stop(n, f.path, i));
  }
  return out;
}

/** What « Suivante » goes through: the host's threads, the agent's remarks, or both. */
export type NextScope = 'both' | 'host' | 'agent';

/** The host's open threads and the agent's open remarks in one order: apart from the files first, then file by file, by
 * line (at one line, the host's first). */
export function mergeStops(ordered: PrContentsFile[], host: ThreadStop[], agent: ThreadStop[], scope: NextScope): ThreadStop[] {
  const picked = scope === 'host' ? host : scope === 'agent' ? agent : [...host, ...agent];
  const file = (p: string | null) => (p === null ? -1 : ordered.findIndex((f) => f.path === p));
  const rank = new Map(picked.map((s, i) => [s, i]));
  return [...picked].sort((a, b) => file(a.path) - file(b.path) || a.index - b.index || rank.get(a)! - rank.get(b)!);
}

/** The remarks kept, in the order of the tab: what « Publier les n remarques gardées » sends. */
export function keptNotes(notes: ReviewNote[], paths: string[]): ReviewNote[] {
  return sortNotes(
    notes.filter((n) => n.choice === 'kept'),
    'place',
    paths,
  );
}

/** The mark on the row of its pull request. */
export function reviewMark(r: AgentReview): { text: string; tone: 'ask' | 'muted' | 'fail' } {
  if (r.phase === 'reading') return { text: t('prs.review.mark.reading'), tone: 'muted' };
  if (r.phase === 'failed' && !r.notes.length) return { text: t('prs.review.mark.failed'), tone: 'fail' };
  const open = r.notes.filter(isLive).length;
  return open ? { text: t('prs.review.mark.open', { count: open }), tone: 'ask' } : { text: t('prs.review.mark.none'), tone: 'muted' };
}

/** The lines a suggestion replaces and what it puts in their place, as a diff: `-` its code, `+` the suggestion. */
export function suggestionDiff(n: ReviewNote): { kind: '+' | '-'; text: string }[] {
  if (n.suggestion === undefined) return [];
  const removed = n.code.map((text) => ({ kind: '-' as const, text }));
  const added = n.suggestion === '' ? [] : n.suggestion.split('\n').map((text) => ({ kind: '+' as const, text }));
  return [...removed, ...added];
}
