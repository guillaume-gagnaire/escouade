import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The cards' test launches have logs (xterm.js): none in jsdom.
vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import Board from './Board.svelte';

const col = (name: string) => screen.getByRole('region', { name });

describe('Board', () => {
  beforeEach(() => {
    resetApp({
      tickets: [
        ticket(),
        ticket({ id: 't2', key: 'DEM-2', title: 'Deuxième', rank: 2 }),
        ticket({ id: 't3', key: 'DEM-3', title: 'En route', column: 'doing', agentId: 'a1', iteration: 1, startedAt: 1 }),
      ],
    });
    app.claudeFound = true;
  });

  it('shows the four columns with their counts and the board header', () => {
    fakeBackend();
    render(Board, { project: app.projects[0] });
    expect(within(col('À faire')).getByText('2', { selector: '.count' })).toBeInTheDocument();
    expect(within(col('En cours')).getByText('DEM-3')).toBeInTheDocument();
    expect(within(col('À tester')).getByText('Rien à tester')).toBeInTheDocument();
    expect(within(col('Terminé')).getByText('Aucun ticket terminé')).toBeInTheDocument();
    expect(screen.getByText('demo-api · 3 tickets · 1 en boucle')).toBeInTheDocument();
    expect(screen.getByText('1 place libre')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Après validation : merge squash → main/ })).toBeInTheDocument();
  });

  it('says that no ticket starts while Claude Code is not found', () => {
    fakeBackend();
    app.claudeFound = false;
    render(Board, { project: app.projects[0] });
    expect(screen.getByText('Claude Code introuvable — aucun ticket ne démarre')).toBeInTheDocument();
    expect(screen.queryByText('1 place libre')).not.toBeInTheDocument();
  });

  it('says why no ticket starts while its target branch cannot start one, and no card claims a place', () => {
    fakeBackend();
    const issue = "main n'a encore aucun commit — aucun ticket ne démarre";
    app.boardIssues = { p1: issue };
    render(Board, { project: app.projects[0] });
    const said = screen.getByText(issue);
    expect(said).toHaveClass('places', 'missing');
    expect(said).toHaveAttribute('title', issue);
    expect(screen.queryByText('1 place libre')).not.toBeInTheDocument();
    const todo = col('À faire');
    expect(within(todo).queryByText("Pris dès qu'une place se libère")).not.toBeInTheDocument();
    expect(within(todo).getAllByText('En attente de la branche cible')).toHaveLength(2);
    // Another project's issue says nothing of this one.
    app.boardIssues = { p2: issue };
    return expect.poll(() => screen.queryByText('1 place libre')).toBeInTheDocument();
  });

  it('adds a ticket from the form at the top of "À faire"', async () => {
    const backend = fakeBackend({ ticket_create: (a: any) => ticket({ id: 't9', key: 'DEM-9', title: a.draft.title }) });
    render(Board, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau ticket' }));
    const add = screen.getByRole('button', { name: 'Ajouter' });
    expect(add).toBeDisabled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Limiter les tentatives');
    await userEvent.type(screen.getByRole('textbox', { name: "Critères d'acceptation" }), '5 essais{Enter}Réponse 429');
    await userEvent.click(screen.getByRole('button', { name: '8' }));
    await userEvent.click(add);
    expect(backend.called('ticket_create')[0].args).toEqual({
      projectId: 'p1',
      draft: { title: 'Limiter les tentatives', description: '', criteria: ['5 essais', 'Réponse 429'], maxLoops: 8 },
    });
    expect(await within(col('À faire')).findByText('DEM-9')).toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: 'Titre du ticket' })).not.toBeInTheDocument();
  });

  it('tells each ticket to do when it will start', () => {
    fakeBackend();
    render(Board, { project: app.projects[0] });
    const todo = col('À faire');
    expect(within(todo).getAllByText('2 critères · max 5 boucles')).toHaveLength(2);
    expect(within(todo).getByText("Pris dès qu'une place se libère")).toBeInTheDocument();
    expect(within(todo).getByText("En attente d'une place (1/2)")).toBeInTheDocument();
    expect(within(todo).queryByRole('button', { name: 'Lancer' })).not.toBeInTheDocument();
  });

  it('keeps the newer ticket the backend already sent rather than the one its creation returns', async () => {
    const created = ticket({ id: 't9', key: 'DEM-9', title: 'Vite parti' });
    fakeBackend({
      ticket_create: () => {
        // Its events came first: created, then started at once.
        app.tickets.t9 = { ...created, column: 'doing', agentId: 'a1', iteration: 1, startedAt: 2 };
        return created;
      },
    });
    render(Board, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau ticket' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Vite parti');
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }));
    expect(await within(col('En cours')).findByText('DEM-9')).toBeInTheDocument();
    expect(app.tickets.t9.column).toBe('doing');
    expect(within(col('À faire')).queryByText('DEM-9')).not.toBeInTheDocument();
  });

  it('keeps the newer board the backend already sent rather than the one the autopilot switch gets back', async () => {
    fakeBackend({
      board_set: (a: any) => {
        // Its event came first, then a newer one (the first ticket fixed the target meanwhile).
        app.replaceProject(project({ board: { ...a.settings, target: 'release' } }));
        return project({ board: a.settings });
      },
    });
    render(Board, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('switch', { name: 'Pilote auto' }));
    await new Promise((r) => setTimeout(r));
    expect(app.projects[0].board).toMatchObject({ autopilot: false, target: 'release' });
  });

  it('names its settings button, and gives what validating does in full in its tooltip', () => {
    fakeBackend();
    render(Board, { project: app.projects[0] });
    const button = screen.getByRole('button', { name: /Après validation/ });
    expect(button).toHaveAttribute('title', 'Réglages du Kanban — merge squash → main');
    // The summary has no tooltip of its own, which would hide the button's.
    expect(within(button).getByText('merge squash → main')).not.toHaveAttribute('title');
  });

  it('is the Kanban, whose settings button opens the settings on its tab and project', async () => {
    fakeBackend();
    render(Board, { project: app.projects[0] });
    expect(screen.getByText('Kanban', { selector: '.t' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Après validation/ }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'board', projectId: 'p1' });
  });

  it('turns the autopilot off and lets a ticket be launched by hand', async () => {
    const backend = fakeBackend({
      // The board as the backend's event gives it (the command's answer may come after a newer one).
      board_set: (a: any) => {
        app.replaceProject(project({ board: a.settings }));
        return null;
      },
    });
    render(Board, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('switch', { name: 'Pilote auto' }));
    expect(backend.called('board_set')[0].args.settings.autopilot).toBe(false);
    expect(await screen.findByText('Pilote auto · off')).toBeInTheDocument();
    expect(within(col('À faire')).getAllByText('Pilote auto désactivé')).toHaveLength(2);
    await userEvent.click(within(col('À faire')).getAllByRole('button', { name: 'Lancer' })[0]);
    expect(backend.called('ticket_start')).toEqual([{ cmd: 'ticket_start', args: { id: 't1' } }]);
  });

  it('edits a ticket to do from a click, and moves or deletes it from its menu', async () => {
    const backend = fakeBackend({ ticket_update: (a: any) => ticket({ title: a.draft.title }) });
    render(Board, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('button', { name: /DEM-1/ }));
    const title = screen.getByRole('textbox', { name: 'Titre du ticket' });
    expect(title).toHaveValue('Ajouter le fichier');
    await userEvent.clear(title);
    await userEvent.type(title, 'Ajouter le fichier du ticket');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(backend.called('ticket_update')[0].args.draft.title).toBe('Ajouter le fichier du ticket');
    expect(await screen.findByText('Ajouter le fichier du ticket')).toBeInTheDocument();
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-2/ }));
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Modifier', 'Passer en tête', '', 'Supprimer']);
    menu.open!.items.find((i) => i.label === 'Passer en tête')!.onClick!();
    expect(backend.called('ticket_prioritize')).toEqual([{ cmd: 'ticket_prioritize', args: { id: 't2' } }]);
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-2/ }));
    menu.open!.items.find((i) => i.label === 'Supprimer')!.onClick!();
    await expect.poll(() => app.tickets.t2).toBeUndefined();
  });

  it('opens the agent of a ticket under way', async () => {
    resetApp({ agents: [agent()], tickets: [ticket({ column: 'doing', agentId: 'a1', iteration: 1 })] });
    fakeBackend();
    render(Board, { project: app.projects[0] });
    app.openBoard('p1');
    await userEvent.click(screen.getByRole('button', { name: /DEM-1/ }));
    expect(app.ui.selectedAgent.p1).toBe('a1');
    expect(app.boardOn).toBe(false);
  });
});

describe('Board import', () => {
  beforeEach(() => resetApp());

  it('opens the import of the project’s tickets', async () => {
    fakeBackend();
    render(Board, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('button', { name: 'Importer' }));
    expect(app.modal).toEqual({ kind: 'import', projectId: 'p1' });
  });
});
