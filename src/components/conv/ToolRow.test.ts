import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { ToolItem } from '../../lib/types';
import ToolRow from './ToolRow.svelte';

const CWD = 'C:\\code\\app';
const tool = (over: Partial<ToolItem>): ToolItem => ({ kind: 'tool', id: 't', name: 'Bash', input: {}, status: 'ok', ts: 0, ...over });

describe('ToolRow', () => {
  it('shows an edit with its line counts and expands to the diff', async () => {
    render(ToolRow, {
      item: tool({
        name: 'Edit',
        input: { file_path: 'C:\\code\\app\\src\\auth.ts' },
        result: { isError: false, add: 2, del: 1, patch: [{ oldStart: 4, newStart: 4, lines: ['-old line', '+new line', '+another'] }] },
      }),
      cwd: CWD,
    });
    expect(screen.getByText('src/auth.ts')).toBeInTheDocument();
    expect(screen.getByText('+2')).toBeInTheDocument();
    expect(screen.getByText('−1')).toBeInTheDocument();
    expect(screen.queryByText('new line')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { expanded: false }));
    expect(screen.getByText('new line')).toBeInTheDocument();
    expect(screen.getByText('old line')).toBeInTheDocument();
  });

  it('shows an edit side by side when that diff style is chosen', async () => {
    app.diffSplit = true;
    try {
      render(ToolRow, {
        item: tool({
          name: 'Edit',
          input: { file_path: 'C:\\code\\app\\src\\auth.ts' },
          result: { isError: false, add: 1, del: 1, patch: [{ oldStart: 4, newStart: 4, lines: ['-old line', '+new line'] }] },
        }),
        cwd: CWD,
      });
      await userEvent.click(screen.getByRole('button', { expanded: false }));
      expect(screen.getByText('old line').closest('.srow')).toHaveTextContent('new line');
    } finally {
      app.diffSplit = false;
    }
  });

  it('shows the command output of a Bash call when expanded', async () => {
    render(ToolRow, { item: tool({ input: { command: 'npm test' }, result: { isError: false, text: 'building\n12 passed' } }), cwd: CWD });
    expect(screen.getByText('12 passed')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { expanded: false }));
    expect(screen.getByText('$ npm test')).toBeInTheDocument();
  });

  it('shows a spinner while running and cannot be expanded yet', async () => {
    render(ToolRow, { item: tool({ status: 'running', input: { command: 'npm ci' } }), cwd: CWD });
    expect(screen.getByLabelText('en cours')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /npm ci/ }));
    expect(screen.getByRole('button', { name: /npm ci/ })).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText('$ npm ci')).not.toBeInTheDocument();
  });

  it('lists subagent tools under the Agent call', async () => {
    const child = tool({
      id: 'c1',
      name: 'Read',
      input: { file_path: 'C:\\code\\app\\README.md' },
      parent: 't',
      result: { isError: false, text: 'a\nb' },
    });
    render(ToolRow, {
      item: tool({ name: 'Task', input: { description: 'Explorer', prompt: 'Trouve les routes' } }),
      cwd: CWD,
      childrenOf: (id: string) => (id === 't' ? [child] : []),
    });
    expect(screen.getByText('1 outil')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Explorer/ }));
    expect(screen.getByText('Trouve les routes')).toBeInTheDocument();
    expect(screen.getByText('README.md')).toBeInTheDocument();
  });

  it('shows the tools of a subagent started by a subagent', async () => {
    const inner = tool({ id: 'inner', name: 'Task', input: { description: 'Sous-tâche' }, parent: 'outer' });
    const leaf = tool({ id: 'leaf', name: 'Grep', input: { pattern: 'TODO' }, parent: 'inner', result: { isError: false, text: 'a.ts' } });
    const all: Record<string, ToolItem[]> = { outer: [inner], inner: [leaf] };
    render(ToolRow, {
      item: tool({ id: 'outer', name: 'Task', input: { description: 'Principale' } }),
      cwd: CWD,
      childrenOf: (id: string) => all[id] ?? [],
    });
    await userEvent.click(screen.getByRole('button', { name: /Principale/ }));
    await userEvent.click(screen.getByRole('button', { name: /Sous-tâche/ }));
    expect(screen.getByText('TODO')).toBeInTheDocument();
  });

  it('flags failed tools', () => {
    const { container } = render(ToolRow, {
      item: tool({ status: 'error', input: { command: 'npm run x' }, result: { isError: true, text: 'npm ERR! missing script' } }),
      cwd: CWD,
    });
    expect(container.querySelector('.tool.err')).not.toBeNull();
    expect(screen.getByText('npm ERR! missing script')).toBeInTheDocument();
  });

  it('opens the edited file at its first changed line, and still expands from the rest of the row', async () => {
    const onOpenFile = vi.fn();
    render(ToolRow, {
      item: tool({
        name: 'Edit',
        input: { file_path: 'C:\\code\\app\\src\\auth.ts' },
        result: { isError: false, add: 1, del: 0, patch: [{ oldStart: 4, newStart: 7, lines: ['+x'] }] },
      }),
      cwd: CWD,
      onOpenFile,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir src/auth.ts dans l’éditeur' }));
    expect(onOpenFile).toHaveBeenCalledWith('C:\\code\\app\\src\\auth.ts', 7);
    expect(screen.queryByText('x')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { expanded: false }));
    expect(screen.getByText('x')).toBeInTheDocument();
  });

  it('opens the edited file at the first line the edit changed, not at the top of its context', async () => {
    const onOpenFile = vi.fn();
    render(ToolRow, {
      item: tool({
        name: 'Edit',
        input: { file_path: 'C:\\code\\app\\src\\auth.ts' },
        result: { isError: false, add: 1, del: 1, patch: [{ oldStart: 10, newStart: 10, lines: [' a', ' b', '-old', '+new', ' c'] }] },
      }),
      cwd: CWD,
      onOpenFile,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir src/auth.ts dans l’éditeur' }));
    expect(onOpenFile).toHaveBeenCalledWith('C:\\code\\app\\src\\auth.ts', 12);
  });

  it('opens a written file without a line, and keeps the row’s own keys for the row only', async () => {
    const onOpenFile = vi.fn();
    render(ToolRow, {
      item: tool({
        name: 'Write',
        input: { file_path: 'C:\\code\\app\\notes.md' },
        result: { isError: false, add: 3, del: 0 },
      }),
      cwd: CWD,
      onOpenFile,
    });
    const link = screen.getByRole('button', { name: 'Ouvrir notes.md dans l’éditeur' });
    link.focus();
    // Enter on the path is the path's own action: it neither expands the row nor is swallowed by it.
    await userEvent.keyboard('{Enter}');
    expect(onOpenFile).toHaveBeenCalledWith('C:\\code\\app\\notes.md', null);
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
  });

  it('expands from the keyboard', async () => {
    render(ToolRow, {
      item: tool({ input: { command: 'npm test' }, result: { isError: false, text: '12 passed' } }),
      cwd: CWD,
      onOpenFile: () => {},
    });
    screen.getByRole('button', { expanded: false }).focus();
    await userEvent.keyboard('{Enter}');
    expect(screen.getByText('$ npm test')).toBeInTheDocument();
    await userEvent.keyboard(' ');
    expect(screen.queryByText('$ npm test')).not.toBeInTheDocument();
  });

  it('shows a plain path for a command', () => {
    render(ToolRow, { item: tool({ name: 'Bash', input: { command: 'npm test' } }), cwd: CWD, onOpenFile: () => {} });
    expect(screen.queryByRole('button', { name: /dans l’éditeur/ })).not.toBeInTheDocument();
  });
});

describe('ToolRow in English', () => {
  it('names the file to open in English, and counts the tools of a subagent with a plural', () => {
    setLang('en');
    const edit = tool({ name: 'Edit', input: { file_path: 'C:\\code\\app\\src\\auth.ts' }, result: { isError: false, add: 2, del: 1 } });
    const { unmount } = render(ToolRow, { item: edit, cwd: CWD, onOpenFile: () => {} });
    expect(screen.getByRole('button', { name: 'Open src/auth.ts in the editor' })).toHaveAttribute('title', 'Open in editor');
    unmount();
    const task = tool({ id: 'task', name: 'Task', input: { description: 'Explore the code' }, status: 'ok' });
    const sub = (id: string) => tool({ id, name: 'Read', input: { file_path: 'C:\\code\\app\\a.ts' }, parent: 'task' });
    const kids = (n: number) => Array.from({ length: n }, (_, i) => sub(`s${i}`));
    const { unmount: next } = render(ToolRow, { item: task, cwd: CWD, childrenOf: () => kids(1) });
    expect(screen.getByText('1 tool')).toBeInTheDocument();
    next();
    render(ToolRow, { item: task, cwd: CWD, childrenOf: () => kids(3) });
    expect(screen.getByText('3 tools')).toBeInTheDocument();
  });

  it('says in English that a tool is running, and sums up what it returned', () => {
    setLang('en');
    const { unmount } = render(ToolRow, { item: tool({ status: 'running', input: { command: 'npm test' } }), cwd: CWD });
    expect(screen.getByLabelText('running')).toBeInTheDocument();
    unmount();
    const { unmount: next } = render(ToolRow, {
      item: tool({ name: 'Read', input: { file_path: 'C:\\code\\app\\a.ts' }, result: { isError: false, text: 'a\nb\nc' } }),
      cwd: CWD,
    });
    expect(screen.getByText('3 lines')).toBeInTheDocument();
    next();
    render(ToolRow, { item: tool({ name: 'Grep', input: { pattern: 'x' }, result: { isError: false, text: '' } }), cwd: CWD });
    expect(screen.getByText('no results')).toBeInTheDocument();
  });
});
