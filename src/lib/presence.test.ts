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
});
