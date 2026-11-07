// The board's labels and counts, without state: what its header, columns and cards say.

import { fList, fPct, fTime, fUsd, fWhen } from './format';
import { elsewhere, isTheirs, theirsLine } from './sharing';
import { t } from './i18n';
import type { IconName } from './icons';
import { fSpentUsd, spent, type Spent } from './spend';
import type { Agent, AutopilotPause, BoardAction, BoardSettings, Column, Ticket } from './types';

/**
 * The columns in order (« En revue » only when the board opens pull requests or holds one: `prColumnShown`), with the tone of their ring and how far it is filled (as in the preview: empty to do, half
 * under way, three quarters to test, whole once done); their names and what they say while empty are in the `board`
 * zone, by `id`.
 */
export const COLUMNS: { id: Column; tone: string; fill: number }[] = [
  { id: 'todo', tone: 'var(--code-dim)', fill: 0 },
  { id: 'doing', tone: 'var(--run)', fill: 50 },
  { id: 'review', tone: 'var(--wait)', fill: 75 },
  // « En revue » (ESC-59): its pull request open on its host, the hue of a merge; shown when the board opens them.
  { id: 'pr', tone: 'var(--syn-k)', fill: 90 },
  { id: 'done', tone: 'var(--done)', fill: 100 },
];

/**
 * The branch a ticket gets when it names none: `ticket/dem-3` with the project's « Préfixe des branches des tickets »
 * `prefix` as it is by default, as the backend makes it at its start.
 */
export const ticketBranch = (key: string, prefix: string) => `${prefix}${key.toLowerCase()}`;

/** A column's tickets in order: "À faire" by priority, "Terminé" newest first, the others by arrival. */
export function columnTickets(tickets: Ticket[], column: Column): Ticket[] {
  const list = tickets.filter((ticket) => ticket.column === column);
  if (column === 'todo') return list.sort((a, b) => a.rank - b.rank || a.createdAt - b.createdAt);
  if (column === 'done') return list.sort((a, b) => (b.doneAt ?? 0) - (a.doneAt ?? 0));
  if (column === 'review' || column === 'pr') return list.sort((a, b) => (a.reviewAt ?? 0) - (b.reviewAt ?? 0));
  return list.sort((a, b) => (a.startedAt ?? 0) - (b.startedAt ?? 0));
}

/** Tickets "En cours" holding a place (a blocked one gives it up), and « En revue » while their agent fixes their CI (ESC-60). */
export function busy(tickets: Ticket[]): number {
  // One under way on a teammate's machine (ESC-190) takes no place here.
  return tickets.filter(
    (ticket) => (ticket.column === 'doing' && !ticket.blocked && !elsewhere(ticket)) || (ticket.column === 'pr' && !!ticket.pr?.fixing),
  ).length;
}

/** When the agents stopped by the usage limit resume, if one is waiting: nothing starts until then. */
export function quotaUntil(agents: Agent[]): number | null {
  const times = agents.filter((a) => a.resumeAt && !a.archived).map((a) => a.resumeAt!);
  return times.length ? Math.min(...times) : null;
}

export function placesLabel(busyCount: number, max: number, quota: number | null): string {
  if (quota) return t('boardSettings.places.quota', { time: fTime(quota) });
  const free = Math.max(0, max - busyCount);
  return free ? t('boardSettings.places.free', { count: free }) : t('boardSettings.places.full');
}

/** Why no ticket starts, in the header: a window's end is known to the minute, the end of the pause after a limit only about. */
export function pauseLabel(p: AutopilotPause, now: number): string {
  if (p.reason === 'limit') return t('boardSettings.pause.limit', { time: fTime(p.until) });
  const values = { pct: fPct(p.pct ?? 100), when: fWhen(p.until, now) };
  if (p.reason === 'credits') return t('boardSettings.pause.credits', values);
  if (p.reason === 'budget') return t('boardSettings.pause.budget', values);
  return t('boardSettings.pause.window', { ...values, window: windowName(p.minutes ?? 300) });
}

/** A window of the quota by its length, as the pause says it: « hebdo », « de 5 h », « de 30 jours ». */
export function windowName(minutes: number): string {
  const day = 24 * 60;
  if (minutes === 7 * day) return t('boardSettings.window.week');
  if (minutes % day === 0) return t('boardSettings.window.days', { n: minutes / day });
  return t('boardSettings.window.hours', { n: Math.round(minutes / 60) });
}

/** What validating a ticket does, in the header's button: `prefix` is the project's « Préfixe des branches des tickets ». */
export function settingsSummary(s: BoardSettings, target: string, prefix: string): string {
  switch (s.action) {
    case 'merge':
      return t('boardSettings.summary.merge', { strategy: s.strategy, target });
    case 'pr':
      return t('boardSettings.summary.pr', { target });
    case 'push':
      return t('boardSettings.summary.push', { prefix });
    default:
      return t('boardSettings.summary.keep');
  }
}

/** The button that validates a ticket. Getters: the text is read when it is shown, so that it follows the language. */
export const APPROVE_LABEL: Record<BoardAction, string> = {
  get merge() {
    return t('boardSettings.approve.merge');
  },
  get pr() {
    return t('boardSettings.approve.pr');
  },
  get push() {
    return t('boardSettings.approve.push');
  },
  get keep() {
    return t('boardSettings.approve.keep');
  },
};

/**
 * When a ticket "À faire" starts; `queueIndex` is its place in the queue (`queueIndices`), `waiting` the keys of the
 * tickets it comes after that are not done yet (`waitingFor`). While the board's `issue` holds (its target branch has no
 * commit yet, or is gone), or the autopilot's `pause` (a quota, a usage limit), none starts, whatever its place, even
 * launched by hand. The issue comes first (it needs the user); then the tickets it waits for, which outlast a pause (the
 * header tells that one, and its end), unless it was launched by hand: it starts without them.
 */
export function waitLabel(
  ticket: Ticket,
  queueIndex: number,
  s: BoardSettings,
  busyCount: number,
  issue: string | null = null,
  pause: AutopilotPause | null = null,
  waiting: string[] = [],
  redTarget = false,
): string {
  if (issue) return t('boardSettings.wait.targetBranch');
  if (waiting.length && !ticket.forced) return t('boardSettings.wait.after', { keys: keyList(waiting) });
  if (pause) return t('boardSettings.wait.pause');
  if (ticket.forced) return t('boardSettings.wait.forced');
  // Another one's shared ticket (ESC-190): the autopilot never starts it.
  if (isTheirs(ticket)) return theirsLine(ticket) ?? t('boardSettings.wait.autopilotOff');
  if (!s.autopilot) return t('boardSettings.wait.autopilotOff');
  // « Pause quand la CI de la cible est rouge » (ESC-63): the autopilot's alone, « Lancer » starts it all the same.
  if (redTarget) return t('boardSettings.wait.redTarget');
  return queueIndex === 0 ? t('boardSettings.wait.next') : t('boardSettings.wait.place', { busy: busyCount, max: s.maxParallel });
}

/** "DEM-3", "DEM-3 et DEM-4", "DEM-3, DEM-4 et DEM-6" (and in English "DEM-3, DEM-4, and DEM-6"). */
export function keyList(keys: string[]): string {
  return fList(keys);
}

/** The keys of the tickets `ticket` comes after that are not "Terminé" yet, in its order: one gone (deleted) holds nothing back. */
export function waitingFor(ticket: Ticket, tickets: Record<string, Ticket>): string[] {
  return ticket.after.flatMap((id) => {
    const x = tickets[id];
    return x && x.column !== 'done' ? [x.key] : [];
  });
}

/** What "Lancer" asks first on a ticket that waits for others. */
export function launchAnyway(key: string, waiting: string[]): string {
  return t('boardSettings.launchAnyway', { count: waiting.length, key, keys: keyList(waiting) });
}

/**
 * Each ticket's place in the queue of "À faire" (`todo`, in its order), by id: as the autopilot does, one that waits for
 * others (-1) is passed over for the next, unless it was launched by hand.
 */
export function queueIndices(todo: Ticket[], tickets: Record<string, Ticket>): Record<string, number> {
  let next = 0;
  return Object.fromEntries(todo.map((ticket) => [ticket.id, !ticket.forced && waitingFor(ticket, tickets).length ? -1 : next++]));
}

/**
 * Why `ticket` cannot come after `dep`: `dep` already waits for it, directly or through others, and neither would ever
 * start by itself. Null when it does not.
 */
export function cycleRefusal(ticket: Ticket, dep: Ticket, tickets: Record<string, Ticket>): string | null {
  // Every ticket `dep` waits for, once each: a loop already there (a hand-edited file) cannot hang the search.
  const seen = new Set<string>();
  const next = [...dep.after];
  while (next.length) {
    const id = next.pop()!;
    if (id === ticket.id) return t('boardSettings.cycle', { dep: dep.key, ticket: ticket.key });
    if (seen.has(id)) continue;
    seen.add(id);
    next.push(...(tickets[id]?.after ?? []));
  }
  return null;
}

/**
 * "Lancer": the autopilot is off, a place is free, no agent waits for its quota, the target branch can start a ticket
 * and the autopilot is not paused (a ticket launched meanwhile would wait all the same).
 */
export function canStart(
  ticket: Ticket,
  s: BoardSettings,
  busyCount: number,
  quota: number | null,
  issue: string | null = null,
  pause: AutopilotPause | null = null,
  redTarget = false,
): boolean {
  // Another one's shared ticket (ESC-190) is launched by hand whatever the autopilot: it never starts it. One whose file
  // cannot be read never starts. Held by a red target (ESC-63), the autopilot starts none: « Lancer » does.
  if (ticket.shared?.problem) return false;
  return !issue && !pause && (!s.autopilot || isTheirs(ticket) || redTarget) && !ticket.forced && !quota && busyCount < s.maxParallel;
}

/** As the backend fixes it: the first three letters of the project's name (accents folded), TIC without any. */
export function keyPrefix(name: string): string {
  const letters = name
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .replace(/[^a-z]/gi, '')
    .slice(0, 3)
    .toUpperCase();
  return letters || 'TIC';
}

export function commitPreview(s: BoardSettings, projectName: string): string {
  const key = `${s.prefix || keyPrefix(projectName)}-42`;
  return t(s.conventional ? 'boardSettings.commit.conventional' : 'boardSettings.commit.plain', { key });
}

/** The tag of a ticket's agent in the sidebar. */
export function ticketTag(ticket: Ticket | undefined): string | null {
  if (ticket?.column === 'doing') {
    return t('boardSettings.tag.loop', { key: ticket.key, iteration: ticket.iteration, max: ticket.maxLoops });
  }
  if (ticket?.column === 'review') return t('boardSettings.tag.review', { key: ticket.key });
  return null;
}

export function criteriaMet(ticket: Ticket): number {
  return ticket.criteria.filter((c) => c.ok).length;
}

/**
 * What a ticket's agents used so far: those archived too (a ticket taken up by another agent has several), exact for the
 * finished turns and an estimate while one runs, as the status bar says it. Null until something was used.
 */
export function ticketSpent(ticket: Ticket, agents: Record<string, Agent>): Spent | null {
  const sum: Spent = { tokens: 0, cost: 0, estimated: false };
  // A turn running: something is used already, its cost estimated.
  let running = false;
  for (const a of Object.values(agents)) {
    if (a.ticketId !== ticket.id) continue;
    const s = spent(a);
    sum.tokens += s.tokens;
    sum.cost += s.cost;
    sum.estimated ||= s.estimated;
    if (s.byHarness) sum.byHarness = true;
    running ||= (a.liveTokens ?? 0) > 0;
  }
  return sum.cost > 0 || running ? sum : null;
}

/** "n boucles · coût" of a finished ticket: every loop, those of the rounds it was sent back for included (a ticket saved before they were counted: its last round's). */
export function doneMeta(ticket: Ticket): string {
  return t('boardSettings.loops', { count: ticket.loops || ticket.iteration, cost: fUsd(ticket.cost) });
}

/** A ticket to test, summed up as a finished one: its loops, and what its agents cost so far (`ticketSpent`), if anything. */
export function reviewMeta(ticket: Ticket, used: Spent | null): string {
  const count = ticket.loops || ticket.iteration;
  if (!used) return t('board.card.loops', { count });
  return t('boardSettings.loops', { count, cost: fSpentUsd(used) });
}

/** The glyph the backend writes before an outcome (board.rs), and the icon that stands for it. */
const OUTCOME_ICONS: Record<string, IconName> = { '⤵': 'merge', '⇡': 'push', '◇': 'folder', '∅': 'circle' };

/**
 * A finished ticket's outcome as the backend writes it (« ⤵ Mergé dans main · squash », the same in the comments it
 * publishes): its words, and an icon for its glyph. A pull request, opened or to open, is drawn as one.
 */
export function outcomeShown(outcome: string): { icon: IconName | null; text: string } {
  const at = outcome.indexOf(' ');
  const icon = at > 0 ? OUTCOME_ICONS[outcome.slice(0, at)] : undefined;
  if (!icon) return { icon: null, text: outcome };
  const text = outcome.slice(at + 1);
  return { icon: icon === 'push' && /\bPR\b/.test(text) ? 'pr' : icon, text };
}
