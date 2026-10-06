import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { buffers, lossNotice } from '../../lib/editor/buffers.svelte';
import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import type { LaunchState, Project, TestRecipe, Ticket } from '../../lib/types';
import { agent, board, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import TicketCard from './TicketCard.svelte';

const doing = (over: Partial<Ticket> = {}) =>
  ticket({
    column: 'doing',
    agentId: 'a1',
    iteration: 2,
    criteria: [
      { text: 'Le fichier existe', ok: true, note: 'vu' },
      { text: 'Tests verts', ok: false, note: '' },
    ],
    ...over,
  });

const show = (t: Ticket, p: Project = project()) =>
  render(TicketCard, { ticket: t, project: p, queueIndex: 0, busyCount: 1, quota: null, onedit: () => {} });

const steps = (n: number) => Array.from({ length: n }, (_, i) => `Étape ${i + 1}`);

describe('TicketCard', () => {
  beforeEach(() => resetApp({ agents: [agent({ status: 'running', activity: 'Lit src/db.ts' })] }));

  it('shows a ticket under way with its loop, its criteria, its progress and what its agent does', () => {
    fakeBackend();
    show(doing());
    expect(screen.getByText('Boucle 2/5')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Critères atteints' })).toHaveAttribute('aria-valuenow', '1');
    expect(screen.getByText('Lit src/db.ts')).toBeInTheDocument();
    expect(screen.getByText('refacto-auth')).toBeInTheDocument();
    const items = within(screen.getByRole('list', { name: 'Critères' })).getAllByRole('listitem');
    expect(items[0]).toHaveTextContent('✓Le fichier existe');
    expect(items[1]).toHaveTextContent('○Tests verts');
  });

  it('points at a question waiting for an answer', () => {
    resetApp({ agents: [agent({ status: 'waiting' })] });
    fakeBackend();
    show(doing());
    expect(screen.getByText('Question en attente de ta réponse')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /DEM-1/ })).toHaveClass('waiting');
  });

  it('says its agent sets its worktree up before its first loop', () => {
    resetApp({ agents: [agent({ status: 'idle', setup: '1/2 · npm ci' })] });
    fakeBackend();
    show(doing({ iteration: 1 }));
    expect(screen.getByText('Prépare le worktree · 1/2 · npm ci')).toBeInTheDocument();
  });

  it('takes a blocked ticket up again', async () => {
    const backend = fakeBackend();
    show(doing({ blocked: 'Interrompu' }));
    expect(screen.getByText('Interrompu')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Reprendre' }));
    expect(backend.called('ticket_resume')).toEqual([{ cmd: 'ticket_resume', args: { id: 't1' } }]);
  });

  it('validates a ticket to test with the label of the board settings', async () => {
    const backend = fakeBackend();
    show(doing({ column: 'review', partial: true }), project({ board: board({ action: 'pr' }) }));
    expect(screen.getByText('Objectif partiel')).toBeInTheDocument();
    expect(screen.getByText('1/2 critères')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Valider + PR' }));
    expect(backend.called('ticket_approve')).toEqual([{ cmd: 'ticket_approve', args: { id: 't1' } }]);
  });

  it('sends a ticket back with what is wrong', async () => {
    const backend = fakeBackend();
    show(doing({ column: 'review' }));
    await userEvent.click(screen.getByRole('button', { name: 'Renvoyer' }));
    const send = screen.getByRole('button', { name: 'Renvoyer' });
    expect(send).toBeDisabled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Ce qui ne va pas' }), 'le bouton est mal placé');
    await userEvent.click(send);
    expect(backend.called('ticket_reject')).toEqual([{ cmd: 'ticket_reject', args: { id: 't1', comment: 'le bouton est mal placé' } }]);
    // Typing in the form did not open the agent.
    expect(app.ui.selectedAgent.p1).toBeUndefined();
    expect(screen.queryByRole('textbox', { name: 'Ce qui ne va pas' })).not.toBeInTheDocument();
  });

  it('keeps the send-back form and its comment when the backend refuses it', async () => {
    const backend = fakeBackend({
      ticket_reject: () => {
        throw 'Le ticket est en cours';
      },
    });
    show(doing({ column: 'review' }));
    await userEvent.click(screen.getByRole('button', { name: 'Renvoyer' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Ce qui ne va pas' }), 'le bouton est mal placé');
    await userEvent.click(screen.getByRole('button', { name: 'Renvoyer' }));
    expect(backend.called('ticket_reject')).toHaveLength(1);
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: expect.stringContaining('Le ticket est en cours') });
    expect(screen.getByRole('textbox', { name: 'Ce qui ne va pas' })).toHaveValue('le bouton est mal placé');
    // It can be sent again.
    expect(screen.getByRole('button', { name: 'Renvoyer' })).toBeEnabled();
  });

  it('keeps the native menu of the send-back field, and shows the ticket menu elsewhere on the card', async () => {
    fakeBackend();
    show(doing({ column: 'review' }));
    menu.close();
    await userEvent.click(screen.getByRole('button', { name: 'Renvoyer' }));
    // A right-click in the field is the browser's (copy, paste): the card does not take it.
    const field = screen.getByRole('textbox', { name: 'Ce qui ne va pas' });
    // `fireEvent` says whether the event was left alone: nobody cancelled it.
    expect(await fireEvent.contextMenu(field)).toBe(true);
    expect(menu.open).toBeNull();
    await fireEvent.contextMenu(screen.getByText('Ajouter le fichier'));
    expect(menu.open?.items.map((i) => i.label)).toContain("Ouvrir l'agent");
  });

  it('does not open the agent when a button of the card is used', async () => {
    const backend = fakeBackend();
    show(doing({ column: 'review' }));
    await userEvent.click(screen.getByRole('button', { name: 'Valider et merger' }));
    expect(backend.called('ticket_approve')).toHaveLength(1);
    expect(app.ui.selectedAgent.p1).toBeUndefined();
  });

  it('opens its agent from the keyboard, with Enter or Space, but not from the buttons inside it', async () => {
    fakeBackend();
    show(doing({ column: 'review' }));
    const card = screen.getByRole('button', { name: /DEM-1/ });
    card.focus();
    await userEvent.keyboard(' ');
    expect(app.ui.selectedAgent.p1).toBe('a1');
    delete app.ui.selectedAgent.p1;
    await userEvent.keyboard('{Enter}');
    expect(app.ui.selectedAgent.p1).toBe('a1');
    delete app.ui.selectedAgent.p1;
    // Space on a button of the card presses that button, nothing more.
    screen.getByRole('button', { name: 'Renvoyer' }).focus();
    await userEvent.keyboard(' ');
    expect(screen.getByRole('textbox', { name: 'Ce qui ne va pas' })).toBeInTheDocument();
    expect(app.ui.selectedAgent.p1).toBeUndefined();
  });

  it('shows a validation step, then a conflict with its two ways out', async () => {
    const backend = fakeBackend();
    const { rerender } = show(doing({ column: 'review', step: 'Merge…' }));
    expect(screen.getByText('Merge…')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Valider et merger' })).not.toBeInTheDocument();
    await rerender({ ticket: doing({ column: 'review', blocked: 'Conflit avec main', conflict: true }) });
    expect(screen.getByText('Conflit avec main')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: "L'agent résout" }));
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(backend.called('ticket_resolve_conflict')).toHaveLength(1);
    expect(backend.called('ticket_dismiss')).toHaveLength(1);
  });

  it('retries a validation that failed', async () => {
    const backend = fakeBackend();
    show(doing({ column: 'review', blocked: 'Validation interrompue' }));
    expect(screen.getByText('Validation interrompue')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Réessayer' }));
    expect(backend.called('ticket_approve')).toHaveLength(1);
  });

  it('shows a finished ticket with its outcome, its loops and its cost, and opens its link', async () => {
    const backend = fakeBackend();
    show(
      doing({ column: 'done', iteration: 3, cost: 0.42, outcome: '⇡ PR #12 → main', outcomeUrl: 'https://github.com/acme/demo/pull/12' }),
    );
    expect(screen.getByText('3 boucles · 0,42 $')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /DEM-1/ })).toHaveClass('done');
    await userEvent.click(screen.getByRole('button', { name: '⇡ PR #12 → main' }));
    expect(backend.called('plugin:opener|open_url')[0].args.url).toBe('https://github.com/acme/demo/pull/12');
  });

  it('says so when the link of a finished ticket does not open', async () => {
    const backend = fakeBackend({
      'plugin:opener|open_url': () => {
        throw 'Aucun navigateur';
      },
    });
    show(doing({ column: 'done', outcome: '⇡ PR #12 → main', outcomeUrl: 'https://github.com/acme/demo/pull/12' }));
    await userEvent.click(screen.getByRole('button', { name: '⇡ PR #12 → main' }));
    expect(backend.called('plugin:opener|open_url')).toHaveLength(1);
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ kind: 'error', text: expect.stringContaining('Aucun navigateur') });
    expect(app.ui.selectedAgent.p1).toBeUndefined();
  });

  it('opens its agent, or deletes it after a confirmation while under way', async () => {
    const backend = fakeBackend();
    show(doing());
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    expect(menu.open!.items.map((i) => i.label)).toEqual(["Ouvrir l'agent", '', 'Supprimer']);
    menu.open!.items.find((i) => i.label === 'Supprimer')!.onClick!();
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Supprimer le ticket DEM-1 ?' });
    await (app.modal as any).onConfirm(false);
    expect(backend.called('ticket_delete')).toEqual([{ cmd: 'ticket_delete', args: { id: 't1' } }]);
    await userEvent.click(screen.getByRole('button', { name: /DEM-1/ }));
    expect(app.ui.selectedAgent.p1).toBe('a1');
  });

  it('deletes a finished ticket from its menu without asking', async () => {
    const backend = fakeBackend();
    show(doing({ column: 'done' }));
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    menu.open!.items.find((i) => i.label === 'Supprimer')!.onClick!();
    expect(app.modal).toBeNull();
    await expect.poll(() => backend.called('ticket_delete')).toHaveLength(1);
  });

  describe('test launch', () => {
    const recipe: TestRecipe = {
      prepare: [],
      processes: [{ name: 'web', command: 'node web.js', dir: '', env: {}, url: 'http://localhost:4111' }],
      open: '',
    };
    const running = (): LaunchState => ({ status: 'running', ptyId: 't1', name: 'web', stopping: false, code: null, startedAt: 1 });
    /** The order in which the window stopped the launches and acted on the ticket. */
    const order = (backend: ReturnType<typeof fakeBackend>, cmd: string) =>
      backend.calls.map((c) => c.cmd).filter((c) => c === 'term_kill' || c === cmd);

    it('tests a ticket that has a recipe, and stops or opens what runs', async () => {
      resetApp({ agents: [agent({ recipe })] });
      const backend = fakeBackend({ test_run_start: () => ({ id: 't1', projectId: 'p1', name: 'web', shell: 'pwsh' }) });
      const { rerender } = show(doing({ column: 'review' }));
      await userEvent.click(screen.getByRole('button', { name: '▶ Tester' }));
      expect(app.modal).toEqual({ kind: 'testLaunch', agentId: 'a1' });
      app.launches['test:a1:run:0'] = running();
      await rerender({ ticket: doing({ column: 'review' }) });
      await userEvent.click(screen.getByRole('button', { name: 'Ouvrir' }));
      expect(backend.called('plugin:opener|open_url').at(-1)!.args.url).toBe('http://localhost:4111');
      await userEvent.click(screen.getByRole('button', { name: '■ Arrêter' }));
      expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
      // Testing did not open the agent.
      expect(app.ui.selectedAgent.p1).toBeUndefined();
    });

    it('offers no test while the ticket is being validated, nor without a recipe', () => {
      resetApp({ agents: [agent({ recipe })] });
      fakeBackend();
      const { unmount } = show(doing({ column: 'review', step: 'Tests…' }));
      expect(screen.queryByRole('button', { name: '▶ Tester' })).not.toBeInTheDocument();
      unmount();
      resetApp({ agents: [agent()] });
      show(doing({ column: 'review' }));
      expect(screen.queryByRole('button', { name: '▶ Tester' })).not.toBeInTheDocument();
    });

    it('stops its test launches before validating it', async () => {
      resetApp({ agents: [agent({ recipe })] });
      const backend = fakeBackend();
      app.launches['test:a1:run:0'] = running();
      show(doing({ column: 'review' }));
      await userEvent.click(screen.getByRole('button', { name: 'Valider et merger' }));
      expect(order(backend, 'ticket_approve')).toEqual(['term_kill', 'ticket_approve']);
    });

    it('stops its test launches before deleting it, which archives its agent', async () => {
      resetApp({ agents: [agent({ recipe })] });
      const backend = fakeBackend();
      app.launches['test:a1:run:0'] = running();
      show(doing({ column: 'review' }));
      await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
      menu.open!.items.find((i) => i.label === 'Supprimer')!.onClick!();
      await (app.modal as any).onConfirm(false);
      expect(order(backend, 'ticket_delete')).toEqual(['term_kill', 'ticket_delete']);
    });
  });

  describe('progress', () => {
    const list = () => screen.queryByRole('list', { name: 'Avancement' });
    const shown = () =>
      within(list()!)
        .getAllByRole('listitem')
        .map((i) => i.textContent);

    it('shows the last three items of a ticket under way, and how many are hidden', () => {
      fakeBackend();
      show(doing({ progress: steps(5) }));
      expect(shown()).toEqual(['Étape 3', 'Étape 4', 'Étape 5']);
      expect(screen.getByText('+2')).toBeInTheDocument();
      expect(screen.queryByText('Ce qui a été fait')).not.toBeInTheDocument();
    });

    it('shows all of a short progress under way, with no count of hidden ones', () => {
      fakeBackend();
      show(doing({ progress: steps(2) }));
      expect(shown()).toEqual(['Étape 1', 'Étape 2']);
      expect(screen.queryByText(/^\+\d/)).not.toBeInTheDocument();
    });

    // Three is the limit, not a hidden one.
    it('shows exactly three items without a count', () => {
      fakeBackend();
      show(doing({ progress: steps(3) }));
      expect(shown()).toEqual(['Étape 1', 'Étape 2', 'Étape 3']);
      expect(screen.queryByText(/^\+\d/)).not.toBeInTheDocument();
    });

    it('shows the whole progress of a ticket to test under "Ce qui a été fait"', () => {
      fakeBackend();
      show(doing({ column: 'review', progress: steps(6) }));
      expect(screen.getByText('Ce qui a été fait')).toBeInTheDocument();
      expect(shown()).toEqual(steps(6));
      expect(screen.queryByText(/^\+\d/)).not.toBeInTheDocument();
    });

    it('shows nothing when the agent reported none', () => {
      fakeBackend();
      const { unmount } = show(doing());
      expect(list()).not.toBeInTheDocument();
      unmount();
      show(doing({ column: 'review' }));
      expect(list()).not.toBeInTheDocument();
      expect(screen.queryByText('Ce qui a été fait')).not.toBeInTheDocument();
    });

    it('does not show it on a finished ticket', () => {
      fakeBackend();
      show(doing({ column: 'done', progress: steps(2) }));
      expect(list()).not.toBeInTheDocument();
    });
  });

  describe('validating a merge that removes the worktree', () => {
    const files = {
      fs_read: () => ({ kind: 'text', text: 'a\n', size: 2, hash: 'h1', eol: 'lf', bom: false }),
      fs_base: () => null,
      set_unsaved: () => null,
    };
    const dirty = async (agentId = 'a1') => {
      const k = (await buffers.open('p1', agentId, 'x.ts')).key;
      buffers.edit(k, 'b\n');
    };

    it('asks first when files of its worktree are unsaved in the editor, and validates once confirmed', async () => {
      const backend = fakeBackend(files);
      await dirty();
      show(doing({ column: 'review' }));
      await userEvent.click(screen.getByRole('button', { name: 'Valider et merger' }));
      expect(backend.called('ticket_approve')).toEqual([]);
      expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Valider DEM-1 ?', confirm: 'Valider et merger', danger: true });
      expect((app.modal as any).body.endsWith(lossNotice(1))).toBe(true);
      expect((app.modal as any).body).toContain('1 fichier non enregistré');
      await (app.modal as any).onConfirm(false);
      expect(backend.called('ticket_approve')).toEqual([{ cmd: 'ticket_approve', args: { id: 't1' } }]);
    });

    it('validates at once when nothing of its worktree is unsaved', async () => {
      const backend = fakeBackend(files);
      // Unsaved, but in the project's checkout and in another agent's worktree: they stay.
      await dirty('project');
      await dirty('a2');
      show(doing({ column: 'review' }));
      await userEvent.click(screen.getByRole('button', { name: 'Valider et merger' }));
      expect(app.modal).toBeNull();
      expect(backend.called('ticket_approve')).toHaveLength(1);
    });

    it.each([
      ['a pull request', board({ action: 'pr' }), 'Valider + PR'],
      ['a push', board({ action: 'push' }), 'Valider et pousser'],
      ['a worktree kept after the merge', board({ cleanup: false }), 'Valider et merger'],
      ['a ticket left as it is', board({ action: 'keep' }), 'Valider'],
    ])('does not ask for %s', async (_, b, label) => {
      const backend = fakeBackend(files);
      await dirty();
      show(doing({ column: 'review' }), project({ board: b }));
      await userEvent.click(screen.getByRole('button', { name: label }));
      expect(app.modal).toBeNull();
      expect(backend.called('ticket_approve')).toHaveLength(1);
    });
  });
});

describe('TicketCard of an imported ticket', () => {
  beforeEach(() => resetApp());

  it('names its external ticket, opens it, and says when its last sync failed', async () => {
    const backend = fakeBackend();
    const external = {
      service: 'jira' as const,
      id: 'ATL-1287',
      key: 'ATL-1287',
      container: 'ATL',
      url: 'https://atlas.atlassian.net/browse/ATL-1287',
      error: null,
    };
    const { unmount } = show(ticket({ external }));
    const link = screen.getByRole('button', { name: 'Ouvrir ATL-1287 dans Jira' });
    expect(link).toHaveTextContent('JATL-1287');
    expect(screen.getByText('DEM-1')).toBeInTheDocument();
    expect(screen.queryByRole('img', { name: /Synchro/ })).not.toBeInTheDocument();
    await userEvent.click(link);
    expect(backend.called('plugin:opener|open_url')[0].args.url).toBe('https://atlas.atlassian.net/browse/ATL-1287');
    // The click opens the ticket there, not the card's form.
    expect(app.modal).toBeNull();
    unmount();
    show(ticket({ external: { ...external, error: 'Jira refuse ces identifiants (401)' } }));
    expect(screen.getByRole('img', { name: 'Synchro avec Jira : Jira refuse ces identifiants (401)' })).toBeInTheDocument();
  });

  it('shows nothing of the kind for a ticket made on the Kanban', () => {
    fakeBackend();
    show(ticket());
    expect(screen.queryByRole('button', { name: /Ouvrir .* dans/ })).not.toBeInTheDocument();
  });
});
