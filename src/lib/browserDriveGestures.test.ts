// The user's gestures under WebKit, for the rule of the hand (ESC-679): the real scripts of src-tauri, the page's relay
// (`browserdrive-gestures.js`) run at its start and Escouade's take of it (`browserdrive.js`, `gestures`), each take in
// a world of its own that shares the page's events but none of its own globals, as an isolated world of WebKit does
// (ESC-692): there, no listener of Escouade's outlives the call that set it.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const RELAY = readFileSync(resolve(__dirname, '../../src-tauri/src/browserdrive-gestures.js'), 'utf8');
const LIB = readFileSync(resolve(__dirname, '../../src-tauri/src/browserdrive.js'), 'utf8');

type Listener = (e: { type: string; isTrusted?: boolean; detail?: unknown }) => void;

/** The page's events, shared by every world: a gesture of the user's is `isTrusted`, the page's own are not. */
function page() {
  const listeners = new Map<string, Set<Listener>>();
  const win = {
    top: null as unknown,
    CustomEvent,
    addEventListener: (t: string, f: Listener) => {
      if (!listeners.has(t)) listeners.set(t, new Set());
      listeners.get(t)!.add(f);
    },
    removeEventListener: (t: string, f: Listener) => listeners.get(t)?.delete(f),
    dispatchEvent: (e: { type: string; isTrusted?: boolean }) => {
      for (const f of [...(listeners.get(e.type) ?? [])]) f(e);
      return true;
    },
  };
  win.top = win;
  new Function('window', RELAY)(win);
  return win;
}

/** Escouade's take of the gestures, in a new world: the page's events, none of a former call's globals. */
function take(win: ReturnType<typeof page>): number {
  const world = {
    addEventListener: win.addEventListener,
    removeEventListener: win.removeEventListener,
    dispatchEvent: win.dispatchEvent,
  };
  const lib = new Function(
    'window',
    'CustomEvent',
    'location',
    'document',
    'scrollY',
    'innerWidth',
    `return (${LIB}
)`,
  )(world, CustomEvent, { href: 'http://localhost:4100/' }, { title: 'demo', readyState: 'complete' }, 0, 800) as (args: unknown) => string;
  const out = JSON.parse(lib({ run: 'gestures' }));
  expect(out.error).toBeUndefined();
  return out.gestures;
}

describe('the user’s gestures under WebKit', () => {
  it('are counted in the page and taken whole by each call, in a world of its own', () => {
    const win = page();
    expect(take(win)).toBe(0);
    win.dispatchEvent({ type: 'wheel', isTrusted: true });
    win.dispatchEvent({ type: 'pointerdown', isTrusted: true });
    // The page's own events are not the user's.
    win.dispatchEvent({ type: 'keydown', isTrusted: false });
    expect(take(win)).toBe(2);
    // Taken once: the next call starts from nothing, and still hears what comes after.
    expect(take(win)).toBe(0);
    win.dispatchEvent({ type: 'keydown', isTrusted: true });
    win.dispatchEvent({ type: 'touchstart', isTrusted: true });
    expect(take(win)).toBe(2);
  });

  it('give nothing in a frame, where the relay does not run', () => {
    const win = page();
    const frame = { ...win, top: {} };
    new Function('window', RELAY)(frame);
    win.dispatchEvent({ type: 'wheel', isTrusted: true });
    expect(take(win)).toBe(1);
  });
});
