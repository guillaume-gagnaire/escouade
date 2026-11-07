// The deployments (ESC-67): what a done ticket's card says of its environments (« staging ✓ · production ⏳ »), and
// the lanes of the view « Environnements » (direction A, chosen on preview): the tickets merged and deployed nowhere
// yet, then one lane per environment, production last, each ticket in the furthest one it reached.

import type { Environment, ProjectEnvironments, Ticket, TicketEnv } from './types';

/** As long as the backend follows a done ticket in its environments (`deploys::FOLLOWED_FOR`): 30 days. */
export const FOLLOWED_FOR = 30 * 24 * 3600 * 1000;

/** A commit's short id. */
export const shortSha = (sha: string | undefined | null) => (sha ?? '').slice(0, 7);

/** A done ticket merged lately: the view places it. */
export function merged(t: Ticket, now: number): boolean {
  return t.column === 'done' && !!t.outcomeCommit && !!t.doneAt && now - t.doneAt < FOLLOWED_FOR;
}

/** The environments its card names; none while none is known. */
export const cardEnvironments = (t: Ticket): TicketEnv[] => (t.column === 'done' ? (t.environments ?? []) : []);

export interface Lane {
  env: Environment;
  tickets: Ticket[];
}

export interface Lanes {
  /** Merged, deployed nowhere yet. */
  waiting: Ticket[];
  lanes: Lane[];
}

/**
 * The lanes of the view: each merged ticket in the furthest environment it is deployed in (in the order of the
 * environments, production last), else among those that wait; the newest done first.
 */
export function lanes(tickets: Ticket[], envs: Environment[], now: number): Lanes {
  const out: Lanes = { waiting: [], lanes: envs.map((env) => ({ env, tickets: [] })) };
  const done = tickets.filter((t) => merged(t, now)).sort((a, b) => (b.doneAt ?? 0) - (a.doneAt ?? 0));
  for (const t of done) {
    const deployed = new Set((t.environments ?? []).filter((e) => e.deployed).map((e) => e.name));
    let at = -1;
    envs.forEach((e, i) => {
      if (deployed.has(e.name)) at = i;
    });
    if (at < 0) out.waiting.push(t);
    else out.lanes[at].tickets.push(t);
  }
  return out;
}

/** The merged tickets not in production yet: none when the host has no environment named so. */
export function notInProduction(tickets: Ticket[], envs: Environment[], now: number): Ticket[] {
  const prod = envs.filter((e) => e.production).map((e) => e.name);
  if (!prod.length) return [];
  return tickets.filter((t) => merged(t, now) && !(t.environments ?? []).some((e) => prod.includes(e.name) && e.deployed));
}

/** What the view says of a project before its environments are drawn: being read, the host has none, none yet. */
export function envState(p: ProjectEnvironments | undefined): 'loading' | 'unsupported' | 'error' | 'empty' | 'ready' {
  if (!p) return 'loading';
  if (p.unsupported) return 'unsupported';
  if (!p.environments.length) return p.error ? 'error' : p.readAt ? 'empty' : 'loading';
  return 'ready';
}
