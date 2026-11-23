// Waiting until a launch answers over HTTP: "▶ Tester" for each process of a recipe (and each service of isola), and a
// launch command for the address it opens (ESC-560). One request every `POLL_MS` (`http_ready`, 2 s at most each),
// `READY_LIMIT_MS` at most since `since`: never a fixed delay.

import { api } from './ipc';

/** How often a server is asked whether it is up, and for how long at most. */
export const POLL_MS = 500;
export const READY_LIMIT_MS = 3 * 60_000;

/** `up`: it answered; `ended`: what serves it is no longer running; `over`: no longer waited for; `late`: the limit passed. */
export type Wait = 'up' | 'ended' | 'over' | 'late';

export interface WaitOptions {
  /** When the wait (or the launch) started: the limit runs from it. */
  since: number;
  /** What serves it still runs (a process that ends before answering ends the wait). */
  running?: () => boolean;
  /** Not waited for any more (stopped, replaced, gone). */
  over?: () => boolean;
  /** The agent whose test launch it is, for the facts' bus (PLG-5). */
  agentId?: string;
  limitMs?: number;
  pollMs?: number;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Asks `url` until it answers, what serves it ends, the wait is over or its limit passes. */
export async function waitUntilUp(url: string, o: WaitOptions): Promise<Wait> {
  const over = o.over ?? (() => false);
  const running = o.running ?? (() => true);
  const limit = o.limitMs ?? READY_LIMIT_MS;
  for (;;) {
    if (over()) return 'over';
    if (!running()) return 'ended';
    const up = await api.httpReady(url, o.agentId).catch(() => false);
    if (over()) return 'over';
    if (up) return 'up';
    if (Date.now() - o.since >= limit) return 'late';
    await sleep(o.pollMs ?? POLL_MS);
  }
}
