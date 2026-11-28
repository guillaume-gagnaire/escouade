// The agents' pages in the window (AP-3, docs/AGENT-PAGES.md § 5, direction A): each open page is a row of « Navigateurs »
// (`page:p-12`), drawn by `AgentPageView` as the launches' pages are by `DevPage`; its card in the conversation reads what
// this module knows of it. The backend (`pages.rs`) keeps the pages and says what becomes of them (`agentPage` events):
// shown (a new page or a new version), closed (its row), erased (no longer kept). A page shown while the user looks at
// its agent opens beside its conversation; else its row waits with its point, and the card opens it.

import { t } from './i18n';
import { api } from './ipc';
import { app } from './state.svelte';
import { closePage, devpages, dismissed, isAgentPage, leavePane, nextOrder, AGENT_PAGE_PREFIX, type PageState } from './devpage.svelte';
import { activeItem, itemKey, leaves, type PaneItem } from './panes';
import type { AgentPageEvent, AgentPageRow } from './types';

/** The key of page `page`'s row: `page:p-12`. */
export const pageKey = (page: string) => `${AGENT_PAGE_PREFIX}${page}`;

/** The page a row's key names. */
export const pageOfKey = (key: string): string | null => (isAgentPage(key) ? key.slice(AGENT_PAGE_PREFIX.length) : null);

/** The templates, as the window names them. */
export const TEMPLATES = ['review', 'diagram', 'compare', 'html', 'custom'] as const;
export type PageTemplate = (typeof TEMPLATES)[number];

/** What the window knows of each page it was told of, by its number: null once it is no longer kept. */
class AgentPages {
  info = $state<Record<string, AgentPageRow | null>>({});
}

export const agentpages = new AgentPages();

/** The template's name in words (« Revue »), its key when Escouade does not know it. */
export function templateName(template: string): string {
  return (TEMPLATES as readonly string[]).includes(template) ? t(`devpage.agentPage.templates.${template as PageTemplate}`) : template;
}

/** The last version of a page. */
export const latestOf = (row: AgentPageRow): number => row.versions.at(-1)?.n ?? 1;

/** What the user sees on screen: the active pane's content, or what the work area shows alone. */
function onScreen(): PaneItem | null {
  if (app.ui.view !== 'project') return null;
  return app.panes && app.panesOn ? activeItem(app.panes) : app.singleItem;
}

/** The user looks at the conversation of `agentId`: a page it shows opens beside it (the rule of a test's page, ESC-311). */
export function looking(agentId: string, projectId: string): boolean {
  if (app.ui.activeProject !== projectId) return false;
  const on = onScreen();
  if (on?.kind === 'agent') return on.id === agentId;
  // A browser or a page it drives, active beside it (ESC-678): still its conversation the user follows.
  return !!on && on.kind === 'page' && devpages.pages[on.id]?.driver === agentId;
}

/** Page `row`'s row in « Navigateurs », made or brought up to date. */
export function adopt(row: AgentPageRow): string {
  const key = pageKey(row.page);
  const before = devpages.pages[key];
  agentpages.info[row.page] = row;
  if (!row.open) {
    if (before) closePage(key, true);
    return key;
  }
  const page: PageState = {
    url: '',
    title: row.title,
    phase: row.loaded ? 'ready' : 'loading',
    projectId: row.projectId,
    by: { kind: 'page', id: row.agentId, page: row.page },
    // Its agent drives it (AB-7): one pane per agent, a tab it opened and its page alike (ESC-678).
    driver: row.agentId,
    opened: before?.opened ?? nextOrder(),
    width: before?.width,
    unseen: before?.unseen,
    wants: before?.wants,
  };
  devpages.pages[key] = page;
  return key;
}

/** The row `key` is shown on screen, alone or in a pane. */
function shown(key: string): boolean {
  if (app.pageShown === key) return true;
  return !!app.panes && app.panesOn && app.panes.active === `page:${key}`;
}

/** The row `key` is in a pane of the split screen. */
function inAPane(key: string): boolean {
  if (!app.panes || !app.panesOn) return false;
  return leaves(app.panes.root).some((x) => itemKey(x) === `page:${key}`);
}

/**
 * Opens the row `key` beside the conversation of its agent `agentId`, the conversation staying active (the user goes on
 * reading it, or writing to it). Already on screen, it stays where it is.
 */
export function openBesideAgent(key: string, agentId: string) {
  const page = devpages.pages[key];
  if (!page) return;
  page.unseen = false;
  // From « À toi » (« Ouvrir la page », AP-4) or another view: back to the project, where its pane may already be.
  if (app.ui.view === 'inbox') app.closeInbox();
  if (app.ui.activeProject !== page.projectId) app.ui.activeProject = page.projectId;
  app.ui.view = 'project';
  if (shown(key) || inAPane(key)) return;
  const agent: PaneItem = { kind: 'agent', id: agentId };
  const live = app.agents[agentId] && !app.agents[agentId].archived;
  if (!live) return app.selectPage(key);
  const on = onScreen();
  if (!on || itemKey(on) !== itemKey(agent)) app.selectAgent(agentId);
  // One pane per agent (ESC-678): another browser or page it drives leaves its pane.
  for (const [other, p] of Object.entries(devpages.pages)) {
    if (other !== key && p.projectId === page.projectId && p.driver === agentId && inAPane(other)) leavePane(other);
  }
  app.dropInPane(agent, 'right', { kind: 'page', id: key });
  app.activatePane(itemKey(agent));
}

/** What the window knows of page `page`, read again from the backend. */
export async function refresh(page: string): Promise<AgentPageRow | null> {
  const row = await api.agentpageInfo(page).catch(() => undefined);
  if (row === undefined) return agentpages.info[page] ?? null;
  if (row) adopt(row);
  else {
    agentpages.info[page] = null;
    closePage(pageKey(page), true);
  }
  return row;
}

/** The pages a card is asking about. */
const asking = new Set<string>();

/** Its card asks what became of page `page`, once it is not known (the events keep it true after). */
export function know(page: string) {
  if (asking.has(page) || page in agentpages.info) return;
  asking.add(page);
  void refresh(page).finally(() => asking.delete(page));
}

/** What the backend says of an agent's page. */
export async function onAgentPageEvent(e: AgentPageEvent) {
  const key = pageKey(e.page);
  switch (e.phase) {
    case 'shown': {
      const row = await refresh(e.page);
      if (!row?.open) return;
      if (!row.quiet && !dismissed(row.agentId) && looking(row.agentId, row.projectId)) openBesideAgent(key, row.agentId);
      else if (!shown(key) && !inAPane(key)) {
        const page = devpages.pages[key];
        if (page) page.unseen = true;
      }
      return;
    }
    case 'closed':
      closePage(key, true);
      void refresh(e.page);
      return;
    case 'version':
      // Its agent showed another of its versions (`drive_browser`, ESC-679): its bar says which.
      await refresh(e.page);
      return;
    case 'erased':
      agentpages.info[e.page] = null;
      closePage(key, true);
      return;
    case 'proposed':
      // The page proposed (or its proposal went): the band reads it again, in the request's words.
      void refresh(e.page);
      return;
  }
  const page = devpages.pages[key];
  if (!page) return;
  switch (e.phase) {
    case 'started':
      page.phase = 'loading';
      break;
    case 'finished':
      page.phase = 'ready';
      break;
    case 'wants':
      page.wants = e.url;
      break;
  }
}

/** The user opens page `page` from its card: its row (opened again when it closed), beside its agent's conversation. */
export async function openFromCard(page: string) {
  let row = agentpages.info[page] ?? (await refresh(page));
  if (!row) return;
  if (!row.open) {
    const again = await app.run(api.agentpageReopen(page));
    if (!again) return;
    row = again;
  }
  const key = adopt(row);
  openBesideAgent(key, row.agentId);
}

/** « v2 ▾ » of its bar: the row `key` shows version `n`. */
export async function showVersion(key: string, n: number) {
  const page = pageOfKey(key);
  if (!page) return;
  const row = await app.run(api.agentpageVersion(page, n));
  if (row) adopt(row);
}

/** « La page veut ouvrir… »: in the system's browser (`follow`), or let go. */
export async function answerWants(key: string, follow: boolean) {
  const page = devpages.pages[key];
  const p = pageOfKey(key);
  if (!page || !p) return;
  page.wants = undefined;
  await app.run(api.agentpageOutside(p, follow));
}

/** The rows open when the window starts (a reload of the window: the backend kept them for this run). */
export async function loadRows() {
  const rows = await api.agentpageRows().catch(() => [] as AgentPageRow[]);
  // A core that answers nothing (an older one, or a test's mock) gives no rows, never a throw at the window's start.
  for (const row of Array.isArray(rows) ? rows : []) adopt(row);
}

app.onAgentPage((e) => void onAgentPageEvent(e));

/** The row `key` is on screen: shown alone, or in a pane of the split screen. */
export const rowOnScreen = (key: string): boolean => !!devpages.pages[key] && (shown(key) || inAPane(key));

/** What a call of `show_page` says (its card): its input, and the page its answer names once it is there. */
export interface PageCall {
  title: string;
  template: string;
  /** `p-<n>`, once the call answered. */
  page: string | null;
  version: number | null;
  opened: 'beside' | 'waiting' | null;
}

/** The tool of Escouade's server that shows a page, as every harness names it. */
export const SHOW_PAGE = 'mcp__escouade__show_page';

/**
 * The call `item` of `show_page` as its card draws it; null when it is no call of it, or when it failed (its row of a
 * tool says why). Running, it has no page yet.
 */
export function pageCall(item: {
  name: string;
  input: Record<string, unknown>;
  status: string;
  result?: { text?: string; isError?: boolean };
}): PageCall | null {
  if (item.name !== SHOW_PAGE) return null;
  const title = typeof item.input?.title === 'string' ? item.input.title : '';
  const template = typeof item.input?.template === 'string' ? item.input.template : 'html';
  if (item.status === 'running') return { title, template, page: null, version: null, opened: null };
  if (item.status !== 'ok' || !item.result || item.result.isError) return null;
  try {
    const v = JSON.parse(item.result.text ?? '');
    if (typeof v?.page !== 'string' || !/^p-\d+$/.test(v.page)) return null;
    return {
      title,
      template,
      page: v.page,
      version: typeof v.version === 'number' ? v.version : null,
      opened: v.opened === 'beside' || v.opened === 'waiting' ? v.opened : null,
    };
  } catch {
    return null;
  }
}
