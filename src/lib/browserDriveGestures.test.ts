// The user's gestures under WebKit, for the rule of the hand (ESC-679): the real scripts of src-tauri, the page's relay
// (`browserdrive-gestures.js`) run at its start and Escouade's take of it (`browserdrive.js`, `gestures`), each take in
// a world of its own that shares the page's events but none of its own globals, as an isolated world of WebKit does
// (ESC-692): there, no listener of Escouade's outlives the call that set it.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';

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

// The rule of the hand under WebKit without the guard of Windows (`TAIL_MS`, `arm`: its gestures are real inputs of the
// DevTools protocol, `isTrusted`): there, every gesture of the agent is a script of the isolated world, whose events
// are synthetic, never `isTrusted`, so the relay counts none of them (ESC-694). jsdom's dispatched events are the same.
describe('the agent’s own gestures under WebKit', () => {
  it('are never counted as the user’s', () => {
    document.body.innerHTML =
      '<button id="go">Choisir</button><input id="name"><select id="plan"><option value="free">Gratuit</option><option value="pro">Pro</option></select>';
    // Laid out, as in a page: jsdom gives every element an empty box.
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockReturnValue(new DOMRect(10, 10, 100, 20));
    vi.spyOn(window, 'scrollBy').mockImplementation(() => {});
    let clicks = 0;
    document.getElementById('go')!.addEventListener('click', () => clicks++);
    new Function('window', RELAY)(window);
    const lib = new Function(`return (${LIB}\n)`)() as (args: unknown) => string;
    const drive = (args: object) => {
      const out = JSON.parse(lib(args));
      expect(out.error, JSON.stringify(out)).toBeUndefined();
      return out;
    };
    expect(drive({ run: 'gestures' }).gestures).toBe(0);
    expect(drive({ run: 'click', target: { css: '#go' } }).found).toBe(true);
    expect(clicks).toBe(1);
    expect(drive({ run: 'fill', target: { css: '#name' }, text: 'Ada' }).found).toBe(true);
    expect((document.getElementById('name') as HTMLInputElement).value).toBe('Ada');
    drive({ run: 'key', key: 'Enter', code: 'Enter' });
    expect(drive({ run: 'choose', target: { css: '#plan' }, value: 'Pro' })).toMatchObject({ found: true, option: true });
    drive({ run: 'scroll', by: 200 });
    expect(drive({ run: 'gestures' }).gestures).toBe(0);
    vi.restoreAllMocks();
  });
});
