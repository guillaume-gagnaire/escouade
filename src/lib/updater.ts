// Auto-update through GitHub Releases (tauri-plugin-updater).

import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { app } from './state.svelte';

const FIRST_CHECK_MS = 8000;
const CHECK_EVERY_MS = 5 * 60_000;

// The latest update found: it holds the proxy and the signature of its check, so installing it
// follows a proxy changed since or a release published again.
let latest: Update | null = null;
let installing = false;

function free(update: Update | null) {
  update?.close().catch(() => {});
}

async function install() {
  const update = latest!;
  installing = true;
  try {
    await update.downloadAndInstall();
    await relaunch();
  } finally {
    installing = false;
  }
}

export async function checkForUpdate(manual = false): Promise<boolean> {
  try {
    const proxy = app.settings.proxyUrl?.trim() || undefined;
    const update = await check({ proxy, timeout: 20000 });
    // The update being installed stays: the app relaunches once it is in.
    if (installing) {
      free(update);
      return true;
    }
    free(latest);
    latest = update;
    if (!update) {
      app.update = null;
      return false;
    }
    if (app.update?.version !== update.version) {
      app.update = { version: update.version, notes: update.body ?? '', install };
    }
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
