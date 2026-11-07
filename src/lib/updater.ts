// Auto-update through GitHub Releases (tauri-plugin-updater, driven by the backend: see update-check).
// A version found downloads at once, its signature checked, and the backend keeps it: it installs with a
// restart (« Redémarrer maintenant », or the backend's own once the app is at rest), or when the app is closed.
// A check or a download that fails is retried at the next check; whether the user hears of it is the backend's
// call (`show`: asked for, or failures in a row for long enough), so a passing network failure stays quiet.

import { openUrl } from './opener';
import { t } from './i18n';
import { api } from './ipc';
import { check, UpdateTrouble, type FoundUpdate } from './update-check';
import { app } from './state.svelte';

const FIRST_CHECK_MS = 8000;
const CHECK_EVERY_MS = 5 * 60_000;
// How long a failed download's toast with the browser button stays: the cause is often long (a company proxy
// explaining itself), and the toast has no close button, so the usual 12 s of a toast with a button is too short
// to read it and then act.
const DOWNLOAD_FAILED_WITH_PAGE_MS = 30_000;

// The update downloading or downloaded: found again by a later check, it is not downloaded again.
let latest: FoundUpdate | null = null;

function free(update: FoundUpdate | null) {
  update?.close().catch(() => {});
}

/** Nothing is offered any more: the update's window goes with it. */
function forget() {
  latest = null;
  app.update = null;
  if (app.modal?.kind === 'update') app.modal = null;
}

async function download(update: FoundUpdate, manual: boolean) {
  try {
    const ready = await update.download();
    // A newer one found meanwhile, or this one withdrawn: not this one's to say.
    if (!ready || latest !== update || !app.update) return;
    app.update = { ...app.update, ready: true };
    app.updateTrouble = null;
    // The backend restarts no more by itself for it: the user tries again, or not.
    if (app.failedUpdate === update.version) app.tellFailedUpdate(update.version);
  } catch (e) {
    if (latest !== update) return;
    // Downloaded again at the next check.
    forget();
    if (e instanceof UpdateTrouble) {
      // Said in Réglages › À propos while it lasts (choice 13 of the 2.0 design), told now when the backend says so.
      app.updateTrouble = { version: update.version, message: e.message, page: e.page };
      if (e.show) tellDownloadFailed(e);
    } else if (manual) {
      // The call itself broke: only the user who asked hears of it.
      app.toast(t('shell.update.downloadFailed', { error: String(e) }), 'error');
    }
  }
}

/** The cause, and when the backend knows the page of the release, a way to take the installer from the browser. */
function tellDownloadFailed(trouble: UpdateTrouble) {
  const text = t('shell.update.downloadFailed', { error: trouble.message });
  const page = trouble.page;
  if (!page) {
    app.toast(text, 'error');
    return;
  }
  app.toast(text, 'error', { label: t('shell.update.openInBrowser'), onClick: () => app.run(openUrl(page)) }, DOWNLOAD_FAILED_WITH_PAGE_MS);
}

/**
 * Looks for an update: true when one is on offer, false when the app is up to date, null when the check failed
 * (not the same as up to date: the update it had stays on offer).
 */
export async function checkForUpdate(manual = false): Promise<boolean | null> {
  try {
    const update = await check(manual);
    if (!update) {
      free(latest);
      forget();
      return false;
    }
    if (latest?.version === update.version) {
      free(update);
      return true;
    }
    free(latest);
    latest = update;
    app.update = { version: update.version, notes: update.body ?? '', ready: false };
    // The one it showed is no longer the one a restart would install.
    if (app.modal?.kind === 'update') app.modal = null;
    // In the background: the check is over, the download takes its time.
    void download(update, manual);
    return true;
  } catch (e) {
    // The backend decides whether its own explanation is told; a call that broke is told to the user who asked only.
    if (e instanceof UpdateTrouble) {
      if (e.show) app.toast(t('shell.update.checkFailed', { error: e.message }), 'error');
    } else if (manual) {
      app.toast(t('shell.update.checkFailed', { error: String(e) }), 'error');
    }
    return null;
  }
}

/** Whether `version` is a beta: a pre-release suffix (`2.2.0-beta.1`), as the backend's `updates::accepts` has it. */
export function isBeta(version: string): boolean {
  return /^\d+\.\d+\.\d+-/.test(version);
}

/** A version as the app says it: « (bêta) » after a beta's, the app's own and an update's alike. */
export function shownVersion(version: string): string {
  return isBeta(version) ? t('shell.update.betaVersion', { version }) : version;
}

/** Seconds left before the automatic restart for the update, when one is planned. */
export function restartIn(): number | null {
  return app.restartAt === null ? null : Math.max(0, Math.ceil((app.restartAt - app.now) / 1000));
}

/** The automatic restart the backend planned for the update, put off (the status bar's and Réglages › À propos's « Plus tard »). */
export function postponeRestart() {
  app.restartAt = null;
  app.run(api.updatePostpone());
}

/**
 * Checks shortly after start, then five minutes after each check ends, so two never overlap.
 * Returns what stops the checks.
 */
export function watchForUpdates(run: () => Promise<unknown> = () => checkForUpdate()): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout>;
  const tick = async () => {
    try {
      await run();
    } catch {
      // A failed check waits for the next one.
    }
    if (!stopped) timer = setTimeout(tick, CHECK_EVERY_MS);
  };
  timer = setTimeout(tick, FIRST_CHECK_MS);
  return () => {
    stopped = true;
    clearTimeout(timer);
  };
}
