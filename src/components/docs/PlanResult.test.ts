import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { docsStore } from '../../lib/docs.svelte';
import { planProgress, ticketStarted } from '../../lib/planning';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp, ticket } from '../../test/ipc';
import type { DocsPlan, Ticket } from '../../lib/types';
import PlanResult from './PlanResult.svelte';

// The card of what « Planifier » made (ESC-291): its pill, its note and « Annuler » follow where its tickets stand (none,
// some, all started; undone; removed; nothing made); all started, it folds into its keys; closed, it stays closed.

const made = [
  { id: 't1', key: 'DEM-7', title: 'Implémenter : Relances' },
  { id: 't2', key: 'DEM-8', title: 'Écran des relances' },
  { id: 't3', key: 'DEM-9', title: 'Reçu fiscal' },
];
const plan = (o: Partial<DocsPlan> = {}): DocsPlan => ({
  id: 'pl1',
  at: Date.now() - 60_000,
  pages: [],
  decisions: [],
  made,
  skipped: [],
  cancelled: false,
  ...o,
});
const tk = (id: string, key: string, o: Partial<Ticket> = {}) => ticket({ id, key, ...o });
const todo = () => [tk('t1', 'DEM-7'), tk('t2', 'DEM-8'), tk('t3', 'DEM-9')];

function show(p: DocsPlan, tickets: Ticket[]) {
  resetApp({ tickets });
  fakeBackend({ docs_plan_cancel: () => ({ removed: [], kept: [] }) });
  render(PlanResult, { project: app.projects[0], plan: p });
  const card = document.querySelector('section[data-plan]') as HTMLElement;
  return {
    card,
    pill: () => card.querySelector('[data-plan-state]')?.textContent ?? null,
    note: () => card.querySelector('[data-plan-note]')?.textContent ?? null,
    cancel: () => within(card).queryByRole('button', { name: 'Annuler' }),
  };
}

describe('where the tickets of a planning stand', () => {
  it('counts a ticket started as « Annuler » does: out of « À faire », started or with an agent', () => {
    expect(ticketStarted(ticket())).toBe(false);
    expect(ticketStarted(ticket({ column: 'doing' }))).toBe(true);
    expect(ticketStarted(ticket({ startedAt: 5 }))).toBe(true);
    expect(ticketStarted(ticket({ agentId: 'a1' }))).toBe(true);
  });

  it('tells none, some, all started, undone, removed and nothing made apart', () => {
    const byId = (list: Ticket[]) => Object.fromEntries(list.map((x) => [x.id, x]));
    expect(planProgress(plan({ made: [] }), {}).state).toBe('none');
    expect(planProgress(plan(), byId(todo())).state).toBe('todo');
    const some = planProgress(plan(), byId([tk('t1', 'DEM-7', { column: 'doing' }), tk('t2', 'DEM-8'), tk('t3', 'DEM-9')]));
    expect(some.state).toBe('partial');
    expect(some.toCancel.map((m) => m.key)).toEqual(['DEM-8', 'DEM-9']);
    const all = planProgress(plan(), byId([tk('t1', 'DEM-7', { column: 'review' }), tk('t3', 'DEM-9', { agentId: 'a1' })]));
    expect(all.state).toBe('started');
    expect(all.toCancel).toEqual([]);
    expect(planProgress(plan(), {}).state).toBe('gone');
    const undone = planProgress(plan({ cancelled: true }), byId([tk('t1', 'DEM-7', { column: 'doing' })]));
    expect(undone.state).toBe('cancelled');
    expect(undone.toCancel).toEqual([]);
  });
});

describe('the card of « Planifier »', () => {
  beforeEach(() => {
    localStorage.clear();
    docsStore.closedPlans = [];
  });

  it('none started: « À lancer », the note of the autopilot, and « Annuler » for all of them', () => {
    const c = show(plan(), todo());
    expect(c.card).toHaveTextContent('3 tickets créés');
    expect(c.pill()).toBe('À lancer');
    expect(c.note()).toBe('Pas encore lancés : le pilote auto et les règles du Kanban s’appliquent.');
    expect(c.cancel()).not.toBeNull();
    expect(c.card.querySelector('[data-made="DEM-7"] .col')).toHaveTextContent('À faire');
  });

  it('some started: the count, a note naming those left, and « Annuler » that removes only them', async () => {
    const c = show(plan(), [tk('t1', 'DEM-7', { column: 'doing' }), tk('t2', 'DEM-8', { column: 'done' }), tk('t3', 'DEM-9')]);
    expect(c.pill()).toBe('2 lancés sur 3');
    expect(c.note()).toBe('DEM-9 encore à faire : « Annuler » ne retire que lui.');
    expect(c.card.querySelector('[data-made="DEM-7"] .col')).toHaveTextContent('En cours');
    expect(c.cancel()).not.toBeNull();
    await userEvent.hover(c.cancel()!);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Retirer DEM-9, pas encore lancé');
  });

  it('all started: no note, no « Annuler », the list folded into its keys and unfolded on demand', async () => {
    const c = show(plan(), [
      tk('t1', 'DEM-7', { column: 'doing' }),
      tk('t2', 'DEM-8', { column: 'review' }),
      tk('t3', 'DEM-9', { column: 'done' }),
    ]);
    expect(c.pill()).toBe('Tous lancés');
    expect(c.note()).toBeNull();
    expect(c.cancel()).toBeNull();
    expect(c.card.querySelectorAll('[data-made]')).toHaveLength(0);
    expect(c.card.querySelector('[data-plan-keys]')).toHaveTextContent('DEM-7DEM-8DEM-9');
    await userEvent.click(within(c.card).getByRole('button', { name: 'Voir les tickets' }));
    expect(c.card.querySelectorAll('[data-made]')).toHaveLength(3);
    await userEvent.click(within(c.card).getByRole('button', { name: 'Replier les tickets' }));
    expect(c.card.querySelectorAll('[data-made]')).toHaveLength(0);
  });

  it('a single ticket started says « Lancé »', () => {
    const c = show(plan({ made: [made[0]] }), [tk('t1', 'DEM-7', { startedAt: 3 })]);
    expect(c.card).toHaveTextContent('1 ticket créé');
    expect(c.pill()).toBe('Lancé');
  });

  it('undone: the tickets removed struck out, those kept said, no « Annuler »', () => {
    const c = show(plan({ cancelled: true }), [tk('t1', 'DEM-7', { column: 'doing' })]);
    expect(c.pill()).toBe('Annulé');
    expect(c.note()).toBe('Annulé : DEM-7, déjà lancé, est resté.');
    expect(c.cancel()).toBeNull();
    expect(c.card.querySelector('[data-made="DEM-8"] button')).toBeDisabled();
    expect(c.card.querySelector('[data-made="DEM-8"] .col')).toHaveTextContent('Retiré');
    const none = show(plan({ id: 'pl2', cancelled: true }), []);
    expect(none.note()).toBe('Annulé : les changements sont de nouveau à planifier.');
  });

  it('removed from the board by hand: no note, no « Annuler »', () => {
    const c = show(plan(), []);
    expect(c.pill()).toBe('Retirés du Kanban');
    expect(c.note()).toBeNull();
    expect(c.cancel()).toBeNull();
  });

  it('nothing made: what the board covers already, without a list', () => {
    const c = show(plan({ made: [], skipped: [{ title: 'Archive', key: 'DEM-2' }] }), []);
    expect(c.card).toHaveTextContent('Planifié');
    expect(c.note()).toBe('Aucun ticket nouveau : le Kanban couvre déjà ces changements.');
    expect(c.card.querySelector('[data-skipped]')).toHaveTextContent('Déjà au Kanban : DEM-2');
    expect(c.pill()).toBeNull();
    expect(c.cancel()).toBeNull();
  });

  it('a long title is read whole in its tooltip', async () => {
    const long = 'Relance automatique des adhésions qui arrivent à échéance dans les trente jours, avec un e-mail puis un SMS';
    const c = show(plan({ made: [{ id: 't1', key: 'DEM-7', title: long }] }), [tk('t1', 'DEM-7')]);
    await userEvent.hover(within(c.card).getByRole('button', { name: 'Ouvrir DEM-7 au Kanban' }));
    expect(await screen.findByRole('tooltip')).toHaveTextContent(`DEM-7 · ${long}`);
  });

  it('closed, it stays closed from one start to the next', async () => {
    const c = show(plan(), todo());
    await userEvent.click(within(c.card).getByRole('button', { name: 'Fermer' }));
    expect(docsStore.closedPlans).toEqual(['pl1']);
    expect(JSON.parse(localStorage.getItem('escouade.docs.closedPlans') ?? '[]')).toEqual(['pl1']);
  });
});
