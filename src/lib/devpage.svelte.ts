// The page of a launch, inside Escouade (ESC-311). When an agent's test launch answers, its address opens where the
// project says (« Ouvrir les pages des lancements »): in the agent's view « Page », in the project's « Pages » window,
// or in the system's browser as before. One page per agent: launching again loads the same page. The page itself is
// a webview of the backend's (`devpage.rs`), apart from the window: this module follows what it does and says where
// it goes in the window.

import { t } from './i18n';
import { api } from './ipc';
import { openUrl } from './opener';
import { app } from './state.svelte';
import type { Agent, PageBounds, PageEvent, PagesOpen, Project } from './types';
import { loadModes, nextMode, saveMode, type PageMode } from './devpage-key';

/** A page that started loading and has not finished after this long does not answer. */
export const STALLED_MS = 20_000;

export interface PageState {
  url: string;
  /** « pagination-users · web ». */
  title: string;
  /** `loading`: a load under way; `ready`: loaded; `stalled`: a load that did not finish in `STALLED_MS`. */
  phase: 'loading' | 'ready' | 'stalled';
  /** In the project's « Pages » window. */
  detached: boolean;
}

class DevPages {
  /** By agent. */
  pages = $state<Record<string, PageState>>({});
  /** The agents whose view shows their page rather than their conversation. */
  shown = $state<Record<string, boolean>>({});
  /** The width each page is shown at, by page (« worktree · processus »), kept from one launch to the next. */
  modes = $state<Record<string, PageMode>>(loadModes());
}

export const devpages = new DevPages();

const stalls = new Map<string, ReturnType<typeof setTimeout>>();

/** Where the pages of project `project` open. */
export const pagesOpenOf = (project: Project | undefined): PagesOpen => project?.pagesOpen ?? 'agent';

/** What a page's tab says: the agent (its worktree) and, when one of its processes serves the address, that one. */
export function pageTitle(agent: Agent, address: string): string {
  let origin = '';
  try {
    origin = new URL(address).origin;
  } catch {
    // Not an address: the agent alone.
  }
  const process = agent.recipe?.processes.find((p) => {
    try {
      return !!p.url.trim() && new URL(p.url.trim()).origin === origin;
    } catch {
      return false;
    }
  });
  return process ? `${agent.name} · ${process.name}` : agent.name;
}

/** Its host, for what the window says of it. */
export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/**
 * The launch of `agent` answers at `address`: its page opens where its project says. Returns where it opened. In the
 * agent's view, the agent is shown when the user was looking at it or at its test (the modal of « ▶ Tester »), as the
 * system's browser came to the front before; elsewhere its view « Page » waits for them.
 */
export async function openLaunchPage(agent: Agent, address: string): Promise<PagesOpen> {
  const where = pagesOpenOf(app.projects.find((p) => p.id === agent.projectId));
  if (where === 'system') {
    await openUrl(address);
    return where;
  }
  const title = pageTitle(agent, address);
  const view = await api.devpageOpen(agent.id, title, address, where === 'window', null);
  devpages.pages[agent.id] = { url: view.url, title, phase: 'loading', detached: view.detached };
  // Its width, as it was chosen at its last launch (the « Pages » window lays it out by it).
  const mode = modeOf(agent.id);
  if (mode !== 'full') api.devpageMode(agent.id, mode).catch(() => {});
  if (!view.detached) {
    devpages.shown[agent.id] = true;
    const testing = app.modal?.kind === 'testLaunch' && app.modal.agentId === agent.id;
    if (testing) app.modal = null;
    const looking =
      app.ui.view === 'project' && app.ui.activeProject === agent.projectId && app.ui.selectedAgent[agent.projectId] === agent.id;
    if (testing || looking) app.selectAgent(agent.id);
  }
  return view.detached ? 'window' : 'agent';
}

/** The width the page of `agentId` is shown at. */
export function modeOf(agentId: string): PageMode {
  const page = devpages.pages[agentId];
  return (page && devpages.modes[page.title]) || 'full';
}

/** Shows the page of `agentId` at `mode`, kept for its next launches. */
export function setMode(agentId: string, mode: PageMode) {
  const page = devpages.pages[agentId];
  if (!page) return;
  if (mode === 'full') delete devpages.modes[page.title];
  else devpages.modes[page.title] = mode;
  saveMode(page.title, mode);
  api.devpageMode(agentId, mode).catch(() => {});
}

/** The next width (its shortcut). */
export const cycleMode = (agentId: string) => setMode(agentId, nextMode(modeOf(agentId)));

/** Its agent's view shows the page, or the conversation. */
export function showPage(agentId: string, on: boolean) {
  if (on && !devpages.pages[agentId]) return;
  devpages.shown[agentId] = on;
}

/** Back, forward, reload, or an address typed in its bar. */
export function go(agentId: string, action: 'back' | 'forward' | 'reload' | 'to', to?: string) {
  return app.run(api.devpageGo(agentId, action, to));
}

/** Its devtools. */
export function devtools(agentId: string) {
  return app.run(api.devpageDevtools(agentId));
}

/** To its project's « Pages » window, or back into the agent's view. */
export async function detach(agentId: string, detached: boolean) {
  const view = await app.run(api.devpageDetach(agentId, detached));
  const page = devpages.pages[agentId];
  if (view && page) page.detached = view.detached;
  if (view) devpages.shown[agentId] = !view.detached;
}

/** Where the page goes in the window (its view's room, in CSS pixels, times the zoom), or hidden (null). */
export function place(agentId: string, bounds: PageBounds | null) {
  if (!devpages.pages[agentId]) return Promise.resolve();
  return api.devpagePlace(agentId, bounds).catch(() => {});
}

/** Closes the page of `agentId` (its agent gone, its ticket done). */
export function closePage(agentId: string) {
  if (!devpages.pages[agentId]) return;
  delete devpages.pages[agentId];
  delete devpages.shown[agentId];
  clearTimeout(stalls.get(agentId));
  stalls.delete(agentId);
  api.devpageClose(agentId).catch(() => {});
}

/** What the backend says of a page. */
export function onPageEvent(e: PageEvent) {
  const page = devpages.pages[e.agentId];
  if (!page) return;
  switch (e.phase) {
    case 'started':
      page.url = e.url;
      page.phase = 'loading';
      clearTimeout(stalls.get(e.agentId));
      stalls.set(
        e.agentId,
        setTimeout(() => {
          const p = devpages.pages[e.agentId];
          if (p?.phase === 'loading') p.phase = 'stalled';
        }, STALLED_MS),
      );
      break;
    case 'finished':
      page.url = e.url;
      page.phase = 'ready';
      clearTimeout(stalls.get(e.agentId));
      break;
    case 'outside':
      // Another origin: the system's browser, as every link of the window.
      app.run(openUrl(e.url));
      app.toast(t('devpage.outside', { host: hostOf(e.url), page: hostOf(page.url) }), 'info');
      break;
    case 'responsive':
      cycleMode(e.agentId);
      break;
    case 'refused':
      app.toast(t('devpage.refused', { url: e.url }), 'error');
      break;
  }
}

app.onDevPage(onPageEvent);
app.onAgentRemoved(closePage);
app.onTicketDone(closePage);

/** The overlays of the window (menus, lists, dialogs, the palette): a native page would hide them, it is hidden under them. */
const OVERLAYS = '[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"], .palette';

/** One of the window's overlays crosses `rect`. */
export function coveredAt(rect: DOMRect, root: ParentNode = document): boolean {
  for (const el of root.querySelectorAll<HTMLElement>(OVERLAYS)) {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) continue;
    if (r.right > rect.left && r.left < rect.right && r.bottom > rect.top && r.top < rect.bottom) return true;
  }
  return false;
}

/** `rect` in the window's logical pixels, the interface zoomed by `zoom` percent. */
export function boundsOf(rect: DOMRect, zoom: number): PageBounds {
  const k = zoom / 100;
  return {
    x: Math.round(rect.left * k),
    y: Math.round(rect.top * k),
    width: Math.round(rect.width * k),
    height: Math.round(rect.height * k),
  };
}

export { devtoolsKey } from './devpage-key';
