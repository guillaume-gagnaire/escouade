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
import type { Glance, GlanceAgent, GlanceAsk, GlanceProject } from '$shared/lib/spaces.svelte';
import { RemoteTransport, RETRY_DELAYS, type Socket } from '$shared/lib/transport';
import type { PairedSpace } from './paired';

/** How often each space is asked what waits there. */
export const GLANCE_EVERY = 3000;

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
  phone.spaces = paired.map((p) => ({
    id: p.id,
    name: p.name,
    here: p.here,
    conn: connectionOf(),
    glance: kept.get(p.id)?.glance ?? null,
    at: kept.get(p.id)?.at ?? 0,
  }));
  if (phone.filter !== ALL && !paired.some((p) => p.id === phone.filter)) phone.filter = ALL;
  for (const [i, p] of paired.entries()) {
    const conn = phone.spaces[i].conn;
    const link = new RemoteTransport(
      { url: p.url, token: p.token, version: opts.version ?? appVersion() },
      opts.open,
      deskOnly,
      opts.delays ?? RETRY_DELAYS,
      null,
      conn,
    );
    links.set(p.id, link);
    link.onStale(() => void poll(p.id));
    void poll(p.id);
  }
  poller = setInterval(() => void pollAll(), opts.every ?? GLANCE_EVERY);
  return stopSpaces;
}

/** Every connection closed (what they said last is kept). */
export function stopSpaces(): void {
  if (poller) clearInterval(poller);
  poller = null;
  for (const link of links.values()) link.close();
  links.clear();
  busy.clear();
}

/** The app went to the background: no connection while it is there (§ 5). */
export function pauseSpaces(): void {
  stopSpaces();
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
      s.glance = g;
      s.at = Date.now();
    }
  } catch {
    // Cut meanwhile: its chip and its band say so.
  } finally {
    busy.delete(id);
  }
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

/** What waits in a space: its glance's count (0 before it said). */
export const waitingIn = (s: Pick<PhoneSpace, 'glance'>): number => s.glance?.waiting ?? 0;

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
