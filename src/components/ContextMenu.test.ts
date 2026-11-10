import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { beside, menu, sideOf, type MenuItem } from '../lib/menu.svelte';
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
    expect(entry).toHaveAttribute('data-tip', 'Attends la fin de son tour.');
    expect(screen.getByRole('menuitem', { name: 'Renommer' })).not.toHaveAttribute('data-tip');
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

describe('ContextMenu, a submenu beside it', () => {
  beforeEach(() => menu.close());

  /** Longer than the 150 ms the pointer rests on an entry before its submenu opens or closes. */
  const rest = () => new Promise((r) => setTimeout(r, 200));
  const entries = (ran: string[] = [], parent: Partial<MenuItem> = {}): MenuItem[] => [
    { label: 'Renommer', onClick: () => ran.push('Renommer') },
    {
      label: 'Lancer',
      submenu: [
        { label: 'Front', onClick: () => ran.push('Front') },
        { label: 'API', onClick: () => ran.push('API') },
      ],
      ...parent,
    },
    { label: 'Supprimer', onClick: () => ran.push('Supprimer') },
  ];
  const sub = () => screen.queryByRole('menu', { name: 'Lancer' });

  it('opens on hovering its entry, never on a click, and its entries run and close both menus', async () => {
    const ran: string[] = [];
    render(ContextMenu);
    menu.open = { x: 10, y: 10, items: entries(ran) };
    const parent = await screen.findByRole('menuitem', { name: 'Lancer' });
    expect(parent).toHaveAttribute('aria-haspopup', 'menu');
    expect(parent).toHaveAttribute('aria-expanded', 'false');
    expect(parent.querySelector('.chev')).not.toBeNull();
    // A click (without the pointer coming over it first) neither closes the menu nor opens the submenu.
    await fireEvent.click(parent);
    await rest();
    expect(menu.open).not.toBeNull();
    expect(sub()).toBeNull();
    await userEvent.hover(parent);
    const opened = await screen.findByRole('menu', { name: 'Lancer' });
    expect(parent).toHaveAttribute('aria-expanded', 'true');
    await userEvent.click(within(opened).getByRole('menuitem', { name: 'API' }));
    expect(ran).toEqual(['API']);
    expect(menu.open).toBeNull();
  });

  it('puts the chevron of its entry on the side it opens: the right, the left near the right edge of the window', async () => {
    render(ContextMenu);
    menu.open = { x: 10, y: 10, items: entries() };
    const parent = await screen.findByRole('menuitem', { name: 'Lancer' });
    expect(parent.lastElementChild).toHaveClass('chev');
    expect(parent.querySelector('.chev.left')).toBeNull();
    menu.open = { x: window.innerWidth - 100, y: 10, items: entries() };
    await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Lancer' }).firstElementChild).toHaveClass('chev', 'left'));
    expect(screen.getByRole('menuitem', { name: 'Lancer' }).querySelectorAll('.chev')).toHaveLength(1);
  });

  it('stays open on the way to it, and closes once the pointer rests on another entry', async () => {
    render(ContextMenu);
    menu.open = { x: 10, y: 10, items: entries() };
    await userEvent.hover(await screen.findByRole('menuitem', { name: 'Lancer' }));
    const opened = await screen.findByRole('menu', { name: 'Lancer' });
    // Over the entry below on the way, then in the submenu before the delay is up.
    await userEvent.hover(screen.getByRole('menuitem', { name: 'Supprimer' }));
    await userEvent.hover(within(opened).getByRole('menuitem', { name: 'Front' }));
    await rest();
    expect(sub()).not.toBeNull();
    await userEvent.hover(screen.getByRole('menuitem', { name: 'Supprimer' }));
    await waitFor(() => expect(sub()).toBeNull());
    expect(menu.open).not.toBeNull();
  });

  it('never opens for an entry that is disabled', async () => {
    render(ContextMenu);
    menu.open = { x: 10, y: 10, items: entries([], { disabled: true, title: 'Attends la fin de son tour.' }) };
    const parent = await screen.findByRole('menuitem', { name: 'Lancer' });
    await fireEvent.mouseEnter(parent);
    await rest();
    expect(sub()).toBeNull();
  });

  it('works from the keyboard: → or Enter go into it, ← and Escape come back to its entry', async () => {
    const ran: string[] = [];
    const opener = document.createElement('button');
    document.body.append(opener);
    opener.focus();
    try {
      render(ContextMenu);
      menu.showAt(opener, entries(ran));
      const parent = await screen.findByRole('menuitem', { name: 'Lancer' });
      await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Renommer' })).toHaveFocus());
      await userEvent.keyboard('{ArrowDown}{ArrowRight}');
      await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Front' })).toHaveFocus());
      // Its arrows go round its own entries.
      await userEvent.keyboard('{ArrowDown}');
      expect(screen.getByRole('menuitem', { name: 'API' })).toHaveFocus();
      await userEvent.keyboard('{ArrowDown}');
      expect(screen.getByRole('menuitem', { name: 'Front' })).toHaveFocus();
      await userEvent.keyboard('{ArrowLeft}');
      expect(parent).toHaveFocus();
      await waitFor(() => expect(sub()).toBeNull());
      await userEvent.keyboard('{Enter}');
      await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Front' })).toHaveFocus());
      // Escape closes the submenu only.
      await userEvent.keyboard('{Escape}');
      expect(parent).toHaveFocus();
      expect(menu.open).not.toBeNull();
      await userEvent.keyboard('{ArrowRight}');
      await waitFor(() => expect(screen.getByRole('menuitem', { name: 'Front' })).toHaveFocus());
      await userEvent.keyboard('{ArrowDown}{Enter}');
      expect(ran).toEqual(['API']);
      expect(menu.open).toBeNull();
      await waitFor(() => expect(opener).toHaveFocus());
    } finally {
      opener.remove();
    }
  });

  it('closes first on Escape when the focus is not in the menus', async () => {
    render(ContextMenu);
    menu.open = { x: 10, y: 10, items: entries() };
    await userEvent.hover(await screen.findByRole('menuitem', { name: 'Lancer' }));
    await screen.findByRole('menu', { name: 'Lancer' });
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(sub()).toBeNull());
    expect(menu.open).not.toBeNull();
    await userEvent.keyboard('{Escape}');
    expect(menu.open).toBeNull();
  });

  it('closes when its menu scrolls', async () => {
    render(ContextMenu);
    menu.open = { x: 10, y: 10, items: entries() };
    await userEvent.hover(await screen.findByRole('menuitem', { name: 'Lancer' }));
    await screen.findByRole('menu', { name: 'Lancer' });
    await fireEvent.scroll(document.querySelector('.items')!);
    await waitFor(() => expect(sub()).toBeNull());
  });
});

describe('ContextMenu taller than the window', () => {
  beforeEach(() => menu.close());

  /** jsdom lays nothing out: the list says how tall it is, and where it is scrolled to. */
  const lay = (list: HTMLElement, top: number) => {
    for (const [k, v] of Object.entries({ scrollHeight: 1020, clientHeight: 584, scrollTop: top }))
      Object.defineProperty(list, k, { value: v, configurable: true });
    return fireEvent.scroll(list);
  };
  const edges = () => [...document.querySelectorAll('.edge')].map((e) => e.classList[1]);

  it('scrolls, the edges with entries out of view faded and marked with a chevron', async () => {
    render(ContextMenu);
    menu.open = { x: 10, y: 10, items: Array.from({ length: 30 }, (_, i) => ({ label: `Entrée ${i}`, onClick: () => {} })) };
    const list = (await screen.findByRole('menu')).querySelector<HTMLElement>('.items')!;
    await lay(list, 0);
    await waitFor(() => expect(edges()).toEqual(['down']));
    expect(list).toHaveClass('more-down');
    expect(list).not.toHaveClass('more-up');
    await lay(list, 200);
    await waitFor(() => expect(edges()).toEqual(['up', 'down']));
    expect(list).toHaveClass('more-up', 'more-down');
    await lay(list, 436);
    await waitFor(() => expect(edges()).toEqual(['up']));
    expect(list).not.toHaveClass('more-down');
  });
});

describe('sideOf', () => {
  it('opens the submenus on the right when the widest has room there, else on the side with the more room', () => {
    expect(sideOf({ left: 100, right: 320 }, 1000)).toBe('right');
    expect(sideOf({ left: 760, right: 980 }, 1000)).toBe('left');
    // Room for the widest on neither side: the larger one.
    expect(sideOf({ left: 300, right: 680 }, 1000)).toBe('right');
    expect(sideOf({ left: 340, right: 720 }, 1000)).toBe('left');
  });
});

describe('beside', () => {
  const rect = (left: number, top: number, width: number, height: number) =>
    ({ left, top, width, height, right: left + width, bottom: top + height }) as DOMRect;
  const view = { w: 1000, h: 600 };

  it('puts a submenu on its side of the menu, level with its entry', () => {
    expect(beside(rect(105, 140, 210, 34), rect(100, 100, 220, 300), { w: 220, h: 80 }, view, 'right')).toEqual({ x: 317, y: 135 });
    expect(beside(rect(765, 140, 210, 34), rect(760, 100, 220, 300), { w: 220, h: 80 }, view, 'left')).toEqual({ x: 543, y: 135 });
  });

  it('pushes it back into the window when it has no room on its side', () => {
    expect(beside(rect(15, 140, 970, 34), rect(10, 100, 980, 300), { w: 300, h: 80 }, view, 'right')).toEqual({ x: 692, y: 135 });
    expect(beside(rect(15, 140, 970, 34), rect(10, 100, 980, 300), { w: 300, h: 80 }, view, 'left')).toEqual({ x: 8, y: 135 });
  });

  it('moves it up to stay in the window, never above its top', () => {
    expect(beside(rect(105, 500, 210, 34), rect(100, 100, 220, 450), { w: 220, h: 200 }, view, 'right')).toEqual({ x: 317, y: 392 });
    expect(beside(rect(105, 500, 210, 34), rect(100, 8, 220, 584), { w: 220, h: 700 }, view, 'right')).toEqual({ x: 317, y: 8 });
  });
});
