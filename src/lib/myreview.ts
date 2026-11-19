// « Ta relecture » (ESC-494, PR-7 of the PR in Escouade, direction A of design/pr-2-3): what the window makes of the user's
// own review of a pull request. Its remarks wait under their lines, « en attente · pas publiée », its text in the panel of
// the header's button « Ta relecture », with its verdict; « Publier » sends it all in one review of the host, on that click
// only. See src-tauri/src/myreview.rs.

import { fTime } from './format';
import { t } from './i18n';
import { lineIndex, type PrWriting } from './prthreads';
import type { PendingNote, PrContentsFile, PrFailure, ReviewVerdict, Ticket } from './types';

/** A change of the review waiting (`myreview::PendingChange`). */
export type PendingChange =
  | { kind: 'add'; path: string; line: number; startLine?: number; side: 'left' | 'right'; body: string }
  | { kind: 'edit'; id: number; body: string }
  | { kind: 'remove'; id: number }
  | { kind: 'body'; body: string }
  | { kind: 'discard' };

/** « Publier », as the panel asks it (`myreview::ReviewAsk`). */
export interface ReviewAsk {
  verdict: ReviewVerdict;
  body: string;
  toAgent: boolean;
}

/** The verdicts, in the panel's order (the mockup's). */
export const VERDICTS: ReviewVerdict[] = ['commented', 'approved', 'changesRequested'];

/** A remark's key among the threads of the view. */
export const pendingKey = (n: PendingNote) => `mine:${n.id}`;

/** Where a remark is, in words: « src/a.ts:12 », « src/a.ts:10-12 ». */
export const pendingPlace = (n: PendingNote) => (n.startLine ? `${n.path}:${n.startLine}-${n.line}` : `${n.path}:${n.line}`);

/** The host publishes the user's review (`Writing::review`). */
export const reviews = (w: PrWriting) => !!w.review;

/** Each verdict, offered or not, and why not: the host does not give it, or the pull request is the user's own (the hosts
 * take a comment alone there: the rule of « Mes PR », the author being the login the host is read with). */
export function verdictsOf(
  w: PrWriting,
  own: boolean,
  host: string,
  word: string,
): { verdict: ReviewVerdict; offered: boolean; why?: string }[] {
  return VERDICTS.map((verdict) => {
    if (verdict === 'commented') return { verdict, offered: reviews(w) };
    if (own) return { verdict, offered: false, why: t('prs.page.mine.own', { host, word }) };
    const declared = verdict === 'approved' ? !!w.approve : !!w.requestChanges;
    return declared ? { verdict, offered: true } : { verdict, offered: false, why: t('prs.page.mine.notOffered', { host }) };
  });
}

/** Why it cannot go as it stands, as the core refuses it: a text to ask for changes, something to say. Null when it can. */
export function submitBlock(verdict: ReviewVerdict, body: string, notes: PendingNote[]): string | null {
  const blank = !body.trim();
  if (verdict === 'changesRequested' && blank) return t('prs.page.mine.block.text');
  if (verdict === 'commented' && blank && !notes.length) return t('prs.page.mine.block.nothing');
  return null;
}

/** Where the remarks go in the tab « Fichiers »: under their line in the diff read; apart in their file when their line
 * is gone (stale); above the files when their file left the diff. */
export interface PendingPlacement {
  under: Record<string, Record<number, PendingNote[]>>;
  stale: Record<string, PendingNote[]>;
  top: PendingNote[];
}

export function placePending(files: PrContentsFile[], notes: PendingNote[]): PendingPlacement {
  const out: PendingPlacement = { under: {}, stale: {}, top: [] };
  const byPath = new Map(files.map((f) => [f.path, f]));
  for (const n of notes) {
    const f = byPath.get(n.path);
    if (!f) {
      out.top.push(n);
      continue;
    }
    const i = n.stale ? -1 : lineIndex(f.lines, n.line, n.side);
    if (i < 0) (out.stale[f.path] ??= []).push(n);
    else ((out.under[f.path] ??= {})[i] ??= []).push(n);
  }
  return out;
}

/** The remarks in the order of the tab: file by file in the order of `paths`, by line. */
export function sortPending(notes: PendingNote[], paths: string[]): PendingNote[] {
  const file = (p: string) => paths.indexOf(p) + 1 || paths.length + 1;
  return [...notes].sort((a, b) => file(a.path) - file(b.path) || a.line - b.line || a.id - b.id);
}

/** « Renvoyer aussi à l'agent du ticket »: offered on the pull request of a ticket « En revue » that has an agent, on a
 * verdict that does not approve; checked by default when it asks for changes, or comments on the user's own pull request
 * (where the hosts refuse to ask for changes). */
export function agentOffer(ticket: Ticket | null, verdict: ReviewVerdict, own: boolean): { offered: boolean; checked: boolean } {
  const offered = !!ticket && ticket.column === 'pr' && !!ticket.agentId && verdict !== 'approved';
  return { offered, checked: offered && (verdict === 'changesRequested' || own) };
}

/** A refusal of the host in the panel's words: all of it stays waiting. */
export function mineFailure(f: PrFailure, host: string, online = true): string {
  switch (f.kind) {
    case 'refused':
      return t('prs.page.mine.failure.refused', { host });
    case 'forbidden':
      return t('prs.page.mine.failure.forbidden', { host });
    case 'gone':
      return t('prs.page.mine.failure.gone', { host });
    case 'rateLimit':
      return f.reset
        ? t('prs.page.mine.failure.rateLimitUntil', { host, time: fTime(f.reset * 1000) })
        : t('prs.page.mine.failure.rateLimit', { host });
    case 'unreachable':
      return online ? t('prs.page.mine.failure.unreachable', { host }) : t('prs.page.mine.failure.offline');
    default:
      return t('prs.page.mine.failure.other', { why: f.message });
  }
}
