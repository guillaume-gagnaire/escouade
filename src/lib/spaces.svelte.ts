/**
 * The spaces (ESC-539, `docs/SERVER-UX.md` § 5 and 6, direction B « les piles »): this computer and each server it
 * knows. The window is on one of them; it reads each of the others through a connection of its own — a transport to
 * the server through its tunnel, with its own tries (a cut of one never touches another), or this computer's core
 * over the IPC when the window is on a server — and asks it what waits there (`space_glance`) every few seconds: the
 * piles of the rail count it, « Partout » shows it and answers it there, the notifications name its space.
 *
 * **The three safeguards** told the user: the machine is written on each request of « Partout » and on the gesture
 * that answers it (« Autoriser sur Travail »), and a request of a space that is cut is greyed, with no gesture;
 * `servers.aggregateView` to `false` (this computer's policy for a work server, or the server's own) leaves the
 * numbers alone, never a text nor a gesture — the texts never even reach the window then; the piles turn into plain
 * plates beyond three spaces or at 1024 px.
 *
 * A space's own window (`?space=`) reads no other: its title bar says its machine, the main window keeps the piles.
 */

import { connection, connectionOf, type Connection, type ServerInfo } from './connection.svelte';
import { t } from './i18n';
import { locks } from './locks.svelte';
import { goHome, goToServer, loadServers, servers } from './servers.svelte';
import { app } from './state.svelte';
import { answersHere, spaceKeys } from './shortcuts';
import { invoke, local, RemoteTransport, windowSpace, type RemoteLink } from './transport';
import type { AgentStatus, Locks, PendingRequest } from './types';

/** This computer, as a space. */
export const HERE = 'here';
/** How often another space is asked what waits there. */
export const GLANCE_EVERY = 3000;
/** Beyond this many spaces, the piles turn into plates (and at the width below). */
export const PILES_MAX = 3;
export const NARROW_WIDTH = 1024;

export interface GlanceProject {
  id: string;
  name: string;
  color: string;
}

export interface GlanceAgent {
  id: string;
  projectId: string;
  name: string;
  status: AgentStatus;
  activity: string | null;
  lastActivity: number;
  request: PendingRequest | null;
}

export interface GlanceAsk {
  id: string;
  projectId: string;
  title: string;
  createdAt: number;
}

/** What waits in a space (`glance::Glance`). */
export interface Glance {
  machine: { host: string; user: string } | null;
  version: string;
  waiting: number;
  running: number;
  countsOnly: boolean;
  contentless: boolean;
  colors: string[];
  projects: GlanceProject[];
  agents: GlanceAgent[];
  asks: GlanceAsk[];
}

export interface Space {
  /** `here`, or the server's id. */
  id: string;
  here: boolean;
  name: string;
  /** The label under its plate: « Poste » for this computer. */
  short: string;
  /** Its address: the server's host, or this computer's name. */
  host: string;
  /** Its SSH host (`acme-escouade`), none for this computer. */
  ssh: string;
  /** The window is on it. */
  current: boolean;
  /** Its rank, from 1: `Alt+index`. */
  index: number;
  server: ServerInfo | null;
}

/** What the window knows of another space. */
export interface Watched {
  conn: Connection;
  glance: Glance | null;
  /** When it was last read. */
  at: number;
}

export const spacesState = $state<{ watched: Record<string, Watched>; filter: string }>({ watched: {}, filter: 'all' });

/** The space the window is on: its own window's, else the server the main window is on, else this computer. */
export function currentId(): string {
  return windowSpace ?? connection.server?.id ?? HERE;
}

/** The spaces, this computer first (`Alt+1`), then the servers in the order they were added. */
export function spaceList(): Space[] {
  const cur = currentId();
  const list: Space[] = [
    {
      id: HERE,
      here: true,
      name: t('servers.thisComputer'),
      short: t('spaces.here'),
      host: servers.view?.thisDevice?.split(' · ')[0] ?? '',
      ssh: '',
      current: cur === HERE,
      index: 1,
      server: null,
    },
  ];
  const known = servers.view?.servers ?? (connection.server ? [connection.server] : []);
  for (const s of known) {
    list.push({
      id: s.id,
      here: false,
      name: s.name,
      short: s.name,
      host: s.host,
      ssh: s.ssh,
      current: cur === s.id,
      index: list.length + 1,
      server: s,
    });
  }
  return list;
}

/** The spaces are shown (the piles, « Partout », the shortcuts): two of them at least, in the main window. */
export const spacesShown = (): boolean => !windowSpace && spaceList().length > 1;

// ---------- the policies ----------

/** The host of an entry of `servers.allowed` (`ssh://user@host:port`, `*.domain`), as `locks::ssh_host_of` reads it. */
function hostOf(entry: string): string {
  let h = entry.trim().toLowerCase();
  h = h.replace(/^[a-z]+:\/\//, '');
  h = h.split('/')[0];
  h = h.includes('@') ? h.slice(h.lastIndexOf('@') + 1) : h;
  h = h.replace(/:\d+$/, '');
  return h;
}

/** A work server: one of `servers.allowed` when this computer's policy sets it, else any server. Never this computer. */
export function workSpace(s: Pick<Space, 'here' | 'host'>, l: Locks = ownLocks()): boolean {
  if (s.here) return false;
  const lock = l.locks['servers.allowed'];
  const listed = [...(Array.isArray(lock?.value) ? (lock!.value as string[]) : []), ...(lock?.bounds ?? []).flatMap((b) => b.allowed)];
  if (!lock || !listed.length) return true;
  const asked = hostOf(s.host);
  return listed.some((e) => {
    const entry = hostOf(e);
    return entry.startsWith('*.') ? asked.endsWith(entry.slice(1)) : entry === asked;
  });
}

/** This computer's policies: the window's own on this computer, else as this computer's core gave them. */
export function ownLocks(): Locks {
  return connection.server ? (servers.view?.locks ?? locks.current) : locks.current;
}

const imposed = (key: string, value: boolean, l: Locks): boolean => l.locks[key]?.value === value;

/** « Partout » shows the numbers of `s` alone: this computer's policy for a work server, or the space's own. */
export function countsOnly(s: Pick<Space, 'here' | 'host'>, glance: Glance | null, l: Locks = ownLocks()): boolean {
  return (imposed('servers.aggregateView', false, l) && workSpace(s, l)) || !!glance?.countsOnly;
}

/** The notifications of `s` carry no text. */
export function contentless(s: Pick<Space, 'here' | 'host'>, glance: Glance | null, l: Locks = ownLocks()): boolean {
  return (imposed('servers.contentlessNotifications', true, l) && workSpace(s, l)) || !!glance?.contentless;
}

// ---------- reading the other spaces ----------

/** The transports to the other spaces (not reactive: what they say is, in `spacesState`). */
type SpaceLink = Pick<RemoteTransport, 'invoke' | 'close' | 'retryNow'>;
const transports = new Map<string, SpaceLink | null>();

/** A space's connection, given by a test. */
export function setSpaceLink(id: string, link: SpaceLink | null, conn: Connection): void {
  transports.set(id, link);
  spacesState.watched[id] = { conn, glance: null, at: 0 };
}
let poller: ReturnType<typeof setInterval> | null = null;
/** The requests already told, by space: a notification for a new one alone. */
const told = new Map<string, Set<string>>();

/** A command of the space `id`: the window's own transport when it is on it, this computer's core, or its connection. */
export function callIn<T>(id: string, cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (id === currentId()) return invoke<T>(cmd, args);
  if (id === HERE) return local<T>(cmd, args);
  const tr = transports.get(id);
  if (!tr) return Promise.reject(t('spaces.cut', { name: nameOf(id) }));
  return tr.invoke<T>(cmd, args);
}

const nameOf = (id: string): string => spaceList().find((s) => s.id === id)?.name ?? id;

/** Opened: a connection to each other space, closed to those gone; read now. */
export async function syncSpaces(): Promise<void> {
  if (windowSpace) return;
  const others = spacesShown() ? spaceList().filter((s) => !s.current) : [];
  const wanted = new Set(others.map((s) => s.id));
  for (const [id, tr] of transports) {
    if (!wanted.has(id)) {
      tr?.close();
      transports.delete(id);
      delete spacesState.watched[id];
      told.delete(id);
    }
  }
  // The space the window is on: read too, for the requests of its box (its agents are the window's own).
  const cur = currentId();
  if (spacesShown() && !spacesState.watched[cur]) spacesState.watched[cur] = { conn: connection, glance: null, at: 0 };
  for (const s of others) {
    if (transports.has(s.id)) continue;
    const conn = connectionOf();
    if (s.here) {
      conn.state = 'open';
      transports.set(s.id, null);
    } else {
      transports.set(s.id, null);
      const link = await local<RemoteLink | null>('server_link', { space: s.id }).catch(() => null);
      if (!transports.has(s.id)) continue;
      if (!link) {
        // Not known any more, or this computer's core did not answer: tried again at the next sync.
        transports.delete(s.id);
        continue;
      }
      transports.set(
        s.id,
        new RemoteTransport(link, undefined, undefined, undefined, () => local<RemoteLink | null>('server_link', { space: s.id }), conn),
      );
    }
    spacesState.watched[s.id] = { conn, glance: null, at: 0 };
  }
  void pollAll();
}

/** Each other space asked what waits there (one that is cut keeps what it said last). */
export async function pollAll(): Promise<void> {
  await Promise.all(
    spaceList()
      .filter((s) => spacesState.watched[s.id])
      .map((s) => poll(s)),
  );
}

async function poll(s: Space): Promise<void> {
  const w = spacesState.watched[s.id];
  if (!w || (w.conn.state !== 'open' && w.conn.state !== 'local')) return;
  try {
    const counts = countsOnly(s, null);
    const g = await callIn<Glance>(s.id, 'space_glance', { counts });
    const now = spacesState.watched[s.id];
    if (!now) return;
    now.glance = g;
    now.at = Date.now();
    if (!s.here && !s.current) notifyNew(s, g);
  } catch {
    // Cut meanwhile: its band says so.
  }
}

/** The notification of a request of another space, named by it (pure, for the tests). */
export function notification(
  space: Pick<Space, 'name'>,
  g: Pick<Glance, 'projects'>,
  a: Pick<GlanceAgent, 'name' | 'projectId' | 'request'>,
  bare: boolean,
): { title: string; body: string } {
  const project = g.projects.find((p) => p.id === a.projectId)?.name;
  const title = [space.name, ...(bare ? [] : [project, a.name].filter(Boolean))].join(' · ');
  if (bare) return { title, body: t('spaces.notify.bare') };
  const r = a.request;
  const body = r
    ? r.kind === 'question'
      ? t('spaces.notify.question', { text: r.questions[0]?.question ?? r.arg })
      : t('spaces.notify.permission', { tool: r.tool, arg: r.arg.split('\n')[0] })
    : t('spaces.notify.waits');
  return { title, body };
}

/** What is new in `g` since the last glance of `s`, told (the first glance tells nothing: it is what was there). */
function notifyNew(s: Space, g: Glance): void {
  const keys = new Set([
    ...g.agents.filter((a) => a.status === 'waiting').map((a) => `${a.id}:${a.request?.id ?? ''}`),
    ...g.asks.map((a) => `ask:${a.id}`),
  ]);
  const before = told.get(s.id);
  told.set(s.id, keys);
  if (!before) return;
  const bare = contentless(s, g);
  for (const a of g.agents) {
    const key = `${a.id}:${a.request?.id ?? ''}`;
    if (a.status !== 'waiting' || before.has(key)) continue;
    const { title, body } = notification(s, g, a, bare);
    void local('space_notify', { space: s.id, title, body, agentId: a.id, projectId: a.projectId }).catch(() => {});
  }
}

/** The connections to the other spaces opened, read every few seconds, kept to the servers known. */
export function startSpaces(): () => void {
  if (windowSpace || poller) return () => {};
  void loadServers().then(() => syncSpaces());
  poller = setInterval(() => {
    // On a server, this computer's policies come with its list of servers: read again, a change of them follows.
    if (connection.server) void loadServers();
    void pollAll();
  }, GLANCE_EVERY);
  return () => {
    if (poller) clearInterval(poller);
    poller = null;
    for (const tr of transports.values()) tr?.close();
    transports.clear();
    spacesState.watched = {};
  };
}

// ---------- going from one to another ----------

const OPEN_KEY = 'escouade.spaceOpen';

/** Where to go once the window is loaded again on a space. */
interface Opening {
  space: string;
  agentId?: string;
  projectId?: string;
}

/**
 * The window to the space `id` (S14: its own window brought back when it has one); `then`, the agent or project to show
 * there. Gives the problem that kept it from going, if any.
 */
export async function goToSpace(id: string, then?: { agentId?: string; projectId?: string }): Promise<string | null> {
  if (id === currentId()) {
    if (then?.agentId && app.agents[then.agentId]) app.selectAgent(then.agentId);
    else if (then?.projectId) app.selectProject(then.projectId);
    return null;
  }
  if (await local<boolean>('space_window', { space: id, existing: true }).catch(() => false)) return null;
  try {
    sessionStorage.setItem(OPEN_KEY, JSON.stringify({ space: id, ...then } satisfies Opening));
  } catch {
    // No storage: the space opens on its last screen.
  }
  if (id === HERE) {
    await goHome();
    return null;
  }
  const problem = await goToServer(id);
  return problem ? t('spaces.cantGo', { name: nameOf(id) }) : null;
}

/** After a load on a space: the agent or project asked, shown. */
export function takeOpening(): void {
  let o: Opening | null = null;
  try {
    o = JSON.parse(sessionStorage.getItem(OPEN_KEY) ?? 'null');
    sessionStorage.removeItem(OPEN_KEY);
  } catch {
    return;
  }
  if (!o || o.space !== currentId()) return;
  if (o.projectId && app.projects.some((p) => p.id === o.projectId)) app.selectProject(o.projectId);
  if (o.agentId && app.agents[o.agentId]) app.selectAgent(o.agentId);
}

/** `Alt+n` / `⌃n` (`n` from 1), or `next` (`Ctrl+²`): the space it names. Whether there was one to go to. */
export function spaceShortcut(k: number | 'next'): boolean {
  if (!spacesShown()) return false;
  const list = spaceList();
  const i = list.findIndex((s) => s.current);
  const to = k === 'next' ? list[(i + 1) % list.length] : list[k - 1];
  if (!to) return false;
  void goToSpace(to.id);
  return true;
}

// The keys of the spaces, for `handleShortcut`: a question on screen keeps `Alt+n` for its options, as before.
spaceKeys.go = (k, target) => {
  if (typeof k === 'number' && answersHere(target) && app.agent?.requests?.some((r) => r.kind === 'question')) return false;
  return spaceShortcut(k);
};
spaceKeys.shown = spacesShown;

/** A space in its own window. */
export function openWindow(id: string): Promise<boolean> {
  return local<boolean>('space_window', { space: id });
}

/** « Réessayer » on a space that is cut: its tunnel, then its connection. */
export async function retrySpace(id: string): Promise<void> {
  await local('server_retry', { id }).catch(() => {});
  transports.get(id)?.retryNow();
}

/** What waits in the space `s`: its glance's count, or the window's own for the space it is on. */
export function waitingIn(s: Space): number {
  if (s.current) return Object.values(app.agents).filter((a) => !a.archived && a.status === 'waiting').length;
  return spacesState.watched[s.id]?.glance?.waiting ?? 0;
}

/** How the connection to `s` stands: `open`, or cut (`lost`, `connecting`, `refused`). */
export function stateOf(s: Space): Connection['state'] {
  if (s.current) return connection.state === 'local' ? 'open' : connection.state;
  return spacesState.watched[s.id]?.conn.state ?? 'connecting';
}

/** The piles turn into plates: beyond three spaces, or at 1024 px. */
export function compactPiles(count: number, width: number): boolean {
  return count > PILES_MAX || width <= NARROW_WIDTH;
}
