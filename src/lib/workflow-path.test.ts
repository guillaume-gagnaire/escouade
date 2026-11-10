// ESC-406: from an entry, where a workflow may go and where an agent really goes. The calculation behind the diagram:
// its entries, the one an agent took, what it reaches, the agent's path in its order, and how the layout draws them.

import { describe, expect, it } from 'vitest';
import { gsd, paginationUsers, passage, ralph, track } from '../test/workflows';
import type { Workflow, WorkflowTrack } from './types';
import { diagramOf, entriesOf, entryOf, layout, pathOf, progressOf, reachOf, scopeTo, type Measure } from './workflow-diagram';

/** Every character 7 px wide: enough to lay out without a canvas. */
const measure: Measure = (text) => text.length * 7;

/** « Livraison »: a choice after the tests, a loop of correction, a way back from the review, a door into the fix. */
function ship(): Workflow {
  const step = (name: string, order: number) => ({
    name,
    command: `/${name}`,
    order,
    description: '',
    path: '',
    agents: [],
    reads: [],
    writes: [],
  });
  const go = (from: string, to: string, condition: string | null = null) => ({ from, to, back: false, condition, max: null });
  return {
    id: 'ship',
    name: 'Livraison',
    kind: 'cycle',
    harness: null,
    repo: '',
    doors: [
      { name: 'hotfix', command: '/hotfix', kind: 'command', scope: 'project', path: '', description: '', role: 'entry', enters: ['fix'] },
    ],
    steps: [step('build', 0), step('test', 1), step('fix', 2), step('review', 3), step('merge', 4)],
    transitions: [
      go('build', 'test'),
      go('test', 'fix', 'rouges'),
      go('test', 'review', 'verts'),
      go('review', 'merge', 'approuvée'),
      { from: 'fix', to: 'test', back: true, condition: 'à retester', max: 3 },
      { from: 'review', to: 'build', back: true, condition: 'changements demandés', max: null },
    ],
    stateFiles: [],
    exit: null,
    promptLoop: null,
  };
}

/** An agent that came in by Plan: a round of its loop, on to Verify, back to Plan for gaps, executing again. */
function enteredByPlan(): WorkflowTrack {
  const passages = [
    passage(1, 'gsd:plan-phase', 0, 4, {
      door: '/gsd:plan-phase 3',
      outcome: 'back',
      reason: 'plan refusé : la tâche 4 est sans test',
      backTo: 'gsd:plan-phase',
    }),
    passage(2, 'gsd:plan-phase', 4, 6, { pass: 2 }),
    passage(3, 'gsd:execute-phase', 10, 21),
    passage(4, 'gsd:verify-work', 31, 5, { outcome: 'back', reason: '2 tests rouges', backTo: 'gsd:plan-phase' }),
    passage(5, 'gsd:plan-phase', 36, 3, { pass: 3 }),
    passage(6, 'gsd:execute-phase', 39, 8, { pass: 2, outcome: 'running' }),
  ];
  return track('a1', { step: 'gsd:execute-phase', passages });
}

describe('the entries of a workflow', () => {
  it('lists every way in, not three: the entry doors into the step they enter', () => {
    const w = gsd();
    expect(entriesOf(w, diagramOf(w))).toEqual([
      { door: '/gsd:discuss-phase', to: 'gsd:discuss-phase' },
      { door: '/gsd:autonomous', to: 'gsd:discuss-phase' },
      { door: '/gsd:progress', to: 'gsd:discuss-phase' },
      { door: '/gsd:next', to: 'gsd:discuss-phase' },
    ]);
  });

  it('puts the first step first when no door leads to it, and alone when the workflow has no door', () => {
    const w = ship();
    expect(entriesOf(w, diagramOf(w))).toEqual([
      { door: '/build', to: 'build' },
      { door: '/hotfix', to: 'fix' },
    ]);
    const bare = { ...w, doors: [] };
    expect(entriesOf(bare, diagramOf(bare))).toEqual([{ door: '/build', to: 'build' }]);
  });

  it('takes the entries of a loop of one prompt: its prompt and what runs it', () => {
    const w = ralph();
    const m = diagramOf(w);
    expect(entriesOf(w, m).length).toBeGreaterThan(0);
    expect(entriesOf(w, m)).toEqual(m.entries);
  });
});

describe('the entry an agent took', () => {
  it('is the door of its first passage, without its arguments', () => {
    const w = gsd();
    const m = diagramOf(w);
    expect(entryOf(w, m, paginationUsers())).toEqual({ door: '/gsd:discuss-phase', to: 'gsd:discuss-phase' });
    expect(entryOf(w, m, enteredByPlan())).toEqual({ door: '/gsd:plan-phase', to: 'gsd:plan-phase' });
  });

  it('is the command of its first step when no door was typed', () => {
    const w = gsd();
    const tr = track('a1', { step: 'gsd:execute-phase', passages: [passage(1, 'gsd:execute-phase', 0, 3, { outcome: 'running' })] });
    expect(entryOf(w, diagramOf(w), tr)).toEqual({ door: '/gsd:execute-phase', to: 'gsd:execute-phase' });
  });

  it('keeps a door the workflow does not know as it was typed, into the step it led to', () => {
    const w = gsd();
    const tr = track('a1', { step: 'gsd:plan-phase', passages: [passage(1, 'gsd:plan-phase', 0, 3, { door: '/my-own-door now' })] });
    expect(entryOf(w, diagramOf(w), tr)).toEqual({ door: '/my-own-door', to: 'gsd:plan-phase' });
  });

  it('without a history, is the first entry that reaches its step, else its step', () => {
    const w = gsd();
    const m = diagramOf(w);
    expect(entryOf(w, m, track('a1', { step: 'gsd:verify-work' }))).toEqual({ door: '/gsd:discuss-phase', to: 'gsd:discuss-phase' });
    const s = ship();
    s.doors = [];
    const lone = { ...s, steps: [...s.steps, { ...s.steps[0], name: 'docs', command: '/docs', order: 9 }] };
    // Nothing leads to « docs »: the last step before it and itself only go round.
    lone.transitions = [
      ...lone.transitions,
      { from: 'merge', to: 'merge', back: true, condition: 'encore', max: 2 },
      { from: 'docs', to: 'docs', back: true, condition: 'encore', max: 2 },
    ];
    expect(entryOf(lone, diagramOf(lone), track('a1', { workflow: 'ship', step: 'docs' }))).toEqual({ door: '/docs', to: 'docs' });
  });

  it('is unknown (null) with no agent, another workflow, or steps the workflow no longer has', () => {
    const w = gsd();
    const m = diagramOf(w);
    expect(entryOf(w, m, null)).toBeNull();
    expect(entryOf(w, m, track('a1', { workflow: 'other', step: 'x' }))).toBeNull();
    expect(entryOf(w, m, track('a1', { step: null, passages: [passage(1, 'gone-step', 0, 3)] }))).toBeNull();
  });
});

describe('what an entry reaches', () => {
  it('follows the ways forward and the ways back, not the next round', () => {
    const m = diagramOf(gsd());
    expect([...reachOf(m, 'gsd:plan-phase')]).toEqual(['gsd:plan-phase', 'gsd:execute-phase', 'gsd:verify-work']);
    // By a way back: from Execute, Verify leads back to Plan.
    expect([...reachOf(m, 'gsd:execute-phase')].sort()).toEqual(['gsd:execute-phase', 'gsd:plan-phase', 'gsd:verify-work']);
    expect(reachOf(m, 'gsd:discuss-phase').size).toBe(4);
    expect(reachOf(m, 'nowhere').size).toBe(0);
  });

  it('keeps of the diagram only what the entry reaches, the rest aside', () => {
    const m = diagramOf(gsd());
    const { model, off } = scopeTo(m, { door: '/gsd:plan-phase', to: 'gsd:plan-phase' });
    expect(model.steps.map((s) => s.id)).toEqual(['gsd:plan-phase', 'gsd:execute-phase', 'gsd:verify-work']);
    expect(off.map((s) => s.id)).toEqual(['gsd:discuss-phase']);
    expect(model.go.map((g) => `${g.from}>${g.to}`)).toEqual(['gsd:plan-phase>gsd:execute-phase', 'gsd:execute-phase>gsd:verify-work']);
    expect(model.back).toHaveLength(2);
    expect(model.entries).toEqual([{ door: '/gsd:plan-phase', to: 'gsd:plan-phase' }]);
    // The exit is reached; the next round would go back to a step this entry does not reach.
    expect(model.exit).toMatchObject({ from: 'gsd:verify-work', again: '' });
  });

  it('keeps everything from the first step, and no exit when its step is out of reach', () => {
    const m = diagramOf(gsd());
    const all = scopeTo(m, { door: '/gsd:next', to: 'gsd:discuss-phase' });
    expect(all.off).toEqual([]);
    expect(all.model.steps).toHaveLength(4);
    expect(all.model.exit?.again).toBe('tour suivant');
    const s = ship();
    s.exit = 'merged';
    s.transitions = s.transitions.filter((tr) => tr.from !== 'test' || tr.to !== 'review');
    const cut = scopeTo(diagramOf(s), { door: '/hotfix', to: 'fix' });
    expect(cut.model.steps.map((x) => x.id)).toEqual(['test', 'fix']);
    expect(cut.model.exit).toBeNull();
    expect(cut.off.map((x) => x.id)).toEqual(['build', 'review', 'merge']);
  });

  it('keeps the steps the agent reached otherwise, by a command typed on the way, and where they lead', () => {
    const s = ship();
    s.transitions = s.transitions.filter((tr) => tr.from !== 'test' || tr.to !== 'review');
    const m = diagramOf(s);
    // Came in by /hotfix, then went to the review by its command: the review is not reached from the fix.
    const { model, off } = scopeTo(m, { door: '/hotfix', to: 'fix' }, ['fix', 'test', 'review']);
    expect(model.steps.map((x) => x.id)).toEqual(['build', 'test', 'fix', 'review', 'merge']);
    expect(off).toEqual([]);
  });

  it('keeps the whole diagram for an entry that leads nowhere known', () => {
    const m = diagramOf(gsd());
    const { model, off } = scopeTo(m, { door: '/x', to: 'nowhere' });
    expect(model.steps).toHaveLength(4);
    expect(off).toEqual([]);
  });
});

describe('the real path of an agent, in its order', () => {
  it('numbers each passage, with how it came there: a round of a loop, a way forward, a way back and its reason', () => {
    const w = gsd();
    const path = pathOf(w, diagramOf(w), enteredByPlan());
    expect(path.map((p) => [p.n, p.step, p.pass, p.via])).toEqual([
      [1, 'gsd:plan-phase', 1, 'entry'],
      [2, 'gsd:plan-phase', 2, 'loop'],
      [3, 'gsd:execute-phase', 1, 'go'],
      [4, 'gsd:verify-work', 1, 'go'],
      [5, 'gsd:plan-phase', 3, 'back'],
      [6, 'gsd:execute-phase', 2, 'go'],
    ]);
    expect(path[0].door).toBe('/gsd:plan-phase');
    expect(path[1].reason).toBe('plan refusé : la tâche 4 est sans test');
    expect(path[4]).toMatchObject({ reason: '2 tests rouges', from: 'gsd:verify-work' });
    expect(path.map((p) => p.on)).toEqual([false, false, false, false, false, true]);
    expect(path[4].name).toBe('Plan phase');
  });

  it('says the condition of the way back when the step wrote no reason', () => {
    const w = gsd();
    const tr = paginationUsers();
    tr.passages[3] = { ...tr.passages[3], reason: null };
    expect(pathOf(w, diagramOf(w), tr)[4]).toMatchObject({ via: 'back', reason: 'If gaps are found, run /gsd:plan-phase --gaps' });
  });

  it('skips what is not a step of the workflow, and is empty without an agent or a history', () => {
    const w = gsd();
    const m = diagramOf(w);
    const tr = track('a1', {
      step: 'gsd:plan-phase',
      passages: [
        passage(1, 'gsd:discuss-phase', 0, 2),
        passage(2, null, 2, 1, { doing: 'git status' }),
        passage(3, 'x', 3, 1, { workflow: 'other' }),
        passage(4, 'gsd:plan-phase', 4, 1, { outcome: 'running' }),
      ],
    });
    expect(pathOf(w, m, tr).map((p) => [p.n, p.step])).toEqual([
      [1, 'gsd:discuss-phase'],
      [2, 'gsd:plan-phase'],
    ]);
    expect(pathOf(w, m, null)).toEqual([]);
    expect(pathOf(w, m, track('a1', { step: 'gsd:plan-phase' }))).toEqual([]);
  });

  it('has no step in course once the agent is out of the workflow', () => {
    const w = gsd();
    const tr = { ...enteredByPlan(), outside: 'autre chose' };
    expect(pathOf(w, diagramOf(w), tr).some((p) => p.on)).toBe(false);
  });

  it('gives the diagram the numbers of each step, the ways it took, its rounds and its reasons', () => {
    const w = gsd();
    const m = diagramOf(w);
    const p = progressOf(w, m, enteredByPlan());
    expect(p.seen).toEqual({ 'gsd:plan-phase': [1, 2, 5], 'gsd:execute-phase': [3, 6], 'gsd:verify-work': [4] });
    expect(p.walked).toEqual(['gsd:plan-phase>gsd:execute-phase', 'gsd:execute-phase>gsd:verify-work']);
    expect([...p.took].sort()).toEqual(['gsd:plan-phase>gsd:plan-phase', 'gsd:verify-work>gsd:plan-phase']);
    expect(p.rounds).toEqual({ 'gsd:plan-phase': 1 });
    expect(p.reasons).toEqual({
      'gsd:plan-phase>gsd:plan-phase': 'plan refusé : la tâche 4 est sans test',
      'gsd:verify-work>gsd:plan-phase': '2 tests rouges',
    });
    expect(p.on).toBe('gsd:execute-phase');
    expect(p.next).toEqual(['gsd:verify-work']);
  });
});

describe('the layout of a path on its diagram', () => {
  const drawn = (width = 312) => {
    const w = gsd();
    const m = diagramOf(w);
    const tr = enteredByPlan();
    const entry = entryOf(w, m, tr)!;
    return layout(scopeTo(m, entry).model, { width, measure, progress: progressOf(w, m, tr), entryLabel: 'Entrée' });
  };

  it('starts from the one door taken, drawn as taken, above the first step it reaches', () => {
    const l = drawn();
    expect(l.doors).toEqual([expect.objectContaining({ text: '/gsd:plan-phase', label: 'Entrée', taken: true })]);
    expect(l.nodes.map((n) => n.id)).toEqual(['gsd:plan-phase', 'gsd:execute-phase', 'gsd:verify-work']);
    expect(l.doors[0].y).toBeLessThan(l.nodes[0].y);
    expect(l.edges.filter((e) => e.cls === 'door walked')).toHaveLength(1);
    // Without an agent, the door is only a possible.
    const alone = layout(diagramOf(gsd()), { width: 312, measure });
    expect(alone.doors.every((d) => !d.taken)).toBe(true);
    expect(alone.edges.some((e) => e.cls.includes('walked'))).toBe(false);
  });

  it('numbers the passages on their step, the one in course marked', () => {
    const l = drawn();
    expect(l.nodes.map((n) => n.beads.map((b) => b.n))).toEqual([[1, 2, 5], [3, 6], [4]]);
    expect(l.nodes[1].beads[1].on).toBe(true);
    expect(l.nodes.flatMap((n) => n.beads).filter((b) => b.on)).toHaveLength(1);
    expect(l.nodes.map((n) => n.state)).toEqual(['done', 'on', 'next']);
    // The beads stay inside their step, clear of its name.
    for (const n of l.nodes)
      for (const b of n.beads) {
        expect(b.cx - 9).toBeGreaterThan(n.x + 40);
        expect(b.cx + 9).toBeLessThanOrEqual(n.x + n.w);
      }
  });

  it('keeps a step the way back undid as passed: its numbers stay, it is no longer done', () => {
    const w = gsd();
    const m = diagramOf(w);
    const tr = enteredByPlan();
    tr.passages = tr.passages.slice(0, 5).map((p, i) => (i === 4 ? { ...p, outcome: 'running' as const, ended: null } : p));
    tr.step = 'gsd:plan-phase';
    const l = layout(scopeTo(m, entryOf(w, m, tr)!).model, { width: 312, measure, progress: progressOf(w, m, tr) });
    expect(l.nodes.map((n) => n.state)).toEqual(['on', 'next', 'seen']);
    expect(l.nodes[2].beads.map((b) => b.n)).toEqual([4]);
  });

  it('draws what was taken full and what is only possible light', () => {
    const l = drawn();
    const lines = l.edges.filter((e) => !e.cls.includes('head'));
    expect(lines.filter((e) => e.cls === 'walked')).toHaveLength(2);
    expect(lines.filter((e) => e.cls === 'took')).toHaveLength(1);
    // The exit was never reached: possible.
    expect(lines.filter((e) => e.cls === 'fwd')).toHaveLength(1);
    expect(l.exit?.next).toBe(false);
    // The way back taken says its reason, not the condition written in the workflow.
    expect(l.chips.find((c) => c.kind === 'ret')).toMatchObject({ took: true, full: '2 tests rouges' });
    // The loop on Plan: one round taken of three.
    expect(l.nodes[0].self).toMatchObject({ took: true, turns: 1, max: 3, cond: 'plan refusé : la tâche 4 est sans test' });
  });

  it('keeps the last numbers of a step passed many times, and says how many more', () => {
    const w = gsd();
    const m = diagramOf(w);
    const passages = Array.from({ length: 9 }, (_, i) =>
      passage(i + 1, 'gsd:plan-phase', i, 1, { pass: i + 1, outcome: i < 8 ? 'back' : 'running' }),
    );
    const tr = track('a1', { step: 'gsd:plan-phase', passages });
    const l = layout(scopeTo(m, entryOf(w, m, tr)!).model, { width: 312, measure, progress: progressOf(w, m, tr) });
    expect(l.nodes[0].beads.map((b) => b.n)).toEqual([7, 8, 9]);
    expect(l.nodes[0].beadsMore).toMatchObject({ text: '+6' });
  });

  it('holds in the column and across: nothing out of the width', () => {
    for (const l of [drawn(312), drawn(200)]) {
      for (const n of l.nodes) {
        expect(n.x).toBeGreaterThanOrEqual(0);
        expect(n.x + n.w).toBeLessThanOrEqual(l.width);
      }
      for (const d of l.doors) expect(d.x + d.w).toBeLessThanOrEqual(l.width);
    }
    const w = gsd();
    const m = diagramOf(w);
    const tr = enteredByPlan();
    const wide = layout(scopeTo(m, entryOf(w, m, tr)!).model, { dir: 'LR', names: true, measure, progress: progressOf(w, m, tr) });
    expect(wide.nodes.map((n) => n.beads.length)).toEqual([3, 2, 1]);
    expect(wide.doors[0].x + wide.doors[0].w).toBeLessThan(wide.nodes[0].x);
  });
});
