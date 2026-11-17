import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { buffers } from '../../lib/editor/buffers.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { DeleteLoss } from '../../lib/types';
import { agent, fakeBackend, resetApp, ticket } from '../../test/ipc';
import DeleteAgentModal from './DeleteAgentModal.svelte';

const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a2', branch: 'escouade/a2', baseBranch: 'main' };
const clean: DeleteLoss = { known: true, dirty: 0, names: [], unmerged: 0, base: 'main' };

/** The dialog for `a2` (made by `a1`, « chef »), git answering `loss` (a promise: as late as it likes). */
function open(over: Parameters<typeof agent>[0] = {}, loss: DeleteLoss | Promise<DeleteLoss> = clean, extra = {}) {
  resetApp({
    agents: [agent({ id: 'a1', name: 'chef' }), agent({ id: 'a2', name: 'aide', worktree: wt, createdAt: 2, ...over })],
    ...extra,
  });
  const backend = fakeBackend({ delete_loss: () => loss });
  app.modal = { kind: 'deleteAgent', agentId: 'a2' };
  render(DeleteAgentModal, { agentId: 'a2' });
  return backend;
}
const box = () => screen.getByRole('checkbox') as HTMLInputElement;
const asked = { deleteAsk: { by: 'a1', at: 1, worktree: null } };

describe('DeleteAgentModal', () => {
  beforeEach(() => setLang('fr'));

  it('says what an agent asked to be deleted would lose, and unticks its worktree', async () => {
    const backend = open(asked, { known: true, dirty: 2, names: ['a.txt', 'b.txt'], unmerged: 3, base: 'main' });
    expect(screen.getByRole('dialog', { name: "Supprimer l'agent « aide » ?" })).toBeInTheDocument();
    expect(screen.getByText("Demandé par l'agent « chef », qui l'a créé.")).toBeInTheDocument();
    expect(backend.called('delete_loss')[0].args).toEqual({ id: 'a2' });
    expect(await screen.findByText('2 fichiers modifiés non commités : a.txt, b.txt')).toBeInTheDocument();
    expect(screen.getByText('3 commits pas encore fusionnés dans main')).toBeInTheDocument();
    expect(box()).not.toBeChecked();
    expect(screen.getByText('La case du worktree est décochée : son travail en cours resterait.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Supprimer' }));
    await waitFor(() => expect(backend.called('delete_agent')).toHaveLength(1));
    expect(backend.called('delete_agent')[0].args).toEqual({ id: 'a2', removeWorktree: false });
    expect(app.modal).toBeNull();
  });

  it('ticks the worktree when nothing would be lost, and says more files than it names', async () => {
    open({}, clean);
    await waitFor(() => expect(box()).toBeChecked());
    expect(screen.getByText('Rien de non commité ni de non fusionné.')).toBeInTheDocument();
    // Opened by the user: no word of an agent asking, no « Refuser ».
    expect(screen.getByRole('dialog', { name: "Supprimer l'agent « aide » ?" })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Refuser' })).toBeNull();
    expect(document.querySelector('[data-delete-by]')).toBeNull();
  });

  it('keeps the worktree as the asking agent wants it, when nothing would be lost', async () => {
    open({ deleteAsk: { by: 'a1', at: 1, worktree: false } }, clean);
    await waitFor(() => expect(box()).not.toBeDisabled());
    expect(box()).not.toBeChecked();
  });

  it('names five files and counts the rest', async () => {
    open({}, { known: true, dirty: 7, names: ['1', '2', '3', '4', '5'], unmerged: 0, base: 'main' });
    expect(await screen.findByText('7 fichiers modifiés non commités : 1, 2, 3, 4, 5, et 2 autres')).toBeInTheDocument();
  });

  it('says the state of the worktree unknown when git is late, and unticks it', async () => {
    let answer!: (l: DeleteLoss) => void;
    open({}, new Promise((r) => (answer = r)));
    // Read while git answers: the box waits, and the dialog does not.
    expect(screen.getByText('Lecture du worktree…')).toBeInTheDocument();
    expect(box()).toBeDisabled();
    answer({ ...clean, known: false });
    expect(await screen.findByText('État du worktree inconnu : git n’a pas répondu à temps.')).toBeInTheDocument();
    expect(box()).not.toBeChecked();
  });

  it('says a turn under way, a request waiting and its ticket', async () => {
    open({ status: 'waiting' }, clean, { tickets: [ticket({ agentId: 'a2', key: 'DEM-9', column: 'doing' })] });
    expect(screen.getByText('Une demande en attente de ta réponse')).toBeInTheDocument();
    expect(screen.getByText('Lié au ticket DEM-9')).toBeInTheDocument();
    resetApp();
  });

  it('says a turn under way', () => {
    open({ status: 'running' });
    expect(screen.getByText('Un tour en cours, interrompu')).toBeInTheDocument();
  });

  it('refuses from the dialog of a deletion asked for', async () => {
    const backend = open(asked);
    await userEvent.click(screen.getByRole('button', { name: 'Refuser' }));
    await waitFor(() => expect(backend.called('refuse_delete')).toHaveLength(1));
    expect(backend.called('refuse_delete')[0].args).toEqual({ id: 'a2' });
    expect(backend.called('delete_agent')).toHaveLength(0);
    expect(app.modal).toBeNull();
  });

  it('confirms nothing on Entrée: the focus is the dialog’s, not its button', async () => {
    const backend = open(asked);
    await waitFor(() => expect(box()).not.toBeDisabled());
    const dialog = screen.getByRole('dialog');
    expect(document.activeElement).toBe(dialog);
    await fireEvent.keyDown(dialog, { key: 'Enter' });
    await userEvent.keyboard('{Enter}');
    await new Promise((r) => setTimeout(r));
    expect(backend.called('delete_agent')).toHaveLength(0);
    expect(app.modal).not.toBeNull();
  });

  it('says the branch of an existing one is kept', async () => {
    open({ worktree: { ...wt, branch: 'feat/login', existing: true } });
    expect(await screen.findByText('Supprimer aussi le worktree (la branche feat/login est conservée)')).toBeInTheDocument();
  });

  it('warns that the files of its worktree unsaved in the editor are lost', async () => {
    resetApp({ agents: [agent({ id: 'a2', name: 'aide', worktree: wt })] });
    fakeBackend({
      fs_read: () => ({ kind: 'text', text: 'a\n', size: 2, hash: 'h1', eol: 'lf', bom: false }),
      fs_base: () => null,
      set_unsaved: () => null,
    });
    for (const path of ['x.ts', 'y.ts']) buffers.edit((await buffers.open('p1', 'a2', path)).key, 'mine\n');
    app.modal = { kind: 'deleteAgent', agentId: 'a2' };
    render(DeleteAgentModal, { agentId: 'a2' });
    expect(screen.getByText(/\. 2 fichiers non enregistrés dans l’éditeur seront perdus\.$/)).toBeInTheDocument();
  });

  it('reads in English', async () => {
    setLang('en');
    open(asked, { known: true, dirty: 1, names: ['a.txt'], unmerged: 1, base: 'main' });
    expect(screen.getByRole('dialog', { name: 'Delete the agent “aide”?' })).toBeInTheDocument();
    expect(screen.getByText('Asked by the agent “chef”, which made it.')).toBeInTheDocument();
    expect(await screen.findByText('1 changed file not committed: a.txt')).toBeInTheDocument();
    expect(screen.getByText('1 commit not merged into main yet')).toBeInTheDocument();
    expect(screen.getByText('Also delete the worktree and the branch escouade/a2')).toBeInTheDocument();
    expect(
      screen.getByText(
        'The Claude process is stopped and the conversation is removed from the app (the Claude Code session stays on disk).',
      ),
    ).toBeInTheDocument();
  });
});
