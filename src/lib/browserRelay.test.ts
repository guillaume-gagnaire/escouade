// The console relay of a browser under WebKit and Escouade's drain of it (ESC-668, ESC-692): the
// real scripts of src-tauri, run on a page made of jsdom's events. Each drain runs in a world of
// its own, which shares the page's events but none of its own globals, as an isolated world of
// WKWebView does: there, nothing it kept outlives the evaluation that made it.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const RELAY = readFileSync(resolve(__dirname, '../../src-tauri/src/browserread-relay.js'), 'utf8');
const LIB = readFileSync(resolve(__dirname, '../../src-tauri/src/browserread.js'), 'utf8');

const quiet = () => {};

/** The window of a page as the relay sees it: its events, its console, its `fetch`. */
type Win = EventTarget & {
  top: unknown;
  console: Record<'log' | 'info' | 'warn' | 'error' | 'debug', (...args: unknown[]) => void>;
  fetch?: (input: string) => Promise<unknown>;
  CustomEvent: typeof CustomEvent;
  ErrorEvent: typeof ErrorEvent;
  Event: typeof Event;
};

/** A page, its relay run at its start. */
function page(fetch?: (input: string) => Promise<unknown>): Win {
  const win = Object.assign(new EventTarget(), {
    top: null as unknown,
    console: { log: quiet, info: quiet, warn: quiet, error: quiet, debug: quiet },
    fetch,
    CustomEvent,
    ErrorEvent,
    Event,
  });
  win.top = win;
  new Function('window', 'console', RELAY)(win, win.console);
  return win;
}

/** A drain of the core, in a new world: the page's events, none of the globals of a former call. */
function drain(win: Win): { entries: Record<string, unknown>[]; dropped: number } {
  const world = {
    addEventListener: win.addEventListener.bind(win),
    removeEventListener: win.removeEventListener.bind(win),
    dispatchEvent: win.dispatchEvent.bind(win),
  };
  const lib = new Function(
    'window',
    'CustomEvent',
    `return (${LIB}
)`,
  )(world, CustomEvent) as (args: unknown) => string;
  const out = JSON.parse(lib({ run: 'drain' }));
  return { entries: out.entries.map((e: string) => JSON.parse(e)), dropped: out.dropped };
}

const settle = () => new Promise((r) => setTimeout(r, 0));

describe('the console relay under WebKit', () => {
  it('gives what the page said before and after a first drain, each drain in a world of its own', async () => {
    const win = page((input) =>
      input.includes('missing')
        ? Promise.resolve({ status: 404, url: 'http://127.0.0.1:1/missing-692' })
        : Promise.reject(new TypeError('Load failed')),
    );
    win.console.log('before-692');
    const first = drain(win);
    expect(first.entries.map((e) => e.text)).toEqual(['before-692']);

    // After the first drain: what WKWebView lost (ESC-692).
    win.console.error('after-692');
    win.dispatchEvent(
      new win.ErrorEvent('error', { message: 'Uncaught Error: exception-692', filename: 'http://127.0.0.1:1/', lineno: 3 }),
    );
    const rejection = new win.Event('unhandledrejection');
    Object.assign(rejection, { reason: new Error('rejection-692') });
    win.dispatchEvent(rejection);
    win.fetch!('/missing-692').catch(() => {});
    win.fetch!('http://127.0.0.1:9/refused-692').catch(() => {});
    await settle();

    const second = drain(win);
    expect(second.entries).toEqual([
      expect.objectContaining({ kind: 'console', level: 'error', text: 'after-692' }),
      expect.objectContaining({ kind: 'console', level: 'error', source: 'exception', text: 'Uncaught Error: exception-692', line: 3 }),
      expect.objectContaining({ kind: 'console', source: 'exception', text: 'Unhandled rejection: rejection-692' }),
      expect.objectContaining({ kind: 'network', method: 'GET', url: 'http://127.0.0.1:1/missing-692', status: 404, type: 'fetch' }),
      expect.objectContaining({ kind: 'network', url: 'http://127.0.0.1:9/refused-692', error: 'Load failed', type: 'fetch' }),
    ]);
    // Taken once: the next drain has nothing.
    expect(drain(win).entries).toEqual([]);
  });

  it('keeps 500 entries at most and counts what it dropped', () => {
    const win = page();
    for (let i = 0; i < 520; i++) win.console.log(`line-${i}`);
    const out = drain(win);
    expect(out.entries).toHaveLength(500);
    expect(out.entries[0].text).toBe('line-20');
    expect(out.dropped).toBe(20);
  });

  it('ignores what is not a batch of the relay', () => {
    const win = page();
    win.console.log('kept-692');
    // A page that answers the drain with junk of its own, before the relay does.
    win.addEventListener(
      'escouade-console-relay-drain',
      () => {
        win.dispatchEvent(new win.CustomEvent('escouade-console-relay', { detail: 'not json' }));
        win.dispatchEvent(new win.CustomEvent('escouade-console-relay', { detail: '{"entries":[1,{"a":2}]}' }));
      },
      true,
    );
    expect(drain(win).entries.map((e) => e.text)).toEqual(['kept-692']);
  });
});
