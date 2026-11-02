import { render, screen, within } from '@testing-library/svelte';
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

import { flushSync } from 'svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { branchCi, fakeBackend, resetApp, ticket } from '../../test/ipc';
import Board from './Board.svelte';

const col = (name: string) => screen.getByRole('region', { name });

describe('the CI of the target and of the pushed branches on the board (ESC-63)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({
      tickets: [
        ticket(),
        ticket({
          id: 't2',
          key: 'DEM-2',
          title: 'Poussé',
          column: 'done',
          outcome: '⇡ Poussé sur ticket/dem-2',
          pushed: branchCi({ branch: 'ticket/dem-2', checks: 'pending', redSince: null }),
        }),
      ],
    });
    app.enginesFound = ['claude'];
    // The pills say since when from the window's clock.
    app.now = Date.now();
    fakeBackend();
  });

  it('shows no pill without a host Escouade reads, nor for a target without any CI', () => {
    render(Board, { project: app.projects[0] });
    expect(document.querySelector('[data-ci-pill="target"]')).toBeNull();
    app.targetCi = { p1: branchCi({ checks: null, redSince: null }) };
    flushSync();
    expect(document.querySelector('[data-ci-pill="target"]')).toBeNull();
  });

  it('shows the target’s CI in the header, red since when, and opens its jobs on a click', async () => {
    app.targetCi = { p1: branchCi() };
    render(Board, { project: app.projects[0] });
    const pill = screen.getByRole('button', { name: /^CI de main : 1\/3 jobs en échec/ });
    expect(pill).toHaveTextContent('main ✗ depuis 2 h');
    // Off by default: the autopilot is not paused, the tickets wait for a place as before.
    expect(document.querySelector('[data-red-target]')).toBeNull();
    expect(screen.getByText('2 places libres')).toBeInTheDocument();
    expect(within(col('À faire')).getByText("Pris dès qu'une place se libère")).toBeInTheDocument();
    await userEvent.click(pill);
    expect(app.modal).toEqual({ kind: 'ci', of: { kind: 'target', id: 'p1' }, projectId: 'p1' });
  });

  it('pauses the autopilot while the target is red when the board asks it, « Lancer » still there', async () => {
    app.targetCi = { p1: branchCi() };
    app.projects[0].board.pauseOnRedTarget = true;
    render(Board, { project: app.projects[0] });
    const band = document.querySelector('[data-red-target]')!;
    expect(band).toHaveTextContent('Pilote auto en pause : la CI de main est rouge depuis 2 h');
    // The band says why: no free places beside it.
    expect(screen.queryByText('2 places libres')).toBeNull();
    const todo = col('À faire');
    expect(within(todo).getByText('En attente : CI de la cible rouge')).toBeInTheDocument();
    expect(within(todo).getByRole('button', { name: 'Lancer' })).toBeInTheDocument();
    await userEvent.click(within(band as HTMLElement).getByRole('button', { name: 'Voir les jobs' }));
    expect(app.modal).toEqual({ kind: 'ci', of: { kind: 'target', id: 'p1' }, projectId: 'p1' });
  });

  it('lifts the pause once the target is green again', async () => {
    app.targetCi = { p1: branchCi() };
    app.projects[0].board.pauseOnRedTarget = true;
    render(Board, { project: app.projects[0] });
    expect(document.querySelector('[data-red-target]')).not.toBeNull();
    app.targetCi = { p1: branchCi({ checks: 'success', redSince: null }) };
    await vi.waitFor(() => expect(document.querySelector('[data-red-target]')).toBeNull());
    expect(screen.getByRole('button', { name: /^CI de main/ })).toHaveTextContent('main ✓');
  });

  it('shows the CI of a branch pushed without a pull request on its card « Terminé »', async () => {
    render(Board, { project: app.projects[0] });
    const done = col('Terminé');
    const pill = within(done).getByRole('button', { name: /^CI de ticket\/dem-2/ });
    expect(pill).toHaveTextContent('ticket/dem-2 · CI en cours');
    await userEvent.click(pill);
    expect(app.modal).toEqual({ kind: 'ci', of: { kind: 'pushed', id: 't2' }, projectId: 'p1' });
  });

  it('speaks English', () => {
    setLang('en');
    app.targetCi = { p1: branchCi() };
    app.projects[0].board.pauseOnRedTarget = true;
    render(Board, { project: app.projects[0] });
    expect(screen.getByRole('button', { name: /^CI of main/ })).toHaveTextContent('main ✗ for 2 h');
    expect(document.querySelector('[data-red-target]')).toHaveTextContent('Autopilot paused: the CI of main has been red for 2 h');
    expect(within(col('To do')).getByText('Waiting: the target’s CI is red')).toBeInTheDocument();
  });
});
