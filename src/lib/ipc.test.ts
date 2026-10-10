import { describe, expect, it } from 'vitest';
import { board, fakeBackend, project, ticket } from '../test/ipc';
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
