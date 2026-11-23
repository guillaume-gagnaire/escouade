// The gestures of a view of the box « À toi » (ESC-513): those of its source, done the way its own place does them.
// In the box, through `inbox_view_act` on the view the user read (one settled meanwhile is refused and the box reads
// itself again); « Valider » stops the ticket's test launches first and asks before losing the files open in the
// editor, as the card of the board does. The others open their place: the conversation, the pull request, the board.
import { buffers, lossNotice } from '../editor/buffers.svelte';
import { APPROVE_LABEL } from '../board';
import { t } from '../i18n';
import type { IconName } from '../icons';
import { openUrl } from '../opener';
import { app } from '../state.svelte';
import { stopTests } from '../test-launch.svelte';
import { actOnView, reopenView, snoozeView, type Gesture, type InboxItem, type ViewAct } from './views';
import { inboxViews } from './views.svelte';
import { takeHandoff } from '../handoff';

/** Sends `act` on `v`; the box is read again whatever came of it. Whether it went. */
export async function act(v: InboxItem, a: ViewAct): Promise<boolean> {
  const done = await app.run(actOnView(v.key, a).then(() => true));
  await inboxViews.load();
  return !!done;
}

/** Shows the conversation of the view's agent, the box closed. */
export function goToAgent(v: InboxItem) {
  if (!v.agentId) return;
  app.closeInbox();
  app.selectAgent(v.agentId);
  app.closeEditor(v.projectId);
}

/** Shows the place of the view: its agent, its ticket on the board, its decision in Docs, its pull request. */
export function goToSource(v: InboxItem) {
  switch (v.source) {
    case 'permission':
    case 'question':
    case 'plan':
      return goToAgent(v);
    case 'ticketReview':
    case 'ticketBlocked':
    case 'handoff':
      app.closeInbox();
      return app.openBoard(v.projectId);
    case 'decision':
      app.closeInbox();
      return app.openDocs(v.projectId, 'decisions');
    case 'docs':
      app.closeInbox();
      return app.openDocs(v.projectId);
    case 'review':
      if (v.url) void app.run(openUrl(v.url));
  }
}

/** Where the bar of the box leads for a view: its agent, its board, Docs; none for a review (its gesture is the link). */
export function placeOf(v: InboxItem): { label: string; icon: IconName } | null {
  switch (v.source) {
    case 'permission':
    case 'question':
    case 'plan':
      return v.agentId ? { label: t('inboxViews.act.agent'), icon: 'agents' } : null;
    case 'ticketReview':
    case 'ticketBlocked':
    case 'handoff':
      return { label: t('inboxViews.act.board'), icon: 'kanban' };
    case 'decision':
    case 'docs':
      return { label: t('inboxViews.act.docs'), icon: 'file' };
    case 'review':
      return null;
  }
}

/** « Valider »: as the board's card does it. */
function approve(v: InboxItem): Promise<boolean> {
  const ticket = v.ticketId ? app.tickets[v.ticketId] : undefined;
  const project = app.projects.find((p) => p.id === v.projectId);
  const send = () => {
    if (ticket?.agentId) stopTests(ticket.agentId);
    return act(v, { act: 'approve' });
  };
  const s = project?.board;
  const lost = s?.action === 'merge' && s.cleanup && ticket?.agentId ? buffers.unsavedIn(v.projectId, ticket.agentId) : 0;
  if (!lost) return send();
  return new Promise((resolve) => {
    app.modal = {
      kind: 'confirm',
      title: t('board.card.approveTitle', { key: v.ticketKey ?? '' }),
      body: t('board.card.approveBody') + lossNotice(lost),
      confirm: APPROVE_LABEL.merge,
      danger: true,
      onConfirm: () => void send().then(resolve),
      onCancel: () => resolve(false),
    };
  });
}

/** What a gesture with nothing more to say does. Whether it went (one that opens a place: true). */
export function run(v: InboxItem, g: Gesture): Promise<boolean> {
  switch (g.id) {
    case 'allow':
    case 'always':
    case 'resume':
    case 'resolve':
    case 'dismiss':
    case 'accept':
    case 'share':
      return act(v, { act: g.id });
    case 'retarget':
      return act(v, { act: 'retarget', target: v.otherLine ?? '' });
    case 'deny':
      return act(v, { act: 'deny', message: null });
    case 'approve':
      return approve(v);
    // « Prendre » an agent passed (TS-17): on its harness when this machine has it, else the one the user picks.
    case 'take':
      return takeHandoff(v.ticketId ?? '', (engine) => act(v, { act: 'take', engine }));
    case 'inConversation':
      goToAgent(v);
      return Promise.resolve(true);
    case 'openPr':
      goToSource(v);
      return Promise.resolve(true);
    // These need a text: their pane asks for it.
    case 'answer':
    case 'reject':
    case 'test':
      return Promise.resolve(false);
  }
}

/** « Plus tard » until `until` (none: back in the box now). */
export async function later(v: InboxItem, until: number | null): Promise<void> {
  await app.run(snoozeView(v.key, until));
  await inboxViews.load();
}

/** « Remettre dans la boîte » (ESC-557): what the squad lead took in hand or ticked as settled waits for the user again. */
export async function putBack(v: InboxItem): Promise<boolean> {
  const done = await app.run(reopenView(v.key).then(() => true));
  await inboxViews.load();
  return !!done;
}
