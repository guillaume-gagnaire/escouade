import { describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { editsByTurn, hasDiff, toolArg, toolLabel, toolResultSummary } from './tools';
import type { ConvItem, ToolItem, TurnItem } from './types';

const CWD = 'C:\\code\\app';
const tool = (name: string, input: Record<string, unknown>, extra: Partial<ToolItem> = {}): ToolItem => ({
  kind: 'tool',
  id: 't',
  name,
  input,
  status: 'ok',
  ts: 0,
  ...extra,
});

describe('toolArg', () => {
  it.each([
    ['Bash', { command: 'npm test' }, 'npm test'],
    ['Read', { file_path: 'C:\\code\\app\\src\\a.ts' }, 'src/a.ts'],
    ['Edit', { file_path: 'C:\\code\\app\\README.md' }, 'README.md'],
    ['Grep', { pattern: 'TODO', path: 'C:\\code\\app\\src' }, 'TODO  src'],
    ['Glob', { pattern: '**/*.rs' }, '**/*.rs'],
    ['WebFetch', { url: 'https://example.com' }, 'https://example.com'],
    ['Task', { description: 'Explorer le code' }, 'Explorer le code'],
    ['TodoWrite', { todos: [{}, {}, {}] }, '3 tâches'],
    ['mcp__github__create_issue', { title: 'Bug' }, 'Bug'],
  ])('%s', (name, input, want) => expect(toolArg(tool(name, input), CWD)).toBe(want));
});

describe('toolLabel', () => {
  it('shortens MCP tool names and renames subagents', () => {
    expect(toolLabel('mcp__github__create_issue')).toBe('github·create_issue');
    expect(toolLabel('Task')).toBe('Agent');
    expect(toolLabel('Read')).toBe('Read');
  });
});

describe('toolResultSummary', () => {
  it('counts the lines read', () => {
    expect(toolResultSummary(tool('Read', {}, { result: { text: 'a\nb\nc', isError: false } }))).toBe('3 lignes');
  });
  it('counts search results, singular and empty', () => {
    expect(toolResultSummary(tool('Grep', {}, { result: { text: 'a.ts\nb.ts\n', isError: false } }))).toBe('2 résultats');
    expect(toolResultSummary(tool('Glob', {}, { result: { text: 'a.ts', isError: false } }))).toBe('1 résultat');
    expect(toolResultSummary(tool('Grep', {}, { result: { text: '', isError: false } }))).toBe('aucun résultat');
  });
  it('shows the last output line of a command', () => {
    expect(toolResultSummary(tool('Bash', {}, { result: { text: 'building…\n3 passed · 1 failed\n', isError: false } }))).toBe(
      '3 passed · 1 failed',
    );
  });
  it('shows the error for failed tools', () => {
    expect(toolResultSummary(tool('Bash', {}, { status: 'error', result: { text: 'npm ERR! missing script', isError: true } }))).toBe(
      'npm ERR! missing script',
    );
  });
  it('is empty while running', () => {
    expect(toolResultSummary(tool('Bash', {}, { status: 'running' }))).toBe('');
  });
  it('says when the turn stopped before the tool returned', () => {
    expect(toolResultSummary(tool('Bash', {}, { status: 'interrupted' }))).toBe('interrompu');
  });
});

describe('hasDiff', () => {
  it('is true only when the result carries line counts', () => {
    expect(hasDiff(tool('Edit', {}, { result: { isError: false, add: 0, del: 3 } }))).toBe(true);
    expect(hasDiff(tool('Read', {}, { result: { isError: false, text: 'x' } }))).toBe(false);
  });
});

describe('editsByTurn', () => {
  const edit = (id: string, file: string, add: number, del: number, extra: Partial<ToolItem> = {}): ToolItem =>
    tool('Edit', { file_path: `${CWD}\\${file}` }, { id, result: { isError: false, add, del }, ...extra });
  const end = (id: string): TurnItem => ({
    kind: 'turn',
    id,
    ts: 0,
    durationMs: 1,
    cost: 0,
    tokens: 0,
    isError: false,
    interrupted: false,
    error: null,
  });

  it('sums each turn’s edits per file, subagents’ included, in the order first touched', () => {
    const items: ConvItem[] = [
      edit('e1', 'src\\old.ts', 1, 0),
      end('r1'),
      edit('e2', 'src\\b.ts', 3, 1),
      tool('Read', { file_path: `${CWD}\\src\\a.ts` }, { id: 'x' }),
      edit('e3', 'src\\a.ts', 2, 0, { parent: 'task1' }),
      edit('e4', 'src\\b.ts', 1, 1),
      tool(
        'Write',
        { file_path: `${CWD}\\notes.md` },
        { id: 'w', result: { isError: false, add: 4, del: 0, filePath: `${CWD}\\notes.md` } },
      ),
      // A failed edit changed nothing.
      edit('e5', 'src\\c.ts', 9, 9, { status: 'error', result: { isError: true, text: 'no match' } }),
      end('r2'),
    ];
    const edits = editsByTurn(items, CWD);
    expect(edits.get('r1')).toEqual([{ path: 'src/old.ts', add: 1, del: 0 }]);
    expect(edits.get('r2')).toEqual([
      { path: 'src/b.ts', add: 4, del: 2 },
      { path: 'src/a.ts', add: 2, del: 0 },
      { path: 'notes.md', add: 4, del: 0 },
    ]);
  });
});

describe('the summaries of the tools in English', () => {
  it('count lines, results and tasks with the plural of English, and group the thousands', () => {
    setLang('en');
    const read = (n: number) => tool('Read', {}, { result: { text: Array.from({ length: n }, () => 'x').join('\n'), isError: false } });
    expect(toolResultSummary(read(1))).toBe('1 line');
    expect(toolResultSummary(read(3))).toBe('3 lines');
    expect(toolResultSummary(read(1234))).toBe('1,234 lines');
    expect(toolResultSummary(tool('Grep', {}, { result: { text: 'a.ts', isError: false } }))).toBe('1 result');
    expect(toolResultSummary(tool('Glob', {}, { result: { text: 'a.ts\nb.ts', isError: false } }))).toBe('2 results');
    expect(toolResultSummary(tool('Grep', {}, { result: { text: '', isError: false } }))).toBe('no results');
    expect(toolArg(tool('TodoWrite', { todos: [{}] }), CWD)).toBe('1 task');
    expect(toolArg(tool('TodoWrite', { todos: [{}, {}, {}] }), CWD)).toBe('3 tasks');
  });

  it('say in English that a tool was interrupted, failed or is done, and keep the names of the tools', () => {
    setLang('en');
    expect(toolResultSummary(tool('Bash', {}, { status: 'interrupted' }))).toBe('interrupted');
    expect(toolResultSummary(tool('Bash', {}, { status: 'error', result: { text: '', isError: true } }))).toBe('error');
    expect(toolResultSummary(tool('Task', {}, { result: { text: 'x', isError: false } }))).toBe('done');
    expect(toolLabel('Read')).toBe('Read');
    expect(toolLabel('mcp__github__create_issue')).toBe('github·create_issue');
  });
});
