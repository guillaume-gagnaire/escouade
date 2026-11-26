// Sending a draft, for the composer and the outbox.

import type { DraftAttachment } from './attachments';
import { conversationOf } from './conversations.svelte';
import { giveBack } from './drafts';
import { t } from './i18n';
import { api } from './ipc';
import { hold, holdModeOf, settled, type HoldMode } from './outbox.svelte';
import { errorText, missingAccountOf, missingAccountProjectOf } from './refusal';
import { app } from './state.svelte';
import type { Agent, ConvItem, QuestionItem } from './types';

export function sendDraft(agentId: string, body: string, files: DraftAttachment[]): Promise<void> {
  return api.sendMessage(
    agentId,
    body,
    files.filter((f) => f.kind !== 'file').map(({ name, mediaType, data }) => ({ name, mediaType, data })),
    files.filter((f) => f.file).map((f) => $state.snapshot(f.file!)),
  );
}

/** Says why a message was refused: in its conversation for want of an account (choice 19), else in a toast. */
export function reportRefusal(owner: string, e: unknown) {
  const missing = missingAccountOf(e);
  if (missing) conversationOf(owner).noAccount = { engine: missing, text: errorText(e), project: missingAccountProjectOf(e) };
  else app.toast(errorText(e), 'error');
}

/** The request the agent waits on, as its conversation holds it; none without one. */
export function pendingItemOf(agent: Agent): ConvItem | undefined {
  return agent.pending.length ? conversationOf(agent.id).items.find((i) => i.id === agent.pending[0]) : undefined;
}

/** Where a text typed for `agent` goes: the request it waits on, the outbox, or the core at once. */
export type Route = { kind: 'answer'; item: ConvItem } | { kind: 'hold'; mode: HoldMode } | { kind: 'send' };

export function routeOf(agent: Agent, body: string, pending: ConvItem | undefined): Route {
  // Text typed while it waits answers the question / refuses the permission.
  if (body && pending) return { kind: 'answer', item: pending };
  const mode = holdModeOf(agent);
  return mode ? { kind: 'hold', mode } : { kind: 'send' };
}

/** A message on its way, by agent; `worktree` while the core holds it until the agent's worktree is prepared. */
export const outgoing = $state<Record<string, 'sending' | 'worktree'>>({});

/** What the field keeps once a text is submitted: nothing, its files (an answer takes none), or all of it (refused). */
export type Submitted = { cleared: 'all' | 'text' | 'none'; done: Promise<void> };

/** The send under way, by agent: a message sent before it is taken goes right after it, never dropped. */
const inflight = new Map<string, Promise<void>>();

/**
 * Sends what was typed for `agent` (`owner`: the draft it was typed in, given back if refused), by the conversation's
 * path and the canvas's alike. `pending`: the request it answers, null for none (the canvas: only one its card shows).
 */
export function submitDraft(
  agent: Agent,
  owner: string,
  text: string,
  files: DraftAttachment[],
  pending: ConvItem | null = pendingItemOf(agent) ?? null,
): Submitted {
  const body = text.trim();
  // The one before waits for the worktree, which can take minutes: this one is not dropped without a word.
  if (outgoing[agent.id] === 'worktree') {
    app.toast(t('composer.send.alreadyWaiting'));
    return { cleared: 'none', done: Promise.resolve() };
  }
  const route = routeOf(agent, body, pending ?? undefined);
  if (route.kind === 'hold') {
    hold(agent.id, body, files.slice(), route.mode);
    return { cleared: 'all', done: Promise.resolve() };
  }
  const answering = route.kind === 'answer' ? route.item : undefined;
  const message = { id: agent.id, owner, body, answering, prevText: text, prevFiles: files.slice() };
  // Another one may still be on its way (its answer can show before its call returns): this one leaves the field at
  // once, as it would, and goes right after it.
  const before = inflight.get(agent.id);
  const mine = (async () => {
    if (before) await before;
    // Nor before a held message on its way.
    await settled(message.id);
    await deliver(message);
  })();
  inflight.set(agent.id, mine);
  const done = mine.then(() => {
    if (inflight.get(agent.id) === mine) inflight.delete(agent.id);
  });
  return { cleared: answering ? 'text' : 'all', done };
}

async function deliver({
  id,
  owner,
  body,
  answering,
  prevText,
  prevFiles,
}: {
  id: string;
  owner: string;
  body: string;
  answering: ConvItem | undefined;
  prevText: string;
  prevFiles: DraftAttachment[];
}) {
  outgoing[id] = 'sending';
  const conv = conversationOf(id);
  const agent = app.agents[id];
  // A new worktree is prepared first (made first, when it waits for the first message: ESC-156): the backend holds the
  // message until then, and it shows meanwhile.
  const unhold =
    !answering && (agent?.setup || agent?.worktreePending)
      ? conv.holdMessage({
          text: body,
          images: prevFiles.filter((f) => f.kind === 'image').length,
          files: prevFiles.filter((f) => f.kind !== 'image').map((f) => f.name),
        })
      : undefined;
  if (unhold) outgoing[id] = 'worktree';
  try {
    if (answering?.kind === 'question') {
      const q = answering as QuestionItem;
      await api.answerQuestion(id, q.id, Object.fromEntries(q.questions.map((x) => [x.question, body])));
    } else if (answering?.kind === 'permission') {
      await api.answerPermission(id, answering.id, 'deny', body);
    } else {
      await sendDraft(id, body, prevFiles);
    }
    if (answering && prevFiles.length) {
      app.toast(t('composer.send.filesKept'));
    }
    conv.noAccount = null;
  } catch (e) {
    // The files of an answer never left the field.
    giveBack(owner, prevText, answering ? [] : prevFiles);
    // For want of an account of its harness: said in the conversation, with the way out (choice 19).
    reportRefusal(owner, e);
  }
  // Recorded by now, or given back to the field above.
  unhold?.();
  delete outgoing[id];
}
