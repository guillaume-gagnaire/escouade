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
import { setLang } from '../../lib/i18n';
import { menu } from '../../lib/menu.svelte';
import { ESTIMATE_HINT } from '../../lib/spend';
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

  it('asks before deleting a ticket to do, and deletes it only once confirmed', async () => {
    const backend = fakeBackend();
    show(ticket());
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Modifier', 'Passer en tête', '', 'Supprimer']);
    menu.open!.items.find((i) => i.label === 'Supprimer')!.onClick!();
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Supprimer DEM-1 ?',
      body: 'Le ticket et sa description sont supprimés.',
      confirm: 'Supprimer',
      danger: true,
    });
    expect(backend.called('ticket_delete')).toEqual([]);
    await (app.modal as any).onConfirm(false);
    expect(backend.called('ticket_delete')).toEqual([{ cmd: 'ticket_delete', args: { id: 't1' } }]);
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

describe('TicketCard of a ticket that comes after others', () => {
  const manual = () => project({ board: board({ autopilot: false }) });

  beforeEach(() =>
    resetApp({
      tickets: [ticket({ id: 't3', key: 'DEM-3', column: 'doing' }), ticket({ id: 't4', key: 'DEM-4', column: 'done' })],
    }),
  );

  it('says which tickets it waits for, those done left out, until they are done', async () => {
    fakeBackend();
    show(ticket({ key: 'DEM-5', after: ['t3', 't4'] }));
    expect(screen.getByText('⏸ après DEM-3')).toBeInTheDocument();
    expect(screen.queryByText("Pris dès qu'une place se libère")).not.toBeInTheDocument();
    app.tickets.t3 = { ...app.tickets.t3, column: 'done' };
    await expect.poll(() => screen.queryByText("Pris dès qu'une place se libère")).toBeInTheDocument();
    expect(screen.queryByText(/⏸ après/)).not.toBeInTheDocument();
  });

  it('asks before launching it by hand, and launches it once confirmed', async () => {
    const backend = fakeBackend();
    show(ticket({ key: 'DEM-5', after: ['t3'] }), manual());
    await userEvent.click(screen.getByRole('button', { name: 'Lancer' }));
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Lancer DEM-5 ?',
      body: 'DEM-5 attend DEM-3, pas encore terminé. Le lancer quand même ?',
      confirm: 'Lancer quand même',
    });
    expect(backend.called('ticket_start')).toEqual([]);
    await (app.modal as any).onConfirm(false);
    expect(backend.called('ticket_start')).toEqual([{ cmd: 'ticket_start', args: { id: 't1' } }]);
    // The click did not open its form.
    expect(app.ui.selectedAgent.p1).toBeUndefined();
  });

  it('launches at once a ticket whose tickets before are done', async () => {
    const backend = fakeBackend();
    show(ticket({ key: 'DEM-5', after: ['t4'] }), manual());
    expect(screen.getByText('Pilote auto désactivé')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Lancer' }));
    expect(app.modal).toBeNull();
    expect(backend.called('ticket_start')).toEqual([{ cmd: 'ticket_start', args: { id: 't1' } }]);
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
    expect(screen.queryByRole('button', { name: /Synchro/ })).not.toBeInTheDocument();
    await userEvent.click(link);
    expect(backend.called('plugin:opener|open_url')[0].args.url).toBe('https://atlas.atlassian.net/browse/ATL-1287');
    // The click opens the ticket there, not the card's form.
    expect(app.modal).toBeNull();
    unmount();
    show(ticket({ external: { ...external, error: 'Jira refuse ces identifiants (401)' } }));
    expect(screen.getByRole('button', { name: 'Synchro avec Jira : Jira refuse ces identifiants (401)' })).toBeInTheDocument();
  });

  const failed = {
    service: 'jira' as const,
    id: 'ATL-7',
    key: 'ATL-7',
    container: 'ATL',
    url: 'https://atlas.atlassian.net/browse/ATL-7',
    error: 'Jira refuse ces identifiants (401)',
  };

  it('offers « Resynchroniser » from its ⚠, which tries again at once', async () => {
    const backend = fakeBackend();
    const onedit = vi.fn();
    render(TicketCard, { ticket: ticket({ external: failed }), project: project(), queueIndex: 0, busyCount: 1, quota: null, onedit });
    const warn = screen.getByRole('button', { name: 'Synchro avec Jira : Jira refuse ces identifiants (401)' });
    expect(warn).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('button', { name: 'Resynchroniser' })).not.toBeInTheDocument();
    await userEvent.click(warn);
    expect(warn).toHaveAttribute('aria-expanded', 'true');
    // Its reason in full, without hovering, and what to do about it.
    expect(screen.getByText('Jira refuse ces identifiants (401)')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Resynchroniser' }));
    expect(backend.called('integration_resync')).toEqual([{ cmd: 'integration_resync', args: { ticketId: 't1' } }]);
    // Its buttons act: the card's form does not open.
    expect(onedit).not.toHaveBeenCalled();
    // Through: it closes, the focus left on the card (the ticket comes back without its ⚠).
    expect(screen.queryByRole('button', { name: 'Resynchroniser' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /DEM-1/ })).toHaveFocus();
    // From the keyboard too.
    warn.focus();
    await userEvent.keyboard('{Enter}');
    expect(screen.getByRole('button', { name: 'Resynchroniser' })).toBeInTheDocument();
  });

  it('says why « Resynchroniser » failed again', async () => {
    fakeBackend({
      integration_resync: () => {
        throw 'Jira refuse ces identifiants (401)';
      },
    });
    show(ticket({ external: failed }));
    await userEvent.click(screen.getByRole('button', { name: /Synchro avec Jira/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Resynchroniser' }));
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Jira refuse ces identifiants (401)', kind: 'error' });
    expect(screen.getByRole('button', { name: 'Resynchroniser' })).not.toHaveAttribute('aria-disabled', 'true');
  });

  it('keeps the focus on « Resynchroniser » while it runs, and asks only once', async () => {
    const backend = fakeBackend({ integration_resync: () => new Promise(() => {}) });
    show(ticket({ external: failed }));
    await userEvent.click(screen.getByRole('button', { name: /Synchro avec Jira/ }));
    const again = screen.getByRole('button', { name: 'Resynchroniser' });
    await userEvent.click(again);
    expect(again).toHaveAttribute('aria-disabled', 'true');
    expect(again).toHaveFocus();
    await userEvent.click(again);
    expect(backend.called('integration_resync')).toHaveLength(1);
  });

  it.each(['Resynchroniser', 'Synchro avec Jira : Jira refuse ces identifiants (401)'])(
    'leaves the focus on the card when a retry goes through by itself while « %s » holds it',
    async (name) => {
      fakeBackend();
      const { rerender } = show(ticket({ external: failed }));
      await userEvent.click(screen.getByRole('button', { name: /Synchro avec Jira/ }));
      screen.getByRole('button', { name }).focus();
      await rerender({ ticket: ticket({ external: { ...failed, error: null } }) });
      expect(screen.getByRole('button', { name: /DEM-1/ })).toHaveFocus();
    },
  );

  it('drops its ⚠ once a sync goes through', async () => {
    fakeBackend();
    const { rerender } = show(ticket({ external: failed }));
    await userEvent.click(screen.getByRole('button', { name: /Synchro avec Jira/ }));
    await rerender({ ticket: ticket({ external: { ...failed, error: null } }) });
    expect(screen.queryByRole('button', { name: /Synchro/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Resynchroniser' })).not.toBeInTheDocument();
  });

  it.each([
    ['jira', 'Jira'],
    ['trello', 'Trello'],
    ['github', 'GitHub'],
  ] as const)('says before deleting it that it will not be imported again from %s', async (service, name) => {
    const backend = fakeBackend();
    show(ticket({ external: { service, id: 'x-1', key: 'X-1', container: 'X', url: 'https://example.com/x-1', error: null } }));
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    menu.open!.items.find((i) => i.label === 'Supprimer')!.onClick!();
    // Its own key, not the external one.
    expect(app.modal).toMatchObject({
      title: 'Supprimer DEM-1 ?',
      body: `Le ticket et sa description sont supprimés. Il ne sera plus importé depuis ${name}.`,
      confirm: 'Supprimer',
      danger: true,
    });
    expect(backend.called('ticket_delete')).toEqual([]);
  });

  it('shows nothing of the kind for a ticket made on the Kanban', () => {
    fakeBackend();
    show(ticket());
    expect(screen.queryByRole('button', { name: /Ouvrir .* dans/ })).not.toBeInTheDocument();
  });
});

describe('TicketCard cost of a ticket under way', () => {
  // A ticket sent back and taken up by another agent: the first one is archived, both worked on it.
  const first = () => agent({ id: 'a0', ticketId: 't1', archived: true, tokens: 4000, cost: 1 });
  const second = (over = {}) => agent({ id: 'a1', ticketId: 't1', status: 'running', tokens: 1000, cost: 0.5, ...over });

  it('adds up the agents of a ticket under way, with the running turn as an estimate', () => {
    resetApp({ agents: [first(), second({ liveTokens: 250, liveCost: 0.125 })] });
    fakeBackend();
    show(doing());
    const cost = screen.getByText('≈ 1,63 $');
    expect(cost).toHaveAttribute('title', ESTIMATE_HINT);
  });

  it('shows the exact cost of a ticket to test, next to its criteria', () => {
    resetApp({ agents: [first(), second({ status: 'done' })] });
    fakeBackend();
    show(doing({ column: 'review' }));
    const cost = screen.getByText('1,50 $');
    expect(cost).not.toHaveAttribute('title');
    expect(screen.getByText('1/2 critères')).toBeInTheDocument();
  });

  it('leaves out the agents of other tickets', () => {
    resetApp({ agents: [second(), agent({ id: 'a9', ticketId: 't2', cost: 7 }), agent({ id: 'a8', ticketId: null, cost: 8 })] });
    fakeBackend();
    show(doing());
    expect(screen.getByText('0,50 $')).toBeInTheDocument();
    expect(screen.queryByText(/7,00|8,00/)).not.toBeInTheDocument();
  });

  it('says nothing until its agents used something', () => {
    resetApp({ agents: [second({ tokens: 0, cost: 0 })] });
    fakeBackend();
    show(doing());
    expect(screen.queryByText(/\$/)).not.toBeInTheDocument();
  });

  it('keeps the cost of a finished ticket as it was, and adds no other', () => {
    resetApp({ agents: [first(), second({ status: 'done', archived: true })] });
    fakeBackend();
    show(doing({ column: 'done', iteration: 2, cost: 0.5 }));
    expect(screen.getByText('2 boucles · 0,50 $')).toBeInTheDocument();
    expect(screen.queryByText('1,50 $')).not.toBeInTheDocument();
  });

  it('shows no cost on a ticket to do', () => {
    resetApp({ agents: [first()] });
    fakeBackend();
    show(ticket({ column: 'todo' }));
    expect(screen.queryByText(/\$/)).not.toBeInTheDocument();
  });
});

// jsdom lays nothing out: what keeps the foot inside a column a quarter of the board wide (the e2e test of the
// ticket's loop measures it) is that its figures are one group beside the agent's name, which the CSS wraps under it.
describe('TicketCard foot', () => {
  const foot = (container: HTMLElement) => container.querySelector<HTMLElement>('.foot')!;
  const figuresOf = (container: HTMLElement) => {
    const group = foot(container).querySelector<HTMLElement>(':scope > .figures');
    expect(group, 'the figures of the foot are one group').not.toBeNull();
    return group!;
  };

  it('groups the cost and the criteria met of a ticket to test apart from its agent’s name', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'done', tokens: 1000, cost: 1.5 })] });
    fakeBackend();
    const { container } = show(doing({ column: 'review' }));
    const figures = figuresOf(container);
    expect(within(figures).getByText('1,50 $')).toBeInTheDocument();
    expect(within(figures).getByText('1/2 critères')).toBeInTheDocument();
    expect(within(figures).queryByText('refacto-auth')).not.toBeInTheDocument();
    expect(foot(container)).toContainElement(screen.getByText('refacto-auth'));
  });

  it('groups the loops and the cost of a finished ticket apart from its agent’s name', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'done', archived: true })] });
    fakeBackend();
    const { container } = show(doing({ column: 'done', iteration: 2, cost: 0.5 }));
    const figures = figuresOf(container);
    expect(within(figures).getByText('2 boucles · 0,50 $')).toBeInTheDocument();
    expect(within(figures).queryByText('refacto-auth')).not.toBeInTheDocument();
  });

  it('has no group of figures while there is nothing to say, which would add an empty line under the name', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'running', tokens: 0, cost: 0 })] });
    fakeBackend();
    const { container } = show(doing());
    expect(foot(container)).toContainElement(screen.getByText('refacto-auth'));
    expect(container.querySelector('.figures')).toBeNull();
  });
});

// The window in English: the texts of the card come from the English catalog, its counts follow the plural rules of the
// language and its amounts and times are written as English does. (What `lib/board.ts` and `lib/spend.ts` say is theirs.)
describe('TicketCard in English', () => {
  beforeEach(() => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'running' })] });
    fakeBackend();
    setLang('en');
  });

  it('writes a ticket under way in English', () => {
    show(doing());
    expect(screen.getByText('Loop 2/5')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Criteria met' })).toHaveAttribute('aria-valuenow', '1');
    expect(within(screen.getByRole('list', { name: 'Criteria' })).getAllByRole('listitem')).toHaveLength(2);
    // The agent has not said what it does yet.
    expect(screen.getByText('Thinking')).toBeInTheDocument();
  });

  it('points at a question waiting for an answer, in English', () => {
    resetApp({ agents: [agent({ id: 'a1', status: 'waiting' })] });
    show(doing());
    expect(screen.getByText('Question waiting for your answer')).toBeInTheDocument();
  });

  it('writes the time an agent resumes at in English', () => {
    const at = new Date();
    at.setHours(15, 0, 0, 0);
    resetApp({ agents: [agent({ id: 'a1', status: 'idle', resumeAt: at.getTime() })] });
    app.now = at.getTime() - 1000;
    show(doing());
    expect(screen.getByText('Resumes at 3:00 PM')).toBeInTheDocument();
  });

  it('writes the setup of a worktree in English', () => {
    resetApp({ agents: [agent({ id: 'a1', status: 'idle', setup: '1/2 · npm ci' })] });
    show(doing({ iteration: 1 }));
    expect(screen.getByText('Setting up the worktree · 1/2 · npm ci')).toBeInTheDocument();
  });

  it('counts the criteria of a ticket to do with the plural of English', () => {
    const { unmount } = show(ticket());
    expect(screen.getByText('2 criteria · max 5 loops')).toBeInTheDocument();
    unmount();
    show(ticket({ criteria: [{ text: 'Le fichier existe', ok: false, note: '' }] }));
    expect(screen.getByText('1 criterion · max 5 loops')).toBeInTheDocument();
  });

  it('shows the cost of a ticket to test in dollars, with what it is for a screen reader, and its criteria met', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'done', tokens: 1000, cost: 1.5 })] });
    const { container } = show(doing({ column: 'review' }));
    const figures = container.querySelector<HTMLElement>('.figures')!;
    expect(within(figures).getByText('$1.50')).toBeInTheDocument();
    expect(within(figures).getByText('1/2 criteria')).toBeInTheDocument();
    // The hidden label and the amount are apart for a screen reader.
    expect(figures.textContent).toContain('Ticket cost $1.50');
    expect(screen.getByRole('button', { name: 'Send back' })).toBeInTheDocument();
  });

  it('writes the send-back form in English', async () => {
    show(doing({ column: 'review' }));
    await userEvent.click(screen.getByRole('button', { name: 'Send back' }));
    expect(screen.getByRole('textbox', { name: 'What is wrong' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: 'Send back' })).toHaveLength(1);
  });

  it('writes the menu of a ticket to do, and the confirmation to delete it, in English', async () => {
    show(ticket());
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Edit', 'Move to top', '', 'Delete']);
    menu.open!.items.find((i) => i.label === 'Delete')!.onClick!();
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Delete DEM-1?',
      body: 'The ticket and its description are deleted.',
      confirm: 'Delete',
    });
  });

  it('says it will no longer import a deleted ticket that came from a service', async () => {
    const external = { service: 'jira' as const, id: 'ATL-1', key: 'ATL-1', container: 'ATL', url: 'https://a.test/ATL-1', error: null };
    show(ticket({ external }));
    expect(screen.getByRole('button', { name: 'Open ATL-1 in Jira' })).toBeInTheDocument();
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    menu.open!.items.find((i) => i.label === 'Delete')!.onClick!();
    expect(app.modal).toMatchObject({
      body: 'The ticket and its description are deleted. It will no longer be imported from Jira.',
    });
  });

  it('writes the menu of a ticket under way in English', async () => {
    show(doing());
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Open agent', '', 'Delete']);
    menu.open!.items.find((i) => i.label === 'Delete')!.onClick!();
    expect(app.modal).toMatchObject({ title: 'Delete ticket DEM-1?', body: 'Its agent is archived, along with its worktree.' });
  });

  it('says how many items of the progress are hidden, with the plural of English', () => {
    show(doing({ progress: ['One', 'Two', 'Three', 'Four'] }));
    expect(screen.getByText('+1')).toHaveAttribute('title', '1 more item');
  });
});
