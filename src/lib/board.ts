// The board's labels and counts, without state: what its header, columns and cards say.

import { fPct, fTime, fUsd, fWhen, plural } from './format';
import type { Agent, AutopilotPause, BoardAction, BoardSettings, Column, Ticket } from './types';

export const COLUMNS: { id: Column; label: string; color: string; empty: string }[] = [
  { id: 'todo', label: 'À faire', color: 'var(--dim)', empty: "Ajoute un ticket : un agent le prendra dès qu'une place se libère." },
  { id: 'doing', label: 'En cours', color: 'var(--accent)', empty: 'Aucun agent en boucle' },
  { id: 'review', label: 'À tester', color: 'var(--wait)', empty: 'Rien à tester' },
  { id: 'done', label: 'Terminé', color: 'var(--ok)', empty: 'Aucun ticket terminé' },
];

/** A column's tickets in order: "À faire" by priority, "Terminé" newest first, the others by arrival. */
export function columnTickets(tickets: Ticket[], column: Column): Ticket[] {
  const list = tickets.filter((t) => t.column === column);
  if (column === 'todo') return list.sort((a, b) => a.rank - b.rank || a.createdAt - b.createdAt);
  if (column === 'done') return list.sort((a, b) => (b.doneAt ?? 0) - (a.doneAt ?? 0));
  if (column === 'review') return list.sort((a, b) => (a.reviewAt ?? 0) - (b.reviewAt ?? 0));
  return list.sort((a, b) => (a.startedAt ?? 0) - (b.startedAt ?? 0));
}

/** Tickets "En cours" holding a place (a blocked one gives it up). */
export function busy(tickets: Ticket[]): number {
  return tickets.filter((t) => t.column === 'doing' && !t.blocked).length;
}

/** When the agents stopped by the usage limit resume, if one is waiting: nothing starts until then. */
export function quotaUntil(agents: Agent[]): number | null {
  const times = agents.filter((a) => a.resumeAt && !a.archived).map((a) => a.resumeAt!);
  return times.length ? Math.min(...times) : null;
}

export function placesLabel(busyCount: number, max: number, quota: number | null): string {
  if (quota) return `Quota atteint — reprise à ${fTime(quota)}`;
  const free = Math.max(0, max - busyCount);
  return free ? plural(free, 'place libre', 'places libres') : 'Toutes les places sont prises';
}

/** Why no ticket starts, in the header: a window's end is known to the minute, the end of the pause after a limit only about. */
export function pauseLabel(p: AutopilotPause, now: number): string {
  const why =
    p.reason === 'limit'
      ? `limite d'usage atteinte (reprise vers ${fTime(p.until)})`
      : `quota ${p.reason === 'week' ? 'hebdo' : 'de 5 h'} à ${fPct(p.pct ?? 100)} (reprise ${fWhen(p.until, now)})`;
  return `Pilote auto en pause : ${why}`;
}

/** What validating a ticket does, in the header's button. */
export function settingsSummary(s: BoardSettings, target: string): string {
  switch (s.action) {
    case 'merge':
      return `merge ${s.strategy} → ${target}`;
    case 'pr':
      return `PR → ${target}`;
    case 'push':
      return 'push ticket/*';
    default:
      return "laisser en l'état";
  }
}

export const APPROVE_LABEL: Record<BoardAction, string> = {
  merge: 'Valider et merger',
  pr: 'Valider + PR',
  push: 'Valider et pousser',
  keep: 'Valider',
};

/**
 * When a ticket "À faire" starts; `queueIndex` is its place in the queue (`queueIndices`), `waiting` the keys of the
 * tickets it comes after that are not done yet (`waitingFor`). While the board's `issue` holds (its target branch has no
 * commit yet, or is gone), or the autopilot's `pause` (a quota, a usage limit), none starts, whatever its place, even
 * launched by hand. The issue comes first (it needs the user); then the tickets it waits for, which outlast a pause (the
 * header tells that one, and its end), unless it was launched by hand: it starts without them.
 */
export function waitLabel(
  t: Ticket,
  queueIndex: number,
  s: BoardSettings,
  busyCount: number,
  issue: string | null = null,
  pause: AutopilotPause | null = null,
  waiting: string[] = [],
): string {
  if (issue) return 'En attente de la branche cible';
  if (waiting.length && !t.forced) return `⏸ après ${keyList(waiting)}`;
  if (pause) return 'En attente : pilote auto en pause';
  if (t.forced) return 'Lancement demandé…';
  if (!s.autopilot) return 'Pilote auto désactivé';
  return queueIndex === 0 ? "Pris dès qu'une place se libère" : `En attente d'une place (${busyCount}/${s.maxParallel})`;
}

/** "DEM-3", "DEM-3 et DEM-4", "DEM-3, DEM-4 et DEM-6". */
export function keyList(keys: string[]): string {
  return keys.length > 1 ? `${keys.slice(0, -1).join(', ')} et ${keys.at(-1)}` : (keys[0] ?? '');
}

/** The keys of the tickets `t` comes after that are not "Terminé" yet, in its order: one gone (deleted) holds nothing back. */
export function waitingFor(t: Ticket, tickets: Record<string, Ticket>): string[] {
  return t.after.flatMap((id) => {
    const x = tickets[id];
    return x && x.column !== 'done' ? [x.key] : [];
  });
}

/** What "Lancer" asks first on a ticket that waits for others. */
export function launchAnyway(key: string, waiting: string[]): string {
  const done = waiting.length > 1 ? 'terminés' : 'terminé';
  return `${key} attend ${keyList(waiting)}, pas encore ${done}. Le lancer quand même ?`;
}

/**
 * Each ticket's place in the queue of "À faire" (`todo`, in its order), by id: as the autopilot does, one that waits for
 * others (-1) is passed over for the next, unless it was launched by hand.
 */
export function queueIndices(todo: Ticket[], tickets: Record<string, Ticket>): Record<string, number> {
  let next = 0;
  return Object.fromEntries(todo.map((t) => [t.id, !t.forced && waitingFor(t, tickets).length ? -1 : next++]));
}

/**
 * Why `t` cannot come after `dep`: `dep` already waits for it, directly or through others, and neither would ever start
 * by itself. Null when it does not.
 */
export function cycleRefusal(t: Ticket, dep: Ticket, tickets: Record<string, Ticket>): string | null {
  // Every ticket `dep` waits for, once each: a loop already there (a hand-edited file) cannot hang the search.
  const seen = new Set<string>();
  const next = [...dep.after];
  while (next.length) {
    const id = next.pop()!;
    if (id === t.id) return `${dep.key} attend déjà ${t.key} (directement ou non).`;
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
  t: Ticket,
  s: BoardSettings,
  busyCount: number,
  quota: number | null,
  issue: string | null = null,
  pause: AutopilotPause | null = null,
): boolean {
  return !issue && !pause && !s.autopilot && !t.forced && !quota && busyCount < s.maxParallel;
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
  return s.conventional ? `feat: limiter les tentatives de connexion [${key}]` : `${key} Limiter les tentatives de connexion`;
}

/** The tag of a ticket's agent in the sidebar. */
export function ticketTag(t: Ticket | undefined): string | null {
  if (t?.column === 'doing') return `${t.key} · boucle ${t.iteration}/${t.maxLoops}`;
  if (t?.column === 'review') return `${t.key} · à tester`;
  return null;
}

export function criteriaMet(t: Ticket): number {
  return t.criteria.filter((c) => c.ok).length;
}

/** "n boucles · coût" of a finished ticket: every loop, those of the rounds it was sent back for included (a ticket saved before they were counted: its last round's). */
export function doneMeta(t: Ticket): string {
  return `${plural(t.loops || t.iteration, 'boucle', 'boucles')} · ${fUsd(t.cost)}`;
}
