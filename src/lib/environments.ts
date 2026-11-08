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

/**
 * Which environments the view shows, and in which order (ESC-390), kept per project on this machine: those the user
 * hid, those they showed though nothing ever ran there, and the order they gave (the ones not in it after, as the
 * backend lists them: by name, production last).
 */
export interface EnvPrefs {
  order: string[];
  hidden: string[];
  shown: string[];
}

export const NO_PREFS: EnvPrefs = { order: [], hidden: [], shown: [] };

/** Shown unless the user says otherwise: an environment where a deployment once succeeded (not `github-pages` never built). */
export const shownByDefault = (e: Environment) => !!e.running;

export function isShown(e: Environment, p: EnvPrefs): boolean {
  if (p.hidden.includes(e.name)) return false;
  return p.shown.includes(e.name) || shownByDefault(e);
}

/** Why an environment is not shown: the user hid it, or nothing ever ran there. None when it is shown. */
export function hiddenWhy(e: Environment, p: EnvPrefs): 'byYou' | 'neverRan' | null {
  if (isShown(e, p)) return null;
  return p.hidden.includes(e.name) ? 'byYou' : 'neverRan';
}

/** The environments in the user's order, the others after them in the backend's. */
export function ordered(envs: Environment[], p: EnvPrefs): Environment[] {
  const rank = (e: Environment, i: number) => {
    const at = p.order.indexOf(e.name);
    return at < 0 ? p.order.length + i : at;
  };
  return envs
    .map((e, i) => ({ e, r: rank(e, i) }))
    .sort((a, b) => a.r - b.r)
    .map((x) => x.e);
}

/** All the environments in order, those shown, and those left out because nothing ever ran there (the view says it). */
export function arrange(envs: Environment[], p: EnvPrefs) {
  const all = ordered(envs, p);
  return {
    all,
    shown: all.filter((e) => isShown(e, p)),
    neverRan: all.filter((e) => hiddenWhy(e, p) === 'neverRan'),
  };
}

/** The choices with `e` shown or not; a choice back to the default is forgotten. */
export function setShown(p: EnvPrefs, e: Environment, on: boolean): EnvPrefs {
  const hidden = p.hidden.filter((n) => n !== e.name);
  const shown = p.shown.filter((n) => n !== e.name);
  if (on !== shownByDefault(e)) (on ? shown : hidden).push(e.name);
  return { ...p, hidden, shown };
}

/** The order with `name` moved before `before` (null: last), all the environments named so that it holds. */
export function moveBefore(p: EnvPrefs, all: Environment[], name: string, before: string | null): EnvPrefs {
  const names = all.map((e) => e.name).filter((n) => n !== name);
  const at = before === null ? names.length : names.indexOf(before);
  names.splice(at < 0 ? names.length : at, 0, name);
  return { ...p, order: names };
}

/** The order with `name` one place up (-1) or down (1). */
export function moveBy(p: EnvPrefs, all: Environment[], name: string, delta: -1 | 1): EnvPrefs {
  const names = all.map((e) => e.name);
  const i = names.indexOf(name);
  const j = i + delta;
  if (i < 0 || j < 0 || j >= names.length) return p;
  [names[i], names[j]] = [names[j], names[i]];
  return { ...p, order: names };
}

/** The choices kept for a project, whatever was written (an old or a broken value gives the defaults). */
export function parsePrefs(raw: string | null): EnvPrefs {
  try {
    const v = JSON.parse(raw ?? '');
    const list = (x: unknown) => (Array.isArray(x) ? x.filter((n): n is string => typeof n === 'string') : []);
    return { order: list(v?.order), hidden: list(v?.hidden), shown: list(v?.shown) };
  } catch {
    return { ...NO_PREFS };
  }
}
