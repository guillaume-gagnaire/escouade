// What differs between Windows and macOS in the interface: the main modifier key and how
// shortcuts are written.

/** True on macOS. */
export const IS_MAC = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform || navigator.userAgent);

/** The app's shortcut modifier is held alone: Cmd on macOS, Ctrl elsewhere (for a key or a click). */
export function primaryKey(e: { ctrlKey: boolean; metaKey: boolean }, mac = IS_MAC): boolean {
  return mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
}

/**
 * A shortcut as written on this system: `Ctrl+Shift+F` (or `Ctrl+Maj+F`) becomes `⇧⌘F` on macOS.
 * `Ctrl+Tab` stays as is there: Cmd+Tab switches applications.
 */
export function keyLabel(shortcut: string, mac = IS_MAC): string {
  if (!mac || /Tab$/.test(shortcut)) return shortcut;
  return shortcut.replace(/^Ctrl\+(?:(?:Shift|Maj)\+)?/, (m) => (/Shift|Maj/.test(m) ? '⇧⌘' : '⌘'));
}
