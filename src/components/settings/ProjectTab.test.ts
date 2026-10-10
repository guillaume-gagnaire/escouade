import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const save = () => userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));

describe('ProjectTab', () => {
  beforeEach(() => {
    resetApp({ projects: [project(), project({ id: 'p2', name: 'site', agentsUseEscouade: true })] });
    app.modal = { kind: 'settings' };
  });

  it('lets the project’s agents use Escouade, off by default, and says what it means', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const allowed = screen.getByRole('switch', { name: 'Les agents peuvent utiliser Escouade' });
    expect(allowed).toHaveAttribute('aria-checked', 'false');
    expect(
      screen.getByText(
        'Ils lisent les tickets et les agents, et peuvent créer des tickets ou lancer d’autres agents, ce qui dépense du quota. S’applique au prochain démarrage de leur process.',
      ),
    ).toBeInTheDocument();
    await userEvent.click(allowed);
    expect(allowed).toHaveAttribute('aria-checked', 'true');
    await save();
    expect(backend.called('update_project')).toHaveLength(1);
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p1', agentsUseEscouade: true });
    expect(app.projects[0].agentsUseEscouade).toBe(true);
  });

  it('shows the project’s own setting, and turns it off', async () => {
    const backend = fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p2' });
    const allowed = screen.getByRole('switch', { name: 'Les agents peuvent utiliser Escouade' });
    expect(allowed).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(allowed);
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ id: 'p2', agentsUseEscouade: false });
  });

  it('says it in English', () => {
    setLang('en');
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.getByRole('heading', { name: 'MCP server' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Agents can use Escouade' })).toBeInTheDocument();
    expect(
      screen.getByText(
        'They read the tickets and the agents, and can create tickets or start other agents, which uses quota. Applies the next time their process starts.',
      ),
    ).toBeInTheDocument();
  });
});
