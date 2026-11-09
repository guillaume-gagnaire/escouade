import '../test/pointer';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { paneDrag } from './pane-drag.svelte';
import { app } from './state.svelte';

let card: HTMLElement;
let list: HTMLElement;
let at: Element[] = [];

const ev = (type: string, x: number, y: number) =>
  new PointerEvent(type, { clientX: x, clientY: y, button: 0, bubbles: true, cancelable: true });

beforeEach(() => {
  list = document.body.appendChild(document.createElement('div'));
  list.setAttribute('data-sidebar-drop', '');
  card = list.appendChild(document.createElement('div'));
  at = [];
  document.elementsFromPoint = () => at;
});

afterEach(() => {
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

/** A press on the card at (10, 10) with the given item and sidebar. */
function press(item: Parameters<typeof paneDrag.press>[1], sidebar?: Parameters<typeof paneDrag.press>[4]) {
  card.addEventListener('pointerdown', (e) => paneDrag.press(e as PointerEvent, item, 'x', '', sidebar));
  card.dispatchEvent(ev('pointerdown', 10, 10));
}

describe('paneDrag with the sidebar list', () => {
  it('a drag released over the sidebar list drops there, not in a pane', () => {
    const dropInPane = vi.spyOn(app, 'dropInPane').mockImplementation(() => {});
    const drop = vi.fn();
    press({ kind: 'agent', id: 'a1' }, { id: 'a1', drop });
    at = [list];
    window.dispatchEvent(ev('pointermove', 10, 60));
    expect(paneDrag.on).toBe(true);
    expect(paneDrag.sidebarId).toBe('a1');
    expect(paneDrag.inSidebar).toBe(true);
    expect(paneDrag.target).toBeNull();
    window.dispatchEvent(ev('pointerup', 10, 60));
    expect(drop).toHaveBeenCalledOnce();
    expect(dropInPane).not.toHaveBeenCalled();
    expect(paneDrag.sidebarId).toBeNull();
  });

  it('a folder (no item) over a pane has no target', () => {
    const pane = document.body.appendChild(document.createElement('div'));
    pane.dataset.paneKey = 'agent:a1';
    press(null, { id: 'f1', drop: vi.fn() });
    at = [pane];
    window.dispatchEvent(ev('pointermove', 10, 60));
    expect(paneDrag.on).toBe(true);
    expect(paneDrag.inSidebar).toBe(false);
    expect(paneDrag.target).toBeNull();
  });

  it('under the threshold nothing happens', () => {
    const drop = vi.fn();
    press({ kind: 'agent', id: 'a1' }, { id: 'a1', drop });
    at = [list];
    window.dispatchEvent(ev('pointermove', 12, 12));
    window.dispatchEvent(ev('pointerup', 12, 12));
    expect(paneDrag.on).toBe(false);
    expect(drop).not.toHaveBeenCalled();
  });
});
