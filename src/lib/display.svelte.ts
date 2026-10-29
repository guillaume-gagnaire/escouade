// The look of the window, « Thème » (lib/appearance.ts), and the size of what it shows: « Zoom de l’interface » (the WebView's zoom: text, spacing and icons together),
// and the font sizes of the terminals and of the editor, set apart (and zoomed with the rest). Applied at once, and
// saved a moment after the last change: Ctrl+= pressed five times is one save.
//
// The interface is laid out for 1000 × 600 pixels at least (the window's minimum size): a zoom the window cannot hold
// then is shown at the biggest step it can, and comes back by itself once the window is big enough again.

import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { applyAppearance, themeChoice } from './appearance';
import { api } from './ipc';
import { IS_MAC, primaryKey } from './platform';
import { app } from './state.svelte';
import type { Settings } from './types';

/** The steps of the zoom, in percent (the backend's `ZOOM_LEVELS`). */
export const ZOOM_LEVELS = [80, 90, 100, 110, 125, 150, 175, 200] as const;
/** The font sizes the terminals and the editor may have, in CSS pixels (the backend's `FONT_SIZE_RANGE`). */
export const FONT_SIZE_MIN = 9;
export const FONT_SIZE_MAX = 24;
/** Their size by default: what they had before it could be set. */
export const DEFAULT_FONT_SIZE = 12.5;
/** The least room the interface is laid out for, in CSS pixels: the window's minimum size (the backend's `MIN_ROOM`). */
export const MIN_ROOM = { width: 1000, height: 600 } as const;

/** The step of the zoom nearest `zoom` (100 for what is not a number). */
export function nearestZoom(zoom: number): number {
  if (!Number.isFinite(zoom)) return 100;
  return ZOOM_LEVELS.reduce((best, l) => (Math.abs(l - zoom) < Math.abs(best - zoom) ? l : best), 100);
}

/** The zoom one step in (1) or out (-1) of `zoom`, held at the ends; 0 is back to 100 %. */
export function zoomStep(zoom: number, dir: -1 | 0 | 1): number {
  if (dir === 0) return 100;
  const i = ZOOM_LEVELS.indexOf(nearestZoom(zoom) as (typeof ZOOM_LEVELS)[number]);
  return ZOOM_LEVELS[Math.max(0, Math.min(ZOOM_LEVELS.length - 1, i + dir))];
}

/**
 * The biggest step a window of `room` (its size at 100 %, in logical pixels) holds: the one that leaves the interface
 * its 1000 × 600 pixels. Never under 100 %, which the window's minimum size always holds.
 */
export function fitZoom(room: { width: number; height: number }): number {
  const fits = (z: number) => (room.width * 100) / z >= MIN_ROOM.width - 0.5 && (room.height * 100) / z >= MIN_ROOM.height - 0.5;
  return ZOOM_LEVELS.filter((z) => z <= 100 || fits(z)).at(-1)!;
}

/** A font size as kept: to the half pixel, between the bounds; not a number is the default. */
export function fontSize(size: number): number {
  if (!Number.isFinite(size)) return DEFAULT_FONT_SIZE;
  return Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, Math.round(size * 2) / 2));
}

/**
 * Ctrl+= or Ctrl++ zooms in, Ctrl+- out, Ctrl+0 back to 100 % (Cmd on macOS), as in a browser or VS Code: by the
 * character typed, with or without Shift (Ctrl++ is Ctrl+Shift+= on most keyboards), or by the numeric keypad's keys,
 * and Ctrl+0 by its place too (« à » on AZERTY keyboards). Never with Alt (AltGr on French keyboards).
 */
export function zoomKey(e: KeyboardEvent, mac = IS_MAC): -1 | 0 | 1 | null {
  if (e.altKey || !primaryKey(e, mac)) return null;
  if (e.key === '=' || e.key === '+' || e.code === 'NumpadAdd') return 1;
  if (e.key === '-' || e.code === 'NumpadSubtract') return -1;
  if (!e.shiftKey && (e.key === '0' || e.code === 'Digit0' || e.code === 'Numpad0')) return 0;
  return null;
}

/** The display settings, as the window has them. */
type Display = Pick<Settings, 'theme' | 'pluginTheme' | 'uiZoom' | 'terminalFontSize' | 'editorFontSize'>;

/** What the window holds: the biggest step of the zoom (`fit`), and the zoom shown (`shown`, the one chosen or `fit`). */
export const zoomState = $state({ fit: 200 as number, shown: 100 as number });

/** How long after the last change the settings are saved. */
export const SAVE_DELAY_MS = 400;
let saveTimer: ReturnType<typeof setTimeout> | undefined;

/** The zoom chosen, as the window can show it: the WebView zoomed to it when it is another. */
export function showZoom() {
  const zoom = Math.min(nearestZoom(app.settings.uiZoom ?? 100), zoomState.fit);
  if (zoom === zoomState.shown) return;
  zoomState.shown = zoom;
  getCurrentWebview()
    .setZoom(zoom / 100)
    .catch(() => {});
}

/** The size of the window at 100 %, as the zoom is held to it (none known: no hold). */
async function measureWindow() {
  try {
    const w = getCurrentWindow();
    const [size, scale] = await Promise.all([w.innerSize(), w.scaleFactor()]);
    // Minimized: no size worth holding the zoom to.
    if (size.width > 0 && size.height > 0) zoomState.fit = fitZoom({ width: size.width / scale, height: size.height / scale });
  } catch {
    // The size could not be read: the zoom is not held.
  }
}

/**
 * Zooms the window as the settings say, with what it holds: at once (a page loaded again keeps the zoom it had, which may
 * be another), then each time it is resized. Returns what stops it.
 */
export function watchZoom(): () => void {
  zoomState.shown = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const update = () => measureWindow().then(showZoom);
  const onResize = () => {
    clearTimeout(timer);
    timer = setTimeout(update, 150);
  };
  update();
  window.addEventListener('resize', onResize);
  return () => {
    clearTimeout(timer);
    window.removeEventListener('resize', onResize);
  };
}

/**
 * Changes display settings: shown at once (the zoom here, the terminals and the editor follow `app.settings`), saved a
 * moment after the last change, with the rest of the app's settings as they are.
 */
export function setDisplay(change: Partial<Display>) {
  const s = app.settings;
  if (change.theme !== undefined) applyAppearance((s.theme = themeChoice(change.theme)));
  // The window follows it (App.svelte), its plugin read.
  if (change.pluginTheme !== undefined) s.pluginTheme = change.pluginTheme;
  if (change.uiZoom !== undefined) s.uiZoom = nearestZoom(change.uiZoom);
  if (change.terminalFontSize !== undefined) s.terminalFontSize = fontSize(change.terminalFontSize);
  if (change.editorFontSize !== undefined) s.editorFontSize = fontSize(change.editorFontSize);
  showZoom();
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = undefined;
    app.run(api.saveSettings($state.snapshot(app.settings)));
  }, SAVE_DELAY_MS);
}

/**
 * Runs the zoom's shortcut `e` is, if it is one. Returns true when it was. A step from the zoom shown, never past what
 * the window holds: a key that changes nothing on screen would only be a surprise once the window grows.
 */
export function handleZoomKey(e: KeyboardEvent, mac = IS_MAC): boolean {
  const dir = zoomKey(e, mac);
  if (dir === null) return false;
  const next = Math.min(zoomStep(zoomState.shown, dir), zoomState.fit);
  if (next !== app.settings.uiZoom) setDisplay({ uiZoom: next });
  return true;
}
