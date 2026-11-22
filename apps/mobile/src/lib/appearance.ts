/**
 * The phone's appearance (MO-1): the system's theme, light or dark, followed as it changes (the app has no choice of
 * its own yet), on the app's tokens (src/app.css: `data-theme` on the root); and the system it runs on, for the size
 * of what a finger touches (44 px under iOS, 48 under Android: src/mobile.css).
 */

import type { Platform } from './native';

export type Theme = 'light' | 'dark';

/** The system's theme now. */
export function systemTheme(win: Pick<Window, 'matchMedia'> = window): Theme {
  return win.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

/** The root given `theme` and `platform`. */
export function applyAppearance(theme: Theme, platform: Platform, root: HTMLElement = document.documentElement): void {
  root.dataset.theme = theme;
  root.dataset.platform = platform;
  root.style.colorScheme = theme;
}

/** `changed` called with the system's theme each time it changes; its end. */
export function followSystemTheme(changed: (theme: Theme) => void, win: Pick<Window, 'matchMedia'> = window): () => void {
  const query = win.matchMedia('(prefers-color-scheme: light)');
  const on = () => changed(query.matches ? 'light' : 'dark');
  query.addEventListener('change', on);
  return () => query.removeEventListener('change', on);
}
