import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { RunCommand } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import RunView from './RunView.svelte';

const mounted = vi.hoisted(() => [] as (string | null)[]);
vi.mock('../lib/terminals', () => ({
  launchLog: () => ({
    term: {
      cols: 80,
      rows: 24,
      buffer: { active: { cursorY: 0 } },
      write: (_: unknown, done?: () => void) => done?.(),
      clear() {},
      focus() {},
    },
    fit: { fit() {} },
    search: {},
  }),
  disposeLog() {},
  getXTerm: () => undefined,
  logKey: (id: string) => `run:${id}`,
  mountTerminal: (key: string, el: HTMLElement | null) => mounted.push(el ? key : null),
}));

const FRONT: RunCommand = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
const P = project({ runCommands: [FRONT] });

describe('RunView', () => {
  beforeEach(() => {
    resetApp({ projects: [P] });
    app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
    mounted.length = 0;
  });

  it('shows the command, where it runs, and its log', () => {
    fakeBackend();
    render(RunView, { cmd: FRONT, project: P });
    expect(screen.getByText('Front')).toBeInTheDocument();
    expect(screen.getByText('npm run dev')).toBeInTheDocument();
    expect(screen.getByText('PowerShell 7')).toBeInTheDocument();
    expect(screen.getByText(/demo-api[\\/]web/)).toBeInTheDocument();
    expect(mounted).toEqual(['run:c1']);
    expect(screen.getByText('Pas encore lancée.')).toBeInTheDocument();
  });

  it('writes the folder with the separator of the system it is on', () => {
    fakeBackend();
    // A Windows project keeps the backslash, whatever the way the command spells its folder.
    const win = project({ path: 'C:\\code\\demo-api', runCommands: [FRONT] });
    const { unmount } = render(RunView, { cmd: { ...FRONT, cwd: 'web/admin' }, project: win });
    expect(screen.getByText(/· C:\\code\\demo-api\\web\\admin$/)).toBeInTheDocument();
    unmount();
    // On macOS a backslash would make a name of its own: the folder is under the project with a slash.
    const mac = project({ path: '/Users/guill/dev/demo-api', runCommands: [FRONT] });
    render(RunView, { cmd: { ...FRONT, cwd: 'web/admin' }, project: mac });
    expect(screen.getByText(/· \/Users\/guill\/dev\/demo-api\/web\/admin$/)).toBeInTheDocument();
  });

  it('writes the folder of a recipe step in an agent’s worktree on macOS with a slash too', () => {
    const step: RunCommand = { id: 'test:a7:run:0', name: 'web', command: 'node serveur.js', shell: 'pwsh', cwd: 'web' };
    const wt = { path: '/Users/guill/wt/dem-1', branch: 'ticket/dem-1', baseBranch: 'main' };
    resetApp({ projects: [P], agents: [agent({ id: 'a7', worktree: wt })] });
    fakeBackend();
    render(RunView, { cmd: step, project: P });
    expect(screen.getByText(/· \/Users\/guill\/wt\/dem-1\/web$/)).toBeInTheDocument();
  });

  it('starts, then stops the command', async () => {
    const backend = fakeBackend({ run_start: () => ({ id: 't1', projectId: 'p1', name: 'Front', shell: 'pwsh' }) });
    render(RunView, { cmd: FRONT, project: P });
    await userEvent.click(screen.getAllByRole('button', { name: /Lancer/ })[0]);
    expect(backend.called('run_start')[0].args).toMatchObject({ commandId: 'c1' });
    expect(await screen.findByText('en cours')).toBeInTheDocument();
    expect(screen.queryByText('Pas encore lancée.')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Stopper/ }));
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
  });

  it('says how the last run ended', () => {
    fakeBackend();
    app.launches.c1 = { status: 'crashed', ptyId: null, name: 'Front', stopping: false, code: 1, startedAt: 1 };
    render(RunView, { cmd: FRONT, project: P });
    expect(screen.getByText('planté (code 1)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Relancer/ })).toBeInTheDocument();
  });

  describe('a step of an agent’s recipe', () => {
    const STEP: RunCommand = { id: 'test:a7:run:0', name: 'web', command: 'node serveur.js', shell: 'pwsh', cwd: 'web' };
    const wt = { path: 'C:\\code\\wt\\dem-1', branch: 'ticket/dem-1', baseBranch: 'main' };

    it('runs in its agent’s worktree', () => {
      resetApp({ projects: [P], agents: [agent({ id: 'a7', worktree: wt })] });
      fakeBackend();
      render(RunView, { cmd: STEP, project: P });
      expect(screen.getByText(/C:\\code\\wt\\dem-1\\web/)).toBeInTheDocument();
      expect(screen.queryByText(/demo-api/)).not.toBeInTheDocument();
    });

    it('runs in the root of the worktree when it has no folder', () => {
      resetApp({ projects: [P], agents: [agent({ id: 'a7', worktree: wt })] });
      fakeBackend();
      render(RunView, { cmd: { ...STEP, cwd: '' }, project: P });
      expect(screen.getByText(/· C:\\code\\wt\\dem-1$/)).toBeInTheDocument();
    });

    it('runs in the folder of an agent without a worktree', () => {
      resetApp({ projects: [P], agents: [agent({ id: 'a7', cwd: 'C:\\code\\autre' })] });
      fakeBackend();
      render(RunView, { cmd: { ...STEP, cwd: '' }, project: P });
      expect(screen.getByText(/· C:\\code\\autre$/)).toBeInTheDocument();
    });

    it('starts through the test launch', async () => {
      resetApp({ projects: [P], agents: [agent({ id: 'a7', worktree: wt })] });
      const backend = fakeBackend({ test_run_start: () => ({ id: 't7', projectId: 'p1', name: 'web', shell: 'pwsh' }) });
      render(RunView, { cmd: STEP, project: P });
      await userEvent.click(screen.getAllByRole('button', { name: /Lancer/ })[0]);
      expect(backend.called('test_run_start')[0].args).toMatchObject({ agentId: 'a7', kind: 'run', index: 0 });
      expect(backend.called('run_start')).toHaveLength(0);
    });
  });
});

describe('RunView in English', () => {
  beforeEach(() => {
    resetApp({ projects: [P] });
    app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
    setLang('en');
  });

  it('writes the buttons and the empty state in English', () => {
    fakeBackend();
    render(RunView, { cmd: FRONT, project: P });
    expect(screen.getByText('Not run yet.')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: '▶ Run' })).toHaveLength(2);
    expect(screen.getByRole('button', { name: 'Clear' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^⌕$/ })).toHaveAttribute('title', 'Search (Ctrl+Shift+F)');
  });

  it('says since when it runs, in the time of the language, and offers to restart or stop it', () => {
    fakeBackend();
    const startedAt = new Date(2026, 9, 2, 21, 30).getTime();
    app.launches.c1 = { status: 'running', ptyId: 't1', name: 'Front', stopping: false, code: null, startedAt };
    render(RunView, { cmd: FRONT, project: P });
    expect(screen.getByText('running')).toBeInTheDocument();
    expect(screen.getByText(/^since 9:30\sPM$/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '⟳ Restart' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '■ Stop' })).toBeInTheDocument();
  });

  it('says how the last run ended', () => {
    fakeBackend();
    app.launches.c1 = { status: 'crashed', ptyId: null, name: 'Front', stopping: false, code: 1, startedAt: 1 };
    render(RunView, { cmd: FRONT, project: P });
    expect(screen.getByText('crashed (code 1)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '⟳ Restart' })).toBeInTheDocument();
  });
});
