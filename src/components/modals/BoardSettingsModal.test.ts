import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app } from '../../lib/state.svelte';
import { fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import BoardSettingsModal from './BoardSettingsModal.svelte';

const backendSaving = () => fakeBackend({ git_branches: () => ['main', 'release'], board_set: (a: any) => project({ board: a.settings }) });
const saved = (backend: ReturnType<typeof fakeBackend>) => backend.called('board_set').at(-1)!.args.settings;

describe('BoardSettingsModal', () => {
  beforeEach(() => {
    resetApp();
    app.git.p1 = gitInfo();
    app.modal = { kind: 'boardSettings', projectId: 'p1' };
  });

  it('applies each choice at once', async () => {
    const backend = backendSaving();
    render(BoardSettingsModal, { projectId: 'p1' });
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(await screen.findByRole('button', { name: '⎇ release' }));
    expect(saved(backend)).toMatchObject({ target: 'release', action: 'merge' });
    await userEvent.click(screen.getByRole('button', { name: 'Rebase' }));
    expect(saved(backend).strategy).toBe('rebase');
    expect(screen.queryByRole('switch', { name: 'Ouvrir en brouillon' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('radio', { name: /Ouvrir une pull request/ }));
    expect(screen.queryByRole('button', { name: 'Rebase' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: 'Ouvrir en brouillon' }));
    expect(saved(backend)).toMatchObject({ action: 'pr', draft: true, target: 'release' });
    await userEvent.click(screen.getByRole('radio', { name: /Laisser en l'état/ }));
    expect(screen.queryByText('Branche cible')).not.toBeInTheDocument();
  });

  it('keeps the tests switch off until a command is given, and previews the commit message', async () => {
    const backend = backendSaving();
    render(BoardSettingsModal, { projectId: 'p1' });
    const tests = screen.getByRole('switch', { name: 'Relancer les tests avant' });
    expect(tests).toBeDisabled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Commande de tests' }), 'npm test');
    await userEvent.tab();
    expect(saved(backend).testCommand).toBe('npm test');
    expect(tests).toBeEnabled();
    await userEvent.click(tests);
    expect(saved(backend)).toMatchObject({ testsFirst: true, testCommand: 'npm test' });
    expect(screen.getByText('feat: limiter les tentatives de connexion [DEM-42]')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: 'Message de commit généré' }));
    expect(screen.getByText('DEM-42 Limiter les tentatives de connexion')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('switch', { name: 'Supprimer le worktree après merge' }));
    expect(saved(backend).cleanup).toBe(false);
  });

  it('sets the conflict policy, the agents in parallel and their model', async () => {
    const backend = backendSaving();
    render(BoardSettingsModal, { projectId: 'p1' });
    await userEvent.click(screen.getByRole('button', { name: "L'agent résout" }));
    expect(saved(backend).conflict).toBe('agent');
    await userEvent.click(screen.getByRole('button', { name: '4' }));
    expect(saved(backend).maxParallel).toBe(4);
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Modèle' }), 'opus');
    expect(saved(backend).model).toBe('opus');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Mode' }), 'plan');
    expect(saved(backend).mode).toBe('plan');
    await userEvent.click(screen.getByRole('button', { name: 'Terminé' }));
    expect(app.modal).toBeNull();
  });

  it('puts a choice back when it could not be saved', async () => {
    fakeBackend({
      git_branches: () => ['main'],
      board_set: () => {
        throw 'disque plein';
      },
    });
    render(BoardSettingsModal, { projectId: 'p1' });
    const squash = screen.getByRole('button', { name: 'Squash' });
    expect(squash).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Rebase' }));
    expect(app.toasts.at(-1)?.text).toContain('disque plein');
    expect(squash).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Rebase' })).toHaveAttribute('aria-pressed', 'false');
    expect(app.projects[0].board.strategy).toBe('squash');
  });
});
