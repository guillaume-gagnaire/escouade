import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { RunCommand } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import RunConfigModal from './RunConfigModal.svelte';

vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

const FRONT: RunCommand = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
const SHELLS = [
  { id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' },
  { id: 'bash', label: 'Git Bash', path: 'bash.exe' },
];

const card = (i: number) => screen.getAllByRole('group')[i];

describe('RunConfigModal', () => {
  beforeEach(() => {
    resetApp({ projects: [project({ runCommands: [FRONT] })] });
    app.shells = SHELLS;
    app.modal = { kind: 'runConfig', projectId: 'p1' };
  });

  it('edits, adds and saves the project’s launch commands', async () => {
    const backend = fakeBackend();
    render(RunConfigModal, { projectId: 'p1' });
    expect(within(card(0)).getByLabelText('Nom')).toHaveValue('Front');
    expect(within(card(0)).getByLabelText('Shell')).toHaveValue('pwsh');
    expect(within(card(0)).getByLabelText(/Sous-dossier/)).toHaveValue('web');

    await userEvent.click(screen.getByRole('button', { name: '+ Ajouter une commande' }));
    // A new command runs with the first detected shell, in the project's folder.
    expect(within(card(1)).getByLabelText('Shell')).toHaveValue('pwsh');
    await userEvent.type(within(card(1)).getByLabelText('Nom'), 'API');
    await userEvent.type(within(card(1)).getByLabelText('Commande'), 'cargo run');
    await userEvent.selectOptions(within(card(1)).getByLabelText('Shell'), 'bash');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));

    const saved = backend.called('update_project')[0].args.project.runCommands;
    expect(saved).toEqual([FRONT, { id: expect.any(String), name: 'API', command: 'cargo run', shell: 'bash', cwd: '' }]);
    expect(saved[1].id).not.toBe('c1');
    expect(app.projects[0].runCommands).toEqual(saved);
    expect(app.modal).toBeNull();
  });

  it('needs a name and a command line for each command', async () => {
    const backend = fakeBackend();
    render(RunConfigModal, { projectId: 'p1' });
    await userEvent.clear(within(card(0)).getByLabelText('Commande'));
    expect(screen.getByRole('button', { name: 'Enregistrer' })).toBeDisabled();
    expect(backend.called('update_project')).toHaveLength(0);
  });

  it('stops a removed command that was running', async () => {
    const backend = fakeBackend();
    app.launches.c1 = { status: 'running', ptyId: 't1', name: 'Front', stopping: false, code: null, startedAt: 1 };
    render(RunConfigModal, { projectId: 'p1' });
    await userEvent.click(within(card(0)).getByRole('button', { name: 'Supprimer' }));
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(backend.called('update_project')[0].args.project.runCommands).toEqual([]);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    expect(app.launches.c1).toBeUndefined();
  });

  it('saves the files copied into new worktrees', async () => {
    const backend = fakeBackend();
    render(RunConfigModal, { projectId: 'p1' });
    const field = screen.getByRole('textbox', { name: /Fichiers copiés dans les worktrees/ });
    expect(field).toHaveValue('.env*');
    await userEvent.type(field, '{Enter}**/.env.local');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(backend.called('update_project')[0].args.project.worktreeCopy).toEqual(['.env*', '**/.env.local']);
  });

  it('changes nothing when cancelled', async () => {
    const backend = fakeBackend();
    render(RunConfigModal, { projectId: 'p1' });
    await userEvent.clear(within(card(0)).getByLabelText('Nom'));
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(backend.called('update_project')).toHaveLength(0);
    expect(app.projects[0].runCommands).toEqual([FRONT]);
    expect(app.modal).toBeNull();
  });
});
