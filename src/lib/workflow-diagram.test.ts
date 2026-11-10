import { describe, expect, it } from 'vitest';
import { gsd, paginationUsers, ralph } from '../test/workflows';
import type { Workflow } from './types';
import { diagramOf, layout, mermaidText, progressOf, ranks, toMermaid, type DgModel, type Measure } from './workflow-diagram';
import { ticketsWorkflow } from './workflow-view';
import { ticket } from '../test/ipc';

/** Every character 7 px wide: enough to lay out without a canvas. */
const measure: Measure = (text) => text.length * 7;

/** « Livraison » of the preview: two choices, a loop of correction, a way back from the review. */
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

describe('the diagram of a workflow, read from it', () => {
  it('takes GSD’s steps in their order, its loop on Plan, its way back from Verify, its exit and its next round', () => {
    const m = diagramOf(gsd());
    expect(m.steps.map((s) => s.name)).toEqual(['Discuss phase', 'Plan phase', 'Execute phase', 'Verify work']);
    expect(m.go.map((g) => `${g.from}>${g.to}`)).toEqual([
      'gsd:discuss-phase>gsd:plan-phase',
      'gsd:plan-phase>gsd:execute-phase',
      'gsd:execute-phase>gsd:verify-work',
    ]);
    // A single way forward needs no condition.
    expect(m.go.every((g) => !g.cond)).toBe(true);
    expect(m.back).toEqual([
      { from: 'gsd:plan-phase', to: 'gsd:plan-phase', cond: 'iteration_count < 3', max: 3 },
      { from: 'gsd:verify-work', to: 'gsd:plan-phase', cond: 'If gaps are found, run /gsd:plan-phase --gaps', max: null },
    ]);
    expect(m.exit).toEqual({
      from: 'gsd:verify-work',
      cond: 'The milestone is complete when all phases are complete.',
      label: 'Sortie',
      again: 'tour suivant',
    });
  });

  it('takes the entries that are no step, into the step they enter, three at most', () => {
    const m = diagramOf(gsd());
    expect(m.entries).toEqual([
      { door: '/gsd:autonomous', to: 'gsd:discuss-phase' },
      { door: '/gsd:progress', to: 'gsd:discuss-phase' },
      { door: '/gsd:next', to: 'gsd:discuss-phase' },
    ]);
    expect(diagramOf(ship()).entries).toEqual([{ door: '/hotfix', to: 'fix' }]);
  });

  it('keeps the conditions of a choice, two ways forward out of one step', () => {
    const m = diagramOf(ship());
    expect(m.go.filter((g) => g.from === 'test')).toEqual([
      { from: 'test', to: 'fix', cond: 'rouges' },
      { from: 'test', to: 'review', cond: 'verts' },
    ]);
    // Ranked by the longest way forward: the two branches side by side.
    expect(ranks(m).rows.map((r) => r.map((s) => s.id))).toEqual([['build'], ['test'], ['fix', 'review'], ['merge']]);
  });

  it('reads a loop of one prompt: its steps, its way back, its entry, its exit by the turns', () => {
    const m = diagramOf(ralph());
    expect(m.steps.map((s) => s.id)).toEqual(['1', '2', '3', '4']);
    expect(m.back).toEqual([{ from: '3', to: '2', cond: 'If they are red', max: 3 }]);
    expect(m.entries).toEqual([
      { door: 'PROMPT.md', to: '1' },
      { door: 'file', to: '1' },
    ]);
    expect(m.exit?.cond).toBe('plus de case vide, ou 20 tours');
    expect(m.exit?.again).toBe('tour suivant');
  });

  it('draws « Tickets » with its loop on « En cours » and « Renvoyer », without an exit', () => {
    const w = ticketsWorkflow([ticket({ maxLoops: 5 })])!;
    const m = diagramOf(w, 5);
    expect(m.back[0]).toEqual({ from: 'En cours', to: 'En cours', cond: 'critères non atteints', max: 5 });
    expect(m.back[1]).toMatchObject({ from: 'À tester', to: 'En cours', cond: 'Renvoyer : retour à En cours' });
    expect(m.exit).toBeNull();
  });
});

describe('the code in Mermaid', () => {
  it('writes GSD: entries, steps with their command, the loop and the way back dashed, the exit and the next round', () => {
    expect(toMermaid(diagramOf(gsd()))).toBe(
      [
        'flowchart TD',
        '  %% GSD',
        '  in1(["/gsd:autonomous"]) --> s1',
        '  in2(["/gsd:progress"]) --> s1',
        '  in3(["/gsd:next"]) --> s1',
        '  s1["Discuss phase<br>/gsd:discuss-phase"]',
        '  s2["Plan phase<br>/gsd:plan-phase"]',
        '  s3["Execute phase<br>/gsd:execute-phase"]',
        '  s4["Verify work<br>/gsd:verify-work"]',
        '  s1 --> s2',
        '  s2 --> s3',
        '  s3 --> s4',
        '  s2 -.->|"iteration_count #lt; 3 · 3 tours max"| s2',
        '  s4 -.->|"If gaps are found, run /gsd:plan-phase --gaps"| s2',
        '  s4 -->|"The milestone is complete when all phases are complete."| out(("Sortie"))',
        '  out -.->|"tour suivant"| s1',
        '  classDef door stroke-dasharray: 3 3',
        '  class in1,in2,in3 door',
      ].join('\n'),
    );
  });

  it('writes a choice as two labelled ways out of one step', () => {
    const code = toMermaid(diagramOf(ship()));
    expect(code).toContain('  s2 -->|"rouges"| s3');
    expect(code).toContain('  s2 -->|"verts"| s4');
    expect(code).toContain('  s3 -.->|"à retester · 3 tours max"| s2');
    expect(code).toContain('  in1(["/hotfix"]) --> s3');
    expect(code).not.toContain('out((');
  });

  it('keeps a trapped name a label: no HTML, no way out of its quotes, no click, no link', () => {
    const trap = '<img src=x onerror="alert(1)">"]-->x click s1 call alert(1) %% `md` #lt;\nnext';
    expect(mermaidText(trap)).toBe(
      '#lt;img src=x onerror=#quot;alert(1)#quot;#gt;#quot;]--#gt;x click s1 call alert(1) %% #96;md#96; #35;lt; next',
    );
    const m: DgModel = {
      name: trap,
      steps: [
        { id: 'a', name: trap, cmd: 'javascript:alert(1)' },
        { id: 'b', name: 'B', cmd: '' },
      ],
      go: [{ from: 'a', to: 'b', cond: '' }],
      back: [{ from: 'b', to: 'a', cond: trap, max: null }],
      entries: [{ door: trap, to: 'a' }],
      exit: null,
    };
    const code = toMermaid(m);
    const lines = code.split('\n');
    // One line per statement: the trap's line break did not open one.
    expect(lines).toEqual([
      'flowchart TD',
      expect.stringMatching(/^ {2}%% #lt;img/),
      expect.stringMatching(/^ {2}in1\(\["#lt;img .*"\]\) --> s1$/),
      expect.stringMatching(/^ {2}s1\["#lt;img .*<br>javascript:alert\(1\)"\]$/),
      '  s2["B"]',
      '  s1 --> s2',
      expect.stringMatching(/^ {2}s2 -\.->\|"#lt;img .*"\| s1$/),
      '  classDef door stroke-dasharray: 3 3',
      '  class in1 door',
    ]);
    for (const line of lines.slice(2)) {
      // Inside its quotes, the trap has no quote, no angle bracket, no backtick left: the only HTML is our line break.
      for (const q of line.match(/"[^"]*"/g) ?? []) expect(q.slice(1, -1).replace('<br>', '')).not.toMatch(/["<>`]/);
      // Out of the quotes, only our own syntax: never `click`, `call`, `style` nor a link.
      expect(line.replace(/"[^"]*"/g, '""')).not.toMatch(/click|call|style |href|javascript/);
    }
  });
});

describe('where the agent read is', () => {
  it('has the steps done, the one it is on, the next, and the way back it took', () => {
    const w = gsd();
    const m = diagramOf(w);
    const tr = paginationUsers();
    expect(progressOf(w, m, tr)).toEqual({
      done: ['gsd:discuss-phase', 'gsd:plan-phase', 'gsd:execute-phase'],
      on: 'gsd:verify-work',
      next: [],
      // At the end of the cycle, the exit comes next (ESC-244's nextOf).
      exitNext: true,
      took: ['gsd:verify-work>gsd:plan-phase'],
      // Its path on the diagram (ESC-406): the numbers of its passages, the ways it took, why it came back.
      seen: { 'gsd:discuss-phase': [1], 'gsd:plan-phase': [2, 5], 'gsd:execute-phase': [3, 6], 'gsd:verify-work': [4, 7] },
      at: 7,
      walked: ['gsd:discuss-phase>gsd:plan-phase', 'gsd:plan-phase>gsd:execute-phase', 'gsd:execute-phase>gsd:verify-work'],
      rounds: {},
      reasons: { 'gsd:verify-work>gsd:plan-phase': '2 écarts : le curseur n’est pas validé' },
    });
    expect(progressOf(w, m, null).on).toBeNull();
  });

  it('has both branches of a choice next', () => {
    const w = ship();
    const m = diagramOf(w);
    const p = progressOf(w, m, { ...paginationUsers(), workflow: 'ship', step: 'test', passages: [] });
    expect(p.next).toEqual(['fix', 'review']);
  });
});

describe('the layout', () => {
  it('lays GSD down the column: four steps in a column, the entries above, the exit below', () => {
    const l = layout(diagramOf(gsd()), { width: 312, measure });
    expect(l.width).toBe(312);
    const ys = l.nodes.map((n) => n.y);
    expect([...ys].sort((a, b) => a - b)).toEqual(ys);
    expect(new Set(l.nodes.map((n) => n.x)).size).toBe(1);
    expect(l.doors.every((d) => d.y < ys[0])).toBe(true);
    expect(l.exit!.y).toBeGreaterThan(ys[3]);
    // Everything inside the width.
    for (const n of l.nodes) expect(n.x + n.w).toBeLessThanOrEqual(312);
    for (const c of l.chips) expect(c.x).toBeGreaterThanOrEqual(0);
  });

  it('says the loop on a step in it, the way back on its lane in red, the next round dashed', () => {
    const l = layout(diagramOf(gsd()), { width: 312, measure, maxRounds: (n) => `×${n}` });
    expect(l.nodes[1].self).toMatchObject({ max: 3, cond: 'iteration_count < 3' });
    expect(l.edges.filter((e) => e.cls === 'back')).toHaveLength(1);
    expect(l.edges.filter((e) => e.cls === 'again')).toHaveLength(1);
    expect(l.chips.map((c) => c.kind).sort()).toEqual(['again', 'cond', 'ret']);
  });

  it('sets a choice’s branches side by side, a diamond under the step, a condition on each', () => {
    const l = layout(diagramOf(ship()), { width: 312, measure });
    const at = (id: string) => l.nodes.find((n) => n.id === id)!;
    expect(at('fix').y).toBe(at('review').y);
    expect(at('fix').x).toBeLessThan(at('review').x);
    expect(at('test').fork).not.toBeNull();
    expect(l.chips.filter((c) => c.kind === 'cond').map((c) => c.full)).toEqual(['rouges', 'verts', 'approuvée']);
    // The way back from the left branch goes by the left gutter, the other by the right.
    const ret = l.chips.filter((c) => c.kind === 'ret');
    expect(ret.find((c) => c.full.startsWith('à retester'))!.x).toBeLessThan(at('fix').x + 20);
    expect(ret.find((c) => c.full === 'changements demandés')!.x).toBeGreaterThan(at('build').x + 20);
    // The door into a later step comes in just above it.
    const hotfix = l.doors.find((d) => d.text === '/hotfix')!;
    expect(hotfix.y).toBeLessThan(at('fix').y);
    expect(hotfix.y).toBeGreaterThan(at('test').y + at('test').h);
  });

  it('marks the agent read’s steps, the way back it took, and who is on each step', () => {
    const w = gsd();
    const m = diagramOf(w);
    const l = layout(m, {
      width: 312,
      measure,
      progress: progressOf(w, m, paginationUsers()),
      agents: { 'gsd:verify-work': [{ id: 'a1', name: 'pagination-users', cur: true, wait: false }] },
    });
    expect(l.nodes.map((n) => n.state)).toEqual(['done', 'done', 'done', 'on']);
    expect(l.exit?.next).toBe(true);
    // The way back taken, not the loop on Plan (a single passage there).
    expect(l.edges.filter((e) => e.cls === 'took')).toHaveLength(1);
    expect(l.nodes[3].dots).toEqual([expect.objectContaining({ id: 'a1', cur: true })]);
  });

  it('cuts a long name by an ellipsis and keeps it whole beside', () => {
    const w = gsd();
    w.steps[0] = { ...w.steps[0], named: true, name: 'A very long name of a step that never ends in the column' };
    w.transitions[0] = { ...w.transitions[0], from: w.steps[0].name };
    const l = layout(diagramOf(w), { width: 312, measure });
    expect(l.nodes[0].name.endsWith('…')).toBe(true);
    expect(l.nodes[0].full).toBe('A very long name of a step that never ends in the column');
  });

  it('lays out across when enlarged, the agents by name under their step', () => {
    const l = layout(diagramOf(gsd()), {
      dir: 'LR',
      names: true,
      measure,
      agents: {
        'gsd:plan-phase': [
          { id: 'a', name: 'refacto', cur: false, wait: false },
          { id: 'b', name: 'other', cur: true, wait: false },
        ],
      },
    });
    const xs = l.nodes.map((n) => n.x);
    expect([...xs].sort((a, b) => a - b)).toEqual(xs);
    expect(new Set(l.nodes.map((n) => n.y)).size).toBe(1);
    expect(l.nodes[1].who).toMatchObject({ name: 'other', more: '+1', cur: true });
    expect(l.exit!.x).toBeGreaterThan(xs[3]);
  });
});
