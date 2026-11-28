// The browsers of a project (ESC-657), rows of its sidebar under « Terminaux », one per tab: the page of an agent's
// launch (ESC-311), which opens by itself when its test answers, or a browser the user opened alone by its « + ». Each
// is a webview of the backend's (`devpage.rs`), apart from the window: this module follows what it does and says where
// it goes in the window. A browser is shown as a terminal is: alone in the main area, or in a pane of the split screen.
// It lives while Escouade runs, as a terminal does: neither one is kept at a restart (its width is, by its name).

import { untrack } from 'svelte';
import { t } from './i18n';
import { api } from './ipc';
import { openUrl } from './opener';
import { onThisComputer } from './server-moves.svelte';
import { app } from './state.svelte';
import type {
  Agent,
  BrowserOpenAsk,
  BrowserOpened,
  BrowserWish,
  PageBounds,
  PageConsent,
  PageConsents,
  PageEvent,
  PagePlacement,
  PagesOpen,
  Project,
} from './types';
import { loadModes, nextMode, saveMode, type PageMode } from './devpage-key';
import { activeItem, has, itemKey, leaves, type PaneItem } from './panes';
import { forgetDrive, refreshDrive } from './drives.svelte';

/** A page that started loading and has not finished after this long does not answer. */
export const STALLED_MS = 20_000;

/** The key of a browser the user opened alone (the backend's `BROWSER_PREFIX`). */
export const BROWSER_PREFIX = 'browser:';

/**
 * Who opened a browser: an agent's launch (its key is the agent's id), a launch command run in its project (ESC-560,
 * keyed `run:<project>:<command>`), or the user alone.
 */
export type PageBy =
  { kind: 'agent'; id: string } | { kind: 'command'; id: string } | { kind: 'user' } | { kind: 'page'; id: string; page: string };

/** The key of an agent's page's row (AP-3, the backend's `pages::KEY_PREFIX`): `page:p-12`. */
export const AGENT_PAGE_PREFIX = 'page:';

/** The row `key` is an agent's page (`page:p-12`), shown by `AgentPageView` rather than `DevPage`. */
export const isAgentPage = (key: string): boolean => key.startsWith(AGENT_PAGE_PREFIX);

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
  /** The agents' requests to read it outside the launch, and the leaves given (ESC-669): its band, its row's menu. */
  consents?: PageConsents;
  /** The agent that opened it by `open_browser` (ESC-678): a tab of its own. */
  opener?: string;
  /** The agent that drives it (ESC-678). */
  driver?: string;
  /** Where its agent wants it, while the user looks elsewhere than its conversation (ESC-678): it opens there when they
   * come back. */
  waiting?: Exclude<BrowserWish, 'quiet'>;
  /** The width an agent's capture shows it at, for the capture's time (ESC-670): over its own, never kept. */
  shot?: PageMode;
  /** An agent's page (AP-3): the width the user chose for it, for this run. */
  width?: PageMode;
  /** An agent's page whose row waits for the user (they were not looking at its agent): its point. */
  unseen?: boolean;
  /** An agent's page asked to open this web address (« La page veut ouvrir… », AP-1): offered, never followed alone. */
  wants?: string;
}

class DevPages {
  /** By key: an agent's id, or `browser:<id>`. */
  pages = $state<Record<string, PageState>>({});
  /** The width each page is shown at, by page (« worktree · processus », or its host), kept from one launch to the next. */
  modes = $state<Record<string, PageMode>>(loadModes());
  /** The agents' requests to open an address outside their project's launch (ESC-678, C3): the bands of their
   * conversations. */
  asks = $state<BrowserOpenAsk[]>([]);
}

export const devpages = new DevPages();

const stalls = new Map<string, ReturnType<typeof setTimeout>>();
let seq = 0;
/** The order the rows opened in. */
let order = 0;

/** The place of a row opening now. */
export const nextOrder = () => ++order;

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
  const here = await onThisComputer(address.url);
  const title = hostOf(here);
  const view = await app.run(api.devpageOpen(key, title, here, null, projectId));
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
  // On a server (ESC-538): its test server relayed to a port of this computer, where the page opens.
  const here = await onThisComputer(address);
  const view = await api.devpageOpen(agent.id, title, here, null);
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
  // On a server (ESC-538): its test server relayed to a port of this computer, where the page opens.
  const here = await onThisComputer(address);
  const view = await api.devpageOpen(key, title, here, null, project.id);
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
  if (page?.by.kind === 'page') return page.shot ?? page.width ?? 'full';
  return (page && (page.shot ?? devpages.modes[page.title])) || 'full';
}

/** Shows the page `key` at `mode`, kept for its next launches. */
export function setMode(key: string, mode: PageMode) {
  const page = devpages.pages[key];
  if (!page) return;
  // An agent's page: the window lays it out by its width, for this run (its title is the agent's, not a launch's).
  if (page.by.kind === 'page') {
    page.width = mode;
    return;
  }
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
  return (page.by.kind === 'agent' || page.by.kind === 'page') && app.agents[page.by.id] && !app.agents[page.by.id].archived
    ? { kind: 'agent', id: page.by.id }
    : null;
}

/** The browser `key` in a pane beside what is on screen (`right`) or under it (`bottom`), as a terminal goes. */
export function openPageBeside(key: string, zone: 'right' | 'bottom' = 'right') {
  const page = devpages.pages[key];
  if (!page) return;
  const target = besideOf(key);
  if (!target) return app.openBeside(pageItem(key), zone);
  app.dropInPane(target, zone, pageItem(key));
}

/**
 * « Laisser piloter par… » (AB-9): the user lets the agent `agent` of its project drive the browser `key` they opened
 * alone, or nobody (`null`). The backend tells the agent by a message; nothing opens or moves on screen.
 */
export async function lendBrowser(key: string, agent: string | null) {
  const page = devpages.pages[key];
  if (!page) return;
  const done = await app.run(api.devpageLend(key, agent).then(() => true));
  if (!done) return;
  page.driver = agent ?? undefined;
  page.waiting = undefined;
  void refreshDrive(key);
  const name = agent ? (app.agents[agent]?.name ?? agent) : '';
  app.toast(agent ? t('devpage.drive.lent', { agent: name }) : t('devpage.drive.unlent'), 'info');
}

/** Back, forward, reload, or an address typed in its bar. */
export function go(key: string, action: 'back' | 'forward' | 'reload' | 'to', to?: string) {
  if (action === 'to' && to) return app.run(onThisComputer(to).then((here) => api.devpageGo(key, action, here)));
  return app.run(api.devpageGo(key, action, to));
}

/** Its devtools: on a gesture of the user alone (its bar's button, its key while Escouade has the focus). */
export function devtools(key: string) {
  return app.run(api.devpageDevtools(key));
}

/**
 * Where the page goes in the window (its view's room, in CSS pixels, times the zoom), or hidden (null); `placement`:
 * alone, beside or under something else, as `list_browsers` says it (ESC-669).
 */
export function place(key: string, bounds: PageBounds | null, placement?: PagePlacement) {
  const page = devpages.pages[key];
  if (!page) return Promise.resolve();
  if (page.by.kind === 'page') return api.agentpagePlace(page.by.page, bounds, bounds ? placement : 'hidden').catch(() => {});
  return api.devpagePlace(key, bounds, bounds ? placement : 'hidden').catch(() => {});
}

/**
 * Where a page in the room `room` is shown (ESC-669): alone in the main area, or in a pane of the split screen beside or
 * under something else (narrower than the work area: beside).
 */
export function placementOf(inPane: boolean, room: DOMRect, work: DOMRect | undefined): PagePlacement {
  if (!inPane) return 'full';
  if (!work) return 'beside';
  return room.width < work.width - 2 ? 'beside' : 'below';
}

/** Its band read again from the backend: an agent asked to read it, or its leaves changed. */
export async function loadConsents(key: string) {
  const consents = await api.devpageConsents(key).catch(() => null);
  const page = devpages.pages[key];
  if (page && consents) page.consents = consents;
}

/** « Permettre » or « Refuser » in its band, to `ask`: the agent is told by a message. */
export async function answerConsent(key: string, ask: PageConsent, allow: boolean) {
  const consents = await app.run(api.devpageConsent(key, ask.agent, ask.origin, allow));
  const page = devpages.pages[key];
  if (page && consents) page.consents = consents;
}

/** « Retirer les accords » of its row's menu: the leaves given, and the requests waiting, gone. */
export async function revokeConsents(key: string) {
  const consents = await app.run(api.devpageRevoke(key));
  const page = devpages.pages[key];
  if (page && consents) {
    page.consents = consents;
    app.toast(t('devpage.consent.revoked'), 'info');
  }
}

/** It has leaves given or requests waiting: « Retirer les accords » is offered. */
export const hasConsents = (key: string): boolean => {
  const c = devpages.pages[key]?.consents;
  return !!c && (c.asks.length > 0 || c.grants.length > 0);
};

/** The requests of the agent `id` to open an address, the oldest first. */
export const openAsksOf = (id: string): BrowserOpenAsk[] => devpages.asks.filter((a) => a.agent === id);

/** The requests to open read again from the backend: an agent asked, or was answered. */
export async function loadOpenAsks() {
  const asks = await api.devpageOpenAsks().catch(() => null);
  if (asks) devpages.asks = asks;
}

/** « Permettre une fois » or « Refuser » in the band of the agent's conversation: the agent is told by a message. */
export async function answerOpen(ask: BrowserOpenAsk, allow: boolean) {
  const asks = await app.run(api.devpageOpenAnswer(ask.id, allow));
  if (asks) devpages.asks = asks;
}

/** Who opened the browser `key`, by its key. */
function byOf(key: string): PageBy {
  if (key.startsWith('run:')) return { kind: 'command', id: key.split(':').slice(2).join(':') };
  if (key.startsWith(BROWSER_PREFIX)) return { kind: 'user' };
  return { kind: 'agent', id: key };
}

/**
 * An agent opened a browser by `open_browser`, or showed one again (ESC-678): its row, then where it goes. `waiting`:
 * the user looked elsewhere than its conversation; it opens where the agent wants once they look at it.
 */
export function onBrowserOpened(e: BrowserOpened) {
  const v = e.view;
  const key = v.agentId;
  const before = devpages.pages[key];
  devpages.pages[key] = {
    url: v.url,
    title: before?.title ?? v.title,
    phase: v.loaded ? 'ready' : 'loading',
    docTitle: v.docTitle || before?.docTitle,
    projectId: v.projectId,
    by: before?.by ?? byOf(key),
    opened: before?.opened ?? ++order,
    consents: before?.consents,
    opener: v.opener,
    driver: v.driver ?? e.agent,
    waiting: e.place === 'waiting' && e.wish !== 'quiet' ? e.wish : undefined,
  };
  watchStall(key);
  adopt(key);
  if (!before) {
    const mode = modeOf(key);
    if (mode !== 'full') api.devpageMode(key, mode).catch(() => {});
  }
  if (e.place === 'full' || e.place === 'beside' || e.place === 'below') showWish(key, e.place, e.agent);
}

/**
 * The browser `key` shown where its agent wants it: alone in the main area, or in a pane beside or under the agent's
 * conversation. One pane per agent: another browser it drives leaves its pane (opening again moves it, it adds none).
 */
export function showWish(key: string, wish: Exclude<BrowserWish, 'quiet'>, agent: string) {
  const page = devpages.pages[key];
  if (!page) return;
  // Never the user's screen taken elsewhere (AB-9): the backend said where while they looked at the agent; if they
  // have gone since, the row waits for them; if they closed its pane, nothing opens.
  if (dismissed(agent)) {
    page.waiting = undefined;
    return;
  }
  if (app.ui.view !== 'project' || app.ui.activeProject !== page.projectId || !app.shownAgents.includes(agent)) {
    page.waiting = wish;
    return;
  }
  page.waiting = undefined;
  if (wish === 'full') {
    app.selectPage(key);
    return;
  }
  for (const [other, p] of browsersOf(page.projectId)) {
    if (other !== key && p.driver === agent && inPane(other)) leavePane(other);
  }
  if (inPane(key)) return;
  const conversation: PaneItem = { kind: 'agent', id: agent };
  app.dropInPane(conversation, wish === 'below' ? 'bottom' : 'right', pageItem(key));
  // The conversation stays the active pane: the user goes on reading it, or writing to it.
  if (inPane(key)) app.activatePane(itemKey(conversation));
}

/**
 * The rows that leave the screen by the window's own doing (one pane per agent, an agent closing its browser or its
 * page), not by the user's: their leaving is no close of the user's (AB-9).
 */
const quietly = new Set<string>();

/** The rows the user closed (their ×, their menu) since the screen was last looked at: gone, yet a close of theirs. */
const closedByUser = new Set<string>();

/** The pane of the row `key` taken off by the window itself: one pane per agent, another of its rows opening. */
export function leavePane(key: string) {
  quietly.add(key);
  app.closePaneOf(`page:${key}`);
}

/**
 * The agents whose pane the user closed (AB-9), with whether the user has left their conversation since: nothing of
 * theirs opens on screen (a browser, a page) until the user leaves it and comes back. The backend holds the same, for
 * what `open_browser` answers.
 */
export const dismissals = $state<Record<string, boolean>>({});

/** The user closed the pane of `agent` and has not come back to its conversation since. */
export const dismissed = (agent: string): boolean => agent in dismissals;

/** The tools of the browsers answer: the MCP server on, « Agents et navigateurs » on. */
const browsersOn = () => !!app.settings?.mcpEnabled && app.settings?.agentBrowsers !== false;

/** The user closed a pane (or the row) of something `agent` drives while looking at its conversation. */
function dismiss(agent: string) {
  dismissals[agent] = !app.shownAgents.includes(agent);
  if (browsersOn()) api.devpageDismiss(agent).catch(() => {});
}

/** The rows on screen now, alone or in the panes of the split screen. */
function rowsOnScreen(): string[] {
  if (app.ui.view !== 'project') return [];
  if (app.panesOn && app.panes) return leaves(app.panes.root).flatMap((x) => (x.kind === 'page' ? [x.id] : []));
  return app.pageShown ? [app.pageShown] : [];
}

/**
 * Closes the browser `key`: its row's ×, its agent gone, its ticket done. An agent's page closes its row alone (it stays
 * kept, its card opens it again); `told`: the backend already closed it (its agent's `close_page`), nothing to ask.
 */
export function closePage(key: string, told = false) {
  const page = devpages.pages[key];
  if (!page) return;
  if (told) quietly.add(key);
  else closedByUser.add(key);
  delete devpages.pages[key];
  forgetDrive(key);
  clearTimeout(stalls.get(key));
  stalls.delete(key);
  if (app.selectedPage[page.projectId] === key) app.selectedPage[page.projectId] = null;
  if (page.by.kind === 'page') {
    if (!told) api.agentpageCloseRow(page.by.page).catch(() => {});
  } else api.devpageClose(key).catch(() => {});
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
  // An agent's request to open an address, keyed by the agent (ESC-678): not a page's.
  if (e.phase === 'opens') return void loadOpenAsks();
  // Its driving changed (a gesture, the hand, « Continuer », a password asked; ESC-680): the band reads it again.
  if (e.phase === 'drive') return void refreshDrive(e.agentId);
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
    case 'consents':
      void loadConsents(e.agentId);
      break;
    case 'closed':
      // Its agent closed it (`close_browser`, ESC-678): no close of the user's.
      quietly.add(e.agentId);
      closePage(e.agentId);
      break;
    case 'undriven':
      page.driver = undefined;
      page.waiting = undefined;
      // The core forgot its driving: the band goes and the row is an ordinary one again (ESC-685).
      void refreshDrive(e.agentId);
      break;
    case 'outside':
      // A mail or phone address (ESC-695: every web origin stays in the tab): the system's.
      app.run(openUrl(e.url));
      app.toast(t('devpage.outside', { url: e.url.slice(0, 200) }), 'info');
      break;
    case 'popups':
      // A window its page asked for, beyond the few it may have open (ESC-695).
      app.toast(t('devpage.popups'), 'info');
      break;
    case 'responsive':
      cycleMode(e.agentId);
      break;
    case 'mode':
      // An agent's `drive_browser` width (ESC-679): kept, as the user's own choice is, and the bar follows.
      if (e.url === 'full' || e.url === 'tablet' || e.url === 'mobile') setMode(e.agentId, e.url);
      break;
    case 'width':
      // An agent's capture at another width: shown at it, then given back its own.
      page.shot = e.url === 'full' || e.url === 'tablet' || e.url === 'mobile' ? e.url : undefined;
      api.devpageMode(e.agentId, modeOf(e.agentId)).catch(() => {});
      break;
    case 'refused':
      app.toast(t('devpage.refused', { url: e.url }), 'error');
      break;
  }
}

app.onDevPage(onPageEvent);
app.onBrowserOpened(onBrowserOpened);

/** The agents on screen, as last told to the backend; null while no tool of the browsers answers. */
let looking: string | null = null;

/**
 * The backend knows whose conversations are on screen (ESC-678), while its tools of the browsers answer (the server on,
 * « Agents et navigateurs » on): an agent's browser opens beside its conversation only then. A row that waited for the
 * user to look at its agent opens where the agent wanted it.
 */
$effect.root(() => {
  $effect(() => {
    const shown = app.shownAgents;
    const on = browsersOn();
    const said = shown.join(',');
    if (!on) looking = null;
    else if (said !== looking) {
      looking = said;
      api.devpageLooking(shown).catch(() => {});
    }
    untrack(() => {
      // A pane the user closed holds until they leave its agent's conversation and come back (AB-9).
      for (const [agent, away] of Object.entries(dismissals)) {
        if (!shown.includes(agent)) dismissals[agent] = true;
        else if (away) delete dismissals[agent];
      }
      for (const [key, p] of Object.entries(devpages.pages)) {
        if (p.waiting && p.driver && shown.includes(p.driver)) showWish(key, p.waiting, p.driver);
      }
    });
  });
  // A row an agent drives that leaves the screen while its conversation stays: the user closed it (its pane's ×, its
  // row's ×, « Fermer le volet », back to one pane), unless the window did (AB-9).
  let before = new Map<string, string | undefined>();
  let shownBefore: string[] = [];
  $effect(() => {
    const keys = rowsOnScreen();
    const shown = app.shownAgents;
    untrack(() => {
      const now = new Map<string, string | undefined>();
      for (const key of keys) now.set(key, devpages.pages[key]?.driver ?? before.get(key));
      for (const [key, agent] of before) {
        if (now.has(key) || quietly.has(key) || !agent) continue;
        // A row gone without the user closing it (its agent removed, its ticket done) is no close of theirs.
        if (!devpages.pages[key] && !closedByUser.has(key)) continue;
        // Its conversation on screen before and after: not the user going back to it from its browser shown alone.
        if (shown.includes(agent) && shownBefore.includes(agent)) dismiss(agent);
      }
      quietly.clear();
      closedByUser.clear();
      before = now;
      shownBefore = shown;
    });
  });
});
void loadOpenAsks();
app.pages = {
  open: (key) => !!devpages.pages[key],
  projectOf: (key) => devpages.pages[key]?.projectId ?? null,
};
// Its agent gone, its ticket done: the window's own close, not the user's (AB-9).
app.onAgentRemoved((id) => {
  quietly.add(id);
  closePage(id);
  devpages.asks = devpages.asks.filter((a) => a.agent !== id);
});
app.onTicketDone((key) => {
  quietly.add(key);
  closePage(key);
});

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
