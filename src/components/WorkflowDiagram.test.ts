// The Diagram view from an entry (ESC-406): the door the agent took, what it reaches (the rest folded), its path traced
// and numbered over the possibles, the same path in words; without an agent, the doors to choose from.

import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import type { Workflow, WorkflowTrack } from '../lib/types';
import { diagramOf, entriesOf, entryOf, pathOf, progressOf, type DgAgent } from '../lib/workflow-diagram';
import { gsd, passage, track } from '../test/workflows';
import WorkflowDiagram from './WorkflowDiagram.svelte';

/** An agent that came in by Plan: a round of its loop, on to Verify, back to Plan for gaps, executing again. */
function enteredByPlan(): WorkflowTrack {
  const passages = [
    passage(1, 'gsd:plan-phase', 0, 4, { door: '/gsd:plan-phase 3', outcome: 'back', reason: 'plan refusé', backTo: 'gsd:plan-phase' }),
    passage(2, 'gsd:plan-phase', 4, 6, { pass: 2 }),
    passage(3, 'gsd:execute-phase', 10, 21),
    passage(4, 'gsd:verify-work', 31, 5, { outcome: 'back', reason: '2 tests rouges', backTo: 'gsd:plan-phase' }),
    passage(5, 'gsd:plan-phase', 36, 3, { pass: 3 }),
    passage(6, 'gsd:execute-phase', 39, 8, { pass: 2, outcome: 'running' }),
  ];
  return track('a1', { step: 'gsd:execute-phase', passages });
}

function draw(w: Workflow, tr: WorkflowTrack | null, more: { big?: boolean; agents?: Record<string, DgAgent[]> } = {}) {
  const model = diagramOf(w);
  return render(WorkflowDiagram, {
    model,
    progress: progressOf(w, model, tr),
    entries: entriesOf(w, model),
    entry: entryOf(w, model, tr),
    path: pathOf(w, model, tr),
    reader: tr ? 'paiement-3ds' : '',
    agents: more.agents ?? {},
    big: more.big ?? false,
  });
}
const lines = (dg: HTMLElement, cls: string) => dg.querySelectorAll(`path.e.${cls}:not(.head)`);

describe('the diagram of an agent, from the door it took', () => {
  it('starts from its door alone, and draws only what that door reaches', () => {
    draw(gsd(), enteredByPlan());
    const dg = screen.getByTestId('workflow-diagram');
    const doors = [...dg.querySelectorAll('.dg-door')];
    expect(doors.map((d) => d.getAttribute('data-door'))).toEqual(['/gsd:plan-phase']);
    expect(doors[0]).toHaveClass('taken');
    expect(doors[0].querySelector('.lb')).toHaveTextContent('Entrée');
    expect([...dg.querySelectorAll('.dg-node')].map((n) => n.getAttribute('data-step'))).toEqual([
      'gsd:plan-phase',
      'gsd:execute-phase',
      'gsd:verify-work',
    ]);
    // No choice of doors: the agent made it.
    expect(screen.queryByRole('radiogroup')).toBeNull();
  });

  it('folds the steps out of this path, and opens them on demand', async () => {
    draw(gsd(), enteredByPlan());
    const off = screen.getByTestId('workflow-off');
    const fold = within(off).getByRole('button', { name: '1 étape hors de ce chemin' });
    expect(fold).toHaveAttribute('aria-expanded', 'false');
    expect(within(off).queryByText('Discuss phase')).toBeNull();
    await userEvent.click(fold);
    expect(fold).toHaveAttribute('aria-expanded', 'true');
    expect(within(off).getByText('Discuss phase')).toBeInTheDocument();
    expect(within(off).getByText('/gsd:discuss-phase')).toBeInTheDocument();
    expect(within(off).getByText('Pas atteignable depuis /gsd:plan-phase.')).toBeInTheDocument();
    await userEvent.click(fold);
    expect(within(off).queryByText('Discuss phase')).toBeNull();
  });

  it('numbers each passage on its step, the one in course apart, and says the next', () => {
    draw(gsd(), enteredByPlan());
    const dg = screen.getByTestId('workflow-diagram');
    const beads = (step: string) => [...dg.querySelectorAll(`[data-step="${step}"] .dg-bead`)];
    expect(beads('gsd:plan-phase').map((b) => b.textContent)).toEqual(['1', '2', '5']);
    expect(beads('gsd:execute-phase').map((b) => b.textContent)).toEqual(['3', '6']);
    expect(beads('gsd:verify-work').map((b) => b.textContent)).toEqual(['4']);
    expect([...dg.querySelectorAll('.dg-bead.on')].map((b) => b.textContent)).toEqual(['6']);
    const nodes = [...dg.querySelectorAll('.dg-node')];
    expect(nodes.map((n) => n.getAttribute('aria-label'))).toEqual([
      'Plan phase, faite',
      'Execute phase, en cours',
      'Verify work, ensuite',
    ]);
    // In words on the step too, not by its colour alone.
    expect(nodes[1].querySelector('.tag')).toHaveTextContent('en cours');
    expect(nodes[2].querySelector('.tag')).toHaveTextContent('ensuite');
    expect(nodes[0].querySelector('title')).toHaveTextContent('3 passages : n° 1, 2, 5');
  });

  it('tells what was taken from what is only possible: full lines against dashed ones', () => {
    draw(gsd(), enteredByPlan());
    const dg = screen.getByTestId('workflow-diagram');
    // The entry and two ways forward taken; the exit only possible.
    expect(lines(dg, 'walked')).toHaveLength(3);
    expect(lines(dg, 'fwd')).toHaveLength(1);
    // The way back taken, with why; the loop on Plan, one round of three.
    expect(lines(dg, 'took')).toHaveLength(1);
    const ret = dg.querySelector('.dg-chip.ret')!;
    expect(ret).toHaveClass('took');
    expect(ret.querySelector('title')).toHaveTextContent('2 tests rouges');
    const loop = dg.querySelector('[data-step="gsd:plan-phase"] .dg-self')!;
    expect(loop).toHaveClass('took');
    expect(loop.querySelector('text')).toHaveTextContent('1/3');
    expect(loop.querySelector('title')).toHaveTextContent('plan refusé : 1 tour pris');
  });

  it('says the path in words, in its order, in a list a screen reader reads', () => {
    draw(gsd(), enteredByPlan());
    const list = screen.getByRole('list', { name: 'Chemin de paiement-3ds, dans l’ordre' });
    expect(list.tagName).toBe('OL');
    expect(
      within(list)
        .getAllByRole('listitem')
        .map((li) => li.textContent),
    ).toEqual([
      'Plan phase, entrée par /gsd:plan-phase',
      'Plan phase, tour de boucle : plan refusé, 2e passage',
      'Execute phase',
      'Verify work',
      'Plan phase, retour depuis Verify work : 2 tests rouges, 3e passage',
      'Execute phase, 2e passage, en cours',
    ]);
  });

  it('keeps the other agents as dots on their step', () => {
    draw(gsd(), enteredByPlan(), {
      agents: {
        'gsd:execute-phase': [{ id: 'a1', name: 'paiement-3ds', cur: true, wait: false }],
        'gsd:verify-work': [{ id: 'a2', name: 'facture-pdf', cur: false, wait: true }],
      },
    });
    const dg = screen.getByTestId('workflow-diagram');
    expect(dg.querySelector('[data-step="gsd:verify-work"] circle.ag.wait title')).toHaveTextContent('facture-pdf');
    expect(dg.querySelector('[data-step="gsd:execute-phase"] circle.ag.cur title')).toHaveTextContent('paiement-3ds');
    // Only the agent read has its path: one passage in course.
    expect(dg.querySelectorAll('.dg-bead.on')).toHaveLength(1);
  });

  it('says the path and the entry in English too', () => {
    setLang('en');
    draw(gsd(), enteredByPlan());
    expect(screen.getByRole('list', { name: 'Path of paiement-3ds, in order' })).toBeInTheDocument();
    expect(screen.getByText('Plan phase, back from Verify work: 2 tests rouges, pass 3')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '1 step off this path' })).toBeInTheDocument();
    setLang('fr');
  });
});

describe('the diagram of a workflow alone: a choice of its doors', () => {
  /** GSD with a door that enters at Execute: Discuss is then out of the path. */
  const withQuick = () => {
    const w = gsd();
    w.doors = w.doors.map((d) => (d.name === 'gsd:quick' ? { ...d, role: 'entry' as const, enters: ['gsd:execute-phase'] } : d));
    return w;
  };

  it('offers every door, the first chosen: everything it reaches is a possible, nothing is taken', () => {
    draw(withQuick(), null);
    const group = screen.getByRole('radiogroup', { name: 'Entrée : par où l’agent commence' });
    const radios = within(group).getAllByRole('radio');
    expect(radios.map((r) => r.textContent)).toEqual(['/gsd:discuss-phase', '/gsd:autonomous', '/gsd:progress', '/gsd:quick', '/gsd:next']);
    expect(radios.map((r) => r.getAttribute('aria-checked'))).toEqual(['true', 'false', 'false', 'false', 'false']);
    const dg = screen.getByTestId('workflow-diagram');
    expect(dg.querySelectorAll('.dg-node')).toHaveLength(4);
    expect([...dg.querySelectorAll('.dg-door')].map((d) => d.getAttribute('data-door'))).toEqual(['/gsd:discuss-phase']);
    expect(dg.querySelector('.dg-door.taken')).toBeNull();
    expect(lines(dg, 'walked')).toHaveLength(0);
    expect(dg.querySelectorAll('.dg-bead')).toHaveLength(0);
    expect(screen.queryByTestId('workflow-off')).toBeNull();
    expect(screen.queryByTestId('workflow-path')).toBeNull();
  });

  it('draws again from the door picked, by the mouse or the arrows', async () => {
    draw(withQuick(), null);
    const dg = screen.getByTestId('workflow-diagram');
    await userEvent.click(screen.getByRole('radio', { name: '/gsd:quick' }));
    expect(screen.getByRole('radio', { name: '/gsd:quick' })).toHaveAttribute('aria-checked', 'true');
    expect([...dg.querySelectorAll('.dg-door')].map((d) => d.getAttribute('data-door'))).toEqual(['/gsd:quick']);
    // Execute, Verify, and Plan by the way back; Discuss folded.
    expect([...dg.querySelectorAll('.dg-node')].map((n) => n.getAttribute('data-step'))).toEqual([
      'gsd:plan-phase',
      'gsd:execute-phase',
      'gsd:verify-work',
    ]);
    expect(screen.getByRole('button', { name: '1 étape hors de ce chemin' })).toBeInTheDocument();
    await userEvent.keyboard('{ArrowRight}');
    expect(screen.getByRole('radio', { name: '/gsd:next' })).toHaveAttribute('aria-checked', 'true');
    expect(dg.querySelectorAll('.dg-node')).toHaveLength(4);
    expect(screen.queryByTestId('workflow-off')).toBeNull();
  });

  it('has no choice to offer with a single way in', () => {
    const w = gsd();
    w.doors = [];
    draw(w, null);
    expect(screen.queryByRole('radiogroup')).toBeNull();
    expect(screen.getByTestId('workflow-diagram').querySelector('.dg-door')?.getAttribute('data-door')).toBe('/gsd:discuss-phase');
  });
});

describe('enlarged', () => {
  it('follows the same logic: the door taken, the path numbered and in words, the fold', () => {
    draw(gsd(), enteredByPlan(), { big: true });
    const big = screen.getByTestId('workflow-diagram-big');
    expect(big.querySelector('.dg-title')).toHaveTextContent('entré par /gsd:plan-phase');
    const dg = big.querySelector('.dg-canvas')!;
    expect([...dg.querySelectorAll('.dg-door')].map((d) => d.getAttribute('data-door'))).toEqual(['/gsd:plan-phase']);
    expect(dg.querySelectorAll('.dg-node')).toHaveLength(3);
    expect([...dg.querySelectorAll('.dg-bead')].map((b) => b.textContent)).toEqual(['1', '2', '5', '3', '6', '4']);
    // The list of the path once, in the enlarged view.
    const lists = screen.getAllByTestId('workflow-path');
    expect(lists).toHaveLength(1);
    expect(big.contains(lists[0])).toBe(true);
    expect(within(big).getByRole('button', { name: '1 étape hors de ce chemin' })).toBeInTheDocument();
  });
});
