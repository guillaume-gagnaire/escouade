import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { t, tIn } from './i18n';
import { canWriteSub, crumbs, isSubagentCall, sidebarSubs, subagentInfo, subDuration, threadOf } from './subagents';
import type { ConvItem, PlanState, SubAgent, ToolItem } from './types';

const sub = (id: string, over: Partial<SubAgent> = {}): SubAgent => ({
  id,
  title: `sous-agent ${id}`,
  status: 'running',
  background: true,
  tools: 0,
  startedAt: 1_000,
  ...over,
});
const planOf = (agents: SubAgent[]): PlanState => ({
  source: null,
  tasks: [],
  agents,
  launched: agents.length,
  workflows: [],
  bgTasks: [],
});
const call = (id: string, over: Partial<ToolItem> = {}): ToolItem => ({
  kind: 'tool',
  id,
  name: 'Agent',
  input: { description: `Appel ${id}`, subagent_type: 'Explore', model: 'haiku', prompt: `Consigne de ${id}` },
  status: 'ok',
  ts: 1,
  ...over,
});
const text = (id: string, parent: string | null, t = 'texte'): ConvItem => ({ kind: 'text', id, parent, text: t, streaming: false });

describe('sidebarSubs', () => {
  it('is nothing for an agent that never had a subagent', () => {
    expect(sidebarSubs(agent())).toBeNull();
    expect(sidebarSubs(agent({ plan: planOf([]) }))).toBeNull();
  });

  it('lists those running in the plan’s order, then those finished, the latest ended first', () => {
    const a = agent({
      plan: planOf([
        sub('s1', { status: 'done', endedAt: 5_000 }),
        sub('s2'),
        sub('s3', { status: 'stopped', endedAt: 9_000 }),
        sub('s4'),
        sub('s5', { status: 'interrupted', endedAt: 7_000 }),
      ]),
    });
    const subs = sidebarSubs(a)!;
    expect(subs.running.map((s) => s.id)).toEqual(['s2', 's4']);
    expect(subs.finished.map((s) => s.id)).toEqual(['s3', 's5', 's1']);
  });

  it('tells how deep a subagent’s subagent is', () => {
    const a = agent({ plan: planOf([sub('s1'), sub('s2', { parent: 's1' }), sub('s3', { parent: 's2' }), sub('s4', { parent: 'gone' })]) });
    expect(sidebarSubs(a)!.running.map((s) => [s.id, s.depth])).toEqual([
      ['s1', 0],
      ['s2', 1],
      ['s3', 2],
      ['s4', 0],
    ]);
  });
});

describe('subagentInfo', () => {
  it('takes what the plan’s row says, and the prompt and report from its call', () => {
    const a = agent({
      plan: planOf([sub('c1', { title: 'Chercher', kind: 'Explore', model: 'haiku', tokens: 1_800, tools: 3, taskId: 't1' })]),
    });
    const info = subagentInfo(a, [call('c1', { report: 'Trouvé.' })], 'c1')!;
    expect(info).toMatchObject({
      call: 'c1',
      title: 'Chercher',
      kind: 'Explore',
      model: 'haiku',
      prompt: 'Consigne de c1',
      status: 'running',
      tokens: 1_800,
      tools: 3,
      startedAt: 1_000,
      report: 'Trouvé.',
      taskId: 't1',
    });
  });

  it('reads a subagent the plan forgot from its call and what the conversation kept of it', () => {
    const items = [call('c1', { run: { status: 'stopped', tokens: 900, tools: 2, startedAt: 10, endedAt: 70 }, report: 'Fini.' })];
    const info = subagentInfo(agent(), items, 'c1')!;
    expect(info).toMatchObject({
      title: 'Appel c1',
      kind: 'Explore',
      model: 'haiku',
      status: 'stopped',
      tokens: 900,
      tools: 2,
      startedAt: 10,
      endedAt: 70,
      report: 'Fini.',
    });
    expect(info.taskId).toBeUndefined();
  });

  it('gives the model it ran on, as the conversation kept it, over the one its call asked for', () => {
    const run = { status: 'done' as const, model: 'claude-haiku-5-5', tools: 1, startedAt: 10, endedAt: 70 };
    expect(subagentInfo(agent(), [call('c1', { run })], 'c1')!.model).toBe('claude-haiku-5-5');
    // While the plan has its row, the row tells.
    const a = agent({ plan: planOf([sub('c1', { model: 'claude-sonnet-5-5' })]) });
    expect(subagentInfo(a, [call('c1', { run })], 'c1')!.model).toBe('claude-sonnet-5-5');
  });

  it('takes the result of a subagent in the foreground as its report, never the words of a launch', () => {
    const done = call('c1', { result: { text: 'Le filtre refuse les PDF.', isError: false } });
    expect(subagentInfo(agent(), [done], 'c1')!.report).toBe('Le filtre refuse les PDF.');
    expect(subagentInfo(agent(), [done], 'c1')!.status).toBe('done');
    const launched = call('c2', { result: { text: 'Async agent launched successfully.\nagentId: x', isError: false } });
    expect(subagentInfo(agent(), [launched], 'c2')!.report).toBeUndefined();
    expect(subagentInfo(agent(), [launched], 'c2')!.status).toBeNull();
    const failed = call('c3', { status: 'error', result: { text: 'Boom', isError: true } });
    expect(subagentInfo(agent(), [failed], 'c3')!.status).toBe('failed');
    expect(subagentInfo(agent(), [failed], 'c3')!.report).toBeUndefined();
  });

  it('says a call left running with no row of the plan is interrupted: the plan keeps every row that runs', () => {
    const left = call('c1', { status: 'running', result: undefined });
    expect(subagentInfo(agent(), [left], 'c1')!.status).toBe('interrupted');
    // With its row, the row says.
    const a = agent({ plan: planOf([sub('c1')]) });
    expect(subagentInfo(a, [left], 'c1')!.status).toBe('running');
  });

  it('names a subagent by its call as the plan does: description, name, type, else Agent', () => {
    const named = (input: Record<string, unknown>) => subagentInfo(agent(), [call('c', { input })], 'c')!.title;
    expect(named({ name: 'qa-mobile', subagent_type: 'Explore' })).toBe('qa-mobile');
    expect(named({ subagent_type: 'Explore' })).toBe('Explore');
    expect(named({})).toBe('Agent');
  });

  it('is nothing for a call that is no subagent’s, or that nothing knows', () => {
    const bash: ToolItem = { kind: 'tool', id: 'b1', name: 'Bash', input: { command: 'ls' }, status: 'ok', ts: 1 };
    expect(subagentInfo(agent(), [bash], 'b1')).toBeNull();
    expect(subagentInfo(agent(), [], 'nope')).toBeNull();
    expect(isSubagentCall(bash)).toBe(false);
    expect(isSubagentCall(call('c1'))).toBe(true);
    expect(isSubagentCall({ ...call('c2'), name: 'Task' })).toBe(true);
  });

  it('is known by its row alone before its call is read', () => {
    const a = agent({ plan: planOf([sub('c1', { title: 'Tôt' })]) });
    expect(subagentInfo(a, [], 'c1')).toMatchObject({ title: 'Tôt', status: 'running' });
  });
});

describe('subDuration', () => {
  it('runs until now while it runs, and stops at its end', () => {
    expect(subDuration({ status: 'running', startedAt: 1_000 }, 4_000)).toBe(3_000);
    expect(subDuration({ status: 'done', startedAt: 1_000, endedAt: 2_500 }, 9_000)).toBe(1_500);
    expect(subDuration({ status: 'done', startedAt: 1_000 }, 9_000)).toBeNull();
    expect(subDuration({ status: null }, 9_000)).toBeNull();
  });
});

describe('threadOf', () => {
  it('is the items of its thread and the requests it made, in order, not those of another', () => {
    const items: ConvItem[] = [
      call('c1'),
      text('t0', null),
      text('t1', 'c1'),
      {
        kind: 'permission',
        id: 'r1',
        toolUseId: 'k1',
        toolName: 'Bash',
        input: {},
        canAlways: false,
        defaultNo: false,
        decision: null,
        ts: 1,
        sub: 'c1',
      },
      text('t2', 'c2'),
      { kind: 'question', id: 'r2', toolUseId: 'q', questions: [], answers: null, ts: 2 },
      call('c3', { parent: 'c1' }),
      text('t3', 'c1'),
    ];
    expect(threadOf(items, 'c1').map((i) => i.id)).toEqual(['t1', 'r1', 'c3', 't3']);
    expect(threadOf(items, 'c2').map((i) => i.id)).toEqual(['t2']);
  });
});

describe('crumbs', () => {
  it('goes from the agent’s thread down to the subagent', () => {
    const items = [call('c1'), call('c2', { parent: 'c1' }), call('c3', { parent: 'c2' })];
    const a = agent({ plan: planOf([sub('c2', { title: 'Milieu', parent: 'c1' })]) });
    expect(crumbs(a, items, 'c3')).toEqual([
      { call: 'c1', title: 'Appel c1' },
      { call: 'c2', title: 'Milieu' },
      { call: 'c3', title: 'Appel c3' },
    ]);
    expect(crumbs(a, items, 'c1')).toEqual([{ call: 'c1', title: 'Appel c1' }]);
  });

  it('never loops on calls that name each other', () => {
    const items = [call('c1', { parent: 'c2' }), call('c2', { parent: 'c1' })];
    expect(crumbs(agent(), items, 'c1').length).toBeLessThanOrEqual(10);
  });

  it('stops at a parent that neither the plan nor the conversation knows: no link that leads to an empty view', () => {
    // By its call.
    expect(crumbs(agent(), [call('c2', { parent: 'gone' })], 'c2')).toEqual([{ call: 'c2', title: 'Appel c2' }]);
    // By its row of the plan.
    const a = agent({ plan: planOf([sub('c2', { parent: 'gone' })]) });
    expect(crumbs(a, [], 'c2')).toEqual([{ call: 'c2', title: 'sous-agent c2' }]);
    // An item with that id which is no subagent's call is no parent either.
    const bash: ToolItem = { kind: 'tool', id: 'b1', name: 'Bash', input: { command: 'ls' }, status: 'ok', ts: 1 };
    expect(crumbs(agent(), [bash, call('c2', { parent: 'b1' })], 'c2')).toEqual([{ call: 'c2', title: 'Appel c2' }]);
  });

  it('stops above the parent that is unknown, and keeps those that are known below it', () => {
    const items = [call('c2', { parent: 'gone' }), call('c3', { parent: 'c2' })];
    expect(crumbs(agent(), items, 'c3')).toEqual([
      { call: 'c2', title: 'Appel c2' },
      { call: 'c3', title: 'Appel c3' },
    ]);
  });

  it('keeps a parent that only its row, or only its call, knows', () => {
    const a = agent({ plan: planOf([sub('c1', { title: 'Seule la rangée' })]) });
    expect(crumbs(a, [call('c2', { parent: 'c1' })], 'c2')).toEqual([
      { call: 'c1', title: 'Seule la rangée' },
      { call: 'c2', title: 'Appel c2' },
    ]);
    const b = agent({ plan: planOf([sub('c2', { parent: 'c1' })]) });
    expect(crumbs(b, [call('c1')], 'c2')).toEqual([
      { call: 'c1', title: 'Appel c1' },
      { call: 'c2', title: 'sous-agent c2' },
    ]);
  });

  it('names the subagent it is open on even when nothing knows it', () => {
    expect(crumbs(agent(), [], 'nope')).toEqual([{ call: 'nope', title: 'Agent' }]);
  });
});

describe('the words of the subagents', () => {
  it('say what “Arrêter” does without the name of a tool of Claude Code', () => {
    expect(t('subagents.stopHint')).toBe('Arrêter ce sous-agent seul : l’agent continue.');
    expect(tIn('en', 'subagents.stopHint')).toBe('Stop this subagent alone: the agent goes on.');
    expect(t('subagents.stopHint')).not.toContain('stop_task');
    expect(tIn('en', 'subagents.stopHint')).not.toContain('stop_task');
  });
});

describe('canWriteSub', () => {
  it('lets one write to a subagent running in the background only', () => {
    const a = agent();
    expect(canWriteSub(a, { status: 'running', taskId: 't1', background: true })).toBe(true);
    expect(canWriteSub(a, { status: 'running', taskId: 't1', background: false })).toBe(false);
    expect(canWriteSub({ ...a, terminal: true }, { status: 'running', taskId: 't1', background: true })).toBe(false);
  });
});
