// The driving of the rows of « Navigateurs », as the window draws it (ESC-680, AB-8 with AP-6; docs/AGENT-BROWSERS.md
// § 4 and § 9, docs/AGENT-PAGES.md § 12, direction A): who drives a row, who has the hand, the agent's last gesture
// and every one of them numbered (the conversation's rows, the pins of an agent's page's rail), « Continuer » while it
// waits, the requests to fill a password. The core holds it all (`browserdrive::Drives`) and says when it changes
// (`phase: "drive"` of a page's event, keyed by the row); the window reads it again then. The user's moves (take the
// hand, give it back, stop, continue) are the core's own: the window asks, the core stops the agent.
import { api } from './ipc';
import { app } from './state.svelte';
import type { DriveView, HandMove } from './types';

export const drives = $state<{ rows: Record<string, DriveView> }>({ rows: {} });

/** The last read asked of each row: an older answer that comes after a newer one is dropped. */
const reads = new Map<string, number>();
let seq = 0;

/** Reads the driving of the row `key` again. */
export async function refreshDrive(key: string): Promise<void> {
  const mine = ++seq;
  reads.set(key, mine);
  try {
    const v = await api.browserDriveState(key);
    if (reads.get(key) !== mine) return;
    drives.rows[key] = v;
  } catch {
    // The core does not know the row (closed meanwhile): nothing to draw.
  }
}

/** The driving of `key` while an agent drives it or the user ended it; null when nobody ever drove it. */
export function driveOf(key: string): DriveView | null {
  const v = drives.rows[key];
  return v && (v.agent || v.stopped) ? v : null;
}

/** The driving of `key` forgotten, with its row. */
export function forgetDrive(key: string): void {
  delete drives.rows[key];
  reads.delete(key);
}

/**
 * « Reprendre la main », « La rendre », « Ne plus laisser piloter », « Continuer ». The core stops the agent (or lets it
 * go on) before it answers; what it says then is drawn.
 */
export async function moveHand(key: string, hand: HandMove): Promise<void> {
  ++seq;
  reads.set(key, seq);
  try {
    drives.rows[key] = await api.browserDriveHand(key, hand);
  } catch (e) {
    app.toast(String(e), 'error');
  }
}

/** « Permettre » or « Refuser » an agent to fill a password in the row `key` (choice C2). */
export async function answerPassword(key: string, agent: string, allow: boolean): Promise<void> {
  try {
    drives.rows[key] = await api.browserDrivePassword(key, agent, allow);
  } catch (e) {
    app.toast(String(e), 'error');
  }
}

/** A pin clicked (the rail, a row of the conversation): the page goes back where its agent had put it. */
export async function revisitPin(key: string, n: number): Promise<void> {
  if (!drives.rows[key]?.pins.some((p) => p.n === n)) return;
  try {
    await api.browserDriveRevisit(key, n);
  } catch {
    // The page is not shown, or the pin is no longer kept: nothing moves.
  }
}

/** The tool of Escouade's server that drives a row, as every harness names it. */
export const DRIVE_BROWSER = 'mcp__escouade__drive_browser';

/** A call of `drive_browser` as its numbered row draws it. */
export interface DriveCall {
  /** The row it drives (`esc-412`, `browser:3`, `page:p-12`). */
  browser: string;
  action: string;
  /** Its number, once it answered. */
  step: number | null;
  /** What it did, as the core says it, once it answered. */
  what: string | null;
}

/** The JSON a tool of the browsers answered: inside its mark of a third party's data, or whole. */
function answerOf(text: string): Record<string, unknown> | null {
  const inside = /<third_party_data>\s*([\s\S]*?)\s*<\/third_party_data>/.exec(text)?.[1];
  for (const candidate of [inside, text.slice(text.indexOf('{'), text.lastIndexOf('}') + 1)]) {
    if (!candidate) continue;
    try {
      const v = JSON.parse(candidate);
      if (v && typeof v === 'object') return v as Record<string, unknown>;
    } catch {
      // the next form
    }
  }
  return null;
}

/**
 * The call `item` of `drive_browser` as its numbered row draws it; null when it is no call of it, when it failed (its
 * row of a tool says why), or when it did nothing (an answer that asks the user first).
 */
export function driveCall(item: {
  name: string;
  input: Record<string, unknown>;
  status: string;
  result?: { text?: string; isError?: boolean };
}): DriveCall | null {
  if (item.name !== DRIVE_BROWSER) return null;
  const browser = typeof item.input?.browser === 'string' ? item.input.browser : '';
  const action = typeof item.input?.action === 'string' ? item.input.action : '';
  if (!browser) return null;
  if (item.status === 'running') return { browser, action, step: null, what: null };
  if (item.status !== 'ok' || !item.result || item.result.isError) return null;
  const v = answerOf(item.result.text ?? '');
  if (!v || v.done !== true || typeof v.step !== 'number') return null;
  return { browser, action, step: v.step, what: typeof v.what === 'string' ? v.what : null };
}

/**
 * A gesture's caption, its verb apart (« Met en évidence » « « Offre Pro » »): the verb is what it did, the rest what
 * on. The core writes it as « <verbe> « <cible> » », or a verb and a number (« Défile de 300 px »).
 */
export function splitWhat(what: string): { verb: string; rest: string } {
  const at = what.search(/[«“]/);
  if (at > 0) return { verb: what.slice(0, at).trim(), rest: what.slice(at).trim() };
  return { verb: what, rest: '' };
}
