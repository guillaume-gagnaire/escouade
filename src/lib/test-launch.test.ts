import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('./terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { agent, fakeBackend, project, resetApp, SETTINGS } from '../test/ipc';
import { app } from './state.svelte';
import { allRunning, anyRunning, flows, READY_LIMIT_MS, stopTests, testAgent } from './test-launch.svelte';
import type { Agent, InitialState, TestRecipe, UiEvent } from './types';

const RECIPE: TestRecipe = {
  prepare: [{ command: 'npm install', dir: '' }],
  processes: [
    { name: 'api', command: 'node api.js', dir: '', env: {}, url: 'http://localhost:4110/health' },
    { name: 'web', command: 'node web.js', dir: '', env: {}, url: 'http://localhost:4111' },
  ],
  open: 'http://localhost:4111/connexion',
};
let A: Agent;

/** Each step starts in a terminal of its own; a server answers once `up` says so. */
function backend(up: (url: string) => boolean, more: Record<string, (args: any) => unknown> = {}) {
  let n = 0;
  return fakeBackend({
    test_run_start: (a: any) => ({ id: `t${++n}`, projectId: 'p1', name: a.kind, shell: 'pwsh' }),
    http_ready: (a: any) => up(a.url),
    ...more,
  });
}

/** A step's process ends with `code`. */
const exit = (id: string, code: number) => Object.assign(app.launches[id], { status: code === 0 ? 'done' : 'crashed', code, ptyId: null });
/** A step's process, stopped on purpose, ends (killed: the backend gives a code). */
const stopped = (id: string) => Object.assign(app.launches[id], { status: 'stopped', code: 1, ptyId: null, stopping: false });

describe('▶ Tester', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    A = agent({
      id: 'a7',
      worktree: { path: 'C:\\code\\demo-api\\.claude\\worktrees\\dem-1', branch: 'ticket/dem-1', baseBranch: 'main' },
      recipe: RECIPE,
    });
    resetApp({ agents: [A] });
    flows.all = {};
    flows.prepared = {};
  });
  afterEach(() => vi.useRealTimers());

  it('prepares, starts the processes, waits for them, then opens the feature, and not before', async () => {
    let up = false;
    const b = backend(() => up);
    const run = testAgent(A, project());
    await vi.advanceTimersByTimeAsync(10);
    expect(app.modal).toEqual({ kind: 'testLaunch', agentId: 'a7' });
    expect(b.called('test_run_start').map((c) => c.args.kind)).toEqual(['prep']);
    exit('test:a7:prep:0', 0);
    await vi.advanceTimersByTimeAsync(2000);
    expect(b.called('test_run_start').map((c) => [c.args.kind, c.args.index])).toEqual([
      ['prep', 0],
      ['run', 0],
      ['run', 1],
    ]);
    expect(flows.all.a7.lines.map((l) => l.detail)).toEqual(['terminée', 'en attente de localhost:4110…', 'en attente de localhost:4111…']);
    expect(b.called('plugin:opener|open_url')).toHaveLength(0);
    up = true;
    await vi.advanceTimersByTimeAsync(600);
    await run;
    expect(flows.all.a7.phase).toBe('ready');
    expect(flows.all.a7.lines[1].detail).toMatch(/^prêt · \d+(,\d)? s$/);
    expect(b.called('plugin:opener|open_url').map((c) => c.args.url)).toEqual(['http://localhost:4111/connexion']);
  });

  it('stops on a preparation that fails', async () => {
    const b = backend(() => true);
    const run = testAgent(A, project());
    await vi.advanceTimersByTimeAsync(10);
    exit('test:a7:prep:0', 2);
    await vi.advanceTimersByTimeAsync(200);
    await run;
    expect(flows.all.a7).toMatchObject({ phase: 'failed', error: 'Préparation en échec (code 2)' });
    expect(flows.all.a7.lines[0]).toMatchObject({ state: 'failed', launchId: 'test:a7:prep:0' });
    expect(b.called('test_run_start')).toHaveLength(1);
  });

  it('tells a process that crashed', async () => {
    backend(() => false);
    flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
    const run = testAgent(A, project());
    await vi.advanceTimersByTimeAsync(10);
    exit('test:a7:run:1', 1);
    await vi.advanceTimersByTimeAsync(1000);
    await run;
    expect(flows.all.a7.phase).toBe('failed');
    expect(flows.all.a7.lines.find((l) => l.label === 'web')?.detail).toBe('planté (code 1)');
  });

  it('gives up on a server silent for 3 minutes', async () => {
    const b = backend((url) => !url.includes('4111'));
    flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
    const run = testAgent(A, project());
    await vi.advanceTimersByTimeAsync(READY_LIMIT_MS + 1000);
    await run;
    expect(flows.all.a7.lines.find((l) => l.label === 'web')?.detail).toBe('Pas de réponse de http://localhost:4111 après 3 min');
    expect(b.called('plugin:opener|open_url')).toHaveLength(0);
  });

  it('reopens the modal and the browser when everything already runs', async () => {
    const b = backend(() => true);
    flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
    const run = testAgent(A, project());
    await vi.advanceTimersByTimeAsync(1000);
    await run;
    app.modal = null;
    await testAgent(A, project());
    expect(app.modal).toEqual({ kind: 'testLaunch', agentId: 'a7' });
    expect(b.called('test_run_start')).toHaveLength(2);
    expect(b.called('plugin:opener|open_url')).toHaveLength(2);
  });

  it('only shows a test under way again when asked twice', async () => {
    const b = backend(() => false);
    const run = testAgent(A, project());
    await vi.advanceTimersByTimeAsync(10);
    app.modal = null;
    await testAgent(A, project());
    expect(app.modal).toEqual({ kind: 'testLaunch', agentId: 'a7' });
    expect(b.called('test_run_start')).toHaveLength(1);
    stopTests('a7');
    stopped('test:a7:prep:0');
    await vi.advanceTimersByTimeAsync(200);
    await run;
  });

  it('knows whether its processes run, all of them or one', async () => {
    backend(() => false);
    flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
    expect(anyRunning('a7')).toBe(false);
    expect(allRunning(A)).toBe(false);
    const run = testAgent(A, project());
    await vi.advanceTimersByTimeAsync(10);
    expect(anyRunning('a7')).toBe(true);
    expect(anyRunning('a70')).toBe(false);
    expect(allRunning(A)).toBe(true);
    exit('test:a7:run:1', 1);
    expect(allRunning(A)).toBe(false);
    expect(anyRunning('a7')).toBe(true);
    await vi.advanceTimersByTimeAsync(1000);
    await run;
  });

  describe('stopped', () => {
    it('ends as stopped, not as a failed preparation, when everything is stopped during it', async () => {
      const b = backend(() => true);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(10);
      stopTests('a7');
      expect(b.called('term_kill').map((c) => c.args.id)).toEqual(['t1']);
      stopped('test:a7:prep:0');
      await vi.advanceTimersByTimeAsync(200);
      await run;
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: 'Arrêté' });
      expect(flows.all.a7.lines[0]).toMatchObject({ state: 'stopped', detail: 'arrêtée' });
      expect(b.called('test_run_start')).toHaveLength(1);
    });

    it('stays stopped when a step stopped while it was starting then fails to start', async () => {
      fakeBackend({
        test_run_start: () => {
          stopTests('a7');
          throw 'Validation en cours : le lancement de test attendra.';
        },
      });
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(200);
      await run;
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: 'Arrêté' });
      expect(flows.all.a7.lines[0]).toMatchObject({ state: 'stopped', detail: 'arrêtée' });
    });

    it('leaves every line stopped when everything is stopped while the processes start', async () => {
      let n = 0;
      backend(() => true, {
        test_run_start: (a: any) => {
          if (a.kind === 'run' && a.index === 0) stopTests('a7');
          return { id: `t${++n}`, projectId: 'p1', name: a.kind, shell: 'pwsh' };
        },
      });
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: 'Arrêté' });
      expect(flows.all.a7.lines.map((l) => [l.state, l.detail])).toEqual([
        ['stopped', 'arrêté'],
        ['stopped', 'arrêté'],
      ]);
    });

    it('starts its processes anew when tested again while they are still stopping, not taking the dying ones', async () => {
      const b = backend(() => true);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const first = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await first;
      stopTests('a7');
      const again = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(10);
      // Their ends come first.
      expect(b.called('test_run_start')).toHaveLength(2);
      stopped('test:a7:run:0');
      stopped('test:a7:run:1');
      await vi.advanceTimersByTimeAsync(1000);
      await again;
      expect(b.called('test_run_start')).toHaveLength(4);
      expect(app.launches['test:a7:run:0']).toMatchObject({ status: 'running', ptyId: 't3' });
      expect(flows.all.a7.phase).toBe('ready');
    });

    it('ends as stopped when its preparation alone is stopped', async () => {
      backend(() => true);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(10);
      stopped('test:a7:prep:0');
      await vi.advanceTimersByTimeAsync(200);
      await run;
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: 'Arrêté' });
      expect(flows.all.a7.lines[0]).toMatchObject({ state: 'stopped', detail: 'arrêtée' });
    });

    it('shows each line stopped, and no browser to reopen, once everything that ran is stopped', async () => {
      const b = backend(() => true);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      expect(flows.all.a7.phase).toBe('ready');
      stopTests('a7');
      expect(b.called('term_kill').map((c) => c.args.id)).toEqual(['t1', 't2']);
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: 'Arrêté', opened: null });
      expect(flows.all.a7.lines.map((l) => [l.state, l.detail])).toEqual([
        ['stopped', 'arrêté'],
        ['stopped', 'arrêté'],
      ]);
    });

    it('keeps the reason of a failure when what still runs is stopped', async () => {
      backend(() => false);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(10);
      exit('test:a7:run:1', 1);
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      stopTests('a7');
      expect(flows.all.a7.error).toBe("web s'est arrêté");
      expect(flows.all.a7.lines.map((l) => [l.label, l.state])).toEqual([
        ['api', 'stopped'],
        ['web', 'failed'],
      ]);
    });
  });

  describe('refused', () => {
    it('tells why a preparation step could not start, which left no log', async () => {
      const b = fakeBackend({
        test_run_start: () => {
          throw 'Validation en cours : le lancement de test attendra.';
        },
      });
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(200);
      await run;
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: "« Préparation 1 » n'a pas pu démarrer" });
      expect(flows.all.a7.lines[0]).toMatchObject({ state: 'failed', detail: 'Validation en cours : le lancement de test attendra.' });
      expect(app.launches['test:a7:prep:0']).toBeUndefined();
      expect(b.called('test_run_start')).toHaveLength(1);
    });

    it('tells why a process could not start, and does not wait for it', async () => {
      const b = backend(() => false, {
        test_run_start: (a: any) => {
          if (a.index === 1) throw 'Seul un agent à worktree a des lancements de test.';
          return { id: 't1', projectId: 'p1', name: 'api', shell: 'pwsh' };
        },
      });
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: "« web » n'a pas pu démarrer" });
      expect(flows.all.a7.lines.find((l) => l.label === 'web')).toMatchObject({
        state: 'failed',
        detail: 'Seul un agent à worktree a des lancements de test.',
      });
      expect(b.called('http_ready').map((c) => c.args.url)).not.toContain('http://localhost:4111');
    });
  });

  describe('over', () => {
    const lines = () => flows.all.a7.lines.map((l) => [l.label, l.state, l.detail]);

    it('leaves no line waiting once a process crashed: the others are no longer waited for', async () => {
      backend(() => false);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(10);
      expect(flows.all.a7.lines[0]).toMatchObject({ state: 'waiting' });
      exit('test:a7:run:1', 1);
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      expect(lines()).toEqual([
        ['api', 'skipped', 'non attendu'],
        ['web', 'failed', 'planté (code 1)'],
      ]);
    });

    it('leaves no line waiting once a process was refused', async () => {
      backend(() => false, {
        test_run_start: (a: any) => {
          if (a.index === 1) throw 'Seul un agent à worktree a des lancements de test.';
          return { id: 't1', projectId: 'p1', name: 'api', shell: 'pwsh' };
        },
      });
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      expect(lines()).toEqual([
        ['api', 'skipped', 'non attendu'],
        ['web', 'failed', 'Seul un agent à worktree a des lancements de test.'],
      ]);
    });

    it('says when a process crashes after it was ready', async () => {
      backend(() => true);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      expect(flows.all.a7.phase).toBe('ready');
      exit('test:a7:run:1', 1);
      await vi.advanceTimersByTimeAsync(600);
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: "web s'est arrêté", opened: null });
      expect(flows.all.a7.lines[0]).toMatchObject({ state: 'ready' });
      expect(flows.all.a7.lines[1]).toMatchObject({ state: 'failed', detail: 'planté (code 1)' });
    });

    it('says when a process is stopped on its own after it was ready', async () => {
      backend(() => true);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      stopped('test:a7:run:0');
      await vi.advanceTimersByTimeAsync(600);
      expect(flows.all.a7).toMatchObject({ phase: 'failed', error: 'Arrêté', opened: null });
      expect(flows.all.a7.lines[0]).toMatchObject({ state: 'stopped', detail: 'arrêté' });
      expect(flows.all.a7.lines[1]).toMatchObject({ state: 'ready' });
    });
  });

  describe('a new recipe', () => {
    let channel: { onmessage: (e: UiEvent) => void } | null = null;
    const emit = (e: UiEvent) => channel!.onmessage(e);
    const RECIPE2: TestRecipe = {
      prepare: [{ command: 'npm ci', dir: '' }],
      processes: [{ name: 'web2', command: 'node web2.js', dir: '', env: {}, url: 'http://localhost:4112' }],
      open: '',
    };

    /** The window, started against a backend whose events the test pushes. */
    async function boot(up: (url: string) => boolean) {
      const initial: InitialState = {
        projects: [project()],
        agents: [A],
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
      const b = backend(up, {
        subscribe: (a: any) => {
          channel = a.channel;
          return initial;
        },
      });
      await app.init();
      return b;
    }

    it('stops and forgets the launches of the old one, and tests the new one from the start', async () => {
      const b = await boot(() => true);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      expect(flows.all.a7.phase).toBe('ready');
      // Another update of the agent, same recipe: nothing moves.
      emit({ type: 'agent', agent: { ...A, recipe: structuredClone(RECIPE), tokens: 12 } });
      expect(app.launches['test:a7:run:0']).toBeDefined();
      expect(flows.all.a7).toBeDefined();
      const A2 = { ...A, recipe: RECIPE2 };
      emit({ type: 'agent', agent: A2 });
      expect(b.called('term_kill').map((c) => c.args.id)).toEqual(['t1', 't2']);
      expect(Object.keys(app.launches)).toEqual([]);
      expect(flows.all.a7).toBeUndefined();
      // Its step 0 is another process: started anew, not taken for the old one.
      const again = testAgent(A2, project());
      await vi.advanceTimersByTimeAsync(10);
      exit('test:a7:prep:0', 0);
      await vi.advanceTimersByTimeAsync(1000);
      await again;
      expect(b.called('test_run_start').map((c) => [c.args.kind, c.args.index])).toEqual([
        ['run', 0],
        ['run', 1],
        ['prep', 0],
        ['run', 0],
      ]);
      expect(flows.all.a7.lines.map((l) => l.label)).toEqual(['Préparation : npm ci', 'web2']);
      expect(flows.all.a7.phase).toBe('ready');
    });

    it('ends a test under way on the old one, which starts nothing more, even once the new one runs', async () => {
      const b = await boot(() => true);
      const old = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(10);
      const A2 = { ...A, recipe: RECIPE2 };
      emit({ type: 'agent', agent: A2 });
      // The new test starts its own preparation at once, under the same id.
      const fresh = testAgent(A2, project());
      await vi.advanceTimersByTimeAsync(10);
      exit('test:a7:prep:0', 0);
      await vi.advanceTimersByTimeAsync(1000);
      await old;
      await fresh;
      expect(b.called('test_run_start').map((c) => [c.args.kind, c.args.index])).toEqual([
        ['prep', 0],
        ['prep', 0],
        ['run', 0],
      ]);
      expect(flows.all.a7.lines.map((l) => [l.label, l.state])).toEqual([
        ['Préparation : npm ci', 'ready'],
        ['web2', 'ready'],
      ]);
      expect(flows.all.a7.phase).toBe('ready');
    });

    it('ends a test waiting for the servers of the old one, which never opens its address on the new processes', async () => {
      let up = false;
      const b = await boot(() => up);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const old = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(10);
      // Same preparation, two other processes under the same ids.
      const A3 = {
        ...A,
        recipe: {
          prepare: RECIPE.prepare,
          processes: [
            { name: 'api3', command: 'node api3.js', dir: '', env: {}, url: 'http://localhost:4120' },
            { name: 'web3', command: 'node web3.js', dir: '', env: {}, url: 'http://localhost:4121' },
          ],
          open: 'http://localhost:4121/nouveau',
        },
      };
      /** The old servers asked whether they are up. */
      const oldPolls = () => b.called('http_ready').filter((c) => /:411\d/.test(c.args.url)).length;
      emit({ type: 'agent', agent: A3 });
      const polled = oldPolls();
      const fresh = testAgent(A3, project());
      await vi.advanceTimersByTimeAsync(10);
      up = true;
      await vi.advanceTimersByTimeAsync(1000);
      await old;
      await fresh;
      expect(b.called('plugin:opener|open_url').map((c) => c.args.url)).toEqual(['http://localhost:4121/nouveau']);
      expect(oldPolls()).toBe(polled);
      expect(flows.all.a7.lines.map((l) => [l.label, l.state])).toEqual([
        ['api3', 'ready'],
        ['web3', 'ready'],
      ]);
    });

    it('forgets the test of an agent that is removed', async () => {
      await boot(() => true);
      flows.prepared.a7 = JSON.stringify(RECIPE.prepare);
      const run = testAgent(A, project());
      await vi.advanceTimersByTimeAsync(1000);
      await run;
      emit({ type: 'agentRemoved', id: 'a7', projectId: 'p1' });
      expect(flows.all.a7).toBeUndefined();
      expect(flows.prepared.a7).toBeUndefined();
    });
  });
});
