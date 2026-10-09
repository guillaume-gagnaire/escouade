import { beforeEach, describe, expect, it, vi } from 'vitest';
import { agent, board, fakeBackend, gitInfo, project, resetApp, SETTINGS, ticket } from '../test/ipc';
import { conversationOf } from './conversations.svelte';
import { buffers } from './editor/buffers.svelte';
import { trees } from './editor/trees.svelte';
import { app } from './state.svelte';
import type { InitialState, LaunchState, UiEvent } from './types';

/** Starts the app against a fake backend and returns a function pushing backend events. */
async function start(over: Partial<InitialState> = {}, handlers: Record<string, (args: any) => unknown> = {}) {
  let channel: { onmessage: (e: UiEvent) => void } | null = null;
  const initial: InitialState = {
    projects: [project(), project({ id: 'p2', name: 'studio-web' })],
    agents: [agent(), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 }), agent({ id: 'b1', projectId: 'p2', name: 'landing' })],
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
    ...over,
  };
  const backend = fakeBackend({
    subscribe: (args: any) => {
      channel = args.channel;
      return initial;
    },
    get_conversation: () => [],
    ...handlers,
  });
  await app.init();
  return { backend, emit: (e: UiEvent) => channel!.onmessage(e) };
}

describe('AppState', () => {
  it('selects the first agent of the active project by default', async () => {
    await start();
    expect(app.project?.id).toBe('p1');
    expect(app.projectAgents.map((a) => a.id)).toEqual(['a1', 'a2']);
    expect(app.agent?.id).toBe('a1');
  });

  it('falls back to the first project when the saved one is gone', async () => {
    await start({ ui: { activeProject: 'deleted', view: 'project', selectedAgent: {} } });
    expect(app.project?.id).toBe('p1');
  });

  it('knows which model each alias runs, from the start and as Claude Code reports it', async () => {
    const { emit } = await start({ models: [{ value: 'sonnet', resolvedModel: 'claude-sonnet-5' }] });
    expect(app.models).toEqual([{ value: 'sonnet', resolvedModel: 'claude-sonnet-5' }]);
    emit({ type: 'models', models: [{ value: 'sonnet', resolvedModel: 'claude-sonnet-5-5' }] });
    expect(app.models).toEqual([{ value: 'sonnet', resolvedModel: 'claude-sonnet-5-5' }]);
  });

  it('keeps what the running Claude processes use', async () => {
    const { emit } = await start();
    const resources = { instances: 1, memory: 300, cpu: 4.5, agents: [{ id: 'a1', memory: 300, cpu: 4.5 }] };
    emit({ type: 'resources', resources });
    expect(app.resources).toEqual(resources);
  });

  it('applies agent upserts and removals from the backend', async () => {
    const { emit } = await start();
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'waiting' }) });
    expect(app.agents.a2.status).toBe('waiting');
    emit({ type: 'agentRemoved', id: 'a2', projectId: 'p1' });
    expect(app.projectAgents.map((a) => a.id)).toEqual(['a1']);
    expect(app.agents.a2).toBeUndefined();
  });

  it('lets other modules forget what they hold of a removed agent, while it is still known', async () => {
    const { emit } = await start();
    const seen: [string, boolean][] = [];
    const off = app.onAgentRemoved((id) => seen.push([id, !!app.agents[id]]));
    emit({ type: 'agentRemoved', id: 'a2', projectId: 'p1' });
    expect(seen).toEqual([['a2', true]]);
    expect(app.agents.a2).toBeUndefined();
    off();
    emit({ type: 'agentRemoved', id: 'b1', projectId: 'p2' });
    expect(seen).toHaveLength(1);
  });

  it('forgets a removed agent even when a module fails to, and runs a hook once however often it is registered', async () => {
    const { emit } = await start();
    const seen: string[] = [];
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    const boom = () => {
      throw new Error('boom');
    };
    const note = (id: string) => seen.push(id);
    const offs = [app.onAgentRemoved(boom), app.onAgentRemoved(note), app.onAgentRemoved(note)];
    try {
      emit({ type: 'agentRemoved', id: 'a2', projectId: 'p1' });
      expect(app.agents.a2).toBeUndefined();
      expect(app.projectAgents.map((a) => a.id)).toEqual(['a1']);
      expect(seen).toEqual(['a2']);
      expect(error).toHaveBeenCalled();
    } finally {
      for (const off of offs) off();
    }
    emit({ type: 'agentRemoved', id: 'b1', projectId: 'p2' });
    expect(seen).toEqual(['a2']);
  });

  it('tells other modules when an agent’s launch recipe changes, and not on its other updates', async () => {
    const { emit } = await start();
    const seen: string[] = [];
    const off = app.onRecipeChanged((id) => seen.push(id));
    const recipe = { prepare: [], processes: [{ name: 'web', command: 'node web.js', dir: '', env: {}, url: '' }], open: '' };
    emit({ type: 'agent', agent: agent({ tokens: 5 }) });
    expect(seen).toEqual([]);
    emit({ type: 'agent', agent: agent({ recipe }) });
    expect(seen).toEqual(['a1']);
    // The same recipe again, in a new object.
    emit({ type: 'agent', agent: agent({ recipe: structuredClone(recipe), tokens: 9 }) });
    expect(seen).toEqual(['a1']);
    emit({ type: 'agent', agent: agent({ recipe: { ...recipe, open: 'http://localhost:4100' } }) });
    expect(seen).toEqual(['a1', 'a1']);
    // A new agent has no launches yet.
    emit({ type: 'agent', agent: agent({ id: 'a9', recipe }) });
    expect(seen).toEqual(['a1', 'a1']);
    off();
    emit({ type: 'agent', agent: agent() });
    expect(seen).toEqual(['a1', 'a1']);
  });

  it('tells other modules once when a ticket goes "Terminé", with its agent', async () => {
    const { emit } = await start({ tickets: [ticket({ column: 'review', agentId: 'a1' })] });
    const seen: string[] = [];
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    const offs = [
      app.onTicketDone(() => {
        throw new Error('boom');
      }),
      app.onTicketDone((id) => seen.push(id)),
    ];
    try {
      emit({ type: 'ticket', ticket: ticket({ column: 'review', agentId: 'a1', step: 'Merge…' }) });
      expect(seen).toEqual([]);
      emit({ type: 'ticket', ticket: ticket({ column: 'done', agentId: 'a1' }) });
      // A hook that fails neither stops the others nor the ticket's update.
      expect(seen).toEqual(['a1']);
      expect(app.tickets.t1.column).toBe('done');
      expect(error).toHaveBeenCalled();
      // Updated again once done (its agent's cost): already told.
      emit({ type: 'ticket', ticket: ticket({ column: 'done', agentId: 'a1', cost: 2 }) });
      expect(seen).toEqual(['a1']);
      // Done with no agent: nothing to tell.
      emit({ type: 'ticket', ticket: ticket({ id: 't2', column: 'review', agentId: null }) });
      emit({ type: 'ticket', ticket: ticket({ id: 't2', column: 'done', agentId: null }) });
      expect(seen).toEqual(['a1']);
    } finally {
      for (const off of offs) off();
    }
    emit({ type: 'ticket', ticket: ticket({ id: 't3', column: 'review', agentId: 'a2' }) });
    emit({ type: 'ticket', ticket: ticket({ id: 't3', column: 'done', agentId: 'a2' }) });
    expect(seen).toEqual(['a1']);
  });

  it('finds the launch on screen among the project’s commands, then among the steps of its agents’ recipes', async () => {
    const run = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: '' };
    const recipe = {
      prepare: [],
      processes: [{ name: 'web', command: 'node serveur.js', dir: '', env: {}, url: '' }],
      open: '',
    };
    await start({
      projects: [project({ runCommands: [run] })],
      agents: [agent({ recipe })],
      shells: [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }],
    });
    expect(app.runCommand).toBeNull();
    app.selectLaunch('c1');
    expect(app.runCommand?.command).toBe('npm run dev');
    app.selectLaunch('test:a1:run:0');
    expect(app.runCommand).toEqual({ id: 'test:a1:run:0', name: 'web', command: 'node serveur.js', shell: 'pwsh', cwd: '' });
    // A step its recipe no longer has opens nothing.
    app.selectLaunch('test:a1:run:3');
    expect(app.runCommand).toBeNull();
    app.selectLaunch(null);
    expect(app.runCommand).toBeNull();
  });

  it('routes conversation ops to the loaded conversation', async () => {
    const { emit } = await start();
    const c = conversationOf('a1');
    await new Promise((r) => setTimeout(r));
    emit({
      type: 'conv',
      agentId: 'a1',
      ops: [{ op: 'append', item: { kind: 'user', id: 'u1', text: 'Salut', images: 0, ts: 1, queued: false } }],
    });
    expect(c.items.map((i) => i.id)).toEqual(['u1']);
  });

  it('only bumps the files refresh tick for the active project', async () => {
    const { emit } = await start();
    const before = app.gitTick;
    const git = gitInfo({ modified: 1, total: 1, upstream: 'origin/main', behind: 2, hasRemote: true });
    emit({ type: 'git', projectId: 'p2', git });
    expect(app.gitTick).toBe(before);
    expect(app.git.p2.total).toBe(1);
    expect(app.git.p2.behind).toBe(2);
    emit({ type: 'git', projectId: 'p1', git });
    expect(app.gitTick).toBe(before + 1);
  });

  it('focuses the agent a notification was clicked for', async () => {
    const { emit, backend } = await start();
    app.openStats();
    emit({ type: 'focus', projectId: 'p2', agentId: 'b1' });
    expect(app.ui.view).toBe('project');
    expect(app.project?.id).toBe('p2');
    expect(app.agent?.id).toBe('b1');
    await new Promise((r) => setTimeout(r, 300));
    expect(backend.called('set_ui').at(-1)?.args.ui).toMatchObject({ activeProject: 'p2', selectedAgent: { p2: 'b1' } });
  });

  it('restores the saved screen layout, classic by default', async () => {
    await start();
    expect(app.split).toBe(false);
    await start({ ui: { activeProject: 'p1', view: 'project', selectedAgent: {}, layout: 'split' } });
    expect(app.split).toBe(true);
  });

  it('toggles the screen layout and saves it', async () => {
    const { backend } = await start();
    app.toggleLayout();
    expect(app.split).toBe(true);
    await new Promise((r) => setTimeout(r, 300));
    expect(backend.called('set_ui').at(-1)?.args.ui).toMatchObject({ layout: 'split' });
    app.toggleLayout();
    expect(app.split).toBe(false);
  });

  it('cycles through waiting agents across projects', async () => {
    const { emit } = await start();
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'waiting', lastActivity: 5 }) });
    emit({ type: 'agent', agent: agent({ id: 'b1', projectId: 'p2', name: 'landing', status: 'waiting', lastActivity: 9 }) });
    app.nextWaiting();
    expect(app.agent?.id).toBe('a2');
    app.nextWaiting();
    expect(app.agent?.id).toBe('b1');
    expect(app.project?.id).toBe('p2');
    app.nextWaiting();
    expect(app.agent?.id).toBe('a2');
  });

  it('sends the focus to the message field of the waiting agent Ctrl+J goes to, whatever it waits for', async () => {
    const { emit } = await start();
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'waiting', pending: ['req-1'] }) });
    const focus = app.focusComposer;
    app.nextWaiting();
    expect(app.agent?.id).toBe('a2');
    expect(app.focusComposer).toBe(focus + 1);
  });

  it('flags an agent that asks, finishes or fails out of sight until it is seen', async () => {
    const { emit } = await start();
    const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(true);
    // b1 (another project) finishes, a2 (not selected) asks: both need a look.
    emit({ type: 'agent', agent: agent({ id: 'b1', projectId: 'p2', name: 'landing', status: 'running' }) });
    emit({ type: 'agent', agent: agent({ id: 'b1', projectId: 'p2', name: 'landing', status: 'done' }) });
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'waiting' }) });
    expect(app.attention).toEqual({ b1: true, a2: true });
    expect(app.attentionIn('p2').map((a) => a.id)).toEqual(['b1']);
    // The agent on screen, window in front, is never flagged.
    emit({ type: 'agent', agent: agent({ status: 'running' }) });
    emit({ type: 'agent', agent: agent({ status: 'done' }) });
    expect(app.attention.a1).toBeUndefined();
    // Shown: seen.
    app.selectAgent('b1');
    app.markSeen();
    expect(app.attention).toEqual({ a2: true });
    // Answered from elsewhere (claude.ai): nothing left to look at.
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'running' }) });
    expect(app.attention).toEqual({});
    focus.mockRestore();
  });

  it('flags the agent on screen when the window is in the background, until it comes back', async () => {
    const { emit } = await start();
    const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(false);
    emit({ type: 'agent', agent: agent({ status: 'running' }) });
    emit({ type: 'agent', agent: agent({ status: 'waiting' }) });
    expect(app.attention).toEqual({ a1: true });
    app.markSeen();
    expect(app.attention).toEqual({ a1: true });
    focus.mockReturnValue(true);
    app.markSeen();
    expect(app.attention).toEqual({});
    focus.mockRestore();
  });

  it('goes to an agent that finished out of sight with Ctrl+J', async () => {
    const { emit } = await start();
    emit({ type: 'agent', agent: agent({ id: 'b1', projectId: 'p2', name: 'landing', status: 'running' }) });
    emit({ type: 'agent', agent: agent({ id: 'b1', projectId: 'p2', name: 'landing', status: 'done' }) });
    app.nextWaiting();
    expect(app.agent?.id).toBe('b1');
  });

  it('turns backend errors into error toasts', async () => {
    await start();
    const out = await app.run(Promise.reject('Le dossier C:\\x n’existe plus'));
    expect(out).toBeUndefined();
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'Le dossier C:\\x n’existe plus' });
  });
});

describe('AppState start-up', () => {
  it('keeps backend events received while the initial snapshot was loading', async () => {
    let channel: { onmessage: (e: UiEvent) => void } | null = null;
    let release!: (s: InitialState) => void;
    fakeBackend({
      subscribe: (args: any) => {
        channel = args.channel;
        return new Promise<InitialState>((r) => (release = r));
      },
    });
    const init = app.init();
    await new Promise((r) => setTimeout(r));
    // The agent started working before the (older) snapshot reached the UI.
    channel!.onmessage({ type: 'agent', agent: agent({ status: 'running', tokens: 500 }) });
    release({
      projects: [project()],
      agents: [agent({ status: 'done', tokens: 100 })],
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
    });
    await init;
    expect(app.agents.a1).toMatchObject({ status: 'running', tokens: 500 });
  });
});

describe('AppState launch commands', () => {
  const running = (over: Partial<LaunchState> = {}): LaunchState => ({
    status: 'running',
    ptyId: 't9',
    name: 'Front',
    stopping: false,
    code: null,
    startedAt: 1,
    ...over,
  });

  it('marks a command that exits with an error as crashed, and says so', async () => {
    const { emit } = await start();
    app.launches.c1 = running();
    emit({ type: 'terminalExit', id: 't9', code: 2 });
    expect(app.launches.c1).toMatchObject({ status: 'crashed', code: 2, ptyId: null });
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: expect.stringContaining('Front') });
  });

  it('does not call a stopped or finished command a crash', async () => {
    const { emit } = await start();
    app.launches.c1 = running({ stopping: true });
    app.launches.c2 = running({ ptyId: 't10', name: 'Build' });
    const toasts = app.toasts.length;
    emit({ type: 'terminalExit', id: 't9', code: 1 });
    emit({ type: 'terminalExit', id: 't10', code: 0 });
    expect(app.launches.c1.status).toBe('stopped');
    expect(app.launches.c2).toMatchObject({ status: 'done', code: 0 });
    expect(app.toasts.length).toBe(toasts);
  });

  it('shows one thing at a time in the main area: an agent, a terminal or a launch command', async () => {
    await start();
    app.selectLaunch('c1');
    expect(app.selectedLaunch.p1).toBe('c1');
    app.selectTerm('t1');
    expect(app.selectedLaunch.p1).toBeNull();
    app.selectLaunch('c1');
    expect(app.selectedTerm.p1).toBeNull();
    app.selectAgent('a1');
    expect(app.selectedLaunch.p1).toBeNull();
  });
});

describe('editor', () => {
  const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\wt', branch: 'escouade/wt', baseBranch: 'main' };
  beforeEach(() => {
    resetApp({ agents: [agent(), agent({ id: 'a2', name: 'wt', createdAt: 2, worktree: wt })] });
  });

  it('opens on a source and a file with its folders unfolded, and reopens as it was left', async () => {
    fakeBackend();
    await app.openEditor({ source: 'a2', path: 'src/a/x.ts' });
    expect(app.editorOn).toBe(true);
    expect(app.editor.p1.source).toBe('a2');
    expect(app.editor.p1.places.a2).toEqual({ open: ['src/a/x.ts'], active: 'src/a/x.ts', expanded: { src: true, 'src/a': true } });
    app.closeEditor();
    expect(app.editorOn).toBe(false);
    await app.openEditor({ source: 'a2' });
    expect(app.editor.p1.places.a2.open).toEqual(['src/a/x.ts']);
  });

  it('unfolds a folder with the ones above it, folds it alone, and folds them all', async () => {
    fakeBackend();
    await app.openEditor({ source: 'project' });
    const place = () => app.editor.p1.places.project;
    // A row `src/lib/editor` stands for its three folders: they all open with it.
    app.toggleEditorDir('p1', 'project', 'src/lib/editor');
    expect(place().expanded).toEqual({ src: true, 'src/lib': true, 'src/lib/editor': true });
    app.toggleEditorDir('p1', 'project', 'src/lib/editor');
    expect(place().expanded).toEqual({ src: true, 'src/lib': true, 'src/lib/editor': false });
    app.toggleEditorDir('p1', 'project', 'constructor');
    expect(place().expanded.constructor).toBe(true);
    app.expandEditorDir('p1', 'project', 'docs/api');
    expect(place().expanded).toMatchObject({ docs: true, 'docs/api': true });
    app.expandEditorDir('p1', 'project', '');
    expect(Object.hasOwn(place().expanded, '')).toBe(false);
    app.collapseEditorDirs('p1', 'project');
    expect(place().expanded).toEqual({});
  });

  it('does not count the editor as open behind the statistics', async () => {
    fakeBackend();
    await app.openEditor({ source: 'project', path: 'a.ts' });
    app.ui.view = 'stats';
    expect(app.editorOn).toBe(false);
    app.ui.view = 'project';
    expect(app.editorOn).toBe(true);
  });

  it('opens an absolute path from its source root, at a line', async () => {
    fakeBackend({ fs_tree: () => ({ root: 'C:/code/demo-api/.claude/worktrees/wt', files: [], truncated: false }) });
    await app.openEditor({ source: 'a2', abs: 'C:\\code\\demo-api\\.claude\\worktrees\\wt\\src\\x.ts', line: 12 });
    expect(app.editor.p1.places.a2.active).toBe('src/x.ts');
    expect(app.editor.p1.reveal).toMatchObject({ path: 'src/x.ts', line: 12 });
  });

  it.each([
    ['a Windows file of another folder', 'C:\\Users\\guill\\.claude\\plans\\plan.md'],
    ['a rooted file', '/tmp/notes.md'],
    ['a file of a sibling folder', 'C:\\code\\demo-api-old\\x.ts'],
    ['a file beside the folder, through ..', 'C:\\code\\demo-api\\..\\other\\x.ts'],
  ])('does not open %s, and says it is outside the folder', async (_, abs) => {
    fakeBackend({ fs_tree: () => ({ root: 'C:/code/demo-api', files: [], truncated: false }) });
    await app.openEditor({ source: 'project', path: 'a.ts' });
    app.closeEditor();
    const before = JSON.parse(JSON.stringify(app.editor.p1));
    await app.openEditor({ source: 'project', abs, line: 4 });
    expect(JSON.parse(JSON.stringify(app.editor.p1))).toEqual(before);
    expect(app.editorOn).toBe(false);
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'info', text: expect.stringMatching(/ est en dehors du dossier du projet\.$/) });
  });

  it.each([
    ['the project', 'project', 'C:/Users/RUNNER~1/demo-api', 'C:\\code\\demo-api\\src\\x.ts'],
    ['an agent’s worktree', 'a2', 'D:/real/demo-api/.claude/worktrees/wt', 'C:\\code\\demo-api\\.claude\\worktrees\\wt\\src\\x.ts'],
  ])('opens a link into %s spelled from its folder when git spells the root otherwise', async (_, source, root, abs) => {
    fakeBackend({ fs_tree: () => ({ root, files: ['src/x.ts'], truncated: false }) });
    await app.openEditor({ source, abs, line: 2 });
    expect(app.editor.p1.places[source]?.active).toBe('src/x.ts');
    expect(app.toasts).toEqual([]);
  });

  it('opens a link spelled in another case on the tree’s file, not in a second tab', async () => {
    fakeBackend({ fs_tree: () => ({ root: 'C:/code/demo-api', files: ['src/app.ts', 'lib/util.ts', 'lib/Util.ts'], truncated: false }) });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    await app.openEditor({ source: 'project', abs: 'C:\\code\\demo-api\\src\\App.ts', line: 1 });
    expect(app.editor.p1.places.project).toMatchObject({ open: ['src/app.ts'], active: 'src/app.ts' });
    expect(app.editor.p1.reveal).toMatchObject({ path: 'src/app.ts' });
    // Both spellings in the tree (a case-sensitive disk): the one linked.
    await app.openEditor({ source: 'project', abs: 'C:\\code\\demo-api\\lib\\Util.ts' });
    expect(app.editor.p1.places.project.active).toBe('lib/Util.ts');
  });

  it('names the agent, not the project, for a file outside an agent’s folder', async () => {
    fakeBackend({ fs_tree: () => ({ root: 'C:/code/demo-api/.claude/worktrees/wt', files: [], truncated: false }) });
    await app.openEditor({ source: 'a2', abs: 'C:\\code\\demo-api\\src\\x.ts' });
    expect(app.toasts.at(-1)?.text).toBe('x.ts est en dehors du dossier de cet agent.');
    expect(app.editor.p1).toBeUndefined();
  });

  it('closes a tab and shows the last one left', async () => {
    fakeBackend();
    await app.openEditor({ source: 'project', path: 'a.ts' });
    await app.openEditor({ source: 'project', path: 'b.ts' });
    app.closeEditorTab('p1', 'project', 'b.ts');
    expect(app.editor.p1.places.project).toMatchObject({ open: ['a.ts'], active: 'a.ts' });
  });

  it('follows the agent picked in the sidebar, and gives way to a terminal or a launch', async () => {
    fakeBackend();
    await app.openEditor({ source: 'project' });
    app.selectAgent('a2');
    expect(app.editor.p1.source).toBe('a2');
    app.selectAgent('a1');
    expect(app.editor.p1.source).toBe('project');
    expect(app.editorOn).toBe(true);
    app.selectTerm('t1');
    expect(app.editorOn).toBe(false);
  });

  it('does not count a conversation hidden by the editor as seen', async () => {
    fakeBackend();
    vi.spyOn(document, 'hasFocus').mockReturnValue(true);
    app.ui.selectedAgent.p1 = 'a1';
    await app.openEditor({ source: 'project' });
    app.markSeen();
    app.attention = { a1: true };
    app.markSeen();
    expect(app.attention).toEqual({ a1: true });
    app.closeEditor();
    app.markSeen();
    expect(app.attention).toEqual({});
    vi.restoreAllMocks();
  });

  it('gives way to a launch command picked in the sidebar', async () => {
    resetApp({
      projects: [project({ runCommands: [{ id: 'c1', name: 'Front', command: 'x', shell: 'pwsh', cwd: '' }] })],
      agents: [agent()],
    });
    fakeBackend();
    await app.openEditor({ source: 'project' });
    expect(app.editorOn).toBe(true);
    app.selectLaunch('c1');
    expect(app.editorOn).toBe(false);
    expect(app.runCommand?.id).toBe('c1');
  });

  it('still opens the file when the state was read while the tree of its source was loading', async () => {
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    fakeBackend({
      fs_tree: async () => {
        await gate;
        return { root: 'C:/code/demo-api/.claude/worktrees/wt', files: [], truncated: false };
      },
    });
    await app.openEditor({ source: 'project', path: 'a.ts' });
    app.closeEditor();
    const read = app.editor.p1;
    const opening = app.openEditor({ source: 'a2', abs: 'C:\\code\\demo-api\\.claude\\worktrees\\wt\\src\\x.ts', line: 3 });
    // The file is not resolved yet, and may turn out to be outside the folder: nothing shows until it is.
    expect([read.on, read.source, read.places.a2, read.reveal]).toEqual([false, 'project', undefined, null]);
    release();
    await opening;
    expect([read.on, read.source, read.places.a2.active]).toEqual([true, 'a2', 'src/x.ts']);
    expect(app.editor.p1.places.a2.active).toBe('src/x.ts');
    expect(app.editor.p1.places.a2.open).toEqual(['src/x.ts']);
    expect(app.editor.p1.places.a2.expanded).toEqual({ src: true });
    expect(app.editor.p1.reveal).toMatchObject({ path: 'src/x.ts', line: 3 });
  });
});

describe('editor and the agent a notification or Ctrl+J brings up', () => {
  beforeEach(() => resetApp());

  it('closes the editor of the project when a notification is clicked, to show the agent', async () => {
    const { emit } = await start();
    await app.openEditor({ source: 'project' });
    expect(app.editorOn).toBe(true);
    emit({ type: 'focus', projectId: 'p1', agentId: 'a2' });
    expect(app.editorOn).toBe(false);
    expect(app.agent?.id).toBe('a2');
  });

  it('closes the editor of the project when Ctrl+J goes to the next agent waiting', async () => {
    const { emit } = await start();
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'waiting' }) });
    await app.openEditor({ source: 'project' });
    expect(app.editorOn).toBe(true);
    app.nextWaiting();
    expect(app.agent?.id).toBe('a2');
    expect(app.editorOn).toBe(false);
  });

  it('closes the editor of the project for a new agent, to show its conversation, and keeps its source', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a9', branch: 'escouade/a9', baseBranch: 'main' };
    await start({}, { create_agent: () => agent({ id: 'a9', name: 'agent-3', createdAt: 9, worktree: wt }) });
    await app.openEditor({ source: 'project', path: 'a.ts' });
    const focus = app.focusComposer;
    await app.newAgent('p1');
    expect(app.agent?.id).toBe('a9');
    expect(app.editorOn).toBe(false);
    expect(app.focusComposer).toBe(focus + 1);
    expect(app.editor.p1).toMatchObject({ source: 'project', places: { project: { active: 'a.ts' } } });
  });

  it('keeps the editor open when an agent is picked in the sidebar', async () => {
    await start();
    await app.openEditor({ source: 'project' });
    app.selectAgent('a2');
    expect(app.editorOn).toBe(true);
  });
});

describe('editor of a removed agent or project', () => {
  const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\wt', branch: 'escouade/wt', baseBranch: 'main' };
  const files = {
    fs_tree: () => ({ root: 'C:/code/demo-api', files: ['x.ts'], truncated: false }),
    fs_read: () => ({ kind: 'text', text: 'a\n', size: 2, hash: 'h1', eol: 'lf', bom: false }),
    fs_base: () => null,
    set_unsaved: () => null,
  };
  beforeEach(() => resetApp());

  it('forgets the files and tabs of a deleted agent’s worktree, and shows the project instead', async () => {
    const { emit, backend } = await start({ agents: [agent(), agent({ id: 'a2', name: 'wt', createdAt: 2, worktree: wt })] }, files);
    await app.openEditor({ source: 'project', path: 'x.ts' });
    await app.openEditor({ source: 'a2', path: 'x.ts', line: 3 });
    await trees.load('p1', 'a2');
    const mine = (await buffers.open('p1', 'a2', 'x.ts')).key;
    const kept = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(mine, 'mine\n');
    buffers.edit(kept, 'kept\n');
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 2 });
    emit({ type: 'agentRemoved', id: 'a2', projectId: 'p1' });
    expect(buffers.all[mine]).toBeUndefined();
    expect(buffers.all[kept]?.text).toBe('kept\n');
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 1 });
    expect(trees.get('p1', 'a2')).toBeUndefined();
    expect(app.editor.p1).toMatchObject({ on: true, source: 'project', reveal: null });
    expect(app.editor.p1.places.a2).toBeUndefined();
    expect(app.editor.p1.places.project.active).toBe('x.ts');
  });

  it('keeps the editor on its source when another agent is deleted', async () => {
    const { emit } = await start({ agents: [agent(), agent({ id: 'a2', name: 'wt', createdAt: 2, worktree: wt })] }, files);
    await app.openEditor({ source: 'a2', path: 'x.ts' });
    emit({ type: 'agentRemoved', id: 'a1', projectId: 'p1' });
    expect(app.editor.p1.source).toBe('a2');
    expect(app.editor.p1.places.a2.active).toBe('x.ts');
  });

  /** An editor on the worktree of `a2`, ticket `t1`'s agent, with a file edited there and one in the project. */
  async function ticketEditor(b: ReturnType<typeof board>) {
    const started = await start(
      {
        projects: [project({ board: b }), project({ id: 'p2', name: 'studio-web' })],
        agents: [agent(), agent({ id: 'a2', name: 'wt', createdAt: 2, worktree: wt })],
        tickets: [ticket({ column: 'review', agentId: 'a2' })],
      },
      files,
    );
    await app.openEditor({ source: 'a2', path: 'x.ts' });
    await trees.load('p1', 'a2');
    const mine = (await buffers.open('p1', 'a2', 'x.ts')).key;
    buffers.edit(mine, 'mine\n');
    return { ...started, mine };
  }

  it('forgets the worktree files of a ticket that goes "Terminé" by a merge removing its worktree', async () => {
    const { emit, mine } = await ticketEditor(board({ action: 'merge', cleanup: true }));
    emit({ type: 'ticket', ticket: ticket({ column: 'review', agentId: 'a2', step: 'Merge…' }) });
    expect(buffers.all[mine]).toBeDefined();
    emit({ type: 'ticket', ticket: ticket({ column: 'done', agentId: 'a2' }) });
    expect(buffers.all[mine]).toBeUndefined();
    expect(trees.get('p1', 'a2')).toBeUndefined();
    expect(app.editor.p1).toMatchObject({ source: 'project' });
    expect(app.editor.p1.places.a2).toBeUndefined();
  });

  it.each([
    ['a pull request', board({ action: 'pr', cleanup: true })],
    ['a push', board({ action: 'push', cleanup: true })],
    ['no cleanup', board({ action: 'merge', cleanup: false })],
    ['the ticket left as it is', board({ action: 'keep', cleanup: true })],
  ])('keeps the worktree files of a ticket that goes "Terminé" by %s', async (_, b) => {
    const { emit, mine } = await ticketEditor(b);
    emit({ type: 'ticket', ticket: ticket({ column: 'done', agentId: 'a2' }) });
    expect(buffers.all[mine]?.text).toBe('mine\n');
    expect(trees.get('p1', 'a2')).toBeDefined();
    expect(app.editor.p1).toMatchObject({ source: 'a2' });
  });

  it('forgets the editor of a closed project, its unsaved files included', async () => {
    const { backend } = await start({}, files);
    app.selectProject('p2');
    await app.openEditor({ projectId: 'p2', source: 'project', path: 'x.ts' });
    await trees.load('p2', 'project');
    const gone = (await buffers.open('p2', 'project', 'x.ts')).key;
    const kept = (await buffers.open('p1', 'project', 'x.ts')).key;
    buffers.edit(gone, 'mine\n');
    buffers.edit(kept, 'kept\n');
    app.forgetProject('p2');
    expect(app.projects.map((p) => p.id)).toEqual(['p1']);
    expect(app.ui.activeProject).toBe('p1');
    expect(app.editor.p2).toBeUndefined();
    expect(buffers.all[gone]).toBeUndefined();
    expect(buffers.all[kept]?.text).toBe('kept\n');
    expect(trees.get('p2', 'project')).toBeUndefined();
    expect(backend.called('set_unsaved').at(-1)?.args).toEqual({ count: 1 });
  });
});

describe('board', () => {
  beforeEach(() => resetApp());

  it('keeps the tickets of every project and follows their events', async () => {
    const { emit } = await start({ tickets: [ticket()] });
    expect(app.tickets.t1.key).toBe('DEM-1');
    emit({ type: 'ticket', ticket: ticket({ id: 't2', key: 'DEM-2' }) });
    expect(Object.keys(app.tickets)).toEqual(['t1', 't2']);
    emit({ type: 'ticketRemoved', id: 't1', projectId: 'p1' });
    expect(Object.keys(app.tickets)).toEqual(['t2']);
    emit({ type: 'project', project: project({ board: board({ prefix: 'DEM' }) }) });
    expect(app.projects[0].board.prefix).toBe('DEM');
    expect(app.reviewCount('p1')).toBe(0);
    emit({ type: 'ticket', ticket: ticket({ id: 't2', column: 'review' }) });
    expect(app.reviewCount('p1')).toBe(1);
  });

  it('knows the external accounts from the start, and says what the automatic import brought', async () => {
    const accounts = [{ service: 'jira' as const, connected: true, label: '@ada' }];
    const { emit } = await start({ accounts });
    expect(app.accounts).toEqual(accounts);
    emit({ type: 'toast', text: '2 tickets importés depuis GitHub dans demo' });
    expect(app.toasts.at(-1)).toMatchObject({ text: '2 tickets importés depuis GitHub dans demo', kind: 'info' });
    // An older backend sends none.
    await start();
    expect(app.accounts).toEqual([]);
  });

  it('knows why a board starts nothing, from the start and as the backend tells it', async () => {
    const unborn = "main n'a encore aucun commit — aucun ticket ne démarre";
    const { emit } = await start({ boardIssues: { p1: unborn } });
    expect(app.boardIssues).toEqual({ p1: unborn });
    const missing = 'Branche cible release introuvable — aucun ticket ne démarre';
    emit({ type: 'boardIssue', projectId: 'p2', issue: missing });
    emit({ type: 'boardIssue', projectId: 'p1', issue: null });
    expect(app.boardIssues).toEqual({ p2: missing });
    // A closed project takes its board's issue with it.
    app.forgetProject('p2');
    expect(app.boardIssues).toEqual({});
  });

  it('starts without tickets when the snapshot has none', async () => {
    resetApp({ tickets: [ticket()] });
    await start({ tickets: undefined });
    expect(app.tickets).toEqual({});
  });

  it('counts only the tickets "À tester" of the project asked for', async () => {
    await start({
      tickets: [
        ticket({ id: 't1', column: 'review' }),
        ticket({ id: 't2', column: 'review' }),
        ticket({ id: 't3', column: 'doing' }),
        ticket({ id: 't4', projectId: 'p2', column: 'review' }),
      ],
    });
    expect(app.reviewCount('p1')).toBe(2);
    expect(app.reviewCount('p2')).toBe(1);
    expect(app.reviewCount('p3')).toBe(0);
  });

  it('keeps the board settings of a project it knows, and ignores those of one it does not', async () => {
    const { emit } = await start();
    emit({ type: 'project', project: project({ id: 'p2', name: 'studio-web', board: board({ action: 'pr' }) }) });
    expect(app.projects.map((p) => [p.id, p.board.action])).toEqual([
      ['p1', 'merge'],
      ['p2', 'pr'],
    ]);
    emit({ type: 'project', project: project({ id: 'p9', name: 'gone' }) });
    expect(app.projects.map((p) => p.id)).toEqual(['p1', 'p2']);
  });

  it('shows the board in place of the agents until an agent, a terminal, a launch or the editor is picked', async () => {
    await start();
    app.openBoard('p1');
    expect(app.boardOn).toBe(true);
    app.selectAgent('a2');
    expect(app.boardOn).toBe(false);
    app.openBoard('p1');
    app.selectTerm('t1');
    expect(app.boardOn).toBe(false);
    app.openBoard('p1');
    app.selectLaunch('c1');
    expect(app.boardOn).toBe(false);
    app.openBoard('p1');
    await app.openEditor({ source: 'project' });
    expect(app.boardOn).toBe(false);
    app.openBoard('p1');
    app.closeBoard();
    expect(app.boardOn).toBe(false);
    // Each project keeps its own.
    app.openBoard('p2');
    expect(app.ui.activeProject).toBe('p2');
    app.selectProject('p1');
    expect(app.boardOn).toBe(false);
    app.selectProject('p2');
    expect(app.boardOn).toBe(true);
  });

  it('keeps the board when no terminal or launch is picked, and hides it behind the statistics', async () => {
    await start();
    app.openBoard('p1');
    app.selectTerm(null);
    app.selectLaunch(null);
    expect(app.boardOn).toBe(true);
    app.openStats();
    expect(app.boardOn).toBe(false);
    app.selectProject('p1');
    expect(app.boardOn).toBe(true);
  });

  it('takes the place of the editor, a terminal and a launch log of the project', async () => {
    await start();
    app.selectedTerm.p1 = 't1';
    app.selectedLaunch.p1 = 'c1';
    await app.openEditor({ source: 'project', path: 'a.ts' });
    app.openBoard('p1');
    expect(app.boardOn).toBe(true);
    expect(app.editorOn).toBe(false);
    expect(app.selectedTerm.p1).toBeNull();
    expect(app.selectedLaunch.p1).toBeNull();
    // The editor keeps its tabs for when it comes back.
    expect(app.editor.p1.places.project.open).toEqual(['a.ts']);
    expect(app.agent?.id).toBe('a1');
  });

  it('does not open a board without a project', async () => {
    await start({ projects: [], agents: [] });
    app.openBoard();
    expect(app.board).toEqual({});
  });

  it('does not count a conversation hidden by the board as seen', async () => {
    await start();
    vi.spyOn(document, 'hasFocus').mockReturnValue(true);
    app.ui.selectedAgent.p1 = 'a1';
    app.openBoard('p1');
    app.attention = { a1: true };
    app.markSeen();
    expect(app.attention).toEqual({ a1: true });
    app.closeBoard();
    app.markSeen();
    expect(app.attention).toEqual({});
    vi.restoreAllMocks();
  });

  it('flags an agent that finishes while the board hides its conversation', async () => {
    const { emit } = await start();
    vi.spyOn(document, 'hasFocus').mockReturnValue(true);
    app.openBoard('p1');
    emit({ type: 'agent', agent: agent({ status: 'running' }) });
    emit({ type: 'agent', agent: agent({ status: 'done' }) });
    expect(app.attention).toEqual({ a1: true });
    vi.restoreAllMocks();
  });

  it('tells the ticket of an agent, the one it works on first', async () => {
    await start({
      tickets: [ticket({ id: 'old', agentId: 'a1', column: 'done' }), ticket({ id: 'now', agentId: 'a1', column: 'review' })],
    });
    expect(app.ticketOf('a1')?.id).toBe('now');
    expect(app.ticketOf('a2')).toBeUndefined();
  });

  it('tells the last ticket of an agent that works on none', async () => {
    await start({
      tickets: [ticket({ id: 'first', agentId: 'a1', column: 'done' }), ticket({ id: 'last', agentId: 'a1', column: 'todo' })],
    });
    expect(app.ticketOf('a1')?.id).toBe('last');
  });

  it('gives way to the agent a notification or Ctrl+J brings up, and to a new agent', async () => {
    const { emit } = await start({}, { create_agent: () => agent({ id: 'a9', name: 'agent-3', createdAt: 9 }) });
    app.openBoard('p1');
    emit({ type: 'focus', projectId: 'p1', agentId: 'a2' });
    expect(app.boardOn).toBe(false);
    expect(app.agent?.id).toBe('a2');
    app.openBoard('p1');
    emit({ type: 'agent', agent: agent({ id: 'a1', status: 'waiting', lastActivity: 5 }) });
    app.nextWaiting();
    expect(app.boardOn).toBe(false);
    expect(app.agent?.id).toBe('a1');
    app.openBoard('p1');
    await app.newAgent('p1');
    expect(app.boardOn).toBe(false);
    expect(app.agent?.id).toBe('a9');
  });

  it('keeps the board when a notification is clicked for a project without naming an agent', async () => {
    const { emit } = await start();
    app.openBoard('p2');
    app.selectProject('p1');
    emit({ type: 'focus', projectId: 'p2', agentId: null });
    expect(app.project?.id).toBe('p2');
    expect(app.boardOn).toBe(true);
  });

  it('forgets the board of a closed project', async () => {
    await start();
    app.openBoard('p2');
    app.forgetProject('p2');
    expect(app.board.p2).toBeUndefined();
    expect(app.ui.activeProject).toBe('p1');
    expect(app.boardOn).toBe(false);
  });

  it('opens the board a ticket notification was clicked for, and the page a pull request needs', async () => {
    const { emit, backend } = await start();
    emit({ type: 'focusBoard', projectId: 'p2' });
    expect(app.ui.activeProject).toBe('p2');
    expect(app.boardOn).toBe(true);
    emit({ type: 'openUrl', url: 'https://github.com/acme/demo/compare/main...ticket/dem-1' });
    await vi.waitFor(() => expect(backend.called('plugin:opener|open_url')).toHaveLength(1));
    expect(backend.called('plugin:opener|open_url')[0].args.url).toBe('https://github.com/acme/demo/compare/main...ticket/dem-1');
    expect(app.toasts).toEqual([]);
  });

  it('shows the board of the project of a notification even when another view is up', async () => {
    const { emit } = await start();
    app.selectAgent('a2');
    emit({ type: 'focusBoard', projectId: 'p1' });
    expect(app.boardOn).toBe(true);
    expect(app.agent?.id).toBe('a2');
  });

  it('says so when the page of a pull request does not open', async () => {
    const { emit, backend } = await start(
      {},
      {
        'plugin:opener|open_url': () => {
          throw 'Aucun navigateur';
        },
      },
    );
    emit({ type: 'openUrl', url: 'https://github.com/acme/demo/compare/main...ticket/dem-1' });
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ kind: 'error', text: expect.stringContaining('Aucun navigateur') });
    expect(backend.called('plugin:opener|open_url')).toHaveLength(1);
  });

  it('tells whether the ticket of an agent is under way', async () => {
    await start({
      tickets: [
        ticket({ id: 't1', agentId: 'a1', column: 'doing' }),
        ticket({ id: 't2', agentId: 'a2', column: 'review' }),
        ticket({ id: 't3', agentId: 'b1', column: 'doing', blocked: 'Interrompu' }),
        ticket({ id: 't4', agentId: null, column: 'doing' }),
      ],
    });
    expect(app.ticketDoing('a1')).toBe(true);
    expect(app.ticketDoing('a2')).toBe(false);
    // A blocked ticket is still under way: the board tells about it.
    expect(app.ticketDoing('b1')).toBe(true);
    expect(app.ticketDoing('nobody')).toBe(false);
  });

  it('does not flag the agent of a ticket under way at each end of turn, but does for its questions', async () => {
    const { emit } = await start({ tickets: [ticket({ column: 'doing', agentId: 'a2' })] });
    vi.spyOn(document, 'hasFocus').mockReturnValue(false);
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'running' }) });
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'done' }) });
    expect(app.attention.a2).toBeUndefined();
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'running' }) });
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'error' }) });
    expect(app.attention.a2).toBeUndefined();
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'waiting' }) });
    expect(app.attention.a2).toBe(true);
    vi.restoreAllMocks();
  });

  it('still flags the end of a turn of an agent whose ticket is not under way', async () => {
    const { emit } = await start({ tickets: [ticket({ column: 'review', agentId: 'a2' })] });
    vi.spyOn(document, 'hasFocus').mockReturnValue(false);
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'running' }) });
    emit({ type: 'agent', agent: agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, status: 'done' }) });
    expect(app.attention.a2).toBe(true);
    // Neither does an agent without a ticket escape it.
    emit({ type: 'agent', agent: agent({ status: 'running' }) });
    emit({ type: 'agent', agent: agent({ status: 'done' }) });
    expect(app.attention.a1).toBe(true);
    vi.restoreAllMocks();
  });
});

describe('quitting with unsaved files', () => {
  it('asks before quitting, and quits once confirmed', async () => {
    const { emit, backend } = await start();
    emit({ type: 'quitRequested', unsaved: 2 });
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Quitter Escouade ?', confirm: 'Quitter quand même' });
    expect((app.modal as any).body).toContain('2 fichiers ne sont pas enregistrés');
    await (app.modal as any).onConfirm(false);
    expect(backend.called('quit_app')).toHaveLength(1);
  });
});
