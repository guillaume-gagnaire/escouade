// The messages typed during a turn, held by the window until their delay is over or the turn ends (Cursor, OpenCode),
// so they can be edited or cancelled. The backend only hears of one once it goes.

import type { DraftAttachment } from './attachments';
import { flushDrafts, giveBack } from './drafts';
import { conversationOf } from './conversations.svelte';
import { capsOf } from './engines';
import { api } from './ipc';
import { readPref, removePref, writePref } from './prefs';
import { reportRefusal, sendDraft } from './send.svelte';
import { app, atWork } from './state.svelte';
import type { Agent } from './types';

export type HoldMode = 'delay' | 'turnEnd';

export interface Held {
  id: string;
  agentId: string;
  body: string;
  files: DraftAttachment[];
  mode: HoldMode;
  /** When it goes (ms since the epoch); null when it waits for the end of the turn. */
  at: number | null;
}

const KEPT = 'held';
let held = $state.raw<Record<string, Held[]>>({});
const timers = new Map<string, ReturnType<typeof setTimeout>>();
/** Each agent's sends, one after the other: a message never overtakes one held before it. */
const chains = new Map<string, Promise<void>>();
let seq = 0;

export function heldOf(agentId: string): Held[] {
  return held[agentId] ?? [];
}

/** Resolves once the messages of `agentId` let go of are sent or refused. */
export function settled(agentId: string): Promise<void> {
  return chains.get(agentId) ?? Promise.resolve();
}

/** How a message typed now for `agent` is held, or null when it goes at once. Behind a held one, it is held the same way. */
export function holdModeOf(agent: Agent): HoldMode | null {
  const before = heldOf(agent.id).at(-1);
  if (before) return before.mode;
  if (!atWork(agent) || agent.setup || agent.worktreePending) return null;
  const midTurn = capsOf(agent.engine)?.midTurn;
  switch (midTurn) {
    case 'turnEnd':
      return 'turnEnd';
    case 'queued':
    case 'immediate':
      return app.settings.midTurnDelaySecs > 0 ? 'delay' : null;
    default:
      return null;
  }
}

export function hold(agentId: string, body: string, files: DraftAttachment[], mode: HoldMode): Held {
  // Never before the one held before it, whatever the delay is now.
  const after = heldOf(agentId).at(-1)?.at ?? 0;
  const at = mode === 'delay' ? Math.max(Date.now() + Math.max(0, app.settings.midTurnDelaySecs) * 1000, after) : null;
  const h: Held = { id: `held-${++seq}`, agentId, body, files, mode, at };
  put(agentId, [...heldOf(agentId), h]);
  if (at !== null)
    timers.set(
      h.id,
      setTimeout(() => sendNow(h.id), at - Date.now()),
    );
  return h;
}

/** Sends the message `id` now, after the ones held before it. */
export function sendNow(id: string) {
  const h = find(id);
  if (!h) return;
  const list = heldOf(h.agentId);
  for (const m of list.slice(0, list.indexOf(h) + 1)) release(m);
}

export function editHeld(id: string) {
  const h = find(id);
  if (!h) return;
  drop(h);
  giveBack(h.agentId, h.body, h.files);
  app.focusComposer++;
}

export function cancelHeld(id: string) {
  const h = find(id);
  if (!h) return;
  drop(h);
  for (const f of h.files) if (f.file && !f.file.origin) api.attachDiscard(h.agentId, f.file.path).catch(() => {});
}

/** Gives every message held for `agentId` back to its draft, in their order, sending none. */
export function returnHeld(agentId: string) {
  // `giveBack` puts each before the draft: newest first, so the oldest ends at its head.
  for (const h of [...heldOf(agentId)].reverse()) {
    drop(h);
    giveBack(agentId, h.body, h.files);
  }
}

function find(id: string): Held | undefined {
  for (const list of Object.values(held)) {
    const h = list.find((m) => m.id === id);
    if (h) return h;
  }
}

function put(agentId: string, list: Held[]) {
  const { [agentId]: _, ...rest } = held;
  const had = agentId in held;
  held = list.length ? { ...rest, [agentId]: list } : rest;
  // The backend then skips the « done » alert and holds off an update's restart.
  if (had !== list.length > 0) api.setHeld(Object.keys(held)).catch(() => {});
  keep();
}

/** The texts held, in the preferences: a quit or a crash before they go leaves them for the next start. */
function keep() {
  const texts = Object.entries(held)
    .map(([agentId, list]) => [
      agentId,
      list
        .map((h) => h.body)
        .filter(Boolean)
        .join('\n\n'),
    ])
    .filter(([, text]) => text);
  if (texts.length) writePref(KEPT, JSON.stringify(Object.fromEntries(texts)));
  else removePref(KEPT);
}

/** Gives the texts held when the app last stopped back to their drafts. */
export function restoreHeld() {
  const kept = readPref(KEPT);
  if (!kept) return;
  removePref(KEPT);
  try {
    for (const [agentId, text] of Object.entries(JSON.parse(kept) as Record<string, string>)) giveBack(agentId, text, []);
  } catch {
    // Not written by this version: dropped.
  }
}

function drop(h: Held) {
  clearTimeout(timers.get(h.id));
  timers.delete(h.id);
  put(
    h.agentId,
    heldOf(h.agentId).filter((m) => m.id !== h.id),
  );
}

function release(h: Held) {
  drop(h);
  const before = chains.get(h.agentId) ?? Promise.resolve();
  const mine = before.then(() => deliver(h)).catch((e) => console.error(e));
  chains.set(h.agentId, mine);
  void mine.then(() => {
    if (chains.get(h.agentId) === mine) chains.delete(h.agentId);
  });
}

async function deliver(h: Held) {
  const agent = app.agents[h.agentId];
  if (!agent) return;
  if (agent.archived) return giveBack(h.agentId, h.body, h.files);
  try {
    await sendDraft(h.agentId, h.body, h.files);
    conversationOf(h.agentId).noAccount = null;
  } catch (e) {
    if (!app.agents[h.agentId]) return;
    giveBack(h.agentId, h.body, h.files);
    reportRefusal(h.agentId, e);
    // Refused, it started no turn: the next one waiting for a turn end goes now.
    if (h.mode === 'turnEnd' && !atWork(app.agents[h.agentId])) turnEnded(h.agentId);
  }
}

function turnEnded(agentId: string) {
  if (app.agents[agentId]?.archived) return returnHeld(agentId);
  const first = heldOf(agentId).find((m) => m.mode === 'turnEnd');
  if (first) release(first);
}

/** The window closing or reloading: what is held goes to the drafts. */
function returnAll() {
  for (const agentId of Object.keys(held)) returnHeld(agentId);
  flushDrafts();
}

restoreHeld();
app.onTurnEnded(turnEnded);
app.onAgentRemoved((agentId) => {
  for (const h of heldOf(agentId)) drop(h);
});
if (typeof window !== 'undefined') window.addEventListener('pagehide', returnAll);
