import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest';
import { buffers } from '../lib/editor/buffers.svelte';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import { openTerminal } from '../lib/terminals';
import { agent, BARE_ENGINE, branchInfo, CLAUDE_ENGINE, fakeBackend, gitInfo, project, pullRequest, resetApp, ticket } from '../test/ipc';
import ContextMenu from './ContextMenu.svelte';
import Sidebar from './Sidebar.svelte';
import { getByTip } from '../test/tip';

// The terminal opened is not one of xterm.js (jsdom has no canvas for it): only what is asked of the backend is looked at.
vi.mock('../lib/terminals', async (original) => ({
  ...(await original<typeof import('../lib/terminals')>()),
  openTerminal: vi.fn((projectId: string, shell: string, name: string) => Promise.resolve({ id: 't1', projectId, name, shell })),
  // A launch's log, without the canvas jsdom lacks.
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
}));

describe('Sidebar', () => {
  beforeEach(() =>
    resetApp({
      projects: [project()],
      agents: [
        agent({ status: 'running' }),
        agent({ id: 'a2', name: 'tests-e2e', status: 'waiting', createdAt: 2 }),
        agent({ id: 'a3', name: 'vieux', archived: true }),
      ],
    }),
  );

  it('lists the project’s agents with their status', () => {
    fakeBackend();
    render(Sidebar, { project: project() });
    const cards = screen.getAllByRole('button', { name: /refacto-auth|tests-e2e/ });
    expect(cards).toHaveLength(2);
    expect(within(cards[0]).getByText('En cours')).toBeInTheDocument();
    expect(within(cards[1]).getByText('Question')).toBeInTheDocument();
    expect(screen.getByText(/Archivés \(1\)/)).toBeInTheDocument();
  });

  it('says on an archived agent’s card why its worktree was kept (ESC-143)', async () => {
    fakeBackend();
    app.agents.a3 = { ...app.agents.a3, worktreeKept: 'worktree gardé : 1 fichier(s) non commité(s)' };
    app.showArchived = true;
    // Shown for this test only: the others find the archived agents folded.
    onTestFinished(() => {
      app.showArchived = false;
    });
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /vieux/ });
    expect(within(card).getByText('worktree gardé : 1 fichier(s) non commité(s)')).toBeInTheDocument();
    // None on the others.
    expect(within(screen.getByRole('button', { name: /tests-e2e/ })).queryByText(/worktree gardé/)).not.toBeInTheDocument();
  });

  it('says on an agent’s card what it reported doing, until it reports something else', async () => {
    fakeBackend();
    app.agents.a1 = { ...app.agents.a1, progressLine: 'Écrit les tests du parseur' };
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    const line = within(card).getByText('Écrit les tests du parseur');
    expect(line).toHaveAttribute('data-tip', 'Écrit les tests du parseur');
    // Assistive technologies are told what the line is.
    expect(card).toHaveTextContent('Ce que l’agent dit faire Écrit les tests du parseur');
    // The other agent said nothing: no line, no label.
    expect(within(screen.getByRole('button', { name: /tests-e2e/ })).queryByText('Ce que l’agent dit faire')).not.toBeInTheDocument();
    // A new report replaces it; an empty one takes it off.
    app.agents.a1 = { ...app.agents.a1, progressLine: 'Relit le diff' };
    await Promise.resolve();
    expect(within(card).getByText('Relit le diff')).toBeInTheDocument();
    expect(within(card).queryByText('Écrit les tests du parseur')).not.toBeInTheDocument();
    app.agents.a1 = { ...app.agents.a1, progressLine: null };
    await Promise.resolve();
    expect(card).not.toHaveTextContent('Relit le diff');
    expect(card).not.toHaveTextContent('Ce que l’agent dit faire');
  });

  it('says an agent sets its new worktree up', () => {
    fakeBackend();
    app.agents.a1 = { ...app.agents.a1, status: 'idle', setup: '2/3 · npm run gen (web)' };
    render(Sidebar, { project: project() });
    const status = within(screen.getByRole('button', { name: /refacto-auth/ })).getByText('Préparation…');
    expect(status).toHaveAttribute('data-tip', 'Préparation du worktree : 2/3 · npm run gen (web)');
  });

  it('says on the card of an agent whose worktree waits for its first message that it will be made then (ESC-156)', () => {
    fakeBackend();
    app.agents.a1 = { ...app.agents.a1, status: 'idle', worktree: null, worktreePending: true };
    render(Sidebar, { project: project() });
    const mark = within(screen.getByRole('button', { name: /refacto-auth/ })).getByText('⎇ au premier message');
    expect(mark).toHaveAttribute('data-tip', 'Le worktree sera créé au premier message, au nom de l’agent');
  });

  it('names an agent’s model with the version Claude Code runs for it', () => {
    fakeBackend();
    app.models = [{ value: 'opus', resolvedModel: 'claude-opus-5-5' }];
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(within(card).getByText('Opus 5.5')).toBeInTheDocument();
  });

  it('makes the card of an agent to look at blink until it is seen', async () => {
    fakeBackend();
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /tests-e2e/ });
    expect(card).not.toHaveClass('alert');
    app.attention = { a2: true };
    await Promise.resolve();
    expect(card).toHaveClass('alert');
    expect(card.style.getPropertyValue('--alert')).toBe('var(--wait)');
  });

  it('renames an agent once with Enter (the blur that follows does not rename again)', async () => {
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    await userEvent.dblClick(screen.getByText('refacto-auth'));
    const input = screen.getByDisplayValue('refacto-auth');
    await userEvent.clear(input);
    await userEvent.type(input, 'auth-jwt{Enter}');
    expect(backend.called('rename_agent')).toEqual([{ cmd: 'rename_agent', args: { id: 'a1', name: 'auth-jwt' } }]);
  });

  it('says why the agent’s branch kept its name, the agent renamed all the same, and nothing when it followed', async () => {
    const kept = 'La branche « escouade/refacto-auth » garde son nom : elle est déjà poussée.';
    fakeBackend({ rename_agent: (a: any) => (a.name === 'auth-jwt' ? kept : null) });
    render(Sidebar, { project: project() });
    await userEvent.dblClick(screen.getByText('refacto-auth'));
    await userEvent.clear(screen.getByDisplayValue('refacto-auth'));
    await userEvent.type(screen.getByDisplayValue(''), 'auth-jwt{Enter}');
    await vi.waitFor(() => expect(app.toasts).toEqual([expect.objectContaining({ text: kept, kind: 'info' })]));
    app.toasts = [];
    await userEvent.dblClick(screen.getByText('refacto-auth'));
    await userEvent.clear(screen.getByDisplayValue('refacto-auth'));
    await userEvent.type(screen.getByDisplayValue(''), 'autre{Enter}');
    await new Promise((r) => setTimeout(r, 20));
    expect(app.toasts).toEqual([]);
  });

  it('cancels a rename with Escape', async () => {
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    await userEvent.dblClick(screen.getByText('refacto-auth'));
    await userEvent.type(screen.getByDisplayValue('refacto-auth'), 'xx{Escape}');
    expect(backend.called('rename_agent')).toHaveLength(0);
    expect(screen.getByText('refacto-auth')).toBeInTheDocument();
  });

  it('creates an agent', async () => {
    const backend = fakeBackend({ create_agent: () => agent({ id: 'a4', name: 'agent-3', createdAt: 4 }) });
    render(Sidebar, { project: project() });
    await userEvent.click(screen.getByRole('button', { name: /Nouvel agent/ }));
    expect(backend.called('create_agent')[0].args).toEqual({ projectId: 'p1', model: null, engine: null, isolated: null, base: null });
    expect(app.agent?.id).toBe('a4');
  });

  describe('a new agent on a branch', () => {
    const FOLDER = branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api' });
    const LIST = [FOLDER, branchInfo({ name: 'feat/login' }), branchInfo({ name: 'origin/hotfix', remote: true })];
    const more = () => screen.getByRole('button', { name: 'Autres façons de créer un agent' });
    const handlers = () => ({
      branch_list: () => LIST,
      create_agent_on_branch: (a: any) => agent({ id: 'a9', name: 'agent-9', projectId: a.projectId, createdAt: 9 }),
    });

    it('offers it in a menu beside « Nouvel agent »', async () => {
      fakeBackend(handlers());
      render(Sidebar, { project: project() });
      expect(more()).toHaveAttribute('aria-haspopup', 'menu');
      await userEvent.click(more());
      expect(menu.open!.items.map((i) => i.label)).toEqual([
        'Nouvel agent sur une branche…',
        'Nouvel agent dans un worktree',
        'Nouvel agent depuis une autre branche…',
      ]);
      // The button itself still makes an agent at once.
      expect(screen.getByRole('button', { name: /^\+?\s*Nouvel agent$/ })).toBeInTheDocument();
    });

    it('opens the branch picker, which gives the agent its branch', async () => {
      const backend = fakeBackend(handlers());
      render(Sidebar, { project: project() });
      await userEvent.click(more());
      menu.open!.items[0].onClick!();
      menu.close();
      const picker = await screen.findByRole('dialog', { name: 'Choisir une branche' });
      expect(within(picker).queryByRole('button', { name: /Nouvelle branche/ })).toBeNull();
      await userEvent.click(await within(picker).findByText('origin/hotfix'));
      expect(backend.called('create_agent_on_branch')[0].args).toEqual({
        projectId: 'p1',
        branch: 'origin/hotfix',
        model: null,
        engine: null,
      });
      await vi.waitFor(() => expect(app.agent?.id).toBe('a9'));
      expect(screen.queryByRole('dialog', { name: 'Choisir une branche' })).toBeNull();
    });

    it('creates the branch typed in the picker, and starts the agent on it (ESC-155)', async () => {
      const backend = fakeBackend({ ...handlers(), branch_check: () => null });
      render(Sidebar, { project: project() });
      await userEvent.click(more());
      menu.open!.items[0].onClick!();
      menu.close();
      const picker = await screen.findByRole('dialog', { name: 'Choisir une branche' });
      await within(picker).findByText('feat/login');
      await userEvent.keyboard('feat/signup');
      await userEvent.click(await within(picker).findByRole('option', { name: /Créer la branche « feat\/signup »/ }));
      expect(backend.called('create_agent_on_branch')[0].args).toEqual({
        projectId: 'p1',
        branch: 'feat/signup',
        model: null,
        engine: null,
        create: true,
        base: null,
      });
      await vi.waitFor(() => expect(app.agent?.id).toBe('a9'));
    });

    it('cannot give it the branch of the project’s folder', async () => {
      const backend = fakeBackend(handlers());
      render(Sidebar, { project: project() });
      await userEvent.click(more());
      menu.open!.items[0].onClick!();
      menu.close();
      const picker = await screen.findByRole('dialog', { name: 'Choisir une branche' });
      const main = (await within(picker).findByText('main')).closest('[role="option"]')!;
      expect(main).toHaveAttribute('aria-disabled', 'true');
      await userEvent.click(main);
      expect(backend.called('create_agent_on_branch')).toHaveLength(0);
    });

    it('works from the keyboard, and gives the focus back to the button once the picker is closed', async () => {
      fakeBackend(handlers());
      render(Sidebar, { project: project() });
      render(ContextMenu);
      more().focus();
      await userEvent.keyboard('{Enter}');
      const entry = await screen.findByRole('menuitem', { name: 'Nouvel agent sur une branche…' });
      expect(entry).toHaveFocus();
      await userEvent.keyboard('{Enter}');
      await screen.findByRole('dialog', { name: 'Choisir une branche' });
      expect(screen.getByRole('combobox')).toHaveFocus();
      await userEvent.keyboard('{Escape}');
      await vi.waitFor(() => expect(screen.queryByRole('dialog', { name: 'Choisir une branche' })).toBeNull());
      expect(more()).toHaveFocus();
    });

    it('is written in English', async () => {
      setLang('en');
      fakeBackend(handlers());
      render(Sidebar, { project: project() });
      await userEvent.click(screen.getByRole('button', { name: 'Other ways to create an agent' }));
      expect(menu.open!.items.map((i) => i.label)).toEqual([
        'New agent on a branch…',
        'New agent in a worktree',
        'New agent from another branch…',
      ]);
      menu.open!.items[0].onClick!();
      menu.close();
      expect(await screen.findByRole('dialog', { name: 'Choose a branch' })).toBeInTheDocument();
    });
  });

  describe('a new agent from another branch (ESC-160)', () => {
    const LIST = [
      branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api' }),
      branchInfo({ name: 'release' }),
      branchInfo({ name: 'origin/hotfix', remote: true }),
    ];
    const more = () => screen.getByRole('button', { name: 'Autres façons de créer un agent' });
    const made = (a: any) => agent({ id: 'a9', name: 'agent-9', projectId: a.projectId, createdAt: 9 });
    const entry = (label: string) => menu.open!.items.find((i) => i.label === label)!;

    it('opens the picker of the starting branch, any branch, and makes the agent in a worktree from the one picked', async () => {
      const backend = fakeBackend({ branch_list: () => LIST, create_agent: made });
      render(Sidebar, { project: project({ worktreePerAgent: false }) });
      await userEvent.click(more());
      entry('Nouvel agent depuis une autre branche…').onClick!();
      menu.close();
      const picker = await screen.findByRole('dialog', { name: 'Branche de départ' });
      // The folder's branch too: it is only where the worktree starts.
      const main = (await within(picker).findByText('main')).closest('[role="option"]')!;
      expect(main).not.toHaveAttribute('aria-disabled');
      await userEvent.click(within(picker).getByText('release'));
      await waitFor(() => expect(backend.called('create_agent')).toHaveLength(1));
      expect(backend.called('create_agent')[0].args).toEqual({
        projectId: 'p1',
        model: null,
        engine: null,
        isolated: true,
        base: 'release',
      });
      await vi.waitFor(() => expect(app.agent?.id).toBe('a9'));
      expect(screen.queryByRole('dialog', { name: 'Branche de départ' })).toBeNull();
    });

    it('creates the branch typed from another branch, chosen next', async () => {
      const backend = fakeBackend({ branch_list: () => LIST, branch_check: () => null, create_agent_on_branch: made });
      render(Sidebar, { project: project() });
      await userEvent.click(more());
      entry('Nouvel agent sur une branche…').onClick!();
      menu.close();
      const picker = await screen.findByRole('dialog', { name: 'Choisir une branche' });
      await within(picker).findByText('release');
      await userEvent.keyboard('feat/signup');
      await userEvent.click(await within(picker).findByRole('option', { name: /Créer « feat\/signup » depuis une autre branche…/ }));
      const base = await screen.findByRole('dialog', { name: 'Branche de départ de « feat/signup »' });
      await userEvent.click(await within(base).findByText('origin/hotfix'));
      await waitFor(() => expect(backend.called('create_agent_on_branch')).toHaveLength(1));
      expect(backend.called('create_agent_on_branch')[0].args).toEqual({
        projectId: 'p1',
        branch: 'feat/signup',
        model: null,
        engine: null,
        create: true,
        base: 'origin/hotfix',
      });
      await vi.waitFor(() => expect(app.agent?.id).toBe('a9'));
    });

    it('makes nothing when the picker is closed', async () => {
      const backend = fakeBackend({ branch_list: () => LIST, create_agent: made });
      render(Sidebar, { project: project() });
      await userEvent.click(more());
      entry('Nouvel agent depuis une autre branche…').onClick!();
      menu.close();
      await screen.findByRole('dialog', { name: 'Branche de départ' });
      await userEvent.keyboard('{Escape}');
      await vi.waitFor(() => expect(screen.queryByRole('dialog', { name: 'Branche de départ' })).toBeNull());
      expect(backend.called('create_agent')).toHaveLength(0);
    });
  });

  describe('a new agent the other way round from « Un worktree par agent » (ESC-144)', () => {
    const more = () => screen.getByRole('button', { name: 'Autres façons de créer un agent' });

    it('makes one without a worktree in a project that gives each its own, and one in a worktree otherwise', async () => {
      const backend = fakeBackend({ create_agent: (a: any) => agent({ id: 'a8', name: 'agent-8', projectId: a.projectId, createdAt: 8 }) });
      const { unmount } = render(Sidebar, { project: project({ worktreePerAgent: true }) });
      await userEvent.click(more());
      const without = menu.open!.items.find((i) => i.label === 'Nouvel agent sans worktree')!;
      expect(menu.open!.items.map((i) => i.label)).not.toContain('Nouvel agent dans un worktree');
      without.onClick!();
      menu.close();
      await waitFor(() => expect(backend.called('create_agent')).toHaveLength(1));
      expect(backend.called('create_agent')[0].args).toEqual({ projectId: 'p1', model: null, engine: null, isolated: false, base: null });
      await vi.waitFor(() => expect(app.agent?.id).toBe('a8'));
      unmount();
      render(Sidebar, { project: project({ worktreePerAgent: false }) });
      await userEvent.click(more());
      menu.open!.items.find((i) => i.label === 'Nouvel agent dans un worktree')!.onClick!();
      menu.close();
      await waitFor(() => expect(backend.called('create_agent')).toHaveLength(2));
      expect(backend.called('create_agent')[1].args).toEqual({ projectId: 'p1', model: null, engine: null, isolated: true, base: null });
    });

    it('is written in English', async () => {
      setLang('en');
      fakeBackend();
      render(Sidebar, { project: project({ worktreePerAgent: true }) });
      await userEvent.click(screen.getByRole('button', { name: 'Other ways to create an agent' }));
      expect(menu.open!.items.map((i) => i.label)).toContain('New agent without a worktree');
    });
  });

  describe('« Intégrer <base> »', () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a1', branch: 'escouade/a1', baseBranch: 'develop' };

    it('is in the menu of an agent with a worktree, and integrates its base', async () => {
      resetApp({ agents: [agent({ worktree: wt })] });
      const backend = fakeBackend({ integrate_base: () => ({ kind: 'done' }) });
      render(Sidebar, { project: project() });
      await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
      const labels = menu.open!.items.map((i) => i.label);
      expect(labels).toContain('Intégrer develop');
      expect(labels.indexOf('Intégrer develop')).toBeGreaterThan(labels.indexOf('Archiver'));
      menu.open!.items.find((i) => i.label === 'Intégrer develop')!.onClick!();
      await vi.waitFor(() => expect(backend.called('integrate_base')).toHaveLength(1));
      expect(backend.called('integrate_base')[0].args).toEqual({ id: 'a1' });
      await vi.waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'develop intégrée dans escouade/a1.', kind: 'ok' }));
    });

    it('is not offered for an agent in the project’s folder, nor for an archived one', async () => {
      resetApp({ agents: [agent(), agent({ id: 'a3', name: 'vieux', archived: true, worktree: wt })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
      expect(menu.open!.items.map((i) => i.label).join()).not.toContain('Intégrer');
      await userEvent.click(screen.getByText(/Archivés/));
      await fireEvent.contextMenu(screen.getByRole('button', { name: /vieux/ }));
      expect(menu.open!.items.map((i) => i.label).join()).not.toContain('Intégrer');
    });
  });

  describe('deleting the agent of an existing branch', () => {
    it('says the branch is kept, where an ordinary agent’s goes with its worktree', async () => {
      const own = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a1', branch: 'escouade/a1', baseBranch: 'main' };
      const taken = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\login', branch: 'feat/login', baseBranch: 'main', existing: true };
      resetApp({ agents: [agent({ worktree: own }), agent({ id: 'a2', name: 'login', worktree: taken, createdAt: 2 })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      const option = async (name: RegExp) => {
        await fireEvent.contextMenu(screen.getByRole('button', { name }));
        // The last entry, whatever the language: it deletes.
        menu.open!.items.find((i) => i.danger)!.onClick!();
        return (app.modal as any).option.label as string;
      };
      expect(await option(/refacto-auth/)).toBe('Supprimer aussi le worktree et la branche escouade/a1');
      expect(await option(/login/)).toBe('Supprimer aussi le worktree (la branche feat/login est conservée)');
      setLang('en');
      expect(await option(/login/)).toBe('Also delete the worktree (the branch feat/login is kept)');
    });
  });

  it('tells when an agent stopped by the usage limit resumes', () => {
    fakeBackend();
    app.now = new Date(2026, 8, 30, 12, 0).getTime();
    app.agents.a1 = { ...app.agents.a1, status: 'error', resumeAt: new Date(2026, 8, 30, 15, 0).getTime() };
    render(Sidebar, { project: project() });
    const status = screen.getByText('Reprise à 15:00');
    expect(status).toHaveAttribute('data-tip', 'Arrêté par la limite d’usage : reprise automatique à 15:00');
  });

  it('leaves the project’s color to its tab menu', () => {
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(screen.queryAllByRole('button', { name: /^Couleur/ })).toEqual([]);
    expect(screen.queryByText('Couleur')).not.toBeInTheDocument();
  });
});

describe('Sidebar launch commands', () => {
  it('no longer highlights the agent while a launch command’s log is shown', async () => {
    const p = project({ runCommands: [{ id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: '' }] });
    resetApp({ projects: [p], agents: [agent()] });
    fakeBackend();
    render(Sidebar, { project: p });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(card).toHaveClass('sel');
    await userEvent.click(screen.getByRole('button', { name: /^Front/ }));
    expect(card).not.toHaveClass('sel');
  });
});

describe('Sidebar agent card', () => {
  it('shows the tokens and the estimated cost of the running turn', () => {
    resetApp({ projects: [project()], agents: [agent({ status: 'running', tokens: 1000, cost: 0.2, liveTokens: 500, liveCost: 0.1 })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(within(card).getByText('1,5 k tok')).toBeInTheDocument();
    expect(within(card).getByText('≈ 0,30 $')).toBeInTheDocument();
  });
});

describe('Sidebar agent card plan line', () => {
  const task = (id: string, status: 'pending' | 'inProgress' | 'done', over: Record<string, unknown> = {}) => ({
    id,
    title: `Tâche ${id}`,
    status,
    ...over,
  });
  const sub = (id: string, status = 'running') => ({ id, title: `sous-agent ${id}`, status, background: false, tools: 0, startedAt: 1 });
  const planOf = (tasks: unknown[], agents: unknown[] = [], over: Record<string, unknown> = {}) =>
    ({ source: 'tools', tasks, agents, launched: agents.length, workflows: [], ...over }) as never;
  const seven = () => [
    task('1', 'done'),
    task('2', 'done'),
    task('3', 'inProgress', { steps: [1, 2] }),
    task('4', 'pending'),
    task('5', 'pending'),
    task('6', 'pending'),
    task('7', 'pending'),
  ];
  const mini = (name: RegExp) => screen.getByRole('button', { name }).querySelector<HTMLElement>('.plan-mini');

  it('tells the tasks done and how far the plan is, between the figures and the ticket tag', () => {
    resetApp({ agents: [agent({ plan: planOf(seven()) })], tickets: [ticket({ column: 'doing', agentId: 'a1', iteration: 1 })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    const line = mini(/refacto-auth/)!;
    expect(line).not.toBeNull();
    expect(line).toHaveTextContent('Plan 2/7');
    expect(line).not.toHaveTextContent('sous-agent');
    // After the line of tokens, cost and files; before the ticket.
    const parts = [...card.children];
    expect(parts.indexOf(line)).toBe(parts.indexOf(card.querySelector('.meta.dim')!) + 1);
    expect(parts.indexOf(card.querySelector('.ticket-tag')!)).toBe(parts.indexOf(line) + 1);
    // (100 + 100 + 50) / 7 = 36 %: the bar is that wide, and says nothing to a screen reader the text does not.
    const bar = line.querySelector<HTMLElement>('.plan-bar')!;
    expect(bar).toHaveAttribute('aria-hidden', 'true');
    expect(bar.querySelector<HTMLElement>('span')!.style.width).toBe('36%');
  });

  it('counts the subagents running, in the plural and in the singular', () => {
    resetApp({
      agents: [
        agent({ plan: planOf(seven(), [sub('a'), sub('b'), sub('c'), sub('d', 'done')]) }),
        agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, plan: planOf(seven(), [sub('a'), sub('b', 'interrupted')]) }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(mini(/refacto-auth/)).toHaveTextContent('Plan 2/7 · 3 sous-agents');
    expect(mini(/tests-e2e/)).toHaveTextContent('Plan 2/7 · 1 sous-agent');
    expect(mini(/tests-e2e/)).not.toHaveTextContent('sous-agents');
  });

  it('is not on the card of an agent without tasks, with or without subagents', () => {
    resetApp({
      agents: [
        agent(),
        agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, plan: planOf([], [], { source: 'plan', planFile: 'docs/p.md' }) }),
        agent({ id: 'a3', name: 'explorateur', createdAt: 3, plan: planOf([], [sub('a')], { source: null }) }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    for (const name of [/refacto-auth/, /tests-e2e/, /explorateur/]) expect(mini(name)).toBeNull();
  });

  it('follows the plan', async () => {
    resetApp({ agents: [agent({ plan: planOf([task('1', 'inProgress'), task('2', 'pending')]) })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(mini(/refacto-auth/)).toHaveTextContent('Plan 0/2');
    app.agents.a1 = agent({ plan: planOf([task('1', 'done'), task('2', 'inProgress')]) });
    await Promise.resolve();
    expect(mini(/refacto-auth/)).toHaveTextContent('Plan 1/2');
    expect(mini(/refacto-auth/)!.querySelector<HTMLElement>('.plan-bar span')!.style.width).toBe('50%');
    app.agents.a1 = agent();
    await Promise.resolve();
    expect(mini(/refacto-auth/)).toBeNull();
  });

  it('shows the words of the agent as text', () => {
    const bad = '<img src=x onerror=alert(1)>';
    resetApp({ agents: [agent({ plan: planOf([task('1', 'inProgress', { title: bad })], [], { title: bad }) })] });
    fakeBackend();
    const { container } = render(Sidebar, { project: project() });
    expect(container.querySelector('img')).toBeNull();
  });
});

describe('Sidebar remote control', () => {
  const items = () => menu.open?.items.map((i) => i.label) ?? [];
  const click = (label: string) => menu.open!.items.find((i) => i.label === label)!.onClick!();

  it('turns Remote Control on from the agent’s context menu', async () => {
    resetApp({ projects: [project()], agents: [agent()] });
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(items()).toContain('Activer le remote control');
    click('Activer le remote control');
    await new Promise((r) => setTimeout(r));
    expect(backend.called('set_remote_control')[0].args).toEqual({ id: 'a1', enabled: true });
  });

  it('shows a remote agent’s link state and offers its claude.ai session', async () => {
    const url = 'https://claude.ai/code/session_abc';
    resetApp({ projects: [project()], agents: [agent({ remoteControl: true, remoteUrl: url, remoteState: 'connected' })] });
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(getByTip(card, /Remote control : connecté/)).toBeInTheDocument();
    await fireEvent.contextMenu(card);
    expect(items()).toEqual(expect.arrayContaining(['Désactiver le remote control', 'Ouvrir sur claude.ai', 'Copier le lien claude.ai']));
    click('Désactiver le remote control');
    await new Promise((r) => setTimeout(r));
    expect(backend.called('set_remote_control')[0].args).toEqual({ id: 'a1', enabled: false });
  });

  it('asks a worktree agent for its launch recipe from its menu', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a1', branch: 'escouade/a1', baseBranch: 'main' };
    resetApp({ agents: [agent({ worktree: wt })] });
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    menu.open!.items.find((i) => i.label === 'Préparer le lancement')!.onClick!();
    expect(backend.called('agent_prepare_launch')).toEqual([{ cmd: 'agent_prepare_launch', args: { id: 'a1' } }]);
  });

  it('does not ask for a recipe an agent without a worktree, or a ticket agent under way whose ticket already asks', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a1', branch: 'escouade/a1', baseBranch: 'main' };
    resetApp({
      agents: [agent({ worktree: wt }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 })],
      tickets: [ticket({ agentId: 'a1', column: 'doing' })],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(items()).not.toContain('Préparer le lancement');
    await fireEvent.contextMenu(screen.getByRole('button', { name: /tests-e2e/ }));
    expect(items()).not.toContain('Préparer le lancement');
  });

  describe('test launches', () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a1', branch: 'escouade/a1', baseBranch: 'main' };
    /** The order in which the window stopped the launches and acted on the agent. */
    const order = (backend: ReturnType<typeof fakeBackend>, cmd: string) =>
      backend.calls.map((c) => c.cmd).filter((c) => c === 'term_kill' || c === cmd);

    beforeEach(() => {
      resetApp({ agents: [agent({ worktree: wt })] });
      app.launches['test:a1:run:0'] = { status: 'running', ptyId: 't1', name: 'web', stopping: false, code: null, startedAt: 1 };
    });

    it('stops them before archiving their agent', async () => {
      const backend = fakeBackend();
      render(Sidebar, { project: project() });
      await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
      click('Archiver');
      await new Promise((r) => setTimeout(r));
      expect(order(backend, 'archive_agent')).toEqual(['term_kill', 'archive_agent']);
    });

    it('stops them before deleting their agent', async () => {
      const backend = fakeBackend();
      render(Sidebar, { project: project() });
      await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
      click('Supprimer…');
      await (app.modal as any).onConfirm(true);
      expect(order(backend, 'delete_agent')).toEqual(['term_kill', 'delete_agent']);
    });
  });

  it('asks before archiving the agent of a ticket under way or to test, whose ticket goes back to do', async () => {
    resetApp({
      agents: [agent(), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 }), agent({ id: 'a3', name: 'libre', createdAt: 3 })],
      tickets: [
        ticket({ column: 'doing', agentId: 'a1' }),
        ticket({ id: 't2', key: 'DEM-2', column: 'review', agentId: 'a2' }),
        ticket({ id: 't3', key: 'DEM-3', column: 'done', agentId: 'a3' }),
      ],
    });
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    for (const [name, key, id] of [
      ['refacto-auth', 'DEM-1', 'a1'],
      ['tests-e2e', 'DEM-2', 'a2'],
    ] as const) {
      await fireEvent.contextMenu(screen.getByRole('button', { name: new RegExp(name) }));
      click('Archiver');
      expect(backend.called('archive_agent')).toHaveLength(0);
      expect(app.modal).toMatchObject({
        kind: 'confirm',
        title: `Archiver ${name} ?`,
        body: `Son ticket ${key} repartira « À faire ».`,
        confirm: 'Archiver',
      });
      await (app.modal as any).onConfirm(false);
      expect(backend.called('archive_agent').at(-1)?.args).toEqual({ id, archived: true });
      backend.calls.length = 0;
      app.modal = null;
    }
    // A done ticket holds its agent no more: archived at once.
    await fireEvent.contextMenu(screen.getByRole('button', { name: /libre/ }));
    click('Archiver');
    await new Promise((r) => setTimeout(r));
    expect(app.modal).toBeNull();
    expect(backend.called('archive_agent')[0].args).toEqual({ id: 'a3', archived: true });
  });

  it('says when a remote agent is not reachable yet', () => {
    resetApp({ projects: [project()], agents: [agent({ remoteControl: true, remoteState: null })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(getByTip(document.body, /Remote control : en attente de connexion/)).toBeInTheDocument();
  });

  describe('where it comes from', () => {
    const STATE = 'Remote control : connecté (accessible depuis claude.ai et l’app Claude)';

    it.each([
      ['hand', 'activé à la main'],
      ['project', 'activé par le réglage du projet'],
      ['global', 'activé par le réglage de tous les agents'],
    ] as const)('says on the card of an agent whose remote control is %s’s, after its link state', (remoteSource, source) => {
      resetApp({ projects: [project()], agents: [agent({ remoteControl: true, remoteSource, remoteState: 'connected' })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      const card = screen.getByRole('button', { name: /refacto-auth/ });
      expect(getByTip(card, `${STATE} · ${source}`)).toBeInTheDocument();
    });

    it('follows the link state with the source, whatever the state', () => {
      resetApp({
        projects: [project()],
        agents: [
          agent({ remoteControl: true, remoteSource: 'project', remoteState: 'ready' }),
          agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, remoteControl: true, remoteSource: 'global', remoteState: null }),
        ],
      });
      fakeBackend();
      render(Sidebar, { project: project() });
      expect(
        getByTip(
          screen.getByRole('button', { name: /refacto-auth/ }),
          'Remote control : connexion… (accessible depuis claude.ai et l’app Claude) · activé par le réglage du projet',
        ),
      ).toBeInTheDocument();
      expect(
        getByTip(
          screen.getByRole('button', { name: /tests-e2e/ }),
          'Remote control : en attente de connexion (accessible depuis claude.ai et l’app Claude) · activé par le réglage de tous les agents',
        ),
      ).toBeInTheDocument();
    });

    it('keeps the plain state when the backend gives no source', () => {
      resetApp({ projects: [project()], agents: [agent({ remoteControl: true, remoteSource: null, remoteState: 'connected' })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      expect(getByTip(document.body, STATE)).toBeInTheDocument();
    });

    it('has no icon for an agent without remote control, set apart or not', () => {
      resetApp({
        projects: [project()],
        agents: [agent({ remoteControl: false }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, remoteOff: true })],
      });
      fakeBackend();
      render(Sidebar, { project: project() });
      expect(document.querySelector('.rc')).toBeNull();
    });
  });

  describe('of an agent in Claude Code’s view', () => {
    const STATE_CONNECTED = 'Remote control : connecté (accessible depuis claude.ai et l’app Claude)';
    const STATE_CONNECTING = 'Remote control : connexion… (accessible depuis claude.ai et l’app Claude)';
    const NO_INTERFACE = 'Remote control : aucune interface lancée, pas joignable pour l’instant (ouvre l’agent pour la lancer)';
    const NEXT_LAUNCH_STATE = 'Remote control : au prochain lancement de son interface Claude Code';
    const UNAVAILABLE =
      'Remote control : pas ouvert par Claude Code (compte sans abonnement claude.ai, réseau, proxy ou règle de l’organisation)';
    const STILL_ON =
      'Remote control encore actif dans l’interface ouverte, jusqu’à son prochain lancement (sa commande /remote-control le coupe)';
    const NEXT_LAUNCH =
      'Pris en compte au prochain lancement de son interface Claude Code ; d’ici là, c’est sa commande /remote-control qui le fait.';
    const SESSION = 'https://claude.ai/code/session_abc';
    const toasts = () => app.toasts.map((x) => [x.text, x.kind]);
    const settle = () => new Promise((r) => setTimeout(r));
    /** An agent in Claude Code’s view, with the remote control `over` says. */
    const inClaudeCode = (over: Partial<Parameters<typeof agent>[0]> = {}) => agent({ terminal: true, ...over });
    const icon = (name: RegExp, text: string) => getByTip(screen.getByRole('button', { name }), text);

    describe('on its card', () => {
      it('says no interface runs when it has none: not reachable until one starts', () => {
        resetApp({
          projects: [project()],
          agents: [
            // The state is the interface’s: with none, a stale one is of no account.
            inClaudeCode({ remoteControl: true, remoteSource: 'global', interfaceRemote: null, remoteState: 'connected' }),
          ],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        const tip = icon(/refacto-auth/, `${NO_INTERFACE} · activé par le réglage de tous les agents`);
        expect(tip).toBeInTheDocument();
        // Neutral: no link to speak of.
        expect(tip).not.toHaveClass('on');
        expect(tip).not.toHaveClass('warn');
      });

      it('says its interface takes the remote control from its next launch when it was started without it', () => {
        resetApp({
          projects: [project()],
          agents: [
            inClaudeCode({ remoteControl: true, remoteSource: 'project', interfaceRemote: false }),
            // Whatever the state the interface reports: it has not the remote control of Escouade’s.
            inClaudeCode({
              id: 'a2',
              name: 'tests-e2e',
              createdAt: 2,
              remoteControl: true,
              remoteSource: 'hand',
              interfaceRemote: false,
              remoteState: 'connected',
            }),
          ],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        const later = icon(/refacto-auth/, `${NEXT_LAUNCH_STATE} · activé par le réglage du projet`);
        expect(later).not.toHaveClass('on');
        expect(later).not.toHaveClass('warn');
        expect(icon(/tests-e2e/, `${NEXT_LAUNCH_STATE} · activé à la main`)).not.toHaveClass('on');
      });

      it('says Claude Code did not open the remote control when its interface runs without having been linked (no subscription, network, proxy or policy)', () => {
        resetApp({
          projects: [project()],
          agents: [inClaudeCode({ remoteControl: true, remoteSource: 'global', interfaceRemote: true, remoteState: 'unavailable' })],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        const tip = icon(/refacto-auth/, `${UNAVAILABLE} · activé par le réglage de tous les agents`);
        expect(tip).not.toHaveClass('on');
        expect(tip).not.toHaveClass('warn');
      });

      it('is blue and says it is connected when its interface says its remote control is on', () => {
        resetApp({
          projects: [project()],
          agents: [
            inClaudeCode({
              remoteControl: true,
              remoteSource: 'project',
              interfaceRemote: true,
              remoteUrl: SESSION,
              remoteState: 'connected',
            }),
          ],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        const tip = icon(/refacto-auth/, `${STATE_CONNECTED} · activé par le réglage du projet`);
        expect(tip).toHaveClass('on');
        expect(tip).not.toHaveClass('warn');
      });

      it('says it is connecting while its interface, started with the remote control, does not say it is on yet', () => {
        resetApp({
          projects: [project()],
          agents: [
            inClaudeCode({ remoteControl: true, remoteSource: 'hand', interfaceRemote: true, remoteState: null }),
            inClaudeCode({ id: 'a2', name: 'tests-e2e', createdAt: 2, remoteControl: true, interfaceRemote: true, remoteState: 'ready' }),
          ],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        const tip = icon(/refacto-auth/, `${STATE_CONNECTING} · activé à la main`);
        expect(tip).not.toHaveClass('on');
        expect(tip).not.toHaveClass('warn');
        expect(icon(/tests-e2e/, STATE_CONNECTING)).not.toHaveClass('on');
      });

      it('shows a lasting marker when the remote control is off but the open interface is still linked', () => {
        resetApp({
          projects: [project()],
          agents: [
            inClaudeCode({ remoteControl: false, remoteOff: true, interfaceRemote: true, remoteState: 'connected', remoteUrl: SESSION }),
          ],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        const tip = icon(/refacto-auth/, STILL_ON);
        // Set apart from the blue and the grey ones: a warning, barred.
        expect(tip).toHaveClass('warn');
        expect(tip).not.toHaveClass('on');
        expect(tip.querySelector('.bar')).not.toBeNull();
      });

      it('bars only the lasting marker', () => {
        resetApp({
          projects: [project()],
          agents: [inClaudeCode({ remoteControl: true, interfaceRemote: true, remoteState: 'connected' })],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        expect(document.querySelector('.rc')).not.toBeNull();
        expect(document.querySelector('.rc .bar')).toBeNull();
      });

      it('shows no marker when the interface is not linked, or is not the one that is shown', () => {
        resetApp({
          projects: [project()],
          agents: [
            // Off, and its interface started without it, or none.
            inClaudeCode({ remoteControl: false, interfaceRemote: false }),
            inClaudeCode({ id: 'a2', name: 'tests-e2e', createdAt: 2, remoteControl: false, interfaceRemote: null }),
            // Off, its interface started with it, but Claude Code did not open it: it is linked to nothing.
            inClaudeCode({ id: 'a3', name: 'lint', createdAt: 3, remoteControl: false, interfaceRemote: true, remoteState: 'unavailable' }),
            // Off, its interface started with it, but not linked (yet): nothing to warn about.
            inClaudeCode({ id: 'a5', name: 'booting', createdAt: 5, remoteControl: false, interfaceRemote: true, remoteState: null }),
            // Not in Claude Code’s view: the interface is not Escouade’s to show.
            agent({ id: 'a4', name: 'docs', createdAt: 4, remoteControl: false, interfaceRemote: true }),
          ],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        expect(document.querySelector('.rc')).toBeNull();
      });
    });

    describe('in its menu', () => {
      it('offers to turn it on, as an agent in Escouade’s view, with no claude.ai session to open', async () => {
        resetApp({ projects: [project()], agents: [inClaudeCode({ remoteUrl: SESSION })] });
        const backend = fakeBackend();
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        expect(items()).toContain('Activer le remote control');
        expect(items()).not.toContain('Désactiver le remote control');
        expect(items()).not.toContain('Ouvrir sur claude.ai');
        expect(items()).not.toContain('Copier le lien claude.ai');
        click('Activer le remote control');
        await settle();
        expect(backend.called('set_remote_control')[0].args).toEqual({ id: 'a1', enabled: true });
      });

      it('offers to turn it off, and its claude.ai session, once its interface says it is connected', async () => {
        resetApp({
          projects: [project()],
          agents: [
            inClaudeCode({
              remoteControl: true,
              remoteSource: 'global',
              interfaceRemote: true,
              remoteUrl: SESSION,
              remoteState: 'connected',
            }),
          ],
        });
        const backend = fakeBackend();
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        expect(items()).toContain('Désactiver le remote control');
        expect(items()).not.toContain('Activer le remote control');
        expect(items()).toEqual(expect.arrayContaining(['Ouvrir sur claude.ai', 'Copier le lien claude.ai']));
        click('Désactiver le remote control');
        await settle();
        expect(backend.called('set_remote_control')[0].args).toEqual({ id: 'a1', enabled: false });
      });

      it.each([
        ['it is not connected yet', { interfaceRemote: true, remoteState: null, remoteUrl: SESSION }],
        ['Claude Code did not open it', { interfaceRemote: true, remoteState: 'unavailable', remoteUrl: SESSION }],
        ['the interface says connected but gave no link', { interfaceRemote: true, remoteState: 'connected', remoteUrl: null }],
        ['no interface runs', { interfaceRemote: null, remoteState: null, remoteUrl: SESSION }],
      ] as const)('offers no claude.ai session when %s', async (_, over) => {
        resetApp({ projects: [project()], agents: [inClaudeCode({ remoteControl: true, ...over })] });
        fakeBackend();
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        expect(items()).toContain('Désactiver le remote control');
        expect(items()).not.toContain('Ouvrir sur claude.ai');
        expect(items()).not.toContain('Copier le lien claude.ai');
      });

      it('offers to turn it on, and no claude.ai session, while only its open interface is still linked', async () => {
        resetApp({
          projects: [project()],
          agents: [
            inClaudeCode({ remoteControl: false, remoteOff: true, interfaceRemote: true, remoteState: 'connected', remoteUrl: SESSION }),
          ],
        });
        const backend = fakeBackend({ set_remote_control: () => true });
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        expect(items()).toContain('Activer le remote control');
        expect(items()).not.toContain('Désactiver le remote control');
        expect(items()).not.toContain('Ouvrir sur claude.ai');
        expect(items()).not.toContain('Copier le lien claude.ai');
        click('Activer le remote control');
        await expect.poll(toasts).toEqual([[NEXT_LAUNCH, 'ok']]);
        expect(backend.called('set_remote_control')[0].args).toEqual({ id: 'a1', enabled: true });
      });

      it('copies the claude.ai session of its interface', async () => {
        const write = vi.fn(() => Promise.resolve());
        Object.defineProperty(navigator, 'clipboard', { value: { writeText: write }, configurable: true });
        resetApp({
          projects: [project()],
          agents: [inClaudeCode({ remoteControl: true, interfaceRemote: true, remoteUrl: SESSION, remoteState: 'connected' })],
        });
        fakeBackend();
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        click('Copier le lien claude.ai');
        await expect.poll(toasts).toEqual([['Lien claude.ai copié', 'ok']]);
        expect(write).toHaveBeenCalledWith(SESSION);
      });

      it('is not offered to an archived agent, in either view', async () => {
        resetApp({
          projects: [project()],
          agents: [
            agent({ archived: true, remoteControl: true }),
            agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, archived: true, terminal: true }),
          ],
        });
        fakeBackend();
        app.showArchived = true;
        render(Sidebar, { project: project() });
        for (const name of [/refacto-auth/, /tests-e2e/]) {
          await fireEvent.contextMenu(screen.getByRole('button', { name }));
          expect(items()).toContain('Restaurer');
          expect(items().filter((x) => /remote control|claude\.ai/.test(x))).toEqual([]);
        }
        app.showArchived = false;
      });
    });

    describe('when it is changed', () => {
      it.each([
        ['on', false],
        ['off', true],
      ] as const)(
        'tells it applies at the next launch of its interface when the backend says so, turning it %s',
        async (_, remoteControl) => {
          resetApp({ projects: [project()], agents: [inClaudeCode({ terminalId: 't1', remoteControl, interfaceRemote: remoteControl })] });
          fakeBackend({ set_remote_control: () => true });
          render(Sidebar, { project: project() });
          await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
          click(remoteControl ? 'Désactiver le remote control' : 'Activer le remote control');
          await expect.poll(toasts).toEqual([[NEXT_LAUNCH, 'ok']]);
        },
      );

      it('says it is reachable when turned on and the backend applies it now: it starts it in the background', async () => {
        resetApp({ projects: [project()], agents: [inClaudeCode({ terminalId: 't1' })] });
        const backend = fakeBackend({ set_remote_control: () => false });
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        click('Activer le remote control');
        await expect.poll(toasts).toEqual([['refacto-auth est accessible depuis claude.ai et l’app Claude', 'ok']]);
        expect(backend.called('set_remote_control')[0].args).toEqual({ id: 'a1', enabled: true });
      });

      it('says nothing when turned off and the backend applies it now: it stops the one it started', async () => {
        resetApp({ projects: [project()], agents: [inClaudeCode({ terminalId: null, remoteControl: true })] });
        const backend = fakeBackend({ set_remote_control: () => false });
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        click('Désactiver le remote control');
        await settle();
        expect(backend.called('set_remote_control')[0].args).toEqual({ id: 'a1', enabled: false });
        expect(toasts()).toEqual([]);
      });

      it('tells the refusal of the backend, and not that the change applies later', async () => {
        resetApp({ projects: [project()], agents: [inClaudeCode({ terminalId: 't1' })] });
        fakeBackend({
          set_remote_control: () => {
            throw 'Le remote control a échoué.';
          },
        });
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        click('Activer le remote control');
        await expect.poll(toasts).toEqual([['Le remote control a échoué.', 'error']]);
      });

      it('still says the agent is reachable when it is turned on in Escouade’s view', async () => {
        resetApp({ projects: [project()], agents: [agent({ terminalId: 't1' })] });
        fakeBackend({ set_remote_control: () => false });
        render(Sidebar, { project: project() });
        await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
        click('Activer le remote control');
        await expect.poll(toasts).toEqual([['refacto-auth est accessible depuis claude.ai et l’app Claude', 'ok']]);
      });
    });
  });
});

describe('Sidebar terminal entry', () => {
  const WT = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a2', branch: 'escouade/a2', baseBranch: 'main' };

  beforeEach(() => {
    vi.mocked(openTerminal).mockClear();
    resetApp({
      agents: [agent(), agent({ id: 'a2', name: 'wt-agent', worktree: WT }), agent({ id: 'a3', name: 'vieux', archived: true })],
    });
    app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
    fakeBackend();
  });
  // The list of archived agents, opened by one test, is not closed by the reset.
  afterEach(() => (app.showArchived = false));

  const menuOf = async (name: RegExp) => {
    await fireEvent.contextMenu(screen.getByRole('button', { name }));
    return menu.open!.items.map((i) => i.label);
  };
  const click = (label: string) => menu.open!.items.find((i) => i.label === label)!.onClick!();

  it('opens a terminal in the worktree of an agent from its menu, named after both', async () => {
    render(Sidebar, { project: project() });
    await menuOf(/wt-agent/);
    click('Ouvrir un terminal');
    expect(openTerminal).toHaveBeenCalledWith('p1', 'pwsh', 'pwsh · wt-agent', { agentId: 'a2' });
    await expect.poll(() => app.term?.name).toBe('pwsh · wt-agent');
  });

  it('asks it for an agent without a worktree too: the backend gives it the project’s folder', async () => {
    render(Sidebar, { project: project() });
    await menuOf(/refacto-auth/);
    click('Ouvrir un terminal');
    expect(openTerminal).toHaveBeenCalledWith('p1', 'pwsh', 'pwsh · refacto-auth', { agentId: 'a1' });
  });

  it('waits for the first message of an agent whose worktree waits for it: no terminal nor editor in the project’s folder (ESC-156)', async () => {
    app.agents.a1 = { ...app.agents.a1, worktreePending: true };
    render(Sidebar, { project: project() });
    await menuOf(/refacto-auth/);
    const why = 'Le worktree sera créé au premier message, au nom de l’agent';
    for (const label of ['Ouvrir un terminal', 'Ouvrir dans l’éditeur'])
      expect(menu.open!.items.find((i) => i.label === label)).toMatchObject({ disabled: true, title: why });
  });

  it('keeps it by the editor entry, and offers none for an archived agent', async () => {
    render(Sidebar, { project: project() });
    const labels = await menuOf(/wt-agent/);
    expect(labels.indexOf('Ouvrir un terminal')).toBe(labels.indexOf('Ouvrir dans l’éditeur') + 1);
    await userEvent.click(screen.getByText(/Archivés/));
    expect(await menuOf(/vieux/)).not.toContain('Ouvrir un terminal');
  });
});

describe('Sidebar, a launch command in the worktree of an agent (ESC-19)', () => {
  const WT = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a2', branch: 'escouade/a2', baseBranch: 'main' };
  const FRONT = { id: 'c1', name: 'Front', command: 'npm run dev', shell: 'pwsh', cwd: 'web' };
  const P = project({ runCommands: [FRONT] });

  beforeEach(() => {
    resetApp({ projects: [P], agents: [agent(), agent({ id: 'a2', name: 'wt-agent', worktree: WT })] });
    app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
  });

  const menuOf = async (name: RegExp) => {
    await fireEvent.contextMenu(screen.getByRole('button', { name }));
    return menu.open!.items.map((i) => i.label);
  };

  it('offers « Lancer… » for an agent with a worktree, then the commands, run in its worktree', async () => {
    const backend = fakeBackend({ run_start: () => ({ id: 't9', projectId: 'p1', name: 'Front · wt-agent', shell: 'pwsh' }) });
    render(Sidebar, { project: P });
    expect(await menuOf(/refacto-auth/)).not.toContain('Lancer…');
    expect(await menuOf(/wt-agent/)).toContain('Lancer…');
    menu.open!.items.find((i) => i.label === 'Lancer…')!.onClick!();
    await vi.waitFor(() => expect(menu.open?.items.map((i) => i.label)).toEqual(['Front']));
    menu.open!.items[0].onClick!();
    await vi.waitFor(() => expect(backend.called('run_start')).toHaveLength(1));
    expect(backend.called('run_start')[0].args).toMatchObject({ projectId: 'p1', commandId: 'c1', agentId: 'a2' });
  });

  it('offers nothing to run for a project without launch commands', async () => {
    resetApp({ projects: [project()], agents: [agent({ id: 'a2', name: 'wt-agent', worktree: WT })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(await menuOf(/wt-agent/)).not.toContain('Lancer…');
  });

  it('says it in English', async () => {
    setLang('en');
    fakeBackend();
    render(Sidebar, { project: P });
    expect(await menuOf(/wt-agent/)).toContain('Run…');
  });
});

describe('Sidebar terminals menu (ESC-18)', () => {
  const wt = (id: string) => ({ path: `C:\\code\\demo-api\\.claude\\worktrees\\${id}`, branch: `escouade/${id}`, baseBranch: 'main' });
  const plus = () => getByTip(document.body, 'Nouveau terminal');
  const labels = () => menu.open!.items.map((i) => i.label);

  beforeEach(() => {
    vi.mocked(openTerminal).mockClear();
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [
        // Created in this order, which is not the alphabetical one: the sidebar lists them so.
        agent({ id: 'a1', name: 'zeta', worktree: wt('a1'), createdAt: 1 }),
        agent({ id: 'a2', name: 'alpha', worktree: wt('a2'), createdAt: 2 }),
        // In the project's folder, archived, or of another project: no worktree to open one in.
        agent({ id: 'a3', name: 'dans-le-dossier', createdAt: 3 }),
        agent({ id: 'a4', name: 'vieux', worktree: wt('a4'), archived: true, createdAt: 4 }),
        agent({ id: 'a5', name: 'ailleurs', projectId: 'p2', worktree: wt('a5'), createdAt: 5 }),
      ],
    });
    app.shells = [
      { id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' },
      { id: 'bash', label: 'Git Bash', path: 'bash.exe' },
    ];
    fakeBackend();
  });
  afterEach(() => menu.close());

  it('lists the shells, then « Dans le worktree de… » after a separator', async () => {
    render(Sidebar, { project: project() });
    await userEvent.click(plus());
    expect(labels()).toEqual(['PowerShell 7', 'Git Bash', '', 'Dans le worktree de…']);
    expect(menu.open!.items[2]).toMatchObject({ separator: true });
    // The shells still open a terminal in the project.
    menu.open!.items[1].onClick!();
    expect(openTerminal).toHaveBeenCalledWith('p1', 'bash', 'bash-1', undefined);
  });

  it('replaces the menu with the agents of the project that have a worktree, in the order of the sidebar', async () => {
    render(Sidebar, { project: project() });
    await userEvent.click(plus());
    menu.open!.items[3].onClick!();
    await vi.waitFor(() => expect(labels()).toEqual(['zeta', 'alpha']));
    // Neither the one in the project’s folder, the archived one, nor the other project’s.
    expect(menu.open!.items.every((i) => i.hint === undefined && !i.separator)).toBe(true);
    // Opened from the button, as the first menu was: the keyboard works it.
    expect(menu.open).toMatchObject({ keyboard: true });
  });

  it('opens a terminal in the worktree of the agent chosen, named after it', async () => {
    render(Sidebar, { project: project() });
    await userEvent.click(plus());
    menu.open!.items[3].onClick!();
    await vi.waitFor(() => expect(labels()).toEqual(['zeta', 'alpha']));
    menu.open!.items.find((i) => i.label === 'alpha')!.onClick!();
    expect(openTerminal).toHaveBeenCalledWith('p1', 'pwsh', 'pwsh · alpha', { agentId: 'a2' });
    await expect.poll(() => app.term?.name).toBe('pwsh · alpha');
  });

  it('works from the keyboard: the focus goes into the second menu and comes back to the button', async () => {
    render(Sidebar, { project: project() });
    render(ContextMenu);
    plus().focus();
    await userEvent.keyboard('{Enter}');
    // (Its name carries the hint: the shell’s glyph.)
    expect(await screen.findByRole('menuitem', { name: /^PowerShell 7/ })).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}{ArrowDown}');
    expect(screen.getByRole('menuitem', { name: 'Dans le worktree de…' })).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    const first = await screen.findByRole('menuitem', { name: 'zeta' });
    expect(screen.queryByRole('menuitem', { name: /^PowerShell 7/ })).toBeNull();
    await vi.waitFor(() => expect(first).toHaveFocus());
    await userEvent.keyboard('{ArrowDown}{Enter}');
    expect(openTerminal).toHaveBeenCalledWith('p1', 'pwsh', 'pwsh · alpha', { agentId: 'a2' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(plus()).toHaveFocus();
  });

  it('works with the mouse as well', async () => {
    render(Sidebar, { project: project() });
    render(ContextMenu);
    await userEvent.click(plus());
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Dans le worktree de…' }));
    await userEvent.click(await screen.findByRole('menuitem', { name: 'zeta' }));
    expect(openTerminal).toHaveBeenCalledWith('p1', 'pwsh', 'pwsh · zeta', { agentId: 'a1' });
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('offers neither the entry nor a separator when no agent of the project has a worktree to open', async () => {
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [
        agent({ id: 'a3', name: 'dans-le-dossier' }),
        agent({ id: 'a4', name: 'vieux', worktree: wt('a4'), archived: true, createdAt: 2 }),
        agent({ id: 'a5', name: 'ailleurs', projectId: 'p2', worktree: wt('a5'), createdAt: 3 }),
      ],
    });
    app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
    render(Sidebar, { project: project() });
    await userEvent.click(plus());
    expect(labels()).toEqual(['PowerShell 7']);
    menu.close();
    // No agent at all: the same.
    resetApp({ projects: [project()], agents: [] });
    app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
    await userEvent.click(plus());
    expect(labels()).toEqual(['PowerShell 7']);
  });

  it('keeps telling there is no shell, whatever the agents', async () => {
    app.shells = [];
    render(Sidebar, { project: project() });
    await userEvent.click(plus());
    expect(menu.open).toBeNull();
    expect(app.toasts.at(-1)).toMatchObject({
      text: 'Aucun shell détecté (PowerShell 7, Git Bash, WSL). Vérifie les réglages.',
      kind: 'error',
    });
  });

  it('is written in English', async () => {
    setLang('en');
    render(Sidebar, { project: project() });
    await userEvent.click(getByTip(document.body, 'New terminal'));
    expect(labels()).toEqual(['PowerShell 7', 'Git Bash', '', 'In the worktree of…']);
    menu.open!.items[3].onClick!();
    await vi.waitFor(() => expect(labels()).toEqual(['zeta', 'alpha']));
  });
});

describe('Sidebar editor entries', () => {
  it('browses the project branch from its footer, an agent from its menu', async () => {
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo();
    fakeBackend();
    render(Sidebar, { project: project() });
    await userEvent.click(screen.getByRole('button', { name: 'Parcourir' }));
    expect(app.editor.p1).toMatchObject({ on: true, source: 'project' });
    app.closeEditor();
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    menu.open!.items.find((i) => i.label === 'Ouvrir dans l’éditeur')!.onClick!();
    expect(app.editorOn).toBe(true);
  });

  it('puts the browse button by the branch, full width for the path, and keeps it without a repository', async () => {
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo({ branch: 'feat/x' });
    fakeBackend();
    const { unmount } = render(Sidebar, { project: project() });
    expect(screen.getByRole('button', { name: 'Parcourir' }).closest('.branch')).toHaveTextContent('feat/x');
    expect(getByTip(document.body, project().path).parentElement).toHaveClass('foot');
    unmount();
    app.git.p1 = gitInfo({ isRepo: false });
    render(Sidebar, { project: project() });
    expect(screen.getByRole('button', { name: 'Parcourir' }).closest('.branch')).toHaveTextContent('Pas de dépôt git');
  });

  it('warns that the unsaved files of an agent’s worktree are lost when it is deleted', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a2', branch: 'escouade/a2', baseBranch: 'main' };
    resetApp({ agents: [agent(), agent({ id: 'a2', name: 'wt-agent', worktree: wt })] });
    fakeBackend({
      fs_read: () => ({ kind: 'text', text: 'a\n', size: 2, hash: 'h1', eol: 'lf', bom: false }),
      fs_base: () => null,
      set_unsaved: () => null,
    });
    for (const [source, path] of [
      ['a2', 'x.ts'],
      ['a2', 'y.ts'],
      ['project', 'x.ts'],
    ]) {
      buffers.edit((await buffers.open('p1', source, path)).key, 'mine\n');
    }
    render(Sidebar, { project: project() });
    const deleteBody = async (name: RegExp) => {
      await fireEvent.contextMenu(screen.getByRole('button', { name }));
      menu.open!.items.find((i) => i.label === 'Supprimer…')!.onClick!();
      return (app.modal as any).body as string;
    };
    expect(await deleteBody(/wt-agent/)).toMatch(/\. 2 fichiers non enregistrés dans l’éditeur seront perdus\.$/);
    // An agent without a worktree edits the project's checkout: its files stay open with the project.
    expect(await deleteBody(/refacto-auth/)).not.toContain('éditeur');
  });

  it('opens an agent with a worktree on that worktree, and offers nothing for an archived one', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a2', branch: 'escouade/a2', baseBranch: 'main' };
    resetApp({ agents: [agent({ id: 'a2', name: 'wt-agent', worktree: wt }), agent({ id: 'a3', name: 'vieux', archived: true })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /wt-agent/ }));
    menu.open!.items.find((i) => i.label === 'Ouvrir dans l’éditeur')!.onClick!();
    expect(app.editor.p1).toMatchObject({ on: true, source: 'a2' });
    await userEvent.click(screen.getByText(/Archivés/));
    await fireEvent.contextMenu(screen.getByRole('button', { name: /vieux/ }));
    expect(menu.open!.items.map((i) => i.label)).not.toContain('Ouvrir dans l’éditeur');
  });

  it('no longer highlights the agent while the board fills the main area', () => {
    resetApp({ agents: [agent()] });
    fakeBackend();
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(card).toHaveClass('sel');
    app.openBoard('p1');
    return expect.poll(() => card.classList.contains('sel')).toBe(false);
  });

  it('tags the agent of a ticket with its loop', () => {
    resetApp({ agents: [agent()], tickets: [ticket({ column: 'doing', agentId: 'a1', iteration: 2 })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(within(card).getByText('▸ DEM-1 · boucle 2/5')).toBeInTheDocument();
  });

  it('tags the agent of a ticket to test, follows the ticket, and leaves the others untagged', async () => {
    resetApp({
      agents: [agent(), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 }), agent({ id: 'a3', name: 'vieux', createdAt: 3 })],
      tickets: [ticket({ column: 'review', agentId: 'a1' }), ticket({ id: 't2', key: 'DEM-2', column: 'done', agentId: 'a3' })],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    const card = (name: RegExp) => screen.getByRole('button', { name });
    expect(within(card(/refacto-auth/)).getByText('▸ DEM-1 · à tester')).toBeInTheDocument();
    // No ticket, or one that is done: nothing to tag.
    expect(card(/tests-e2e/).querySelector('.ticket-tag')).toBeNull();
    expect(card(/vieux/).querySelector('.ticket-tag')).toBeNull();
    app.tickets.t1 = ticket({ column: 'doing', agentId: 'a1', iteration: 3, maxLoops: 8 });
    expect(await within(card(/refacto-auth/)).findByText('▸ DEM-1 · boucle 3/8')).toBeInTheDocument();
  });

  it('switches to the board, with the count of tickets to test, and back to the agents', async () => {
    resetApp({
      agents: [agent()],
      tickets: [
        ticket({ column: 'review', agentId: 'a1' }),
        ticket({ id: 't2', column: 'review' }),
        ticket({ id: 't3', projectId: 'p2', column: 'review' }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    const tab = screen.getByRole('button', { name: /^Kanban/ });
    expect(within(tab).getByText('2')).toBeInTheDocument();
    await userEvent.click(tab);
    expect(app.boardOn).toBe(true);
    expect(tab).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Agents' }));
    expect(app.boardOn).toBe(false);
    app.openBoard('p1');
    await userEvent.click(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(app.boardOn).toBe(false);
  });
});

describe('Sidebar copies of an agent', () => {
  const copyItem = () => menu.open!.items.find((i) => i.label === 'Dupliquer la conversation');

  it('duplicates an agent’s conversation from its menu and shows the copy', async () => {
    resetApp({ agents: [agent()] });
    const backend = fakeBackend({ duplicate_agent: () => agent({ id: 'a4', name: 'refacto-auth (copie)', createdAt: 4 }) });
    app.openEditor({ projectId: 'p1', source: 'project' });
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(copyItem()).toMatchObject({ disabled: false, title: undefined });
    copyItem()!.onClick!();
    await expect.poll(() => app.agent?.id).toBe('a4');
    expect(backend.called('duplicate_agent')).toEqual([{ cmd: 'duplicate_agent', args: { id: 'a1' } }]);
    // Its conversation is what the copy is for: the editor gives way to it.
    expect(app.editorOn).toBe(false);
    expect(await screen.findByRole('button', { name: /refacto-auth \(copie\)/ })).toBeInTheDocument();
  });

  it('offers no copy during a turn, and says why', async () => {
    for (const status of ['running', 'waiting'] as const) {
      resetApp({ agents: [agent({ status })] });
      fakeBackend();
      const { unmount } = render(Sidebar, { project: project() });
      await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
      expect(copyItem()).toMatchObject({ disabled: true, title: 'Attends la fin de son tour.' });
      unmount();
    }
  });

  it('offers no copy of an archived agent', async () => {
    resetApp({ agents: [agent({ archived: true })] });
    fakeBackend();
    app.showArchived = true;
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(copyItem()).toBeUndefined();
  });

  it('tells why a copy was not made', async () => {
    resetApp({ agents: [agent()] });
    fakeBackend({
      duplicate_agent: () => {
        throw 'worktree de la copie non créé';
      },
    });
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    copyItem()!.onClick!();
    await expect.poll(() => app.toasts.map((t) => [t.text, t.kind])).toEqual([['worktree de la copie non créé', 'error']]);
    expect(app.agent?.id).toBe('a1');
  });
});

describe('Sidebar « Vue <harness> » (ESC-34)', () => {
  const menuOf = async (name: RegExp) => {
    await fireEvent.contextMenu(screen.getByRole('button', { name }));
    return menu.open!.items.map((i) => i.label);
  };

  it('names the view after the agent’s harness, and hides it for one that has none', async () => {
    const withView = { ...BARE_ENGINE, name: 'Codex', caps: { ...BARE_ENGINE.caps, terminalView: true } };
    resetApp({ agents: [agent({ engine: 'codex' })] });
    app.engines = [CLAUDE_ENGINE, withView];
    fakeBackend();
    const { unmount } = render(Sidebar, { project: project() });
    let labels = await menuOf(/refacto-auth/);
    expect(labels).toContain('Vue Codex');
    expect(labels).not.toContain('Vue Claude Code');
    unmount();
    resetApp({ agents: [agent({ engine: 'codex' })] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    fakeBackend();
    render(Sidebar, { project: project() });
    labels = await menuOf(/refacto-auth/);
    expect(labels.filter((l) => l.startsWith('Vue '))).toEqual([]);
  });
});

describe('Sidebar « Vue Claude Code » / « Vue Escouade »', () => {
  const VIEWS = ['Vue Claude Code', 'Vue Escouade'];
  const menuOf = async (name: RegExp) => {
    await fireEvent.contextMenu(screen.getByRole('button', { name }));
    return menu.open!.items.map((i) => i.label);
  };
  const click = (label: string) => menu.open!.items.find((i) => i.label === label)!.onClick!();
  // The list of archived agents, opened by one test, is not closed by the reset.
  afterEach(() => (app.showArchived = false));

  it('puts an agent in Escouade’s view into Claude Code’s, as the button of its header', async () => {
    resetApp({ agents: [agent()] });
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    const labels = await menuOf(/refacto-auth/);
    expect(labels).toContain('Vue Claude Code');
    expect(labels).not.toContain('Vue Escouade');
    // By the entries that open its worktree elsewhere.
    expect(labels.indexOf('Vue Claude Code')).toBe(labels.indexOf('Ouvrir un terminal') + 1);
    click('Vue Claude Code');
    expect(backend.called('set_agent_view')).toEqual([{ cmd: 'set_agent_view', args: { id: 'a1', terminal: true } }]);
  });

  it('puts an agent in Claude Code’s view back into Escouade’s', async () => {
    resetApp({ agents: [agent({ terminal: true })] });
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    const labels = await menuOf(/refacto-auth/);
    expect(labels).toContain('Vue Escouade');
    expect(labels).not.toContain('Vue Claude Code');
    click('Vue Escouade');
    expect(backend.called('set_agent_view')).toEqual([{ cmd: 'set_agent_view', args: { id: 'a1', terminal: false } }]);
  });

  it('tells why the backend refused it', async () => {
    resetApp({ agents: [agent({ status: 'running' })] });
    fakeBackend({
      set_agent_view: () => {
        throw 'L’agent travaille encore : change de vue quand son tour est fini.';
      },
    });
    render(Sidebar, { project: project() });
    await menuOf(/refacto-auth/);
    click('Vue Claude Code');
    await expect
      .poll(() => app.toasts.map((t) => [t.text, t.kind]))
      .toEqual([['L’agent travaille encore : change de vue quand son tour est fini.', 'error']]);
  });

  it('is never offered to a ticket’s agent, in either view: its board drives it over the stream', async () => {
    resetApp({
      agents: [agent({ ticketId: 't1' }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, ticketId: 't2', terminal: true })],
      tickets: [ticket({ agentId: 'a1', column: 'doing' }), ticket({ id: 't2', key: 'DEM-2', agentId: 'a2', column: 'review' })],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    for (const name of [/refacto-auth/, /tests-e2e/]) {
      const labels = await menuOf(name);
      expect(labels).toContain('Renommer');
      for (const view of VIEWS) expect(labels).not.toContain(view);
    }
  });

  it('is never offered to an archived agent, in either view', async () => {
    resetApp({
      agents: [agent({ archived: true }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, archived: true, terminal: true })],
    });
    fakeBackend();
    app.showArchived = true;
    render(Sidebar, { project: project() });
    for (const name of [/refacto-auth/, /tests-e2e/]) {
      const labels = await menuOf(name);
      expect(labels).toContain('Restaurer');
      for (const view of VIEWS) expect(labels).not.toContain(view);
    }
  });
});

describe('Sidebar in English', () => {
  it('writes the plan line with the plural of English', () => {
    setLang('en');
    const plan = (subs: number) =>
      ({
        source: 'tools',
        tasks: [
          { id: '1', title: 'A', status: 'done' },
          { id: '2', title: 'B', status: 'pending' },
        ],
        agents: Array.from({ length: subs }, (_, i) => ({
          id: `s${i}`,
          title: 'x',
          status: 'running',
          background: false,
          tools: 0,
          startedAt: 1,
        })),
        launched: subs,
        workflows: [],
      }) as never;
    resetApp({
      agents: [
        agent({ plan: plan(0) }),
        agent({ id: 'a2', name: 'alpha', createdAt: 2, plan: plan(1) }),
        agent({ id: 'a3', name: 'beta', createdAt: 3, plan: plan(2) }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    const line = (name: RegExp) => screen.getByRole('button', { name }).querySelector('.plan-mini');
    expect(line(/refacto-auth/)).toHaveTextContent('Plan 1/2');
    expect(line(/refacto-auth/)).not.toHaveTextContent('subagent');
    expect(line(/alpha/)).toHaveTextContent('Plan 1/2 · 1 subagent');
    expect(line(/alpha/)).not.toHaveTextContent('subagents');
    expect(line(/beta/)).toHaveTextContent('Plan 1/2 · 2 subagents');
  });

  it('labels the line an agent reported in English', () => {
    fakeBackend();
    app.agents.a1 = { ...app.agents.a1, progressLine: 'Writes the parser tests' };
    render(Sidebar, { project: project() });
    expect(screen.getByRole('button', { name: /refacto-auth/ })).toHaveTextContent(
      'What the agent says it is doing Writes the parser tests',
    );
  });

  beforeEach(() => {
    resetApp({
      projects: [project()],
      agents: [
        agent({ status: 'running', tokens: 1500, cost: 0.2 }),
        agent({ id: 'a2', name: 'tests-e2e', status: 'waiting', createdAt: 2 }),
        agent({ id: 'a3', name: 'vieux', archived: true }),
      ],
      tickets: [ticket({ id: 't9', key: 'DEM-9', column: 'review', agentId: 'a2' })],
    });
    setLang('en');
  });
  const items = () => menu.open?.items.map((i) => i.label) ?? [];

  it('names the statuses, the counts and the footer in English', () => {
    fakeBackend();
    app.git.p1 = gitInfo({ modified: 2, added: 1, deleted: 3 });
    app.git.p1.agents = { a1: 1, a2: 4 };
    render(Sidebar, { project: project() });
    const cards = screen.getAllByRole('button', { name: /refacto-auth|tests-e2e/ });
    expect(within(cards[0]).getByText('Running')).toBeInTheDocument();
    expect(within(cards[0]).getByText('1.5k tok')).toBeInTheDocument();
    expect(within(cards[0]).getByText('1 file')).toBeInTheDocument();
    expect(within(cards[1]).getByText('Question')).toBeInTheDocument();
    expect(within(cards[1]).getByText('4 files')).toBeInTheDocument();
    expect(screen.getByText(/Archived \(1\)/)).toBeInTheDocument();
    expect(screen.getByText('Terminals')).toBeInTheDocument();
    expect(screen.getByText('No open terminals')).toBeInTheDocument();
    expect(screen.getByText('~2 modified')).toBeInTheDocument();
    expect(screen.getByText('+1 added')).toBeInTheDocument();
    expect(screen.getByText('−3 deleted')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Browse' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Project view' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /New agent/ })).toHaveAttribute('data-tip', 'New agent (Ctrl+N)');
    expect(getByTip(document.body, '1 to review')).toBeInTheDocument();
  });

  it('says when an agent stopped by the usage limit resumes, and when its worktree is set up', () => {
    fakeBackend();
    app.now = new Date(2026, 8, 30, 12, 0).getTime();
    app.agents.a1 = { ...app.agents.a1, status: 'error', resumeAt: new Date(2026, 8, 30, 15, 0).getTime() };
    app.agents.a2 = { ...app.agents.a2, status: 'idle', setup: '2/3 · npm run gen (web)' };
    render(Sidebar, { project: project() });
    expect(screen.getByText('Resumes at 3:00 PM')).toHaveAttribute(
      'data-tip',
      'Stopped by the usage limit: resumes automatically at 3:00 PM',
    );
    expect(screen.getByText('Setting up…')).toHaveAttribute('data-tip', 'Setting up the worktree: 2/3 · npm run gen (web)');
  });

  it('writes the remote control states and the menu of an agent in English', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a1', branch: 'escouade/a1', baseBranch: 'main' };
    app.agents.a1 = {
      ...app.agents.a1,
      worktree: wt,
      remoteControl: true,
      remoteUrl: 'https://claude.ai/code/session_abc',
      remoteState: 'ready',
    };
    fakeBackend();
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(getByTip(card, 'Remote control: connecting… (reachable from claude.ai and the Claude app)')).toBeInTheDocument();
    await fireEvent.contextMenu(card);
    expect(items()).toEqual([
      'Rename',
      'Duplicate the conversation',
      'Archive',
      'Prepare launch',
      'Integrate main',
      'Open in editor',
      'Open a terminal',
      'Claude Code view',
      '',
      'Turn off remote control',
      'Open on claude.ai',
      'Copy the claude.ai link',
      '',
      'Delete…',
    ]);
    expect(menu.open!.items.find((i) => i.label === 'Archive')).toMatchObject({ hint: 'keeps the conversation' });
  });

  it('writes where the remote control of an agent comes from, and its state in Claude Code’s view, in English', async () => {
    const view = { terminal: true, remoteControl: true } as const;
    resetApp({
      projects: [project()],
      agents: [
        agent({ remoteControl: true, remoteSource: 'hand', remoteState: 'connected' }),
        agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, remoteControl: true, remoteSource: 'project', remoteState: 'connected' }),
        agent({ id: 'a3', name: 'lint', createdAt: 3, remoteControl: true, remoteSource: 'global', remoteState: null }),
        agent({ id: 'a4', name: 'docs', createdAt: 4, ...view, remoteSource: 'global', interfaceRemote: null }),
        agent({ id: 'a5', name: 'relaunch', createdAt: 5, ...view, remoteSource: 'project', interfaceRemote: false }),
        agent({ id: 'a6', name: 'denied', createdAt: 6, ...view, remoteSource: 'hand', interfaceRemote: true, remoteState: 'unavailable' }),
        agent({
          id: 'a7',
          name: 'linked',
          createdAt: 7,
          ...view,
          remoteSource: 'global',
          interfaceRemote: true,
          remoteState: 'connected',
          remoteUrl: 'https://claude.ai/code/session_abc',
        }),
        agent({ id: 'a8', name: 'booting', createdAt: 8, ...view, remoteSource: 'hand', interfaceRemote: true, remoteState: null }),
        agent({ id: 'a9', name: 'sticky', createdAt: 9, terminal: true, remoteOff: true, interfaceRemote: true, remoteState: 'connected' }),
        // Off, its interface started with the remote control but never linked: no marker.
        agent({ id: 'a10', name: 'dormant', createdAt: 10, terminal: true, remoteOff: true, interfaceRemote: true, remoteState: null }),
      ],
    });
    setLang('en');
    fakeBackend({ set_remote_control: () => true });
    render(Sidebar, { project: project() });
    const tipOf = (name: RegExp, text: string) => getByTip(screen.getByRole('button', { name }), text);
    const reach = '(reachable from claude.ai and the Claude app)';
    expect(tipOf(/refacto-auth/, `Remote control: connected ${reach} · turned on by hand`)).toBeInTheDocument();
    expect(tipOf(/tests-e2e/, `Remote control: connected ${reach} · turned on by the project’s setting`)).toBeInTheDocument();
    expect(
      tipOf(/lint/, `Remote control: waiting for a connection ${reach} · turned on by the setting for all agents`),
    ).toBeInTheDocument();
    expect(
      tipOf(
        /docs/,
        'Remote control: no interface running, not reachable for now (open the agent to start it) · turned on by the setting for all agents',
      ),
    ).toBeInTheDocument();
    expect(
      tipOf(/relaunch/, 'Remote control: from the next launch of its Claude Code interface · turned on by the project’s setting'),
    ).toBeInTheDocument();
    expect(
      tipOf(
        /denied/,
        'Remote control: not opened by Claude Code (no claude.ai subscription, network, proxy or organization policy) · turned on by hand',
      ),
    ).toBeInTheDocument();
    expect(tipOf(/linked/, `Remote control: connected ${reach} · turned on by the setting for all agents`)).toHaveClass('on');
    expect(tipOf(/booting/, `Remote control: connecting… ${reach} · turned on by hand`)).toBeInTheDocument();
    expect(
      tipOf(/sticky/, 'Remote control still on in the open interface, until its next launch (its /remote-control command turns it off)'),
    ).toHaveClass('warn');
    expect(screen.getByRole('button', { name: /dormant/ }).querySelector('.rc')).toBeNull();
    // The menu of an agent in Claude Code’s view with a link, and what a change tells.
    await fireEvent.contextMenu(screen.getByRole('button', { name: /linked/ }));
    expect(items()).toEqual(expect.arrayContaining(['Turn off remote control', 'Open on claude.ai', 'Copy the claude.ai link']));
    await fireEvent.contextMenu(screen.getByRole('button', { name: /docs/ }));
    expect(items()).toContain('Turn off remote control');
    expect(items()).not.toContain('Open on claude.ai');
    menu.open!.items.find((i) => i.label === 'Turn off remote control')!.onClick!();
    await expect
      .poll(() => app.toasts.map((x) => [x.text, x.kind]))
      .toEqual([['Applies at the next launch of its Claude Code interface; until then, its /remote-control command does it.', 'ok']]);
    // The marker of a lasting remote control offers to turn it on.
    await fireEvent.contextMenu(screen.getByRole('button', { name: /sticky/ }));
    expect(items()).toContain('Turn on remote control');
    expect(items()).not.toContain('Turn off remote control');
  });

  it('offers to go back to Escouade’s view in English', async () => {
    app.agents.a1 = { ...app.agents.a1, terminal: true };
    fakeBackend();
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(items()).toContain('Escouade view');
    expect(items()).not.toContain('Claude Code view');
  });

  it('asks in English before archiving and deleting', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a2', branch: 'escouade/a2', baseBranch: 'main' };
    app.agents.a2 = { ...app.agents.a2, worktree: wt };
    fakeBackend();
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /tests-e2e/ }));
    menu.open!.items.find((i) => i.label === 'Archive')!.onClick!();
    expect(app.modal).toMatchObject({
      title: 'Archive tests-e2e?',
      body: 'Its ticket DEM-9 goes back to “To do”.',
      confirm: 'Archive',
    });
    app.modal = null;
    menu.open!.items.find((i) => i.label === 'Delete…')!.onClick!();
    expect(app.modal).toMatchObject({
      title: 'Delete the agent “tests-e2e”?',
      confirm: 'Delete',
      option: { label: 'Also delete the worktree and the branch escouade/a2' },
    });
    expect((app.modal as any).body).toBe(
      'The Claude process is stopped and the conversation is removed from the app (the Claude Code session stays on disk).',
    );
  });

  it('tells in English why a copy waits, and when there is no shell', async () => {
    fakeBackend();
    app.shells = [];
    render(Sidebar, { project: project() });
    await fireEvent.contextMenu(screen.getByRole('button', { name: /tests-e2e/ }));
    expect(menu.open!.items.find((i) => i.label === 'Duplicate the conversation')).toMatchObject({
      disabled: true,
      title: 'Wait for its turn to end.',
    });
    await userEvent.click(getByTip(document.body, 'New terminal'));
    expect(app.toasts.at(-1)).toMatchObject({ text: 'No shell found (PowerShell 7, Git Bash, WSL). Check the settings.', kind: 'error' });
  });
});

describe('Sidebar subagents of an agent', () => {
  const sub = (id: string, over: Record<string, unknown> = {}) => ({
    id,
    title: `sous-agent ${id}`,
    status: 'running',
    background: true,
    tools: 0,
    startedAt: 1,
    ...over,
  });
  const withSubs = (agents: unknown[]) => ({ source: null, tasks: [], agents, launched: agents.length, workflows: [] }) as never;
  const group = (name = 'Sous-agents de refacto-auth') => screen.getByRole('group', { name });
  const fold = () => within(group()).getByRole('button', { name: /^Sous-agents/ });

  it('unfolds under the card those running, with their model and what they do, then how many finished', async () => {
    resetApp({
      agents: [
        agent({
          plan: withSubs([
            sub('s1', { title: 'Chercher la cause', model: 'haiku', doing: 'Lit src/a.ts' }),
            sub('s2', { title: 'Relire', model: 'sonnet' }),
            sub('s3', { title: 'Vieux', status: 'done', endedAt: 5 }),
          ]),
        }),
        agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    // Under the card, not in it.
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(card).not.toContainElement(fold());
    expect(fold()).toHaveAttribute('aria-expanded', 'true');
    expect(fold()).toHaveTextContent('Sous-agents 2 en cours 1 fini');
    const list = within(group()).getByRole('list');
    const first = within(list).getByRole('button', { name: /Chercher la cause/ });
    expect(first).toHaveTextContent('Chercher la cause');
    expect(first).toHaveTextContent('Haiku');
    expect(first).toHaveTextContent('Lit src/a.ts');
    expect(within(list).getByRole('button', { name: /Relire/ })).toBeInTheDocument();
    // The finished ones are counted, folded.
    expect(within(list).queryByRole('button', { name: /Vieux/ })).not.toBeInTheDocument();
    const finished = within(list).getByRole('button', { name: '1 sous-agent fini' });
    expect(finished).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(finished);
    expect(finished).toHaveAttribute('aria-expanded', 'true');
    const old = within(list).getByRole('button', { name: /Vieux/ });
    expect(old).toHaveTextContent('Terminé');
    // An agent without subagents has no fold.
    expect(screen.queryByRole('group', { name: 'Sous-agents de tests-e2e' })).not.toBeInTheDocument();
  });

  it('is folded when none runs, and the chevron unfolds it', async () => {
    resetApp({
      agents: [
        agent({
          plan: withSubs([sub('s1', { status: 'stopped', endedAt: 3 }), sub('s2', { status: 'interrupted', endedAt: 4 })]),
        }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(fold()).toHaveAttribute('aria-expanded', 'false');
    expect(fold()).toHaveTextContent('2 finis');
    expect(fold()).not.toHaveTextContent('en cours');
    expect(within(group()).queryByRole('list')).not.toBeInTheDocument();
    await userEvent.click(fold());
    expect(fold()).toHaveAttribute('aria-expanded', 'true');
    await userEvent.click(screen.getByRole('button', { name: '2 sous-agents finis' }));
    expect(screen.getByRole('button', { name: /sous-agent s1/ })).toHaveTextContent('Arrêté');
    expect(screen.getByRole('button', { name: /sous-agent s2/ })).toHaveTextContent('Interrompu');
    // Folded again by the reader, it stays so.
    await userEvent.click(fold());
    expect(fold()).toHaveAttribute('aria-expanded', 'false');
  });

  it('opens a subagent’s conversation in place of the agent’s, and says which is open', async () => {
    resetApp({
      agents: [agent({ plan: withSubs([sub('s1', { title: 'Chercher' })]) }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 })],
    });
    app.ui.selectedAgent = { p1: 'a2' };
    fakeBackend();
    render(Sidebar, { project: project() });
    const row = screen.getByRole('button', { name: /Chercher/ });
    expect(row).not.toHaveAttribute('aria-current');
    await userEvent.click(row);
    expect(app.ui.selectedAgent.p1).toBe('a1');
    expect(app.subView.a1).toBe('s1');
    expect(row).toHaveAttribute('aria-current', 'page');
    // The card of the agent goes back to its own conversation.
    await userEvent.click(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(app.subView.a1).toBeNull();
    expect(row).not.toHaveAttribute('aria-current');
  });

  it('indents a subagent’s subagent', () => {
    resetApp({ agents: [agent({ plan: withSubs([sub('s1'), sub('s2', { parent: 's1' })]) })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(screen.getByRole('button', { name: /sous-agent s1/ }).style.getPropertyValue('--depth')).toBe('0');
    expect(screen.getByRole('button', { name: /sous-agent s2/ }).style.getPropertyValue('--depth')).toBe('1');
  });

  it('speaks English', async () => {
    setLang('en');
    resetApp({ agents: [agent({ plan: withSubs([sub('s1'), sub('s2', { status: 'failed', endedAt: 2 })]) })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    const toggle = within(screen.getByRole('group', { name: 'Subagents of refacto-auth' })).getByRole('button', { name: /^Subagents/ });
    expect(toggle).toHaveTextContent('Subagents 1 running 1 finished');
    await userEvent.click(screen.getByRole('button', { name: '1 finished subagent' }));
    expect(screen.getByRole('button', { name: /sous-agent s2/ })).toHaveTextContent('Failed');
  });

  describe('when there are many', () => {
    const many = (count: number, over: Record<string, unknown> = {}, from = 1) =>
      Array.from({ length: count }, (_, i) => sub(`s${from + i}`, { ...over, ...(over.status ? { endedAt: 1000 - i } : {}) }));
    /** The rows of subagents (not the buttons that fold and unroll). */
    const subRows = () => [...group().querySelectorAll<HTMLElement>('.sub')];
    const unrollFinished = () => userEvent.click(screen.getByRole('button', { name: /sous-agents? finis?$/ }));

    it('shows five of those running, then a button for the others that unrolls them', async () => {
      resetApp({ agents: [agent({ plan: withSubs(many(12)) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      expect(fold()).toHaveTextContent('12 en cours');
      expect(subRows().map((r) => r.textContent?.trim())).toEqual(['s1', 's2', 's3', 's4', 's5'].map((s) => `sous-agent ${s}`));
      const more = within(group()).getByRole('button', { name: '7 autres en cours' });
      expect(more).toHaveAttribute('aria-expanded', 'false');
      // The button comes after the rows it hides.
      expect(more.compareDocumentPosition(subRows()[4]) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
      await userEvent.click(more);
      expect(subRows()).toHaveLength(12);
      expect(subRows()[11]).toHaveTextContent('sous-agent s12');
      // The same button, still there to fold them back: the focus does not go with it.
      expect(more).toBeInTheDocument();
      expect(more).toHaveAttribute('aria-expanded', 'true');
      expect(more).toHaveTextContent('Réduire');
      expect(more).toHaveFocus();
      await userEvent.click(more);
      expect(subRows()).toHaveLength(5);
      expect(more).toHaveTextContent('7 autres en cours');
    });

    it('has no button for five running, and says one other in the singular for six', () => {
      resetApp({ agents: [agent({ plan: withSubs(many(5)) })] });
      fakeBackend();
      const { unmount } = render(Sidebar, { project: project() });
      expect(subRows()).toHaveLength(5);
      expect(within(group()).queryByRole('button', { name: /autres? en cours/ })).not.toBeInTheDocument();
      unmount();
      resetApp({ agents: [agent({ plan: withSubs(many(6)) })] });
      render(Sidebar, { project: project() });
      expect(subRows()).toHaveLength(5);
      expect(within(group()).getByRole('button', { name: '1 autre en cours' })).toBeInTheDocument();
    });

    it('shows ten of those finished once unfolded, then a button for the rest that unrolls them', async () => {
      resetApp({ agents: [agent({ plan: withSubs(many(15, { status: 'done' })) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      await userEvent.click(fold());
      await unrollFinished();
      // The latest ended first.
      expect(subRows()).toHaveLength(10);
      expect(subRows()[0]).toHaveTextContent('sous-agent s1');
      const more = within(group()).getByRole('button', { name: '5 de plus' });
      expect(more).toHaveAttribute('aria-expanded', 'false');
      await userEvent.click(more);
      expect(subRows()).toHaveLength(15);
      expect(more).toHaveAttribute('aria-expanded', 'true');
      await userEvent.click(more);
      expect(subRows()).toHaveLength(10);
    });

    it('has no button for ten finished', async () => {
      resetApp({ agents: [agent({ plan: withSubs(many(10, { status: 'done' })) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      await userEvent.click(fold());
      await unrollFinished();
      expect(subRows()).toHaveLength(10);
      expect(within(group()).queryByRole('button', { name: /de plus/ })).not.toBeInTheDocument();
    });

    it('caps the running and the finished lists on their own', async () => {
      resetApp({ agents: [agent({ plan: withSubs([...many(8), ...many(12, { status: 'done' }, 100)]) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      await unrollFinished();
      // Five running and ten finished, each list with its button.
      expect(subRows()).toHaveLength(15);
      expect(within(group()).getByRole('button', { name: '3 autres en cours' })).toBeInTheDocument();
      expect(within(group()).getByRole('button', { name: '2 de plus' })).toBeInTheDocument();
      await userEvent.click(within(group()).getByRole('button', { name: '2 de plus' }));
      expect(subRows()).toHaveLength(17);
    });

    it('unrolls the list of one agent, not of the others', async () => {
      resetApp({
        agents: [agent({ plan: withSubs(many(8)) }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, plan: withSubs(many(8, {}, 50)) })],
      });
      fakeBackend();
      render(Sidebar, { project: project() });
      await userEvent.click(within(group()).getByRole('button', { name: '3 autres en cours' }));
      expect(subRows()).toHaveLength(8);
      const other = screen.getByRole('group', { name: 'Sous-agents de tests-e2e' });
      expect(other.querySelectorAll('.sub')).toHaveLength(5);
      expect(within(other).getByRole('button', { name: '3 autres en cours' })).toHaveAttribute('aria-expanded', 'false');
    });

    it('speaks English', async () => {
      setLang('en');
      resetApp({ agents: [agent({ plan: withSubs([...many(7), ...many(11, { status: 'done' }, 100)]) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      const more = within(group('Subagents of refacto-auth')).getByRole('button', { name: '2 more running' });
      await userEvent.click(more);
      expect(more).toHaveTextContent('Show fewer');
      await userEvent.click(screen.getByRole('button', { name: '11 finished subagents' }));
      expect(within(group('Subagents of refacto-auth')).getByRole('button', { name: '1 more' })).toBeInTheDocument();
    });
  });
});

describe('Sidebar background tasks of an agent', () => {
  const bg = (id: string, over: Record<string, unknown> = {}) => ({
    id,
    kind: 'command',
    title: `tâche ${id}`,
    status: 'running',
    startedAt: 1_000,
    ...over,
  });
  const flow = (id: string, over: Record<string, unknown> = {}) => ({
    id,
    title: `workflow ${id}`,
    status: 'running',
    tools: 0,
    tokens: 0,
    startedAt: 1_000,
    ...over,
  });
  const sub = (id: string) => ({ id, title: `sous-agent ${id}`, status: 'running', background: true, tools: 0, startedAt: 1 });
  const withBg = (bgTasks: unknown[], workflows: unknown[] = [], agents: unknown[] = []) =>
    ({ source: null, tasks: [], agents, launched: agents.length, workflows, bgTasks }) as never;
  const group = (name = 'Tâches de fond de refacto-auth') => screen.getByRole('group', { name });
  const fold = () => within(group()).getByRole('button', { name: /^Tâches de fond/ });
  /** The rows of tasks (not the buttons that fold and unroll). */
  const bgRows = () => [...group().querySelectorAll<HTMLElement>('[data-bg-row]')];

  it('unfolds under the card those running, with what they are, how long they run and what they do, then how many finished', async () => {
    resetApp({
      agents: [
        agent({
          plan: withBg(
            [
              bg('b1', { title: 'Ticker', command: 'node ticker.js', startedAt: 1_000 }),
              bg('b2', { kind: 'monitor', title: 'Watch', startedAt: 20_000 }),
              bg('b3', { title: 'Vieux', status: 'done', endedAt: 5_000, summary: 'Background command "Vieux" completed (exit code 0)' }),
            ],
            [flow('w1', { now: 'Phase 1 : lecteur A', startedAt: 30_000 })],
          ),
        }),
        agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 }),
      ],
    });
    app.now = 61_000;
    fakeBackend();
    render(Sidebar, { project: project() });
    // Under the card, not in it.
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    expect(card).not.toContainElement(fold());
    expect(fold()).toHaveAttribute('aria-expanded', 'true');
    expect(fold()).toHaveTextContent('Tâches de fond 3 en cours 1 finie');
    const list = within(group()).getByRole('list');
    const ticker = within(list).getByRole('button', { name: /Ticker/ });
    expect(ticker).toHaveTextContent('Ticker');
    expect(ticker).toHaveTextContent('1m 00s');
    expect(ticker).toHaveTextContent('Commande en fond · node ticker.js');
    expect(within(list).getByRole('button', { name: /Watch/ })).toHaveTextContent('Surveillance');
    expect(within(list).getByRole('button', { name: /workflow w1/ })).toHaveTextContent('Workflow · Phase 1 : lecteur A');
    // In the order they were launched in.
    expect(bgRows().map((r) => r.dataset.bgRow)).toEqual(['b1', 'b2', 'w1']);
    // The finished ones are counted, folded.
    expect(within(list).queryByRole('button', { name: /Vieux/ })).not.toBeInTheDocument();
    const finished = within(list).getByRole('button', { name: '1 tâche finie' });
    expect(finished).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(finished);
    expect(finished).toHaveAttribute('aria-expanded', 'true');
    expect(within(list).getByRole('button', { name: /Vieux/ })).toHaveTextContent('Commande en fond · Terminée (code 0)');
    // An agent without a task has no fold.
    expect(screen.queryByRole('group', { name: 'Tâches de fond de tests-e2e' })).not.toBeInTheDocument();
  });

  it('counts the duration of those running live, and stops at the end of those finished', () => {
    resetApp({ agents: [agent({ plan: withBg([bg('b1'), bg('b2', { status: 'done', startedAt: 1_000, endedAt: 31_000 })]) })] });
    app.now = 61_000;
    fakeBackend();
    render(Sidebar, { project: project() });
    const row = screen.getByRole('button', { name: /tâche b1/ });
    expect(row).toHaveTextContent('1m 00s');
    app.now = 121_000;
    return waitFor(() => expect(row).toHaveTextContent('2m 00s'));
  });

  it('tells a failure apart by its color, and a stop or an interruption', async () => {
    resetApp({
      agents: [
        agent({
          plan: withBg([
            bg('b1', { status: 'failed', endedAt: 5 }),
            bg('b2', { status: 'stopped', endedAt: 4 }),
            bg('b3', { status: 'interrupted', endedAt: 3 }),
            bg('b4', { status: 'done', endedAt: 2 }),
          ]),
        }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    await userEvent.click(fold());
    await userEvent.click(screen.getByRole('button', { name: '4 tâches finies' }));
    expect(bgRows().map((r) => [r.dataset.bgRow, r.querySelector('.sdot')?.className])).toEqual([
      ['b1', expect.stringContaining('failed')],
      ['b2', expect.stringContaining('stopped')],
      ['b3', expect.stringContaining('interrupted')],
      ['b4', expect.stringContaining('done')],
    ]);
    expect(screen.getByRole('button', { name: /tâche b1/ })).toHaveTextContent('En échec');
  });

  it('is folded when none runs, and the chevron unfolds it', async () => {
    resetApp({
      agents: [agent({ plan: withBg([bg('b1', { status: 'stopped', endedAt: 3 }), bg('b2', { status: 'done', endedAt: 4 })]) })],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(fold()).toHaveAttribute('aria-expanded', 'false');
    expect(fold()).toHaveTextContent('2 finies');
    expect(fold()).not.toHaveTextContent('en cours');
    expect(within(group()).queryByRole('list')).not.toBeInTheDocument();
    await userEvent.click(fold());
    expect(fold()).toHaveAttribute('aria-expanded', 'true');
    expect(app.bgOpen.a1).toBe(true);
    await userEvent.click(screen.getByRole('button', { name: '2 tâches finies' }));
    expect(screen.getByRole('button', { name: /tâche b1/ })).toHaveTextContent('Arrêtée');
    expect(screen.getByRole('button', { name: /tâche b2/ })).toHaveTextContent('Terminée');
    // Folded again by the reader, it stays so.
    await userEvent.click(fold());
    expect(fold()).toHaveAttribute('aria-expanded', 'false');
  });

  it('unfolds by itself when a task starts, unless the reader folded the list', async () => {
    resetApp({ agents: [agent({ plan: withBg([bg('b1', { status: 'done', endedAt: 3 })]) })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(fold()).toHaveAttribute('aria-expanded', 'false');
    app.agents.a1.plan = withBg([bg('b1', { status: 'done', endedAt: 3 }), bg('b2')]);
    await waitFor(() => expect(fold()).toHaveAttribute('aria-expanded', 'true'));
    await userEvent.click(fold());
    app.agents.a1.plan = withBg([bg('b1', { status: 'done', endedAt: 3 }), bg('b2'), bg('b3')]);
    await Promise.resolve();
    expect(fold()).toHaveAttribute('aria-expanded', 'false');
  });

  it('opens a task’s view in place of the agent’s conversation, and says which is open', async () => {
    resetApp({ agents: [agent({ plan: withBg([bg('b1', { title: 'Ticker' })]) }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 })] });
    app.ui.selectedAgent = { p1: 'a2' };
    fakeBackend();
    render(Sidebar, { project: project() });
    const row = screen.getByRole('button', { name: /Ticker/ });
    expect(row).not.toHaveAttribute('aria-current');
    await userEvent.click(row);
    expect(app.ui.selectedAgent.p1).toBe('a1');
    expect(app.taskView.a1).toBe('b1');
    expect(row).toHaveAttribute('aria-current', 'page');
    // The card of the agent goes back to its own conversation.
    await userEvent.click(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(app.taskView.a1).toBeNull();
    expect(row).not.toHaveAttribute('aria-current');
  });

  it('opens a workflow the same way, by its id', async () => {
    resetApp({ agents: [agent({ plan: withBg([], [flow('w1')]) })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    await userEvent.click(screen.getByRole('button', { name: /workflow w1/ }));
    expect(app.taskView.a1).toBe('w1');
  });

  it('leaves the subagents’ own list alone, and the two views exclude each other', async () => {
    resetApp({ agents: [agent({ plan: withBg([bg('b1', { title: 'Ticker' })], [], [sub('s1')]) })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    const subs = screen.getByRole('group', { name: 'Sous-agents de refacto-auth' });
    expect(within(subs).getByRole('button', { name: /sous-agent s1/ })).toBeInTheDocument();
    expect(within(subs).queryByRole('button', { name: /Ticker/ })).not.toBeInTheDocument();
    expect(within(group()).queryByRole('button', { name: /sous-agent s1/ })).not.toBeInTheDocument();
    await userEvent.click(within(group()).getByRole('button', { name: /Ticker/ }));
    expect(app.taskView.a1).toBe('b1');
    await userEvent.click(within(subs).getByRole('button', { name: /sous-agent s1/ }));
    expect(app.subView.a1).toBe('s1');
    expect(app.taskView.a1).toBeNull();
    expect(within(group()).getByRole('button', { name: /Ticker/ })).not.toHaveAttribute('aria-current');
  });

  it('comes after the subagents under the card', () => {
    resetApp({ agents: [agent({ plan: withBg([bg('b1')], [], [sub('s1')]) })] });
    fakeBackend();
    render(Sidebar, { project: project() });
    const subs = screen.getByRole('group', { name: 'Sous-agents de refacto-auth' });
    expect(subs.compareDocumentPosition(group()) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('speaks English', async () => {
    setLang('en');
    resetApp({
      agents: [agent({ plan: withBg([bg('b1', { kind: 'monitor', title: 'Watch' }), bg('b2', { status: 'failed', endedAt: 2 })]) })],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    const toggle = within(screen.getByRole('group', { name: 'Background tasks of refacto-auth' })).getByRole('button', {
      name: /^Background tasks/,
    });
    expect(toggle).toHaveTextContent('Background tasks 1 running 1 finished');
    expect(screen.getByRole('button', { name: /Watch/ })).toHaveTextContent('Monitor');
    await userEvent.click(screen.getByRole('button', { name: '1 finished task' }));
    expect(screen.getByRole('button', { name: /tâche b2/ })).toHaveTextContent('Background command · Failed');
  });

  describe('on the agent’s card', () => {
    const chip = (card: HTMLElement) => card.querySelector<HTMLElement>('[data-bg-chip]');
    const cardOf = (name: RegExp) => screen.getByRole('button', { name });

    it('says how many run in the background, in full for a screen reader', () => {
      resetApp({
        agents: [agent({ plan: withBg([bg('b1'), bg('b2', { kind: 'monitor' })]) }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 })],
      });
      fakeBackend();
      render(Sidebar, { project: project() });
      const card = cardOf(/refacto-auth/);
      const c = within(card).getByRole('img', { name: '2 tâches de fond en cours' });
      expect(c).toBe(chip(card));
      expect(c).toHaveTextContent('2 en fond');
      expect(c).toHaveAttribute('data-count', '2');
      // The card is the same button, and says it.
      expect(card).toHaveAccessibleName(/2 tâches de fond en cours/);
      // Another agent, none.
      expect(chip(cardOf(/tests-e2e/))).toBeNull();
    });

    it('says one in the singular, and nothing at zero', async () => {
      resetApp({ agents: [agent({ plan: withBg([bg('b1')]) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      const card = cardOf(/refacto-auth/);
      expect(within(card).getByRole('img', { name: '1 tâche de fond en cours' })).toHaveTextContent('1 en fond');
      app.agents.a1.plan = withBg([bg('b1', { status: 'done', endedAt: 5 })]);
      await waitFor(() => expect(chip(card)).toBeNull());
    });

    it('counts the workflows running, not the subagents, nor those finished', () => {
      resetApp({
        agents: [
          agent({
            plan: withBg(
              [bg('b1'), bg('b2', { status: 'failed', endedAt: 2 })],
              [flow('w1'), flow('w2', { status: 'done', endedAt: 3 })],
              [sub('s1'), sub('s2')],
            ),
          }),
        ],
      });
      fakeBackend();
      render(Sidebar, { project: project() });
      expect(chip(cardOf(/refacto-auth/))).toHaveTextContent('2 en fond');
    });

    it('is not there for an agent with subagents only', () => {
      resetApp({ agents: [agent({ plan: withBg([], [], [sub('s1')]) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      expect(chip(cardOf(/refacto-auth/))).toBeNull();
      expect(screen.queryByRole('group', { name: 'Tâches de fond de refacto-auth' })).not.toBeInTheDocument();
    });

    it('has its full text for a tooltip, and speaks English', () => {
      setLang('en');
      resetApp({ agents: [agent({ plan: withBg([bg('b1'), bg('b2')]) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      const c = chip(cardOf(/refacto-auth/))!;
      expect(c).toHaveTextContent('2 in background');
      expect(c).toHaveAccessibleName('2 background tasks running');
      expect(c).toHaveAttribute('data-tip', '2 background tasks running');
    });
  });

  describe('when there are many', () => {
    const many = (count: number, over: Record<string, unknown> = {}, from = 1) =>
      Array.from({ length: count }, (_, i) =>
        bg(`b${from + i}`, { ...over, startedAt: 1_000 + from + i, ...(over.status ? { endedAt: 100_000 - i } : {}) }),
      );
    const unrollFinished = () => userEvent.click(screen.getByRole('button', { name: /tâches? finies?$/ }));

    it('shows five of those running, then a button for the others that unrolls them', async () => {
      resetApp({ agents: [agent({ plan: withBg(many(12)) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      expect(fold()).toHaveTextContent('12 en cours');
      expect(bgRows().map((r) => r.dataset.bgRow)).toEqual(['b1', 'b2', 'b3', 'b4', 'b5']);
      const more = within(group()).getByRole('button', { name: '7 autres en cours' });
      expect(more).toHaveAttribute('aria-expanded', 'false');
      // The button comes after the rows it hides.
      expect(more.compareDocumentPosition(bgRows()[4]) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
      await userEvent.click(more);
      expect(bgRows()).toHaveLength(12);
      expect(more).toHaveAttribute('aria-expanded', 'true');
      expect(more).toHaveTextContent('Réduire');
      expect(more).toHaveFocus();
      await userEvent.click(more);
      expect(bgRows()).toHaveLength(5);
      expect(more).toHaveTextContent('7 autres en cours');
    });

    it('has no button for five running, and says one other in the singular for six', () => {
      resetApp({ agents: [agent({ plan: withBg(many(5)) })] });
      fakeBackend();
      const { unmount } = render(Sidebar, { project: project() });
      expect(bgRows()).toHaveLength(5);
      expect(within(group()).queryByRole('button', { name: /autres? en cours/ })).not.toBeInTheDocument();
      unmount();
      resetApp({ agents: [agent({ plan: withBg(many(6)) })] });
      render(Sidebar, { project: project() });
      expect(bgRows()).toHaveLength(5);
      expect(within(group()).getByRole('button', { name: '1 autre en cours' })).toBeInTheDocument();
    });

    it('shows ten of those finished once unfolded, then a button for the rest that unrolls them', async () => {
      resetApp({ agents: [agent({ plan: withBg(many(15, { status: 'done' })) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      await userEvent.click(fold());
      await unrollFinished();
      // The latest ended first.
      expect(bgRows()).toHaveLength(10);
      expect(bgRows()[0].dataset.bgRow).toBe('b1');
      const more = within(group()).getByRole('button', { name: '5 de plus' });
      expect(more).toHaveAttribute('aria-expanded', 'false');
      await userEvent.click(more);
      expect(bgRows()).toHaveLength(15);
      expect(more).toHaveAttribute('aria-expanded', 'true');
      await userEvent.click(more);
      expect(bgRows()).toHaveLength(10);
    });

    it('caps the running and the finished lists on their own, and the list of one agent only', async () => {
      resetApp({
        agents: [
          agent({ plan: withBg([...many(8), ...many(12, { status: 'done' }, 100)]) }),
          agent({ id: 'a2', name: 'tests-e2e', createdAt: 2, plan: withBg(many(8, {}, 50)) }),
        ],
      });
      fakeBackend();
      render(Sidebar, { project: project() });
      await unrollFinished();
      expect(bgRows()).toHaveLength(15);
      expect(within(group()).getByRole('button', { name: '3 autres en cours' })).toBeInTheDocument();
      expect(within(group()).getByRole('button', { name: '2 de plus' })).toBeInTheDocument();
      await userEvent.click(within(group()).getByRole('button', { name: '2 de plus' }));
      expect(bgRows()).toHaveLength(17);
      const other = screen.getByRole('group', { name: 'Tâches de fond de tests-e2e' });
      expect(other.querySelectorAll('[data-bg-row]')).toHaveLength(5);
      expect(within(other).getByRole('button', { name: '3 autres en cours' })).toHaveAttribute('aria-expanded', 'false');
    });

    it('speaks English', async () => {
      setLang('en');
      resetApp({ agents: [agent({ plan: withBg([...many(7), ...many(11, { status: 'done' }, 100)]) })] });
      fakeBackend();
      render(Sidebar, { project: project() });
      const more = within(group('Background tasks of refacto-auth')).getByRole('button', { name: '2 more running' });
      await userEvent.click(more);
      expect(more).toHaveTextContent('Show fewer');
      await userEvent.click(screen.getByRole('button', { name: '11 finished tasks' }));
      expect(within(group('Background tasks of refacto-auth')).getByRole('button', { name: '1 more' })).toBeInTheDocument();
    });
  });
});

describe('Sidebar of agents on harnesses that do not all do the same', () => {
  const items = () => menu.open?.items.map((i) => i.label) ?? [];

  it('hides, not greys out, what the harness of an agent cannot do', async () => {
    const url = 'https://claude.ai/code/session_abc';
    const on = { remoteControl: true, remoteUrl: url, remoteState: 'connected' as const, cost: 1.25 };
    resetApp({
      agents: [agent({ ...on }), agent({ ...on, id: 'a2', name: 'sur-codex', engine: 'codex', createdAt: 2 })],
    });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    fakeBackend();
    render(Sidebar, { project: project() });
    // Claude Code's agent: copy, terminal view, remote control and cost.
    const claude = screen.getByRole('button', { name: /refacto-auth/ });
    expect(getByTip(claude, /Remote control : connecté/)).toBeInTheDocument();
    expect(claude).toHaveTextContent('1,25 $');
    await fireEvent.contextMenu(claude);
    expect(items()).toEqual(
      expect.arrayContaining(['Dupliquer la conversation', 'Vue Claude Code', 'Désactiver le remote control', 'Ouvrir sur claude.ai']),
    );
    // The other one: none of them, not even disabled.
    const other = screen.getByRole('button', { name: /sur-codex/ });
    expect(other.querySelector('.rc')).toBeNull();
    expect(other).not.toHaveTextContent('1,25 $');
    await fireEvent.contextMenu(other);
    for (const label of [
      'Dupliquer la conversation',
      'Vue Claude Code',
      'Désactiver le remote control',
      'Activer le remote control',
      'Ouvrir sur claude.ai',
    ]) {
      expect(items()).not.toContain(label);
    }
    expect(items()).toContain('Renommer');
  });
});

describe('Sidebar: choosing the harness and its brand (ESC-29)', () => {
  const more = () => screen.getByRole('button', { name: 'Autres façons de créer un agent' });

  it('offers « Nouvel agent sur … » for each harness found, from two on, the project’s first', async () => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    const backend = fakeBackend({ create_agent: (a: any) => agent({ id: 'a7', engine: a.engine, createdAt: 7 }) });
    render(Sidebar, { project: project({ engine: 'codex' }) });
    // One harness found: as before.
    await userEvent.click(more());
    expect(menu.open!.items.map((i) => i.label)).toEqual([
      'Nouvel agent sur une branche…',
      'Nouvel agent dans un worktree',
      'Nouvel agent depuis une autre branche…',
    ]);
    menu.close();
    app.enginesFound = ['claude', 'codex'];
    await userEvent.click(more());
    expect(menu.open!.items.map((i) => i.label)).toEqual([
      'Nouvel agent sur Harness nu',
      'Nouvel agent sur Claude Code',
      '',
      'Nouvel agent sur une branche…',
      'Nouvel agent dans un worktree',
      'Nouvel agent depuis une autre branche…',
    ]);
    menu.open!.items[1].onClick!();
    menu.close();
    await waitFor(() => expect(backend.called('create_agent')).toHaveLength(1));
    expect(backend.called('create_agent')[0].args).toEqual({ projectId: 'p1', model: null, engine: 'claude', isolated: null, base: null });
  });

  it('shows the brand of each agent’s harness only once agents run on two', () => {
    resetApp({ agents: [agent({ id: 'a1', name: 'alpha' }), agent({ id: 'a2', name: 'beta', createdAt: 2 })] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    fakeBackend();
    const { unmount } = render(Sidebar, { project: project() });
    expect(screen.queryByRole('img', { name: /^Harness :/ })).toBeNull();
    unmount();
    resetApp({ agents: [agent({ id: 'a1', name: 'alpha' }), agent({ id: 'a2', name: 'beta', engine: 'codex', createdAt: 2 })] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    render(Sidebar, { project: project() });
    expect(within(screen.getByRole('button', { name: /alpha/ })).getByRole('img', { name: 'Harness : Claude Code' })).toHaveTextContent(
      'Claude',
    );
    expect(within(screen.getByRole('button', { name: /beta/ })).getByRole('img', { name: 'Harness : Harness nu' })).toHaveTextContent(
      'Harness',
    );
  });
});

describe('Sidebar: what goes on after the turn of an agent (ESC-138)', () => {
  const planOf = (over: Record<string, unknown>) =>
    ({ source: null, tasks: [], agents: [], launched: 0, workflows: [], bgTasks: [], ...over }) as never;
  const cardOf = (name: RegExp) => screen.getByRole('button', { name });
  const pending = (card: HTMLElement) => card.querySelector<HTMLElement>('[data-pending]');

  it('says « Attend » instead of « Terminé » while a background task runs, and « Terminé » once it ended', async () => {
    resetApp({
      agents: [
        agent({
          plan: planOf({
            bgTasks: [{ id: 'b1', kind: 'command', title: 'Wait until PR 148 checks finish', status: 'running', startedAt: 1 }],
          }),
        }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    const card = cardOf(/refacto-auth/);
    expect(pending(card)).toHaveTextContent('Attend');
    expect(within(card).queryByText('Terminé')).not.toBeInTheDocument();
    expect(pending(card)).toHaveAttribute('data-tip', 'Son tour est fini, mais il attend :\n1 tâche de fond en cours');
    app.agents.a1.plan = planOf({ bgTasks: [{ id: 'b1', kind: 'command', title: 'x', status: 'done', startedAt: 1, endedAt: 2 }] });
    await waitFor(() => expect(pending(card)).toBeNull());
    expect(within(card).getByText('Terminé')).toBeInTheDocument();
  });

  it('says when Claude Code wakes it up', () => {
    app.now = Date.now();
    resetApp({
      agents: [
        agent({ plan: planOf({ wakes: [{ id: 'loop', kind: 'loop', at: Date.now() + 3_600_000, recurring: false, durable: false }] }) }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(pending(cardOf(/refacto-auth/))).toHaveTextContent(/^Réveil /);
  });

  it('lists under the card the agents it drives while they work, and opens one', async () => {
    resetApp({
      agents: [
        agent({
          id: 'chef',
          name: 'deploiement-socle',
          plan: planOf({
            driven: [
              { agent: 'w1', at: 1 },
              { ticket: 'DEM-1', at: 2 },
            ],
          }),
        }),
        agent({ id: 'w1', name: 'pr-149', status: 'running', createdAt: 2 }),
        agent({ id: 'w2', name: 'pr-148', status: 'waiting', createdAt: 3 }),
      ],
      tickets: [ticket({ agentId: 'w2' })],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(pending(cardOf(/deploiement-socle/))).toHaveTextContent('Attend');
    const group = screen.getByRole('group', { name: 'Agents pilotés par deploiement-socle' });
    expect(within(group).getByRole('button', { name: /^Agents pilotés/ })).toHaveTextContent('Agents pilotés 2 au travail');
    // The latest driven first.
    const rows = within(group).getAllByRole('button', { name: /^Ouvrir/ });
    expect(rows.map((r) => r.getAttribute('aria-label'))).toEqual(['Ouvrir pr-148', 'Ouvrir pr-149']);
    await userEvent.click(rows[1]);
    expect(app.ui.selectedAgent.p1).toBe('w1');
    // They are done: the list goes, and so does « Attend ».
    app.agents.w1.status = 'done';
    app.agents.w2.status = 'done';
    await waitFor(() => expect(screen.queryByRole('group', { name: 'Agents pilotés par deploiement-socle' })).not.toBeInTheDocument());
    expect(pending(cardOf(/deploiement-socle/))).toBeNull();
  });
});

describe('Sidebar — an agent’s pull requests (ESC-64)', () => {
  const WT = { path: 'C:\wt\login', branch: 'escouade/login', baseBranch: 'main', existing: false };
  afterEach(() => setLang('fr'));

  it('shows its pull requests under its card, and hands the corrections to the agent on a click only', async () => {
    const prs = [
      pullRequest({ number: 12, checks: 'failure', changesRequested: true, openThreads: 2 }),
      pullRequest({ number: 9, state: 'merged' }),
      pullRequest({ number: 5, state: 'closed', checks: 'failure' }),
      pullRequest({ number: 3, state: 'closed' }),
    ];
    resetApp({ projects: [project()], agents: [agent({ worktree: WT, prs, prWord: 'PR' })] });
    const backend = fakeBackend();
    render(Sidebar, { project: project() });
    const group = within(screen.getByRole('group', { name: 'Pull requests de refacto-auth' }));
    expect(group.getAllByRole('button', { name: /^Ouvrir la PR/ }).map((b) => b.getAttribute('aria-label'))).toEqual([
      'Ouvrir la PR #12 dans le navigateur (ouverte)',
      'Ouvrir la PR #9 dans le navigateur (mergée)',
      'Ouvrir la PR #5 dans le navigateur (fermée)',
    ]);
    const tipped = getByTip(
      screen.getByRole('group', { name: 'Pull requests de refacto-auth' }),
      'PR #12 · Connexion par jeton\nouverte · CI en échec · changements demandés · 2 fils ouverts',
    );
    expect(tipped).toBe(group.getByRole('button', { name: /PR #12/ }));
    // Only the open one has something to correct; nothing was sent by itself.
    expect(group.getAllByRole('button', { name: 'L’agent corrige' })).toHaveLength(1);
    expect(group.getAllByRole('button', { name: 'L’agent corrige les retours' })).toHaveLength(1);
    expect(backend.called('agent_pr_fix')).toHaveLength(0);
    await userEvent.click(group.getByRole('button', { name: 'L’agent corrige' }));
    await userEvent.click(group.getByRole('button', { name: 'L’agent corrige les retours' }));
    expect(backend.called('agent_pr_fix').map((c) => c.args)).toEqual([
      { id: 'a1', number: 12, fix: 'checks' },
      { id: 'a1', number: 12, fix: 'reviews' },
    ]);
    // The card's menu goes to the open one.
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    expect(menu.open!.items.map((i) => i.label)).toContain('Voir la PR #12');
  });

  it('offers « Ouvrir une PR… » in the card’s menu of an agent with a branch, and shows nothing without a pull request', async () => {
    resetApp({
      projects: [project()],
      agents: [
        agent({ worktree: WT, prWord: 'PR' }),
        agent({ id: 'a2', name: 'sans-branche', createdAt: 2 }),
        agent({ id: 'a3', name: 'sans-hebergeur', worktree: WT, createdAt: 3 }),
      ],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    expect(screen.queryByRole('group', { name: /Pull requests/ })).not.toBeInTheDocument();
    await fireEvent.contextMenu(screen.getByRole('button', { name: /refacto-auth/ }));
    const item = menu.open!.items.find((i) => i.label === 'Ouvrir une PR…')!;
    item.onClick!();
    await waitFor(() => expect(app.modal).toEqual({ kind: 'pullRequest', agentId: 'a1' }));
    menu.close();
    await fireEvent.contextMenu(screen.getByRole('button', { name: /sans-branche/ }));
    expect(menu.open!.items.map((i) => i.label)).not.toContain('Ouvrir une PR…');
    menu.close();
    // A branch, but no code host that opens pull requests (no remote, GitLab for now, no account).
    await fireEvent.contextMenu(screen.getByRole('button', { name: /sans-hebergeur/ }));
    expect(menu.open!.items.map((i) => i.label)).not.toContain('Ouvrir une PR…');
  });

  it('says it in English', async () => {
    setLang('en');
    resetApp({
      projects: [project()],
      agents: [agent({ worktree: WT, prs: [pullRequest({ number: 12, checks: 'failure', openThreads: 1 })], prWord: 'MR' })],
    });
    fakeBackend();
    render(Sidebar, { project: project() });
    const group = within(screen.getByRole('group', { name: 'Pull requests of refacto-auth' }));
    expect(group.getByRole('button', { name: 'Open MR #12 in the browser (open)' })).toBeInTheDocument();
    expect(group.getByRole('button', { name: 'Agent fixes it' })).toBeInTheDocument();
    expect(group.getByRole('button', { name: 'Agent addresses the feedback' })).toBeInTheDocument();
  });
});
