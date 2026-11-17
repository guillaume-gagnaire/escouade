// The box « À toi » as the window holds it (ESC-510, docs/INBOX.md, direction B « La file »): the requests read from the
// core (`inbox_list`, every project), read again on its event `inbox` and when the MCP server is turned on or off; the
// queue's groups, its count, the request open, the files read once.
//
// The box is the MCP server's (§ 6): off, it holds nothing and the window shows nothing of it.

import { fDateShort, fTime } from '../format';
import { intlLocale, t } from '../i18n';

import { api } from '../ipc';
import { app, asksUser } from '../state.svelte';
import type { Agent, AskFile, WindowAsk } from '../types';

/** A closed request stays in « Répondu récemment » that long, and so many at most (§ 3.4). */
export const RECENT_MS = 7 * 24 * 3600 * 1000;
export const RECENT_MAX = 50;

export type Scope = 'all' | 'project';

/** What the queue shows, row by row: an agent stopped (a blocking request, until IN-3's views), or a request. */
export type QueueItem = { kind: 'stopped'; agent: Agent } | { kind: 'ask'; ask: WindowAsk };

export interface QueueGroups {
  stopped: QueueItem[];
  today: QueueItem[];
  earlier: QueueItem[];
  later: WindowAsk[];
  recent: WindowAsk[];
}

/** The key of a row: what `selected` holds. */
export const itemKey = (i: QueueItem) => (i.kind === 'ask' ? i.ask.id : `agent:${i.agent.id}`);

const startOfDay = (now: number) => {
  const d = new Date(now);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
};

/** A request that waits in the queue now (not put off for later). */
export const isOpen = (a: WindowAsk) => a.state === 'open';

/** It goes in « Aujourd'hui »: urgent today, back from « Plus tard », a ticket waits for it, or left today. */
export function isToday(a: WindowAsk, now = Date.now()) {
  return a.urgency === 'today' || a.woke || a.tickets.length > 0 || a.createdAt >= startOfDay(now);
}

/** Answered, withdrawn, expired or taken out: closed, or answered but not read yet. */
export const isClosed = (a: WindowAsk) => !['open', 'snoozed'].includes(a.state);

/** When it left the queue: its answer, else when it closed. */
export const closedAt = (a: WindowAsk) => a.answer?.at ?? a.closedAt ?? a.updatedAt;

/** The queue's groups (§ 10.2), in their order: the stopped agents first, the oldest first; then today, earlier. */
export function groups(asks: WindowAsk[], stopped: Agent[], now = Date.now()): QueueGroups {
  const open = asks.filter(isOpen);
  const byAge = (a: WindowAsk, b: WindowAsk) =>
    Number(b.urgency === 'today') - Number(a.urgency === 'today') || Number(b.woke) - Number(a.woke) || a.createdAt - b.createdAt;
  const today = open.filter((a) => isToday(a, now)).sort(byAge);
  const earlier = open.filter((a) => !isToday(a, now)).sort(byAge);
  return {
    stopped: [...stopped].sort((a, b) => a.lastActivity - b.lastActivity).map((agent) => ({ kind: 'stopped', agent })),
    today: today.map((ask) => ({ kind: 'ask', ask })),
    earlier: earlier.map((ask) => ({ kind: 'ask', ask })),
    later: asks.filter((a) => a.state === 'snoozed').sort((a, b) => (a.snoozedUntil ?? 0) - (b.snoozedUntil ?? 0)),
    recent: asks
      .filter((a) => isClosed(a) && now - closedAt(a) <= RECENT_MS)
      .sort((a, b) => closedAt(b) - closedAt(a))
      .slice(0, RECENT_MAX),
  };
}

/** The rows one answers in turn (J, K): the stopped agents, then today, then earlier. */
export const actionable = (g: QueueGroups): QueueItem[] => [...g.stopped, ...g.today, ...g.earlier];

/**
 * What the count of the rail says (§ 10.1): what waits for the user in the box, every project — the requests open and
 * the agents stopped, the latter being the same agents as « N en attente » of the status bar (`asksUser`): never two
 * numbers that disagree. A request put off for later does not count until it comes back.
 */
export function inboxCount(asks: WindowAsk[], stopped: Agent[]): { asks: number; stopped: number; total: number } {
  const n = asks.filter(isOpen).length;
  return { asks: n, stopped: stopped.length, total: n + stopped.length };
}

/** The agents stopped on a request (permission, question, plan, a deletion asked), as the status bar counts them. */
export const stoppedAgents = (agents: Record<string, Agent>, projectId?: string | null) =>
  Object.values(agents).filter((a) => !a.archived && asksUser(a) && (!projectId || a.projectId === projectId));

/** What a blocking request waits for, for its row. */
export function stoppedWhat(a: Agent): 'permission' | 'question' | 'plan' | 'delete' {
  const r = a.requests?.[0];
  if (!r) return a.deleteAsk ? 'delete' : 'permission';
  if (r.kind === 'question') return 'question';
  return r.tool === 'ExitPlanMode' ? 'plan' : 'permission';
}

/** What a stopped agent's row says it waits for: its question, or the tool and what it acts on. */
export function stoppedTitle(a: Agent): string {
  const r = a.requests?.[0];
  if (!r) return '';
  if (r.kind === 'question') return r.questions[0]?.question ?? r.arg;
  return [r.tool, r.arg.split('\n')[0]].filter(Boolean).join(' : ');
}

/** A file's bytes, read from the core's copy by parts of 4 MB. */
export async function readFile(id: string, f: Pick<AskFile, 'file' | 'size'>): Promise<Uint8Array> {
  const out = new Uint8Array(f.size);
  let at = 0;
  while (at < f.size) {
    const c = await api.inboxFile(id, f.file, at);
    const bytes = Uint8Array.from(atob(c.data), (ch) => ch.charCodeAt(0));
    if (!bytes.length) break;
    out.set(bytes.subarray(0, Math.max(0, f.size - at)), at);
    at += bytes.length;
  }
  return at >= f.size ? out : out.subarray(0, at);
}

/** Bytes as a `data:` URL of their media type. */
export function dataUrl(b: Uint8Array, type: string): Promise<string> {
  return new Promise((ok, ko) => {
    const r = new FileReader();
    r.onload = () => ok(String(r.result));
    r.onerror = () => ko(r.error);
    r.readAsDataURL(new Blob([b as BlobPart], { type }));
  });
}

/** The images of an option (or of the request itself, `option` null), in their order. */
export const imagesOf = (a: WindowAsk, option: string | null) => a.files.filter((f) => f.kind === 'image' && f.option === option);

/** Whether an image is the light render of its option, by its name (« -clair », « -light »). */
export const isLight = (f: AskFile) => /(^|[-_. ])(clair|light)([-_. ]|$)/i.test(f.name.replace(/\.[^.]+$/, ''));

/** The letter of an option, as the box writes it: « B ». */
export const letter = (id: string) => id.toUpperCase();

class Inbox {
  /** The requests of every project, as the core gave them; empty while the server is off. */
  asks = $state.raw<WindowAsk[]>([]);
  enabled = $state(false);
  loaded = $state(false);
  scope = $state<Scope>('all');
  /** The row open in the panel (`itemKey`). */
  selected = $state<string | null>(null);
  /** The last answer given, while its 10 seconds run: « Annuler » takes it back. */
  /** The pages of each PDF read, by `<ask>/<file>`: its chip says them once the reader opened it. */
  pages = $state<Record<string, number>>({});
  sending = $state<{ id: string; title: string; sendAt: number } | null>(null);
  /** A request's new arrival, said to a screen reader without moving the focus. */
  announce = $state<{ author: string; title: string } | null>(null);
  /** The answers handled today, for the meter. */
  private seq = 0;
  private known = new Set<string>();
  /** The files read, by `<ask>/<file>`: a `data:` URL of an image, the bytes of a file. */
  private urls = new Map<string, Promise<string>>();
  private bytes = new Map<string, Promise<Uint8Array>>();

  /** The agents stopped, in the scope. */
  stopped = $derived(stoppedAgents(app.agents, this.scope === 'project' ? app.ui.activeProject : null));
  /** The requests in the scope. */
  scoped = $derived(this.scope === 'project' ? this.asks.filter((a) => a.projectId === app.ui.activeProject) : this.asks);
  groups = $derived(groups(this.scoped, this.stopped));
  rows = $derived(actionable(this.groups));
  /** The rail's count: every project, whatever the scope. */
  count = $derived(inboxCount(this.asks, stoppedAgents(app.agents)));
  /** In the scope: the header's number. */
  scopedCount = $derived(inboxCount(this.scoped, this.stopped));
  /** Handled today: answered by the user (or for them) since midnight. */
  doneToday = $derived(this.scoped.filter((a) => a.answer && a.answer.at >= startOfDay(Date.now())).length);

  /** The row open, found again among the requests as they are now. */
  current = $derived.by((): QueueItem | null => {
    const key = this.selected;
    if (!key) return null;
    if (key.startsWith('agent:')) {
      const agent = app.agents[key.slice(6)];
      return agent ? { kind: 'stopped', agent } : null;
    }
    const ask = this.asks.find((a) => a.id === key);
    return ask ? { kind: 'ask', ask } : null;
  });

  /** Reads the box again: what the core holds now. */
  async load() {
    const seq = ++this.seq;
    try {
      const v = await api.inboxList();
      if (seq !== this.seq) return;
      this.enabled = v.enabled;
      this.tellNew(v.asks);
      this.asks = v.asks;
    } catch {
      if (seq !== this.seq) return;
      this.enabled = false;
      this.asks = [];
    }
    this.loaded = true;
    if (this.sending && !this.asks.some((a) => a.id === this.sending?.id && a.state === 'answered' && a.answer?.sendAt))
      this.sending = null;
  }

  /** The requests left since the last reading, said once (not at the first reading). */
  private tellNew(asks: WindowAsk[]) {
    const fresh = asks.filter((a) => a.state === 'open' && !this.known.has(a.id));
    for (const a of asks) this.known.add(a.id);
    if (!this.loaded || !fresh.length) return;
    const a = fresh[fresh.length - 1];
    this.announce = { author: a.author.name, title: a.title };
  }

  /** The server turned off: nothing of the box stays in the window. */
  clear() {
    this.seq++;
    this.enabled = false;
    this.asks = [];
    this.selected = null;
    this.sending = null;
    this.urls.clear();
    this.bytes.clear();
    this.pages = {};
  }

  /** As at the start (the tests). */
  reset() {
    this.clear();
    this.loaded = false;
    this.scope = 'all';
    this.announce = null;
    this.known.clear();
  }

  /** A request changed by a command: in place, at once, before the event reads the box again. */
  put(a: WindowAsk) {
    this.asks = this.asks.some((x) => x.id === a.id) ? this.asks.map((x) => (x.id === a.id ? a : x)) : [...this.asks, a];
  }

  /**
   * Opens a row (none: the first that waits) and, opened by the user (`seen`), tells the core it was seen: a request opened
   * by itself keeps its marks (« revenue de Plus tard ») until the user goes to it.
   */
  select(key: string | null, seen = true) {
    this.selected = key ?? (this.rows[0] ? itemKey(this.rows[0]) : null);
    const id = this.selected;
    if (seen && id && !id.startsWith('agent:')) {
      const a = this.asks.find((x) => x.id === id);
      if (a && (!a.seen || a.woke)) void api.inboxSeen(id).catch(() => {});
    }
  }

  /** The place of the row open among those that wait (from 1), and their number: « 3 sur 9 ». */
  position(): { n: number; total: number } {
    const i = this.rows.findIndex((r) => itemKey(r) === this.selected);
    return { n: i + 1, total: this.rows.length };
  }

  /** The next (or previous) row that waits, around the ends. */
  step(by: 1 | -1) {
    const rows = this.rows;
    if (!rows.length) return;
    const i = rows.findIndex((r) => itemKey(r) === this.selected);
    const next = i < 0 ? (by > 0 ? 0 : rows.length - 1) : (i + by + rows.length) % rows.length;
    this.select(itemKey(rows[next]));
  }

  /** After an answer or a « Plus tard »: the row that came after it, or the first. */
  after(key: string) {
    const rows = this.rows.filter((r) => itemKey(r) !== key);
    const i = this.rows.findIndex((r) => itemKey(r) === key);
    const next = rows[Math.min(Math.max(0, i), rows.length - 1)];
    this.select(next ? itemKey(next) : null);
  }

  /**
   * An image of a request, as a `data:` URL, read once: what the viewer shows, copies and saves (« Enregistrer sous… » takes
   * the image itself, never a path).
   */
  imageUrl(a: Pick<WindowAsk, 'id'>, f: AskFile): Promise<string> {
    const key = `${a.id}/${f.file}`;
    let u = this.urls.get(key);
    if (!u) {
      u = this.fileBytes(a, f).then((b) => dataUrl(b, f.mediaType));
      u.catch(() => this.urls.delete(key));
      this.urls.set(key, u);
    }
    return u;
  }

  /** A file's bytes, read once. */
  fileBytes(a: Pick<WindowAsk, 'id'>, f: AskFile): Promise<Uint8Array> {
    const key = `${a.id}/${f.file}`;
    let b = this.bytes.get(key);
    if (!b) {
      b = readFile(a.id, f);
      b.catch(() => this.bytes.delete(key));
      this.bytes.set(key, b);
    }
    return b;
  }
}

export const inbox = new Inbox();

/** The times « Plus tard » offers (§ 8.4), from `now`: in an hour, this evening at 18:00 (before 17:00), tomorrow at 9:00, Monday at 9:00 (unless tomorrow is Monday). */
export function laterTimes(now = Date.now()): { key: 'hour' | 'evening' | 'tomorrow' | 'monday'; at: number }[] {
  const d = new Date(now);
  const at = (days: number, h: number) => {
    const x = new Date(d);
    x.setDate(x.getDate() + days);
    x.setHours(h, 0, 0, 0);
    return x.getTime();
  };
  const out: { key: 'hour' | 'evening' | 'tomorrow' | 'monday'; at: number }[] = [{ key: 'hour', at: now + 3600_000 }];
  if (d.getHours() < 17) out.push({ key: 'evening', at: at(0, 18) });
  out.push({ key: 'tomorrow', at: at(1, 9) });
  const toMonday = (8 - d.getDay()) % 7 || 7;
  if (toMonday > 1) out.push({ key: 'monday', at: at(toMonday, 9) });
  return out;
}

/**
 * A time near `now`, as « Plus tard » and the trace say it: « 17:50 » today, « demain 9:00 », « lundi 9:00 » within the
 * week, « 2 décembre 9:00 » beyond.
 */
export function fLater(ts: number, now = Date.now()): string {
  const day = (x: number) => {
    const d = new Date(x);
    d.setHours(0, 0, 0, 0);
    return d.getTime();
  };
  const days = Math.round((day(ts) - day(now)) / 86_400_000);
  const time = fTime(ts);
  if (days === 0) return time;
  if (days === 1) return t('inbox.later.tomorrowAt', { time });
  if (days === -1) return t('inbox.later.yesterdayAt', { time });
  if (days > 1 && days < 7)
    return t('inbox.later.dayAt', { day: new Date(ts).toLocaleDateString(intlLocale(), { weekday: 'long' }), time });
  return t('inbox.later.dayAt', { day: fDateShort(ts, now), time });
}

/** The option the answer starts on: the agent's recommendation, else the first (never chosen without the user). */
export const startOption = (a: WindowAsk) => a.options.find((o) => o.id === a.recommend?.option)?.id ?? a.options[0]?.id ?? '';
