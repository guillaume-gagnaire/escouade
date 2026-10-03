import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend, project, resetApp } from '../test/ipc';
import { app } from './state.svelte';

vi.mock('./terminals', () => ({
  openTerminal: (projectId: string, shell: string, name: string) => Promise.resolve({ id: 't1', projectId, name, shell }),
  disposeTerminal() {},
}));

import { newTerminal } from './term-actions';

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
