import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Test launches have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { app } from '../../lib/state.svelte';
import { flows } from '../../lib/test-launch.svelte';
import { agent, fakeBackend, resetApp, ticket } from '../../test/ipc';
import TestLaunchModal from './TestLaunchModal.svelte';

describe('TestLaunchModal', () => {
  beforeEach(() => {
    resetApp({ agents: [agent({ id: 'a7' })], tickets: [ticket({ agentId: 'a7', column: 'review' })] });
    app.modal = { kind: 'testLaunch', agentId: 'a7' };
    app.launches['test:a7:run:0'] = { status: 'running', ptyId: 't1', name: 'web', stopping: false, code: null, startedAt: 1 };
    flows.all = {
      a7: {
        phase: 'ready',
        error: null,
        opened: 'http://localhost:4111/connexion',
        lines: [{ id: 'test:a7:run:0', label: 'web', state: 'ready', detail: 'prêt · 1,8 s', launchId: 'test:a7:run:0' }],
      },
    };
  });

  it('shows each line, where the browser went, and reopens it', async () => {
    const backend = fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.getByRole('dialog', { name: 'Tester DEM-1' })).toBeInTheDocument();
    expect(screen.getByText('prêt · 1,8 s')).toBeInTheDocument();
    expect(screen.getByText(/Ouvert dans le navigateur/)).toHaveTextContent('Ouvert dans le navigateur : http://localhost:4111/connexion');
    await userEvent.click(screen.getByRole('button', { name: 'Rouvrir' }));
    expect(backend.called('plugin:opener|open_url')[0].args.url).toBe('http://localhost:4111/connexion');
  });

  it('stops everything, or shows the log of a step', async () => {
    const backend = fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    await userEvent.click(screen.getByRole('button', { name: 'Tout arrêter' }));
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    await userEvent.click(screen.getByRole('button', { name: 'Voir les logs' }));
    expect(app.selectedLaunch.p1).toBe('test:a7:run:0');
    expect(app.modal).toBeNull();
  });

  it('says it is stopped once everything is, with nothing left to reopen', async () => {
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    await userEvent.click(screen.getByRole('button', { name: 'Tout arrêter' }));
    expect(screen.getByText('Arrêté')).toBeInTheDocument();
    expect(screen.getByText('arrêté')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Rouvrir' })).not.toBeInTheDocument();
    expect(screen.queryByText(/Ouvert dans le navigateur/)).not.toBeInTheDocument();
  });

  it('names the agent when it has no ticket', () => {
    resetApp({ agents: [agent({ id: 'a7', name: 'refacto-auth' })] });
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.getByRole('dialog', { name: 'Tester refacto-auth' })).toBeInTheDocument();
  });

  it('offers the log of a step that failed, and the reason of one that could not start, which has none', async () => {
    fakeBackend();
    app.launches['test:a7:prep:0'] = { status: 'crashed', ptyId: null, name: 'Préparation 1', stopping: false, code: 2, startedAt: 1 };
    flows.all.a7 = {
      phase: 'failed',
      error: "« web » n'a pas pu démarrer",
      opened: null,
      lines: [
        { id: 'test:a7:prep:0', label: 'Préparation : npm install', state: 'failed', detail: 'code 2', launchId: 'test:a7:prep:0' },
        { id: 'test:a7:run:1', label: 'web', state: 'failed', detail: 'Validation en cours', launchId: 'test:a7:run:1' },
      ],
    };
    render(TestLaunchModal, { agentId: 'a7' });
    const lines = within(screen.getByRole('list', { name: 'Étapes du lancement' })).getAllByRole('listitem');
    expect(within(lines[1]).getByText('Validation en cours')).toBeInTheDocument();
    expect(within(lines[1]).queryByRole('button', { name: 'Voir le log' })).not.toBeInTheDocument();
    expect(screen.getByText("« web » n'a pas pu démarrer")).toBeInTheDocument();
    await userEvent.click(within(lines[0]).getByRole('button', { name: 'Voir le log' }));
    expect(app.selectedLaunch.p1).toBe('test:a7:prep:0');
    expect(app.modal).toBeNull();
  });

  it('says the recipe changed when the test it showed was dropped for a new one', async () => {
    const recipe = { prepare: [], processes: [{ name: 'web', command: 'node web.js', dir: '', env: {}, url: '' }], open: '' };
    app.agents.a7 = { ...app.agents.a7, recipe };
    fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.queryByText('La recette a changé : relance ▶ Tester.')).not.toBeInTheDocument();
    // What the window does when the agent sends another recipe.
    delete flows.all.a7;
    expect(await screen.findByText('La recette a changé : relance ▶ Tester.')).toBeInTheDocument();
    expect(screen.queryByRole('list', { name: 'Étapes du lancement' })).not.toBeInTheDocument();
  });

  it('has no logs to show before any step ran', () => {
    fakeBackend();
    app.launches = {};
    flows.all = {};
    render(TestLaunchModal, { agentId: 'a7' });
    expect(screen.getByRole('button', { name: 'Voir les logs' })).toBeDisabled();
  });

  it('closes, leaving the servers running', async () => {
    const backend = fakeBackend();
    render(TestLaunchModal, { agentId: 'a7' });
    const dialog = screen.getByRole('dialog');
    await userEvent.click(within(dialog.querySelector('.foot') as HTMLElement).getByRole('button', { name: 'Fermer' }));
    expect(app.modal).toBeNull();
    expect(backend.called('term_kill')).toHaveLength(0);
    expect(app.launches['test:a7:run:0'].status).toBe('running');
  });
});
