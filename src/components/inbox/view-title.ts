// What a line of the box says of a view of Escouade's (ESC-513): its title, and the label of a gesture.
import { t, type Key } from '../../lib/i18n';
import type { Gesture, InboxItem } from '../../lib/inbox/views';

/** The view's title: the request, the ticket's, the decision's; for the docs, how many files; for a plan its first line. */
export function viewTitle(v: InboxItem): string {
  switch (v.source) {
    case 'docs':
      return t('inboxViews.docsTitle', { count: v.files });
    case 'plan':
      return v.title || t('inboxViews.planTitle');
    case 'review':
      return t('inboxViews.prTitle', { word: v.word ?? 'PR', number: v.number ?? 0, title: v.title });
    default:
      return v.title;
  }
}

/** A gesture's label: « Ouvrir la PR » names the host's word. */
export function gestureLabel(v: InboxItem, g: Gesture): string {
  if (g.id === 'openPr') return t('inboxViews.act.openPr', { word: v.word ?? 'PR' });
  // The others take nothing.
  return (t as (k: Key) => string)(g.label);
}
