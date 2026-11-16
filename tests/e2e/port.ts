// The DevTools port of the app under test (fixture.ts): a test must never attach to the instance of
// the test before, which is dying but still answers on it.

import net from 'node:net';

/** How long a connection may take to be answered, which a closed port does (refused) in about two seconds on Windows. */
const ANSWER_MS = 5000;

/** Whether something accepts connections on 127.0.0.1:`port` (an unanswered attempt counts as yes). */
function isListening(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = net.connect({ host: '127.0.0.1', port });
    socket.setTimeout(ANSWER_MS);
    socket.once('connect', () => {
      socket.destroy();
      resolve(true);
    });
    socket.once('timeout', () => {
      socket.destroy();
      resolve(true);
    });
    socket.once('error', () => resolve(false));
  });
}

/**
 * Waits until nothing listens on `port` any more: true then, false when something still does after
 * `timeoutMs` (30 s by default).
 */
export async function waitForPortToClose(
  port: number,
  { timeoutMs = 30_000, intervalMs = 250 }: { timeoutMs?: number; intervalMs?: number } = {},
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (await isListening(port)) {
    if (Date.now() >= deadline) return false;
    await new Promise((r) => setTimeout(r, intervalMs));
  }
  return true;
}

/**
 * A port nothing listens on, nor on the `span - 1` after it (a second app's): drawn by `draw` again while it is taken, at
 * most `tries` times (20 by default). Outside CI, the DevTools port of a test is drawn at random, and another worktree's
 * e2e may run an app on it: waiting for that one to stop would be waiting for nothing (ESC-496).
 */
export async function freePort(draw: () => number, { tries = 20, span = 1 }: { tries?: number; span?: number } = {}): Promise<number> {
  const taken: number[] = [];
  for (let i = 0; i < tries; i++) {
    const port = draw();
    const busy = await Promise.all(Array.from({ length: span }, (_, k) => isListening(port + k)));
    if (!busy.some(Boolean)) return port;
    taken.push(port);
  }
  throw new Error(`no free DevTools port: ${tries} ports tried, all taken (${taken.join(', ')})`);
}
