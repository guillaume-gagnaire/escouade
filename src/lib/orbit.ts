// The Overseer's orbit (ESC-468, docs/OVERSEER.md « Son écran », design/overseer/ecran-discussion.png): the tasks of every
// project but the private ones, in arcs around its orb by their state: under way at the top, what waits for the user on
// the right (amber, and closer), what is done going away at the bottom and on the left. Each card in its project's
// colour; a task just finished melts toward the orb while it tells it. Pure: the screen places what this answers.

import type { Agent, OverseerNews, Project, Ticket } from './types';

export type OrbitState = 'working' | 'waiting' | 'done';

/** Why a task waits for the user. */
export type WaitWhy = 'answer' | 'review' | 'pr' | 'blocked';

export interface OrbitTask {
  /** `t-<ticket id>` or `a-<agent id>`. */
  id: string;
  ticketId: string | null;
  agentId: string | null;
  /** A ticket's key; none for an agent. */
  key: string | null;
  title: string;
  projectId: string;
  /** Its project's colour. */
  color: string;
  state: OrbitState;
  why: WaitWhy | null;
  /** The criteria met, 0 to 1, of a ticket under way that has some. */
  progress: number | null;
  /** When it came to its state (epoch ms): the newest first in its arc. */
  at: number;
  /** Just finished, told by the Overseer now: it melts toward the orb. */
  fresh: boolean;
}

/** How many of each state the orbit shows, the newest. */
export const ARC_MAX: Record<OrbitState, number> = { working: 6, waiting: 4, done: 4 };
/** How long a finished task is done « à l'instant », melting toward the orb (ms). */
export const FRESH_MS = 8_000;
/** How long a ticket done stays in the orbit (ms). */
export const DONE_KEPT_MS = 24 * 3600_000;

/**
 * The tasks of the orbit: the tickets under way, waiting (« À tester », a pull request, blocked, or their agent asking)
 * and done today; the agents without a ticket at work or asking. Never a private project's, nor the Overseer's own.
 */
export function orbitTasks(projects: Project[], tickets: Ticket[], agents: Agent[], news: OverseerNews[], now: number): OrbitTask[] {
  const open = new Map(projects.filter((p) => !p.private).map((p) => [p.id, p]));
  const byId = new Map(agents.map((a) => [a.id, a]));
  const toldAt = new Map(news.map((n) => [n.ticketId, n.at]));
  const out: OrbitTask[] = [];
  for (const t of tickets) {
    const p = open.get(t.projectId);
    if (!p) continue;
    const agent = t.agentId ? byId.get(t.agentId) : undefined;
    const asks = !!agent?.pending.length;
    const met = t.criteria.length ? t.criteria.filter((c) => c.ok).length / t.criteria.length : null;
    const base = { id: `t-${t.id}`, ticketId: t.id, agentId: t.agentId, key: t.key, title: t.title, projectId: p.id, color: p.color };
    let state: OrbitState;
    let why: WaitWhy | null = null;
    let at: number;
    if (t.column === 'done') {
      at = t.doneAt ?? 0;
      if (now - at > DONE_KEPT_MS) continue;
      state = 'done';
    } else if (t.blocked || t.conflict) {
      state = 'waiting';
      why = 'blocked';
      at = t.startedAt ?? t.createdAt;
    } else if (asks) {
      state = 'waiting';
      why = 'answer';
      at = t.startedAt ?? t.createdAt;
    } else if (t.column === 'review' || t.column === 'pr') {
      state = 'waiting';
      why = t.column === 'pr' ? 'pr' : 'review';
      at = t.reviewAt ?? t.startedAt ?? t.createdAt;
    } else if (t.column === 'doing') {
      state = 'working';
      at = t.startedAt ?? t.createdAt;
    } else continue;
    // Finished just now: « À tester » or « Terminé » reached, as the Overseer was told.
    const told = toldAt.get(t.id);
    const fresh = (state === 'done' || why === 'review') && told !== undefined && now - told < FRESH_MS;
    out.push({ ...base, state, why, progress: state === 'working' ? met : null, at, fresh });
  }
  for (const a of agents) {
    if (a.ticketId || a.archived) continue;
    const p = open.get(a.projectId);
    if (!p) continue;
    const asks = a.pending.length > 0;
    if (!asks && a.status !== 'running') continue;
    out.push({
      id: `a-${a.id}`,
      ticketId: null,
      agentId: a.id,
      key: null,
      title: a.name,
      projectId: p.id,
      color: p.color,
      state: asks ? 'waiting' : 'working',
      why: asks ? 'answer' : null,
      progress: null,
      at: a.lastActivity || a.createdAt,
      fresh: false,
    });
  }
  return out;
}

export interface Placed extends OrbitTask {
  /** Where its card's centre goes, in % of the orbit's box. */
  x: number;
  y: number;
  /** How far it went from the orb: the older a task done, the farther and the fainter (0 to 1). */
  fade: number;
}

/** The arcs, in degrees (0 on the right, 90 at the bottom), and their distance to the orb (1: the orbit's ring). */
const ARCS: Record<OrbitState, { from: number; to: number; r: number }> = {
  // Under way: across the top.
  working: { from: -145, to: -35, r: 0.92 },
  // What waits for the user: on the right, closer.
  waiting: { from: -12, to: 42, r: 0.74 },
  // What is done: from the bottom to the left, going away.
  done: { from: 92, to: 168, r: 0.96 },
};

/** Its box's half sizes the arcs fill, in % (the cards are wider than tall). */
const RX = 40;
const RY = 41;

/** The tasks placed on their arcs: in each, the newest first, spread evenly; a fresh one at the bottom, closest. */
export function place(tasks: OrbitTask[]): Placed[] {
  const out: Placed[] = [];
  for (const state of ['working', 'waiting', 'done'] as OrbitState[]) {
    const arc = ARCS[state];
    const mine = tasks
      .filter((t) => t.state === state)
      .sort((a, b) => Number(b.fresh) - Number(a.fresh) || b.at - a.at)
      .slice(0, ARC_MAX[state]);
    mine.forEach((t, i) => {
      const n = mine.length;
      // One alone sits in the middle of its arc; several are spread from one end to the other.
      const k = n === 1 ? 0.5 : i / (n - 1);
      const deg = arc.from + k * (arc.to - arc.from);
      // Done: the older, the farther and the fainter.
      const r = state === 'done' ? arc.r + (n > 1 ? k * 0.1 : 0) : arc.r;
      const rad = (deg * Math.PI) / 180;
      out.push({
        ...t,
        x: round(50 + Math.cos(rad) * r * RX),
        y: round(50 + Math.sin(rad) * r * RY),
        fade: state === 'done' ? round(n > 1 ? k : 0) : 0,
      });
    });
  }
  return out;
}

const round = (v: number) => Math.round(v * 100) / 100;

/** The vector from a card to the orb, in % of the box: where a fresh one melts. */
export function toOrb(p: Placed): { dx: number; dy: number } {
  return { dx: round(50 - p.x), dy: round(50 - p.y) };
}
