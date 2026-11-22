/**
 * Where the app is (MO-1, direction B): its three tabs (À toi, Agents, Kanban) and the sheet open above them (the « + »
 * of the header). The system's back goes one step back: it closes the sheet, then brings « À toi » back, then leaves
 * the app (Android puts it in the background, as its other apps do at their first screen).
 *
 * Android's back button comes from Capacitor (`native.ts`); a browser's (the e2e, `npm run dev`) from its history: the
 * page keeps one entry above its own, and each back that comes there is one step.
 */

export type Tab = 'inbox' | 'agents' | 'kanban';
export const TABS: Tab[] = ['inbox', 'agents', 'kanban'];
export type Sheet = 'create';

export const nav = $state<{ tab: Tab; sheet: Sheet | null }>({ tab: 'inbox', sheet: null });

export function goTab(tab: Tab): void {
  nav.sheet = null;
  nav.tab = tab;
}

export function openSheet(sheet: Sheet): void {
  nav.sheet = sheet;
}

export function closeSheet(): void {
  nav.sheet = null;
}

/** One step back: whether the app stays (`false`: it is at its first screen, it leaves). */
export function back(): boolean {
  if (nav.sheet) {
    nav.sheet = null;
    return true;
  }
  if (nav.tab !== 'inbox') {
    nav.tab = 'inbox';
    return true;
  }
  return false;
}

const GUARD = { escouade: 'back' };

/**
 * A browser's back, through its history: an entry above the page's own; each back that lands there is a step, and the
 * entry is put back while the app stays. At its first screen, the back goes on out of the page.
 */
export function followHistory(win: Window = window): () => void {
  const guarded = () => win.history.state?.escouade === 'back';
  if (!guarded()) win.history.pushState(GUARD, '');
  const onPop = () => {
    if (guarded()) return;
    if (back()) win.history.pushState(GUARD, '');
    else win.history.back();
  };
  win.addEventListener('popstate', onPop);
  return () => win.removeEventListener('popstate', onPop);
}
