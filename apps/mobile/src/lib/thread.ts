/**
 * The thread of « À toi » (MO-5, direction B, `docs/MOBILE.md` § 3.3 and § 3.6): what waits in every space shown, in
 * the order of the desktop's box (`src/lib/inbox/`): the agents that stopped first, the longest waiting first
 * (« Tes agents se sont arrêtés »), then the requests and the views of the box (« Aujourd'hui », « Plus tôt »); apart,
 * what the squad lead took in hand (« Le chef s'en occupe »), what was put off (« Plus tard ») and what was answered
 * lately (« Répondu récemment », whence one goes back on an answer).
 *
 * A line that changes stays in its place, as on a board of departures, and keeps its dot until it has been seen
 * (`steady`, `unseen`). Pure: the screens give it the spaces, the order it had and what was seen.
 */

import { isClosed, isOpen, isToday, closedAt, RECENT_MS, startOfDay } from '$shared/lib/inbox/rules';
import { waitsForUser, type InboxItem } from '$shared/lib/inbox/views';
import type { GlanceAgent, GlanceAsk, GlanceProject } from '$shared/lib/spaces.svelte';
import type { PendingRequest, WindowAsk } from '$shared/lib/types';
import type { PhoneSpace } from './phone-spaces.svelte';

/** What an agent stopped for. */
export type Stop = 'permission' | 'question' | 'plan' | 'delete';

/** Where a line comes from: its space, by what names it. */
export type SpaceRef = Pick<PhoneSpace, 'id' | 'name' | 'here'>;

interface Base {
  /** `<space>:<what>`: unique over the spaces. */
  key: string;
  space: SpaceRef;
  project: GlanceProject | null;
  title: string;
  /** Since when it waits (ms). */
  since: number;
  /** Its state, as its dot reads it: a new mark is a line that changed. */
  mark: string;
}

/** An agent that stopped: its request (from its view of the box, else from the glance, maybe cut). */
export interface StoppedLine extends Base {
  kind: 'stopped';
  agentId: string;
  agentName: string;
  stop: Stop;
  request: PendingRequest | null;
  /** Its view in the box, through which the answer goes when there is one. */
  view: InboxItem | null;
}

/** A request an agent left in the box: whole once the box is read, its title alone from the glance. */
export interface AskLine extends Base {
  kind: 'ask';
  ask: WindowAsk | null;
}

/** A view of what waits elsewhere (a ticket to approve, a decision, the docs, a review…). */
export interface ViewLine extends Base {
  kind: 'view';
  view: InboxItem;
}

export type Line = StoppedLine | AskLine | ViewLine;

export interface Thread {
  stopped: Line[];
  today: Line[];
  earlier: Line[];
  /** « Le chef s'en occupe » (ESC-557). */
  claimed: Line[];
  /** « Plus tard ». */
  later: Line[];
  /** « Répondu récemment »: the answers of the last days, and what the squad lead settled. */
  recent: Line[];
}

export type Group = keyof Thread;
export const GROUPS: Group[] = ['stopped', 'today', 'earlier', 'claimed', 'later', 'recent'];
/** The groups whose lines wait for the user: those one answers in turn. */
export const WAITING: Group[] = ['stopped', 'today', 'earlier'];

/** The most answers « Répondu récemment » shows. */
export const RECENT_SHOWN = 12;

/** What an agent's request stops it for. */
export function stopOf(r: Pick<PendingRequest, 'kind' | 'tool'> | null | undefined): Stop {
  if (!r) return 'permission';
  if (r.kind === 'question') return 'question';
  return r.tool === 'ExitPlanMode' ? 'plan' : 'permission';
}

/** What a stopped agent's line says: its question, what the permission does, or the plan. */
export function stoppedTitle(r: PendingRequest | null, planTitle: string): string {
  if (!r) return '';
  if (r.kind === 'question') return r.questions[0]?.question ?? r.arg;
  if (r.tool === 'ExitPlanMode') return planTitle;
  return r.description?.trim() || [r.tool, r.arg.split('\n')[0]].filter(Boolean).join(' : ');
}

const askMark = (a: WindowAsk) => `${a.state}:${a.updatedAt}`;
const viewMark = (v: InboxItem) => `${v.source}:${v.since}:${v.column ?? ''}:${v.request?.id ?? ''}:${v.mark?.kind ?? ''}`;

/** The thread of `spaces` (those the chip shows), at `now`; `planTitle` names a plan's line. */
export function threadOf(spaces: PhoneSpace[], now: number, planTitle: string): Thread {
  const th: Thread = { stopped: [], today: [], earlier: [], claimed: [], later: [], recent: [] };
  for (const s of spaces) {
    const g = s.glance;
    if (!g || g.countsOnly) continue;
    const space: SpaceRef = { id: s.id, name: s.name, here: s.here };
    const projectOf = (id: string, name?: string): GlanceProject | null =>
      g.projects.find((p) => p.id === id) ?? (name ? { id, name, color: 'var(--muted)' } : null);
    const box = s.box?.enabled ? s.box : null;

    // The agents stopped: through their view when the box has one (the request whole), else from the glance.
    const stoppedViews = box?.views.filter((v) => v.group === 'stopped') ?? [];
    const viewed = new Set(stoppedViews.map((v) => v.agentId));
    for (const v of stoppedViews) {
      const agent = g.agents.find((a) => a.id === v.agentId);
      const line: StoppedLine = {
        kind: 'stopped',
        // Keyed by its agent, as from the glance: the line keeps its place and stays open when the box comes.
        key: v.agentId ? `${s.id}:agent:${v.agentId}` : `${s.id}:view:${v.key}`,
        space,
        project: projectOf(v.projectId, v.projectName),
        title: stoppedTitle(v.request, planTitle) || v.title,
        since: v.since,
        mark: `agent:${v.request?.id ?? ''}`,
        agentId: v.agentId ?? '',
        agentName: v.agentName ?? agent?.name ?? '',
        stop: stopOf(v.request) === 'permission' && v.source === 'plan' ? 'plan' : stopOf(v.request),
        request: v.request,
        view: v,
      };
      if (v.mark?.kind === 'claimed') th.claimed.push(line);
      else if (v.mark?.kind === 'settled') th.recent.push(line);
      else th.stopped.push(line);
    }
    for (const a of g.agents.filter((x: GlanceAgent) => x.status === 'waiting' && !viewed.has(x.id))) {
      th.stopped.push({
        kind: 'stopped',
        key: `${s.id}:agent:${a.id}`,
        space,
        project: projectOf(a.projectId),
        title: stoppedTitle(a.request, planTitle) || a.name,
        since: a.lastActivity,
        mark: `agent:${a.request?.id ?? ''}`,
        agentId: a.id,
        agentName: a.name,
        stop: a.request ? stopOf(a.request) : 'delete',
        request: a.request,
        view: null,
      });
    }

    // The requests of the box.
    if (box) {
      for (const a of box.asks) {
        const line: AskLine = {
          kind: 'ask',
          key: `${s.id}:ask:${a.id}`,
          space,
          project: projectOf(a.projectId, a.projectName),
          title: a.title,
          since: a.createdAt,
          mark: askMark(a),
          ask: a,
        };
        if (isOpen(a)) (isToday(a, now) ? th.today : th.earlier).push(line);
        else if (a.state === 'snoozed') th.later.push({ ...line, since: a.snoozedUntil ?? a.createdAt });
        else if (isClosed(a) && now - closedAt(a) <= RECENT_MS) th.recent.push({ ...line, since: closedAt(a) });
      }
      for (const v of box.views.filter((x) => x.group !== 'stopped')) {
        const line: ViewLine = {
          kind: 'view',
          key: `${s.id}:view:${v.key}`,
          space,
          project: projectOf(v.projectId, v.projectName),
          title: v.title,
          since: v.since,
          mark: viewMark(v),
          view: v,
        };
        if (v.mark?.kind === 'claimed') th.claimed.push(line);
        else if (v.mark?.kind === 'settled') th.recent.push({ ...line, since: v.mark.at });
        else if (v.snoozedUntil) th.later.push({ ...line, since: v.snoozedUntil });
        else if (waitsForUser(v)) (v.group === 'today' ? th.today : th.earlier).push(line);
      }
    } else {
      // The box not read (cut, or not yet): the requests the glance names, by their day.
      for (const a of g.asks as GlanceAsk[]) {
        th[a.createdAt >= startOfDay(now) ? 'today' : 'earlier'].push({
          kind: 'ask',
          key: `${s.id}:ask:${a.id}`,
          space,
          project: projectOf(a.projectId),
          title: a.title,
          since: a.createdAt,
          mark: `glance:${a.createdAt}`,
          ask: null,
        });
      }
    }
  }
  // The desktop's order: the stopped, the longest waiting first; a request « aujourd'hui » urgent or back from « Plus
  // tard » before the others, then the oldest first; the views after the requests of their group.
  const asRank = (l: Line): [number, number, number] => {
    if (l.kind === 'ask' && l.ask) return [0, -(Number(l.ask.urgency === 'today') * 2 + Number(l.ask.woke)), l.since];
    return [l.kind === 'view' ? 1 : 0, 0, l.since];
  };
  const byRank = (a: Line, b: Line) => {
    const x = asRank(a);
    const y = asRank(b);
    return x[0] - y[0] || x[1] - y[1] || x[2] - y[2];
  };
  th.stopped.sort((a, b) => a.since - b.since);
  th.today.sort(byRank);
  th.earlier.sort(byRank);
  th.claimed.sort((a, b) => a.since - b.since);
  th.later.sort((a, b) => a.since - b.since);
  th.recent = th.recent.sort((a, b) => b.since - a.since).slice(0, RECENT_SHOWN);
  return th;
}

/**
 * The order of a group after a reading (`next`, as it comes), the one it had before (`before`) kept: a line still
 * there keeps its place among the others; a new one comes where `next` puts it, after the line it follows there.
 */
export function steady(before: string[], next: string[]): string[] {
  const now = new Set(next);
  const out = before.filter((k) => now.has(k));
  const placed = new Set(out);
  for (let i = 0; i < next.length; i++) {
    const k = next[i];
    if (placed.has(k)) continue;
    let at = 0;
    for (let j = i - 1; j >= 0; j--) {
      const p = out.indexOf(next[j]);
      if (p >= 0) {
        at = p + 1;
        break;
      }
    }
    out.splice(at, 0, k);
    placed.add(k);
  }
  return out;
}

/** The lines of `lines` in the order `order` gives. */
export function inOrder<L extends { key: string }>(lines: L[], order: string[]): L[] {
  const at = new Map(order.map((k, i) => [k, i]));
  return [...lines].sort((a, b) => (at.get(a.key) ?? 1e9) - (at.get(b.key) ?? 1e9));
}

/**
 * Whether a line keeps its dot: not seen in the state it is in. A request of the box says it itself (the core keeps
 * whether the user saw it, on any of their screens, and when it came back from « Plus tard »).
 */
export function unseen(l: Line, seen: Record<string, string>): boolean {
  if (l.kind === 'ask' && l.ask) return !l.ask.seen || l.ask.woke;
  return seen[l.key] !== l.mark;
}

/** The lines one answers in turn: the stopped, then today, then earlier. */
export const waitingLines = (th: Thread): Line[] => WAITING.flatMap((g) => th[g]);

/** After a line was settled: the line that came after it among those that wait, else the one before, else none. */
export function nextAfter(order: string[], settled: string, left: string[]): string | null {
  const i = order.indexOf(settled);
  const rest = new Set(left);
  for (let j = i + 1; j < order.length; j++) if (rest.has(order[j]) && order[j] !== settled) return order[j];
  for (let j = i - 1; j >= 0; j--) if (rest.has(order[j]) && order[j] !== settled) return order[j];
  return left.find((k) => k !== settled) ?? null;
}
