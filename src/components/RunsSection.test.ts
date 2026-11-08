import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import { forgetAgentTests, runIn, startLaunch, stopAgentTests, testLaunchIds } from '../lib/launch-actions';
import { menu } from '../lib/menu.svelte';
import { inWorktree } from '../lib/recipe';
import { shortWindow } from '../lib/short-window.svelte';
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

  it('folds to its header in a short window, a crash still said, and its title unfolds and folds it (choice 9)', async () => {
    fakeBackend();
    app.launches.c2 = state({ status: 'crashed', code: 2, ptyId: null, name: 'API' });
    shortWindow.now = true;
    try {
      render(RunsSection, { project: P });
      const section = screen.getByRole('region', { name: 'Lancement' });
      const title = within(section).getByRole('button', { name: 'Lancement' });
      expect(title).toHaveAttribute('aria-expanded', 'false');
      expect(section).toHaveClass('folded');
      expect(section).toHaveTextContent('1 planté');
      await userEvent.click(title);
      expect(title).toHaveAttribute('aria-expanded', 'true');
      expect(section).not.toHaveClass('folded');
      await userEvent.click(title);
      expect(section).toHaveClass('folded');
    } finally {
      shortWindow.now = false;
    }
  });

  it('keeps a plain title, never folded, in a taller window', () => {
    fakeBackend();
    render(RunsSection, { project: P });
    const section = screen.getByRole('region', { name: 'Lancement' });
    expect(within(section).queryByRole('button', { name: 'Lancement' })).toBeNull();
    expect(section).not.toHaveClass('folded');
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
    expect(app.modal).toEqual({ kind: 'settings', page: 'launch', projectId: 'p1' });
  });

  it('offers Claude’s proposal when there is nothing to launch: the settings on the launch group, which read the project', async () => {
    fakeBackend();
    render(RunsSection, { project: project() });
    expect(screen.getByText('Aucune commande.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Proposer des commandes' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'launch', projectId: 'p1', suggest: true });
  });

  it('does not offer it once there are commands', () => {
    fakeBackend();
    render(RunsSection, { project: P });
    expect(screen.queryByRole('button', { name: 'Proposer des commandes' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Configurer' })).not.toBeInTheDocument();
  });

  it('edits the commands from the section header', async () => {
    fakeBackend();
    render(RunsSection, { project: P });
    await userEvent.click(screen.getByRole('button', { name: 'Commandes de lancement…' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'launch', projectId: 'p1' });
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
    const names = screen.getAllByRole('button', { name: /^(Front|API|Préparation 1|web)/ }).map((b) => b.querySelector('.nm')?.textContent);
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

describe('RunsSection, a launch command in a worktree', () => {
  const W = agent({
    id: 'a1',
    name: 'refacto-auth',
    worktree: { path: 'C:/p/.claude/worktrees/refacto-auth', branch: 'refacto-auth', baseBranch: 'main' },
  });
  const PLAIN = agent({ id: 'a2', name: 'sans-worktree' });
  const OLD = agent({ id: 'a3', name: 'archivé', archived: true, worktree: { path: 'C:/p/wt/old', branch: 'old', baseBranch: 'main' } });

  beforeEach(() => {
    resetApp({ projects: [P], agents: [W, PLAIN, OLD] });
    menu.close();
  });

  it('asks where on ▶ when an agent has a worktree, and runs there with its agent', async () => {
    const backend = fakeBackend({ run_start: () => ({ id: 't9', projectId: 'p1', name: 'Front · refacto-auth', shell: 'pwsh' }) });
    render(RunsSection, { project: P });
    await userEvent.click(within(row('Front')).getByRole('button', { name: 'Lancer' }));
    // Only the agents with a worktree, not archived.
    expect(menu.open?.items.map((i) => i.label)).toEqual(['Dans le projet', '', 'Dans le worktree de refacto-auth']);
    expect(backend.called('run_start')).toHaveLength(0);
    menu.open!.items[2].onClick!();
    await vi.waitFor(() => expect(backend.called('run_start')).toHaveLength(1));
    expect(backend.called('run_start')[0].args).toMatchObject({ projectId: 'p1', commandId: 'c1', agentId: 'a1' });
    // Under its agent's name, with its own status; the project's row stays ready.
    expect(await screen.findByText('refacto-auth')).toBeInTheDocument();
    expect(within(row('Front · refacto-auth')).getByText('en cours')).toBeInTheDocument();
    expect(within(screen.getAllByRole('button', { name: /^Front/ })[0]).getByText('prêt')).toBeInTheDocument();
    expect(screen.getByText('0/2')).toBeInTheDocument();
  });

  it('runs in the project from the menu, with no agent', async () => {
    const backend = fakeBackend({ run_start: () => ({ id: 't1', projectId: 'p1', name: 'Front', shell: 'pwsh' }) });
    render(RunsSection, { project: P });
    await userEvent.click(within(row('Front')).getByRole('button', { name: 'Lancer' }));
    menu.open!.items[0].onClick!();
    await vi.waitFor(() => expect(backend.called('run_start')).toHaveLength(1));
    expect(backend.called('run_start')[0].args.agentId).toBeUndefined();
  });

  it('says first that the command runs elsewhere, and stops the other one when asked', async () => {
    app.launches.c1 = state({ ptyId: 't1', name: 'Front' });
    const backend = fakeBackend({ run_start: () => ({ id: 't9', projectId: 'p1', name: 'Front · refacto-auth', shell: 'pwsh' }) });
    await runIn(P, FRONT, 'a1');
    expect(app.modal).toMatchObject({ kind: 'confirm', title: '« Front » tourne déjà', confirm: "Arrêter l'autre et lancer" });
    const m = app.modal as Extract<typeof app.modal, { kind: 'confirm' }>;
    expect(m.body).toContain('dans le projet');
    expect(m.alt?.label).toBe('Lancer quand même');
    expect(backend.called('run_start')).toHaveLength(0);
    const done = m.onConfirm(false);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
    // The project's run ends: the other one starts after it.
    app.launches.c1.status = 'stopped';
    await done;
    expect(backend.called('run_start')[0].args).toMatchObject({ commandId: 'c1', agentId: 'a1' });
  });

  it('runs both when the user says so, and names the worktree it runs in', async () => {
    app.launches['wt:a1:c1'] = state({ ptyId: 't9', name: 'Front · refacto-auth' });
    const backend = fakeBackend({ run_start: () => ({ id: 't1', projectId: 'p1', name: 'Front', shell: 'pwsh' }) });
    await runIn(P, FRONT, null);
    const m = app.modal as Extract<typeof app.modal, { kind: 'confirm' }>;
    expect(m.body).toContain('dans le worktree de refacto-auth');
    await m.alt!.onClick();
    expect(backend.called('term_kill')).toHaveLength(0);
    expect(backend.called('run_start')[0].args).toMatchObject({ commandId: 'c1' });
    expect(backend.called('run_start')[0].args.agentId).toBeUndefined();
  });

  it('says first which commands run in a worktree before « Tout lancer », and runs all once they stopped', async () => {
    app.launches['wt:a1:c1'] = state({ ptyId: 't9', name: 'Front · refacto-auth' });
    let n = 0;
    const backend = fakeBackend({ run_start: () => ({ id: `t${++n}`, projectId: 'p1', name: 'x', shell: 'pwsh' }) });
    render(RunsSection, { project: P });
    await userEvent.click(screen.getByRole('button', { name: 'Tout lancer' }));
    expect(backend.called('run_start')).toHaveLength(0);
    const m = app.modal as Extract<typeof app.modal, { kind: 'confirm' }>;
    expect(m).toMatchObject({ title: 'Des commandes tournent déjà ailleurs', confirm: 'Les arrêter et tout lancer' });
    expect(m.body).toContain('« Front · refacto-auth »');
    const done = m.onConfirm(false);
    expect(backend.called('term_kill')[0].args).toEqual({ id: 't9' });
    app.launches['wt:a1:c1'].status = 'stopped';
    await done;
    expect(backend.called('run_start').map((c) => c.args.commandId)).toEqual(['c1', 'c2']);
    expect(backend.called('run_start').every((c) => c.args.agentId === undefined)).toBe(true);
  });

  it('shows no run when the backend refuses it, and says why', async () => {
    const why = "Le dossier « web » de « Front » n'existe pas dans le worktree de « refacto-auth » (sa branche ne l'a pas, ou pas encore).";
    fakeBackend({
      run_start: () => {
        throw why;
      },
    });
    expect(await startLaunch(P, inWorktree(FRONT, W))).toBe(why);
    expect(app.launches['wt:a1:c1']).toBeUndefined();
  });

  it('stops the launches in its worktree with its test launches (archive, deletion), and forgets them when it goes', () => {
    app.launches['wt:a1:c1'] = state({ ptyId: 't9', name: 'Front · refacto-auth' });
    app.launches.c1 = state({ ptyId: 't1' });
    const backend = fakeBackend();
    stopAgentTests('a1');
    expect(backend.called('term_kill').map((c) => c.args.id)).toEqual(['t9']);
    expect(app.launches['wt:a1:c1'].stopping).toBe(true);
    expect(testLaunchIds('p1')).toEqual(['wt:a1:c1']);
    forgetAgentTests('a1');
    expect(app.launches['wt:a1:c1']).toBeUndefined();
    expect(app.launches.c1).toBeDefined();
  });

  it('runs on ▶ in the project as before when no agent has a worktree', async () => {
    resetApp({ projects: [P], agents: [PLAIN] });
    const backend = fakeBackend({ run_start: () => ({ id: 't1', projectId: 'p1', name: 'Front', shell: 'pwsh' }) });
    render(RunsSection, { project: P });
    await userEvent.click(within(row('Front')).getByRole('button', { name: 'Lancer' }));
    expect(menu.open).toBeNull();
    expect(backend.called('run_start')[0].args).toMatchObject({ commandId: 'c1' });
  });

  it('asks where in English', async () => {
    setLang('en');
    fakeBackend();
    render(RunsSection, { project: P });
    await userEvent.click(within(row('Front')).getByRole('button', { name: 'Run' }));
    expect(menu.open?.items.map((i) => i.label)).toEqual(['In the project', '', 'In the worktree of refacto-auth']);
    app.launches.c1 = state({ ptyId: 't1', name: 'Front' });
    await runIn(P, FRONT, 'a1');
    expect(app.modal).toMatchObject({ title: '“Front” is already running', confirm: 'Stop the other and run' });
    const m = app.modal as Extract<typeof app.modal, { kind: 'confirm' }>;
    expect(m.body).toContain('in the project');
    expect(m.alt?.label).toBe('Run anyway');
  });
});

describe('RunsSection in English', () => {
  beforeEach(() => {
    resetApp({ projects: [P] });
    setLang('en');
  });

  it('writes the section, the statuses and the buttons in English', () => {
    fakeBackend();
    app.launches.c2 = state({ status: 'crashed', code: 2, ptyId: null, name: 'API' });
    render(RunsSection, { project: P });
    expect(screen.getByText('Launch')).toBeInTheDocument();
    expect(within(row('Front')).getByText('ready')).toBeInTheDocument();
    expect(within(row('Front')).getByRole('button', { name: 'Run' })).toBeInTheDocument();
    expect(within(row('API')).getByText('crashed (code 2)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Run all' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Launch commands…' })).toBeInTheDocument();
  });

  it('offers to restart or stop what runs, and to stop everything', () => {
    fakeBackend();
    app.launches.c1 = state({});
    render(RunsSection, { project: P });
    expect(within(row('Front')).getByText('running')).toBeInTheDocument();
    expect(within(row('Front')).getByRole('button', { name: 'Restart' })).toBeInTheDocument();
    expect(within(row('Front')).getByRole('button', { name: 'Stop' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Stop all' })).toBeInTheDocument();
  });

  it('says there is nothing to launch, and offers Claude’s proposal', () => {
    fakeBackend();
    render(RunsSection, { project: project() });
    expect(screen.getByText('No commands.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Configure' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Suggest commands' })).toBeInTheDocument();
  });

  it('names the steps of a recipe in English', () => {
    const a = agent({
      id: 'a7',
      name: 'dem-1-add',
      recipe: {
        prepare: [{ command: 'npm ci', dir: '' }],
        processes: [{ name: '', command: 'node app.js', dir: '', env: {}, url: '' }],
        open: '',
      },
    });
    resetApp({ projects: [P], agents: [a] });
    app.launches['test:a7:prep:0'] = state({ status: 'done', ptyId: null, name: 'Setup 1' });
    app.launches['test:a7:run:0'] = state({ name: 'process 1' });
    fakeBackend();
    render(RunsSection, { project: P });
    expect(within(row('Setup 1')).getByText('done')).toBeInTheDocument();
    expect(within(row('process 1')).getByText('running')).toBeInTheDocument();
  });
});
