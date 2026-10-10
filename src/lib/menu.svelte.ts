// Global context menu.

export interface MenuItem {
  label: string;
  onClick?: () => void;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  hint?: string;
  /** Its tooltip: why it is disabled, for one. */
  title?: string;
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
