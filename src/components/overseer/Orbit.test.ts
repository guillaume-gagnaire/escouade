import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { orbitTasks, place } from '../../lib/orbit';
import type { OverseerNews } from '../../lib/types';
import { project, ticket } from '../../test/ipc';
import Orbit from './Orbit.svelte';

const NOW = Date.now();
const P1 = project({ id: 'p1', color: 'rgb(80, 140, 250)' });
const P2 = project({ id: 'p2', name: 'site', color: 'rgb(230, 90, 170)' });

const NEWS: OverseerNews[] = [
  { id: 'n-1', at: NOW - 500, kind: 'review', ticketId: 'fin', ticket: 'DEM-31', title: 'Paiement', projectId: 'p1', project: 'demo-api' },
];

const TASKS = place(
  orbitTasks(
    [P1, P2],
    [
      ticket({ id: 'run', key: 'DEM-41', title: 'Relances', column: 'doing', startedAt: NOW }),
      ticket({ id: 'ws', projectId: 'p2', key: 'WS-12', title: 'Panier', column: 'review', reviewAt: NOW - 60_000 }),
      ticket({ id: 'fin', key: 'DEM-31', title: 'Paiement en 4 fois', column: 'review', reviewAt: NOW - 500 }),
      ticket({ id: 'old', key: 'DEM-38', title: 'Tarif famille', column: 'done', doneAt: NOW - 3600_000 }),
    ],
    [],
    NEWS,
    NOW,
  ),
);

function shown(over: Partial<Parameters<typeof Orbit>[1]> = {}) {
  const open = vi.fn();
  render(Orbit, {
    props: { tasks: TASKS, phase: 'idle', level: 0, played: 0, heard: null, subtitle: null, thinking: false, open, ...over },
  });
  return open;
}

const card = (key: string) => [...document.querySelectorAll<HTMLElement>('[data-orbit-task]')].find((c) => c.textContent?.includes(key))!;

describe('the orbit of the Overseer’s screen (ESC-468)', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('places each task on the arc of its state, in its project’s colour, and opens it on a click', async () => {
    const open = shown();
    const run = card('DEM-41');
    const ws = card('WS-12');
    const old = card('DEM-38');
    expect(run.dataset.state).toBe('working');
    expect(ws.dataset.state).toBe('waiting');
    expect(old.dataset.state).toBe('done');
    // Under way at the top, waiting on the right, done at the bottom.
    expect(parseFloat(run.style.top)).toBeLessThan(30);
    expect(parseFloat(ws.style.left)).toBeGreaterThan(70);
    expect(parseFloat(old.style.top)).toBeGreaterThan(50);
    expect(run.style.getPropertyValue('--c')).toBe(P1.color);
    expect(ws.style.getPropertyValue('--c')).toBe(P2.color);
    expect(ws).toHaveTextContent('À valider');
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir DEM-41' }));
    expect(open).toHaveBeenCalledWith(expect.objectContaining({ ticketId: 'run' }));
  });

  it('melts the task just finished toward the orb, lit, while it tells it', () => {
    shown();
    const fin = card('DEM-31');
    expect(fin.hasAttribute('data-fresh')).toBe(true);
    expect(fin).toHaveTextContent('Terminé · à l’instant');
    // Its way to the orb, which the animation takes.
    expect(fin.style.getPropertyValue('--dx')).toMatch(/cqw$/);
    expect(document.querySelector('[data-orbit]')).toHaveClass('lit');
    expect(screen.getByRole('status')).toHaveTextContent('L’Overseer te parle');
    expect(card('WS-12').hasAttribute('data-fresh')).toBe(false);
  });

  it('writes the sentence said under the orb, and its own as a subtitle', () => {
    shown({ phase: 'talking', heard: 'Quand la CI est verte, pose le tag' });
    expect(screen.getByRole('status')).toHaveTextContent('Je t’écoute');
    expect(document.querySelector('[data-orb-caption]')).toHaveTextContent('« Quand la CI est verte, pose le tag');
    expect(document.querySelector('[data-orbit]')).toHaveClass('listening');
  });

  it('says its answer as a subtitle while it speaks', () => {
    shown({ phase: 'speaking', subtitle: 'C’est noté : je pose le tag.' });
    expect(document.querySelector('[data-orb-caption]')).toHaveTextContent('« C’est noté : je pose le tag. »');
  });

  it('moves nothing with prefers-reduced-motion, and the state is read all the same', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn((q: string) => ({ matches: q.includes('reduce'), media: q, addEventListener() {}, removeEventListener() {} })),
    );
    shown();
    const orbit = document.querySelector('[data-orbit]') as HTMLElement;
    expect(orbit.dataset.motion).toBe('reduced');
    expect(orbit).toHaveClass('still');
    // The finished task still says it, without moving.
    expect(card('DEM-31')).toHaveTextContent('Terminé · à l’instant');
  });
});
