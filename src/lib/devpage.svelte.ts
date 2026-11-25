// The browsers of a project (ESC-657), rows of its sidebar under « Terminaux », one per tab: the page of an agent's
// launch (ESC-311), which opens by itself when its test answers, or a browser the user opened alone by its « + ». Each
// is a webview of the backend's (`devpage.rs`), apart from the window: this module follows what it does and says where
// it goes in the window. A browser is shown as a terminal is: alone in the main area, or in a pane of the split screen.
// It lives while Escouade runs, as a terminal does: neither one is kept at a restart (its width is, by its name).

import { t } from './i18n';
import { api } from './ipc';
import { openUrl } from './opener';
import { app } from './state.svelte';
import type { Agent, PageBounds, PageEvent, PagesOpen, Project } from './types';
import { loadModes, nextMode, saveMode, type PageMode } from './devpage-key';
import { activeItem, has, itemKey, type PaneItem } from './panes';

/** A page that started loading and has not finished after this long does not answer. */
export const STALLED_MS = 20_000;

/** The key of a browser the user opened alone (the backend's `BROWSER_PREFIX`). */
export const BROWSER_PREFIX = 'browser:';

/**
 * Who opened a browser: an agent's launch (its key is the agent's id), a launch command run in its project (ESC-560,
 * keyed `run:<project>:<command>`), or the user alone.
 */
export type PageBy = { kind: 'agent'; id: string } | { kind: 'command'; id: string } | { kind: 'user' };

export interface PageState {
  url: string;
  /** What it is named by: « pagination-users · web » for an agent's launch, its host for a browser opened alone. */
  title: string;
  /** Its document's title, once the page gave one. */
  docTitle?: string;
  /** `loading`: a load under way; `ready`: loaded; `stalled`: a load that did not finish in `STALLED_MS`. */
  phase: 'loading' | 'ready' | 'stalled';
  projectId: string;
  by: PageBy;
  /** Its row's place: the order it opened in, the oldest first. */
  opened: number;
}

class DevPages {
  /** By key: an agent's id, or `browser:<id>`. */
  pages = $state<Record<string, PageState>>({});
  /** The width each page is shown at, by page (« worktree · processus », or its host), kept from one launch to the next. */
  modes = $state<Record<string, PageMode>>(loadModes());
}

export const devpages = new DevPages();

const stalls = new Map<string, ReturnType<typeof setTimeout>>();
let seq = 0;
/** The order the rows opened in. */
let order = 0;

/** Where the pages of project `project` open: in Escouade, or in the system's browser. The « Pages » window is gone (ESC-657): set before, it opens the browser's row. */
export const pagesOpenOf = (project: Project | undefined): 'agent' | 'system' => (project?.pagesOpen === 'system' ? 'system' : 'agent');

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

/** The browsers of project `projectId`, in the window (not in a « Pages » one), the oldest first: its rows. */
export function browsersOf(projectId: string): [string, PageState][] {
  return Object.entries(devpages.pages)
    .filter(([, p]) => p.projectId === projectId)
    .sort(([, a], [, b]) => a.opened - b.opened);
}

/**
 * What its row says: an agent's launch by its agent and process, a launch command by its name; a browser alone by its
 * document's title, else its host.
 */
export function rowName(page: PageState): string {
  if (page.by.kind !== 'user') return page.title;
  return page.docTitle || hostOf(page.url);
}

/**
 * The address typed in « + » (ESC-657), as the launches' pages take one: an http or https address with a host, `http://`
 * when it has no scheme. The backend checks it again (`launch_url`) and refuses Escouade's own.
 */
export function browserAddress(typed: string): { url: string } | { error: string } {
  const text = typed.trim();
  if (!text) return { error: t('devpage.browsers.empty') };
  const whole = /^[a-z][a-z0-9+.-]*:/i.test(text) && !/^[^:/]+:\d+(\/|$)/.test(text) ? text : `http://${text}`;
  let url: URL;
  try {
    url = new URL(whole);
  } catch {
    return { error: t('devpage.browsers.notAnAddress') };
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return { error: t('devpage.browsers.scheme') };
  if (!url.hostname) return { error: t('devpage.browsers.notAnAddress') };
  return { url: url.href };
}

/** A key of its own for a browser opened alone. */
function newKey(): string {
  seq += 1;
  return `${BROWSER_PREFIX}${Date.now().toString(36)}${seq.toString(36)}`;
}

/**
 * A browser opened alone in project `projectId` at the address typed (« + » of « Navigateurs »): its row, shown in the
 * main area as a terminal just opened is. Returns its key, or null when the address cannot open (a toast says why).
 */
export async function newBrowser(projectId: string, typed: string): Promise<string | null> {
  const address = browserAddress(typed);
  if ('error' in address) {
    app.toast(address.error, 'error');
    return null;
  }
  const key = newKey();
  const title = hostOf(address.url);
  const view = await app.run(api.devpageOpen(key, title, address.url, null, projectId));
  if (!view) return null;
  devpages.pages[key] = {
    url: view.url,
    title,
    phase: view.loaded ? 'ready' : 'loading',
    docTitle: view.docTitle || undefined,
    projectId,
    by: { kind: 'user' },
    opened: ++order,
  };
  watchStall(key);
  adopt(key);
  const mode = modeOf(key);
  if (mode !== 'full') api.devpageMode(key, mode).catch(() => {});
  app.selectPage(key);
  return key;
}

/**
 * The launch of `agent` answers at `address`: its page opens where its project says. Returns where it opened. In the
 * window, it is its browser's row (ESC-657): shown when the user was looking at the agent or at its test (the modal of
 * « ▶ Tester »), as the system's browser came to the front before; elsewhere its row waits for them.
 */
export async function openLaunchPage(agent: Agent, address: string, named?: string): Promise<'agent' | 'system'> {
  const where = pagesOpenOf(app.projects.find((p) => p.id === agent.projectId));
  if (where === 'system') {
    await openUrl(address);
    return where;
  }
  const title = named ?? pageTitle(agent, address);
  const view = await api.devpageOpen(agent.id, title, address, null);
  const before = devpages.pages[agent.id];
  devpages.pages[agent.id] = {
    url: view.url,
    title,
    phase: view.loaded ? 'ready' : 'loading',
    docTitle: view.docTitle || undefined,
    projectId: agent.projectId,
    by: { kind: 'agent', id: agent.id },
    opened: before?.opened ?? ++order,
  };
  // Its width, as it was chosen at its last launch (the « Pages » window lays it out by it).
  watchStall(agent.id);
  adopt(agent.id);
  const mode = modeOf(agent.id);
  if (mode !== 'full') api.devpageMode(agent.id, mode).catch(() => {});
  const testing = app.modal?.kind === 'testLaunch' && app.modal.agentId === agent.id;
  if (testing) app.modal = null;
  const looking =
    app.ui.view === 'project' &&
    app.ui.activeProject === agent.projectId &&
    app.ui.selectedAgent[agent.projectId] === agent.id &&
    !app.term &&
    !app.runCommand;
  if (testing || looking) {
    if (app.ui.activeProject !== agent.projectId) app.ui.activeProject = agent.projectId;
    app.ui.view = 'project';
    app.selectPage(agent.id);
  }
  return 'agent';
}

/** The key of the page of launch command `commandId` run in project `projectId` (ESC-560): it has no agent. */
export const projectPageKey = (projectId: string, commandId: string) => `run:${projectId}:${commandId}`;

/**
 * A launch command run in its project answers at `address` (ESC-560): its page opens in its browser's row (ESC-657), one
 * per command (loaded again rather than doubled), or in the system's browser when the project says so. Its row is shown
 * when the user was looking at the command's log, as an agent's when they looked at the agent; elsewhere it waits.
 */
export async function openProjectPage(project: Project, commandId: string, title: string, address: string): Promise<'agent' | 'system'> {
  if (pagesOpenOf(project) === 'system') {
    await openUrl(address);
    return 'system';
  }
  const key = projectPageKey(project.id, commandId);
  const view = await api.devpageOpen(key, title, address, null, project.id);
  const before = devpages.pages[key];
  devpages.pages[key] = {
    url: view.url,
    title,
    phase: view.loaded ? 'ready' : 'loading',
    docTitle: view.docTitle || undefined,
    projectId: project.id,
    by: { kind: 'command', id: commandId },
    opened: before?.opened ?? ++order,
  };
  watchStall(key);
  adopt(key);
  const mode = modeOf(key);
  if (mode !== 'full') api.devpageMode(key, mode).catch(() => {});
  const looking = app.ui.view === 'project' && app.ui.activeProject === project.id && app.selectedLaunch[project.id] === commandId;
  if (looking) app.selectPage(key);
  return 'agent';
}

/** The width the page `key` is shown at. */
export function modeOf(key: string): PageMode {
  const page = devpages.pages[key];
  return (page && devpages.modes[page.title]) || 'full';
}

/** Shows the page `key` at `mode`, kept for its next launches. */
export function setMode(key: string, mode: PageMode) {
  const page = devpages.pages[key];
  if (!page) return;
  if (mode === 'full') delete devpages.modes[page.title];
  else devpages.modes[page.title] = mode;
  saveMode(page.title, mode);
  api.devpageMode(key, mode).catch(() => {});
}

/** The next width (its shortcut). */
export const cycleMode = (key: string) => setMode(key, nextMode(modeOf(key)));

/** The browser `key` as a content of the split screen (ESC-419, ESC-657). */
export const pageItem = (key: string): PaneItem => ({ kind: 'page', id: key });

/** The browser `key` is in a pane of the split screen on screen. */
export const inPane = (key: string): boolean => app.panesOn && has(app.panes, pageItem(key));

/** Shows the browser `key` (its row clicked, the « Page » of its agent's header): alone, or its pane made active. */
export function showPage(key: string) {
  const page = devpages.pages[key];
  if (!page) return;
  if (inPane(key)) return app.activatePane(`page:${key}`);
  if (app.ui.activeProject !== page.projectId) app.ui.activeProject = page.projectId;
  app.ui.view = 'project';
  app.selectPage(key);
}

/**
 * What « Ouvrir à côté » of the browser `key` goes beside: what is on screen; when that is the browser itself, the
 * conversation of the agent whose launch opened it. Null: nothing to go beside (a browser opened alone, shown alone).
 */
export function besideOf(key: string): PaneItem | null {
  const page = devpages.pages[key];
  if (!page) return null;
  const on = app.panes && app.panesOn ? activeItem(app.panes) : app.singleItem;
  if (!on || itemKey(on) !== `page:${key}`) return on;
  return page.by.kind === 'agent' && app.agents[page.by.id] && !app.agents[page.by.id].archived ? { kind: 'agent', id: page.by.id } : null;
}

/** The browser `key` in a pane beside what is on screen (`right`) or under it (`bottom`), as a terminal goes. */
export function openPageBeside(key: string, zone: 'right' | 'bottom' = 'right') {
  const page = devpages.pages[key];
  if (!page) return;
  const target = besideOf(key);
  if (!target) return app.openBeside(pageItem(key), zone);
  app.dropInPane(target, zone, pageItem(key));
}

/** Back, forward, reload, or an address typed in its bar. */
export function go(key: string, action: 'back' | 'forward' | 'reload' | 'to', to?: string) {
  return app.run(api.devpageGo(key, action, to));
}

/** Its devtools: on a gesture of the user alone (its bar's button, its key while Escouade has the focus). */
export function devtools(key: string) {
  return app.run(api.devpageDevtools(key));
}

/** Where the page goes in the window (its view's room, in CSS pixels, times the zoom), or hidden (null). */
export function place(key: string, bounds: PageBounds | null) {
  if (!devpages.pages[key]) return Promise.resolve();
  return api.devpagePlace(key, bounds).catch(() => {});
}

/** Closes the browser `key`: its row's ×, its agent gone, its ticket done. */
export function closePage(key: string) {
  const page = devpages.pages[key];
  if (!page) return;
  delete devpages.pages[key];
  clearTimeout(stalls.get(key));
  stalls.delete(key);
  if (app.selectedPage[page.projectId] === key) app.selectedPage[page.projectId] = null;
  api.devpageClose(key).catch(() => {});
  // Its pane, if it had one, closes.
  app.settlePanes();
}

/**
 * A load of the page `key` under way: it does not answer once `STALLED_MS` pass without its end. Armed by its `started`,
 * and at its opening too: the first load's `started` comes before its row exists, and would be lost.
 */
function watchStall(key: string) {
  clearTimeout(stalls.get(key));
  stalls.set(
    key,
    setTimeout(() => {
      const p = devpages.pages[key];
      if (p?.phase === 'loading') p.phase = 'stalled';
    }, STALLED_MS),
  );
}

/**
 * What the pages the window does not know yet said (their opening under way: an event can come between the backend's
 * answer and their row), by key: replayed once their row exists, within 30 s. Bounded: 20 events a page, 50 pages.
 */
const early = new Map<string, { at: number; events: PageEvent[] }>();
const EARLY_MS = 30_000;

/** The row `key` now exists: what its page said before. */
function adopt(key: string) {
  const held = early.get(key);
  early.delete(key);
  if (held && Date.now() - held.at < EARLY_MS) for (const e of held.events) onPageEvent(e);
}

/** What the backend says of a page. */
export function onPageEvent(e: PageEvent) {
  const page = devpages.pages[e.agentId];
  if (!page) {
    // Its loads and its title only: what opens or refuses something is not done later.
    if (e.phase !== 'started' && e.phase !== 'finished' && e.phase !== 'title') return;
    const held = early.get(e.agentId) ?? { at: Date.now(), events: [] };
    if (held.events.length < 20 && (early.has(e.agentId) || early.size < 50)) held.events.push(e);
    early.set(e.agentId, held);
    return;
  }
  switch (e.phase) {
    case 'started':
      page.url = e.url;
      page.phase = 'loading';
      watchStall(e.agentId);
      break;
    case 'finished':
      page.url = e.url;
      page.phase = 'ready';
      clearTimeout(stalls.get(e.agentId));
      break;
    case 'title':
      page.docTitle = e.title || undefined;
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
app.pages = {
  open: (key) => !!devpages.pages[key],
  projectOf: (key) => devpages.pages[key]?.projectId ?? null,
};
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
