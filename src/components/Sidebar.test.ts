import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { buffers } from '../lib/editor/buffers.svelte';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, gitInfo, project, resetApp, ticket } from '../test/ipc';
import Sidebar from './Sidebar.svelte';

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

  it('says an agent sets its new worktree up', () => {
    fakeBackend();
    app.agents.a1 = { ...app.agents.a1, status: 'idle', setup: '2/3 · npm run gen (web)' };
    render(Sidebar, { project: project() });
    const status = within(screen.getByRole('button', { name: /refacto-auth/ })).getByText('Préparation…');
    expect(status).toHaveAttribute('title', 'Préparation du worktree : 2/3 · npm run gen (web)');
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
    expect(backend.called('create_agent')[0].args).toEqual({ projectId: 'p1', model: null });
    expect(app.agent?.id).toBe('a4');
  });

  it('tells when an agent stopped by the usage limit resumes', () => {
    fakeBackend();
    app.now = new Date(2026, 8, 30, 12, 0).getTime();
    app.agents.a1 = { ...app.agents.a1, status: 'error', resumeAt: new Date(2026, 8, 30, 15, 0).getTime() };
    render(Sidebar, { project: project() });
    const status = screen.getByText('Reprise à 15:00');
    expect(status).toHaveAttribute('title', 'Arrêté par la limite d’usage : reprise automatique à 15:00');
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
    expect(within(card).getByTitle(/Remote control : connecté/)).toBeInTheDocument();
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
    expect(screen.getByTitle(/Remote control : en attente de connexion/)).toBeInTheDocument();
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
    expect(screen.getByTitle(project().path).parentElement).toHaveClass('foot');
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
