import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../lib/state.svelte';
import type { LaunchState, RunCommand, TestRecipe } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import RunsSection from './RunsSection.svelte';

vi.mock('../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

const FRONT: RunCommand = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
const API: RunCommand = { id: 'c2', name: 'API', command: 'cargo run', shell: 'bash', cwd: '' };
const P = project({ runCommands: [FRONT, API] });

const state = (over: Partial<LaunchState>): LaunchState => ({
  status: 'running',
  ptyId: 't1',
  name: 'Front',
  stopping: false,
  code: null,
  startedAt: 1,
  ...over,
});

const row = (name: string) => screen.getByRole('button', { name: new RegExp(`^${name}`) });

describe('RunsSection', () => {
  beforeEach(() => resetApp({ projects: [P] }));

  it('lists the launch commands with their live status', () => {
    fakeBackend();
    app.launches.c2 = state({ status: 'crashed', code: 2, ptyId: null, name: 'API' });
    render(RunsSection, { project: P });
    expect(within(row('Front')).getByText('prêt')).toBeInTheDocument();
    expect(within(row('API')).getByText('planté (code 2)')).toBeInTheDocument();
  });

  it('shows a command’s log when it is clicked', async () => {
    fakeBackend();
    render(RunsSection, { project: P });
    await userEvent.click(row('API'));
    expect(app.selectedLaunch.p1).toBe('c2');
    expect(row('API')).toHaveClass('sel');
  });

  it('starts a command, then offers to restart or stop it', async () => {
    const backend = fakeBackend({ run_start: () => ({ id: 't1', projectId: 'p1', name: 'Front', shell: 'pwsh' }) });
    render(RunsSection, { project: P });
    await userEvent.click(within(row('Front')).getByRole('button', { name: 'Lancer' }));
    expect(backend.called('run_start')[0].args).toMatchObject({ projectId: 'p1', commandId: 'c1' });
    expect(await within(row('Front')).findByText('en cours')).toBeInTheDocument();
    expect(within(row('Front')).getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
    await userEvent.click(within(row('Front')).getByRole('button', { name: 'Stopper' }));
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    // Starting from the row does not open its log.
    expect(app.selectedLaunch.p1 ?? null).toBeNull();
  });

  it('starts them all, then stops them all', async () => {
    let n = 0;
    const backend = fakeBackend({ run_start: () => ({ id: `t${++n}`, projectId: 'p1', name: 'x', shell: 'pwsh' }) });
    render(RunsSection, { project: P });
    await userEvent.click(screen.getByRole('button', { name: 'Tout lancer' }));
    expect(backend.called('run_start').map((c) => c.args.commandId)).toEqual(['c1', 'c2']);
    await userEvent.click(await screen.findByRole('button', { name: 'Tout arrêter' }));
    expect(backend.called('term_kill').map((c) => c.args.id)).toEqual(['t1', 't2']);
  });

  it('opens the configuration, also when there is nothing to launch yet', async () => {
    fakeBackend();
    render(RunsSection, { project: project() });
    expect(screen.queryByRole('button', { name: 'Tout lancer' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Configurer' }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects', projectId: 'p1', section: 'launch' });
  });

  it('offers Claude’s proposal when there is nothing to launch: the settings on the launch group, which read the project', async () => {
    fakeBackend();
    render(RunsSection, { project: project() });
    expect(screen.getByText('Aucune commande.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: '✦ Proposer des commandes' }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects', projectId: 'p1', section: 'launch', suggest: true });
  });

  it('does not offer it once there are commands', () => {
    fakeBackend();
    render(RunsSection, { project: P });
    expect(screen.queryByRole('button', { name: '✦ Proposer des commandes' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Configurer' })).not.toBeInTheDocument();
  });

  it('edits the commands from the section header', async () => {
    fakeBackend();
    render(RunsSection, { project: P });
    await userEvent.click(screen.getByRole('button', { name: 'Commandes de lancement…' }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects', projectId: 'p1', section: 'launch' });
  });

  const recipe = (over: Partial<TestRecipe> = {}): TestRecipe => ({
    prepare: [],
    processes: [{ name: 'web', command: 'node serveur.js', dir: '', env: {}, url: 'http://localhost:4101' }],
    open: '',
    ...over,
  });

  it('lists the test launches of each agent under its name', async () => {
    const a = agent({ id: 'a7', name: 'dem-1-ajouter', recipe: recipe() });
    resetApp({ projects: [P], agents: [a] });
    app.launches['test:a7:run:0'] = state({ name: 'web' });
    fakeBackend();
    render(RunsSection, { project: P });
    expect(screen.getByText('dem-1-ajouter')).toBeInTheDocument();
    await userEvent.click(row('web'));
    expect(app.selectedLaunch.p1).toBe('test:a7:run:0');
    expect(app.runCommand?.command).toBe('node serveur.js');
  });

  it('shows an agent’s steps once one of them ran, the preparation first, and no group before', () => {
    const a = agent({
      id: 'a7',
      name: 'dem-1-ajouter',
      recipe: recipe({ prepare: [{ command: 'npm i', dir: 'web' }] }),
    });
    const idle = agent({ id: 'a8', name: 'dem-2-sans-lancement', recipe: recipe() });
    const elsewhere = agent({ id: 'a9', projectId: 'p2', name: 'dem-3-ailleurs', recipe: recipe() });
    resetApp({ projects: [P, project({ id: 'p2', name: 'autre' })], agents: [a, idle, elsewhere] });
    app.launches['test:a7:run:0'] = state({ name: 'web' });
    app.launches['test:a7:prep:0'] = state({ status: 'done', ptyId: null, code: 0, name: 'Préparation 1' });
    app.launches['test:a9:run:0'] = state({ name: 'web' });
    fakeBackend();
    render(RunsSection, { project: P });
    expect(screen.getByText('dem-1-ajouter')).toBeInTheDocument();
    expect(screen.queryByText('dem-2-sans-lancement')).not.toBeInTheDocument();
    expect(screen.queryByText('dem-3-ailleurs')).not.toBeInTheDocument();
    const names = screen
      .getAllByRole('button', { name: /^(Front|API|Préparation 1|web)/ })
      .map((b) => b.querySelector('.name')?.textContent);
    expect(names).toEqual(['Front', 'API', 'Préparation 1', 'web']);
    expect(within(row('Préparation 1')).getByText('terminé')).toBeInTheDocument();
    expect(within(row('web')).getByText('en cours')).toBeInTheDocument();
    // The header counts the project's commands only.
    expect(screen.getByText('0/2')).toBeInTheDocument();
  });

  it('starts, restarts and stops a step from its row, through the test launch', async () => {
    const a = agent({ id: 'a7', name: 'dem-1-ajouter', recipe: recipe() });
    resetApp({ projects: [P], agents: [a] });
    app.launches['test:a7:run:0'] = state({ status: 'stopped', ptyId: null, name: 'web' });
    const backend = fakeBackend({ test_run_start: () => ({ id: 't7', projectId: 'p1', name: 'web', shell: 'pwsh' }) });
    render(RunsSection, { project: P });
    await userEvent.click(within(row('web')).getByRole('button', { name: 'Lancer' }));
    expect(backend.called('test_run_start')[0].args).toMatchObject({ agentId: 'a7', kind: 'run', index: 0 });
    expect(backend.called('run_start')).toHaveLength(0);
    expect(await within(row('web')).findByText('en cours')).toBeInTheDocument();
    await userEvent.click(within(row('web')).getByRole('button', { name: 'Stopper' }));
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't7' });
  });
});
