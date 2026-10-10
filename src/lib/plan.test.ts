import { describe, expect, it } from 'vitest';
import { agent, ticket } from '../test/ipc';
import { planView } from './plan';
import type { Agent, PlanState, PlanTask, SubAgent } from './types';

const task = (id: string, status: PlanTask['status'], over: Partial<PlanTask> = {}): PlanTask => ({
  id,
  title: `Tâche ${id}`,
  status,
  ...over,
});
const sub = (id: string, over: Partial<SubAgent> = {}): SubAgent => ({
  id,
  title: `sous-agent ${id}`,
  status: 'running',
  background: false,
  tools: 0,
  startedAt: 1,
  ...over,
});
const plan = (tasks: PlanTask[], over: Partial<PlanState> = {}): PlanState => ({
  source: 'tools',
  tasks,
  agents: [],
  launched: 0,
  workflows: [],
  ...over,
});
const view = (tasks: PlanTask[], over: Partial<PlanState> = {}, a: Partial<Agent> = {}) => {
  const v = planView(agent({ plan: plan(tasks, over), ...a }));
  if (!v) throw new Error('no view');
  return v;
};

describe('planView', () => {
  describe('when there is something to show', () => {
    it('shows nothing for an agent with no plan', () => {
      expect(planView(agent())).toBeNull();
    });

    it('shows nothing without tasks, whatever else the plan holds, unless a subagent runs', () => {
      // An empty TodoWrite leaves an empty plan; a plan file, a launched subagent that ended, a workflow: no task list.
      expect(planView(agent({ plan: plan([]) }))).toBeNull();
      expect(planView(agent({ plan: plan([], { source: 'plan', planFile: 'docs/plan.md', title: 'Un plan' }) }))).toBeNull();
      expect(
        planView(agent({ plan: plan([], { agents: [sub('s1', { status: 'done' }), sub('s2', { status: 'interrupted' })], launched: 2 }) })),
      ).toBeNull();
      expect(
        planView(
          agent({
            plan: plan([], { workflows: [{ id: 'w1', title: 'Revue', status: 'running', tools: 0, tokens: 0, startedAt: 1 }] }),
          }),
        ),
      ).toBeNull();
    });

    it('is keyed on the tasks, not on where they come from', () => {
      expect(view([task('1', 'pending')], { source: null }).total).toBe(1);
      expect(view([task('1', 'pending')], { source: 'plan', planFile: 'docs/p.md' }).total).toBe(1);
    });

    it('shows the subagents running even without a task list, and no task list', () => {
      const v = view([], { agents: [sub('s1', { title: 'Explore' }), sub('s2', { status: 'done' })], launched: 2 });
      expect(v.total).toBe(0);
      expect(v.rows).toEqual([]);
      expect(v.focusId).toBeNull();
      expect(v.subsRunning).toBe(1);
      expect(v.subDots).toEqual([{ name: 'Explore' }]);
      expect(v.percent).toBe(0);
      // The card of the agent has its line only for a plan with tasks.
      expect(v.mini).toBeNull();
    });
  });

  describe('the states of the tasks', () => {
    it('reads the status of each task, and counts the ones done', () => {
      const v = view([task('1', 'done'), task('2', 'inProgress'), task('3', 'pending'), task('4', 'done')]);
      expect(v.rows.map((r) => r.state)).toEqual(['done', 'run', 'todo', 'done']);
      expect(v.done).toBe(2);
      expect(v.total).toBe(4);
    });

    it('ranks the tasks on two digits, and never shows the id of the call that made one', () => {
      const v = view([
        task('toolu_01AbC', 'done', { title: 'Premier' }),
        task('toolu_02', 'pending', { title: 'Second', blockedBy: ['toolu_01AbC'] }),
        ...Array.from({ length: 9 }, (_, i) => task(`x${i}`, 'pending')),
      ]);
      expect(v.rows.map((r) => r.n).slice(0, 3)).toEqual(['01', '02', '03']);
      expect(v.rows[10].n).toBe('11');
      expect(v.rows[0].id).toBe('toolu_01AbC');
      // What the window shows of a row: its rank and its title, never the id.
      expect(JSON.stringify(v.rows.map(({ n, title, state, after }) => ({ n, title, state, after })))).not.toContain('toolu_');
    });

    it('takes the title of a task as it is written', () => {
      const v = view([task('1', 'pending', { title: '<img src=x onerror=alert(1)>' })]);
      expect(v.rows[0].title).toBe('<img src=x onerror=alert(1)>');
    });

    it('blocks the first task under way when the agent waits for an answer or a permission', () => {
      const v = view(
        [task('1', 'done'), task('2', 'inProgress'), task('3', 'inProgress'), task('4', 'pending')],
        {},
        { status: 'waiting' },
      );
      expect(v.rows.map((r) => r.state)).toEqual(['done', 'block', 'run', 'todo']);
    });

    it('blocks nothing when the agent waits with no task under way', () => {
      const v = view([task('1', 'done'), task('2', 'pending')], {}, { status: 'waiting' });
      expect(v.rows.map((r) => r.state)).toEqual(['done', 'todo']);
    });

    it.each(['running', 'idle', 'done', 'error'] as const)('blocks nothing for an agent that is %s', (status) => {
      const v = view([task('1', 'inProgress')], {}, { status });
      expect(v.rows[0].state).toBe('run');
    });
  });

  describe('the task to look at', () => {
    it('is the first one blocked, else the first under way, else the first to do', () => {
      const tasks = [task('1', 'done'), task('2', 'pending'), task('3', 'inProgress'), task('4', 'inProgress')];
      const running = view(tasks);
      expect(running.focusId).toBe('3');
      expect(running.rows.map((r) => r.focus)).toEqual([false, false, true, false]);
      const blocked = view(tasks, {}, { status: 'waiting' });
      expect(blocked.focusId).toBe('3');
      const todo = view([task('1', 'done'), task('2', 'pending'), task('3', 'pending')]);
      expect(todo.focusId).toBe('2');
      expect(todo.rows.map((r) => r.focus)).toEqual([false, true, false]);
    });

    it('prefers a blocked task to one under way that comes first in a list the agent does not wait on', () => {
      // The blocked one is always the first under way: the rule is checked on the model, not on the order of the states.
      const v = view([task('1', 'inProgress'), task('2', 'inProgress')], {}, { status: 'waiting' });
      expect(v.rows.map((r) => r.state)).toEqual(['block', 'run']);
      expect(v.focusId).toBe('1');
    });

    it('is none once every task is done', () => {
      const v = view([task('1', 'done'), task('2', 'done')]);
      expect(v.focusId).toBeNull();
      expect(v.rows.every((r) => !r.focus)).toBe(true);
    });
  });

  describe('how far it is', () => {
    it('is 100 for a task done, 0 for a task to do', () => {
      const v = view([task('1', 'done'), task('2', 'pending')]);
      expect(v.rows.map((r) => r.percent)).toEqual([100, 0]);
    });

    it('is the share of the steps done for a task under way that has some', () => {
      const v = view([task('1', 'inProgress', { steps: [2, 4] }), task('2', 'inProgress', { steps: [1, 3] })]);
      expect(v.rows.map((r) => r.percent)).toEqual([50, 33]);
      expect(v.rows[0].steps).toEqual([2, 4]);
    });

    it('is not measured for a task under way without steps, nor with none done yet: a task begun is not shown at 0 %', () => {
      const v = view([task('1', 'inProgress'), task('2', 'inProgress', { steps: [0, 0] }), task('3', 'inProgress', { steps: [0, 5] })]);
      expect(v.rows.map((r) => r.percent)).toEqual([null, null, null]);
      // The steps are still told, for the tooltip.
      expect(v.rows[2].steps).toEqual([0, 5]);
      // And a blocked task is the same.
      const blocked = view([task('1', 'inProgress', { steps: [0, 3] })], {}, { status: 'waiting' });
      expect(blocked.rows[0].state).toBe('block');
      expect(blocked.rows[0].percent).toBeNull();
    });

    it('holds a task’s steps between 0 and 100', () => {
      const v = view([task('1', 'inProgress', { steps: [9, 3] })]);
      expect(v.rows[0].percent).toBe(100);
    });

    it('is the same for a blocked task: its steps, else nothing', () => {
      const v = view([task('1', 'inProgress', { steps: [1, 4] }), task('2', 'inProgress')], {}, { status: 'waiting' });
      expect(v.rows.map((r) => r.state)).toEqual(['block', 'run']);
      expect(v.rows[0].percent).toBe(25);
      const none = view([task('1', 'inProgress')], {}, { status: 'waiting' });
      expect(none.rows[0].percent).toBeNull();
    });

    it('is, for the whole plan, the mean of the tasks’, one that nothing measures counting 0, rounded', () => {
      expect(view([task('1', 'done'), task('2', 'pending'), task('3', 'pending')]).percent).toBe(33);
      expect(view([task('1', 'done'), task('2', 'pending')]).percent).toBe(50);
      expect(view([task('1', 'done'), task('2', 'inProgress')]).percent).toBe(50);
      expect(view([task('1', 'done'), task('2', 'inProgress', { steps: [1, 2] })]).percent).toBe(75);
      expect(view([task('1', 'done'), task('2', 'done'), task('3', 'pending')]).percent).toBe(67);
    });

    it('never goes over 100', () => {
      expect(view([task('1', 'inProgress', { steps: [50, 2] })]).percent).toBe(100);
      expect(view([task('1', 'done'), task('2', 'done')]).percent).toBe(100);
    });
  });

  describe('what a task waits for', () => {
    it('lists, for a task to do, the ranks of the tasks it waits for that are not done', () => {
      const v = view([
        task('a', 'pending'),
        task('b', 'done'),
        task('c', 'inProgress'),
        task('d', 'pending'),
        task('e', 'pending', { blockedBy: ['d', 'a', 'b'] }),
      ]);
      // In the order of the list; the one done (b, 02) is not waited for any more.
      expect(v.rows[4].after).toBe('01, 04');
    });

    it('is said for a task to do only', () => {
      const v = view([
        task('1', 'pending'),
        task('2', 'inProgress', { blockedBy: ['1'] }),
        task('3', 'done', { blockedBy: ['1'] }),
        task('4', 'pending', { blockedBy: ['1'] }),
      ]);
      expect(v.rows.map((r) => r.after)).toEqual([undefined, undefined, undefined, '01']);
    });

    it('is not said when every task it waited for is done, nor for one that is not in the list', () => {
      const v = view([task('1', 'done'), task('2', 'pending', { blockedBy: ['1'] }), task('3', 'pending', { blockedBy: ['gone', '9'] })]);
      expect(v.rows[1].after).toBeUndefined();
      expect(v.rows[2].after).toBeUndefined();
    });

    it('names a task by its rank even while its id is the call that made it', () => {
      const v = view([task('toolu_01', 'pending'), task('toolu_02', 'pending', { blockedBy: ['toolu_01'] })]);
      expect(v.rows[1].after).toBe('01');
    });
  });

  describe('the subagent on a task', () => {
    const tasks = [task('1', 'inProgress', { active: 'Lance les tests' }), task('2', 'inProgress'), task('3', 'pending')];

    it('is the one running that is linked to it, with its model and what it does now', () => {
      const v = view(tasks, {
        agents: [sub('s1', { title: 'visuel-produit', model: 'sonnet', planTask: '1', doing: 'Édite src/Hero.tsx' })],
      });
      expect(v.rows[0].sub).toEqual({ name: 'visuel-produit', model: 'sonnet' });
      expect(v.rows[0].step).toBe('Édite src/Hero.tsx');
      expect(v.rows[1].sub).toBeUndefined();
    });

    it('gives no model when the call named none', () => {
      const v = view(tasks, { agents: [sub('s1', { planTask: '1' })] });
      expect(v.rows[0].sub).toEqual({ name: 'sous-agent s1' });
      expect(v.rows[0].sub).not.toHaveProperty('model');
    });

    it('falls back to the task’s own words for what it does, and to nothing', () => {
      const idle = view(tasks, { agents: [sub('s1', { planTask: '1' })] });
      expect(idle.rows[0].step).toBe('Lance les tests');
      expect(idle.rows[1].step).toBeUndefined();
      expect(view(tasks).rows[0].step).toBe('Lance les tests');
    });

    it('is shown only while it runs', () => {
      for (const status of ['done', 'failed', 'stopped', 'interrupted'] as const) {
        const v = view(tasks, { agents: [sub('s1', { status, planTask: '1', doing: 'Édite src/a.ts' })] });
        expect(v.rows[0].sub).toBeUndefined();
        expect(v.rows[0].step).toBe('Lance les tests');
      }
    });

    it('tells nothing of what a task to do or done does', () => {
      const v = view([task('1', 'done', { active: 'Fini' }), task('2', 'pending', { active: 'Plus tard' })]);
      expect(v.rows.map((r) => r.step)).toEqual([undefined, undefined]);
    });

    it('leaves the words of a blocked task to the window, which says it waits for an answer', () => {
      const v = view(
        [task('1', 'inProgress', { active: 'Migre la base' })],
        { agents: [sub('s1', { planTask: '1', doing: 'Lance prisma' })] },
        { status: 'waiting' },
      );
      expect(v.rows[0].state).toBe('block');
      expect(v.rows[0].step).toBeUndefined();
    });

    it('counts and names every subagent running, linked to a task or not', () => {
      const v = view(tasks, {
        agents: [
          sub('s1', { title: 'A', planTask: '1' }),
          sub('s2', { title: 'B' }),
          sub('s3', { title: 'C', status: 'done' }),
          sub('s4', { title: 'D', status: 'interrupted' }),
        ],
        launched: 4,
      });
      expect(v.subsRunning).toBe(2);
      expect(v.subDots).toEqual([{ name: 'A' }, { name: 'B' }]);
    });
  });

  describe('the title', () => {
    it('is the plan’s, else the ticket’s, else the agent’s name', () => {
      const a = agent({ name: 'refacto-auth', plan: plan([task('1', 'pending')], { title: 'Refonte du hero' }) });
      expect(planView(a, ticket({ title: 'Ajouter le fichier' }))!.title).toBe('Refonte du hero');
      const noTitle = agent({ name: 'refacto-auth', plan: plan([task('1', 'pending')]) });
      expect(planView(noTitle, ticket({ title: 'Ajouter le fichier' }))!.title).toBe('Ajouter le fichier');
      expect(planView(noTitle)!.title).toBe('refacto-auth');
    });

    it('skips a title that is blank', () => {
      const a = agent({ name: 'refacto-auth', plan: plan([task('1', 'pending')], { title: '   ' }) });
      expect(planView(a, ticket({ title: '' }))!.title).toBe('refacto-auth');
      expect(planView(a, ticket({ title: '  Le ticket ' }))!.title).toBe('Le ticket');
    });

    it('names the plan file only when the tasks come from one', () => {
      expect(view([task('1', 'pending')], { source: 'plan', planFile: 'docs/superpowers/plans/x.md' }).file).toBe(
        'docs/superpowers/plans/x.md',
      );
      // An agent that keeps its own list and follows a plan: the list is the agent's, the file is not where it comes from.
      expect(view([task('1', 'pending')], { source: 'tools', planFile: 'docs/superpowers/plans/x.md' }).file).toBeUndefined();
      expect(view([task('1', 'pending')], { source: 'plan' }).file).toBeUndefined();
    });
  });

  describe('the line of the agent’s card', () => {
    it('tells the tasks done, the tasks, the subagents running and how far the plan is', () => {
      const v = view([task('1', 'done'), task('2', 'inProgress', { steps: [1, 2] }), task('3', 'pending'), task('4', 'pending')], {
        agents: [sub('s1'), sub('s2'), sub('s3', { status: 'done' })],
      });
      expect(v.mini).toEqual({ done: 1, total: 4, subs: 2, percent: 38 });
    });
  });
});
