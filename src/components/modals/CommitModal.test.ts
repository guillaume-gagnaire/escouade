import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app, type Modal } from '../../lib/state.svelte';
import type { CommitScope, FileChange } from '../../lib/types';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import CommitModal from './CommitModal.svelte';

const change = (path: string, status: FileChange['status'] = 'M'): FileChange => ({
  path,
  status,
  add: 2,
  del: 1,
  agentId: 'a1',
  inWorktree: false,
});
const SCOPE: CommitScope = { files: [change('src/auth.ts'), change('src/new.ts', 'A')], leftOut: [] };
const PATHS = ['src/auth.ts', 'src/new.ts'];

const field = () => screen.getByRole('textbox', { name: 'Message' });
const commitButton = () => screen.getByRole('button', { name: 'Commiter' });
const WRITING = 'Haiku rédige le message…';

describe('CommitModal', () => {
  beforeEach(() => {
    resetApp({ agents: [agent()] });
    app.modal = { kind: 'commit', projectId: 'p1', agentId: 'a1' };
  });

  it('shows the agent’s files with Haiku’s proposal, and commits only on « Commiter »', async () => {
    const backend = fakeBackend({
      commit_preview: () => SCOPE,
      commit_propose: () => 'feat(auth): connexion par jeton',
      commit_direct: () => 'abc1234',
    });
    render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    expect(screen.getByRole('dialog', { name: 'Commit' })).toBeInTheDocument();
    expect(screen.getByText('Modifications de refacto-auth')).toBeInTheDocument();
    await waitFor(() => expect(field()).toHaveValue('feat(auth): connexion par jeton'));
    const files = within(screen.getByRole('list', { name: 'Fichiers du commit' })).getAllByRole('listitem');
    expect(files.map((f) => f.textContent?.replace(/\s+/g, ' ').trim())).toEqual(['M src/auth.ts', 'A src/new.ts']);
    expect(backend.called('commit_preview')[0].args).toEqual({ projectId: 'p1', agentId: 'a1' });
    expect(backend.called('commit_propose')[0].args).toEqual({ projectId: 'p1', agentId: 'a1', paths: PATHS });
    // Proposed, never committed by itself.
    expect(backend.called('commit_direct')).toHaveLength(0);

    await userEvent.type(field(), '{Enter}{Enter}Relu avant le commit.');
    expect(backend.called('commit_direct')).toHaveLength(0);
    await userEvent.click(commitButton());
    expect(backend.called('commit_direct').map((c) => c.args)).toEqual([
      { projectId: 'p1', agentId: 'a1', paths: PATHS, message: 'feat(auth): connexion par jeton\n\nRelu avant le commit.' },
    ]);
    expect(app.modal).toBeNull();
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Commit abc1234 créé', kind: 'ok' });
  });

  it('lists 500 files at most, and counts the others in the singular or the plural', async () => {
    const many = (n: number): CommitScope => ({ files: Array.from({ length: n }, (_, i) => change(`src/f${i}.ts`)), leftOut: [] });
    fakeBackend({ commit_preview: () => many(501), commit_propose: () => 'feat: tout' });
    const { unmount } = render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    expect(await screen.findByText('… et 1 autre fichier')).toBeInTheDocument();
    expect(within(screen.getByRole('list', { name: 'Fichiers du commit' })).getAllByRole('listitem')).toHaveLength(500);
    unmount();

    fakeBackend({ commit_preview: () => many(503), commit_propose: () => 'feat: tout' });
    render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    expect(await screen.findByText('… et 3 autres fichiers')).toBeInTheDocument();
  });

  it('keeps the field editable while Haiku writes, and keeps what is typed then', async () => {
    let answer: (m: string) => void = () => {};
    fakeBackend({ commit_preview: () => SCOPE, commit_propose: () => new Promise<string>((r) => (answer = r)) });
    render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    expect(await screen.findByText(WRITING)).toBeInTheDocument();
    expect(field()).toBeEnabled();
    expect(commitButton()).toBeDisabled();
    await userEvent.type(field(), 'fix: écrit à la main');
    expect(commitButton()).toBeEnabled();
    answer('feat: arrivé trop tard');
    await waitFor(() => expect(screen.queryByText(WRITING)).not.toBeInTheDocument());
    expect(field()).toHaveValue('fix: écrit à la main');
  });

  it('cannot commit an empty message', async () => {
    const backend = fakeBackend({ commit_preview: () => SCOPE, commit_propose: () => 'feat: proposé' });
    render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    await waitFor(() => expect(field()).toHaveValue('feat: proposé'));
    expect(commitButton()).toBeEnabled();
    await userEvent.clear(field());
    expect(commitButton()).toBeDisabled();
    await userEvent.type(field(), '   ');
    expect(commitButton()).toBeDisabled();
    await userEvent.click(commitButton());
    expect(backend.called('commit_direct')).toHaveLength(0);
  });

  it('asks Haiku again with « Régénérer »', async () => {
    const answers = ['feat: une première idée', 'feat: une autre idée'];
    const backend = fakeBackend({ commit_preview: () => SCOPE, commit_propose: () => answers.shift() });
    render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    await waitFor(() => expect(field()).toHaveValue('feat: une première idée'));
    await userEvent.click(screen.getByRole('button', { name: 'Régénérer' }));
    await waitFor(() => expect(field()).toHaveValue('feat: une autre idée'));
    expect(backend.called('commit_propose')).toHaveLength(2);
    expect(backend.called('commit_direct')).toHaveLength(0);
  });

  it('says why there is no proposal, and leaves the field empty', async () => {
    fakeBackend({
      commit_preview: () => SCOPE,
      commit_propose: () => {
        throw 'claude introuvable';
      },
    });
    render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    expect(await screen.findByText('Pas de proposition : claude introuvable.')).toBeInTheDocument();
    expect(screen.queryByText(WRITING)).not.toBeInTheDocument();
    expect(field()).toHaveValue('');
    expect(commitButton()).toBeDisabled();
  });

  it('shows git’s refusal in the modal, which stays open', async () => {
    fakeBackend({
      commit_preview: () => SCOPE,
      commit_propose: () => 'feat: proposé',
      commit_direct: () => {
        throw 'lint en échec';
      },
    });
    render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    await waitFor(() => expect(field()).toHaveValue('feat: proposé'));
    await userEvent.click(commitButton());
    expect(await screen.findByRole('alert')).toHaveTextContent('lint en échec');
    expect(app.modal).toMatchObject({ kind: 'commit' });
    expect(field()).toHaveValue('feat: proposé');
  });

  it('cannot be closed while the commit runs, and then shows git’s refusal', async () => {
    let refuse: (e: string) => void = () => {};
    fakeBackend({
      commit_preview: () => SCOPE,
      commit_propose: () => 'feat: proposé',
      commit_direct: () => new Promise<string>((_, reject) => (refuse = reject)),
    });
    render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    await waitFor(() => expect(field()).toHaveValue('feat: proposé'));
    await userEvent.click(commitButton());
    const running = screen.getByRole('button', { name: 'Commit…' });
    expect(running).toBeDisabled();
    // Escape, « Annuler », the close button and the overlay leave it open.
    await userEvent.keyboard('{Escape}');
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    await userEvent.click(screen.getByRole('button', { name: 'Fermer' }));
    const overlay = screen.getByRole('dialog', { name: 'Commit' }).parentElement!;
    await fireEvent.mouseDown(overlay);
    await fireEvent.click(overlay);
    expect(app.modal).toMatchObject({ kind: 'commit' });
    refuse('lint en échec');
    expect(await screen.findByRole('alert')).toHaveTextContent('lint en échec');
    expect(commitButton()).toBeEnabled();
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
  });

  it('tells a refusal that arrives once the window is gone', async () => {
    let refuse: (e: string) => void = () => {};
    fakeBackend({
      commit_preview: () => SCOPE,
      commit_propose: () => 'feat: proposé',
      commit_direct: () => new Promise<string>((_, reject) => (refuse = reject)),
    });
    const { unmount } = render(CommitModal, { projectId: 'p1', agentId: 'a1' });
    await waitFor(() => expect(field()).toHaveValue('feat: proposé'));
    await userEvent.click(commitButton());
    // Another modal took its place.
    unmount();
    refuse('lint en échec');
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'lint en échec', kind: 'error' }));
  });

  it('commits the project’s own checkout, and names the copied files it leaves out', async () => {
    const backend = fakeBackend({
      commit_preview: () => ({ files: [change('README.md')], leftOut: ['.env'] }),
      commit_propose: () => 'docs: le README',
    });
    app.modal = { kind: 'commit', projectId: 'p1', agentId: null };
    render(CommitModal, { projectId: 'p1', agentId: null });
    expect(screen.getByText('Modifications de demo-api')).toBeInTheDocument();
    expect(await screen.findByText('Jamais commité : .env (copié dans les worktrees).')).toBeInTheDocument();
    expect(backend.called('commit_preview')[0].args).toEqual({ projectId: 'p1', agentId: null });
    await waitFor(() => expect(field()).toHaveValue('docs: le README'));
  });

  describe('closed with a message written', () => {
    const ASK = { kind: 'confirm', title: 'Abandonner le message ?', confirm: 'Abandonner', danger: true };

    it('closes at once on the proposal as it came, or on an empty message', async () => {
      fakeBackend({ commit_preview: () => SCOPE, commit_propose: () => 'feat: proposé' });
      const { unmount } = render(CommitModal, { projectId: 'p1', agentId: 'a1' });
      await waitFor(() => expect(field()).toHaveValue('feat: proposé'));
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toBeNull();
      unmount();

      app.modal = { kind: 'commit', projectId: 'p1', agentId: 'a1' };
      render(CommitModal, { projectId: 'p1', agentId: 'a1' });
      await waitFor(() => expect(field()).toHaveValue('feat: proposé'));
      await userEvent.clear(field());
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      expect(app.modal).toBeNull();
    });

    it('asks before Escape, « Annuler » or the overlay drops it, and comes back to it with its message', async () => {
      const backend = fakeBackend({ commit_preview: () => SCOPE, commit_propose: () => 'feat: proposé' });
      const commit = app.modal;
      const { unmount } = render(CommitModal, { projectId: 'p1', agentId: 'a1' });
      await waitFor(() => expect(field()).toHaveValue('feat: proposé'));
      await userEvent.type(field(), ' (relu)');
      for (const leave of [
        () => userEvent.keyboard('{Escape}'),
        () => userEvent.click(screen.getByRole('button', { name: 'Annuler' })),
        async () => {
          const overlay = screen.getByRole('dialog', { name: 'Commit' }).parentElement!;
          await fireEvent.mouseDown(overlay);
          await fireEvent.click(overlay);
        },
      ]) {
        app.modal = commit;
        await leave();
        expect(app.modal).toMatchObject(ASK);
      }
      // As App.svelte does: the confirmation takes the window's place, and « Annuler » there brings it back.
      unmount();
      const ask = app.modal as Extract<Modal, { kind: 'confirm' }>;
      app.modal = null;
      ask.onCancel!();
      expect(app.modal).toMatchObject({ kind: 'commit', projectId: 'p1', agentId: 'a1' });
      // Read again: the type checker takes it for the null set above.
      const back = app.modal as Modal | null as Extract<Modal, { kind: 'commit' }>;
      render(CommitModal, { projectId: back.projectId, agentId: back.agentId, resume: back.resume });
      expect(field()).toHaveValue('feat: proposé (relu)');
      // Its files are listed again; its message is not proposed again over what was written.
      await screen.findByRole('list', { name: 'Fichiers du commit' });
      expect(backend.called('commit_propose')).toHaveLength(1);
      // Still asked: what was written is still not the proposal.
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toMatchObject(ASK);
    });
  });

  it('has nothing to commit, nor to propose, when the scope has no file', async () => {
    const backend = fakeBackend({ commit_preview: () => ({ files: [], leftOut: [] }) });
    render(CommitModal, { projectId: 'p1', agentId: null });
    expect(await screen.findByText(/Aucune modification dans le dossier du projet/)).toBeInTheDocument();
    expect(backend.called('commit_propose')).toHaveLength(0);
    expect(commitButton()).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Régénérer' })).toBeDisabled();
  });
});
