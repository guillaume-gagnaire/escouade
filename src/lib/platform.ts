// What differs between Windows, macOS and Linux in the interface: the main modifier key, how
// shortcuts are written, the name of the file manager.
import { t } from './i18n';

/** A system the app runs on, for what the interface names after it (its file manager). */
export type Os = 'windows' | 'mac' | 'linux';

/** The system of what the WebView says of it: its `navigator.platform`, else its user agent. */
export function osOf(platform: string): Os {
  if (/Mac/i.test(platform)) return 'mac';
  // At the start of a word: « darwin » holds « win ».
  return /\bWin/i.test(platform) ? 'windows' : 'linux';
}

/** The system the app runs on. */
export const OS: Os = typeof navigator !== 'undefined' ? osOf(navigator.platform || navigator.userAgent) : 'windows';

/** True on macOS. */
export const IS_MAC = OS === 'mac';

/** True on Windows. */
export const IS_WINDOWS = OS === 'windows';

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
