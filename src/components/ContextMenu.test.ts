import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { menu } from '../lib/menu.svelte';
import ContextMenu from './ContextMenu.svelte';

describe('ContextMenu', () => {
  beforeEach(() => menu.close());

  it('offers a row of colors, the current one checked, and closes once one is picked', async () => {
    const picked: string[] = [];
    render(ContextMenu);
    menu.open = {
      x: 10,
      y: 10,
      items: [
        { label: 'Renommer…', onClick: () => {} },
        { label: 'Couleur', colors: { values: ['red', 'green', 'blue'], selected: 'green', onPick: (c) => picked.push(c) } },
      ],
    };
    const row = await screen.findByRole('group', { name: 'Couleur' });
    const swatches = [...row.querySelectorAll('button')];
    expect(swatches.map((s) => [s.getAttribute('aria-label'), s.getAttribute('aria-checked')])).toEqual([
      ['Couleur 1', 'false'],
      ['Couleur 2', 'true'],
      ['Couleur 3', 'false'],
    ]);
    await userEvent.click(screen.getByRole('menuitemradio', { name: 'Couleur 3' }));
    expect(picked).toEqual(['blue']);
    expect(menu.open).toBeNull();
  });

  it('says on an entry it offers no more why, in its title', async () => {
    render(ContextMenu);
    menu.open = {
      x: 10,
      y: 10,
      items: [
        { label: 'Renommer', onClick: () => {} },
        { label: 'Dupliquer la conversation', disabled: true, title: 'Attends la fin de son tour.', onClick: () => {} },
      ],
    };
    const entry = await screen.findByRole('menuitem', { name: 'Dupliquer la conversation' });
    expect(entry).toBeDisabled();
    expect(entry).toHaveAttribute('title', 'Attends la fin de son tour.');
    expect(screen.getByRole('menuitem', { name: 'Renommer' })).not.toHaveAttribute('title');
  });
});
