import { history, undo } from '@codemirror/commands';
import { Compartment, EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { afterEach, describe, expect, it } from 'vitest';
import { comparison, showComparison, type Comparison } from './compare';

const views: EditorView[] = [];
afterEach(() => views.splice(0).forEach((v) => v.destroy()));

/** An editor showing `doc` compared as `c`, and how to bring another comparison in, as the editor does. */
function editor(doc: string, c: Comparison | null) {
  const slot = new Compartment();
  const view = new EditorView({ state: EditorState.create({ doc, extensions: [history(), comparison(slot, c)] }), parent: document.body });
  views.push(view);
  /** Brings `next` in; false when there was nothing to do. */
  const show = (next: Comparison | null) => {
    const tr = showComparison(view.state, slot, next);
    if (tr) view.dispatch(tr);
    return !!tr;
  };
  return { view, show };
}

/** The lines of the other version drawn above each block, block by block. */
const removed = (view: EditorView) =>
  [...view.dom.querySelectorAll('.cm-deletedChunk')].map((c) => [...c.querySelectorAll('.cm-deletedLine')].map((l) => l.textContent));
const buttons = (view: EditorView) => [...view.dom.querySelectorAll<HTMLButtonElement>('.cm-deletedChunk button')];

const REFERENCE = 'a\nb\nc\nd\ne\nf\n';
const TYPED = 'a\nB\nc\nd\ne\nF\n';

describe('comparison', () => {
  it('shows above each block that differs the lines the reference has there, with « Annuler ce bloc »', () => {
    const { view } = editor(TYPED, { original: REFERENCE, against: 'reference' });
    expect(removed(view)).toEqual([['b'], ['f']]);
    expect(buttons(view).map((b) => b.textContent)).toEqual(['Annuler ce bloc', 'Annuler ce bloc']);
    expect(buttons(view).every((b) => b.type === 'button')).toBe(true);
  });

  it('puts the block of the reference back, which an undo takes away again', () => {
    const { view } = editor(TYPED, { original: REFERENCE, against: 'reference' });
    buttons(view)[1].click();
    expect(view.state.doc.toString()).toBe('a\nB\nc\nd\ne\nf\n');
    expect(removed(view)).toEqual([['b']]);
    undo(view);
    expect(view.state.doc.toString()).toBe(TYPED);
    expect(removed(view)).toEqual([['b'], ['f']]);
  });

  it('takes the block of the disk with « Prendre ce bloc »', () => {
    const { view } = editor('a\nmine\nc\nd\ne\nf\n', { original: 'a\nb\nc\nd\ne\nagent\n', against: 'disk' });
    expect(buttons(view).map((b) => b.textContent)).toEqual(['Prendre ce bloc', 'Prendre ce bloc']);
    buttons(view)[1].click();
    expect(view.state.doc.toString()).toBe('a\nmine\nc\nd\ne\nagent\n');
  });

  it('keeps the focus in the text on a press of the mouse, and gives it back to the text after the button is used from the keyboard', () => {
    const { view } = editor(TYPED, { original: REFERENCE, against: 'reference' });
    const press = new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0 });
    buttons(view)[0].dispatchEvent(press);
    expect(press.defaultPrevented).toBe(true);
    buttons(view)[0].focus();
    expect(buttons(view)[0]).toHaveFocus();
    // Enter or Space on a button is a click.
    buttons(view)[0].click();
    expect(view.state.doc.toString()).toBe('a\nb\nc\nd\ne\nF\n');
    expect(view.hasFocus).toBe(true);
  });

  it('follows the text typed: a block made the same as the reference is no longer one', () => {
    const { view } = editor(TYPED, { original: REFERENCE, against: 'reference' });
    view.dispatch({ changes: { from: 2, to: 3, insert: 'b' } });
    expect(removed(view)).toEqual([['f']]);
  });

  it('shows nothing once taken off, and another version compared afresh', () => {
    const { view, show } = editor(TYPED, { original: REFERENCE, against: 'reference' });
    expect(show(null)).toBe(true);
    expect(view.dom.querySelector('.cm-deletedChunk')).toBeNull();
    show({ original: 'a\nB\nc\nd\ne\ndisk\n', against: 'disk' });
    expect(removed(view)).toEqual([['disk']]);
    expect(buttons(view).map((b) => b.textContent)).toEqual(['Prendre ce bloc']);
    // Straight from one version to another: none of the blocks of the first is left.
    show({ original: REFERENCE, against: 'reference' });
    expect(removed(view)).toEqual([['b'], ['f']]);
    expect(buttons(view).map((b) => b.textContent)).toEqual(['Annuler ce bloc', 'Annuler ce bloc']);
  });

  it('changes nothing for the comparison already shown, given again', () => {
    const { view, show } = editor(TYPED, null);
    expect(show(null)).toBe(false);
    show({ original: REFERENCE, against: 'reference' });
    const shown = view.dom.querySelector('.cm-deletedChunk');
    expect(show({ original: REFERENCE, against: 'reference' })).toBe(false);
    expect(view.dom.querySelector('.cm-deletedChunk')).toBe(shown);
  });

  it('compares a version with Windows line breaks line by line', () => {
    const { view } = editor(TYPED, { original: REFERENCE.split('\n').join('\r\n'), against: 'reference' });
    expect(removed(view)).toEqual([['b'], ['f']]);
  });
});
