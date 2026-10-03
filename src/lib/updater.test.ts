import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetApp } from '../test/ipc';
import { app } from './state.svelte';

// What the update server announces on the next check.
const server = vi.hoisted(() => ({ check: vi.fn() }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: server.check }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));

import { checkForUpdate, watchForUpdates } from './updater';

const MINUTE = 60_000;

function release(version: string) {
  return { version, body: `Notes ${version}`, close: vi.fn(async () => {}), downloadAndInstall: vi.fn(async () => {}) };
}

beforeEach(() => {
  resetApp();
  server.check.mockReset();
});

describe('watchForUpdates', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('checks shortly after start, then every five minutes', async () => {
    vi.useFakeTimers();
    const check = vi.fn().mockResolvedValue(false);
    const stop = watchForUpdates(check);

    await vi.advanceTimersByTimeAsync(7999);
    expect(check).toHaveBeenCalledTimes(0);
    await vi.advanceTimersByTimeAsync(1);
    expect(check).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(5 * MINUTE - 1);
    expect(check).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(check).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(5 * MINUTE);
    expect(check).toHaveBeenCalledTimes(3);
    stop();
  });

  it('never starts a check while the previous one is still running', async () => {
    vi.useFakeTimers();
    let finish = () => {};
    const check = vi.fn(() => new Promise<boolean>((done) => (finish = () => done(false))));
    const stop = watchForUpdates(check);

    await vi.advanceTimersByTimeAsync(8000 + 15 * MINUTE);
    expect(check).toHaveBeenCalledTimes(1);
    finish();
    await vi.advanceTimersByTimeAsync(5 * MINUTE);
    expect(check).toHaveBeenCalledTimes(2);
    stop();
  });

  it('keeps checking after a check that failed', async () => {
    vi.useFakeTimers();
    const check = vi
      .fn()
      .mockRejectedValueOnce(new Error('offline'))
      .mockImplementationOnce(() => {
        throw new Error('broken');
      })
      .mockResolvedValue(false);
    const stop = watchForUpdates(check);

    await vi.advanceTimersByTimeAsync(8000 + 10 * MINUTE);
    expect(check).toHaveBeenCalledTimes(3);
    stop();
  });

  it('stops checking once stopped, even during a check', async () => {
    vi.useFakeTimers();
    let finish = () => {};
    const check = vi.fn(() => new Promise<boolean>((done) => (finish = () => done(false))));
    const stop = watchForUpdates(check);

    await vi.advanceTimersByTimeAsync(8000);
    stop();
    finish();
    await vi.advanceTimersByTimeAsync(60 * MINUTE);
    expect(check).toHaveBeenCalledTimes(1);

    const later = vi.fn().mockResolvedValue(false);
    watchForUpdates(later)();
    await vi.advanceTimersByTimeAsync(60 * MINUTE);
    expect(later).not.toHaveBeenCalled();
  });
});

describe('checkForUpdate', () => {
  it('shows the version found', async () => {
    server.check.mockResolvedValue(release('0.2.0'));
    expect(await checkForUpdate()).toBe(true);
    expect(app.update).toMatchObject({ version: '0.2.0', notes: 'Notes 0.2.0' });
  });

  // An update holds the proxy and the signature of its check: installing the latest one found
  // follows a proxy changed since, or a release published again.
  it('keeps the offer shown when the same version is found again, but installs the latest one found', async () => {
    const first = release('0.2.0');
    const again = release('0.2.0');
    server.check.mockResolvedValueOnce(first).mockResolvedValueOnce(again);
    await checkForUpdate();
    const shown = app.update;

    expect(await checkForUpdate()).toBe(true);
    expect(app.update).toBe(shown);
    expect(first.close).toHaveBeenCalled();

    await app.update!.install();
    expect(again.downloadAndInstall).toHaveBeenCalled();
    expect(first.downloadAndInstall).not.toHaveBeenCalled();
  });

  it('replaces the update shown by a newer one, and frees the older one', async () => {
    const older = release('0.2.0');
    const newer = release('0.3.0');
    server.check.mockResolvedValueOnce(older).mockResolvedValueOnce(newer);
    await checkForUpdate();
    await checkForUpdate();

    expect(app.update?.version).toBe('0.3.0');
    expect(older.close).toHaveBeenCalled();
    await app.update!.install();
    expect(newer.downloadAndInstall).toHaveBeenCalled();
  });

  it('leaves the update being installed alone', async () => {
    const installing = release('0.2.0');
    let finish = () => {};
    installing.downloadAndInstall.mockReturnValue(new Promise<void>((done) => (finish = done)));
    const found = [installing, release('0.2.0'), release('0.3.0'), null];
    for (const u of found) server.check.mockResolvedValueOnce(u);
    await checkForUpdate();
    const install = app.update!.install();

    for (let i = 1; i < found.length; i++) await checkForUpdate();
    expect(app.update?.version).toBe('0.2.0');
    expect(installing.close).not.toHaveBeenCalled();
    expect(found[1]!.close).toHaveBeenCalled();
    expect(found[2]!.close).toHaveBeenCalled();

    finish();
    await install;
  });

  it('stops offering a release withdrawn from the server', async () => {
    const withdrawn = release('0.2.0');
    server.check.mockResolvedValueOnce(withdrawn).mockResolvedValueOnce(null);
    await checkForUpdate();

    expect(await checkForUpdate()).toBe(false);
    expect(app.update).toBeNull();
    expect(withdrawn.close).toHaveBeenCalled();
  });

  it('says nothing when an automatic check fails, but tells the user who asked', async () => {
    server.check.mockRejectedValue(new Error('offline'));
    expect(await checkForUpdate()).toBe(false);
    expect(app.toasts).toEqual([]);

    expect(await checkForUpdate(true)).toBe(false);
    expect(app.toasts).toHaveLength(1);
    expect(app.toasts[0]).toMatchObject({ kind: 'error' });
  });
});
