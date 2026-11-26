// The subagents of an agent, as the window shows them: under its card in the sidebar, and each one's conversation in
// its own view (ESC-13). The backend gives the facts: the rows of the plan (`Agent.plan.agents`, the running ones and
// the latest ended), and the conversation, where the items of a subagent's thread name its call (`parent`), its
// requests name it too (`sub`), and its call keeps its prompt, its report and what it was once it ended (`run`).

import type { Agent, ConvItem, RunStatus, SubAgent, ToolItem } from './types';

/** A call that launches a subagent: Claude Code names its tool `Agent`, `Task` in other versions. */
export const isSubagentCall = (item: ConvItem | undefined): item is ToolItem =>
  item?.kind === 'tool' && (item.name === 'Agent' || item.name === 'Task');

/** A row of the sidebar's list: how deep it is, a subagent of a subagent being one deeper than its parent. */
export type SubRow = SubAgent & { depth: number };

/** The deepest a row is drawn: a chain of parents longer than that is no deeper. */
const MAX_DEPTH = 4;

/**
 * The subagents under the card of `agent`: those running, in the plan's order, then those that ended, the latest first.
 * Null for an agent that never had one (or whose plan was reset since).
 */
export function sidebarSubs(agent: Agent): { running: SubRow[]; finished: SubRow[] } | null {
  const rows = agent.plan?.agents ?? [];
  if (!rows.length) return null;
  const byId = new Map(rows.map((r) => [r.id, r]));
  const depth = (row: SubAgent) => {
    let d = 0;
    for (let p = row.parent; p && byId.has(p) && d < MAX_DEPTH; p = byId.get(p)!.parent) d++;
    return d;
  };
  const withDepth = (r: SubAgent): SubRow => ({ ...r, depth: depth(r) });
  return {
    running: rows.filter((r) => r.status === 'running').map(withDepth),
    finished: rows
      .filter((r) => r.status !== 'running')
      .sort((a, b) => (b.endedAt ?? 0) - (a.endedAt ?? 0))
      .map(withDepth),
  };
}

/** What a subagent's view shows in its head and at its end. */
export interface SubagentInfo {
  /** The call that launched it, its id everywhere. */
  call: string;
  title: string;
  /** Its type ("Explore"…). */
  kind?: string;
  /** The model its call asked for, as written. */
  model?: string;
  /** Its « consigne »: the prompt of its call, as the backend kept it (16 000 bytes of UTF-8 at most). */
  prompt?: string;
  /** Null when nothing tells it: a call the plan forgot, launched in the background, with no end kept. */
  status: RunStatus | null;
  tokens?: number;
  tools?: number;
  startedAt?: number;
  endedAt?: number;
  /** What it handed back: its report, or the result of a subagent in the foreground. */
  report?: string;
  /** Its task, once Claude Code told it: what « Arrêter » stops and what a message names. */
  taskId?: string;
  /** In the background: it goes on after the turn. */
  background?: boolean;
}

const word = (v: unknown) => (typeof v === 'string' && v.trim() ? v.trim() : undefined);

/** The title of a call, as the plan names a subagent: its description, its name, its type, else « Agent ». */
export const callTitle = (item: ToolItem) =>
  word(item.input?.description) ?? word(item.input?.name) ?? word(item.input?.subagent_type) ?? 'Agent';

/** A launch in the background answers at once with these words: no report. */
const launched = (text: string | undefined) => !!text && /^\s*Async agent launched/.test(text);

/** The status a call says by itself, without a row nor a `run`: its result, if it ended in the foreground. */
function callStatus(item: ToolItem): RunStatus | null {
  if (item.status === 'error') return 'failed';
  if (item.status === 'interrupted') return 'interrupted';
  if (item.status === 'running') return 'running';
  return launched(item.result?.text) ? null : 'done';
}

/** What is known of the subagent `call` of `agent`, from its row of the plan and its call in `items`; null when nothing is. */
export function subagentInfo(agent: Agent, items: ConvItem[], call: string): SubagentInfo | null {
  const row = agent.plan?.agents.find((a) => a.id === call);
  const found = items.find((i) => i.id === call);
  const item = isSubagentCall(found) ? found : undefined;
  if (!row && !item) return null;
  const run = item?.run;
  // The plan keeps every row that runs: a call it has no row of is not running (its process went with the app, say),
  // whatever its item was left at.
  const own = item ? callStatus(item) : null;
  const status = row?.status ?? run?.status ?? (own === 'running' ? 'interrupted' : own);
  const result = item?.status === 'ok' && !launched(item.result?.text) ? word(item.result?.text) : undefined;
  const info: SubagentInfo = {
    call,
    title: row?.title ?? callTitle(item!),
    status,
  };
  const set = <K extends keyof SubagentInfo>(k: K, v: SubagentInfo[K] | undefined | null) => {
    if (v !== undefined && v !== null) info[k] = v;
  };
  set('kind', row?.kind ?? word(item?.input?.subagent_type));
  set('model', row?.model ?? run?.model ?? word(item?.input?.model));
  set('prompt', word(item?.input?.prompt));
  set('tokens', row?.tokens ?? run?.tokens);
  set('tools', row ? row.tools : run?.tools);
  set('startedAt', row?.startedAt ?? run?.startedAt);
  set('endedAt', row?.endedAt ?? run?.endedAt);
  set('report', word(item?.report) ?? result);
  set('taskId', row?.taskId);
  set('background', row?.background);
  return info;
}

/** How long it ran, in ms: until `now` while it runs; null when its start or its end is not known. */
export function subDuration(info: Pick<SubagentInfo, 'status' | 'startedAt' | 'endedAt'>, now: number): number | null {
  if (info.startedAt === undefined) return null;
  if (info.status === 'running') return Math.max(0, now - info.startedAt);
  return info.endedAt === undefined ? null : Math.max(0, info.endedAt - info.startedAt);
}

/** What the view of the subagent `call` draws, in order: the items of its thread, and the requests it made (which the agent's conversation shows too). */
export function threadOf(items: ConvItem[], call: string): ConvItem[] {
  return items.filter((i) => i.parent === call || ((i.kind === 'permission' || i.kind === 'question') && i.sub === call));
}

/** The longest chain of subagents a breadcrumb follows. */
const MAX_CRUMBS = 10;

/**
 * The subagents from the agent's own thread down to `call`, outermost first: what the breadcrumb names after the agent.
 * The chain stops at a parent that neither the plan (a row) nor the conversation (a call) knows: a link to it would lead
 * to an empty view. `call` itself is always named, it is the view the reader is in.
 */
export function crumbs(agent: Agent, items: ConvItem[], call: string): { call: string; title: string }[] {
  const chain: { call: string; title: string }[] = [];
  const seen = new Set<string>();
  for (let c: string | null | undefined = call; c && !seen.has(c) && chain.length < MAX_CRUMBS;) {
    seen.add(c);
    const row = agent.plan?.agents.find((a) => a.id === c);
    const found = items.find((i) => i.id === c);
    const item = isSubagentCall(found) ? found : undefined;
    if (c !== call && !row && !item) break;
    chain.unshift({ call: c, title: row?.title ?? (item ? callTitle(item) : 'Agent') });
    c = item?.parent ?? row?.parent;
  }
  return chain;
}

/**
 * One can write to a subagent while it runs, and in the background: the plan never opens an ended row again, so one taken
 * up again by a message would still say it ended; in the foreground the agent waits for it, and would pass the message on
 * once it ended.
 */
export const canWriteSub = (agent: Agent, sub: { status: RunStatus | null; taskId?: string; background?: boolean }): boolean =>
  sub.status === 'running' && !!sub.taskId && sub.background === true && !agent.terminal && !agent.archived;
