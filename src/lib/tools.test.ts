import { describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { editsByTurn, escouadeArgs, hasDiff, isEscouadeTool, isQuietTool, toolArg, toolLabel, toolResultSummary } from './tools';
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

describe('the task tools of Claude Code', () => {
  const arg = (name: string, input: Record<string, unknown>) => toolArg(tool(name, input), CWD);

  it('sum up a task made by its subject', () => {
    expect(arg('TaskCreate', { subject: 'Écrire les tests', description: 'Long texte', activeForm: 'Écrit les tests' })).toBe(
      'Tâche : Écrire les tests',
    );
    expect(arg('TaskCreate', {})).toBe('');
  });

  it('sum up a task changed by its id and what became of it, in words', () => {
    expect(arg('TaskUpdate', { taskId: '3', status: 'completed' })).toBe('Tâche #3 : terminée');
    expect(arg('TaskUpdate', { taskId: '3', status: 'in_progress' })).toBe('Tâche #3 : en cours');
    expect(arg('TaskUpdate', { taskId: '3', status: 'pending' })).toBe('Tâche #3 : à faire');
    expect(arg('TaskUpdate', { taskId: '3', status: 'deleted' })).toBe('Tâche #3 : supprimée');
    // The id may be a number; a change that is no status (a subject, what it waits for) is a change.
    expect(arg('TaskUpdate', { taskId: 12, status: 'completed' })).toBe('Tâche #12 : terminée');
    expect(arg('TaskUpdate', { taskId: '3', subject: 'Autre titre' })).toBe('Tâche #3 : modifiée');
    expect(arg('TaskUpdate', { taskId: '3', addBlockedBy: ['1'] })).toBe('Tâche #3 : modifiée');
    expect(arg('TaskUpdate', { status: 'completed' })).toBe('');
  });

  it('keep the words the agent wrote as they are', () => {
    expect(arg('TaskCreate', { subject: '<b>gras</b> {id}' })).toBe('Tâche : <b>gras</b> {id}');
  });

  it('say nothing of a result: the line says it all, and a list or a task read is long', () => {
    for (const name of ['TaskCreate', 'TaskUpdate', 'TaskList', 'TaskGet']) {
      expect(toolResultSummary(tool(name, {}, { result: { text: 'Task #1 created successfully: x\n- #2 y', isError: false } }))).toBe('');
    }
    // A failure is still said.
    expect(toolResultSummary(tool('TaskUpdate', {}, { status: 'error', result: { text: 'No such task', isError: true } }))).toBe(
      'No such task',
    );
  });

  it('keep the reading of the tasks in the background of the conversation', () => {
    expect(isQuietTool('TaskList')).toBe(true);
    expect(isQuietTool('TaskGet')).toBe(true);
    expect(isQuietTool('TaskCreate')).toBe(false);
    expect(isQuietTool('TaskUpdate')).toBe(false);
    expect(isQuietTool('TodoWrite')).toBe(false);
  });

  it('read in English', () => {
    setLang('en');
    expect(arg('TaskCreate', { subject: 'Write the tests' })).toBe('Task: Write the tests');
    expect(arg('TaskUpdate', { taskId: '3', status: 'in_progress' })).toBe('Task #3: in progress');
    expect(arg('TaskUpdate', { taskId: '3', status: 'completed' })).toBe('Task #3: done');
    expect(arg('TaskUpdate', { taskId: '3', subject: 'x' })).toBe('Task #3: updated');
  });

  it('leave the list of a TodoWrite and the row of a subagent as they were', () => {
    expect(arg('TodoWrite', { todos: [{}, {}] })).toBe('2 tâches');
    expect(arg('Agent', { description: 'Explorer', subagent_type: 'Explore' })).toBe('Explorer');
    expect(toolLabel('Agent')).toBe('Agent');
    expect(toolResultSummary(tool('Agent', {}, { result: { text: 'x', isError: false } }))).toBe('terminé');
  });
});

describe('toolLabel', () => {
  it('shortens MCP tool names and renames subagents', () => {
    expect(toolLabel('mcp__github__create_issue')).toBe('github·create_issue');
    expect(toolLabel('Task')).toBe('Agent');
    expect(toolLabel('Read')).toBe('Read');
  });
});

describe('the tools of Escouade’s own server', () => {
  it('are named in words, after Escouade', () => {
    expect(toolLabel('mcp__escouade__create_ticket')).toBe('Escouade · Créer un ticket');
    expect(toolLabel('mcp__escouade__create_agent')).toBe('Escouade · Lancer un agent');
    expect(toolLabel('mcp__escouade__list_projects')).toBe('Escouade · Lister les projets');
    expect(toolLabel('mcp__escouade__get_agent_summary')).toBe('Escouade · Résumer un agent');
    // One this window does not know yet: as Claude names it.
    expect(toolLabel('mcp__escouade__frobnicate')).toBe('Escouade · frobnicate');
    // Another server's keep their names.
    expect(toolLabel('mcp__github__create_issue')).toBe('github·create_issue');
    expect(isEscouadeTool('mcp__escouade__create_ticket')).toBe(true);
    expect(isEscouadeTool('mcp__escouade_bis__create_ticket')).toBe(false);
    expect(isEscouadeTool('mcp__github__create_issue')).toBe(false);
  });

  it('are summed up by what they act on: the title, the ticket or the agent, as the backend does', () => {
    const arg = (name: string, input: Record<string, unknown>) => toolArg(tool(`mcp__escouade__${name}`, input), CWD);
    expect(arg('create_ticket', { project: 'demo', title: 'Corriger la connexion', description: 'Le jeton expire' })).toBe(
      'Corriger la connexion',
    );
    expect(arg('update_ticket', { description: 'Plus court', ticket: 'DEM-4' })).toBe('DEM-4');
    expect(arg('send_message', { text: 'Relis le test', agent: 'fix-login' })).toBe('fix-login');
    // Without one: what the agent is told, its line, its project.
    expect(arg('create_agent', { project: 'demo', message: 'Écris la doc', worktree: true })).toBe('Écris la doc');
    expect(arg('report_progress', { line: 'Tests verts' })).toBe('Tests verts');
    expect(arg('list_tickets', { project: 'demo', column: 'todo' })).toBe('demo');
    expect(arg('list_projects', {})).toBe('');
  });

  it('give each argument in clear, in its order: a text as written, a list one item per line, anything else as JSON', () => {
    expect(
      escouadeArgs({
        project: 'demo',
        title: 'Corriger la connexion',
        description: 'Le jeton expire.\nIl faut le renouveler.',
        criteria: ['Le test passe', 'La doc suit'],
        after: ['DEM-1'],
        worktree: true,
        position: 2,
        before: null,
        tickets: [{ title: 'A' }],
      }),
    ).toEqual([
      { name: 'project', value: 'demo' },
      { name: 'title', value: 'Corriger la connexion' },
      { name: 'description', value: 'Le jeton expire.\nIl faut le renouveler.' },
      { name: 'criteria', value: 'Le test passe\nLa doc suit' },
      { name: 'after', value: 'DEM-1' },
      { name: 'worktree', value: 'true' },
      { name: 'position', value: '2' },
      { name: 'tickets', value: '[\n  {\n    "title": "A"\n  }\n]' },
    ]);
    expect(escouadeArgs({})).toEqual([]);
  });

  it('give a value of 2,000 characters at most, counted as they are read', () => {
    expect(escouadeArgs({ description: 'x'.repeat(2000) })[0].value).toBe('x'.repeat(2000));
    const [cut] = escouadeArgs({ description: '😀'.repeat(2500) });
    expect([...cut.value]).toHaveLength(2001);
    expect(cut.value.endsWith('😀…')).toBe(true);
  });

  it('count the characters a cut value hides, and say nothing of one that fits', () => {
    expect(escouadeArgs({ description: 'x'.repeat(2000) })).toEqual([{ name: 'description', value: 'x'.repeat(2000) }]);
    const [cut] = escouadeArgs({ description: '😀'.repeat(2500), title: 'court' });
    expect(cut.hidden).toBe(500);
    expect(escouadeArgs({ description: 'a'.repeat(2001) })[0].hidden).toBe(1);
    expect(escouadeArgs({ description: 'court' })[0]).not.toHaveProperty('hidden');
  });

  it('are named in English', () => {
    setLang('en');
    expect(toolLabel('mcp__escouade__create_ticket')).toBe('Escouade · Create a ticket');
    expect(toolLabel('mcp__escouade__create_agent')).toBe('Escouade · Start an agent');
    expect(toolLabel('mcp__escouade__frobnicate')).toBe('Escouade · frobnicate');
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
