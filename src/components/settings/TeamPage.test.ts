import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { BARE_ENGINE, CLAUDE_ENGINE, agent, branchInfo, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import { afterGesture } from '../../test/settings';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

describe('TeamPage', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site' })] });
    app.modal = { kind: 'settings' };
  });

  it('shows in « Ton nom dans ce projet » the name git knows in its folder when none was given (ESC-243)', async () => {
    app.settings = { ...app.settings, userName: '' };
    fakeBackend({ git_user_name: ({ projectId }: { projectId: string | null }) => (projectId === 'p1' ? 'Bob (dépôt)' : 'Bob (global)') });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    const field = screen.getByRole('textbox', { name: 'Ton nom dans ce projet' });
    await vi.waitFor(() => expect(field).toHaveAttribute('placeholder', 'Bob (dépôt)'));
    expect(field).toHaveValue('');
    expect(
      screen.getByText(
        'Vide : Bob (dépôt), repris de git (git config user.name de ce dépôt). Gardé sur ce poste, jamais dans .escouade.yml.',
      ),
    ).toBeInTheDocument();
  });

  it('chooses the project’s « Chef d’escouade » among its agents, none by default (ESC-423)', async () => {
    app.agents = {
      a1: agent({ id: 'a1', name: 'overseer', projectId: 'p1' }),
      a2: agent({ id: 'a2', name: 'esc-1', projectId: 'p1' }),
      a3: agent({ id: 'a3', name: 'ailleurs', projectId: 'p2' }),
    };
    const be = fakeBackend({ git_user_name: () => null, update_project: () => null });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    const field = screen.getByRole('combobox', { name: 'Chef d’escouade' });
    expect(field).toHaveValue('');
    // Its own agents alone.
    expect(
      within(field)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['Aucun', 'esc-1', 'overseer']);
    await userEvent.selectOptions(field, 'a1');
    // Without « Seulement le chef d’escouade », it is said to do nothing yet.
    expect(screen.getByTestId('chef-off')).toBeInTheDocument();
    await afterGesture();
    expect(be.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', chef: 'a1' });
    await userEvent.selectOptions(field, '');
    await afterGesture();
    expect(be.called('update_project').at(-1)!.args.project).toMatchObject({ id: 'p1', chef: null });
  });

  it('shows the app’s name before git’s in « Ton nom dans ce projet » (ESC-243)', async () => {
    app.settings = { ...app.settings, userName: 'Robert' };
    fakeBackend({ git_user_name: () => 'Bob (dépôt)' });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    const field = screen.getByRole('textbox', { name: 'Ton nom dans ce projet' });
    await new Promise((r) => setTimeout(r, 20));
    expect(field).toHaveAttribute('placeholder', 'Robert');
    expect(
      screen.getByText('Vide : celui de Réglages › Général (Robert). Gardé sur ce poste, jamais dans .escouade.yml.'),
    ).toBeInTheDocument();
  });
});
