/**
 * The phone's spaces (MO-1, docs/MOBILE.md § 3.2 and § 5): one connection to each, with its own tries — the app's
 * `RemoteTransport` (src/lib/transport.ts, the WebSocket of the remote API), as the desktop window reads its other
 * spaces (src/lib/spaces.svelte.ts) — and what waits there, asked every few seconds (`space_glance`). A cut of one never
 * touches another; « Partout » is all of them, a chip one.
 *
 * In the foreground alone: the app closes them when it goes to the background (`pauseSpaces`) and opens them again
 * when it comes back (`resumeSpaces`); what each said last stays shown meanwhile, dated (M13).
 *
 * The phone has no core of its own: a command the core keeps for its own window (`desk`) is refused here, never run.
 */

import { connectionOf, type Connection } from '$shared/lib/connection.svelte';
import { t } from '$shared/lib/i18n';
import type { InboxItem, InboxViews } from '$shared/lib/inbox/views';
import type { Glance, GlanceAgent, GlanceAsk, GlanceProject } from '$shared/lib/spaces.svelte';
import type { InboxView, WindowAsk } from '$shared/lib/types';
import { RemoteTransport, RETRY_DELAYS, type RemoteLink, type Socket } from '$shared/lib/transport';
import { EscouadeMobile, type EscouadeMobilePlugin } from '@escouade/capacitor-mobile';
import { keepGlance, keptGlances } from './kept';
import type { PairedSpace } from './paired';

/** How often each space is asked what waits there. */
export const GLANCE_EVERY = 3000;
/** The box of a space is read again when its glance changes, and at least that often (its views: a ticket to approve). */
export const BOX_EVERY = 15_000;

/** The box « À toi » of a space (MO-5): its requests and its views, as `inbox_list` and `inbox_views` give them. */
export interface SpaceBox {
  /** The core's MCP server is on: off, the box holds nothing (its agents still stop, the glance says them). */
  enabled: boolean;
  asks: WindowAsk[];
  views: InboxItem[];
  /** When it was read (ms). */
  at: number;
}

/** A space, as the phone shows it. */
export interface PhoneSpace {
  id: string;
  name: string;
  here: boolean;
  /** How its connection stands (the transport writes it). */
  conn: Connection;
  /** What it said last, kept through a cut. */
  glance: Glance | null;
  /** When it said it (ms), 0 before. */
  at: number;
  /** Its box, once read (MO-5); none before, and never kept on the phone (M13). */
  box: SpaceBox | null;
}

/** « Partout », or a space's id. */
export const ALL = 'all';

export const phone = $state<{ spaces: PhoneSpace[]; filter: string }>({ spaces: [], filter: ALL });

type Link = Pick<RemoteTransport, 'invoke' | 'close' | 'retryNow' | 'onStale'>;

export interface SpacesOptions {
  /** The socket to open (a test's), the browser's `WebSocket` else. */
  open?: (url: string) => Socket;
  delays?: number[];
  every?: number;
  /** The app's version, said in the handshake. */
  version?: string;
  /** The native side the ways of the spaces kept by the app are asked of (a test's). */
  native?: Pick<EscouadeMobilePlugin, 'startTunnel' | 'tunnelStatus' | 'stopTunnel'>;
}

const links = new Map<string, Link>();
const busy = new Set<string>();
let poller: ReturnType<typeof setInterval> | null = null;
let given: PairedSpace[] = [];
let options: SpacesOptions = {};

declare const __ESCOUADE_VERSION__: string;
const appVersion = (): string => (typeof __ESCOUADE_VERSION__ === 'string' ? __ESCOUADE_VERSION__ : '0.0.0');

/** A command the core keeps for its own window: the phone has none to run it. */
const deskOnly = <T>(cmd: string): Promise<T> => Promise.reject(t('mobile.error.deskOnly', { command: cmd }));

/** A connection to each space of `paired`, read now, then every few seconds. Closes the ones opened before. */
export function startSpaces(paired: PairedSpace[], opts: SpacesOptions = {}): () => void {
  stopSpaces();
  given = paired;
  options = opts;
  const kept = new Map(phone.spaces.map((s) => [s.id, s]));
  // At the opening, what each space said last before (M13): shown, dated, until it says again.
  const stored = keptGlances();
  phone.spaces = paired.map((p) => ({
    id: p.id,
    name: p.name,
    here: p.here,
    conn: connectionOf(),
    glance: kept.get(p.id)?.glance ?? stored[p.id]?.glance ?? null,
    at: kept.get(p.id)?.at ?? stored[p.id]?.at ?? 0,
    box: kept.get(p.id)?.box ?? null,
  }));
  if (phone.filter !== ALL && !paired.some((p) => p.id === phone.filter)) phone.filter = ALL;
  const version = opts.version ?? appVersion();
  for (const [i, p] of paired.entries()) {
    const conn = phone.spaces[i].conn;
    const made = (first: RemoteLink, refresh: (() => Promise<RemoteLink | null>) | null) => {
      const link = new RemoteTransport(first, opts.open, deskOnly, opts.delays ?? RETRY_DELAYS, refresh, conn);
      links.set(p.id, link);
      link.onStale(() => void poll(p.id));
      void poll(p.id);
    };
    if (!p.kept) {
      made({ url: p.url, token: p.token, version }, null);
      continue;
    }
    // Paired by the app (MO-4): its way asked of the native side, now and before each try (its tunnel's proxy may
    // have taken another port). The page gives an empty token: the proxy writes it (SR1-15).
    const era = generation;
    const refresh = () => linkOf(p.id, version, opts.native ?? EscouadeMobile);
    void refresh()
      .catch(() => null)
      .then((first) => {
        if (era !== generation) return;
        made(first ?? { url: `ws://127.0.0.1:9/${p.id}`, token: '', version }, refresh);
      });
  }
  poller = setInterval(() => void pollAll(), opts.every ?? GLANCE_EVERY);
  return stopSpaces;
}

/** Each start of the spaces: what a late answer of the native side belongs to. */
let generation = 0;

/** The way to a space kept by the app: its tunnel up (or its address's proxy), its WebSocket through the proxy. */
export async function linkOf(
  id: string,
  version: string,
  native: Pick<EscouadeMobilePlugin, 'startTunnel' | 'tunnelStatus'>,
  waitMs = 15_000,
): Promise<RemoteLink> {
  let s = await native.tunnelStatus({ space: id }).catch(() => null);
  if (!s || s.state === 'stopped' || s.state === 'failed') s = await native.startTunnel({ space: id });
  const until = Date.now() + waitMs;
  while (s.state === 'connecting' && Date.now() < until) {
    await new Promise((r) => setTimeout(r, 250));
    s = await native.tunnelStatus({ space: id });
  }
  return { url: s.websocketUrl, token: s.token ?? '', version };
}

/** The ways of the spaces kept by the app stopped (the app in the background: no tunnel left running). */
async function stopWays(native: Pick<EscouadeMobilePlugin, 'stopTunnel'>): Promise<void> {
  await Promise.all(given.filter((p) => p.kept).map((p) => native.stopTunnel({ space: p.id }).catch(() => {})));
}

/** Every connection closed (what they said last is kept). */
export function stopSpaces(): void {
  generation++;
  if (poller) clearInterval(poller);
  poller = null;
  for (const link of links.values()) link.close();
  links.clear();
  busy.clear();
  boxBusy.clear();
}

/** The app went to the background: no connection while it is there (§ 5). */
export function pauseSpaces(): void {
  stopSpaces();
  void stopWays(options.native ?? EscouadeMobile);
}

/** The app came back: the connections opened again, each space read again. */
export function resumeSpaces(): void {
  if (given.length && !links.size) startSpaces(given, options);
}

/** « Réessayer » on a space that is cut. */
export function retrySpace(id: string): void {
  links.get(id)?.retryNow();
}

/** A command of the space `id`, through its own connection. */
export function callIn<T>(id: string, cmd: string, args?: Record<string, unknown>): Promise<T> {
  const link = links.get(id);
  if (!link) return Promise.reject(t('mobile.spaces.chipCut', { name: spaceOf(id)?.name ?? id }));
  return link.invoke<T>(cmd, args);
}

const spaceOf = (id: string): PhoneSpace | undefined => phone.spaces.find((s) => s.id === id);

/** Each space that is open asked what waits there. */
export async function pollAll(): Promise<void> {
  await Promise.all(phone.spaces.filter((s) => s.conn.state === 'open').map((s) => poll(s.id)));
}

async function poll(id: string): Promise<void> {
  const link = links.get(id);
  if (!link || busy.has(id)) return;
  busy.add(id);
  try {
    // Before its first connection, the call waits for it.
    const g = await link.invoke<Glance>('space_glance', { counts: false });
    const s = spaceOf(id);
    if (s && links.get(id) === link) {
      const changed = glanceMark(s.glance) !== glanceMark(g);
      s.glance = g;
      s.at = Date.now();
      keepGlance(id, g, s.at);
      if (g.countsOnly) s.box = null;
      else if (changed || !s.box || s.at - s.box.at >= BOX_EVERY) void readBox(id);
    }
  } catch {
    // Cut meanwhile: its chip and its band say so.
  } finally {
    busy.delete(id);
  }
}

/** What of a glance says the box changed: the agents that wait and on what, the requests open. */
export function glanceMark(g: Glance | null): string {
  if (!g) return '';
  const agents = g.agents.map((a) => `${a.id}:${a.status}:${a.request?.id ?? ''}`).join(',');
  return `${g.waiting}|${agents}|${g.asks.map((a) => a.id).join(',')}`;
}

const boxBusy = new Set<string>();

/** Reads the box of the space `id` again (after a glance that changed, or a gesture). */
export async function readBox(id: string): Promise<void> {
  const link = links.get(id);
  if (!link || boxBusy.has(id)) return;
  boxBusy.add(id);
  try {
    const [list, views] = await Promise.all([
      link.invoke<InboxView>('inbox_list', { projectId: null }),
      link.invoke<InboxViews>('inbox_views', { projectId: null }),
    ]);
    const s = spaceOf(id);
    if (s && links.get(id) === link) s.box = { enabled: list.enabled, asks: list.asks, views: views.views, at: Date.now() };
  } catch {
    // Cut, or a core without the box: the glance alone says what waits.
  } finally {
    boxBusy.delete(id);
  }
}

/** The glance and the box of a space read again at once (after a gesture). */
export async function refreshSpace(id: string): Promise<void> {
  await poll(id);
  // The poll may have started a reading: wait for it, then read once more for what the gesture changed.
  for (let i = 0; i < 40 && boxBusy.has(id); i++) await new Promise((r) => setTimeout(r, 50));
  await readBox(id);
}

// ---------- what the screens read (pure) ----------

/** How a space stands, as its chip and its band say it. */
export type SpaceState = 'open' | 'connecting' | 'cut' | 'refused';

export function stateOf(s: Pick<PhoneSpace, 'conn'>): SpaceState {
  switch (s.conn.state) {
    case 'open':
    case 'local':
      return 'open';
    case 'lost':
      return 'cut';
    case 'refused':
      return 'refused';
    default:
      return s.conn.lostAt !== null || s.conn.tries > 0 ? 'cut' : 'connecting';
  }
}

/** The spaces that revoked this phone, or no longer know its token (MO-4): each has its screen. */
export const revokedSpaces = (spaces: PhoneSpace[]): PhoneSpace[] =>
  spaces.filter((s) => s.conn.state === 'refused' && s.conn.refusal?.code === 'token');

/**
 * What waits in a space: as the box of the desktop counts it once its box is read (the requests open, the agents
 * stopped, the views that wait for the user and are no agent's request: a ticket to approve, a decision), else its
 * glance's count (0 before it said).
 */
export function waitingIn(s: Pick<PhoneSpace, 'glance'> & { box?: SpaceBox | null }): number {
  const g = s.glance;
  if (!s.box?.enabled || !g || g.countsOnly) return g?.waiting ?? 0;
  const views = s.box.views;
  const handled = new Set(views.filter((v) => v.group === 'stopped' && v.mark && v.agentId).map((v) => v.agentId));
  const stopped = g.agents.filter((a) => a.status === 'waiting' && !handled.has(a.id)).length;
  const asks = s.box.asks.filter((a) => a.state === 'open').length;
  const other = views.filter((v) => !v.snoozedUntil && !v.mark && v.group !== 'stopped').length;
  return stopped + asks + other;
}

/** The spaces a filter shows: all of them for « Partout ». */
export function shownSpaces<S extends { id: string }>(spaces: S[], filter: string): S[] {
  return filter === ALL ? spaces : spaces.filter((s) => s.id === filter);
}

/** An agent or a request of a space, with what names it. */
export interface Item<T> {
  space: Pick<PhoneSpace, 'id' | 'name' | 'here'>;
  project: GlanceProject | null;
  item: T;
}

function itemsOf<T>(spaces: PhoneSpace[], pick: (g: Glance) => T[], projectId: (x: T) => string): Item<T>[] {
  const out: Item<T>[] = [];
  for (const s of spaces) {
    const g = s.glance;
    if (!g || g.countsOnly) continue;
    for (const x of pick(g)) {
      out.push({
        space: { id: s.id, name: s.name, here: s.here },
        project: g.projects.find((p) => p.id === projectId(x)) ?? null,
        item: x,
      });
    }
  }
  return out;
}

/** « Tes agents se sont arrêtés »: the agents that wait, the longest waiting first, of every space shown. */
export function stoppedAgents(spaces: PhoneSpace[]): Item<GlanceAgent>[] {
  return itemsOf(
    spaces,
    (g) => g.agents.filter((a) => a.status === 'waiting'),
    (a) => a.projectId,
  ).sort((a, b) => a.item.lastActivity - b.item.lastActivity);
}

/** The requests of the box « À toi » of every space shown, the newest first. */
export function boxAsks(spaces: PhoneSpace[]): Item<GlanceAsk>[] {
  return itemsOf(
    spaces,
    (g) => g.asks,
    (k) => k.projectId,
  ).sort((a, b) => b.item.createdAt - a.item.createdAt);
}

/** The agents of every space shown: those that wait, then those at work (the latest first). */
export function allAgents(spaces: PhoneSpace[]): Item<GlanceAgent>[] {
  const items = itemsOf(
    spaces,
    (g) => g.agents,
    (a) => a.projectId,
  );
  const waiting = items.filter((x) => x.item.status === 'waiting').sort((a, b) => a.item.lastActivity - b.item.lastActivity);
  const working = items.filter((x) => x.item.status !== 'waiting').sort((a, b) => b.item.lastActivity - a.item.lastActivity);
  return [...waiting, ...working];
}

/** The spaces shown whose glance gives its numbers alone (`servers.aggregateView`). */
export const countsOnlySpaces = (spaces: PhoneSpace[]): PhoneSpace[] => spaces.filter((s) => s.glance?.countsOnly);
