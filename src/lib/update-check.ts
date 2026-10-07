// The app's updates as the updater plugin's `check` gives them, but checked and installed by the
// backend: they follow the network settings as they are now (proxy, TLS verification off).

import { api } from './ipc';

export interface FoundUpdate {
  version: string;
  body?: string;
  downloadAndInstall(): Promise<void>;
  /** It is no longer offered: the backend lets go of it. */
  close(): Promise<void>;
}

export async function check(): Promise<FoundUpdate | null> {
  const found = await api.updateCheck();
  if (!found) return null;
  return {
    version: found.version,
    body: found.notes,
    downloadAndInstall: () => api.updateInstall(found.id),
    close: () => api.updateClose(found.id),
  };
}
