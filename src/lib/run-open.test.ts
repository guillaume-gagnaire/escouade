// « Ouvrir à son lancement » (ESC-560): a launch command's address opens once it answers, by the bounded wait of
// "▶ Tester", in a launch page (the agent's, or a tab of the project's « Pages » window) or in the browser; nothing
// opens when the command ends first, and its log says so.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { agent, fakeBackend, project } from '../test/ipc';
import { browsersOf, devpages, rowName } from './devpage.svelte';
import { setLang } from './i18n';
import { app } from './state.svelte';
import type { InitialState, PageView, Project, RunCommand, UiEvent } from './types';
import { READY_LIMIT_MS } from './wait-ready';

const logs = vi.hoisted(() => ({}) as Record<string, string>);
vi.mock('./terminals', () => ({
  launchLog: (id: string) => ({
    _: (logs[id] ??= ''),
    term: {
      cols: 100,
      rows: 30,
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

import { runIn, startLaunch, stopLaunch } from './launch-actions';
import { addressOf } from './run-open';

const FRONT: RunCommand = {
  id: 'c1',
  name: 'Front',
  command: 'npm run dev',
  shell: 'pwsh',
  cwd: 'web',
  url: 'http://localhost:${ESCOUADE_PORT_BASE:-5173}/',
  openIn: 'page',
};
const PLAIN: RunCommand = { id: 'c2', name: 'API', command: 'cargo run', shell: 'bash', cwd: '' };
const A = agent({ id: 'a7', name: 'refacto-auth', portBase: 4120 });

let channel: { onmessage: (e: UiEvent) => void } | null = null;
const emit = (e: UiEvent) => channel!.onmessage(e);

/** The app against a fake backend whose server answers from the `answersAt`-th request on (never with 0). */
async function boot(p: Project, answersAt = 2) {
  for (const k of Object.keys(logs)) delete logs[k];
  const initial = {
    projects: [p],
    agents: [A],
    ui: { activeProject: 'p1', view: 'project', selectedAgent: {} },
    settings: (await import('../test/ipc')).SETTINGS,
    usage: { todayCost: 0, accounts: [], current: { claude: 'principal' } },
    git: {},
    shells: [],
    terminals: [],
    tickets: [],
    claudeFound: true,
    version: '0.1.0',
    models: [],
    engines: [(await import('../test/ipc')).CLAUDE_ENGINE],
  } as unknown as InitialState;
  let n = 0;
  let asked = 0;
  const backend = fakeBackend({
    subscribe: (args: any) => {
      channel = args.channel;
      return initial;
    },
    run_start: () => ({ id: `t${++n}`, projectId: 'p1', name: 'Front', shell: 'pwsh' }),
    http_ready: () => answersAt > 0 && ++asked >= answersAt,
    devpage_open: (a: any): PageView => ({
      agentId: a.agentId,
      projectId: a.projectId ?? 'p1',
      title: a.title,
      url: a.url,
    }),
    open_url: () => undefined,
  });
  app.launches = {};
  app.toasts = [];
  devpages.pages = {};
  await app.init();
  return backend;
}

describe('a launch command that opens its address (ESC-560)', () => {
  beforeEach(() => {
    setLang('fr');
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());

  it('in the project: waits until it answers, then opens its browser’s row (ESC-657), one per command', async () => {
    const P = project({ runCommands: [FRONT, PLAIN] });
    const b = await boot(P);
    await startLaunch(P, FRONT);
    expect(logs.c1).toContain('Escouade ouvrira http://localhost:5173/ dès que l’adresse répondra.');
    // Not before it answers: no fixed delay, the same requests as "▶ Tester".
    expect(b.called('devpage_open')).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(600);
    expect(b.called('http_ready').map((c) => c.args.url)).toEqual(['http://localhost:5173/', 'http://localhost:5173/']);
    expect(b.called('devpage_open').map((c) => c.args)).toEqual([
      { agentId: 'run:p1:c1', title: 'Front', url: 'http://localhost:5173/', bounds: null, projectId: 'p1' },
    ]);
    expect(logs.c1).toContain('Ouverte dans sa ligne de Navigateurs : http://localhost:5173/');
    // Its row, named after the command; not shown, the user looking elsewhere.
    expect(browsersOf('p1').map(([k, p]) => [k, rowName(p), p.by])).toEqual([['run:p1:c1', 'Front', { kind: 'command', id: 'c1' }]]);
    expect(app.pageShown).toBeNull();
    expect(b.called('open_url')).toHaveLength(0);
    // A command without an address asks nothing.
    await startLaunch(P, PLAIN);
    await vi.advanceTimersByTimeAsync(2000);
    expect(b.called('http_ready')).toHaveLength(2);
    // Started again: the same page, by its key (the backend loads it again rather than opening another).
    stopLaunch('c1');
    emit({ type: 'terminalExit', id: 't1', code: 1 });
    await startLaunch(P, FRONT);
    await vi.advanceTimersByTimeAsync(600);
    expect(b.called('devpage_open').map((c) => c.args.agentId)).toEqual(['run:p1:c1', 'run:p1:c1']);
    expect(browsersOf('p1')).toHaveLength(1);
  });

  it('in an agent’s worktree: the address follows its ports and opens in its page', async () => {
    const P = project({ runCommands: [FRONT] });
    const b = await boot(P);
    await runIn(P, FRONT, 'a7');
    await vi.advanceTimersByTimeAsync(600);
    expect(b.called('http_ready').map((c) => c.args)).toEqual([
      { url: 'http://localhost:4120/', agentId: undefined },
      { url: 'http://localhost:4120/', agentId: undefined },
    ]);
    expect(b.called('devpage_open').map((c) => c.args)).toEqual([
      { agentId: 'a7', title: 'refacto-auth · Front', url: 'http://localhost:4120/', bounds: null },
    ]);
    expect(logs['wt:a7:c1']).toContain('Ouverte dans sa ligne de Navigateurs : http://localhost:4120/');
    expect(browsersOf('p1').map(([k, p]) => [k, p.by])).toEqual([['a7', { kind: 'agent', id: 'a7' }]]);
  });

  it('in the browser, when the command or the project says so', async () => {
    let P = project({ runCommands: [{ ...FRONT, openIn: 'browser' }] });
    let b = await boot(P, 1);
    await startLaunch(P, P.runCommands[0]);
    await vi.advanceTimersByTimeAsync(10);
    expect(b.called('open_url').map((c) => c.args.url)).toEqual(['http://localhost:5173/']);
    expect(b.called('devpage_open')).toHaveLength(0);
    expect(logs.c1).toContain('Ouverte dans le navigateur : http://localhost:5173/');

    P = project({ runCommands: [FRONT], pagesOpen: 'system' });
    b = await boot(P, 1);
    await startLaunch(P, FRONT);
    await vi.advanceTimersByTimeAsync(10);
    expect(b.called('open_url').map((c) => c.args.url)).toEqual(['http://localhost:5173/']);
    expect(b.called('devpage_open')).toHaveLength(0);
  });

  it('opens nothing when the command crashes before it answers, and its log says so', async () => {
    const P = project({ runCommands: [FRONT] });
    const b = await boot(P, 0);
    await startLaunch(P, FRONT);
    await vi.advanceTimersByTimeAsync(600);
    emit({ type: 'terminalExit', id: 't1', code: 1 });
    await vi.advanceTimersByTimeAsync(2000);
    expect(b.called('devpage_open')).toHaveLength(0);
    expect(b.called('open_url')).toHaveLength(0);
    expect(logs.c1).toContain('« Front » s’est arrêtée avant que http://localhost:5173/ réponde : rien ne s’ouvre.');
    // No more requests once it ended.
    const asked = b.called('http_ready').length;
    await vi.advanceTimersByTimeAsync(5000);
    expect(b.called('http_ready')).toHaveLength(asked);
  });

  it('says nothing when the user stops it, and gives up after the limit of "▶ Tester"', async () => {
    const P = project({ runCommands: [FRONT] });
    const b = await boot(P, 0);
    await startLaunch(P, FRONT);
    stopLaunch('c1');
    emit({ type: 'terminalExit', id: 't1', code: 1 });
    await vi.advanceTimersByTimeAsync(2000);
    expect(logs.c1).not.toContain('rien ne s’ouvre');

    await startLaunch(P, FRONT);
    await vi.advanceTimersByTimeAsync(READY_LIMIT_MS + 1000);
    expect(b.called('devpage_open')).toHaveLength(0);
    expect(logs.c1).toContain('Pas de réponse de http://localhost:5173/ après 3 min : rien ne s’ouvre.');
  });

  it('says why an address cannot be opened where the command runs', async () => {
    const P = project({ runCommands: [{ ...FRONT, url: 'http://localhost:${ESCOUADE_PORT_BASE}/' }] });
    await boot(P);
    expect(addressOf(P.runCommands[0])).toEqual({ error: expect.stringContaining('${ESCOUADE_PORT_BASE:-3000}') });
    expect(addressOf({ ...FRONT, id: 'wt:a7:c1', url: 'http://localhost:${ESCOUADE_PORT_BASE}/' })).toEqual({
      url: 'http://localhost:4120/',
    });
    expect(addressOf(PLAIN)).toBeNull();
    expect(addressOf({ ...FRONT, url: 'file:///etc/passwd' })).toEqual({ error: 'Seule une adresse http:// ou https:// s’ouvre.' });
  });
});
