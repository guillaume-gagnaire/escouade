// What the window shows of an agent's progress through a plan: the banner of the conversation and the line of the
// agent's card. The backend gives the facts (`Agent.plan`: tasks, subagents, workflows); what is worked out here is what
// the facts cannot say: the rank of a task, whether it is blocked, how far it is, which one to look at.

import type { Agent, Ticket } from './types';

export type PlanRowState = 'done' | 'run' | 'todo' | 'block';

export interface PlanRow {
  /** Its rank in the list, on two digits at least ("01"): what the tasks that wait for it call it. */
  n: string;
  /** The task's id, for the facts (never shown: Claude Code's id for it may still be the id of the call that made it). */
  id: string;
  title: string;
  state: PlanRowState;
  /** How far it is, 0 to 100; null for a task under way that nothing measures. */
  percent: number | null;
  /** The steps of the task a plan file counts: done, in all. */
  steps?: [done: number, total: number];
  /** What a task under way does now, in the agent's words (a subagent's last tool, else the task's own). */
  step?: string;
  /** The subagent running on it. */
  sub?: { name: string; model?: string };
  /** The ranks of the tasks it waits for ("03, 05"). */
  after?: string;
  /** The one to look at: the list centres on it. */
  focus: boolean;
}

export interface PlanViewModel {
  title: string;
  /** The plan file the tasks come from, when they come from one. */
  file?: string;
  done: number;
  total: number;
  /** The whole plan's, 0 to 100. */
  percent: number;
  subsRunning: number;
  /** The subagents running, by their titles. */
  subDots: { name: string }[];
  rows: PlanRow[];
  focusId: string | null;
  /** The line of the agent's card; none without tasks. */
  mini: { done: number; total: number; subs: number; percent: number } | null;
}

/** Two digits at least: "01", "12", "100". */
const rank = (i: number) => String(i + 1).padStart(2, '0');

/**
 * How far a task under way is by its steps, 0 to 100; null when nothing counts them, or none is done yet (a task begun is
 * shown at "0 %" beside a spinner as stalled: it is under way, nobody knows how far).
 */
function measured(steps: [number, number] | undefined): number | null {
  if (!steps || !(steps[1] > 0) || !(steps[0] > 0)) return null;
  return Math.max(0, Math.min(100, Math.round((steps[0] / steps[1]) * 100)));
}

const worded = (text: string | undefined) => text?.trim() || undefined;

/**
 * What to show of the agent's plan, or null when there is nothing: no task and no subagent running. It is keyed on the
 * tasks and the subagents running, not on the plan being there (an empty `TodoWrite` leaves an empty plan) nor on where
 * the tasks come from. Without tasks but with subagents running, the model has no rows: the banner shows who works.
 */
export function planView(agent: Agent, ticket?: Ticket): PlanViewModel | null {
  const plan = agent.plan;
  if (!plan) return null;
  const running = plan.agents.filter((s) => s.status === 'running');
  if (!plan.tasks.length && !running.length) return null;

  // The agent waits for an answer or a permission: the first task under way waits with it. That is all the stream says.
  const waiting = agent.status === 'waiting';
  const blocked = waiting ? plan.tasks.findIndex((x) => x.status === 'inProgress') : -1;
  const ranks = new Map<string, number>();
  plan.tasks.forEach((x, i) => {
    if (!ranks.has(x.id)) ranks.set(x.id, i);
  });

  const rows: PlanRow[] = plan.tasks.map((x, i) => {
    const state: PlanRowState = x.status === 'done' ? 'done' : x.status === 'pending' ? 'todo' : i === blocked ? 'block' : 'run';
    const sub = running.find((s) => s.planTask === x.id);
    // A task done is whole, a task to do has not begun; one under way is as far as its steps say, else nobody knows.
    const percent = state === 'done' ? 100 : state === 'todo' ? 0 : measured(x.steps);
    const row: PlanRow = { n: rank(i), id: x.id, title: x.title, state, percent, focus: false };
    if (x.steps) row.steps = x.steps;
    if (sub) row.sub = { name: sub.title, ...(sub.model ? { model: sub.model } : {}) };
    // The words of a blocked task are the window's: it waits for an answer.
    if (state === 'run') {
      const step = worded(sub?.doing) ?? worded(x.active);
      if (step) row.step = step;
    }
    if (state === 'todo') {
      const waitedFor = (x.blockedBy ?? [])
        .map((id) => ranks.get(id))
        .filter((r): r is number => r !== undefined && plan.tasks[r].status !== 'done')
        .sort((a, b) => a - b);
      const after = [...new Set(waitedFor)].map(rank).join(', ');
      if (after) row.after = after;
    }
    return row;
  });

  // The one to look at: blocked, else under way, else the first to do.
  const focus = rows.find((r) => r.state === 'block') ?? rows.find((r) => r.state === 'run') ?? rows.find((r) => r.state === 'todo');
  if (focus) focus.focus = true;

  const done = rows.filter((r) => r.state === 'done').length;
  const percent = rows.length ? Math.min(100, Math.round(rows.reduce((sum, r) => sum + (r.percent ?? 0), 0) / rows.length)) : 0;
  const file = plan.source === 'plan' ? plan.planFile : undefined;
  return {
    title: worded(plan.title) ?? worded(ticket?.title) ?? agent.name,
    ...(file ? { file } : {}),
    done,
    total: rows.length,
    percent,
    subsRunning: running.length,
    subDots: running.map((s) => ({ name: s.title })),
    rows,
    focusId: focus?.id ?? null,
    mini: rows.length ? { done, total: rows.length, subs: running.length, percent } : null,
  };
}
