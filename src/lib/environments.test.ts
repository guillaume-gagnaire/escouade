import { describe, expect, it } from 'vitest';
import {
  arrange,
  cardEnvironments,
  envState,
  FOLLOWED_FOR,
  hiddenWhy,
  isShown,
  lanes,
  merged,
  moveBefore,
  moveBy,
  NO_PREFS,
  notInProduction,
  ordered,
  parsePrefs,
  setShown,
  shortSha,
} from './environments';
import type { Environment } from './types';
import { ticket } from '../test/ipc';

const NOW = 1_800_000_000_000;
const env = (name: string, production = false): Environment => ({ name, production, failed: false });
const done = (key: string, deployed: string[], doneAt = NOW - 1000) =>
  ticket({
    id: key,
    key,
    column: 'done',
    outcomeCommit: 'abc',
    doneAt,
    environments: ['staging', 'production'].map((name) => ({ name, deployed: deployed.includes(name) })),
  });

describe('environments (ESC-67)', () => {
  it('places each merged ticket in the furthest environment it reached, the newest first', () => {
    const envs = [env('staging'), env('production', true)];
    const old = done('A', ['staging', 'production'], NOW - 5000);
    const l = lanes([old, done('B', ['staging']), done('C', []), done('D', ['staging', 'production'])], envs, NOW);
    expect(l.waiting.map((t) => t.key)).toEqual(['C']);
    expect(l.lanes.map((x) => [x.env.name, x.tickets.map((t) => t.key)])).toEqual([
      ['staging', ['B']],
      ['production', ['D', 'A']],
    ]);
  });

  it('counts the merged tickets not in production yet, none without production', () => {
    const tickets = [done('A', ['staging']), done('B', []), done('C', ['staging', 'production'])];
    expect(notInProduction(tickets, [env('staging'), env('production', true)], NOW).map((t) => t.key)).toEqual(['A', 'B']);
    expect(notInProduction(tickets, [env('staging')], NOW)).toEqual([]);
  });

  it('follows a done ticket merged for thirty days', () => {
    expect(merged(done('A', []), NOW)).toBe(true);
    expect(merged(done('A', [], NOW - FOLLOWED_FOR - 1), NOW)).toBe(false);
    expect(merged({ ...done('A', []), outcomeCommit: null }, NOW)).toBe(false);
    expect(merged({ ...done('A', []), column: 'pr' }, NOW)).toBe(false);
  });

  it('names on a card the environments of a done ticket only', () => {
    expect(cardEnvironments(done('A', ['staging'])).map((e) => e.deployed)).toEqual([true, false]);
    expect(cardEnvironments({ ...done('A', []), column: 'review' })).toEqual([]);
    expect(cardEnvironments(ticket({ column: 'done' }))).toEqual([]);
  });

  it('tells what the view says before drawing lanes', () => {
    const base = { projectId: 'p1', host: 'GitHub', unsupported: false, environments: [] };
    expect(envState(undefined)).toBe('loading');
    expect(envState(base)).toBe('loading');
    expect(envState({ ...base, readAt: 1 })).toBe('empty');
    expect(envState({ ...base, error: 'x' })).toBe('error');
    expect(envState({ ...base, unsupported: true })).toBe('unsupported');
    expect(envState({ ...base, environments: [env('staging')] })).toBe('ready');
    expect(shortSha('0123456789')).toBe('0123456');
  });
});

describe('the environments shown (ESC-390)', () => {
  const ran = (name: string, production = false): Environment => ({
    ...env(name, production),
    running: { environment: name, state: 'success', url: null },
  });
  const ENVS = [env('github-pages'), ran('npm'), ran('staging'), ran('production', true)];

  it('shows by default those where a deployment succeeded, and says the others never ran', () => {
    const a = arrange(ENVS, NO_PREFS);
    expect(a.shown.map((e) => e.name)).toEqual(['npm', 'staging', 'production']);
    expect(a.neverRan.map((e) => e.name)).toEqual(['github-pages']);
    expect(hiddenWhy(ENVS[0], NO_PREFS)).toBe('neverRan');
    expect(hiddenWhy(ENVS[1], NO_PREFS)).toBeNull();
  });

  it('keeps only the choices that differ from the default', () => {
    let p = setShown(NO_PREFS, ENVS[1], false);
    expect(p).toEqual({ order: [], hidden: ['npm'], shown: [] });
    expect(hiddenWhy(ENVS[1], p)).toBe('byYou');
    expect(arrange(ENVS, p).neverRan.map((e) => e.name)).toEqual(['github-pages']);
    p = setShown(p, ENVS[0], true);
    expect(p).toEqual({ order: [], hidden: ['npm'], shown: ['github-pages'] });
    expect(isShown(ENVS[0], p)).toBe(true);
    p = setShown(setShown(p, ENVS[1], true), ENVS[0], false);
    expect(p).toEqual(NO_PREFS);
  });

  it('orders by the user, the environments they never placed after in the backend’s order', () => {
    const p = moveBy(NO_PREFS, ENVS, 'npm', 1);
    expect(p.order).toEqual(['github-pages', 'staging', 'npm', 'production']);
    expect(moveBy(p, ENVS, 'github-pages', -1)).toBe(p);
    expect(moveBefore(NO_PREFS, ENVS, 'npm', null).order).toEqual(['github-pages', 'staging', 'production', 'npm']);
    expect(moveBefore(NO_PREFS, ENVS, 'production', 'staging').order).toEqual(['github-pages', 'npm', 'production', 'staging']);
    // An environment new since the order was given comes after the ones placed.
    expect(ordered([...ENVS, ran('qa')], { ...NO_PREFS, order: ['production', 'npm'] }).map((e) => e.name)).toEqual([
      'production',
      'npm',
      'github-pages',
      'staging',
      'qa',
    ]);
  });

  it('reads back what was kept, whatever was written', () => {
    expect(parsePrefs(JSON.stringify({ order: ['a'], hidden: ['b', 3], shown: 'x' }))).toEqual({ order: ['a'], hidden: ['b'], shown: [] });
    expect(parsePrefs('{oops')).toEqual(NO_PREFS);
    expect(parsePrefs(null)).toEqual(NO_PREFS);
  });
});
