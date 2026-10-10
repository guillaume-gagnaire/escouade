// What differs between Windows and macOS in the interface: the main modifier key and how
// shortcuts are written.
import { t } from './i18n';

/** True on macOS. */
export const IS_MAC = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform || navigator.userAgent);

/** The app's shortcut modifier is held alone: Cmd on macOS, Ctrl elsewhere (for a key or a click). */
export function primaryKey(e: { ctrlKey: boolean; metaKey: boolean }, mac = IS_MAC): boolean {
  return mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
}

/** A key of a shortcut, named in the language of the interface: given in either (`Shift` or `Maj`, `Enter` or `Entrée`). */
function keyName(key: string): string {
  if (/^(Shift|Maj)$/.test(key)) return t('format.keys.shift');
  if (/^(Enter|Entrée)$/.test(key)) return t('format.keys.enter');
  if (/^(Esc|Escape|Échap)$/.test(key)) return t('format.keys.escape');
  return key;
}

/**
 * A shortcut as written on this system and in the language of the interface: `Ctrl+Shift+F` (or `Ctrl+Maj+F`) is
 * `Ctrl+Maj+F` in French, `Ctrl+Shift+F` in English, `⇧⌘F` on macOS. `Ctrl+Tab` stays as is there: Cmd+Tab
 * switches applications.
 */
export function keyLabel(shortcut: string, mac = IS_MAC): string {
  const keys = shortcut.split('+');
  if (!mac || keys[0] !== 'Ctrl' || /Tab$/.test(shortcut)) return keys.map(keyName).join('+');
  const shift = /^(Shift|Maj)$/.test(keys[1] ?? '');
  const rest = keys.slice(shift ? 2 : 1).map(keyName);
  return (shift ? '⇧⌘' : '⌘') + rest.join('+');
}
