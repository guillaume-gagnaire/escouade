import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { buffers, lossNotice } from '../../lib/editor/buffers.svelte';
import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import type { Project, Ticket } from '../../lib/types';
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
