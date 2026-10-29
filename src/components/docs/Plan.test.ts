import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { docsStore } from '../../lib/docs.svelte';
import { canPlan, pageTitle, subjectTickets, unplannedCount } from '../../lib/planning';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import type { Discussion, DocsPlan, DocsTree } from '../../lib/types';
import TicketCard from '../board/TicketCard.svelte';
import DocsView from './DocsView.svelte';

// « Planifier » (ESC-262): on a discussion whose changes were poured, and on top of the Specification while changes have
// no tickets; what the planner made, with « Annuler »; the subject's card shows its tickets, a ticket its pages.

const info = (path: string, title: string, o: object = {}) => ({
  path,
  title,
  compared: '',
  tickets: [],
  decisions: [],
  gaps: 0,
  draft: false,
  ...o,
});

const PLAN: DocsPlan = {
  id: 'pl1',
  at: Date.now() - 60_000,
  pages: ['adherent/relances'],
  decisions: [3],
  made: [
    { id: 't1', key: 'DEM-7', title: 'Implémenter : Relances' },
    { id: 't2', key: 'DEM-8', title: 'Écran des relances' },
  ],
  skipped: [{ title: 'Archive', key: 'DEM-2' }],
  cancelled: false,
};

const tree = (planning: DocsTree['planning']): DocsTree => ({
  branch: '',
  pages: 2,
  subjects: [
    {
      name: 'adherent',
      title: 'Espace adhérent',
      pages: [
        info('adherent/relances', 'Relances', { tickets: ['DEM-7', 'DEM-8'] }),
        info('adherent/recu', 'Reçu', { tickets: ['DEM-7'] }),
      ],
    },
  ],
  existingSpec: [],
  planning,
});

const disc = (o: Partial<Discussion> = {}): Discussion => ({
  kind: 'discussion',
  subject: 'Relances',
  pages: [],
  decisions: [],
  ...o,
});

function backend(t: DocsTree, over: Record<string, (a: any) => unknown> = {}) {
  return fakeBackend({
    docs_spec_tree: () => t,
    docs_decisions: () => [],
    docs_drive: () => ({ folders: [], files: [] }),
    docs_recent: () => [],
    docs_pending: () => ({ files: [], count: 0, codeCommits: 0, conflict: null }),
    get_conversation: () => [],
    ...over,
  });
}

describe('the pure helpers of « Planifier »', () => {
  it('counts the changes without tickets, offers planning once poured, and names pages and tickets', () => {
    expect(unplannedCount(null)).toBe(0);
    expect(unplannedCount({ pages: ['a/b', 'a/c'], decisions: [3], running: false })).toBe(3);
    expect(canPlan(disc())).toBe(false);
    expect(canPlan(disc({ poured: { at: 5, pages: ['a/b'], decisions: [] } }))).toBe(true);
    expect(canPlan(disc({ poured: { at: 5, pages: ['a/b'], decisions: [] }, planned: { ...PLAN, at: 6 } }))).toBe(false);
    // Poured again after planning: to plan again.
    expect(canPlan(disc({ poured: { at: 9, pages: ['a/b'], decisions: [] }, planned: { ...PLAN, at: 6 } }))).toBe(true);
    const t = tree(null);
    expect(pageTitle('adherent/relances', t)).toBe('Relances');
    expect(pageTitle('autre/sans-titre', t)).toBe('sans-titre');
    expect(subjectTickets(t.subjects[0].pages)).toEqual(['DEM-7', 'DEM-8']);
  });
});

describe('« Planifier »', () => {
  beforeEach(() => {
    docsStore.tree = {};
    docsStore.planning = {};
  });

  it('is on top of the Specification while changes have no tickets, and tells what the planner made', async () => {
    resetApp({ tickets: [ticket({ id: 't1', key: 'DEM-7', column: 'todo' }), ticket({ id: 't2', key: 'DEM-8', column: 'done' })] });
    app.openDocs('p1', 'spec');
    let t = tree({ pages: ['adherent/relances', 'adherent/recu'], decisions: [3], running: false });
    const b = backend(t, {
      docs_spec_tree: () => t,
      docs_plan: () => {
        t = tree({ pages: [], decisions: [], last: PLAN, running: false });
        return PLAN;
      },
    });
    render(DocsView, { project: app.projects[0] });
    const bar = await waitFor(() => {
      const el = document.querySelector('[data-plan-bar]') as HTMLElement;
      expect(el).not.toBeNull();
      return el;
    });
    expect(bar).toHaveTextContent('3 changements sans tickets');
    // The subject's card shows its tickets.
    const card = document.querySelector('[data-subject-card="adherent"]') as HTMLElement;
    expect(card.querySelector('[data-subject-tickets]')).toHaveTextContent('DEM-7DEM-8');
    await userEvent.click(within(bar).getByRole('button', { name: 'Planifier' }));
    await waitFor(() => expect(b.called('docs_plan')).toHaveLength(1));
    expect(b.called('docs_plan')[0].args).toEqual({ id: 'p1', agent: null });
    await waitFor(() => expect(app.toasts.map((x) => x.text)).toContain('2 tickets créés dans « À faire ».'));
    // The bar is gone; the plan is told on the map.
    await waitFor(() => expect(document.querySelector('[data-plan-bar]')).toBeNull());
    const plan = document.querySelector('[data-plan="pl1"]') as HTMLElement;
    expect(plan).toHaveTextContent(/2 tickets créés/);
    expect(plan).toHaveTextContent('Déjà au Kanban : DEM-2');
    // DEM-8 is done: one started, « Annuler » for DEM-7 only.
    expect(plan.querySelector('[data-plan-state]')).toHaveTextContent('1 lancé sur 2');
    expect(plan.querySelector('[data-plan-note]')).toHaveTextContent('DEM-7 encore à faire');
    expect(plan.querySelector('[data-made="DEM-7"]')).toHaveTextContent('À faire');
    await userEvent.click(within(plan).getByRole('button', { name: 'Ouvrir DEM-7 au Kanban' }));
    expect(app.board.p1).toBe(true);
  });

  it('runs on a discussion whose changes were poured, and « Annuler » removes its tickets', async () => {
    const poured = disc({ poured: { at: 5, pages: ['adherent/relances'], decisions: [3] } });
    resetApp({ agents: [agent({ id: 'd1', status: 'done', discussion: poured })], tickets: [ticket({ id: 't1', key: 'DEM-7' })] });
    app.openDiscussion('p1', 'd1');
    const b = backend(tree(null), {
      docs_plan: () => {
        app.agents.d1 = { ...app.agents.d1, discussion: { ...poured, planned: { ...PLAN, discussion: 'd1', at: 10 } } };
        return PLAN;
      },
      docs_plan_cancel: () => ({ removed: ['DEM-8'], kept: ['DEM-7'] }),
    });
    render(DocsView, { project: app.projects[0] });
    // Nothing left to pour: « Planifier » is the main action.
    const plan = await screen.findByRole('button', { name: 'Planifier' });
    expect(plan).toHaveClass('primary');
    await userEvent.click(plan);
    await waitFor(() => expect(b.called('docs_plan')[0]?.args).toEqual({ id: 'p1', agent: 'd1' }));
    // What it made, under the discussion; planned, no « Planifier » until something else is poured.
    const panel = await screen.findByRole('complementary', { name: 'Ce qui ira dans la spec' });
    const made = await within(panel).findByRole('region', { name: 'Tickets' });
    expect(made).toHaveTextContent('Implémenter : Relances');
    expect(screen.queryByRole('button', { name: 'Planifier' })).toBeNull();
    await userEvent.click(within(made).getByRole('button', { name: 'Annuler' }));
    await waitFor(() => expect(b.called('docs_plan_cancel')[0]?.args).toEqual({ id: 'p1', plan: 'pl1' }));
    await waitFor(() =>
      expect(app.toasts.map((x) => x.text)).toContain(
        '1 ticket retiré, les changements sont de nouveau à planifier. DEM-7 déjà lancé, gardé.',
      ),
    );
  });

  it('is not offered on a discussion that poured nothing', async () => {
    resetApp({ agents: [agent({ id: 'd1', discussion: disc() })] });
    app.openDiscussion('p1', 'd1');
    backend(tree(null));
    render(DocsView, { project: app.projects[0] });
    await screen.findByText(/Rien pour l’instant/);
    expect(screen.queryByRole('button', { name: 'Planifier' })).toBeNull();
  });

  it('shows on a ticket the pages it implements, each opening in the Docs tab', async () => {
    resetApp();
    docsStore.tree.p1 = tree(null);
    backend(tree(null));
    const tk = ticket({ id: 't1', key: 'DEM-7', pages: ['adherent/relances', 'autre/page'] });
    app.tickets.t1 = tk;
    render(TicketCard, { ticket: tk, project: project(), queueIndex: 0, busyCount: 1, quota: null, onedit: () => {} });
    const line = document.querySelector('[data-ticket-pages]') as HTMLElement;
    expect(line).toHaveTextContent('Relances, page');
    await userEvent.click(within(line).getByRole('button', { name: 'Ouvrir la page Relances de la spec' }));
    expect(app.docs.p1).toBe(true);
    expect(app.docsPage.p1).toBe('adherent/relances');
  });
});
