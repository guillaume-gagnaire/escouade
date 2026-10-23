import { describe, expect, it } from 'vitest';
import { agent, board, fakeBackend, project, ticket } from '../test/ipc';
import { api } from './ipc';
import type { SearchResult } from './types';

describe('board commands', () => {
  const draft = { title: 'Ajouter le fichier', description: '', criteria: ['Le fichier existe'], maxLoops: 5, after: ['t2'] };

  it('asks the backend for tickets with the names its commands take', async () => {
    const backend = fakeBackend({ ticket_create: () => ticket(), ticket_update: () => ticket({ title: 'Autre' }) });
    expect((await api.ticketCreate('p1', draft)).key).toBe('DEM-1');
    expect((await api.ticketUpdate('t1', draft)).title).toBe('Autre');
    await api.ticketDelete('t1');
    await api.ticketPrioritize('t1');
    await api.ticketStart('t1');
    expect(backend.calls).toEqual([
      { cmd: 'ticket_create', args: { projectId: 'p1', draft } },
      { cmd: 'ticket_update', args: { id: 't1', draft } },
      { cmd: 'ticket_delete', args: { id: 't1' } },
      { cmd: 'ticket_prioritize', args: { id: 't1' } },
      { cmd: 'ticket_start', args: { id: 't1' } },
    ]);
  });

  it('asks the backend to resume, validate, send back, resolve or dismiss a ticket', async () => {
    const backend = fakeBackend();
    await api.ticketResume('t1');
    await api.ticketApprove('t1');
    await api.ticketReject('t1', 'le bouton est mal placé');
    await api.ticketResolveConflict('t1');
    await api.ticketDismiss('t1');
    expect(backend.calls).toEqual([
      { cmd: 'ticket_resume', args: { id: 't1' } },
      { cmd: 'ticket_approve', args: { id: 't1' } },
      { cmd: 'ticket_reject', args: { id: 't1', comment: 'le bouton est mal placé' } },
      { cmd: 'ticket_resolve_conflict', args: { id: 't1' } },
      { cmd: 'ticket_dismiss', args: { id: 't1' } },
    ]);
  });

  it('saves the board settings and lists the branches of a project', async () => {
    const settings = board({ action: 'pr', target: 'main' });
    const backend = fakeBackend({ board_set: () => project({ board: settings }), git_branches: () => ['main', 'dev'] });
    expect((await api.boardSet('p1', settings)).board.action).toBe('pr');
    expect(await api.gitBranches('p1')).toEqual(['main', 'dev']);
    expect(backend.calls).toEqual([
      { cmd: 'board_set', args: { projectId: 'p1', settings } },
      { cmd: 'git_branches', args: { projectId: 'p1' } },
    ]);
  });

  it('starts an agent on an existing branch and integrates its base with the names its commands take', async () => {
    const backend = fakeBackend({
      create_agent_on_branch: () => agent({ id: 'a9' }),
      integrate_base: () => ({ kind: 'conflict', files: ['src/a.ts'], rebase: false }),
    });
    expect((await api.createAgentOnBranch('p1', 'origin/feat', 'opus')).id).toBe('a9');
    await api.createAgentOnBranch('p1', 'feat');
    await api.createAgentOnNewBranch('p1', 'feat/signup');
    await api.createAgentOnNewBranch('p1', 'fix/urgent', 'origin/hotfix');
    expect(await api.integrateBase('a1')).toEqual({ kind: 'conflict', files: ['src/a.ts'], rebase: false });
    expect(backend.calls).toEqual([
      { cmd: 'create_agent_on_branch', args: { projectId: 'p1', branch: 'origin/feat', model: 'opus', engine: null } },
      { cmd: 'create_agent_on_branch', args: { projectId: 'p1', branch: 'feat', model: null, engine: null } },
      {
        cmd: 'create_agent_on_branch',
        args: { projectId: 'p1', branch: 'feat/signup', model: null, engine: null, create: true, base: null },
      },
      {
        cmd: 'create_agent_on_branch',
        args: { projectId: 'p1', branch: 'fix/urgent', model: null, engine: null, create: true, base: 'origin/hotfix' },
      },
      { cmd: 'integrate_base', args: { id: 'a1' } },
    ]);
  });
});

describe('code search', () => {
  it('asks the backend to search the files of a source with the names its command takes', async () => {
    const result: SearchResult = {
      matches: [{ path: 'src/a.ts', line: 3, col: 7, text: 'const foo = 1;', offset: 0, ranges: [[6, 9]] }],
      truncated: false,
      timedOut: false,
    };
    const backend = fakeBackend({ code_search: () => result });
    const query = { pattern: 'foo', regex: false, caseSensitive: true, wholeWord: true, maxResults: 2000 };
    expect(await api.codeSearch('p1', 'a2', query)).toEqual(result);
    expect(backend.calls).toEqual([{ cmd: 'code_search', args: { projectId: 'p1', agentId: 'a2', query } }]);
  });
});

describe('test launch commands', () => {
  it('asks the backend to prepare a launch, start a step of a recipe and probe an address', async () => {
    const info = { id: 't1', projectId: 'p1', name: 'web', shell: 'pwsh' };
    const backend = fakeBackend({ test_run_start: () => info, http_ready: () => true });
    await api.agentPrepareLaunch('a1');
    const out: ArrayBuffer[] = [];
    expect(await api.testRunStart({ agentId: 'a1', kind: 'run', index: 1, cols: 100, rows: 30, cursorRow: 4 }, (d) => out.push(d))).toEqual(
      info,
    );
    expect(await api.httpReady('http://localhost:4101')).toBe(true);
    expect(backend.calls.map((c) => c.cmd)).toEqual(['agent_prepare_launch', 'test_run_start', 'http_ready']);
    expect(backend.calls[0].args).toEqual({ id: 'a1' });
    expect(backend.calls[1].args).toMatchObject({ agentId: 'a1', kind: 'run', index: 1, cols: 100, rows: 30, cursorRow: 4 });
    // The output comes back through a channel the backend writes to.
    expect(backend.calls[1].args.output).toBeDefined();
    expect(backend.calls[2].args).toEqual({ url: 'http://localhost:4101' });
  });

  it('sends the recipe the user read to be approved', async () => {
    const backend = fakeBackend({ test_recipe_approve: () => undefined });
    const recipe = { prepare: [{ command: 'npm install', dir: 'web' }], processes: [], open: '' };
    await api.testRecipeApprove('a1', recipe);
    expect(backend.calls).toEqual([{ cmd: 'test_recipe_approve', args: { agentId: 'a1', recipe } }]);
  });
});

describe('project commands', () => {
  it('adds a project with or without the history of its folder, and closes it keeping that history or not', async () => {
    const backend = fakeBackend({ create_project: () => project() });
    const a = { path: 'C:\code\demo-api', name: 'demo-api', color: 'red', worktreePerAgent: false, firstAgent: null };
    await api.createProject({ ...a, restore: true });
    await api.createProject({ ...a, restore: false });
    await api.removeProject('p1', false);
    await api.removeProject('p1', true);
    expect(backend.calls).toEqual([
      { cmd: 'create_project', args: { ...a, restore: true } },
      { cmd: 'create_project', args: { ...a, restore: false } },
      { cmd: 'remove_project', args: { id: 'p1', erase: false } },
      { cmd: 'remove_project', args: { id: 'p1', erase: true } },
    ]);
  });

  it('names the history to look at, and to reopen, only when the window was opened on one', async () => {
    const backend = fakeBackend({ inspect_folder: () => null, create_project: () => project() });
    const a = { path: 'C:\\code\\demo-api', name: 'demo-api', color: 'red', worktreePerAgent: false, firstAgent: null, restore: true };
    await api.inspectFolder('C:\\code\\demo-api');
    await api.inspectFolder('C:\\code\\demo-api', 'p7');
    await api.createProject(a);
    await api.createProject({ ...a, historyId: 'p7' });
    expect(backend.calls).toEqual([
      { cmd: 'inspect_folder', args: { path: 'C:\\code\\demo-api' } },
      { cmd: 'inspect_folder', args: { path: 'C:\\code\\demo-api', historyId: 'p7' } },
      { cmd: 'create_project', args: a },
      { cmd: 'create_project', args: { ...a, historyId: 'p7' } },
    ]);
    // Not a key left empty: the backend reads no history id as « the latest of the folder ».
    expect(Object.keys(backend.calls[0].args)).toEqual(['path']);
    expect(Object.keys(backend.calls[2].args)).not.toContain('historyId');
  });

  it('lists the closed projects and erases the history of one', async () => {
    const closed = { id: 'p9', name: 'old', color: 'red', path: 'C:\code\old', worktreePerAgent: false, agents: 2, tickets: 3 };
    const backend = fakeBackend({ closed_projects: () => [closed] });
    expect(await api.closedProjects()).toEqual([closed]);
    await api.eraseClosedProject('p9');
    expect(backend.calls).toEqual([
      { cmd: 'closed_projects', args: {} },
      { cmd: 'erase_closed_project', args: { id: 'p9' } },
    ]);
  });
});

describe('terminal writes and reports', () => {
  it('gives the number the backend gave a write', async () => {
    const backend = fakeBackend({ term_write: () => 41 });
    await expect(api.termWrite('t1', 'a')).resolves.toBe(41);
    expect(backend.calls).toEqual([{ cmd: 'term_write', args: { id: 't1', data: 'a' } }]);
  });

  it('tells the backend which renderer a terminal got and how slow an echo was, with the names its command takes', async () => {
    const backend = fakeBackend({ term_report: () => undefined });
    await api.termReport('t1', { kind: 'renderer', renderer: 'webgl', detail: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4080)' });
    await api.termReport('t1', { kind: 'renderer', renderer: 'dom' });
    await api.termReport('t1', { kind: 'slowEcho', seq: 41, bytes: 1, sentToReceived: 1500.5, receivedToPainted: 18.25 });
    await api.termReport('t1', {
      kind: 'slowEcho',
      seq: 42,
      bytes: 2,
      sentToReceived: 1500.5,
      receivedToPainted: 18.25,
      sentToAcked: 0.75,
      outputEnd: 4096,
    });
    expect(backend.calls).toEqual([
      {
        cmd: 'term_report',
        args: { id: 't1', report: { kind: 'renderer', renderer: 'webgl', detail: 'ANGLE (NVIDIA, NVIDIA GeForce RTX 4080)' } },
      },
      { cmd: 'term_report', args: { id: 't1', report: { kind: 'renderer', renderer: 'dom' } } },
      {
        cmd: 'term_report',
        args: { id: 't1', report: { kind: 'slowEcho', seq: 41, bytes: 1, sentToReceived: 1500.5, receivedToPainted: 18.25 } },
      },
      {
        cmd: 'term_report',
        args: {
          id: 't1',
          report: {
            kind: 'slowEcho',
            seq: 42,
            bytes: 2,
            sentToReceived: 1500.5,
            receivedToPainted: 18.25,
            sentToAcked: 0.75,
            outputEnd: 4096,
          },
        },
      },
    ]);
  });
});

describe('attached files', () => {
  const path = 'C:\\data\\attachments\\a1\\1f2e3d4c\\rapport.xlsx';
  const copy = { name: 'rapport.xlsx', path, claudePath: path, size: 3, origin: false };

  it('opens, fills, finishes, cancels and discards a copy with the names its commands take', async () => {
    const backend = fakeBackend({ attach_open: () => 'h1', attach_finish: () => copy });
    expect(await api.attachOpen('a1', 'rapport.xlsx', 3)).toBe('h1');
    await api.attachChunk('h1', new Uint8Array([1, 2, 3]));
    expect(await api.attachFinish('h1')).toEqual(copy);
    await api.attachCancel('h1');
    await api.attachDiscard('a1', path);
    expect(backend.calls.map((c) => c.cmd)).toEqual(['attach_open', 'attach_chunk', 'attach_finish', 'attach_cancel', 'attach_discard']);
    expect(backend.calls[0].args).toEqual({ owner: 'a1', name: 'rapport.xlsx', size: 3 });
    expect(backend.calls[2].args).toEqual({ handle: 'h1' });
    expect(backend.calls[3].args).toEqual({ handle: 'h1' });
    expect(backend.calls[4].args).toEqual({ owner: 'a1', path });
  });

  it('sends a chunk as the raw bytes, its handle in a header: no base64 in a JSON', async () => {
    const backend = fakeBackend();
    const seen: { cmd: string; args: unknown; options: unknown }[] = [];
    const internals = (window as any).__TAURI_INTERNALS__;
    const invoke = internals.invoke;
    internals.invoke = (cmd: string, args: unknown, options: unknown) => {
      seen.push({ cmd, args, options });
      return invoke(cmd, args, options);
    };
    const bytes = new Uint8Array([1, 2, 3]);
    await api.attachChunk('h1', bytes);
    expect(seen).toEqual([{ cmd: 'attach_chunk', args: bytes, options: { headers: { 'x-handle': 'h1' } } }]);
    expect(backend.called('attach_chunk')).toHaveLength(1);
  });

  it('attaches a file where it is by the id the host holds for it, and asks the host where a file of the window comes from', async () => {
    const original = { ...copy, path: 'C:\\Users\\Jean\\rapport.xlsx', origin: true };
    const backend = fakeBackend({ attach_origin: () => original, file_origin: () => original.path });
    expect(await api.attachOrigin('a1', '3f2a')).toEqual(original);
    expect(await api.fileOrigin('3f2a')).toBe(original.path);
    expect(backend.calls).toEqual([
      { cmd: 'attach_origin', args: { owner: 'a1', id: '3f2a' } },
      { cmd: 'file_origin', args: { id: '3f2a' } },
    ]);
  });

  it('sends the files of a message beside its attachments, none when it has none', async () => {
    const backend = fakeBackend();
    await api.sendMessage('a1', 'Lis ça', [], [copy]);
    await api.sendMessage('a1', 'Salut');
    expect(backend.calls).toEqual([
      { cmd: 'send_message', args: { id: 'a1', text: 'Lis ça', attachments: [], files: [copy] } },
      { cmd: 'send_message', args: { id: 'a1', text: 'Salut', attachments: [], files: [] } },
    ]);
  });
});
