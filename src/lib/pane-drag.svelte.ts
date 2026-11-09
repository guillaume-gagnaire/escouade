// Dragging a content from the sidebar onto the work area (ESC-392): an agent's card, a terminal's or a launch's row,
// the title bar's « Éditeur ». A pointer gesture rather than HTML5 drag and drop: the card that follows and the compass
// draw the same on WebView2, WKWebView and WebKitGTK, and Escape cancels. It starts past a few pixels: a click stays a
// click (the click that ends a drag is swallowed).

import { has, itemKey, leaves, zoneAt, zoneFits, type PaneItem, type PaneZone } from './panes';
import { app } from './state.svelte';

/** How far the pointer goes before a press becomes a drag. */
const THRESHOLD = 5;

export interface PaneTarget {
  /** The content of the pane under the pointer (or the one shown alone). */
  item: PaneItem;
  rect: { left: number; top: number; width: number; height: number };
  /** Where it would go; null: a side with no room left (nothing happens on release). */
  zone: PaneZone | null;
  /** The zone under the pointer, even when there is no room for it: the compass lights it, crossed out. */
  aimed: PaneZone;
  /** The zones that fit, for the compass. */
  fits: Record<PaneZone, boolean>;
}

const ZONES: PaneZone[] = ['left', 'right', 'top', 'bottom', 'center'];

class PaneDrag {
  item = $state<PaneItem | null>(null);
  label = $state('');
  detail = $state('');
  /** The press became a drag. */
  on = $state(false);
  x = $state(0);
  y = $state(0);
  target = $state<PaneTarget | null>(null);
  private startX = 0;
  private startY = 0;

  /** A press on something that can be dragged: `item`, shown as `label` (and `detail`) on the card that follows. */
  press(e: PointerEvent, item: PaneItem, label: string, detail = '') {
    if (e.button !== 0 || this.item) return;
    // A button or a field inside the card keeps its own press.
    const inner = (e.target as Element | null)?.closest('button, input, textarea, select, a');
    if (inner && inner !== e.currentTarget) return;
    this.item = item;
    this.label = label;
    this.detail = detail;
    this.startX = e.clientX;
    this.startY = e.clientY;
    window.addEventListener('pointermove', this.move, true);
    window.addEventListener('pointerup', this.up, true);
    window.addEventListener('pointercancel', this.cancel, true);
    window.addEventListener('keydown', this.key, true);
    window.addEventListener('blur', this.cancel);
  }

  private move = (e: PointerEvent) => {
    if (!this.on) {
      if (Math.hypot(e.clientX - this.startX, e.clientY - this.startY) < THRESHOLD) return;
      this.on = true;
      document.documentElement.classList.add('pane-dragging');
      window.getSelection()?.removeAllRanges();
    }
    e.preventDefault();
    this.x = e.clientX;
    this.y = e.clientY;
    this.target = this.find(e.clientX, e.clientY);
  };

  /** The pane under the point, read from the `data-pane-key` its element carries. */
  private find(x: number, y: number): PaneTarget | null {
    const el = document.elementsFromPoint(x, y).find((n): n is HTMLElement => n instanceof HTMLElement && !!n.dataset.paneKey);
    if (!el || !this.item) return null;
    const key = el.dataset.paneKey!;
    const shown = app.panesOn && app.panes ? leaves(app.panes.root) : app.singleItem ? [app.singleItem] : [];
    const item = shown.find((s) => itemKey(s) === key);
    if (!item) return null;
    const r = el.getBoundingClientRect();
    const aimed = zoneAt((x - r.left) / r.width, (y - r.top) / r.height);
    // A content moved from another pane frees its place.
    const count = shown.length - (app.panesOn && has(app.panes, this.item) ? 1 : 0);
    const fits = Object.fromEntries(ZONES.map((z) => [z, zoneFits(z, r.width, r.height, count)])) as Record<PaneZone, boolean>;
    return { item, rect: { left: r.left, top: r.top, width: r.width, height: r.height }, zone: fits[aimed] ? aimed : null, aimed, fits };
  }

  private up = (e: PointerEvent) => {
    const { on, target, item } = this;
    this.end();
    if (!on) return;
    e.preventDefault();
    // The click that follows the release is the drag's, not a click on the card.
    const swallow = (c: Event) => {
      c.stopPropagation();
      c.preventDefault();
    };
    window.addEventListener('click', swallow, { capture: true, once: true });
    setTimeout(() => window.removeEventListener('click', swallow, true), 0);
    if (item && target?.zone) app.dropInPane(target.item, target.zone, item);
  };

  private key = (e: KeyboardEvent) => {
    if (e.key !== 'Escape' || !this.item) return;
    if (this.on) {
      e.preventDefault();
      e.stopPropagation();
    }
    this.end();
  };

  private cancel = () => this.end();

  private end() {
    window.removeEventListener('pointermove', this.move, true);
    window.removeEventListener('pointerup', this.up, true);
    window.removeEventListener('pointercancel', this.cancel, true);
    window.removeEventListener('keydown', this.key, true);
    window.removeEventListener('blur', this.cancel);
    document.documentElement.classList.remove('pane-dragging');
    this.item = null;
    this.on = false;
    this.target = null;
  }
}

export const paneDrag = new PaneDrag();
