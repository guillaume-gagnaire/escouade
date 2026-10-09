// The app's updates as the updater plugin's `check` gives them, but checked and downloaded by the
// backend: they follow the network settings as they are now (proxy, TLS verification off).

import { api } from './ipc';

export interface FoundUpdate {
  version: string;
  body?: string;
  /** Downloads it, its signature checked, and the backend keeps it until it installs: true once it is the one ready. */
  download(): Promise<boolean>;
  /** It is no longer offered: the backend lets go of it. */
  close(): Promise<void>;
}

export async function check(): Promise<FoundUpdate | null> {
  const found = await api.updateCheck();
  if (!found) return null;
  return {
    version: found.version,
    body: found.notes,
    download: () => api.updateDownload(found.id),
    close: () => api.updateClose(found.id),
  };
}
