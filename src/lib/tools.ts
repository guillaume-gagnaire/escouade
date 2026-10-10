// One-line summaries of tool calls for the compact rows of the conversation.

import { fInt, relPath } from './format';
import { t, type Key } from './i18n';
import type { ConvItem, ToolItem } from './types';

const lines = (s: string | undefined) => (s ? s.split('\n').length : 0);

function lastLine(s: string | undefined): string {
  const l = (s ?? '').trim().split('\n').filter(Boolean);
  return l.length ? l[l.length - 1].slice(0, 120) : '';
}

/** How Claude Code names the tools of Escouade's own MCP server: `mcp__escouade__<tool>`. */
const ESCOUADE = 'mcp__escouade__';

/** The name in words of each tool of Escouade's server (keys only: the text is read where it is shown). */
const ESCOUADE_TOOLS: Record<string, Key> = {
  list_projects: 'mcp.tools.listProjects',
  list_agents: 'mcp.tools.listAgents',
  list_tickets: 'mcp.tools.listTickets',
  get_ticket: 'mcp.tools.getTicket',
  get_usage: 'mcp.tools.getUsage',
  get_agent_summary: 'mcp.tools.getAgentSummary',
  create_ticket: 'mcp.tools.createTicket',
  update_ticket: 'mcp.tools.updateTicket',
  move_ticket: 'mcp.tools.moveTicket',
  start_ticket: 'mcp.tools.startTicket',
  create_agent: 'mcp.tools.createAgent',
  send_message: 'mcp.tools.sendMessage',
  stop_agent: 'mcp.tools.stopAgent',
  report_progress: 'mcp.tools.reportProgress',
  split_ticket: 'mcp.tools.splitTicket',
};

/**
 * What a tool of Escouade is about, first found first (`ESCOUADE_MAIN_ARGS` of the backend's `tool_arg` reads them in
 * the same order): the ticket it makes, the ticket or the agent it acts on, what an agent it starts is told, the line an
 * agent reports, the project it reads.
 */
const ESCOUADE_MAIN_ARGS = ['title', 'ticket', 'agent', 'message', 'line', 'project'];

/** The most characters of a value the permission card of a tool of Escouade shows. */
export const ARG_MAX = 2000;

/**
 * The task tools of Claude Code that only read: the list or one task. They are not what the agent does to its plan, only a
 * look at it; the conversation keeps them in the background.
 */
export const isQuietTool = (name: string) => name === 'TaskList' || name === 'TaskGet';

/** What became of a task a `TaskUpdate` changes, in words (keys only: the text is read where it is shown). */
const TASK_STATUS: Record<string, Key> = {
  pending: 'conv.tools.taskStatus.pending',
  in_progress: 'conv.tools.taskStatus.inProgress',
  completed: 'conv.tools.taskStatus.completed',
  deleted: 'conv.tools.taskStatus.deleted',
};

/** A tool of Escouade's own MCP server. */
export const isEscouadeTool = (name: string) => name.startsWith(ESCOUADE);

export function toolLabel(name: string): string {
  if (isEscouadeTool(name)) {
    const tool = name.slice(ESCOUADE.length);
    const key = Object.hasOwn(ESCOUADE_TOOLS, tool) ? ESCOUADE_TOOLS[tool] : null;
    return t('mcp.tools.named', { tool: key ? t(key) : tool });
  }
  if (name.startsWith('mcp__')) {
    const [, server, tool] = name.split('__');
    return `${server}·${tool ?? ''}`;
  }
  if (name === 'Task' || name === 'Agent') return 'Agent';
  return name;
}

/** `text` in `ARG_MAX` characters at most (as they are read: an emoji is one), « … » after it when it was longer. */
function capped(text: string): string {
  const chars = [...text];
  return chars.length > ARG_MAX ? `${chars.slice(0, ARG_MAX).join('')}…` : text;
}

/** A value as the permission card reads it: a text as written, a list of texts one per line, anything else as JSON. */
function inClear(v: unknown): string {
  if (typeof v === 'string') return v;
  if (typeof v === 'number' || typeof v === 'boolean') return String(v);
  if (Array.isArray(v) && v.every((x) => typeof x === 'string' || typeof x === 'number')) return v.join('\n');
  return JSON.stringify(v, null, 2);
}

/**
 * Each argument a tool of Escouade is given, in its order, as its permission card lists them: its name and its value
 * in clear (`inClear`), `ARG_MAX` characters at most, and `hidden`, how many more there were, when the value was cut.
 * An argument left out (null) is not listed.
 */
export function escouadeArgs(input: Record<string, unknown>): { name: string; value: string; hidden?: number }[] {
  return Object.entries(input ?? {})
    .filter(([, v]) => v !== null && v !== undefined)
    .map(([name, v]) => {
      const text = inClear(v);
      const hidden = [...text].length - ARG_MAX;
      return { name, value: capped(text), ...(hidden > 0 ? { hidden } : {}) };
    });
}

/**
 * Tools whose summary (`toolArg`, and `tool_arg` in the backend) says what a permission asks: the command, the file,
 * the search, the address. Another tool's says part of it at most (an MCP tool's first field, a subagent's title, a
 * skill's name): asked for permission, it is read in the conversation, where the whole request is.
 */
export const SUMMED_UP: ReadonlySet<string> = new Set([
  'Bash',
  'PowerShell',
  'SlashCommand',
  'Read',
  'Edit',
  'Write',
  'MultiEdit',
  'NotebookEdit',
  'Grep',
  'Glob',
  'WebFetch',
  'WebSearch',
]);

export function toolArg(tool: ToolItem, cwd: string): string {
  const i = tool.input ?? {};
  const p = (x: unknown) => (typeof x === 'string' ? relPath(cwd, x) : '');
  if (isEscouadeTool(tool.name)) {
    const main = ESCOUADE_MAIN_ARGS.map((k) => i[k]).find((v) => typeof v === 'string');
    return typeof main === 'string' ? main.slice(0, 160) : '';
  }
  switch (tool.name) {
    case 'Bash':
    case 'PowerShell':
      return i.command ?? '';
    case 'Read':
    case 'Edit':
    case 'Write':
    case 'MultiEdit':
      return p(i.file_path);
    case 'NotebookEdit':
      return p(i.notebook_path);
    case 'Grep':
      return `${i.pattern ?? ''}${i.path ? '  ' + p(i.path) : ''}`;
    case 'Glob':
      return `${i.pattern ?? ''}${i.path ? '  ' + p(i.path) : ''}`;
    case 'WebFetch':
      return i.url ?? '';
    case 'WebSearch':
      return i.query ?? '';
    case 'Task':
    case 'Agent':
      return i.description ?? i.subagent_type ?? '';
    case 'TodoWrite':
      return Array.isArray(i.todos) ? t('conv.tools.tasks', { count: i.todos.length }) : '';
    case 'TaskCreate':
      return typeof i.subject === 'string' && i.subject ? t('conv.tools.taskCreate', { subject: i.subject }) : '';
    case 'TaskUpdate': {
      // The id is a number as text for Claude Code, a number for some callers.
      if (typeof i.taskId !== 'string' && typeof i.taskId !== 'number') return '';
      const status =
        typeof i.status === 'string' && Object.hasOwn(TASK_STATUS, i.status) ? TASK_STATUS[i.status] : 'conv.tools.taskStatus.changed';
      return t('conv.tools.taskUpdate', { id: String(i.taskId), status: t(status) });
    }
    case 'Skill':
      return i.skill ?? i.command ?? '';
    case 'SlashCommand':
      return i.command ?? '';
    default: {
      const first = Object.values(i).find((v) => typeof v === 'string');
      return typeof first === 'string' ? first.slice(0, 160) : '';
    }
  }
}

export function toolResultSummary(tool: ToolItem): string {
  const r = tool.result;
  if (tool.status === 'running') return '';
  if (tool.status === 'interrupted') return t('conv.tools.interrupted');
  if (!r) return '';
  if (r.isError) return lastLine(r.text) || t('conv.tools.error');
  switch (tool.name) {
    case 'Read': {
      const n = lines(r.text);
      return t('conv.tools.lines', { count: n, n: fInt(n) });
    }
    case 'Grep':
    case 'Glob': {
      const n = (r.text ?? '').split('\n').filter(Boolean).length;
      return n ? t('conv.tools.results', { count: n, n: fInt(n) }) : t('conv.tools.noResults');
    }
    case 'Bash':
    case 'PowerShell':
      return lastLine(r.text) || '✓';
    case 'Task':
    case 'Agent':
      return t('conv.tools.done');
    case 'TodoWrite':
    // The line says what was done; what a list or a task read says is long, and the banner shows it better.
    case 'TaskCreate':
    case 'TaskUpdate':
    case 'TaskList':
    case 'TaskGet':
      return '';
    default:
      return lastLine(r.text) ? lastLine(r.text).slice(0, 80) : '✓';
  }
}

export function hasDiff(tool: ToolItem): boolean {
  return !!tool.result && typeof tool.result.add === 'number';
}

export interface FileEdit {
  path: string;
  add: number;
  del: number;
}

/** The files each turn edited (by turn id), subagents included, with their line counts summed. */
export function editsByTurn(items: ConvItem[], cwd: string): Map<string, FileEdit[]> {
  const out = new Map<string, FileEdit[]>();
  let current = new Map<string, FileEdit>();
  for (const it of items) {
    if (it.kind === 'turn') {
      out.set(it.id, [...current.values()]);
      current = new Map();
    } else if (it.kind === 'tool' && hasDiff(it) && !it.result!.isError) {
      const file = it.result!.filePath ?? it.input?.file_path;
      if (typeof file !== 'string') continue;
      const path = relPath(cwd, file);
      const e = current.get(path) ?? { path, add: 0, del: 0 };
      e.add += it.result!.add ?? 0;
      e.del += it.result!.del ?? 0;
      current.set(path, e);
    }
  }
  return out;
}
