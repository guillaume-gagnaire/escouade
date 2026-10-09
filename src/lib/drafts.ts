// What is typed in each agent's composer. The text is kept in the local preferences, so that it
// survives a restart, an update or a reload. Attachments stay in memory: they are heavy, and the
// files they were read from may have moved by then.

import type { DraftAttachment } from './attachments';
import { readPref, removePref, writePref } from './prefs';
import { app } from './state.svelte';

export interface Draft {
  text: string;
  files: DraftAttachment[];
}

/** Wait after the last keystroke before the text is written: typing is not a write per key. */
export const SAVE_DELAY = 300;

/** The drafts of this session; one still absent from here may be in the preferences. */
const live = new Map<string, Draft>();
/** The writes waiting for the end of the typing, by agent. */
const timers = new Map<string, ReturnType<typeof setTimeout>>();

const pref = (agentId: string) => `draft.${agentId}`;

export function getDraft(agentId: string): Draft {
  return live.get(agentId) ?? { text: readPref(pref(agentId)) ?? '', files: [] };
}

/** Records the draft as it is now: the text is written once the typing pauses, and removed as soon as it is empty. */
export function setDraft(agentId: string, draft: Draft) {
  if (draft.text || draft.files.length) live.set(agentId, draft);
  else live.delete(agentId);
  clearTimeout(timers.get(agentId));
  if (!draft.text) {
    timers.delete(agentId);
    removePref(pref(agentId));
    return;
  }
  timers.set(
    agentId,
    setTimeout(() => {
      timers.delete(agentId);
      save(agentId);
    }, SAVE_DELAY),
  );
}

/** Drops the draft of an agent that is gone, and the write it was waiting for. */
export function forgetDraft(agentId: string) {
  live.delete(agentId);
  clearTimeout(timers.get(agentId));
  timers.delete(agentId);
  removePref(pref(agentId));
}

function save(agentId: string) {
  const text = live.get(agentId)?.text ?? '';
  // A draft read back from the preferences and not changed since is not written again.
  if (text && readPref(pref(agentId)) !== text) writePref(pref(agentId), text);
}

/** Writes at once the drafts still waiting for the end of the typing. */
export function flushDrafts() {
  for (const [agentId, timer] of timers) {
    clearTimeout(timer);
    save(agentId);
  }
  timers.clear();
}

// The window closing or reloading, or the app about to restart for an update, must not take the
// last keystrokes with it.
if (typeof window !== 'undefined') window.addEventListener('pagehide', flushDrafts);
app.onRestartWarned(flushDrafts);

app.onAgentRemoved(forgetDraft);
