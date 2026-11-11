// Global context menu.

import type { IconName } from './icons';
import type { EngineId } from './types';

export interface MenuItem {
  label: string;
  onClick?: () => void;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  hint?: string;
  /** Its tooltip. */
  title?: string;
  /** Why it is disabled, written under it: what cannot be done stays in its place, greyed, with its reason (choice 17 of the 2.0 design). */
  reason?: string;
  /** A line under its label that says more of it (always shown, unlike `reason`). */
  detail?: string;
  /** A project's colour, as a small tile, in place of an icon (a closed project to reopen). */
  swatch?: string;
  /** An icon of the app before its label. */
  icon?: IconName;
  /** The brand of a harness before its label, shown whatever the agents run on (the menu « Harness »). */
  engine?: EngineId;
  /** A title over the entries that follow, not an entry (its `hint`: a shortcut, in a key). */
  heading?: boolean;
  /** The key that picks it while the menu is open (« 1 » to « 4 » in the menu « Harness »), shown at its end. */
  key?: string;
  /** The one chosen among its siblings: a dot after its label (`menuitemradio`). */
  checked?: boolean;
  /** A row of colors to pick from, named by the label. */
  colors?: { values: string[]; selected: string; onPick: (color: string) => void };
  /** The entries it opens beside the menu, on hovering it (never on a click), or on → / Enter / Space. One level only. */
  submenu?: MenuItem[];
}

/** The gap kept between a menu and the window's edges. */
export const EDGE = 8;

/** The widest a menu gets (its CSS `max-width`). */
export const MENU_MAX_W = 380;

export type Side = 'left' | 'right';

/**
 * The side the submenus of a menu at `frame` open on, known before they open so that their chevrons say it: on the right
 * when the widest of them has room there, else on the side with the more room.
 */
export function sideOf(frame: { left: number; right: number }, viewW: number): Side {
  const right = viewW - EDGE - (frame.right - 3);
  const left = frame.left + 3 - EDGE;
  return right >= MENU_MAX_W || right >= left ? 'right' : 'left';
}

/**
 * Where a submenu of `size` stands beside the `frame` of its menu, on its `side`, level with its `entry`; pushed back
 * into the window when it has no room there, moved up to stay in it.
 */
export function beside(entry: DOMRect, frame: DOMRect, size: { w: number; h: number }, view: { w: number; h: number }, side: Side) {
  // A little over the menu, so that the pointer never crosses a gap.
  const x = side === 'right' ? frame.right - 3 : frame.left - size.w + 3;
  // Its first entry level with its parent, past the menu's padding.
  const y = Math.max(EDGE, Math.min(entry.top - 5, view.h - EDGE - size.h));
  return { x: Math.max(EDGE, Math.min(x, view.w - EDGE - size.w)), y };
}

class MenuState {
  /** `keyboard`: opened from an element (a button, the menu key): the focus goes into it, and comes back. */
  open = $state<{ x: number; y: number; items: MenuItem[]; keyboard?: boolean } | null>(null);

  show(e: MouseEvent, items: MenuItem[]) {
    e.preventDefault();
    e.stopPropagation();
    this.open = { x: e.clientX, y: e.clientY, items };
  }

  showAt(el: HTMLElement, items: MenuItem[]) {
    const r = el.getBoundingClientRect();
    this.open = { x: r.left, y: r.bottom + 4, items, keyboard: true };
  }

  close() {
    this.open = null;
  }
}

export const menu = new MenuState();
