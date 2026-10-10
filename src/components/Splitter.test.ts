import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../test/pointer';
import Splitter from './Splitter.svelte';

const base = { value: 240, min: 160, max: 500, reset: 240, label: 'Largeur de la colonne des fichiers' };

function setup(over: Record<string, unknown> = {}) {
  const onresize = vi.fn();
  const oncommit = vi.fn();
  const view = render(Splitter, { ...base, onresize, oncommit, ...over });
  return { onresize, oncommit, handle: screen.getByRole('separator'), ...view };
}

const down = (el: Element, clientX: number, pointerId = 1) => fireEvent.pointerDown(el, { clientX, pointerId, button: 0 });
// A pointer that moves while it is down has its button pressed, as the browser reports it.
const move = (el: Element, clientX: number, pointerId = 1, buttons = 1) => fireEvent.pointerMove(el, { clientX, pointerId, buttons });
const up = (el: Element, pointerId = 1) => fireEvent.pointerUp(el, { pointerId });

describe('Splitter', () => {
  beforeEach(() => {
    // jsdom has no pointer capture.
    Element.prototype.setPointerCapture = vi.fn();
  });

  it('is a focusable vertical separator that tells its size and bounds', () => {
    const { handle } = setup();
    expect(handle).toHaveAccessibleName('Largeur de la colonne des fichiers');
    expect(handle).toHaveAttribute('aria-orientation', 'vertical');
    expect(handle).toHaveAttribute('aria-valuenow', '240');
    expect(handle).toHaveAttribute('aria-valuemin', '160');
    expect(handle).toHaveAttribute('aria-valuemax', '500');
    expect(handle).toHaveAttribute('tabindex', '0');
  });

  describe('dragged', () => {
    it('follows the pointer from the width it started at, and keeps the width where it is let go', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      expect(Element.prototype.setPointerCapture).toHaveBeenCalledWith(1);
      await move(handle, 340);
      expect(onresize).toHaveBeenLastCalledWith(280);
      await move(handle, 260);
      expect(onresize).toHaveBeenLastCalledWith(200);
      expect(oncommit).not.toHaveBeenCalled();
      await up(handle);
      expect(oncommit).toHaveBeenCalledOnce();
      expect(oncommit).toHaveBeenCalledWith(200);
    });

    it('moves by whole pixels, whatever the screen’s scale gives the pointer', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await move(handle, 340.6);
      expect(onresize).toHaveBeenLastCalledWith(281);
      await up(handle);
      expect(oncommit).toHaveBeenCalledWith(281);
    });

    it('stops at the smallest and at the largest width', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await move(handle, -400);
      expect(onresize).toHaveBeenLastCalledWith(160);
      await move(handle, 5000);
      expect(onresize).toHaveBeenLastCalledWith(500);
      await up(handle);
      expect(oncommit).toHaveBeenCalledWith(500);
    });

    it('does not repeat a width it already has', async () => {
      const { handle, onresize } = setup({ value: 160 });
      await down(handle, 300);
      await move(handle, 200);
      await move(handle, 100);
      expect(onresize).not.toHaveBeenCalled();
    });

    it('is a plain click, nothing kept, when the pointer did not move', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await up(handle);
      expect(onresize).not.toHaveBeenCalled();
      expect(oncommit).not.toHaveBeenCalled();
    });

    it('keeps the width it ends at even when that is the one it started at, as the column was told of the moves', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await move(handle, 340);
      await move(handle, 300);
      expect(onresize).toHaveBeenLastCalledWith(240);
      await up(handle);
      expect(oncommit).toHaveBeenCalledWith(240);
    });

    it('ignores a pointer that is not down on it', async () => {
      const { handle, onresize } = setup();
      await move(handle, 400);
      await down(handle, 300, 1);
      await move(handle, 400, 2);
      expect(onresize).not.toHaveBeenCalled();
    });

    it('ignores any button but the main one', async () => {
      const { handle, onresize } = setup();
      await fireEvent.pointerDown(handle, { clientX: 300, pointerId: 1, button: 2 });
      await move(handle, 340);
      expect(onresize).not.toHaveBeenCalled();
    });

    it('is over once the pointer is cancelled or the capture lost', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await move(handle, 320);
      await fireEvent.pointerCancel(handle, { pointerId: 1 });
      expect(oncommit).toHaveBeenCalledWith(260);
      await move(handle, 400);
      expect(onresize).toHaveBeenCalledTimes(1);

      await down(handle, 300);
      await move(handle, 310);
      await fireEvent.lostPointerCapture(handle, { pointerId: 1 });
      expect(oncommit).toHaveBeenLastCalledWith(250);
      await up(handle);
      expect(oncommit).toHaveBeenCalledTimes(2);
    });

    it('is over when the pointer moves with no button pressed any more: the release was missed', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await move(handle, 340);
      expect(onresize).toHaveBeenLastCalledWith(280);
      // The button was let go where no event came back (over a menu, out of the window): the next move says so.
      await move(handle, 380, 1, 0);
      expect(oncommit).toHaveBeenCalledOnce();
      expect(oncommit).toHaveBeenCalledWith(280);
      expect(onresize).toHaveBeenCalledTimes(1);
      expect(handle).not.toHaveClass('dragging');
      // And the column does not follow the pointer any more, button pressed again or not.
      await move(handle, 420);
      expect(onresize).toHaveBeenCalledTimes(1);
      await up(handle);
      expect(oncommit).toHaveBeenCalledOnce();
    });

    it('keeps nothing when the pointer moves with no button pressed before the column moved', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await move(handle, 340, 1, 0);
      expect(onresize).not.toHaveBeenCalled();
      expect(oncommit).not.toHaveBeenCalled();
      expect(handle).not.toHaveClass('dragging');
    });

    it('is over when the window loses the focus', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await move(handle, 340);
      await fireEvent.blur(window);
      expect(oncommit).toHaveBeenCalledOnce();
      expect(oncommit).toHaveBeenCalledWith(280);
      expect(handle).not.toHaveClass('dragging');
      await move(handle, 400);
      expect(onresize).toHaveBeenCalledTimes(1);
      // Nothing is dragged any more: a blur and the release that comes after change nothing.
      await fireEvent.blur(window);
      await up(handle);
      expect(oncommit).toHaveBeenCalledOnce();
    });

    it('does not mind a blur when nothing is dragged', async () => {
      const { oncommit } = setup();
      await fireEvent.blur(window);
      expect(oncommit).not.toHaveBeenCalled();
    });

    it('starts again from a new press when the previous drag never ended, keeping what it had moved', async () => {
      const { handle, onresize, oncommit, rerender } = setup();
      await down(handle, 300, 1);
      await move(handle, 340, 1);
      expect(onresize).toHaveBeenLastCalledWith(280);
      // The column now has the width it was told of; the release of the first press never came.
      await rerender({ value: 280 });
      await down(handle, 500, 2);
      expect(oncommit).toHaveBeenCalledOnce();
      expect(oncommit).toHaveBeenCalledWith(280);
      expect(handle).toHaveClass('dragging');
      // The old pointer is forgotten, the new one is followed from where the column is.
      await move(handle, 600, 1);
      expect(onresize).toHaveBeenCalledTimes(1);
      await move(handle, 520, 2);
      expect(onresize).toHaveBeenLastCalledWith(300);
      await up(handle, 2);
      expect(oncommit).toHaveBeenLastCalledWith(300);
      expect(oncommit).toHaveBeenCalledTimes(2);
    });

    it('starts again from a new press of the same pointer when the previous drag never ended', async () => {
      const { handle, onresize, oncommit } = setup();
      await down(handle, 300);
      await down(handle, 400);
      await move(handle, 420);
      expect(onresize).toHaveBeenLastCalledWith(260);
      await up(handle);
      expect(oncommit).toHaveBeenCalledOnce();
      expect(oncommit).toHaveBeenCalledWith(260);
    });

    it('marks the handle while it is dragged', async () => {
      const { handle } = setup();
      expect(handle).not.toHaveClass('dragging');
      await down(handle, 300);
      expect(handle).toHaveClass('dragging');
      await up(handle);
      expect(handle).not.toHaveClass('dragging');
    });
  });

  describe('double-clicked', () => {
    it('goes back to the usual width', async () => {
      const { handle, onresize, oncommit } = setup({ value: 400 });
      await fireEvent.dblClick(handle);
      expect(onresize).toHaveBeenCalledWith(240);
      expect(oncommit).toHaveBeenCalledWith(240);
    });

    it('stays within the bounds when the usual width does not fit', async () => {
      const { handle, oncommit } = setup({ value: 200, min: 160, max: 220 });
      await fireEvent.dblClick(handle);
      expect(oncommit).toHaveBeenCalledWith(220);
    });

    it('does nothing at the usual width already', async () => {
      const { handle, onresize, oncommit } = setup();
      await fireEvent.dblClick(handle);
      expect(onresize).not.toHaveBeenCalled();
      expect(oncommit).not.toHaveBeenCalled();
    });
  });

  describe('with the keyboard', () => {
    it('moves by 16 px with the arrows', async () => {
      const { handle, onresize, oncommit, rerender } = setup();
      handle.focus();
      await userEvent.keyboard('{ArrowRight}');
      expect(onresize).toHaveBeenLastCalledWith(256);
      expect(oncommit).toHaveBeenLastCalledWith(256);
      await rerender({ value: 256 });
      await userEvent.keyboard('{ArrowLeft}{ArrowLeft}');
      expect(onresize).toHaveBeenLastCalledWith(240);
      expect(oncommit).toHaveBeenCalledTimes(3);
    });

    it('goes to the bounds with Home and End', async () => {
      const { handle, onresize, oncommit } = setup();
      handle.focus();
      await userEvent.keyboard('{Home}');
      expect(oncommit).toHaveBeenLastCalledWith(160);
      await userEvent.keyboard('{End}');
      expect(onresize).toHaveBeenLastCalledWith(500);
      expect(oncommit).toHaveBeenLastCalledWith(500);
    });

    it('stops at the bounds', async () => {
      const { handle, onresize } = setup({ value: 490 });
      handle.focus();
      await userEvent.keyboard('{ArrowRight}');
      expect(onresize).toHaveBeenLastCalledWith(500);
    });

    it('says nothing when it is at the bound already', async () => {
      const { handle, onresize, oncommit } = setup({ value: 500 });
      handle.focus();
      await userEvent.keyboard('{ArrowRight}{End}');
      expect(onresize).not.toHaveBeenCalled();
      expect(oncommit).not.toHaveBeenCalled();
    });

    it('leaves the shortcuts with a modifier to the app', async () => {
      const { handle, onresize } = setup();
      handle.focus();
      await userEvent.keyboard('{Control>}{ArrowRight}{/Control}{Alt>}{ArrowLeft}{/Alt}{Meta>}{End}{/Meta}');
      expect(onresize).not.toHaveBeenCalled();
    });
  });
});
