import { describe, expect, it } from 'vitest';
import { DONE_KEPT_MS, FRESH_MS, orbitTasks, place, toOrb } from './orbit';
import { agent, project, ticket } from '../test/ipc';
import type { OverseerNews } from './types';

const NOW = 1_800_000_000_000;
const P1 = project({ id: 'p1', name: 'portail', color: 'oklch(0.7 0.15 250)' });
const P2 = project({ id: 'p2', name: 'site', color: 'oklch(0.7 0.15 330)' });
const SECRET = project({ id: 'p3', name: 'secret', color: 'red', private: true });

const news = (ticketId: string, at: number, kind: OverseerNews['kind'] = 'review'): OverseerNews => ({
  id: `n-${ticketId}`,
  at,
  kind,
  ticketId,
  ticket: ticketId.toUpperCase(),
  title: '',
  projectId: 'p1',
  project: 'portail',
});

describe('the orbit’s tasks (ESC-468)', () => {
  it('takes every project’s tickets and lone agents by their state, never a private project’s', () => {
    const tasks = orbitTasks(
      [P1, P2, SECRET],
      [
        ticket({
          id: 'a',
          projectId: 'p1',
          key: 'DEM-41',
          column: 'doing',
          startedAt: NOW - 10,
          criteria: [
            { text: 'x', ok: true, note: '' },
            { text: 'y', ok: false, note: '' },
            { text: 'z', ok: false, note: '' },
            { text: 'w', ok: false, note: '' },
          ],
        }),
        ticket({ id: 'b', projectId: 'p2', key: 'WS-12', column: 'review', reviewAt: NOW - 20 }),
        ticket({ id: 'c', projectId: 'p1', key: 'DEM-38', column: 'done', doneAt: NOW - 3600_000 }),
        ticket({ id: 'd', projectId: 'p1', key: 'DEM-1', column: 'done', doneAt: NOW - DONE_KEPT_MS - 1 }),
        ticket({ id: 'e', projectId: 'p1', key: 'DEM-50', column: 'todo' }),
        ticket({ id: 'f', projectId: 'p3', key: 'SEC-1', column: 'doing' }),
        ticket({ id: 'g', projectId: 'p2', key: 'WS-13', column: 'doing', agentId: 'asker' }),
        ticket({ id: 'h', projectId: 'p2', key: 'WS-14', column: 'doing', blocked: 'tests en échec' }),
      ],
      [
        agent({ id: 'asker', projectId: 'p2', ticketId: 'g', status: 'waiting', pending: ['q1'] }),
        agent({ id: 'free', projectId: 'p1', name: 'Revue Codex', ticketId: null, status: 'running', lastActivity: NOW - 5 }),
        agent({ id: 'rest', projectId: 'p1', ticketId: null, status: 'idle' }),
        agent({ id: 'hidden', projectId: 'p3', ticketId: null, status: 'running' }),
      ],
      [],
      NOW,
    );
    const by = Object.fromEntries(tasks.map((x) => [x.key ?? x.title, x]));
    expect(Object.keys(by).sort()).toEqual(['DEM-38', 'DEM-41', 'Revue Codex', 'WS-12', 'WS-13', 'WS-14']);
    expect(by['DEM-41']).toMatchObject({ state: 'working', progress: 0.25, color: P1.color });
    expect(by['WS-12']).toMatchObject({ state: 'waiting', why: 'review', color: P2.color });
    expect(by['WS-13']).toMatchObject({ state: 'waiting', why: 'answer' });
    expect(by['WS-14']).toMatchObject({ state: 'waiting', why: 'blocked' });
    expect(by['DEM-38']).toMatchObject({ state: 'done', progress: null });
    expect(by['Revue Codex']).toMatchObject({ state: 'working', agentId: 'free', ticketId: null });
  });

  it('marks a task finished just now, as the Overseer was told it, to melt toward the orb', () => {
    const tickets = [ticket({ id: 'r', column: 'review', key: 'DEM-31' }), ticket({ id: 'o', column: 'review', key: 'DEM-32' })];
    const tasks = orbitTasks([P1], tickets, [], [news('r', NOW - 1000), news('o', NOW - FRESH_MS - 1)], NOW);
    expect(tasks.find((x) => x.key === 'DEM-31')?.fresh).toBe(true);
    expect(tasks.find((x) => x.key === 'DEM-32')?.fresh).toBe(false);
  });
});

describe('the arcs', () => {
  const tasks = orbitTasks(
    [P1],
    [
      ticket({ id: 'w1', key: 'W-1', column: 'doing', startedAt: NOW - 1 }),
      ticket({ id: 'w2', key: 'W-2', column: 'doing', startedAt: NOW - 2 }),
      ticket({ id: 'w3', key: 'W-3', column: 'doing', startedAt: NOW - 3 }),
      ticket({ id: 'q1', key: 'Q-1', column: 'review', reviewAt: NOW - 1 }),
      ticket({ id: 'q2', key: 'Q-2', column: 'pr', reviewAt: NOW - 2 }),
      ticket({ id: 'd1', key: 'D-1', column: 'done', doneAt: NOW - 1000 }),
      ticket({ id: 'd2', key: 'D-2', column: 'done', doneAt: NOW - 9000 }),
    ],
    [],
    [],
    NOW,
  );
  const placed = place(tasks);
  const at = (key: string) => placed.find((x) => x.key === key)!;
  const dist = (k: string) => Math.hypot(at(k).x - 50, at(k).y - 50);

  it('puts what is under way at the top, what waits on the right and closer, what is done at the bottom and left', () => {
    for (const k of ['W-1', 'W-2', 'W-3']) expect(at(k).y).toBeLessThan(30);
    for (const k of ['Q-1', 'Q-2']) {
      expect(at(k).x).toBeGreaterThan(70);
      expect(dist(k)).toBeLessThan(Math.min(dist('W-1'), dist('D-1')));
    }
    for (const k of ['D-1', 'D-2']) expect(at(k).y).toBeGreaterThan(45);
    expect(at('D-2').x).toBeLessThan(at('D-1').x);
  });

  it('sends the older tasks done farther and fainter, and keeps a few of each', () => {
    expect(dist('D-2')).toBeGreaterThan(dist('D-1'));
    expect(at('D-2').fade).toBeGreaterThan(at('D-1').fade);
    const many = orbitTasks(
      [P1],
      Array.from({ length: 12 }, (_, i) => ticket({ id: `x${i}`, key: `X-${i}`, column: 'doing', startedAt: NOW - i })),
      [],
      [],
      NOW,
    );
    const shown = place(many);
    expect(shown).toHaveLength(6);
    expect(shown.map((x) => x.key)).toContain('X-0');
    expect(shown.map((x) => x.key)).not.toContain('X-11');
  });

  it('knows the way from a card to the orb', () => {
    const p = at('D-1');
    expect(toOrb(p)).toEqual({ dx: Math.round((50 - p.x) * 100) / 100, dy: Math.round((50 - p.y) * 100) / 100 });
  });
});
