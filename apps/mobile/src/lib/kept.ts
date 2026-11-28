/**
 * What the phone keeps between two openings (M13, `docs/MOBILE.md` § 5), in the app's private storage (the WebView's,
 * which neither Android's backup nor another app reads): the last glance of each space — its numbers, the titles of
 * its requests, the names of its agents and projects — so that the app opens on what it knew, dated, rather than on an
 * empty screen; the drafts (a message, a word, an answer); the lines already seen (their keys alone).
 *
 * **Never** a conversation, a command, a file, a token: the glance is kept without what an agent asks (`request` keeps
 * its kind and its tool, not its argument, its reason, its questions), and a request's body never comes here.
 */

import type { Glance } from '$shared/lib/spaces.svelte';

const GLANCES = 'escouade.glances';
const DRAFTS = 'escouade.drafts';
const SEEN = 'escouade.seen';
/** The most lines whose « seen » is kept: the oldest go first. */
const SEEN_MAX = 400;

function read<T>(key: string, empty: T): T {
  try {
    const raw = globalThis.localStorage?.getItem(key);
    return raw ? (JSON.parse(raw) as T) : empty;
  } catch {
    return empty;
  }
}

function write(key: string, value: unknown): void {
  try {
    globalThis.localStorage?.setItem(key, JSON.stringify(value));
  } catch {
    // A private window or a full storage: the app works on without it.
  }
}

/** A glance as it may be kept: what an agent asks taken off, to its kind and its tool. */
export function keptGlance(g: Glance): Glance {
  return {
    ...g,
    agents: g.agents.map((a) => ({
      ...a,
      activity: null,
      request: a.request
        ? {
            id: a.request.id,
            kind: a.request.kind,
            tool: a.request.tool,
            arg: '',
            description: null,
            reason: null,
            questions: [],
            cut: true,
            defaultNo: a.request.defaultNo,
          }
        : null,
    })),
  };
}

/** The last glance of each space kept, and when it was read. */
export function keptGlances(): Record<string, { glance: Glance; at: number }> {
  return read(GLANCES, {});
}

export function keepGlance(space: string, glance: Glance, at: number): void {
  write(GLANCES, { ...keptGlances(), [space]: { glance: keptGlance(glance), at } });
}

/** A space removed from the phone: nothing of it stays. */
export function forgetKept(space: string): void {
  const all = keptGlances();
  delete all[space];
  write(GLANCES, all);
  const drafts = read<Record<string, string>>(DRAFTS, {});
  for (const k of Object.keys(drafts)) if (k.startsWith(`${space}:`)) delete drafts[k];
  write(DRAFTS, drafts);
}

/** A draft (`<space>:<what>`), empty when none. */
export const draftOf = (key: string): string => read<Record<string, string>>(DRAFTS, {})[key] ?? '';

export function keepDraft(key: string, text: string): void {
  const drafts = read<Record<string, string>>(DRAFTS, {});
  if (text.trim()) drafts[key] = text;
  else delete drafts[key];
  write(DRAFTS, drafts);
}

/** The lines seen, by their key: the mark (their state) they had when they were. */
export const keptSeen = (): Record<string, string> => read(SEEN, {});

export function keepSeen(seen: Record<string, string>): void {
  const keys = Object.keys(seen);
  const kept = keys.length > SEEN_MAX ? Object.fromEntries(keys.slice(-SEEN_MAX).map((k) => [k, seen[k]])) : seen;
  write(SEEN, kept);
}
