import { invoke } from './transport';

/**
 * Opening outside Escouade, by the core (`outside.rs`): a link in the system's browser, a file in
 * its program, a folder in the file manager; Windows' under WSLg, where `xdg-open` usually has
 * nothing to open with. In the place of Tauri's opener, which goes to `xdg-open` there.
 */
export const openUrl = (url: string | URL): Promise<void> => invoke<void>('open_url', { url: String(url) });

/** A file in its program, or a folder in the file manager (see `openUrl`). */
export const openPath = (path: string): Promise<void> => invoke<void>('open_path', { path });
