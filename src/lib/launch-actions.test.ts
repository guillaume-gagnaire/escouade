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

import {
  forgetLaunches,
  launchStatus,
  restartLaunch,
  startAll,
  startLaunch,
  stopAgentTests,
  stopAll,
  stopLaunch,
  testLaunchIds,
} from './launch-actions';

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

  it('waits for a run being stopped to end, then starts a run of its own rather than taking the dying one', async () => {
    const { backend, emit } = await boot();
    await startLaunch(P, FRONT);
    stopLaunch('c1');
    const again = startLaunch(P, FRONT);
    await new Promise((r) => setTimeout(r, 120));
    expect(backend.called('run_start')).toHaveLength(1);
    emit({ type: 'terminalExit', id: 't1', code: 1 });
    expect(await again).toBeNull();
    expect(backend.called('run_start')).toHaveLength(2);
    expect(app.launches.c1).toMatchObject({ status: 'running', ptyId: 't2', stopping: false });
  });

  it('gives up on a run that does not stop, and says why', async () => {
    const { backend } = await boot();
    await startLaunch(P, FRONT);
    stopLaunch('c1');
    vi.useFakeTimers();
    try {
      const again = startLaunch(P, FRONT);
      await vi.advanceTimersByTimeAsync(10_000);
      expect(await again).toBe("« Front » ne s'arrête pas");
    } finally {
      vi.useRealTimers();
    }
    expect(backend.called('run_start')).toHaveLength(1);
    expect(app.launches.c1).toMatchObject({ status: 'running', ptyId: 't1', stopping: true });
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

describe('test launches', () => {
  const WEB: RunCommand = { id: 'test:a1:run:0', name: 'web', command: 'node serveur.js', shell: 'pwsh', cwd: '' };
  const PREP: RunCommand = { id: 'test:a1:prep:0', name: 'Préparation 1', command: 'npm i', shell: 'pwsh', cwd: 'web' };

  it('start a step of an agent’s recipe through its own command, and stop with their agent', async () => {
    const { backend } = await boot({ test_run_start: () => info('t9', 'web') });
    await startLaunch(P, WEB);
    expect(backend.called('test_run_start')[0].args).toMatchObject({
      agentId: 'a1',
      kind: 'run',
      index: 0,
      cols: 100,
      rows: 30,
      cursorRow: 4,
    });
    expect(backend.called('run_start')).toHaveLength(0);
    expect(app.launches['test:a1:run:0']).toMatchObject({ status: 'running', ptyId: 't9' });
    expect(testLaunchIds('p1')).toEqual(['test:a1:run:0']);
    expect(testLaunchIds('p2')).toEqual([]);
    stopAgentTests('a1');
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't9' });
  });

  it('start a preparation step through the same command, and a launch command through its own', async () => {
    const { backend } = await boot({ test_run_start: () => info('t9', 'Préparation 1') });
    await startLaunch(P, PREP);
    await startLaunch(P, FRONT);
    expect(backend.called('test_run_start').map((c) => c.args)).toMatchObject([{ agentId: 'a1', kind: 'prep', index: 0 }]);
    expect(backend.called('run_start').map((c) => c.args.commandId)).toEqual(['c1']);
  });

  it('stops the test launches of one agent only, without calling it a crash', async () => {
    let n = 8;
    const { backend, emit } = await boot({ test_run_start: () => info(`t${++n}`, 'web') });
    app.agents.a2 = agent({ id: 'a2', projectId: 'p2' });
    await startLaunch(P, WEB);
    await startLaunch(P, { ...WEB, id: 'test:a2:run:0' });
    await startLaunch(P, FRONT);
    stopAgentTests('a1');
    expect(backend.called('term_kill').map((c) => c.args.id)).toEqual(['t9']);
    emit({ type: 'terminalExit', id: 't9', code: 1 });
    expect(app.launches['test:a1:run:0'].status).toBe('stopped');
    expect(app.launches['test:a2:run:0'].status).toBe('running');
    expect(app.launches.c1.status).toBe('running');
    expect(app.toasts).toHaveLength(0);
  });

  it('lists the test launches of the agents of a project, not the launch commands or those of others', async () => {
    let n = 8;
    await boot({ test_run_start: () => info(`t${++n}`, 'web'), run_start: () => info('t1') });
    app.agents.a2 = agent({ id: 'a2', projectId: 'p2' });
    await startLaunch(P, PREP);
    await startLaunch(P, FRONT);
    await startLaunch(P, { ...WEB, id: 'test:a2:run:0' });
    // The launch of an agent the window does not know belongs to no project.
    await startLaunch(P, { ...WEB, id: 'test:a3:run:0' });
    expect(testLaunchIds('p1')).toEqual(['test:a1:prep:0']);
    expect(testLaunchIds('p2')).toEqual(['test:a2:run:0']);
  });

  it('shows why a step was refused and leaves nothing of it behind', async () => {
    const { backend } = await boot({
      test_run_start: () => {
        throw 'Validation en cours : DEM-1 passe en revue';
      },
    });
    await startLaunch(P, WEB);
    expect(backend.called('test_run_start')).toHaveLength(1);
    // Not a run that crashed: no entry in the section, no log to open, nothing running.
    expect(app.launches['test:a1:run:0']).toBeUndefined();
    expect(logs['test:a1:run:0']).toBeUndefined();
    expect(testLaunchIds('p1')).toEqual([]);
    expect(app.toasts).toHaveLength(1);
    expect(app.toasts[0]).toMatchObject({ kind: 'error' });
    expect(app.toasts[0].text).toContain('Validation en cours : DEM-1 passe en revue');
    expect(backend.called('term_kill')).toHaveLength(0);
  });

  it('starts a step again once it is no longer refused', async () => {
    let refused = true;
    const { backend } = await boot({
      test_run_start: () => {
        if (refused) throw 'Validation en cours : DEM-1 passe en revue';
        return info('t9', 'web');
      },
    });
    await startLaunch(P, WEB);
    refused = false;
    await startLaunch(P, WEB);
    expect(backend.called('test_run_start')).toHaveLength(2);
    expect(app.launches['test:a1:run:0']).toMatchObject({ status: 'running', ptyId: 't9' });
  });

  it('keeps the last run of a step whose new start is refused', async () => {
    let refused = false;
    const { emit } = await boot({
      test_run_start: () => {
        if (refused) throw 'Validation en cours : DEM-1 passe en revue';
        return info('t9', 'web');
      },
    });
    await startLaunch(P, WEB);
    emit({ type: 'terminalExit', id: 't9', code: 0 });
    const before = { ...app.launches['test:a1:run:0'] };
    refused = true;
    await startLaunch(P, WEB);
    expect(app.launches['test:a1:run:0']).toEqual(before);
    expect(app.launches['test:a1:run:0'].status).toBe('done');
    expect(logs['test:a1:run:0']).toBeDefined();
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'error' });
  });

  it('says nothing of a refusal to a step stopped while it was starting', async () => {
    await boot({
      test_run_start: () => {
        stopLaunch('test:a1:run:0');
        throw 'Validation en cours : DEM-1 passe en revue';
      },
    });
    await startLaunch(P, WEB);
    expect(app.launches['test:a1:run:0']).toBeUndefined();
    expect(app.toasts).toHaveLength(0);
  });

  it('forgets the test launches of an agent that is removed, and closing the project no longer finds them', async () => {
    let n = 8;
    const { backend, emit } = await boot({ test_run_start: () => info(`t${++n}`, 'web'), run_start: () => info('t1') });
    app.agents.a2 = agent({ id: 'a2' });
    await startLaunch(P, PREP);
    await startLaunch(P, WEB);
    await startLaunch(P, { ...WEB, id: 'test:a2:run:0' });
    await startLaunch(P, FRONT);
    emit({ type: 'agentRemoved', id: 'a1', projectId: 'p1' });
    // Stopped silently, their logs dropped; the others stay.
    expect(backend.called('term_kill').map((c) => c.args.id)).toEqual(['t9', 't10']);
    expect(Object.keys(app.launches)).toEqual(['test:a2:run:0', 'c1']);
    expect(logs['test:a1:prep:0']).toBeUndefined();
    expect(logs['test:a1:run:0']).toBeUndefined();
    expect(logs['test:a2:run:0']).toBeDefined();
    expect(testLaunchIds('p1')).toEqual(['test:a2:run:0']);
    // Their exits come after: neither a crash nor an error.
    emit({ type: 'terminalExit', id: 't9', code: 1 });
    emit({ type: 'terminalExit', id: 't10', code: 1 });
    expect(app.toasts).toHaveLength(0);
    expect(app.launches.c1.status).toBe('running');
  });

  it('ignores the end of a terminal it never knew', async () => {
    const { emit } = await boot({ test_run_start: () => info('t9', 'web') });
    await startLaunch(P, WEB);
    expect(() => emit({ type: 'terminalExit', id: 't404', code: 1 })).not.toThrow();
    expect(app.toasts).toHaveLength(0);
    expect(app.launches['test:a1:run:0']).toMatchObject({ status: 'running', ptyId: 't9' });
  });
});
