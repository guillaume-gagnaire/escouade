// « Faire relire par un agent » (ESC-66): what the window says of a review written by an agent. Each comment is kept, edited
// or dropped; the review leaves all at once, on the user's click, once none is left to decide. See src-tauri/src/prreview.rs.

import { t } from './i18n';
import type { AgentReview, ReviewNote, ReviewVerdict } from './types';

export const VERDICTS: ReviewVerdict[] = ['commented', 'changesRequested', 'approved'];

/** The review's comments file by file, in the order of the diff. */
export function notesByFile(notes: ReviewNote[]): { path: string; notes: ReviewNote[] }[] {
  const files: { path: string; notes: ReviewNote[] }[] = [];
  for (const n of notes) {
    const file = files.find((f) => f.path === n.path);
    if (file) file.notes.push(n);
    else files.push({ path: n.path, notes: [n] });
  }
  return files;
}

/** How many are left to decide, kept and dropped. */
export function tally(notes: ReviewNote[]) {
  return {
    open: notes.filter((n) => n.choice === 'open').length,
    kept: notes.filter((n) => n.choice === 'kept').length,
    dropped: notes.filter((n) => n.choice === 'dropped').length,
  };
}

/** The user wrote over what the agent proposed. */
export const edited = (n: ReviewNote) => n.body !== n.proposed;

/** Who opened it is who reads: the hosts refuse an approval, or changes asked, of one's own pull request. */
export const isOwn = (r: AgentReview, me: string) =>
  !!me && !!r.author && r.author.localeCompare(me, undefined, { sensitivity: 'accent' }) === 0;

/** Why it cannot be published yet, as the backend refuses it; null when it can. */
export function publishBlock(r: AgentReview, verdict: ReviewVerdict, summary: string): string | null {
  const { open, kept } = tally(r.notes);
  if (open) return t('prs.review.block.open', { count: open });
  const blank = !summary.trim();
  if (verdict === 'changesRequested' && blank) return t('prs.review.block.text');
  if (verdict === 'commented' && blank && !kept) return t('prs.review.block.nothing');
  return null;
}

/** The mark on the row of its pull request. */
export function reviewMark(r: AgentReview): { text: string; tone: 'ask' | 'muted' | 'fail' } {
  if (r.phase === 'reading') return { text: t('prs.review.mark.reading'), tone: 'muted' };
  if (r.phase === 'failed') return { text: t('prs.review.mark.failed'), tone: 'fail' };
  if (r.phase === 'published') return { text: t('prs.review.mark.published'), tone: 'muted' };
  const { open } = tally(r.notes);
  return { text: open ? t('prs.review.mark.open', { count: open }) : t('prs.review.mark.ready'), tone: 'ask' };
}
