import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend, resetApp } from '../test/ipc';
import { watchPresence } from './presence';
import { app } from './state.svelte';
import { flows } from './test-launch.svelte';

// The launches' terminals (xterm) are not what this is about.
vi.mock('./terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

describe('watchPresence', () => {
  beforeEach(() => {
    resetApp();
    flows.all = {};
    vi.useFakeTimers({ now: Date.UTC(2026, 9, 10, 9, 0, 0) });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('tells the backend what an automatic restart waits for, each time it changes', async () => {
    const backend = fakeBackend();
    const sent = () => backend.called('update_presence').map((c) => c.args.presence);
    const start = Date.now();
    const stop = watchPresence();
    await vi.advanceTimersByTimeAsync(0);
    expect(sent()).toEqual([{ modal: false, testing: false, activeAt: start }]);

    // Nothing new: nothing sent.
    await vi.advanceTimersByTimeAsync(20_000);
    expect(sent()).toHaveLength(1);

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' }));
    const typed = Date.now();
    await vi.advanceTimersByTimeAsync(5_000);
    expect(sent().at(-1)).toEqual({ modal: false, testing: false, activeAt: typed });

    app.modal = { kind: 'newProject' };
    flows.all.a1 = { lines: [], phase: 'running', error: null, opened: null };
    await vi.advanceTimersByTimeAsync(5_000);
    expect(sent().at(-1)).toEqual({ modal: true, testing: true, activeAt: typed });

    // A test whose servers answer is no longer being prepared.
    flows.all.a1.phase = 'ready';
    await vi.advanceTimersByTimeAsync(5_000);
    expect(sent().at(-1)).toMatchObject({ testing: false });

    stop();
    window.dispatchEvent(new Event('pointerdown'));
    await vi.advanceTimersByTimeAsync(60_000);
    expect(sent()).toHaveLength(4);
  });

  it('tells at once the first input of a restart’s countdown, which then cannot restart the app under the user', async () => {
    const backend = fakeBackend();
    const sent = () => backend.called('update_presence').map((c) => c.args.presence.activeAt);
    const stop = watchPresence();
    await vi.advanceTimersByTimeAsync(1_000);
    expect(sent()).toHaveLength(1);

    // No countdown: told at the next report, within 5 s.
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' }));
    const typed = Date.now();
    await vi.advanceTimersByTimeAsync(0);
    expect(sent()).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(4_000);
    expect(sent()).toEqual([expect.any(Number), typed]);

    // The app restarts by itself in 30 s: the user back at the mouse is told at once.
    app.restartAt = Date.now() + 30_000;
    await vi.advanceTimersByTimeAsync(1_000);
    window.dispatchEvent(new Event('pointermove'));
    const back = Date.now();
    await vi.advanceTimersByTimeAsync(0);
    expect(sent().at(-1)).toBe(back);
    expect(sent()).toHaveLength(3);
    // Once per countdown: the mouse moving on is told at the next report, as usual.
    await vi.advanceTimersByTimeAsync(500);
    window.dispatchEvent(new Event('pointermove'));
    const moved = Date.now();
    await vi.advanceTimersByTimeAsync(0);
    expect(sent()).toHaveLength(3);
    await vi.advanceTimersByTimeAsync(5_000);
    expect(sent().at(-1)).toBe(moved);

    // Called off, then warned again: its first input is told at once too.
    app.restartAt = null;
    await vi.advanceTimersByTimeAsync(60_000);
    app.restartAt = Date.now() + 30_000;
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'b' }));
    const again = Date.now();
    await vi.advanceTimersByTimeAsync(0);
    expect(sent().at(-1)).toBe(again);
    stop();
  });
});
