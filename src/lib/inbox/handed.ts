// A request that changed hands (ESC-585): its agent's ticket validated, or the agent archived or deleted, while it was
// still the user's to do. It went to the project's squad lead (its author is the lead now), or, with none, it stays
// without a living author. The box says it quietly: a mark on its line, a sentence under its title.
import { t } from '../i18n';
import type { WindowAsk } from '../types';

/** It has no living author: it was handed over and the project had no squad lead. */
export function orphaned(a: WindowAsk): boolean {
  return !!a.handed && !a.author.agent;
}

/** Why it changed hands, from whom: « le ticket DEM-7 de dev est validé ». */
export function handedWhy(a: WindowAsk): string {
  const h = a.handed;
  if (!h) return '';
  if (h.cause === 'ticket')
    return h.ticketKey
      ? t('inbox.handed.why.ticket', { key: h.ticketKey, from: h.from })
      : t('inbox.handed.why.ticketNoKey', { from: h.from });
  return t(`inbox.handed.why.${h.cause}`, { from: h.from });
}

/** The mark of its line in the queue: « reprise de dev », « sans auteur ». Empty for one that never changed hands. */
export function handedMark(a: WindowAsk): string {
  if (!a.handed) return '';
  return orphaned(a) ? t('inbox.handed.markNone') : t('inbox.handed.mark', { from: a.handed.from });
}

/** The sentence of its pane, `name` its author's name of today. */
export function handedSentence(a: WindowAsk, name: string): string {
  if (!a.handed) return '';
  return orphaned(a) ? t('inbox.handed.none', { why: handedWhy(a) }) : t('inbox.handed.to', { agent: name, why: handedWhy(a) });
}
