import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { buffers } from '../lib/editor/buffers.svelte';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import { openTerminal } from '../lib/terminals';
import { agent, fakeBackend, gitInfo, project, resetApp, ticket } from '../test/ipc';
import Sidebar from './Sidebar.svelte';

// The xterm.js instance a terminal draws on is not one of jsdom's: only what is asked of the backend is looked at.
vi.mock('../lib/terminals', async (original) => ({
  ...(await original<typeof import('../lib/terminals')>()),
  openTerminal: vi.fn((projectId: string, shell: string, name: string) => Promise.resolve({ id: 't1', projectId, name, shell })),
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

  it('says on an agent’s card what it reported doing, until it reports something else', async () => {
    fakeBackend();
    app.agents.a1 = { ...app.agents.a1, progressLine: 'Écrit les tests du parseur' };
    render(Sidebar, { project: project() });
    const card = screen.getByRole('button', { name: /refacto-auth/ });
    const line = within(card).getByText('Écrit les tests du parseur');
    expect(line).toHaveAttribute('title', 'Écrit les tests du parseur');
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

  it('keeps it by the editor entry, and offers none for an archived agent', async () => {
    render(Sidebar, { project: project() });
    const labels = await menuOf(/wt-agent/);
    expect(labels.indexOf('Ouvrir un terminal')).toBe(labels.indexOf('Ouvrir dans l’éditeur') + 1);
    await userEvent.click(screen.getByText(/Archivés/));
    expect(await menuOf(/vieux/)).not.toContain('Ouvrir un terminal');
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

describe('Sidebar in English', () => {
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
    expect(screen.getByRole('button', { name: /New agent/ })).toHaveAttribute('title', 'New agent (Ctrl+N)');
    expect(screen.getByTitle('1 to review')).toBeInTheDocument();
  });

  it('says when an agent stopped by the usage limit resumes, and when its worktree is set up', () => {
    fakeBackend();
    app.now = new Date(2026, 8, 30, 12, 0).getTime();
    app.agents.a1 = { ...app.agents.a1, status: 'error', resumeAt: new Date(2026, 8, 30, 15, 0).getTime() };
    app.agents.a2 = { ...app.agents.a2, status: 'idle', setup: '2/3 · npm run gen (web)' };
    render(Sidebar, { project: project() });
    expect(screen.getByText('Resumes at 3:00 PM')).toHaveAttribute('title', 'Stopped by the usage limit: resumes automatically at 3:00 PM');
    expect(screen.getByText('Setting up…')).toHaveAttribute('title', 'Setting up the worktree: 2/3 · npm run gen (web)');
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
    expect(within(card).getByTitle('Remote control: connecting… (reachable from claude.ai and the Claude app)')).toBeInTheDocument();
    await fireEvent.contextMenu(card);
    expect(items()).toEqual([
      'Rename',
      'Duplicate the conversation',
      'Archive',
      'Prepare launch',
      'Open in editor',
      'Open a terminal',
      '',
      'Turn off remote control',
      'Open on claude.ai',
      'Copy the claude.ai link',
      '',
      'Delete…',
    ]);
    expect(menu.open!.items.find((i) => i.label === 'Archive')).toMatchObject({ hint: 'keeps the conversation' });
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
    await userEvent.click(screen.getByTitle('New terminal'));
    expect(app.toasts.at(-1)).toMatchObject({ text: 'No shell found (PowerShell 7, Git Bash, WSL). Check the settings.', kind: 'error' });
  });
});
