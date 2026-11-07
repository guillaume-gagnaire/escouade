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
