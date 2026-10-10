import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend, project, resetApp } from '../test/ipc';
import { app } from './state.svelte';

vi.mock('./terminals', () => ({
  openTerminal: vi.fn((projectId: string, shell: string, name: string) => Promise.resolve({ id: 't1', projectId, name, shell })),
  disposeTerminal() {},
}));

import { newTerminal, terminalIn } from './term-actions';
import { openTerminal } from './terminals';

describe('terminals', () => {
  beforeEach(() => resetApp({ projects: [project({ runCommands: [{ id: 'c1', name: 'Front', command: 'x', shell: 'pwsh', cwd: '' }] })] }));

  it('shows a new terminal in place of the launch log that was shown', async () => {
    fakeBackend();
    app.selectLaunch('c1');
    await newTerminal('p1', 'pwsh');
    expect(app.term?.id).toBe('t1');
    expect(app.runCommand).toBeNull();
  });

  it('shows a new terminal in place of the editor of the project', async () => {
    fakeBackend();
    await app.openEditor({ source: 'project' });
    expect(app.editorOn).toBe(true);
    await newTerminal('p1', 'pwsh');
    expect(app.editorOn).toBe(false);
    expect(app.term?.id).toBe('t1');
  });

  it('shows a new terminal in place of the board of the project', async () => {
    fakeBackend();
    app.openBoard('p1');
    expect(app.boardOn).toBe(true);
    await newTerminal('p1', 'pwsh');
    expect(app.boardOn).toBe(false);
    expect(app.term?.id).toBe('t1');
  });

  it('leaves the board of another project alone', async () => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'studio-web' })] });
    fakeBackend();
    app.openBoard('p2');
    await newTerminal('p1', 'pwsh');
    expect(app.board.p2).toBe(true);
  });
});

describe('terminals opened in an agent or a folder', () => {
  beforeEach(() => {
    vi.mocked(openTerminal).mockClear();
    resetApp({ projects: [project()] });
    app.shells = [
      { id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' },
      { id: 'bash', label: 'Git Bash', path: 'bash.exe' },
    ];
  });

  it('opens one in the worktree of an agent, with the first shell, named after both', async () => {
    fakeBackend();
    await terminalIn('p1', { agentId: 'a1' }, 'refacto-auth');
    expect(openTerminal).toHaveBeenCalledWith('p1', 'pwsh', 'pwsh · refacto-auth', { agentId: 'a1' });
    expect(app.term).toMatchObject({ id: 't1', name: 'pwsh · refacto-auth' });
  });

  it('opens one in a folder of the source shown, named after the folder', async () => {
    fakeBackend();
    await terminalIn('p1', { agentId: 'a2', subdir: 'src/lib' }, 'lib');
    expect(openTerminal).toHaveBeenCalledWith('p1', 'pwsh', 'pwsh · lib', { agentId: 'a2', subdir: 'src/lib' });
  });

  it('shows it in place of the editor, as any new terminal', async () => {
    fakeBackend();
    await app.openEditor({ source: 'project' });
    await terminalIn('p1', { subdir: 'src' }, 'src');
    expect(app.editorOn).toBe(false);
    expect(app.term?.id).toBe('t1');
  });

  it('says when no shell was found, and opens nothing', async () => {
    fakeBackend();
    app.shells = [];
    await terminalIn('p1', { agentId: 'a1' }, 'refacto-auth');
    expect(openTerminal).not.toHaveBeenCalled();
    expect(app.toasts[0]).toMatchObject({ kind: 'error' });
    expect(app.terminals).toEqual([]);
  });
});
