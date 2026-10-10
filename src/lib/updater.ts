// Auto-update through GitHub Releases (tauri-plugin-updater, driven by the backend: see update-check).
// A version found downloads at once, its signature checked, and the backend keeps it: it installs with a
// restart (« Redémarrer maintenant », or the backend's own once the app is at rest), or when the app is closed.

import { check, type FoundUpdate } from './update-check';
import { app } from './state.svelte';

const FIRST_CHECK_MS = 8000;
const CHECK_EVERY_MS = 5 * 60_000;

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
    // The backend restarts no more by itself for it: the user tries again, or not.
    if (app.failedUpdate === update.version) app.tellFailedUpdate(update.version);
  } catch (e) {
    if (latest !== update) return;
    // Downloaded again at the next check.
    forget();
    if (manual) app.toast(`Téléchargement de la mise à jour impossible : ${e}`, 'error');
  }
}

export async function checkForUpdate(manual = false): Promise<boolean> {
  try {
    const update = await check();
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
    if (manual) app.toast(`Vérification des mises à jour impossible : ${e}`, 'error');
    return false;
  }
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
