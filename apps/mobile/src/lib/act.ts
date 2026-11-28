/**
 * The phone's gestures (MO-5, `docs/MOBILE.md` § 3.4 to § 3.6): each goes to the core of its space, through its own
 * connection (`callIn`), never another's; then that space is read again. An agent's request is answered through its
 * view of the box when there is one (`inbox_view_act`: a request settled meanwhile is refused, « Réglé ailleurs »),
 * else as its conversation answers it (`answer_permission`, `answer_question`).
 *
 * **The lock** (M3): the gestures that make an agent or the machine act ask the system's lock first (`confirmByLock`):
 * « Autoriser », « Approuver le plan », « Oui » to a go-ahead, « Valider » a ticket, take an agent passed, accept a task
 * that launches agents. Refusing, answering a question, choosing an option, a message, « Plus tard » do not.
 *
 * **A cut**: a call left without an answer comes back as an error, never replayed (the transport); the space is read
 * again at its return, and the line says whether the answer arrived.
 */

import { t } from '$shared/lib/i18n';
import type { InboxItem, ViewAct } from '$shared/lib/inbox/views';
import type { AskAnswerIn, AskFile, Attachment, ConvItem, WindowAsk } from '$shared/lib/types';
import { confirmByLock } from './lock';
import { callIn, refreshSpace } from './phone-spaces.svelte';
import type { SpaceRef, StoppedLine } from './thread';

/** What a gesture came to: done, cancelled at the lock (nothing went), or failed (its text). */
export type Outcome = { ok: true } | { ok: false; locked: true } | { ok: false; error: string };

const DONE: Outcome = { ok: true };
const LOCKED: Outcome = { ok: false, locked: true };

/** The text of a refusal of the core, or of a call that got no answer. */
export function failText(e: unknown): string {
  const text = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
  if (/settled|réglé|gone|not found|introuvable|plus en attente|no longer/i.test(text)) return t('inboxViews.settled');
  if (/lost|closed|timeout|injoignable|unreachable|perdu/i.test(text)) return t('mobile.answer.lost');
  return text || t('mobile.answer.failed');
}

/** The reason the system's lock shows: what goes, and where (« Autoriser la commande de tests-e2e ? · Escouade · Travail »). */
export const lockReason = (what: string, space: Pick<SpaceRef, 'name'>) => t('mobile.lock.reason', { what, space: space.name });

/** A call of `space`, then its glance and its box read again, whatever came of it. */
async function run(space: string, call: () => Promise<unknown>): Promise<Outcome> {
  try {
    await call();
    return DONE;
  } catch (e) {
    return { ok: false, error: failText(e) };
  } finally {
    void refreshSpace(space);
  }
}

/** Asks the lock with `reason` when `locked`, then runs the gesture. */
async function gated(locked: boolean, reason: string, space: string, call: () => Promise<unknown>): Promise<Outcome> {
  if (locked && !(await confirmByLock(reason))) return LOCKED;
  return run(space, call);
}

/** « Autoriser » (under the lock) or « Refuser » with a word, on a permission or a plan. */
export function answerStop(line: StoppedLine, decision: 'allow' | 'deny', message: string | null = null): Promise<Outcome> {
  const s = line.space.id;
  const plan = line.stop === 'plan';
  const what =
    decision === 'allow'
      ? plan
        ? t('mobile.lock.plan', { agent: line.agentName })
        : t('mobile.lock.allow', { agent: line.agentName })
      : '';
  const note = message?.trim() || null;
  return gated(decision === 'allow', lockReason(what, line.space), s, () =>
    line.view
      ? callIn(s, 'inbox_view_act', {
          key: line.view.key,
          act: decision === 'allow' ? { act: 'allow' } : { act: 'deny', message: note },
        })
      : callIn(s, 'answer_permission', { id: line.agentId, requestId: line.request?.id ?? '', decision, message: note }),
  );
}

/** A question answered on the spot: its question and the label(s) chosen (several joined, as the desktop does). */
export function answerQuestion(line: StoppedLine, answers: Record<string, string>): Promise<Outcome> {
  const s = line.space.id;
  return run(s, () =>
    line.view
      ? callIn(s, 'inbox_view_act', { key: line.view.key, act: { act: 'answer', answers } })
      : callIn(s, 'answer_question', { id: line.agentId, requestId: line.request?.id ?? '', answers }),
  );
}

/** Whether a gesture of a view makes an agent or the machine act (M3). */
export function viewActLocked(v: InboxItem, act: ViewAct['act']): boolean {
  if (act === 'allow' || act === 'approve' || act === 'take') return true;
  return act === 'accept' && (v.source === 'scheduled' || v.source === 'teamAccept');
}

/** A gesture of a view of the box (a ticket, a decision, the docs…), under the lock when it makes something act. */
export function actOnView(space: SpaceRef, v: InboxItem, act: ViewAct, what: string): Promise<Outcome> {
  return gated(viewActLocked(v, act.act), lockReason(what, space), space.id, () => callIn(space.id, 'inbox_view_act', { key: v.key, act }));
}

/** An answer to a request of the box, said to come from the phone; a « Oui » to a go-ahead under the lock. */
export function answerAsk(space: SpaceRef, a: WindowAsk, answer: AskAnswerIn): Promise<Outcome> {
  const locked = a.gate && answer.yes === true;
  const what = t('mobile.lock.gate', { label: a.yesLabel || t('inbox.answer.yes') });
  return gated(locked, lockReason(what, space), space.id, () =>
    callIn<WindowAsk>(space.id, 'inbox_answer', { id: a.id, answer: { ...answer, via: 'phone' }, files: [] }),
  );
}

/** « Plus tard » (until `until`, ms), on a request or a view. */
export function snooze(space: SpaceRef, target: { ask: WindowAsk } | { view: InboxItem }, until: number): Promise<Outcome> {
  return run(space.id, () =>
    'ask' in target
      ? callIn(space.id, 'inbox_snooze', { id: target.ask.id, until })
      : callIn(space.id, 'inbox_view_snooze', { key: target.view.key, until }),
  );
}

/** Back in the box: a request put off, or an answer taken back (« Revenir sur ma réponse »: while its agent has not read it). */
export const reopenAsk = (space: SpaceRef, a: WindowAsk): Promise<Outcome> =>
  run(space.id, () => callIn(space.id, 'inbox_reopen', { id: a.id }));

/** A request put off brought back now. */
export const wakeAsk = (space: SpaceRef, a: WindowAsk): Promise<Outcome> =>
  run(space.id, () => callIn(space.id, 'inbox_snooze', { id: a.id, until: null }));

/** A view put off, or marked by the squad lead, back in the box (« Remettre dans la boîte »). */
export function reopenView(space: SpaceRef, v: InboxItem): Promise<Outcome> {
  return run(space.id, () =>
    v.mark ? callIn(space.id, 'inbox_view_reopen', { key: v.key }) : callIn(space.id, 'inbox_view_snooze', { key: v.key, until: null }),
  );
}

/** A request left to an agent (the squad lead) to answer in the user's place. */
export const delegateAsk = (space: SpaceRef, a: WindowAsk, agent: string): Promise<Outcome> =>
  run(space.id, () => callIn(space.id, 'inbox_delegate', { id: a.id, agent }));

/** A request opened by the user: the core keeps it seen, on every screen (its dot goes). */
export function seeAsk(space: SpaceRef, a: WindowAsk): void {
  if (!a.seen || a.woke) void callIn(space.id, 'inbox_seen', { id: a.id }).catch(() => {});
}

/** A message to an agent, with its photos (images in the message, as the desktop's composer sends them). */
export const sendMessage = (space: SpaceRef, agent: string, text: string, attachments: Attachment[]): Promise<Outcome> =>
  run(space.id, () => callIn(space.id, 'send_message', { id: agent, text, attachments, files: [] }));

/** An agent's conversation, read whole from its core (held in memory alone, never on the phone's disk: M13). */
export const conversationOf = (space: string, agent: string): Promise<ConvItem[]> =>
  callIn<ConvItem[]>(space, 'get_conversation', { id: agent });

// ---------- the files of a request ----------

const bytes = new Map<string, Promise<Uint8Array>>();
const urls = new Map<string, Promise<string>>();

/** A file of a request, read from the core's copy by parts (`inbox_file`), once per opening of the app. */
export function fileBytes(space: string, ask: Pick<WindowAsk, 'id'>, f: Pick<AskFile, 'file' | 'size'>): Promise<Uint8Array> {
  const key = `${space}/${ask.id}/${f.file}`;
  let b = bytes.get(key);
  if (!b) {
    b = readParts(space, ask.id, f);
    b.catch(() => bytes.delete(key));
    bytes.set(key, b);
  }
  return b;
}

async function readParts(space: string, id: string, f: Pick<AskFile, 'file' | 'size'>): Promise<Uint8Array> {
  const out = new Uint8Array(f.size);
  let at = 0;
  while (at < f.size) {
    const c = await callIn<{ data: string }>(space, 'inbox_file', { id, file: f.file, offset: at, length: null });
    const part = Uint8Array.from(atob(c.data), (ch) => ch.charCodeAt(0));
    if (!part.length) break;
    out.set(part.subarray(0, Math.max(0, f.size - at)), at);
    at += part.length;
  }
  return at >= f.size ? out : out.subarray(0, at);
}

/** An image of a request as a `data:` URL, read once. */
export function imageUrl(space: string, ask: Pick<WindowAsk, 'id'>, f: AskFile): Promise<string> {
  const key = `${space}/${ask.id}/${f.file}`;
  let u = urls.get(key);
  if (!u) {
    u = fileBytes(space, ask, f).then(
      (b) =>
        new Promise<string>((ok, ko) => {
          const r = new FileReader();
          r.onload = () => ok(String(r.result));
          r.onerror = () => ko(r.error);
          r.readAsDataURL(new Blob([b as BlobPart], { type: f.mediaType }));
        }),
    );
    u.catch(() => urls.delete(key));
    urls.set(key, u);
  }
  return u;
}

/** The files read forgotten (the app went to the background: nothing of a request stays longer than needed). */
export function forgetFiles(): void {
  bytes.clear();
  urls.clear();
}
