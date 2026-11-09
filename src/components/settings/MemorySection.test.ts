import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { forgetOffers, offerClaudeMemory } from '../../lib/memory';
import { app } from '../../lib/state.svelte';
import type { Memory, OfferedMemory } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import ClaudeMemoryModal from '../modals/ClaudeMemoryModal.svelte';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const MEMORIES: Memory[] = [
  { name: 'e2e-build', description: 'Lancer les e2e après un build de debug', kind: 'feedback', body: 'npx tauri build --debug' },
  { name: 'ports', description: 'Ports réservés', kind: 'project', body: 'Jamais dans un fichier versionné.' },
];

const OFFERED: OfferedMemory[] = [
  { ...MEMORIES[0], exists: true },
  { name: 'notes', description: 'Notes libres', kind: 'user', body: 'Texte', exists: false },
];

describe('the shared memory in the project’s settings (ESC-255)', () => {
  beforeEach(() => {
    app.modal = { kind: 'settings' };
  });

  it('lists the memories with their kind, and shows a text on a click', async () => {
    resetApp({ projects: [project()] });
    fakeBackend({ memory_list: () => MEMORIES });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    const list = await screen.findByRole('list', { name: 'Mémoire partagée' });
    expect(within(list).getByText('e2e-build')).toBeInTheDocument();
    expect(within(list).getByText('Façon de faire')).toBeInTheDocument();
    expect(within(list).getByText('Ports réservés')).toBeInTheDocument();
    expect(screen.getByText('2 souvenirs')).toBeInTheDocument();
    expect(
      screen.getByText(/Elle est dans \.escouade\/memory\/ du dossier du projet ; Escouade ne la commite jamais\./),
    ).toBeInTheDocument();
    await userEvent.click(within(list).getByText('e2e-build'));
    expect(within(list).getByText('npx tauri build --debug')).toBeVisible();
  });

  it('says when there is none yet', async () => {
    resetApp({ projects: [project()] });
    fakeBackend();
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    expect(await screen.findByText(/Aucun souvenir pour l’instant/)).toBeInTheDocument();
  });

  it('forgets a memory on a second click only', async () => {
    resetApp({ projects: [project()] });
    const backend = fakeBackend({ memory_list: () => MEMORIES });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    const forget = await screen.findByRole('button', { name: 'Oublier le souvenir ports' });
    await userEvent.click(forget);
    expect(backend.called('memory_forget')).toHaveLength(0);
    await userEvent.click(screen.getByRole('button', { name: 'Confirmer l’oubli' }));
    expect(backend.called('memory_forget')[0].args).toEqual({ projectId: 'p1', name: 'ports' });
    expect(screen.queryByText('Ports réservés')).toBeNull();
    expect(app.toasts.at(-1)?.text).toBe('Souvenir « ports » oublié.');
  });

  it('reads it again when the backend says it changed', async () => {
    resetApp({ projects: [project()] });
    let memories: Memory[] = [];
    const backend = fakeBackend({ memory_list: () => memories });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    await screen.findByText(/Aucun souvenir pour l’instant/);
    memories = MEMORIES;
    // What the 'memory' event does.
    app.memoryTicks.p1 = (app.memoryTicks.p1 ?? 0) + 1;
    expect(await screen.findByText('Ports réservés')).toBeInTheDocument();
    expect(backend.called('memory_list').length).toBeGreaterThanOrEqual(2);
  });
});

describe('Claude Code’s memory, offered once (ESC-255)', () => {
  beforeEach(() => {
    forgetOffers();
  });

  it('ticks nothing at first, and takes only what was ticked', async () => {
    resetApp({ projects: [project()] });
    app.modal = { kind: 'claudeMemory', projectId: 'p1', offered: OFFERED };
    const backend = fakeBackend({ claude_memory_take: () => project({ claudeMemoryAsked: true }) });
    render(ClaudeMemoryModal, { projectId: 'p1', offered: OFFERED });
    const take = screen.getByRole('button', { name: 'Reprendre' });
    expect(take).toBeDisabled();
    expect(screen.getByText('Remplace celui du projet')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('checkbox', { name: /notes/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Reprendre 1 souvenir' }));
    expect(backend.called('claude_memory_take')[0].args).toEqual({ projectId: 'p1', names: ['notes'] });
    expect(app.modal).toBeNull();
    expect(app.projects[0].claudeMemoryAsked).toBe(true);
    expect(app.toasts.at(-1)?.text).toBe('1 souvenir repris dans la mémoire partagée.');
  });

  it('« Ne rien reprendre » settles it with nothing taken', async () => {
    resetApp({ projects: [project()] });
    app.modal = { kind: 'claudeMemory', projectId: 'p1', offered: OFFERED };
    const backend = fakeBackend({ claude_memory_take: () => project({ claudeMemoryAsked: true }) });
    render(ClaudeMemoryModal, { projectId: 'p1', offered: OFFERED });
    await userEvent.click(screen.getByRole('button', { name: 'Tout cocher' }));
    expect(screen.getByRole('button', { name: 'Reprendre 2 souvenirs' })).toBeEnabled();
    await userEvent.click(screen.getByRole('button', { name: 'Ne rien reprendre' }));
    expect(backend.called('claude_memory_take')[0].args).toEqual({ projectId: 'p1', names: [] });
    expect(app.modal).toBeNull();
  });

  it('is offered for a project not asked yet, once, and never over another window', async () => {
    resetApp({ projects: [project()] });
    const backend = fakeBackend({ claude_memory_offer: () => OFFERED });
    app.modal = { kind: 'telemetry' };
    await offerClaudeMemory('p1');
    expect(backend.called('claude_memory_offer')).toHaveLength(0);
    app.modal = null;
    app.ui.activeProject = 'p1';
    await offerClaudeMemory('p1');
    expect(app.modal).toEqual({ kind: 'claudeMemory', projectId: 'p1', offered: OFFERED });
    app.modal = null;
    await offerClaudeMemory('p1');
    expect(backend.called('claude_memory_offer')).toHaveLength(1);
  });

  it('is not offered once asked, nor when Claude Code has nothing', async () => {
    resetApp({ projects: [project({ claudeMemoryAsked: true }), project({ id: 'p2' })] });
    const backend = fakeBackend();
    await offerClaudeMemory('p1');
    expect(backend.called('claude_memory_offer')).toHaveLength(0);
    app.ui.activeProject = 'p2';
    await offerClaudeMemory('p2');
    await waitFor(() => expect(backend.called('claude_memory_offer')).toHaveLength(1));
    expect(app.modal).toBeNull();
  });
});
