import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { forgetDraft, setDraft } from '../../lib/drafts';
import { buffers } from '../../lib/editor/buffers.svelte';
import { readPref } from '../../lib/prefs';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import UpdateModal from './UpdateModal.svelte';

const NOTES = '### Ajouts\n\n- Les mises à jour s’installent **sans fenêtre**.';
const files = {
  fs_read: () => ({ kind: 'text', text: 'a\n', size: 2, hash: 'h1', eol: 'lf', bom: false }),
  fs_base: () => null,
  set_unsaved: () => null,
};

/** `n` files edited and not saved in the editor. */
async function unsaved(n: number) {
  for (let i = 0; i < n; i++) buffers.edit((await buffers.open('p1', 'project', `f${i}.ts`)).key, 'b\n');
}

const restartButton = () => screen.getByRole('button', { name: 'Redémarrer maintenant' });

describe('UpdateModal', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'update' };
  });

  it('shows the notes of the version ready, rendered', () => {
    fakeBackend();
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    expect(screen.getByRole('dialog', { name: 'Escouade 1.6.0 est prête' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Ajouts' })).toBeInTheDocument();
    expect(screen.getByText('sans fenêtre').tagName).toBe('STRONG');
    expect(restartButton()).toBeEnabled();
  });

  it('restarts the app for the update, and says so meanwhile', async () => {
    let finish = () => {};
    const backend = fakeBackend({ update_restart: () => new Promise<void>((done) => (finish = done)) });
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    await userEvent.click(restartButton());
    expect(backend.called('update_restart')).toHaveLength(1);
    expect(screen.getByRole('button', { name: 'Redémarrage…' })).toBeDisabled();
    finish();
  });

  it('tells why a restart failed, and lets it be tried again', async () => {
    fakeBackend({ update_restart: () => Promise.reject('Aucune mise à jour n’est prête : recherche-la de nouveau.') });
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    await userEvent.click(restartButton());
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ kind: 'error' });
    expect(restartButton()).toBeEnabled();
  });

  it('leaves the restart for later', async () => {
    const backend = fakeBackend();
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    await userEvent.click(screen.getByRole('button', { name: 'Plus tard' }));
    expect(app.modal).toBeNull();
    expect(backend.called('update_restart')).toHaveLength(0);
  });

  it('asks to save the files first, the restart closed until then', async () => {
    fakeBackend(files);
    await unsaved(2);
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    expect(screen.getByText('Enregistre d’abord tes fichiers : 2 fichiers ne sont pas enregistrés.')).toBeInTheDocument();
    expect(restartButton()).toBeDisabled();
  });

  it('says it of a single file too', async () => {
    fakeBackend(files);
    await unsaved(1);
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    expect(screen.getByText('Enregistre d’abord tes fichiers : 1 fichier n’est pas enregistré.')).toBeInTheDocument();
  });

  it('says what a restart does to the agents at work or waiting for an answer, and stays possible', () => {
    resetApp({
      agents: [
        agent({ id: 'a1', status: 'running' }),
        agent({ id: 'a2', status: 'waiting' }),
        agent({ id: 'a3', status: 'running', archived: true }),
        agent({ id: 'a4', status: 'done' }),
      ],
    });
    fakeBackend();
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    expect(
      screen.getByText(
        '2 agents travaillent ou attendent ta réponse : leur tour en cours sera interrompu. Les agents de ticket reprennent d’eux-mêmes ; les autres attendent ton prochain message.',
      ),
    ).toBeInTheDocument();
    expect(restartButton()).toBeEnabled();
  });

  it('says it of a single agent too', () => {
    resetApp({ agents: [agent({ id: 'a1', status: 'waiting' })] });
    fakeBackend();
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    expect(
      screen.getByText(
        '1 agent travaille ou attend ta réponse : son tour en cours sera interrompu. S’il travaille sur un ticket, il reprend de lui-même ; sinon, il attend ton prochain message.',
      ),
    ).toBeInTheDocument();
  });

  it('keeps what is typed in the composers before the app restarts', async () => {
    let written: string | null = null;
    fakeBackend({ update_restart: () => void (written = readPref('draft.a1')) });
    setDraft('a1', { text: 'Pas encore envoyé', files: [] });
    render(UpdateModal, { version: '1.6.0', notes: NOTES });
    await userEvent.click(restartButton());
    expect(written).toBe('Pas encore envoyé');
    forgetDraft('a1');
  });

  it('shows the notes of the version installed, with nothing to restart', async () => {
    fakeBackend();
    app.modal = { kind: 'notes', version: '1.6.0', notes: NOTES };
    render(UpdateModal, { version: '1.6.0', notes: NOTES, installed: true });
    expect(screen.getByRole('dialog', { name: 'Nouveautés d’Escouade 1.6.0' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Ajouts' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Redémarrer maintenant' })).toBeNull();
    // The footer's, after the title bar's ×.
    await userEvent.click(screen.getAllByRole('button', { name: 'Fermer' }).at(-1)!);
    expect(app.modal).toBeNull();
  });
});
