import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
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

describe('ContextMenu on the keyboard', () => {
  let opener: HTMLButtonElement;
  beforeEach(() => {
    menu.close();
    opener = document.createElement('button');
    document.body.append(opener);
    opener.focus();
  });
  afterEach(() => opener.remove());

  const entries = (ran: string[] = []) => [
    { label: 'Premier', onClick: () => ran.push('Premier') },
    { label: 'Interdit', disabled: true, title: 'Pas maintenant.' },
    { label: 'Dernier', onClick: () => ran.push('Dernier') },
  ];

  it('takes the focus when opened from an element, and the arrows go from one entry to the next, the disabled ones skipped', async () => {
    render(ContextMenu);
    menu.showAt(opener, entries());
    const [first, last] = [await screen.findByRole('menuitem', { name: 'Premier' }), screen.getByRole('menuitem', { name: 'Dernier' })];
    await waitFor(() => expect(first).toHaveFocus());
    await userEvent.keyboard('{ArrowDown}');
    expect(last).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}');
    expect(first).toHaveFocus();
    await userEvent.keyboard('{ArrowUp}');
    expect(last).toHaveFocus();
    await userEvent.keyboard('{Home}');
    expect(first).toHaveFocus();
    await userEvent.keyboard('{End}');
    expect(last).toHaveFocus();
  });

  it('runs the entry under the focus on Enter, closes, and gives the focus back to where it came from', async () => {
    const ran: string[] = [];
    render(ContextMenu);
    menu.showAt(opener, entries(ran));
    await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Premier' })).toHaveFocus());
    await userEvent.keyboard('{ArrowDown}{Enter}');
    expect(ran).toEqual(['Dernier']);
    expect(menu.open).toBeNull();
    await waitFor(() => expect(opener).toHaveFocus());
  });

  it('closes on Escape, and on Tab, and gives the focus back', async () => {
    render(ContextMenu);
    menu.showAt(opener, entries());
    await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Premier' })).toHaveFocus());
    await userEvent.keyboard('{Escape}');
    expect(menu.open).toBeNull();
    await waitFor(() => expect(opener).toHaveFocus());
    menu.showAt(opener, entries());
    await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Premier' })).toHaveFocus());
    await userEvent.tab();
    expect(menu.open).toBeNull();
    await waitFor(() => expect(opener).toHaveFocus());
  });

  it('keeps its Escape to itself while the focus is in it: a dialog under it does not close with it', async () => {
    const seen: string[] = [];
    const listener = (e: KeyboardEvent) => seen.push(e.key);
    render(ContextMenu);
    menu.showAt(opener, entries());
    await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Premier' })).toHaveFocus());
    window.addEventListener('keydown', listener);
    try {
      await userEvent.keyboard('{Escape}');
      expect(menu.open).toBeNull();
      expect(seen).toEqual([]);
      // Once closed, the key goes on its way again.
      await userEvent.keyboard('{Escape}');
      expect(seen).toEqual(['Escape']);
    } finally {
      window.removeEventListener('keydown', listener);
    }
  });

  it('leaves the focus to the dialog an entry opens', async () => {
    render(ContextMenu);
    const dialog = document.createElement('div');
    dialog.tabIndex = -1;
    document.body.append(dialog);
    menu.showAt(opener, [
      {
        label: 'Ouvrir',
        onClick: () => dialog.focus(),
      },
    ]);
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Ouvrir' }));
    await new Promise((r) => setTimeout(r, 20));
    expect(dialog).toHaveFocus();
    dialog.remove();
  });

  it('keeps the focus where it is when opened by a right click', async () => {
    render(ContextMenu);
    menu.open = { x: 5, y: 5, items: entries() };
    await screen.findByRole('menuitem', { name: 'Premier' });
    expect(opener).toHaveFocus();
  });

  it('is not given a native menu of its own by a right click on it', async () => {
    render(ContextMenu);
    menu.showAt(opener, entries());
    const entry = await screen.findByRole('menuitem', { name: 'Premier' });
    // `false`: something called preventDefault.
    expect(await fireEvent.contextMenu(entry)).toBe(false);
  });
});

describe('ContextMenu in English', () => {
  beforeEach(() => {
    menu.close();
    setLang('en');
  });

  it('numbers the swatches of a row of colors in English', async () => {
    render(ContextMenu);
    menu.open = {
      x: 10,
      y: 10,
      items: [{ label: 'Color', colors: { values: ['red', 'green'], selected: 'red', onPick: () => {} } }],
    };
    const row = await screen.findByRole('group', { name: 'Color' });
    expect([...row.querySelectorAll('button')].map((s) => s.getAttribute('aria-label'))).toEqual(['Color 1', 'Color 2']);
  });
});
