import { undo } from '@codemirror/commands';
import { javascript } from '@codemirror/lang-javascript';
import { ensureSyntaxTree, indentUnit, syntaxTree } from '@codemirror/language';
import { openSearchPanel } from '@codemirror/search';
import { EditorSelection, Transaction } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import type { NavResolver } from '../../lib/editor/goto';
import CodeEditor from './CodeEditor.svelte';

const none = { changed: [], deleted: [], count: 0 };
const base = { indent: { tabs: false, size: 2 }, changes: none, oncursor: () => {} };

function viewOf(container: HTMLElement): EditorView {
  return EditorView.findFromDOM(container.querySelector('.cm-editor') as HTMLElement)!;
}

/** A click on the text; jsdom has no layout, CodeMirror's own selection takes it at position 0. */
function click(view: EditorView, init: MouseEventInit) {
  const o = { bubbles: true, cancelable: true, button: 0, detail: 1, clientX: 1, clientY: 1, ...init };
  view.contentDOM.dispatchEvent(new MouseEvent('mousedown', o));
  view.contentDOM.dispatchEvent(new MouseEvent('mouseup', o));
}

const press = (el: EventTarget, key: string, init: KeyboardEventInit = {}) =>
  el.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));

/** Navigation from `from` to `to` (`def` in `abc def`), recording the positions it was asked about. */
function navOn(asked: number[] = [], from = 4, to = 7) {
  const resolver: NavResolver = ({ pos }) => {
    asked.push(pos);
    return pos >= from && pos <= to ? { from, to, resolve: async () => [{ path: 'b.ts', line: 2 }] } : null;
  };
  return { path: 'a.ts', files: ['a.ts', 'b.ts'], resolvers: [resolver] };
}

describe('CodeEditor', () => {
  it('shows the text and reports what is typed', async () => {
    const onchange = vi.fn();
    const { container } = render(CodeEditor, { ...base, docKey: 'k1', text: 'const a = 1;\n', version: 0, onchange });
    await tick();
    const view = viewOf(container);
    expect(view.state.doc.toString()).toBe('const a = 1;\n');
    view.dispatch({ changes: { from: 0, insert: '// x\n' } });
    expect(onchange).toHaveBeenLastCalledWith('// x\nconst a = 1;\n');
  });

  it('takes a reloaded text without reporting it as typed, and another file in full', async () => {
    const onchange = vi.fn();
    const { container, rerender } = render(CodeEditor, { ...base, docKey: 'k1', text: 'a\n', version: 0, onchange });
    await tick();
    await rerender({ ...base, docKey: 'k1', text: 'agent\n', version: 1, onchange });
    expect(viewOf(container).state.doc.toString()).toBe('agent\n');
    await rerender({ ...base, docKey: 'k2', text: 'other\n', version: 0, onchange });
    expect(viewOf(container).state.doc.toString()).toBe('other\n');
    expect(onchange).not.toHaveBeenCalled();
  });

  it('reports the cursor line and column', async () => {
    const oncursor = vi.fn();
    const { container } = render(CodeEditor, { ...base, oncursor, docKey: 'k1', text: 'ab\ncd\n', version: 0, onchange: () => {} });
    await tick();
    viewOf(container).dispatch({ selection: { anchor: 4 } });
    expect(oncursor).toHaveBeenLastCalledWith({ line: 2, col: 2 });
  });

  it('keeps reporting what is typed after a reloaded text', async () => {
    const onchange = vi.fn();
    const { container, rerender } = render(CodeEditor, { ...base, docKey: 'k1', text: 'a\n', version: 0, onchange });
    await tick();
    await rerender({ ...base, docKey: 'k1', text: 'agent\n', version: 1, onchange });
    viewOf(container).dispatch({ changes: { from: 0, insert: 'x' } });
    expect(onchange).toHaveBeenCalledTimes(1);
    expect(onchange).toHaveBeenLastCalledWith('xagent\n');
  });

  it('keeps the cursor in place when the text is reloaded, within the new text', async () => {
    const props = { ...base, docKey: 'k1', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, { ...props, text: 'abc\ndef\n' });
    await tick();
    viewOf(container).dispatch({ selection: { anchor: 6 } });
    await rerender({ ...props, text: 'abc\ndef\nghi\n', version: 1 });
    expect(viewOf(container).state.selection.main.head).toBe(6);
    await rerender({ ...props, text: 'a', version: 2 });
    expect(viewOf(container).state.selection.main.head).toBe(1);
  });

  it('cannot be undone back over a text reloaded from disk, only over what was typed', async () => {
    const onchange = vi.fn();
    const props = { ...base, docKey: 'k1', onchange };
    const { container, rerender } = render(CodeEditor, { ...props, text: 'a\nb\n', version: 0 });
    await tick();
    const view = viewOf(container);
    view.dispatch({ changes: { from: 4, insert: 'x' }, userEvent: 'input.type' });
    await rerender({ ...props, text: 'new\na\nb\nx', version: 1 });
    expect(view.state.doc.toString()).toBe('new\na\nb\nx');
    onchange.mockClear();
    undo(view);
    expect(view.state.doc.toString()).toBe('new\na\nb\n');
    expect(onchange).toHaveBeenLastCalledWith('new\na\nb\n');
    undo(view);
    expect(view.state.doc.toString()).toBe('new\na\nb\n');
  });

  it('does not bring back, by undoing, a text the reload replaced', async () => {
    const onchange = vi.fn();
    const props = { ...base, docKey: 'k1', onchange };
    const { container, rerender } = render(CodeEditor, { ...props, text: 'a\n', version: 0 });
    await tick();
    const view = viewOf(container);
    view.dispatch({ changes: { from: 0, insert: 'x' }, userEvent: 'input.type' });
    await rerender({ ...props, text: 'agent\n', version: 1 });
    onchange.mockClear();
    undo(view);
    expect(view.state.doc.toString()).toBe('agent\n');
    expect(onchange).not.toHaveBeenCalledWith('xa\n');
    expect(onchange).not.toHaveBeenCalledWith('a\n');
  });

  it('keeps the cursor on the same text when lines are added above it by a reload', async () => {
    const props = { ...base, docKey: 'k1', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, { ...props, text: 'a\nb\nc\n' });
    await tick();
    const view = viewOf(container);
    view.dispatch({ selection: { anchor: 3 } });
    expect(view.state.doc.lineAt(3).number).toBe(2);
    await rerender({ ...props, text: 'x\ny\na\nb\nc\n', version: 1 });
    const head = view.state.selection.main.head;
    expect(view.state.doc.lineAt(head).number).toBe(4);
    expect(view.state.doc.lineAt(head).text).toBe('b');
  });

  it('keeps several cursors across a reload', async () => {
    const props = { ...base, docKey: 'k1', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, { ...props, text: 'a\nb\nc\n' });
    await tick();
    const view = viewOf(container);
    view.dispatch({ selection: EditorSelection.create([EditorSelection.cursor(3), EditorSelection.cursor(5)]) });
    await rerender({ ...props, text: 'x\ny\na\nb\nc\n', version: 1 });
    expect(view.state.selection.ranges.map((r) => view.state.doc.lineAt(r.head).text)).toEqual(['b', 'c']);
  });

  it('finds what changed in a text with Windows line breaks too', async () => {
    const props = { ...base, docKey: 'k1', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, { ...props, text: 'a\r\nb\r\n' });
    await tick();
    const view = viewOf(container);
    view.dispatch({ selection: { anchor: 3 } });
    await rerender({ ...props, text: 'x\r\na\r\nb\r\n', version: 1 });
    expect(view.state.doc.toString()).toBe('x\na\nb\n');
    expect(view.state.doc.lineAt(view.state.selection.main.head).text).toBe('b');
  });

  it('leaves the document and its undo history alone when the reload brings nothing new', async () => {
    const onchange = vi.fn();
    const props = { ...base, docKey: 'k1', onchange };
    const { container, rerender } = render(CodeEditor, { ...props, text: 'a\n', version: 0 });
    await tick();
    const view = viewOf(container);
    // typed a while ago, so that the history keeps it apart from what comes next
    view.dispatch({ changes: { from: 0, insert: 'x' }, annotations: Transaction.time.of(Date.now() - 1000) });
    await rerender({ ...props, text: 'xa\n', version: 1 });
    expect(view.state.doc.toString()).toBe('xa\n');
    expect(onchange).toHaveBeenCalledTimes(1);
    undo(view);
    expect(view.state.doc.toString()).toBe('a\n');
  });

  it('allows several cursors', async () => {
    const { container } = render(CodeEditor, { ...base, docKey: 'k1', text: 'ab\ncd\n', version: 0, onchange: () => {} });
    await tick();
    const view = viewOf(container);
    view.dispatch({ selection: EditorSelection.create([EditorSelection.cursor(0), EditorSelection.cursor(2)]) });
    expect(view.state.selection.ranges.length).toBe(2);
  });

  it('reports the cursor when a file is shown, at first and on every switch', async () => {
    const oncursor = vi.fn();
    const props = { ...base, oncursor, version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, { ...props, docKey: 'k1', text: 'ab\ncd\n' });
    await tick();
    expect(oncursor).toHaveBeenCalledWith({ line: 1, col: 1 });
    viewOf(container).dispatch({ selection: { anchor: 4 } });
    expect(oncursor).toHaveBeenLastCalledWith({ line: 2, col: 2 });
    const calls = oncursor.mock.calls.length;
    await rerender({ ...props, docKey: 'k2', text: 'xyz\n' });
    expect(oncursor.mock.calls.length).toBeGreaterThan(calls);
    expect(oncursor).toHaveBeenLastCalledWith({ line: 1, col: 1 });
  });

  it('marks the changed lines in the gutter, follows new changes and applies those of another file', async () => {
    const marked = (c: HTMLElement) => [...c.querySelectorAll('.cm-change')].map((e) => e.className);
    const props = { ...base, docKey: 'k1', text: 'a\nb\nc\n', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, { ...props, changes: { changed: [2], deleted: [], count: 1 } });
    await tick();
    expect(marked(container)).toEqual(['cm-change changed']);
    await rerender({ ...props, changes: { changed: [1], deleted: [3], count: 1 } });
    expect(marked(container)).toEqual(['cm-change changed', 'cm-change deleted']);
    await rerender({ ...props, docKey: 'k2', text: 'x\ny\n', changes: { changed: [2], deleted: [], count: 1 } });
    expect(marked(container)).toEqual(['cm-change changed']);
  });

  it('moves the cursor to the line to reveal, within the file', async () => {
    const props = { ...base, docKey: 'k1', text: 'a\nb\nc\n', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, props);
    await tick();
    await rerender({ ...props, reveal: { line: 2, seq: 1 } });
    expect(viewOf(container).state.selection.main.head).toBe(2);
    await rerender({ ...props, reveal: { line: 99, seq: 2 } });
    expect(viewOf(container).state.selection.main.head).toBe(6);
  });

  it('puts the cursor on the column to reveal, within the line', async () => {
    const props = { ...base, docKey: 'k1', text: 'a\nbcdef\nc\n', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, props);
    await tick();
    await rerender({ ...props, reveal: { line: 2, col: 3, seq: 1 } });
    expect(viewOf(container).state.selection.main.head).toBe(4);
    await rerender({ ...props, reveal: { line: 2, col: 99, seq: 2 } });
    expect(viewOf(container).state.selection.main.head).toBe(7);
  });

  it('counts the column to reveal and the one it reports in characters, an emoji being one', async () => {
    const oncursor = vi.fn();
    const props = { ...base, oncursor, docKey: 'k1', text: 'a\n😀é x\n', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, props);
    await tick();
    await rerender({ ...props, reveal: { line: 2, col: 4, seq: 1 } });
    // `x`: after the emoji (two UTF-16 units), `é` and the space.
    expect(viewOf(container).state.selection.main.head).toBe(6);
    expect(oncursor).toHaveBeenLastCalledWith({ line: 2, col: 4 });
  });

  it('tells when what a link leads to could not be found', async () => {
    const onnaverror = vi.fn();
    const failing: NavResolver = () => ({ from: 4, to: 7, resolve: () => Promise.reject('boom') });
    const nav = { path: 'a.ts', files: [], resolvers: [failing] };
    const { container } = render(CodeEditor, { ...base, docKey: 'k1', text: 'abc def\n', version: 0, onchange: () => {}, nav, onnaverror });
    await tick();
    const view = viewOf(container);
    view.dispatch({ selection: { anchor: 5 } });
    press(view.contentDOM, 'F12');
    await vi.waitFor(() => expect(onnaverror).toHaveBeenCalledExactlyOnceWith('boom'));
  });

  it('adds a cursor with Alt+click, and with Ctrl+click follows the link under the mouse instead', async () => {
    const ontargets = vi.fn();
    const props = { ...base, docKey: 'k1', text: 'abc def\n', version: 0, onchange: () => {}, nav: navOn(), ontargets };
    const { container } = render(CodeEditor, props);
    await tick();
    const view = viewOf(container);
    view.dispatch({ selection: { anchor: 5 } });
    click(view, { altKey: true });
    expect(view.state.selection.ranges.map((r) => r.head)).toEqual([0, 5]);
    view.dispatch({ selection: { anchor: 1 } });
    vi.spyOn(view, 'posAtCoords').mockReturnValue(5);
    click(view, { ctrlKey: true });
    expect(view.state.selection.ranges.map((r) => r.head)).toEqual([1]);
    await vi.waitFor(() =>
      expect(ontargets).toHaveBeenCalledExactlyOnceWith(
        [{ path: 'b.ts', line: 2 }],
        { path: 'a.ts', line: 1, col: 6 },
        expect.objectContaining({ label: 'def' }),
      ),
    );
  });

  it('places the cursor with Ctrl+click where nothing leads elsewhere', async () => {
    const ontargets = vi.fn();
    const props = { ...base, docKey: 'k1', text: 'abc def\n', version: 0, onchange: () => {}, nav: navOn(), ontargets };
    const { container } = render(CodeEditor, props);
    await tick();
    const view = viewOf(container);
    view.dispatch({ selection: { anchor: 5 } });
    click(view, { ctrlKey: true });
    expect(view.state.selection.ranges.map((r) => r.head)).toEqual([0]);
    expect(ontargets).not.toHaveBeenCalled();
  });

  it('follows the link at the cursor with F12', async () => {
    const ontargets = vi.fn();
    const asked: number[] = [];
    const props = { ...base, docKey: 'k1', text: 'xyz\nabc def\n', version: 0, onchange: () => {}, nav: navOn(asked, 8, 11), ontargets };
    const { container } = render(CodeEditor, props);
    await tick();
    const view = viewOf(container);
    view.dispatch({ selection: { anchor: 10 } });
    press(view.contentDOM, 'F12');
    await vi.waitFor(() =>
      expect(ontargets).toHaveBeenCalledExactlyOnceWith(
        [{ path: 'b.ts', line: 2 }],
        { path: 'a.ts', line: 2, col: 7 },
        expect.objectContaining({ label: 'def' }),
      ),
    );
    expect(asked).toEqual([10]);
  });

  it('goes back and forth with Alt+← and Alt+→, and the buttons of the mouse, in the editor only', async () => {
    const onback = vi.fn();
    const onforward = vi.fn();
    const props = { ...base, docKey: 'k1', text: 'abc def\n', version: 0, onchange: () => {}, nav: navOn(), onback, onforward };
    const { container } = render(CodeEditor, props);
    await tick();
    const view = viewOf(container);
    press(view.contentDOM, 'ArrowLeft', { altKey: true });
    expect(onback).toHaveBeenCalledTimes(1);
    press(view.contentDOM, 'ArrowRight', { altKey: true });
    expect(onforward).toHaveBeenCalledTimes(1);
    view.contentDOM.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, cancelable: true, button: 3 }));
    view.contentDOM.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, cancelable: true, button: 4 }));
    expect([onback.mock.calls.length, onforward.mock.calls.length]).toEqual([2, 2]);
    press(document.body, 'ArrowLeft', { altKey: true });
    expect(onback).toHaveBeenCalledTimes(2);
  });

  it('tells which request for a line it brought into view', async () => {
    const onrevealed = vi.fn();
    const props = { ...base, docKey: 'k1', text: 'a\nb\nc\n', version: 0, onchange: () => {}, onrevealed };
    const { container, rerender } = render(CodeEditor, props);
    await tick();
    expect(onrevealed).not.toHaveBeenCalled();
    await rerender({ ...props, reveal: { line: 3, seq: 7 } });
    expect(viewOf(container).state.selection.main.head).toBe(4);
    expect(onrevealed).toHaveBeenCalledExactlyOnceWith(7);
  });

  it('takes the language once it is loaded', async () => {
    const props = { ...base, docKey: 'k1', text: 'const a = 1;\n', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, props);
    await tick();
    expect(syntaxTree(viewOf(container).state).length).toBe(0);
    await rerender({ ...props, language: javascript() });
    expect(ensureSyntaxTree(viewOf(container).state, 100)?.type.name).toBe('Script');
  });

  it('opens its search panel in French', async () => {
    const { container } = render(CodeEditor, { ...base, docKey: 'k1', text: 'abc\n', version: 0, onchange: () => {} });
    await tick();
    expect(openSearchPanel(viewOf(container))).toBe(true);
    const panel = container.querySelector('.cm-search') as HTMLElement;
    expect(panel.querySelector('input[name=search]')?.getAttribute('placeholder')).toBe('Rechercher');
    expect(panel.querySelector('button[name=next]')?.textContent).toBe('suivant');
    expect(panel.querySelector('button[name=close]')?.getAttribute('aria-label')).toBe('fermer');
  });

  it('indents the way the project does, and follows a change of style', async () => {
    const props = { ...base, docKey: 'k1', text: '', version: 0, onchange: () => {} };
    const { container, rerender } = render(CodeEditor, { ...props, indent: { tabs: false, size: 4 } });
    await tick();
    expect(viewOf(container).state.facet(indentUnit)).toBe('    ');
    await rerender({ ...props, indent: { tabs: true, size: 4 } });
    expect(viewOf(container).state.facet(indentUnit)).toBe('\t');
  });

  it('measures its text again when the room it has changes width', async () => {
    // A column dragged next to it changes its width without the window moving.
    const Real = globalThis.ResizeObserver;
    const reports: ResizeObserverCallback[] = [];
    globalThis.ResizeObserver = class {
      constructor(cb: ResizeObserverCallback) {
        reports.push(cb);
      }
      observe() {}
      unobserve() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
    try {
      const { container } = render(CodeEditor, { ...base, docKey: 'k1', text: 'abc\n', version: 0, onchange: () => {} });
      await tick();
      const measure = vi.spyOn(viewOf(container), 'requestMeasure');
      for (const report of reports) report([{ contentRect: { width: 700 } } as ResizeObserverEntry], {} as ResizeObserver);
      expect(measure).toHaveBeenCalled();
    } finally {
      globalThis.ResizeObserver = Real;
    }
  });
});
