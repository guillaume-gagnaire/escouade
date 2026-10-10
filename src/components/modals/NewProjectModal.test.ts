import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import NewProjectModal from './NewProjectModal.svelte';

const folder = (over: Partial<{ exists: boolean; isRepo: boolean; branch: string; dirty: number; name: string }> = {}) => ({
  exists: true,
  isRepo: true,
  branch: 'main',
  dirty: 0,
  name: 'demo',
  ...over,
});

/** Types a path and waits for the folder to be inspected (the modal waits a moment before it asks). */
async function typePath(path: string) {
  await userEvent.type(screen.getByPlaceholderText(/^C:/), path);
}

describe('NewProjectModal', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'newProject' };
  });

  it('names the first agent’s models with the version Claude Code runs for them', () => {
    fakeBackend();
    app.models = [{ value: 'haiku', resolvedModel: 'claude-haiku-4-5-20251001' }];
    render(NewProjectModal);
    expect(screen.getByRole('button', { name: 'Haiku 4.5' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sonnet' })).toBeInTheDocument();
  });

  it('asks for the folder first, then says what it holds', async () => {
    fakeBackend({ inspect_folder: () => folder({ dirty: 3 }) });
    render(NewProjectModal);
    expect(screen.getByRole('dialog', { name: 'Nouveau projet' })).toBeInTheDocument();
    expect(screen.getByText('Choisis le dossier du projet')).toBeInTheDocument();
    await typePath('C:\\code\\demo');
    await waitFor(() => expect(screen.getByText('Dépôt git détecté · branche main · 3 fichiers modifiés')).toBeInTheDocument());
    expect(screen.getByRole('button', { name: 'Créer le projet' })).toBeEnabled();
  });

  it('says a clean repository, a single change, a folder that is not one and a folder that is missing', async () => {
    const answers = [folder(), folder({ dirty: 1 }), folder({ isRepo: false }), folder({ exists: false })];
    let n = 0;
    fakeBackend({ inspect_folder: () => answers[n++] });
    render(NewProjectModal);
    const said = [
      'Dépôt git détecté · branche main · propre',
      'Dépôt git détecté · branche main · 1 fichier modifié',
      'Aucun dépôt git · un dépôt sera initialisé',
      'Dossier introuvable',
    ];
    for (const [i, text] of said.entries()) {
      await typePath(i ? 'x' : 'C:\\code\\demo');
      await waitFor(() => expect(screen.getByText(text)).toBeInTheDocument());
    }
  });
});

describe('NewProjectModal in English', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'newProject' };
    setLang('en');
  });

  it('writes the labels, the switches and the buttons in English', () => {
    fakeBackend();
    render(NewProjectModal);
    expect(screen.getByRole('dialog', { name: 'New project' })).toBeInTheDocument();
    expect(screen.getByText('Folder')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Browse…' })).toBeInTheDocument();
    expect(screen.getByText('Choose the project folder')).toBeInTheDocument();
    expect(screen.getByText('Tab preview')).toBeInTheDocument();
    expect(screen.getByText('new-project')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Color 1' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Create a first agent' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'One git worktree per agent' })).toBeInTheDocument();
    expect(screen.getByText('Opens a conversation in this project right away')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create project' })).toBeDisabled();
    expect(screen.getByPlaceholderText('C:\\path\\to\\the\\project')).toBeInTheDocument();
  });

  it('says what the folder holds, with a plural', async () => {
    fakeBackend({ inspect_folder: () => folder({ dirty: 3 }) });
    render(NewProjectModal);
    await typePath('C:\\code\\demo');
    await waitFor(() => expect(screen.getByText('Git repository detected · branch main · 3 files changed')).toBeInTheDocument());
  });
});
