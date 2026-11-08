import { cleanup, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { BARE_ENGINE, CLAUDE_ENGINE, branchInfo, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
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
