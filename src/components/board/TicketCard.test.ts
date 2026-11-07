import { cleanup, fireEvent, render, screen, within } from '@testing-library/svelte';
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
import { estimateHint } from '../../lib/spend';
import { app } from '../../lib/state.svelte';
import type { LaunchState, Project, TestRecipe, Ticket } from '../../lib/types';
import { agent, BARE_ENGINE, board, CLAUDE_ENGINE, fakeBackend, project, resetApp, ticket, ticketPr } from '../../test/ipc';
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

  it('shows a ticket under way with its loop in squares, its criteria checked and what its agent does', () => {
    fakeBackend();
    const { container } = show(doing());
    expect(screen.getByText('Boucle 2/5')).toBeInTheDocument();
    // A square a loop, those gone through filled.
    const squares = [...container.querySelectorAll('.loops i')];
    expect(squares).toHaveLength(5);
    expect(squares.map((i) => i.classList.contains('on'))).toEqual([true, true, false, false, false]);
    // The verb, then what it acts on, as on its card in the list.
    const act = container.querySelector('.act')!;
    expect(act).toHaveTextContent('Lit src/db.ts');
    expect(within(act as HTMLElement).getByText('Lit').tagName).toBe('B');
    expect(within(act as HTMLElement).getByText('src/db.ts')).toHaveClass('mono');
    // Each criterion drawn with an icon, not a character; a met one says so to a screen reader.
    const items = within(screen.getByRole('list', { name: 'Critères' })).getAllByRole('listitem');
    expect(items[0]).toHaveTextContent('Le fichier existe atteint');
    expect(items[0]).toHaveClass('met');
    expect(items[0].querySelector('[data-icon="check-c"]')).not.toBeNull();
    expect(items[1]).toHaveTextContent(/^Tests verts$/);
    expect(items[1].querySelector('[data-icon="circle"]')).not.toBeNull();
  });

  it('shows a paperclip and the number of its images, in every column, and nothing without any', () => {
    fakeBackend();
    const images = [
      { file: 'a1.png', name: 'capture.png', mediaType: 'image/png' },
      { file: 'b2.gif', name: 'schema.gif', mediaType: 'image/gif' },
    ];
    const card = () => screen.getByRole('button', { name: 'DEM-1 Ajouter le fichier' });
    const { unmount } = show(ticket({ images }));
    // The card is a button named after its key and title: the count is its description, for a screen reader to say it.
    expect(card()).toHaveAccessibleDescription('2 images jointes');
    const clip = card().querySelector('.clip')!;
    expect(clip).toHaveAttribute('data-tip', '2 images jointes');
    expect(clip.querySelector('[data-icon="clip"]')).not.toBeNull();
    expect(clip.querySelector('.n')).toHaveTextContent('2');
    unmount();
    const under = show(doing({ images: images.slice(1) }));
    expect(card()).toHaveAccessibleDescription('1 image jointe');
    under.unmount();
    show(ticket());
    expect(card()).not.toHaveAttribute('aria-describedby');
    expect(card().querySelector('.clip')).toBeNull();
  });

  it('counts its files in the paperclip too, the images and the files told apart in what it says', () => {
    fakeBackend();
    const image = { file: 'a1.png', name: 'capture.png', mediaType: 'image/png' };
    const files = [
      { name: 'budget.xlsx', file: 'ab12cd34/budget.xlsx', size: 2048 },
      { name: 'sources.zip', file: 'ef56ab78/sources.zip', size: 4096 },
    ];
    const card = () => screen.getByRole('button', { name: 'DEM-1 Ajouter le fichier' });
    const check = (description: string, count: string) => {
      expect(card()).toHaveAccessibleDescription(description);
      const clip = card().querySelector('.clip')!;
      expect(clip).toHaveAttribute('data-tip', description);
      expect(clip.querySelector('.n')).toHaveTextContent(count);
    };
    const only = show(ticket({ files }));
    check('2 fichiers joints', '2');
    only.unmount();
    const one = show(doing({ files: files.slice(1) }));
    check('1 fichier joint', '1');
    one.unmount();
    show(ticket({ images: [image], files }));
    check('1 image jointe, 2 fichiers joints', '3');
  });

  it('shows the line its agent reported, with what it does right now, until it reports another', () => {
    resetApp({ agents: [agent({ status: 'running', activity: 'Lit src/db.ts', progressLine: 'Écrit les tests du parseur' })] });
    fakeBackend();
    const { unmount } = show(doing());
    const line = screen.getByText('Écrit les tests du parseur');
    expect(line).toHaveAttribute('data-tip', 'Écrit les tests du parseur');
    expect(line).toHaveTextContent('Ce que l’agent dit faire Écrit les tests du parseur');
    expect(document.querySelector('.act')).toHaveTextContent('Lit src/db.ts');
    unmount();
    // Nothing reported, nothing shown.
    resetApp({ agents: [agent({ status: 'running', activity: 'Lit src/db.ts', progressLine: null })] });
    show(doing());
    expect(screen.queryByText('Ce que l’agent dit faire')).not.toBeInTheDocument();
  });

  it('shows the reported line only while the ticket is under way', () => {
    resetApp({ agents: [agent({ status: 'idle', progressLine: 'Écrit les tests du parseur' })] });
    fakeBackend();
    const { unmount } = show(doing({ column: 'review' }));
    expect(screen.queryByText('Écrit les tests du parseur')).not.toBeInTheDocument();
    unmount();
    show(ticket());
    expect(screen.queryByText('Écrit les tests du parseur')).not.toBeInTheDocument();
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
    // « Valider » draws what validating does, which its name says whole.
    const approve = screen.getByRole('button', { name: 'Valider + PR' });
    expect(approve).toHaveTextContent(/^Valider$/);
    expect(approve.querySelector('[data-icon="pr"]')).not.toBeNull();
    await userEvent.click(approve);
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
    expect(screen.getByRole('button', { name: /DEM-1/ })).toHaveClass('past');
    // The backend's glyph is drawn as an icon.
    const link = screen.getByRole('button', { name: 'PR #12 → main' });
    expect(link.querySelector('[data-icon="pr"]')).not.toBeNull();
    await userEvent.click(link);
    expect(backend.called('open_url')[0].args.url).toBe('https://github.com/acme/demo/pull/12');
  });

  it('says so when the link of a finished ticket does not open', async () => {
    const backend = fakeBackend({
      open_url: () => {
        throw 'Aucun navigateur';
      },
    });
    show(doing({ column: 'done', outcome: '⇡ PR #12 → main', outcomeUrl: 'https://github.com/acme/demo/pull/12' }));
    await userEvent.click(screen.getByRole('button', { name: 'PR #12 → main' }));
    expect(backend.called('open_url')).toHaveLength(1);
    await expect.poll(() => app.toasts.at(-1)).toMatchObject({ kind: 'error', text: expect.stringContaining('Aucun navigateur') });
    expect(app.ui.selectedAgent.p1).toBeUndefined();
  });

  it('opens its agent, or deletes it after a confirmation while under way', async () => {
    const backend = fakeBackend();
    show(doing());
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    // « Partager avec l’équipe » too (ESC-190), between its separators.
    expect(menu.open!.items.map((i) => i.label)).toEqual(["Ouvrir l'agent", '', 'Partager avec l’équipe', '', 'Supprimer']);
    menu.open!.items.find((i) => i.label === 'Supprimer')!.onClick!();
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Supprimer le ticket DEM-1 ?',
      body: "Son agent est archivé et son worktree supprimé, sauf s'il garde des changements non commités ou des commits ni mergés ni poussés.",
    });
    await (app.modal as any).onConfirm(false);
    expect(backend.called('ticket_delete')).toEqual([{ cmd: 'ticket_delete', args: { id: 't1' } }]);
    await userEvent.click(screen.getByRole('button', { name: /DEM-1/ }));
    expect(app.ui.selectedAgent.p1).toBe('a1');
  });

  it('asks before deleting a ticket to do, and deletes it only once confirmed', async () => {
    const backend = fakeBackend();
    show(ticket());
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Modifier', 'Passer en tête', '', 'Partager avec l’équipe', '', 'Supprimer']);
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

  describe('deleting a ticket that has images', () => {
    const IMAGES = [
      { file: 'a1.png', name: 'capture.png', mediaType: 'image/png' },
      { file: 'b2.gif', name: 'schema.gif', mediaType: 'image/gif' },
    ];
    /** The body of the confirmation the menu's « Supprimer » opens for this ticket. */
    async function bodyFor(t: Ticket) {
      fakeBackend();
      show(t);
      await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
      menu.open!.items.find((i) => i.label === 'Supprimer')!.onClick!();
      return (app.modal as { body: string }).body;
    }

    it.each([
      [1, 'Le ticket, sa description et son image sont supprimés.'],
      [2, 'Le ticket, sa description et ses 2 images sont supprimés.'],
    ])('says with %i image(s) that they go with the ticket', async (count, body) => {
      expect(await bodyFor(ticket({ images: IMAGES.slice(0, count) }))).toBe(body);
    });

    it.each([
      [1, 'Le ticket, sa description et son image sont supprimés. Il ne sera plus importé depuis Jira.'],
      [2, 'Le ticket, sa description et ses 2 images sont supprimés. Il ne sera plus importé depuis Jira.'],
    ])('says it as well, with %i image(s), for a ticket imported from a service', async (count, body) => {
      const external = { service: 'jira' as const, id: 'ATL-1', key: 'ATL-1', container: 'ATL', url: 'https://a.test/ATL-1', error: null };
      expect(await bodyFor(ticket({ images: IMAGES.slice(0, count), external }))).toBe(body);
    });

    const FILES = [
      { name: 'budget.xlsx', file: 'ab12cd34/budget.xlsx', size: 2048 },
      { name: 'sources.zip', file: 'ef56ab78/sources.zip', size: 4096 },
    ];

    it.each([
      [[], 1, 'Le ticket, sa description et sa pièce jointe sont supprimés.'],
      [[], 2, 'Le ticket, sa description et ses 2 pièces jointes sont supprimés.'],
      // The images and the files count together: they are all attached to it.
      [IMAGES, 1, 'Le ticket, sa description et ses 3 pièces jointes sont supprimés.'],
    ])('says as well that the files go with the ticket (case %#)', async (images, count, body) => {
      expect(await bodyFor(ticket({ images, files: FILES.slice(0, count) }))).toBe(body);
    });

    it('says it of the files for a ticket imported from a service as well', async () => {
      const external = { service: 'jira' as const, id: 'ATL-1', key: 'ATL-1', container: 'ATL', url: 'https://a.test/ATL-1', error: null };
      expect(await bodyFor(ticket({ files: FILES.slice(0, 1), external }))).toBe(
        'Le ticket, sa description et sa pièce jointe sont supprimés. Il ne sera plus importé depuis Jira.',
      );
      cleanup();
      expect(await bodyFor(ticket({ files: FILES, external }))).toBe(
        'Le ticket, sa description et ses 2 pièces jointes sont supprimés. Il ne sera plus importé depuis Jira.',
      );
    });
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
      await userEvent.click(screen.getByRole('button', { name: 'Tester' }));
      expect(app.modal).toEqual({ kind: 'testLaunch', agentId: 'a1' });
      app.launches['test:a1:run:0'] = running();
      await rerender({ ticket: doing({ column: 'review' }) });
      await userEvent.click(screen.getByRole('button', { name: 'Ouvrir' }));
      expect(backend.called('open_url').at(-1)!.args.url).toBe('http://localhost:4111');
      await userEvent.click(screen.getByRole('button', { name: 'Arrêter' }));
      expect(backend.called('term_kill')[0].args).toEqual({ id: 't1' });
      // Testing did not open the agent.
      expect(app.ui.selectedAgent.p1).toBeUndefined();
    });

    it('offers no test while the ticket is being validated, nor without a recipe', () => {
      resetApp({ agents: [agent({ recipe })] });
      fakeBackend();
      const { unmount } = show(doing({ column: 'review', step: 'Tests…' }));
      expect(screen.queryByRole('button', { name: 'Tester' })).not.toBeInTheDocument();
      unmount();
      resetApp({ agents: [agent()] });
      show(doing({ column: 'review' }));
      expect(screen.queryByRole('button', { name: 'Tester' })).not.toBeInTheDocument();
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
    expect(screen.getByText('Après DEM-3')).toBeInTheDocument();
    expect(screen.queryByText("Pris dès qu'une place se libère")).not.toBeInTheDocument();
    app.tickets.t3 = { ...app.tickets.t3, column: 'done' };
    await expect.poll(() => screen.queryByText("Pris dès qu'une place se libère")).toBeInTheDocument();
    expect(screen.queryByText(/Après DEM/)).not.toBeInTheDocument();
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
    expect(backend.called('open_url')[0].args.url).toBe('https://atlas.atlassian.net/browse/ATL-1287');
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
    expect(cost).toHaveAttribute('data-tip', estimateHint());
  });

  it('shows the exact cost of a ticket to test, next to its criteria', () => {
    resetApp({ agents: [first(), second({ status: 'done' })] });
    fakeBackend();
    const { container } = show(doing({ column: 'review' }));
    // With its loops, as a finished ticket: no estimate to tell.
    expect(screen.getByText('2 boucles · 1,50 $')).toBeInTheDocument();
    expect(container.querySelector('.sum')).not.toHaveAttribute('data-tip');
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

// The card's first line: its key, its harness, its paperclip, at the right its loop, or the loops and the cost of a
// ticket to test or done; its last line, what its agent does and what it cost so far.
describe('TicketCard lines', () => {
  it('puts the loops and the cost of a ticket to test at the right of its first line', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'done', tokens: 1000, cost: 1.5 })] });
    fakeBackend();
    const { container } = show(doing({ column: 'review' }));
    expect(container.querySelector('.top .sum')).toHaveTextContent('Coût du ticket 2 boucles · 1,50 $');
    expect(container.querySelector('.act')).toBeNull();
  });

  it('puts the loops and the cost of a finished ticket there too', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'done', archived: true })] });
    fakeBackend();
    const { container } = show(doing({ column: 'done', iteration: 2, cost: 0.5 }));
    expect(container.querySelector('.top .sum')).toHaveTextContent('2 boucles · 0,50 $');
  });

  it('says what the agent of a ticket under way does, and its cost once it used something', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'running', tokens: 0, cost: 0 })] });
    fakeBackend();
    const { container, unmount } = show(doing());
    const act = container.querySelector('.act')!;
    expect(act).toHaveTextContent(/^Réfléchit$/);
    expect(act.querySelector('.dot')).not.toBeNull();
    unmount();
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'running', activity: 'Edit src/a.ts', tokens: 10, cost: 0.41 })] });
    const again = show(doing());
    expect(again.container.querySelector('.act')).toHaveTextContent('Edit src/a.ts Coût du ticket 0,41 $');
  });

  it('shows the brand of the harness its agent runs on, and the one set on a ticket to do', () => {
    // Agents on two harnesses: the brands show.
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'running', engine: 'codex' }), agent({ id: 'a2', ticketId: null })] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.enginesFound = ['claude', 'codex'];
    fakeBackend();
    const { container, unmount } = show(doing());
    expect(container.querySelector('.top [data-engine="codex"]')).toBeInTheDocument();
    unmount();
    const todo = show(ticket({ engine: 'codex' }));
    expect(todo.container.querySelector('.top [data-engine="codex"]')).toBeInTheDocument();
    todo.unmount();
    // On « Celui du projet », none on a ticket to do.
    const plain = show(ticket());
    expect(plain.container.querySelector('.top [data-engine]')).toBeNull();
  });
});

// A ticket that has its own model or effort (the MCP `start_ticket` sets them) says so on every card: they come before
// the Kanban's for its agent.
describe('TicketCard with a model or an effort of its own', () => {
  beforeEach(() => resetApp({ agents: [agent({ status: 'running' })] }));

  const own = (container: HTMLElement) => container.querySelector<HTMLElement>('.top .own');

  it('shows the model and the effort next to the key, with what they are in a tooltip and for a screen reader', () => {
    fakeBackend();
    const { container } = show(ticket({ model: 'opus', effort: 'max' }));
    const label = 'Modèle de ce ticket : Opus, effort Max (avant celui du Kanban)';
    const chip = screen.getByLabelText(label);
    expect(chip).toHaveTextContent('Opus · Max');
    expect(chip).toHaveAttribute('data-tip', label);
    expect(own(container)).toBe(chip);
  });

  it('shows the version Claude Code runs for the model once it told', () => {
    app.models = [{ value: 'opus', resolvedModel: 'claude-opus-5-5' }];
    fakeBackend();
    show(ticket({ model: 'opus' }));
    expect(screen.getByLabelText('Modèle de ce ticket : Opus 5.5 (avant celui du Kanban)')).toHaveTextContent('Opus 5.5');
  });

  it('shows only the one it has: the model, or the effort', () => {
    fakeBackend();
    const { unmount } = show(ticket({ model: 'sonnet' }));
    expect(screen.getByLabelText('Modèle de ce ticket : Sonnet (avant celui du Kanban)')).toHaveTextContent(/^Sonnet$/);
    unmount();
    show(ticket({ effort: 'xhigh' }));
    expect(screen.getByLabelText('Effort de ce ticket : Très élevé (avant celui du Kanban)')).toHaveTextContent(/^Très élevé$/);
  });

  it('shows nothing when the ticket follows the Kanban', () => {
    fakeBackend();
    const { container } = show(ticket());
    expect(own(container)).toBeNull();
    expect(screen.queryByLabelText(/de ce ticket/)).not.toBeInTheDocument();
  });

  it('shows it in every column, the first line of the card', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'done' })] });
    fakeBackend();
    for (const column of ['todo', 'doing', 'review', 'done'] as const) {
      const { container, unmount } = show(doing({ column, model: 'opus', effort: 'low' }));
      expect(own(container), column).toHaveTextContent('Opus · Bas');
      unmount();
    }
  });

  // Haiku takes no effort: its agent starts without one, so the card does not claim one (the ticket's model, else the
  // Kanban's, else the app's is the model its agent runs).
  it('leaves the effort out when the model of its agent takes none: its own Haiku', () => {
    fakeBackend();
    const { container } = show(ticket({ model: 'haiku', effort: 'max' }));
    const chip = screen.getByLabelText('Modèle de ce ticket : Haiku (avant celui du Kanban)');
    expect(chip).toHaveTextContent(/^Haiku$/);
    expect(chip).toHaveAttribute('data-tip', 'Modèle de ce ticket : Haiku (avant celui du Kanban)');
    expect(own(container)).toBe(chip);
    expect(screen.queryByLabelText(/effort/)).not.toBeInTheDocument();
  });

  it('shows nothing for an effort alone when the Kanban is on Haiku', () => {
    fakeBackend();
    const { container } = show(ticket({ effort: 'max' }), project({ board: board({ model: 'haiku' }) }));
    expect(own(container)).toBeNull();
    expect(screen.queryByLabelText(/de ce ticket/)).not.toBeInTheDocument();
  });

  it('shows nothing for an effort alone when the Kanban follows an account on Haiku', () => {
    app.settings.accounts = app.settings.accounts.map((a) => ({ ...a, defaultModel: 'haiku' }));
    fakeBackend();
    const { container } = show(ticket({ effort: 'max' }));
    expect(own(container)).toBeNull();
  });

  it('keeps the effort when the model of its agent takes one: its own model over a Kanban on Haiku, a Kanban on Opus', () => {
    fakeBackend();
    const { unmount } = show(ticket({ model: 'sonnet', effort: 'max' }), project({ board: board({ model: 'haiku' }) }));
    expect(screen.getByLabelText('Modèle de ce ticket : Sonnet, effort Max (avant celui du Kanban)')).toHaveTextContent('Sonnet · Max');
    unmount();
    show(ticket({ effort: 'high' }), project({ board: board({ model: 'opus' }) }));
    expect(screen.getByLabelText('Effort de ce ticket : Élevé (avant celui du Kanban)')).toHaveTextContent(/^Élevé$/);
  });

  it('knows a full Haiku id for Haiku too', () => {
    fakeBackend();
    const { container } = show(ticket({ model: 'claude-haiku-4-5', effort: 'low' }));
    expect(own(container)).toHaveTextContent(/^Haiku 4.5$/);
  });

  it('says it in English', () => {
    fakeBackend();
    setLang('en');
    show(ticket({ model: 'opus', effort: 'max' }));
    expect(screen.getByLabelText('This ticket’s model: Opus, effort Max (over the Kanban’s)')).toHaveTextContent('Opus · Max');
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
    expect(screen.getByText('Le fichier existe')).toHaveTextContent('Le fichier existe met');
    expect(within(screen.getByRole('list', { name: 'Criteria' })).getAllByRole('listitem')).toHaveLength(2);
    // The agent has not said what it does yet.
    expect(screen.getByText('Thinking')).toBeInTheDocument();
  });

  it('counts its images in English', () => {
    show(ticket({ images: [{ file: 'a1.png', name: 'capture.png', mediaType: 'image/png' }] }));
    expect(screen.getByRole('button', { name: 'DEM-1 Ajouter le fichier' })).toHaveAccessibleDescription('1 image attached');
  });

  it('counts its files in English, and says they go with the ticket when it is deleted', async () => {
    const files = [
      { name: 'budget.xlsx', file: 'ab12cd34/budget.xlsx', size: 2048 },
      { name: 'sources.zip', file: 'ef56ab78/sources.zip', size: 4096 },
    ];
    const image = { file: 'a1.png', name: 'capture.png', mediaType: 'image/png' };
    const { unmount } = show(ticket({ images: [image], files: files.slice(1) }));
    expect(screen.getByRole('button', { name: 'DEM-1 Ajouter le fichier' })).toHaveAccessibleDescription(
      '1 image attached, 1 file attached',
    );
    unmount();
    show(ticket({ files }));
    expect(screen.getByRole('button', { name: 'DEM-1 Ajouter le fichier' })).toHaveAccessibleDescription('2 files attached');
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    menu.open!.items.find((i) => i.label === 'Delete')!.onClick!();
    expect(app.modal).toMatchObject({ body: 'The ticket, its description and its 2 attachments are deleted.' });
  });

  it('labels the line its agent reported in English', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'running', progressLine: 'Writes the parser tests' })] });
    show(doing());
    expect(screen.getByText('Writes the parser tests')).toHaveTextContent('What the agent says it is doing Writes the parser tests');
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

  it('says why a ticket to do waits in English', () => {
    show(ticket());
    expect(screen.getByText('Picked up as soon as a slot is free')).toBeInTheDocument();
  });

  it('shows the cost of a ticket to test in dollars, with what it is for a screen reader, and its criteria met', () => {
    resetApp({ agents: [agent({ id: 'a1', ticketId: 't1', status: 'done', tokens: 1000, cost: 1.5 })] });
    const { container } = show(doing({ column: 'review' }));
    const sum = container.querySelector<HTMLElement>('.sum')!;
    expect(within(sum).getByText('2 loops · $1.50')).toBeInTheDocument();
    // The hidden label and the amount are apart for a screen reader.
    expect(sum.textContent).toContain('Ticket cost 2 loops · $1.50');
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
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Edit', 'Move to top', '', 'Share with the team', '', 'Delete']);
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

  it('says in English that the images of a deleted ticket go with it, imported or not', async () => {
    const images = [
      { file: 'a1.png', name: 'capture.png', mediaType: 'image/png' },
      { file: 'b2.gif', name: 'schema.gif', mediaType: 'image/gif' },
    ];
    const external = { service: 'jira' as const, id: 'ATL-1', key: 'ATL-1', container: 'ATL', url: 'https://a.test/ATL-1', error: null };
    const deleteIt = async () => {
      await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
      menu.open!.items.find((i) => i.label === 'Delete')!.onClick!();
    };
    const one = show(ticket({ images: images.slice(0, 1) }));
    await deleteIt();
    expect(app.modal).toMatchObject({ body: 'The ticket, its description and its image are deleted.' });
    one.unmount();
    const two = show(ticket({ images }));
    await deleteIt();
    expect(app.modal).toMatchObject({ body: 'The ticket, its description and its 2 images are deleted.' });
    two.unmount();
    show(ticket({ images, external }));
    await deleteIt();
    expect(app.modal).toMatchObject({
      body: 'The ticket, its description and its 2 images are deleted. It will no longer be imported from Jira.',
    });
  });

  it('writes the menu of a ticket under way in English', async () => {
    show(doing());
    await fireEvent.contextMenu(screen.getByRole('button', { name: /DEM-1/ }));
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Open agent', '', 'Share with the team', '', 'Delete']);
    menu.open!.items.find((i) => i.label === 'Delete')!.onClick!();
    expect(app.modal).toMatchObject({
      title: 'Delete ticket DEM-1?',
      body: 'Its agent is archived and its worktree deleted, unless it holds uncommitted changes or commits neither merged nor pushed.',
    });
  });

  it('says how many items of the progress are hidden, with the plural of English', () => {
    show(doing({ progress: ['One', 'Two', 'Three', 'Four'] }));
    expect(screen.getByText('+1')).toHaveAttribute('data-tip', '1 more item');
  });
});

describe('TicketCard: the brand of its agent’s harness (ESC-29)', () => {
  it('shows beside its agent once agents run on two harnesses', () => {
    resetApp({ agents: [agent({ id: 'a1', engine: 'codex' }), agent({ id: 'a2' })] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    fakeBackend();
    show(doing());
    expect(screen.getByRole('img', { name: 'Harness : Harness nu' })).toBeInTheDocument();
  });
});

// A ticket set on an account (ESC-153) says it on its card, and that its agent waits for that account alone.
describe('TicketCard with an account of its own', () => {
  beforeEach(() => {
    resetApp({ agents: [agent({ status: 'running' })] });
    app.settings = {
      ...app.settings,
      accounts: [{ id: 'pro', name: 'Pro', engine: 'claude', configDir: 'C:\\pro', claudePath: '', active: true }],
    };
  });

  it('names the account next to the key, nothing on « Auto »', () => {
    fakeBackend();
    const { unmount } = show(ticket({ account: 'pro' }));
    const chip = screen.getByLabelText('Compte de ce ticket : Pro (ses agents y restent)');
    expect(chip).toHaveTextContent(/^Pro$/);
    unmount();
    show(ticket());
    expect(screen.queryByLabelText(/^Compte de ce ticket/)).toBeNull();
  });

  it('says its agent waits for the quota of its account, and when it resumes', () => {
    const at = new Date();
    at.setHours(15, 30, 0, 0);
    resetApp({ agents: [agent({ id: 'a1', status: 'idle', resumeAt: at.getTime() })] });
    app.settings = {
      ...app.settings,
      accounts: [{ id: 'pro', name: 'Pro', engine: 'claude', configDir: 'C:\\pro', claudePath: '', active: true }],
    };
    fakeBackend();
    show(doing({ account: 'pro' }));
    expect(screen.getByText('En attente du quota de Pro, reprise à 15:30')).toBeInTheDocument();
  });

  it('says why it went back to « Auto »', () => {
    fakeBackend();
    show(ticket({ accountLost: 'Le compte Pro n’existe plus : repassé en « Auto ».' }));
    expect(screen.getByRole('status')).toHaveTextContent('Le compte Pro n’existe plus : repassé en « Auto ».');
  });

  it('shows a ticket « En revue » with its pull request, its state, its CI, its approvals and its conflict (ESC-59)', async () => {
    fakeBackend();
    const pr = ticketPr({ approvals: 1, checks: 'failure', checksTotal: 3, checksFailed: 1, openThreads: 2, mergeable: false });
    const { container } = show(ticket({ column: 'pr', agentId: 'a1', pr, outcome: '⇡ PR #12 → main' }));
    const line = container.querySelector('[data-ticket-pr="12"]')!;
    expect(line).toHaveTextContent('PR #12');
    expect(line).toHaveTextContent('approuvée');
    expect(line).toHaveTextContent('1/3 jobs en échec');
    expect(container.querySelector('.prfacts')).toHaveTextContent('1 approbation · 2 fils ouverts · conflit avec main');
    expect(container.querySelector('.prfacts .conflicting')).toHaveTextContent('conflit avec main');
    // Its criteria are behind it now: the card stays short.
    expect(screen.queryByRole('list', { name: 'Critères' })).toBeNull();
    // Its number opens it in the browser, not the ticket.
    await userEvent.click(screen.getByRole('button', { name: /Ouvrir la PR #12 dans le navigateur/ }));
    expect(app.modal).toBeNull();
  });

  it('says on the card that its host cannot be read, the ticket staying in review', () => {
    fakeBackend();
    app.now = 1_000_000_000;
    const pr = ticketPr({ error: 'GitLab injoignable : délai dépassé', errorSince: app.now - 3 * 60_000, word: 'MR' });
    const { container } = show(ticket({ column: 'pr', pr }));
    expect(container.querySelector('[data-pr-error]')).toHaveTextContent('GitLab injoignable : délai dépassé · depuis 3 min');
    expect(container.querySelector('[data-ticket-pr]')).toHaveTextContent('MR #12');
  });

  it('offers « Remettre à faire » and « Terminer quand même » once its pull request was closed without a merge', async () => {
    const backend = fakeBackend();
    const pr = ticketPr({ state: 'closed', closed: true });
    show(ticket({ column: 'pr', pr, blocked: 'PR #12 fermée sans merge' }));
    expect(screen.getByText('PR #12 fermée sans merge')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Terminer quand même' }));
    expect(backend.called('ticket_finish_anyway')).toEqual([{ cmd: 'ticket_finish_anyway', args: { id: 't1' } }]);
    await userEvent.click(screen.getByRole('button', { name: 'Remettre à faire' }));
    expect(backend.called('ticket_pr_back_to_todo')).toEqual([{ cmd: 'ticket_pr_back_to_todo', args: { id: 't1' } }]);
  });

  it('opens the preview of its pull request from a card « En revue » (ESC-67)', async () => {
    const backend = fakeBackend();
    const pr = ticketPr({ preview: 'https://demo-git-fix.example.vercel.app' });
    show(ticket({ column: 'pr', pr }));
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir l’aperçu' }));
    expect(backend.called('open_url')[0].args.url).toBe('https://demo-git-fix.example.vercel.app');
    expect(app.modal).toBeNull();
  });

  it('has no preview to open while its pull request has none', () => {
    fakeBackend();
    show(ticket({ column: 'pr', pr: ticketPr() }));
    expect(screen.queryByRole('button', { name: 'Ouvrir l’aperçu' })).toBeNull();
  });

  it('says in which environments a done ticket is deployed, each opening its address (ESC-67)', async () => {
    const backend = fakeBackend();
    const deployed = ticket({
      column: 'done',
      outcome: '⤵ Mergé dans main',
      outcomeCommit: 'abc1234',
      environments: [
        { name: 'staging', url: 'https://staging.example.com', deployed: true },
        { name: 'production', url: 'https://example.com', deployed: false },
      ],
    });
    const { container, unmount } = show(deployed);
    expect(container.querySelector('[data-ticket-envs]')).toHaveTextContent('staging · production');
    // Deployed: a check, in green; not yet: a clock, in amber.
    expect(container.querySelector('.dep-env.deployed [data-icon="check"]')).not.toBeNull();
    expect(container.querySelector('.dep-env:not(.deployed) [data-icon="clock"]')).not.toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'production : pas encore déployé' }));
    expect(backend.called('open_url')[0].args.url).toBe('https://example.com');
    unmount();
    setLang('en');
    show(deployed);
    expect(screen.getByRole('button', { name: 'staging: deployed' })).toBeInTheDocument();
    setLang('fr');
  });

  it('names no environment while none is known', () => {
    fakeBackend();
    const { container } = show(ticket({ column: 'done', outcome: '⤵ Mergé dans main' }));
    expect(container.querySelector('[data-ticket-envs]')).toBeNull();
  });

  it('offers « Merger » only approved and green, with a strategy the repository allows, and says what merged (ESC-62)', async () => {
    const backend = fakeBackend({ ticket_pr_merge: () => 'merged' });
    const green = {
      approvals: 1,
      checks: 'success' as const,
      mergeable: true,
      mergeOptions: { strategies: ['merge' as const, 'rebase' as const], autoMerge: false },
    };
    // Approved but red, or green but not approved: no « Merger ».
    show(ticket({ column: 'pr', pr: ticketPr({ ...green, checks: 'failure' }) }));
    expect(screen.queryByRole('button', { name: /^Merger/ })).toBeNull();
    cleanup();
    show(ticket({ column: 'pr', pr: ticketPr({ ...green, approvals: 0 }) }));
    expect(screen.queryByRole('button', { name: /^Merger/ })).toBeNull();
    cleanup();
    // The Kanban's squash is not allowed: the first strategy the repository allows.
    show(ticket({ column: 'pr', pr: ticketPr(green) }));
    await userEvent.click(screen.getByRole('button', { name: 'Merger (merge commit)' }));
    expect(backend.called('ticket_pr_merge')).toEqual([{ cmd: 'ticket_pr_merge', args: { id: 't1', whenGreen: false } }]);
    expect(app.toasts.at(-1)?.text).toBe('PR #12 mergée');
    expect(screen.queryByRole('button', { name: 'Merger quand c’est vert' })).toBeNull();
    cleanup();
    // It allows none: said, no button.
    const { container } = show(ticket({ column: 'pr', pr: ticketPr({ ...green, mergeOptions: { strategies: [], autoMerge: false } }) }));
    expect(screen.queryByRole('button', { name: /^Merger/ })).toBeNull();
    expect(container.querySelector('[data-pr-no-strategy]')).toHaveTextContent('aucune des façons de merger');
  });

  it('offers « Merger quand c’est vert » while the CI runs: the host’s auto-merge, else Escouade, said (ESC-62)', async () => {
    const backend = fakeBackend({ ticket_pr_merge: () => 'escouade' });
    const running = { approvals: 1, checks: 'pending' as const, mergeOptions: { strategies: ['squash' as const], autoMerge: false } };
    show(ticket({ column: 'pr', pr: ticketPr({ ...running, kind: 'bitbucket', host: 'bitbucket.org' }) }));
    expect(screen.queryByRole('button', { name: /^Merger \(/ })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Merger quand c’est vert' }));
    expect(backend.called('ticket_pr_merge')).toEqual([{ cmd: 'ticket_pr_merge', args: { id: 't1', whenGreen: true } }]);
    expect(app.toasts.at(-1)?.text).toContain('Pas d’auto-merge sur Bitbucket');
    cleanup();
    // Escouade waits: said, with « Ne plus attendre ».
    const { container } = show(ticket({ column: 'pr', pr: ticketPr({ ...running, mergeWhenGreen: 'squash' }) }));
    expect(container.querySelector('[data-pr-wait]')).toHaveTextContent('Escouade la merge (squash) dès qu’elle est approuvée et verte');
    expect(screen.queryByRole('button', { name: 'Merger quand c’est vert' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Ne plus attendre' }));
    expect(backend.called('ticket_pr_stop_waiting')).toEqual([{ cmd: 'ticket_pr_stop_waiting', args: { id: 't1' } }]);
    cleanup();
    // The host's auto-merge on.
    const c2 = show(ticket({ column: 'pr', pr: ticketPr({ ...running, autoMerge: true }) })).container;
    expect(c2.querySelector('[data-pr-wait]')).toHaveTextContent('Auto-merge de GitHub activé');
    cleanup();
    // A merge that did not go is said.
    const c3 = show(ticket({ column: 'pr', pr: ticketPr({ ...running, mergeNote: 'Merge refusé : checks requis' }) })).container;
    expect(c3.querySelector('[data-pr-merge-note]')).toHaveTextContent('Merge refusé : checks requis');
  });

  it('offers « L’agent corrige » on a failed CI, says a CI without log, and shows its agent at work (ESC-60)', async () => {
    const backend = fakeBackend();
    const failed = [
      { name: 'test', url: 'https://github.com/acme/demo/runs/55', log: true },
      { name: 'ci/jenkins', url: 'https://jenkins.example.com/7', log: false },
    ];
    const pr = ticketPr({ checks: 'failure', checksTotal: 2, checksFailed: 2, failed });
    const { container, unmount } = show(ticket({ column: 'pr', agentId: 'a1', pr }));
    expect(container.querySelector('[data-pr-nolog]')).toHaveTextContent('ci/jenkins : CI tierce, sans journal');
    await userEvent.click(screen.getByRole('button', { name: "L'agent corrige" }));
    expect(backend.called('ticket_pr_fix_ci')).toEqual([{ cmd: 'ticket_pr_fix_ci', args: { id: 't1' } }]);
    unmount();
    // Under way: no button, its agent fixes it.
    const at = show(ticket({ column: 'pr', agentId: 'a1', pr: { ...pr, fixing: 1, fixes: 1 } }));
    expect(at.container.querySelector('[data-pr-fixing]')).toHaveTextContent("L'agent corrige la CI…");
    expect(screen.queryByRole('button', { name: "L'agent corrige" })).toBeNull();
    at.unmount();
    // Green, or without an agent: nothing to offer.
    show(ticket({ column: 'pr', agentId: 'a1', pr: ticketPr({ checks: 'success' }) }));
    show(ticket({ column: 'pr', pr }));
    expect(screen.queryByRole('button', { name: "L'agent corrige" })).toBeNull();
  });

  it('counts the new review comments and gives them to its agent on a click (ESC-61)', async () => {
    const backend = fakeBackend();
    const pr = ticketPr({ changesRequested: true, openThreads: 2, feedback: 2 });
    const { container, unmount } = show(ticket({ column: 'pr', agentId: 'a1', pr }), project({ board: board({ autopilot: true }) }));
    expect(container.querySelector('[data-pr-feedback]')).toHaveTextContent('2 retours de relecture');
    // The autopilot gives it nothing by itself: the button waits.
    expect(backend.called('ticket_pr_fix_reviews')).toEqual([]);
    await userEvent.click(screen.getByRole('button', { name: "L'agent corrige les retours" }));
    expect(backend.called('ticket_pr_fix_reviews')).toEqual([{ cmd: 'ticket_pr_fix_reviews', args: { id: 't1' } }]);
    unmount();
    const at = show(ticket({ column: 'pr', agentId: 'a1', pr: { ...pr, feedback: 0, fixing: 1, fixKind: 'reviews' } }));
    expect(at.container.querySelector('[data-pr-fixing]')).toHaveTextContent("L'agent corrige les retours…");
    expect(screen.queryByRole('button', { name: "L'agent corrige les retours" })).toBeNull();
    at.unmount();
    // Every comment given already: nothing to offer.
    show(ticket({ column: 'pr', agentId: 'a1', pr: { ...pr, feedback: 0 } }));
    expect(screen.queryByRole('button', { name: "L'agent corrige les retours" })).toBeNull();
  });

  it('offers « L’agent met à jour la branche » on a conflict with its target, by a merge or a rebase (ESC-61)', async () => {
    const backend = fakeBackend();
    const pr = ticketPr({ mergeable: false });
    const { unmount } = show(ticket({ column: 'pr', agentId: 'a1', pr }));
    const button = screen.getByRole('button', { name: "L'agent met à jour la branche" });
    await userEvent.click(button);
    expect(backend.called('ticket_pr_update_branch')).toEqual([{ cmd: 'ticket_pr_update_branch', args: { id: 't1' } }]);
    unmount();
    const at = show(
      ticket({ column: 'pr', agentId: 'a1', pr: { ...pr, fixing: 1, fixKind: 'rebase' } }),
      project({ board: board({ prUpdate: 'rebase' }) }),
    );
    expect(at.container.querySelector('[data-pr-fixing]')).toHaveTextContent("L'agent rebase la branche sur main…");
    expect(screen.queryByRole('button', { name: "L'agent met à jour la branche" })).toBeNull();
    at.unmount();
    show(ticket({ column: 'pr', agentId: 'a1', pr: ticketPr({ mergeable: true }) }));
    expect(screen.queryByRole('button', { name: "L'agent met à jour la branche" })).toBeNull();
  });

  it('says why the last fix did not go, and that the autopilot spent its loops (ESC-60)', () => {
    fakeBackend();
    const pr = ticketPr({ checks: 'failure', fixes: 3, awaitingCi: true, fixError: "L'agent n'a rien commité : la CI reste en échec" });
    const { container } = show(ticket({ column: 'pr', agentId: 'a1', maxLoops: 3, pr }), project({ board: board({ autopilot: true }) }));
    expect(container.querySelector('[data-pr-fix-error]')).toHaveTextContent("L'agent n'a rien commité");
    expect(container).toHaveTextContent('Boucles épuisées (3/3) : le pilote auto ne la renvoie plus');
    expect(screen.getByRole('button', { name: "L'agent corrige" })).toBeInTheDocument();
  });
});
