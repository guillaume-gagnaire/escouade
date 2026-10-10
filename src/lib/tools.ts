// One-line summaries of tool calls for the compact rows of the conversation.

import { relPath } from './format';
import type { ConvItem, ToolItem } from './types';

const lines = (s: string | undefined) => (s ? s.split('\n').length : 0);

function lastLine(s: string | undefined): string {
  const l = (s ?? '').trim().split('\n').filter(Boolean);
  return l.length ? l[l.length - 1].slice(0, 120) : '';
}

export function toolLabel(name: string): string {
  if (name.startsWith('mcp__')) {
    const [, server, tool] = name.split('__');
    return `${server}·${tool ?? ''}`;
  }
  if (name === 'Task' || name === 'Agent') return 'Agent';
  return name;
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

export function toolArg(t: ToolItem, cwd: string): string {
  const i = t.input ?? {};
  const p = (x: unknown) => (typeof x === 'string' ? relPath(cwd, x) : '');
  switch (t.name) {
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
      return Array.isArray(i.todos) ? `${i.todos.length} tâches` : '';
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

export function toolResultSummary(t: ToolItem): string {
  const r = t.result;
  if (t.status === 'running') return '';
  if (t.status === 'interrupted') return 'interrompu';
  if (!r) return '';
  if (r.isError) return lastLine(r.text) || 'erreur';
  switch (t.name) {
    case 'Read':
      return `${lines(r.text)} lignes`;
    case 'Grep':
    case 'Glob': {
      const n = (r.text ?? '').split('\n').filter(Boolean).length;
      return n ? `${n} résultat${n > 1 ? 's' : ''}` : 'aucun résultat';
    }
    case 'Bash':
    case 'PowerShell':
      return lastLine(r.text) || '✓';
    case 'Task':
    case 'Agent':
      return 'terminé';
    case 'TodoWrite':
      return '';
    default:
      return lastLine(r.text) ? lastLine(r.text).slice(0, 80) : '✓';
  }
}

export function hasDiff(t: ToolItem): boolean {
  return !!t.result && typeof t.result.add === 'number';
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
