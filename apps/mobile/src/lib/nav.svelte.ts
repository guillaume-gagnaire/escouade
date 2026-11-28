/**
 * Where the app is (MO-1, MO-5, direction B): its three tabs (À toi, Agents, Kanban), the line unfolded in the thread,
 * an agent's conversation opened above the tabs, and the sheet open above everything (the « + » of the header; a
 * render in the viewer, a PDF, a command read in full screen, a file). The system's back goes one step back: it closes
 * the sheet, then the conversation, then folds the line, then brings « À toi » back, then leaves the app (Android
 * puts it in the background, as its other apps do at their first screen).
 *
 * Android's back button comes from Capacitor (`native.ts`); a browser's (the e2e, `npm run dev`) from its history: the
 * page keeps one entry above its own, and each back that comes there is one step.
 */

export type Tab = 'inbox' | 'agents' | 'kanban';
export const TABS: Tab[] = ['inbox', 'agents', 'kanban'];
export type Sheet = 'create' | 'project' | 'viewer' | 'pdf' | 'command' | 'file';

/** An agent's conversation, in its space. */
export interface Talk {
  space: string;
  agent: string;
}

export const nav = $state<{ tab: Tab; sheet: Sheet | null; talk: Talk | null; open: string | null }>({
  tab: 'inbox',
  sheet: null,
  talk: null,
  open: null,
});

/** The tab a page's address names (`#kanban`: the development's and the tests' way to open one), if any. */
export function tabOf(hash: string): Tab | null {
  const asked = hash.replace(/^#/, '');
  return (TABS as string[]).includes(asked) ? (asked as Tab) : null;
}

export function goTab(tab: Tab): void {
  nav.sheet = null;
  nav.talk = null;
  nav.tab = tab;
}

export function openSheet(sheet: Sheet): void {
  nav.sheet = sheet;
}

export function closeSheet(): void {
  nav.sheet = null;
}

/** An agent's conversation opened (from a line, the « Agents » tab). */
export function openTalk(space: string, agent: string): void {
  nav.sheet = null;
  nav.talk = { space, agent };
}

export function closeTalk(): void {
  nav.talk = null;
}

/** A line of the thread unfolded (none: all folded). */
export function unfold(key: string | null): void {
  nav.open = key;
}

/** One step back: whether the app stays (`false`: it is at its first screen, it leaves). */
export function back(): boolean {
  if (nav.sheet) {
    nav.sheet = null;
    return true;
  }
  if (nav.talk) {
    nav.talk = null;
    return true;
  }
  if (nav.open && nav.tab === 'inbox') {
    nav.open = null;
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
