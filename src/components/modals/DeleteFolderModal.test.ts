import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import DeleteFolderModal from './DeleteFolderModal.svelte';

const wt = { path: 'C:\\w', branch: 'escouade/x', baseBranch: 'main' };

function open(agents = [agent({ id: 'a1', name: 'un' }), agent({ id: 'a2', name: 'deux' })], tickets = [] as any[]) {
  resetApp({
    projects: [
      project({
        agentFolders: [{ id: 'f1', name: 'Auth', named: true, principal: null, agents: agents.map((a) => a.id) }],
        agentOrder: ['f1'],
      }),
    ],
    agents,
    tickets,
  });
  const backend = fakeBackend();
  app.modal = { kind: 'deleteFolder', projectId: 'p1', folderId: 'f1' };
  render(DeleteFolderModal, { projectId: 'p1', folderId: 'f1' });
  return backend;
}
const confirm = () => screen.getByRole('button', { name: 'Supprimer le dossier' });
const order = (b: ReturnType<typeof fakeBackend>) => b.calls.map((c) => c.cmd);

describe('DeleteFolderModal', () => {
  beforeEach(() => setLang('fr'));

  it('takes the agents out by default, with a button that is not red', () => {
    open();
    expect(screen.getByRole('dialog', { name: 'Supprimer le dossier « Auth » ?' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Sortir les agents du dossier' })).toBeChecked();
    expect(confirm()).not.toHaveClass('danger');
    expect(screen.queryByRole('checkbox')).toBeNull();
  });

  it('turns the button red on « Supprimer », with the worktrees box only when an agent has one', async () => {
    open([agent({ id: 'a1', worktree: wt }), agent({ id: 'a2' })]);
    await userEvent.click(screen.getByRole('radio', { name: 'Supprimer les 2 agents' }));
    expect(confirm()).toHaveClass('danger');
    // One of the two has a worktree: its own, in the singular.
    expect(screen.getByRole('checkbox', { name: 'Supprimer aussi son worktree' })).toBeChecked();
  });

  it('has no worktrees box when no agent has a worktree', async () => {
    open();
    await userEvent.click(screen.getByRole('radio', { name: 'Supprimer les 2 agents' }));
    expect(screen.queryByRole('checkbox')).toBeNull();
  });

  it('takes the agents out: agent_folder_delete alone', async () => {
    const b = open();
    await userEvent.click(confirm());
    await waitFor(() => expect(app.modal).toBeNull());
    expect(order(b)).toEqual(['agent_folder_delete']);
  });

  it('archives each agent, then deletes the folder', async () => {
    const b = open();
    await userEvent.click(screen.getByRole('radio', { name: 'Archiver les 2 agents' }));
    await userEvent.click(confirm());
    await waitFor(() => expect(app.modal).toBeNull());
    expect(order(b)).toEqual(['archive_agent', 'archive_agent', 'agent_folder_delete']);
    expect(b.called('archive_agent').map((c) => c.args)).toEqual([
      { id: 'a1', archived: true },
      { id: 'a2', archived: true },
    ]);
  });

  it('deletes each agent with its worktree, then the folder', async () => {
    const b = open([agent({ id: 'a1', worktree: wt }), agent({ id: 'a2' })]);
    await userEvent.click(screen.getByRole('radio', { name: 'Supprimer les 2 agents' }));
    await userEvent.click(confirm());
    await waitFor(() => expect(app.modal).toBeNull());
    expect(order(b)).toEqual(['delete_agent', 'delete_agent', 'agent_folder_delete']);
    expect(b.called('delete_agent').map((c) => c.args)).toEqual([
      { id: 'a1', removeWorktree: true },
      { id: 'a2', removeWorktree: false },
    ]);
  });

  it('keeps the worktrees when the box is unchecked', async () => {
    const b = open([agent({ id: 'a1', worktree: wt })]);
    await userEvent.click(screen.getByRole('radio', { name: 'Supprimer l’agent' }));
    await userEvent.click(screen.getByRole('checkbox'));
    await userEvent.click(confirm());
    await waitFor(() => expect(app.modal).toBeNull());
    expect(b.called('delete_agent')[0].args).toEqual({ id: 'a1', removeWorktree: false });
  });

  it('flags a ticket under way under « Archiver »', async () => {
    open([agent({ id: 'a1' }), agent({ id: 'a2' })], [ticket({ id: 't1', key: 'DEM-7', column: 'doing', agentId: 'a1' })]);
    expect(screen.queryByText(/DEM-7/)).toBeNull();
    await userEvent.click(screen.getByRole('radio', { name: 'Archiver les 2 agents' }));
    expect(screen.getByText(/DEM-7/)).toBeInTheDocument();
  });

  it('never touches an archived agent of the folder', async () => {
    const b = open([agent({ id: 'a1' }), agent({ id: 'a2', archived: true })]);
    expect(screen.getByRole('radio', { name: 'Archiver l’agent' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('radio', { name: 'Archiver l’agent' }));
    await userEvent.click(confirm());
    await waitFor(() => expect(app.modal).toBeNull());
    expect(b.called('archive_agent').map((c) => c.args.id)).toEqual(['a1']);
  });

  it('closes without doing anything when the folder is gone', async () => {
    resetApp({ projects: [project()] });
    const b = fakeBackend();
    app.modal = { kind: 'deleteFolder', projectId: 'p1', folderId: 'nope' };
    render(DeleteFolderModal, { projectId: 'p1', folderId: 'nope' });
    await waitFor(() => expect(app.modal).toBeNull());
    expect(b.calls).toHaveLength(0);
  });
});
