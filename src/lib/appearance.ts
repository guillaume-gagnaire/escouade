// « Thème » (Réglages › Général › Apparence): Système, Clair or Sombre, Système by default, which follows Windows,
// macOS or Linux (prefers-color-scheme) and its changes. Applied at once, the window with it:
//   - `data-theme` on the root gives the tokens of src/app.css their values, and `color-scheme` (scroll bars, native
//     fields and menus) follows;
//   - the window's own theme (the native title bar of macOS, the corners and shadow of Windows 11, what shows before the
//     page): the backend sets it before the window shows (lib.rs), this as it changes;
//   - the choice is kept in the page's storage as well, read before the first render (`earlyAppearance`, main.ts): the
//     settings come from the backend a moment later, and a light app never flashes dark.

import { getCurrentWindow } from '@tauri-apps/api/window';

export const THEMES = ['system', 'light', 'dark'] as const;
export type ThemeChoice = (typeof THEMES)[number];
export type Theme = 'light' | 'dark';

/** Where the page keeps the choice, for its next start. */
export const THEME_KEY = 'escouade-theme';

/** A choice as kept: Système for anything else (none yet, written by a later version). */
export function themeChoice(value: unknown): ThemeChoice {
  return THEMES.includes(value as ThemeChoice) ? (value as ThemeChoice) : 'system';
}

/** The theme a choice shows, the system's being light or not. */
export function resolveTheme(choice: ThemeChoice, systemLight: boolean): Theme {
  return choice === 'system' ? (systemLight ? 'light' : 'dark') : choice;
}

const systemQuery = () => (typeof matchMedia === 'function' ? matchMedia('(prefers-color-scheme: light)') : null);

let current: ThemeChoice = 'system';
let watching = false;

function paint() {
  const theme = resolveTheme(current, !!systemQuery()?.matches);
  if (document.documentElement.dataset.theme !== theme) document.documentElement.dataset.theme = theme;
}

/** The theme the window shows now. */
export function shownTheme(): Theme {
  return document.documentElement.dataset.theme === 'light' ? 'light' : 'dark';
}

/** Shows `choice` without telling the window (before the first render, and for the system's own changes). */
function show(choice: ThemeChoice) {
  current = choice;
  paint();
  if (!watching) {
    // Système follows the system's changes, at once.
    systemQuery()?.addEventListener('change', paint);
    watching = true;
  }
}

/** Before the first render: the choice the page kept, until the settings say it. */
export function earlyAppearance() {
  let kept: string | null = null;
  try {
    kept = localStorage.getItem(THEME_KEY);
  } catch {
    // No storage: Système until the settings come.
  }
  show(themeChoice(kept));
}

/** Shows `choice`, keeps it for the next start, and gives the window its theme (none: the system's). */
export function applyAppearance(choice: ThemeChoice) {
  const changed = choice !== current;
  show(choice);
  try {
    localStorage.setItem(THEME_KEY, choice);
  } catch {
    // Kept by the settings all the same.
  }
  if (!changed) return;
  try {
    getCurrentWindow()
      .setTheme(choice === 'system' ? null : choice)
      .catch(() => {});
  } catch {
    // Not in the app's window (a test): the page's theme is enough.
  }
}
