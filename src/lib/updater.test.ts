import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetApp } from '../test/ipc';
import { app } from './state.svelte';

// What the update server announces on the next check.
const server = vi.hoisted(() => ({ check: vi.fn() }));
vi.mock('./update-check', () => ({ check: server.check }));

import { checkForUpdate, watchForUpdates } from './updater';

const MINUTE = 60_000;

/** A release found, whose download ends once `finish` is called (at once by default), ready or not. */
function release(version: string, { held = false } = {}) {
  let finish: (ready?: boolean | Error) => void = () => {};
  const done = new Promise<boolean>((resolve, reject) => {
    finish = (r = true) => (r instanceof Error ? reject(r) : resolve(r));
  });
  if (!held) finish();
  return {
    version,
    body: `Notes ${version}`,
    close: vi.fn(async () => {}),
    download: vi.fn(() => done),
    finish: (r?: boolean | Error) => finish(r),
  };
}

beforeEach(async () => {
  resetApp();
  server.check.mockReset();
  // Whatever an earlier test left found or downloaded is withdrawn.
  server.check.mockResolvedValueOnce(null);
  await checkForUpdate();
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
  it('downloads the version found at once, then offers the restart that installs it', async () => {
    const found = release('1.6.0', { held: true });
    server.check.mockResolvedValue(found);
    expect(await checkForUpdate()).toBe(true);
    expect(found.download).toHaveBeenCalledOnce();
    expect(app.update).toEqual({ version: '1.6.0', notes: 'Notes 1.6.0', ready: false });

    found.finish();
    await expect.poll(() => app.update?.ready).toBe(true);
  });

  it('downloads a version once, found again while it downloads or once it is in', async () => {
    const first = release('1.6.0', { held: true });
    const again = release('1.6.0');
    const later = release('1.6.0');
    server.check.mockResolvedValueOnce(first).mockResolvedValueOnce(again).mockResolvedValueOnce(later);
    await checkForUpdate();
    expect(await checkForUpdate()).toBe(true);
    first.finish();
    await expect.poll(() => app.update?.ready).toBe(true);
    await checkForUpdate();

    expect(first.download).toHaveBeenCalledOnce();
    for (const r of [again, later]) {
      expect(r.download).not.toHaveBeenCalled();
      expect(r.close).toHaveBeenCalled();
    }
    expect(app.update).toMatchObject({ version: '1.6.0', ready: true });
  });

  it('replaces the update by a newer one, whose download it waits for', async () => {
    const older = release('1.6.0');
    const newer = release('1.7.0', { held: true });
    server.check.mockResolvedValueOnce(older).mockResolvedValueOnce(newer);
    await checkForUpdate();
    await expect.poll(() => app.update?.ready).toBe(true);
    app.modal = { kind: 'update' };
    await checkForUpdate();

    expect(app.update).toEqual({ version: '1.7.0', notes: 'Notes 1.7.0', ready: false });
    // Its window offered to restart for the older one.
    expect(app.modal).toBeNull();
    newer.finish();
    await expect.poll(() => app.update?.ready).toBe(true);
    expect(app.update?.version).toBe('1.7.0');
  });

  it('does not take for ready an older download that ends after a newer one started', async () => {
    const older = release('1.6.0', { held: true });
    const newer = release('1.7.0', { held: true });
    server.check.mockResolvedValueOnce(older).mockResolvedValueOnce(newer);
    await checkForUpdate();
    await checkForUpdate();
    older.finish();
    await new Promise((r) => setTimeout(r, 0));
    expect(app.update).toMatchObject({ version: '1.7.0', ready: false });
  });

  it('stops offering a release withdrawn from the server, and closes its window', async () => {
    const withdrawn = release('1.6.0');
    server.check.mockResolvedValueOnce(withdrawn).mockResolvedValueOnce(null);
    await checkForUpdate();
    await expect.poll(() => app.update?.ready).toBe(true);
    app.modal = { kind: 'update' };

    expect(await checkForUpdate()).toBe(false);
    expect(app.update).toBeNull();
    expect(app.modal).toBeNull();
  });

  it('tries a failed download again at the next check, telling only the user who asked', async () => {
    const failed = release('1.6.0', { held: true });
    const next = release('1.6.0');
    server.check.mockResolvedValueOnce(failed).mockResolvedValueOnce(next);
    await checkForUpdate();
    failed.finish(new Error('Download request failed with status: 502'));
    await expect.poll(() => app.update).toBeNull();
    expect(app.toasts).toEqual([]);

    await checkForUpdate();
    expect(next.download).toHaveBeenCalledOnce();
    await expect.poll(() => app.update?.ready).toBe(true);

    server.check.mockResolvedValueOnce(null);
    await checkForUpdate();
    const asked = release('1.7.0', { held: true });
    server.check.mockResolvedValueOnce(asked);
    await checkForUpdate(true);
    asked.finish(new Error('502'));
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ kind: 'error' });
    expect(app.toasts.at(-1)?.text).toContain('Téléchargement de la mise à jour impossible');
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
