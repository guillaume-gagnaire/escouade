import { defineZone } from '../types';

// The plan an agent follows: the banner of its conversation, the line of its card. The titles of the tasks, the
// names of the subagents and what they do are the agent’s own words: shown as they come, never translated.
export default defineZone('plan', {
  badge: 'Plan',
  badgeSubs: 'Subagents',
  tasks: { one: '{done}/{count} task', other: '{done}/{count} tasks' },
  percent: '{percent}%',
  subsRunning: { one: '{count} subagent running', other: '{count} subagents running' },
  toggle: 'Plan: {title}',
  source: 'Plan: {file}',
  list: 'Plan tasks',
  state: { done: 'Done', inProgress: 'In progress', todo: 'To do', blocked: 'Blocked' },
  subOn: '↳ {name}',
  after: 'after {list}',
  waiting: 'Waiting for your answer',
  stepsOf: { one: '{done}/{count} step', other: '{done}/{count} steps' },
  grip: 'Drag to resize · double-click to reset',
  gripLabel: 'Height of the task list',
  mini: 'Plan {done}/{total}',
  miniSubs: { one: 'Plan {done}/{total} · {count} subagent', other: 'Plan {done}/{total} · {count} subagents' },
});
