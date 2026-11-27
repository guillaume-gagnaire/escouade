import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { SharedFile } from '../../lib/types';
import { fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import { afterGesture } from '../../test/settings';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));
const picked = vi.hoisted(() => ({ path: 'C:\\code\\demo-api\\design\\revue.html' as string | null }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(async () => picked.path) }));

/** The project's .escouade.yml proposing `page`, waiting for the user's trust. */
const waiting = (page: string) =>
  ({
    keys: ['pages.custom'],
    values: { 'pages.custom': page },
    warnings: [],
    error: null,
    version: 1,
    pending: { hash: 'h', commands: { 'pages.custom': page } },
  }) as SharedFile;

const field = () => screen.getByRole('textbox', { name: 'Page à soi' });
const from = () => screen.getByTestId('custom-page-from');

describe('LaunchPage › Page à soi (AP-7)', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.git.p1 = gitInfo();
    app.modal = { kind: 'settings' };
  });

  it('says there is none, then saves the file chosen, relative to the project’s folder', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    expect(screen.getByRole('heading', { name: 'Pages des agents' })).toBeInTheDocument();
    expect(field()).toHaveValue('');
    expect(from()).toHaveTextContent('Aucune page à soi');
    await userEvent.click(screen.getByRole('button', { name: 'Choisir un fichier…' }));
    expect(field()).toHaveValue('design/revue.html');
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.customPage).toBe('design/revue.html');
    // Removed: none again.
    await userEvent.click(screen.getByRole('button', { name: 'Retirer' }));
    await afterGesture();
    expect(backend.called('update_project').at(-1)!.args.project.customPage).toBe('');
  });

  it('holds back a page that is not an .html document, said on its field', async () => {
    const backend = fakeBackend({ save_settings: () => [] });
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    await fireEvent.input(field(), { target: { value: 'design/revue.js' } });
    await afterGesture();
    expect(field()).toHaveAttribute('aria-invalid', 'true');
    expect(field()).toHaveAccessibleDescription('La page à soi est un document .html.');
    expect(backend.called('update_project').every((c) => c.args.project.customPage !== 'design/revue.js')).toBe(true);
  });

  it('says the page the repository proposes waits for trust, and that this machine’s comes first', async () => {
    fakeBackend({ save_settings: () => [] });
    app.projects[0].shared = { file: waiting('design/equipe.html'), local: [], trusted: '' } as never;
    render(SettingsModal, { page: 'launch', projectId: 'p1' });
    expect(from()).toHaveAttribute('data-from', 'waiting');
    expect(from()).toHaveTextContent('Proposée par le dépôt (.escouade.yml) : design/equipe.html, en attente de ton accord');
    await fireEvent.input(field(), { target: { value: 'mine.html' } });
    expect(from()).toHaveAttribute('data-from', 'machine');
    expect(from()).toHaveTextContent('Réglage de ce poste : il passe devant la page que propose .escouade.yml (design/equipe.html).');
  });

  it('reads the page the file proposes whole among what waits for trust', () => {
    fakeBackend({ save_settings: () => [] });
    app.projects[0].shared = { file: waiting('design/equipe.html'), local: [], trusted: '' } as never;
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    const part = screen.getByRole('heading', { name: 'Page à soi (pages des agents)' }).closest('section')!;
    expect(part).toHaveTextContent('design/equipe.html');
    expect(screen.getByRole('button', { name: 'Faire confiance à ces commandes' })).toBeEnabled();
  });
});
