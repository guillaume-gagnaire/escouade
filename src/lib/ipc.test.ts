import { describe, expect, it } from 'vitest';
import { board, fakeBackend, project, ticket } from '../test/ipc';
import { api } from './ipc';

describe('board commands', () => {
  const draft = { title: 'Ajouter le fichier', description: '', criteria: ['Le fichier existe'], maxLoops: 5 };

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
