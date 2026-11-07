// The shortcut of a launch's devtools (ESC-311), apart from the window's state: the « Pages » window reads it too.

/** F12, Cmd+Opt+I on macOS, Ctrl+Shift+I elsewhere. */
export function devtoolsKey(e: KeyboardEvent, mac: boolean): boolean {
  if (e.key === 'F12') return true;
  const i = e.key.toLowerCase() === 'i' || e.code === 'KeyI';
  return mac ? i && e.metaKey && e.altKey : i && e.ctrlKey && e.shiftKey;
}

/** The address typed in a page's bar: whole, or without its scheme, then the page's (`current`). */
export function typedAddress(typed: string, current: string): string {
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(typed) || /^(about|mailto):/i.test(typed)) return typed;
  let scheme = 'http:';
  try {
    scheme = new URL(current).protocol;
  } catch {
    // Not an address: http.
  }
  return `${scheme}//${typed}`;
}

/** The widths a page is shown at (ESC-311): the whole room, a tablet's, a phone's. */
export type PageMode = 'full' | 'tablet' | 'mobile';
export const PAGE_MODES: PageMode[] = ['full', 'tablet', 'mobile'];
/** The width of a tablet and of a phone, in CSS pixels (an iPad in portrait, a recent iPhone). */
export const DEVICE_WIDTH = { tablet: 768, mobile: 390 } as const;

/** Where the page goes in a room `width` wide: its width, its left from the room's, and whether it was narrowed to the room. */
export function deviceFit(mode: PageMode, width: number): { width: number; left: number; shrunk: boolean } {
  if (mode === 'full') return { width, left: 0, shrunk: false };
  const device = DEVICE_WIDTH[mode];
  const w = Math.min(device, width);
  return { width: w, left: Math.round((width - w) / 2), shrunk: w < device };
}

/** The mode after `mode` (its shortcut goes round them). */
export const nextMode = (mode: PageMode): PageMode => PAGE_MODES[(PAGE_MODES.indexOf(mode) + 1) % PAGE_MODES.length];

/** The shortcut that goes round the widths: Ctrl+Shift+M, Cmd+Shift+M on macOS (the devtools' own device mode's). */
export function modeKey(e: KeyboardEvent, mac: boolean): boolean {
  const m = e.key.toLowerCase() === 'm' || e.code === 'KeyM';
  return m && e.shiftKey && !e.altKey && (mac ? e.metaKey : e.ctrlKey);
}

/** Where the width chosen for each page is kept, by page (« worktree · processus »): a new launch keeps it. */
export const MODES_KEY = 'escouade.devpage.modes';

export function loadModes(): Record<string, PageMode> {
  try {
    const all = JSON.parse(localStorage.getItem(MODES_KEY) ?? '{}');
    return Object.fromEntries(Object.entries(all).filter(([, m]) => PAGE_MODES.includes(m as PageMode))) as Record<string, PageMode>;
  } catch {
    return {};
  }
}

export function saveMode(title: string, mode: PageMode) {
  const all = loadModes();
  if (mode === 'full') delete all[title];
  else all[title] = mode;
  try {
    localStorage.setItem(MODES_KEY, JSON.stringify(all));
  } catch {
    // Storage full or off: kept for this run only.
  }
}
