import { onServer } from './connection.svelte';
import { t } from './i18n';
import { isLoopback, onThisComputer, spaceName } from './server-moves.svelte';
import { invoke } from './transport';

/**
 * Opening outside Escouade, by the core (`outside.rs`): a link in the system's browser, a file in
 * its program, a folder in the file manager; Windows' under WSLg, where `xdg-open` usually has
 * nothing to open with. In the place of Tauri's opener, which goes to `xdg-open` there.
 *
 * On a server (ESC-538): a link always opens in this computer's browser (`open_url` is this computer's); one of the
 * server's own loopback (a test server of an agent) is relayed to this computer first, and opened at the address of
 * this computer. A path is the server's: nothing of it opens here (`openPath` refuses, saying so; the callers that show
 * a folder of a project take it to the panel of the files instead).
 */
export const openUrl = (url: string | URL): Promise<void> =>
  onServer() && isLoopback(String(url))
    ? onThisComputer(String(url)).then((here) => invoke<void>('open_url', { url: here }))
    : invoke<void>('open_url', { url: String(url) });

/**
 * « Ouvrir le dossier » of a project: the file manager of this computer; on a server (ESC-538), the panel of the files of
 * Escouade (its editor, on the project), the folder being the server's.
 */
export function openProjectFolder(p: { id: string; path: string }, show: (projectId: string) => void): Promise<void> {
  if (onServer()) {
    show(p.id);
    return Promise.resolve();
  }
  return openPath(p.path);
}

/** A file in its program, or a folder in the file manager (see `openUrl`). Never on a server: its paths are its own. */
export const openPath = (path: string): Promise<void> =>
  onServer() ? Promise.reject(t('moves.folder.noReveal', { name: spaceName() })) : invoke<void>('open_path', { path });
