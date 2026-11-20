// What Escouade puts in the box « À toi » of its own (ESC-513, docs/INBOX.md § 5): views on what already waits for the
// user elsewhere (the requests an agent stopped for, the tickets to approve and the blocked ones, the decisions
// proposed, the docs to share, the reviews asked), read from the backend (`inbox_views`), never copies. A line of the
// list is drawn from one, with the gestures of its source; a gesture goes through `inbox_view_act`, on the view the user
// read: one settled meanwhile is refused (« Réglé ailleurs »).
import { invoke } from '../transport';
import type { Key } from '../i18n';
import type { Column, PendingRequest } from '../types';

export type ViewSource = 'permission' | 'question' | 'plan' | 'ticketReview' | 'ticketBlocked' | 'decision' | 'docs' | 'review';
export type ViewGroup = 'stopped' | 'today' | 'earlier';

/** A view, as `inbox_views` gives it (`inboxviews::InboxItem`). */
export interface InboxItem {
  key: string;
  source: ViewSource;
  group: ViewGroup;
  projectId: string;
  projectName: string;
  private: boolean;
  title: string;
  since: number;
  agentId: string | null;
  agentName: string | null;
  ticketId: string | null;
  ticketKey: string | null;
  snoozedUntil: number | null;
  request: PendingRequest | null;
  column: Column | null;
  blocked: string | null;
  conflict: boolean;
  partial: boolean;
  /** A ticket of its own target (ESC-552), when it is not the Kanban’s: « vers main » on its line. */
  target?: string | null;
  /** Its validation stopped for another line (ESC-552): the branch it came from, which « Viser » offers. */
  otherLine?: string | null;
  decision: string | null;
  files: number;
  number: number | null;
  url: string | null;
  word: string | null;
}

export interface InboxViews {
  enabled: boolean;
  views: InboxItem[];
  /** The requests agents stopped for, every project. */
  blocking: number;
  /** All that waits in the box, every project: what « à toi » counts. */
  total: number;
}

/** A gesture, as `inbox_view_act` takes it. */
export type ViewAct =
  | { act: 'allow' }
  | { act: 'always' }
  | { act: 'deny'; message?: string | null }
  | { act: 'answer'; answers: Record<string, string> }
  | { act: 'approve' }
  | { act: 'reject'; comment: string }
  | { act: 'resume' }
  | { act: 'resolve' }
  | { act: 'dismiss' }
  | { act: 'retarget'; target: string }
  | { act: 'accept' }
  | { act: 'share' };

export const readViews = (projectId: string | null = null) => invoke<InboxViews>('inbox_views', { projectId });
export const actOnView = (key: string, act: ViewAct) => invoke<void>('inbox_view_act', { key, act });
export const snoozeView = (key: string, until: number | null) => invoke<void>('inbox_view_snooze', { key, until });

/** A gesture of a line, as its source has it: what it does, its label, and whether it is the line's first. */
export interface Gesture {
  id:
    | 'allow'
    | 'always'
    | 'deny'
    | 'answer'
    | 'inConversation'
    | 'approve'
    | 'reject'
    | 'resume'
    | 'resolve'
    | 'dismiss'
    | 'retarget'
    | 'accept'
    | 'share'
    | 'openPr'
    | 'test';
  label: Key;
  primary?: boolean;
  /** Done in the box (`inbox_view_act`); otherwise it opens its place (the conversation, the pull request). */
  inBox: boolean;
}

/**
 * The gestures of a view, those of its source (§ 5): the card of the conversation for a request (a question is answered
 * in the box when it has one question; otherwise in the conversation), the board's for a ticket, Docs' for a decision
 * and the docs, a link for a review.
 */
export function gesturesOf(v: InboxItem): Gesture[] {
  switch (v.source) {
    case 'permission':
      return [
        { id: 'allow', label: 'inboxViews.act.allow', primary: true, inBox: true },
        ...(v.request?.canAlways ? [{ id: 'always', label: 'inboxViews.act.always', inBox: true } as Gesture] : []),
        { id: 'deny', label: 'inboxViews.act.deny', inBox: true },
      ];
    case 'plan':
      return [
        { id: 'allow', label: 'inboxViews.act.approvePlan', primary: true, inBox: true },
        { id: 'deny', label: 'inboxViews.act.deny', inBox: true },
      ];
    case 'question':
      return answerable(v)
        ? [{ id: 'answer', label: 'inboxViews.act.answer', primary: true, inBox: true }]
        : [{ id: 'inConversation', label: 'inboxViews.act.inConversation', primary: true, inBox: false }];
    case 'ticketReview':
      return [
        { id: 'approve', label: 'inboxViews.act.approve', primary: true, inBox: true },
        { id: 'reject', label: 'inboxViews.act.reject', inBox: true },
      ];
    case 'ticketBlocked':
      if (v.conflict)
        return [
          { id: 'resolve', label: 'inboxViews.act.resolve', primary: true, inBox: true },
          { id: 'dismiss', label: 'inboxViews.act.dismiss', inBox: true },
        ];
      // Its validation stopped for another line (ESC-552): its target set to the branch it came from, then « Valider ».
      if (v.otherLine) return [{ id: 'retarget', label: 'inboxViews.act.retarget', primary: true, inBox: true }];
      return v.column === 'doing' ? [{ id: 'resume', label: 'inboxViews.act.resume', primary: true, inBox: true }] : [];
    case 'decision':
      return [{ id: 'accept', label: 'inboxViews.act.accept', primary: true, inBox: true }];
    case 'docs':
      return [{ id: 'share', label: 'inboxViews.act.share', primary: true, inBox: true }];
    case 'review':
      return [{ id: 'openPr', label: 'inboxViews.act.openPr', primary: true, inBox: false }];
  }
}

/** A question the box answers itself: one question, with options, none cut. */
export function answerable(v: InboxItem): boolean {
  const r = v.request;
  return !!r && r.questions.length === 1 && r.questions[0].options.length > 0 && !r.cut;
}

/** A view never put off: its agent waits for it. */
export const blocking = (v: InboxItem) => v.group === 'stopped';

/** The signature of what an agent says of its requests: the box is read again when it changes, not at each token. */
export function requestsMark(a: { status: string; requests?: { id: string }[] } | undefined): string {
  return a ? `${a.status}|${(a.requests ?? []).map((r) => r.id).join(',')}` : '';
}
