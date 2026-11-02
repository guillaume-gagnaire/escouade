import { describe, expect, it } from 'vitest';
import { cardEnvironments, envState, FOLLOWED_FOR, lanes, merged, notInProduction, shortSha } from './environments';
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
