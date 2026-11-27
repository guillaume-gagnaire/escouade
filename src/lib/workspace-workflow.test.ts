import { describe, expect, it } from 'vitest';
import { agent, ticket } from '../test/ipc';
import { GSD_ID, gsd, paginationUsers, passage, scanOf, track } from '../test/workflows';
import { cardStepOf, projectTracks, projectWorkflows } from './workspace-workflow';
import { TICKETS } from './workflow-view';

describe('the step of an agent on its card in the Workspace (ESC-689)', () => {
  const all = [gsd()];

  it('says its workflow, its step, its stay and how it goes', () => {
    const tr = track('a1', {
      step: 'gsd:execute-phase',
      passages: [passage(1, 'gsd:plan-phase', 0, 3), passage(2, 'gsd:execute-phase', 3, 2, { outcome: 'running' })],
    });
    expect(cardStepOf(all, tr)).toEqual({
      workflow: GSD_ID,
      name: 'GSD',
      step: 'Execute phase',
      pass: 1,
      outcome: 'running',
      back: null,
    });
  });

  it('says the way back that brought it here, with its reason', () => {
    const tr = track('a1', {
      step: 'gsd:plan-phase',
      passages: [
        passage(1, 'gsd:plan-phase', 0, 3),
        passage(2, 'gsd:verify-work', 3, 2, { outcome: 'back', reason: ' 2 écarts : le curseur ', backTo: 'gsd:plan-phase' }),
        passage(3, 'gsd:plan-phase', 5, 1, { outcome: 'running', pass: 2 }),
      ],
    });
    expect(cardStepOf(all, tr)).toMatchObject({
      step: 'Plan phase',
      pass: 2,
      back: { from: 'Verify work', reason: '2 écarts : le curseur' },
    });
  });

  it('forgets the way back once the agent went on', () => {
    const tr = paginationUsers('a1');
    expect(cardStepOf(all, tr)?.back).toBeNull();
  });

  it('has nothing for an agent out of any workflow, out of its own, in « Tickets » or in a workflow gone', () => {
    expect(cardStepOf(all, null)).toBeNull();
    expect(cardStepOf(all, track('a1', { step: null }))).toBeNull();
    expect(cardStepOf(all, track('a1', { step: 'gsd:plan-phase', outside: 'review' }))).toBeNull();
    expect(cardStepOf(all, track('a1', { step: 'À faire', workflow: TICKETS }))).toBeNull();
    expect(cardStepOf([], track('a1', { step: 'gsd:plan-phase' }))).toBeNull();
  });
});

describe('the workflows of the project in the Workspace', () => {
  it('lists those read, then « Tickets » while the project has tickets, each with its agents, those at work first', () => {
    const agents = { a1: agent({ id: 'a1' }), a2: agent({ id: 'a2' }), a3: agent({ id: 'a3', ticketId: 't1' }) };
    const tickets = [ticket({ id: 't1', key: 'DEM-5', column: 'doing', agentId: 'a3' })];
    const read = [
      track('a1', { step: 'gsd:plan-phase', outside: 'review' }),
      track('a2', { step: 'gsd:execute-phase' }),
      track('gone', { step: 'gsd:execute-phase' }),
    ];
    const tracks = projectTracks('p1', read, tickets, agents, 0);
    const list = projectWorkflows(scanOf(gsd()), tickets, tracks);
    expect(list.map((l) => [l.workflow.id, l.tracks.map((tr) => tr.agentId)])).toEqual([
      [GSD_ID, ['a2', 'a1']],
      [TICKETS, ['a3']],
    ]);
    expect(projectWorkflows(null, tickets, tracks)).toEqual([]);
    expect(projectWorkflows(scanOf(), [], [])).toEqual([]);
  });
});
