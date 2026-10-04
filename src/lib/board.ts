// The board's labels and counts, without state: what its header, columns and cards say.

import { fTime, fUsd, plural } from './format';
import type { Agent, BoardAction, BoardSettings, Column, Ticket } from './types';

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
 * When a ticket "À faire" starts; `queueIndex` is its place in the column. While the board's `issue` holds (its target
 * branch has no commit yet, or is gone), none starts, whatever its place.
 */
export function waitLabel(t: Ticket, queueIndex: number, s: BoardSettings, busyCount: number, issue: string | null = null): string {
  if (issue) return 'En attente de la branche cible';
  if (t.forced) return 'Lancement demandé…';
  if (!s.autopilot) return 'Pilote auto désactivé';
  return queueIndex === 0 ? "Pris dès qu'une place se libère" : `En attente d'une place (${busyCount}/${s.maxParallel})`;
}

/** "Lancer": the autopilot is off, a place is free, no agent waits for its quota and the target branch can start a ticket. */
export function canStart(t: Ticket, s: BoardSettings, busyCount: number, quota: number | null, issue: string | null = null): boolean {
  return !issue && !s.autopilot && !t.forced && !quota && busyCount < s.maxParallel;
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

export function doneMeta(t: Ticket): string {
  return `${plural(t.iteration, 'boucle', 'boucles')} · ${fUsd(t.cost)}`;
}
