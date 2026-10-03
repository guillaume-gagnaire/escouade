import { describe, expect, it, vi } from 'vitest';
import { agent, fakeBackend, project, SETTINGS } from '../test/ipc';
import { app } from './state.svelte';
import type { InitialState, RunCommand, TermInfo, UiEvent } from './types';

// The logs are xterm.js instances; here, the text written to each one.
const logs = vi.hoisted(() => ({}) as Record<string, string>);
// How each log tells its process about its new size.
const resizers = vi.hoisted(() => ({}) as Record<string, (cols: number, rows: number) => void>);
vi.mock('./terminals', () => ({
  launchLog: (id: string, onResize: (cols: number, rows: number) => void) => ({
    _: [(logs[id] ??= ''), (resizers[id] = onResize)],
    term: {
      cols: 100,
      rows: 30,
      // The header is on screen: the command starts on the 4th row.
      buffer: { active: { cursorY: 3 } },
      write: (d: string | Uint8Array, done?: () => void) => {
        logs[id] = (logs[id] ?? '') + (typeof d === 'string' ? d : new TextDecoder().decode(d));
        done?.();
      },
    },
    fit: { fit() {} },
  }),
  disposeLog: (id: string) => delete logs[id],
}));

import { forgetLaunches, launchStatus, restartLaunch, startAll, startLaunch, stopAll, stopLaunch } from './launch-actions';

const FRONT: RunCommand = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
const API: RunCommand = { id: 'c2', name: 'API', command: 'cargo run', shell: 'bash', cwd: '' };
const P = project({ runCommands: [FRONT, API] });

const info = (id: string, name = 'Front'): TermInfo => ({ id, projectId: 'p1', name, shell: 'pwsh' });

let channel: { onmessage: (e: UiEvent) => void } | null = null;
/** Pushes a backend event. */
const emit = (e: UiEvent) => channel!.onmessage(e);

/** The app against a fake backend. */
async function boot(handlers: Record<string, (args: any) => unknown> = {}) {
  for (const k of Object.keys(logs)) delete logs[k];
  const initial: InitialState = {
    projects: [P],
    agents: [agent()],
    ui: { activeProject: 'p1', view: 'project', selectedAgent: {} },
    settings: SETTINGS,
    usage: { fiveHour: null, sevenDay: null, todayCost: 0, updatedAt: 0 },
    git: {},
    shells: [],
    terminals: [],
    tickets: [],
    claudeFound: true,
    version: '0.1.0',
    models: [],
  };
  let n = 0;
  const backend = fakeBackend({
    subscribe: (args: any) => {
      channel = args.channel;
      return initial;
    },
    run_start: (a: any) => info(`t${++n}`, a.commandId === 'c1' ? 'Front' : 'API'),
    ...handlers,
  });
  app.launches = {};
  app.exitedTerms = {};
  app.toasts = [];
  await app.init();
  return { backend, emit };
}

describe('launch commands', () => {
  it('starts a command in its own terminal, sized like its log, and shows it running', async () => {
    const { backend } = await boot();
    await startLaunch(P, FRONT);
    expect(backend.called('run_start')[0].args).toMatchObject({ projectId: 'p1', commandId: 'c1', cols: 100, rows: 30, cursorRow: 4 });
    expect(app.launches.c1).toMatchObject({ status: 'running', ptyId: 't1', name: 'Front' });
    // The log opens on what runs.
    expect(logs.c1).toContain('$ npm run dev');
  });

  it('does not start a command twice', async () => {
    const { backend } = await boot();
    await startLaunch(P, FRONT);
    await startLaunch(P, FRONT);
    expect(backend.called('run_start')).toHaveLength(1);
  });

  it('stops a running command without calling it a crash', async () => {
    const { backend, emit } = await boot();
    await startLaunch(P, FRONT);
    stopLaunch('c1');
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    emit({ type: 'terminalExit', id: 't1', code: 1 });
    expect(app.launches.c1.status).toBe('stopped');
    expect(app.toasts).toHaveLength(0);
  });

  it('keeps the end of a command that exited before its start returned', async () => {
    await boot({
      run_start: () => {
        // The exit event overtakes the answer of run_start.
        emit({ type: 'terminalExit', id: 't1', code: 2 });
        return info('t1');
      },
    });
    await startLaunch(P, FRONT);
    expect(app.launches.c1).toMatchObject({ status: 'crashed', code: 2, ptyId: null });
    expect(app.exitedTerms.t1).toBeUndefined();
  });

  it('forgets the exits of its processes once handled', async () => {
    await boot();
    await startLaunch(P, FRONT);
    emit({ type: 'terminalExit', id: 't1', code: 0 });
    expect(app.launches.c1.status).toBe('done');
    expect(app.exitedTerms.t1).toBeUndefined();
  });

  it('lets the running process follow the size of its log', async () => {
    const { backend } = await boot();
    await startLaunch(P, FRONT);
    resizers.c1(120, 40);
    expect(backend.called('term_resize')[0].args).toEqual({ id: 't1', cols: 120, rows: 40 });
  });

  it('kills a command removed while it was starting', async () => {
    const { backend } = await boot({
      run_start: () => {
        forgetLaunches(['c1']);
        return info('t1');
      },
    });
    await startLaunch(P, FRONT);
    expect(backend.called('term_kill').map((c) => c.args.id)).toEqual(['t1']);
    expect(app.launches.c1).toBeUndefined();
  });

  it('calls a command stopped while starting stopped, even if its start fails', async () => {
    await boot({
      run_start: () => {
        stopLaunch('c1');
        throw 'shell « pwsh » introuvable';
      },
    });
    await startLaunch(P, FRONT);
    expect(app.launches.c1.status).toBe('stopped');
    expect(app.toasts).toHaveLength(0);
  });

  it('kills a command stopped while it was starting, once it is up', async () => {
    const { backend } = await boot({
      run_start: () => {
        stopLaunch('c1');
        return info('t1');
      },
    });
    await startLaunch(P, FRONT);
    expect(backend.called('term_kill').map((c) => c.args.id)).toEqual(['t1']);
  });

  it('shows why a command could not start, in its log and as a crash', async () => {
    await boot({
      run_start: () => {
        throw 'shell « bash » introuvable';
      },
    });
    await startLaunch(P, API);
    expect(app.launches.c2).toMatchObject({ status: 'crashed', code: null });
    expect(logs.c2).toContain('shell « bash » introuvable');
    expect(app.toasts.at(-1)?.kind).toBe('error');
  });

  it('restarts a command: stops it, waits for its end, starts it again and marks the log', async () => {
    const { backend, emit } = await boot();
    await startLaunch(P, FRONT);
    const done = restartLaunch(P, FRONT);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    expect(backend.called('run_start')).toHaveLength(1);
    emit({ type: 'terminalExit', id: 't1', code: 1 });
    await done;
    expect(backend.called('run_start')).toHaveLength(2);
    expect(app.launches.c1).toMatchObject({ status: 'running', ptyId: 't2' });
    expect(logs.c1).toContain('relancé');
    // A run killed in the middle of a full-screen program leaves the log as it found it.
    expect(logs.c1).toContain('[?1049l[!p');
    expect(app.toasts).toHaveLength(0);
  });

  it('restarts once, however many times it is asked, and a stop that follows holds', async () => {
    const { backend, emit } = await boot();
    await startLaunch(P, FRONT);
    const first = restartLaunch(P, FRONT);
    const second = restartLaunch(P, FRONT);
    emit({ type: 'terminalExit', id: 't1', code: 1 });
    await first;
    stopLaunch('c1');
    emit({ type: 'terminalExit', id: 't2', code: 1 });
    await second;
    expect(backend.called('run_start')).toHaveLength(2);
    expect(app.launches.c1.status).toBe('stopped');
  });

  it('does not restart a command removed while it was stopping', async () => {
    const { backend } = await boot();
    await startLaunch(P, FRONT);
    const done = restartLaunch(P, FRONT);
    forgetLaunches(['c1']);
    await done;
    expect(backend.called('run_start')).toHaveLength(1);
    expect(app.launches.c1).toBeUndefined();
    expect(logs.c1).toBeUndefined();
  });

  it('starts every command not running, and stops every running one', async () => {
    const { backend } = await boot();
    await startLaunch(P, FRONT);
    await startAll(P);
    expect(backend.called('run_start').map((c) => c.args.commandId)).toEqual(['c1', 'c2']);
    stopAll(P);
    expect(backend.called('term_kill').map((c) => c.args.id)).toEqual(['t1', 't2']);
  });

  it('forgets removed commands: stopped silently, their logs dropped', async () => {
    const { emit } = await boot();
    await startLaunch(P, FRONT);
    expect(logs.c1).toBeDefined();
    forgetLaunches(['c1']);
    emit({ type: 'terminalExit', id: 't1', code: 1 });
    expect(app.launches.c1).toBeUndefined();
    expect(logs.c1).toBeUndefined();
    expect(app.toasts).toHaveLength(0);
  });

  it('names each state in the words of the sidebar', () => {
    const l = { ptyId: null, name: 'Front', stopping: false, code: null, startedAt: 1 };
    expect(launchStatus(undefined).label).toBe('prêt');
    expect(launchStatus({ ...l, status: 'running', ptyId: 't1' }).label).toBe('en cours');
    expect(launchStatus({ ...l, status: 'running', stopping: true }).label).toBe('arrêt…');
    expect(launchStatus({ ...l, status: 'stopped' }).label).toBe('arrêté');
    expect(launchStatus({ ...l, status: 'done', code: 0 }).label).toBe('terminé');
    expect(launchStatus({ ...l, status: 'crashed', code: 2 }).label).toBe('planté (code 2)');
    expect(launchStatus({ ...l, status: 'crashed' }).label).toBe('planté');
  });
});
