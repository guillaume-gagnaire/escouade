import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetApp } from '../test/ipc';
import { setLang } from './i18n';
import { app } from './state.svelte';

// What the update server announces on the next check, and the pages the browser was asked to open.
const server = vi.hoisted(() => ({ check: vi.fn(), opened: [] as string[] }));
// Only the check is replaced: the trouble the backend reports is the real class, the one the updater tests for.
vi.mock('./update-check', async (original) => ({ ...(await original<typeof import('./update-check')>()), check: server.check }));
vi.mock('./opener', () => ({ openUrl: vi.fn(async (url: string) => void server.opened.push(url)) }));

import { UpdateTrouble } from './update-check';
import { checkForUpdate, isBeta, shownVersion, watchForUpdates } from './updater';

const MINUTE = 60_000;
const PAGE = 'https://github.com/guillaume-gagnaire/escouade/releases/tag/v1.6.0';

/** What the backend reports of an update that did not go through, as the window receives it. */
function trouble(message: string, { show = true, page = null as string | null } = {}) {
  return new UpdateTrouble({ message, show, page });
}

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
  server.opened.length = 0;
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

  it('tells of a version that did not install at the last try, once it is ready again, with a way to try again', async () => {
    app.failedUpdate = '1.6.0';
    server.check.mockResolvedValueOnce(release('1.6.0'));
    await checkForUpdate();
    await expect.poll(() => app.update?.ready).toBe(true);
    const toast = app.toasts.at(-1)!;
    expect(toast).toMatchObject({ text: 'La mise à jour vers 1.6.0 n’a pas pu s’installer.', kind: 'error' });
    expect(toast.action?.label).toBe('Réessayer');
    toast.action!.onClick();
    expect(app.modal).toEqual({ kind: 'update' });

    // Another version says nothing of it.
    app.toasts = [];
    server.check.mockResolvedValueOnce(release('1.7.0'));
    await checkForUpdate();
    await expect.poll(() => app.update).toMatchObject({ version: '1.7.0', ready: true });
    expect(app.toasts).toEqual([]);
  });

  it('says nothing when a call of an automatic check breaks, but tells the user who asked, and gives no answer', async () => {
    server.check.mockRejectedValue(new Error('offline'));
    expect(await checkForUpdate()).toBeNull();
    expect(app.toasts).toEqual([]);

    expect(await checkForUpdate(true)).toBeNull();
    expect(app.toasts).toHaveLength(1);
    expect(app.toasts[0]).toMatchObject({ kind: 'error' });
  });

  it('tells the backend whether the user asked, for the check', async () => {
    server.check.mockResolvedValue(null);
    await checkForUpdate();
    await checkForUpdate(true);
    expect(server.check.mock.calls).toEqual([[false], [true]]);
  });
});

describe('the troubles the backend reports', () => {
  it('says nothing of a check that failed when the backend keeps it to itself, and keeps the update it had', async () => {
    const found = release('1.6.0');
    server.check.mockResolvedValueOnce(found);
    await checkForUpdate();
    await expect.poll(() => app.update?.ready).toBe(true);
    app.modal = { kind: 'update' };

    server.check.mockRejectedValueOnce(trouble('connexion impossible à github.com', { show: false }));
    expect(await checkForUpdate()).toBeNull();
    expect(app.toasts).toEqual([]);
    expect(app.update).toMatchObject({ version: '1.6.0', ready: true });
    expect(app.modal).toEqual({ kind: 'update' });

    // Still the one downloaded: found again, it is not downloaded twice.
    const again = release('1.6.0');
    server.check.mockResolvedValueOnce(again);
    expect(await checkForUpdate()).toBe(true);
    expect(found.download).toHaveBeenCalledOnce();
    expect(again.download).not.toHaveBeenCalled();
  });

  it('shows the cause of a check that failed when the backend says to, and keeps the update it had', async () => {
    server.check.mockResolvedValueOnce(release('1.6.0'));
    await checkForUpdate();
    await expect.poll(() => app.update?.ready).toBe(true);

    server.check.mockRejectedValueOnce(trouble('403 Forbidden de github.com, par le proxy 127.0.0.1:9000 (Zscaler/6.2)'));
    expect(await checkForUpdate()).toBeNull();
    expect(app.toasts).toHaveLength(1);
    expect(app.toasts[0]).toMatchObject({
      kind: 'error',
      text: 'Vérification des mises à jour impossible : 403 Forbidden de github.com, par le proxy 127.0.0.1:9000 (Zscaler/6.2)',
    });
    expect(app.toasts[0].action).toBeUndefined();
    expect(app.update).toMatchObject({ version: '1.6.0', ready: true });
  });

  it('follows the backend to tell a check, not who asked: shown by the backend, even when automatic', async () => {
    server.check.mockRejectedValue(trouble('cause', { show: true }));
    expect(await checkForUpdate()).toBeNull();
    expect(app.toasts).toHaveLength(1);

    app.toasts = [];
    server.check.mockRejectedValue(trouble('cause', { show: false }));
    expect(await checkForUpdate(true)).toBeNull();
    expect(app.toasts).toEqual([]);
  });

  it('does not take a check that failed for an app up to date', async () => {
    server.check.mockRejectedValueOnce(trouble('cause'));
    expect(await checkForUpdate(true)).toBeNull();
    server.check.mockResolvedValueOnce(null);
    expect(await checkForUpdate(true)).toBe(false);
  });

  it('forgets a download that failed in silence, and tries it again at the next check', async () => {
    const failed = release('1.6.0', { held: true });
    const next = release('1.6.0');
    server.check.mockResolvedValueOnce(failed).mockResolvedValueOnce(next);
    await checkForUpdate();
    expect(app.update).toMatchObject({ version: '1.6.0', ready: false });
    app.modal = { kind: 'update' };
    failed.finish(trouble('403 Forbidden', { show: false, page: PAGE }));
    await expect.poll(() => app.update).toBeNull();
    expect(app.modal).toBeNull();
    expect(app.toasts).toEqual([]);

    await checkForUpdate();
    expect(next.download).toHaveBeenCalledOnce();
    await expect.poll(() => app.update?.ready).toBe(true);
  });

  it('shows the cause of a download that failed when the backend says to, with a way to take it in the browser', async () => {
    const failed = release('1.6.0', { held: true });
    server.check.mockResolvedValueOnce(failed);
    await checkForUpdate();
    failed.finish(trouble('le proxy de ton entreprise (Zscaler/6.2) a refusé l’installateur', { page: PAGE }));
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ kind: 'error' });

    const toast = app.toasts.at(-1)!;
    expect(toast.text).toBe(
      'Téléchargement de la mise à jour impossible : le proxy de ton entreprise (Zscaler/6.2) a refusé l’installateur',
    );
    expect(toast.action?.label).toBe('Télécharger dans le navigateur');
    expect(app.update).toBeNull();
    expect(server.opened).toEqual([]);

    toast.action!.onClick();
    await expect.poll(() => server.opened).toEqual([PAGE]);
  });

  it('keeps a download that failed for Réglages › À propos while it lasts, gone once a download goes through (choice 13)', async () => {
    const failed = release('1.6.0', { held: true });
    server.check.mockResolvedValueOnce(failed);
    await checkForUpdate();
    failed.finish(trouble('le proxy a refusé le fichier (403)', { page: PAGE }));
    await expect.poll(() => app.updateTrouble).toEqual({ version: '1.6.0', message: 'le proxy a refusé le fichier (403)', page: PAGE });
    // Another failure, kept quiet by the backend: still failing, still said there.
    const again = release('1.6.0', { held: true });
    server.check.mockResolvedValueOnce(again);
    await checkForUpdate();
    again.finish(trouble('délai dépassé', { show: false }));
    await expect.poll(() => app.updateTrouble?.message).toBe('délai dépassé');
    expect(app.updateTrouble?.page).toBeNull();
    // Through at last.
    const ok = release('1.6.0');
    server.check.mockResolvedValueOnce(ok);
    await checkForUpdate();
    await expect.poll(() => app.update?.ready).toBe(true);
    expect(app.updateTrouble).toBeNull();
  });

  it('shows the cause of a download that failed without a button when there is no page to open', async () => {
    const failed = release('1.6.0', { held: true });
    server.check.mockResolvedValueOnce(failed);
    await checkForUpdate();
    failed.finish(trouble('404 Not Found', { page: null }));
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ kind: 'error' });
    expect(app.toasts.at(-1)?.action).toBeUndefined();
  });

  describe('how long the toast of a download that failed stays', () => {
    afterEach(() => {
      vi.useRealTimers();
    });

    /** A download that fails with `t`, the toast it leaves on screen under fake timers. */
    async function failWith(t: UpdateTrouble) {
      vi.useFakeTimers();
      const failed = release('1.6.0', { held: true });
      server.check.mockResolvedValueOnce(failed);
      await checkForUpdate();
      failed.finish(t);
      await vi.advanceTimersByTimeAsync(0);
      expect(app.toasts).toHaveLength(1);
    }

    it('stays 30 s with the button to the browser, time to read a long cause and to act', async () => {
      await failWith(trouble('le proxy de ton entreprise (Zscaler/6.2) a refusé l’installateur', { page: PAGE }));
      await vi.advanceTimersByTimeAsync(29_999);
      expect(app.toasts).toHaveLength(1);
      await vi.advanceTimersByTimeAsync(1);
      expect(app.toasts).toEqual([]);
    });

    it('stays as long as any error without that button', async () => {
      await failWith(trouble('404 Not Found', { page: null }));
      await vi.advanceTimersByTimeAsync(6_999);
      expect(app.toasts).toHaveLength(1);
      await vi.advanceTimersByTimeAsync(1);
      expect(app.toasts).toEqual([]);
    });
  });

  it('shows a download that failed when the backend says to, even if the check was automatic', async () => {
    const failed = release('1.6.0', { held: true });
    server.check.mockResolvedValueOnce(failed);
    expect(await checkForUpdate()).toBe(true);
    failed.finish(trouble('cause', { show: true, page: PAGE }));
    await expect.poll(() => app.toasts).toHaveLength(1);
  });

  it('says nothing of a download that failed after a newer update replaced it', async () => {
    const older = release('1.6.0', { held: true });
    const newer = release('1.7.0', { held: true });
    server.check.mockResolvedValueOnce(older).mockResolvedValueOnce(newer);
    await checkForUpdate();
    await checkForUpdate();
    older.finish(trouble('cause', { page: PAGE }));
    await new Promise((r) => setTimeout(r, 0));
    expect(app.toasts).toEqual([]);
    expect(app.update).toMatchObject({ version: '1.7.0', ready: false });
  });

  it('does not take for ready a download the backend dropped for a newer one, and says nothing', async () => {
    const overtaken = release('1.6.0', { held: true });
    server.check.mockResolvedValueOnce(overtaken);
    await checkForUpdate();
    overtaken.finish(false);
    await new Promise((r) => setTimeout(r, 0));
    expect(app.update).toMatchObject({ version: '1.6.0', ready: false });
    expect(app.toasts).toEqual([]);
  });
});

describe('the messages of the updater in English', () => {
  it('tells in English that a download or a check failed, to the user who asked', async () => {
    setLang('en');
    server.check.mockRejectedValue(new Error('offline'));
    expect(await checkForUpdate(true)).toBeNull();
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Couldn’t check for updates: Error: offline', kind: 'error' });

    server.check.mockReset();
    const asked = release('1.7.0', { held: true });
    server.check.mockResolvedValueOnce(asked);
    await checkForUpdate(true);
    asked.finish(new Error('502'));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Couldn’t download the update: Error: 502');
  });

  it('tells in English the cause the backend gives, with the button to the browser', async () => {
    setLang('en');
    server.check.mockRejectedValueOnce(trouble('403 Forbidden from github.com'));
    expect(await checkForUpdate(true)).toBeNull();
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Couldn’t check for updates: 403 Forbidden from github.com', kind: 'error' });

    const failed = release('1.7.0', { held: true });
    server.check.mockResolvedValueOnce(failed);
    await checkForUpdate(true);
    failed.finish(trouble('your company’s proxy refused the installer', { page: PAGE }));
    await expect.poll(() => app.toasts.at(-1)?.text).toBe('Couldn’t download the update: your company’s proxy refused the installer');
    expect(app.toasts.at(-1)?.action?.label).toBe('Download in the browser');
  });
});

describe('a beta', () => {
  beforeEach(() => setLang('fr'));

  it('is a version with a suffix, said with « bêta » after it', () => {
    expect(isBeta('2.2.0-beta.1')).toBe(true);
    expect(isBeta('2.2.0')).toBe(false);
    expect(shownVersion('2.2.0-beta.10')).toBe('2.2.0-beta.10 (bêta)');
    expect(shownVersion('2.1.4')).toBe('2.1.4');
    setLang('en');
    expect(shownVersion('2.2.0-beta.1')).toBe('2.2.0-beta.1 (beta)');
  });
});
