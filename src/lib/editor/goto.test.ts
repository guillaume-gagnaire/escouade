import { EditorState, type Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { charColumn, columnOffset, gotoExtension, nameAt, spotAt, type GotoOptions, type NavResolver, type NavTarget } from './goto';

/** `def` (4 to 7 in `abc def`) leads to b.ts. */
const def: NavResolver = ({ pos }) => (pos >= 4 && pos <= 7 ? { from: 4, to: 7, resolve: async () => [{ path: 'b.ts' }] } : null);

let view: EditorView | undefined;
let ext: Extension = [];
afterEach(() => view?.destroy());

/** An editor showing `doc` with the navigation, the mouse taken to be over position `at` (jsdom has no layout). */
function editor(o: Partial<GotoOptions> = {}, at = 5, doc = 'abc def'): EditorView {
  ext = gotoExtension({ resolvers: [def], onTargets: () => {}, context: () => ({ path: 'a.ts', files: ['b.ts'] }), ...o });
  view = new EditorView({ state: EditorState.create({ doc, extensions: ext }), parent: document.body });
  vi.spyOn(view, 'posAtCoords').mockReturnValue(at);
  return view;
}

const mouse = (v: EditorView, type: string, init: MouseEventInit = {}) =>
  v.contentDOM.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, clientX: 1, clientY: 1, button: 0, ...init }));
const key = (type: string, init: KeyboardEventInit) => window.dispatchEvent(new KeyboardEvent(type, { bubbles: true, ...init }));
const underlined = (v: EditorView) => [...v.dom.querySelectorAll('.cm-goto')].map((e) => e.textContent);

describe('spotAt', () => {
  it('takes the spot of the first resolver that finds one', () => {
    const third = vi.fn(() => null);
    const other: NavResolver = () => ({ from: 0, to: 3, resolve: async () => [] });
    const ctx = { state: EditorState.create({ doc: 'abc def' }), pos: 5, path: 'a.ts', files: [] };
    expect(spotAt([() => null, def, other, third], ctx)).toMatchObject({ from: 4, to: 7 });
    expect(third).not.toHaveBeenCalled();
  });
});

describe('gotoExtension', () => {
  it('underlines what leads elsewhere under the mouse while Ctrl is held, until it is released', () => {
    const v = editor();
    mouse(v, 'mousemove');
    expect(underlined(v)).toEqual([]);
    mouse(v, 'mousemove', { ctrlKey: true });
    expect(underlined(v)).toEqual(['def']);
    key('keyup', { key: 'Control' });
    expect(underlined(v)).toEqual([]);
    // Pressed with the mouse still over it.
    key('keydown', { key: 'Control', ctrlKey: true });
    expect(underlined(v)).toEqual(['def']);
  });

  it('shows the hand over what is underlined', () => {
    editor();
    const css = [...document.querySelectorAll('style')].map((s) => s.textContent ?? '').join('\n');
    expect(css).toMatch(/\.cm-goto\s*\{[^}]*text-decoration:\s*underline[^}]*cursor:\s*pointer/);
  });

  it('takes the underline away when the mouse leaves the spot or the text, or the window loses the focus', () => {
    const v = editor();
    mouse(v, 'mousemove', { ctrlKey: true });
    vi.mocked(v.posAtCoords).mockReturnValue(1);
    mouse(v, 'mousemove', { ctrlKey: true });
    expect(underlined(v)).toEqual([]);
    vi.mocked(v.posAtCoords).mockReturnValue(5);
    mouse(v, 'mousemove', { ctrlKey: true });
    mouse(v, 'mouseleave', { ctrlKey: true });
    expect(underlined(v)).toEqual([]);
    mouse(v, 'mousemove', { ctrlKey: true });
    window.dispatchEvent(new Event('blur'));
    expect(underlined(v)).toEqual([]);
  });

  it('underlines nothing where nothing leads, with AltGr (Ctrl+Alt), or before the file is known', () => {
    mouse(editor({}, 1), 'mousemove', { ctrlKey: true });
    expect(underlined(view!)).toEqual([]);
    view!.destroy();
    mouse(editor(), 'mousemove', { ctrlKey: true, altKey: true });
    expect(underlined(view!)).toEqual([]);
    view!.destroy();
    mouse(editor({ context: () => null }), 'mousemove', { ctrlKey: true });
    expect(underlined(view!)).toEqual([]);
  });

  it('follows with Cmd+click and goes back and forth with Ctrl+- and Ctrl+Shift+- on macOS', async () => {
    const onTargets = vi.fn();
    const onBack = vi.fn();
    const onForward = vi.fn();
    const v = editor({ mac: true, onTargets, onBack, onForward });
    mouse(v, 'mousedown', { ctrlKey: true });
    mouse(v, 'mouseup', { ctrlKey: true });
    expect(onTargets).not.toHaveBeenCalled();
    mouse(v, 'mousedown', { metaKey: true });
    mouse(v, 'mouseup', { metaKey: true });
    await vi.waitFor(() => expect(onTargets).toHaveBeenCalledOnce());
    expect(onTargets.mock.calls[0].slice(0, 2)).toEqual([[{ path: 'b.ts' }], { path: 'a.ts', line: 1, col: 6 }]);
    const press = (init: KeyboardEventInit) =>
      v.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }));
    press({ key: 'ArrowLeft', altKey: true });
    expect(onBack).not.toHaveBeenCalled();
    press({ key: '-', keyCode: 189, ctrlKey: true });
    expect(onBack).toHaveBeenCalledTimes(1);
    press({ key: '_', keyCode: 189, ctrlKey: true, shiftKey: true });
    expect(onForward).toHaveBeenCalledTimes(1);
  });

  it('looks for the uses of the identifier at the cursor with Shift+F12, whatever it is in', () => {
    const onReferences = vi.fn();
    const v = editor({ onReferences }, 5, 'const café = 1; // café');
    const shiftF12 = () =>
      v.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { key: 'F12', shiftKey: true, bubbles: true, cancelable: true }));
    v.dispatch({ selection: { anchor: 8 } });
    shiftF12();
    expect(onReferences).toHaveBeenLastCalledWith('café');
    // A comment's word too: its uses are found by their text.
    v.dispatch({ selection: { anchor: 23 } });
    shiftF12();
    expect(onReferences).toHaveBeenCalledTimes(2);
    // Neither an operator nor a number names anything.
    for (const anchor of [11, 13]) {
      v.dispatch({ selection: { anchor } });
      shiftF12();
    }
    expect(onReferences).toHaveBeenCalledTimes(2);
    expect(nameAt(v.state, 8)).toEqual({ from: 6, to: 10, name: 'café' });
  });

  it('stops listening to the window once the editor is gone', () => {
    const v = editor();
    mouse(v, 'mousemove', { ctrlKey: true });
    const dispatch = vi.spyOn(v, 'dispatch');
    v.destroy();
    key('keyup', { key: 'Control' });
    expect(dispatch).not.toHaveBeenCalled();
  });
});

describe('following a spot', () => {
  /** A resolver for `def` whose answer comes when the test gives it. */
  function later() {
    const asked: { answer: (t: NavTarget[]) => void; fail: (e: unknown) => void }[] = [];
    const resolver: NavResolver = ({ pos }) =>
      pos >= 4 && pos <= 7
        ? { from: 4, to: 7, resolve: () => new Promise<NavTarget[]>((answer, fail) => asked.push({ answer, fail })) }
        : null;
    return { resolver, last: () => asked.at(-1)! };
  }
  const ctrlClick = (v: EditorView) => {
    mouse(v, 'mousedown', { ctrlKey: true });
    mouse(v, 'mouseup', { ctrlKey: true });
  };
  const settled = () => new Promise((r) => setTimeout(r, 0));

  it('tells where it was followed from, what the spot names and where it is on screen', async () => {
    const onTargets = vi.fn();
    const v = editor({ onTargets });
    ctrlClick(v);
    await vi.waitFor(() =>
      expect(onTargets).toHaveBeenCalledExactlyOnceWith(
        [{ path: 'b.ts' }],
        { path: 'a.ts', line: 1, col: 6 },
        {
          label: 'def',
          rect: v.coordsAtPos(4),
        },
      ),
    );
  });

  it('names the spot by its label when it has one', async () => {
    const onTargets = vi.fn();
    const named: NavResolver = (c) => ({ ...def(c)!, label: 'Def' });
    ctrlClick(editor({ resolvers: [named], onTargets }));
    await vi.waitFor(() => expect(onTargets).toHaveBeenCalledOnce());
    expect(onTargets.mock.calls[0][2]).toMatchObject({ label: 'Def' });
  });

  it('counts the column it was followed from in characters, an emoji being one', async () => {
    const onTargets = vi.fn();
    // `😀é def`: the emoji takes two UTF-16 units, `def` is from 4 to 7 as in `abc def`.
    ctrlClick(editor({ onTargets }, 5, '😀é def'));
    await vi.waitFor(() => expect(onTargets).toHaveBeenCalledOnce());
    expect(onTargets.mock.calls[0][1]).toEqual({ path: 'a.ts', line: 1, col: 5 });
  });

  it.each([
    ['another file shown', (v: EditorView) => v.setState(EditorState.create({ doc: 'abc def', extensions: ext }))],
    ['an edit', (v: EditorView) => v.dispatch({ changes: { from: 0, insert: 'x' } })],
    ['the editor gone', (v: EditorView) => v.destroy()],
  ])('drops an answer that comes after the user moved on: %s', async (_, moveOn) => {
    const l = later();
    const onTargets = vi.fn();
    const v = editor({ resolvers: [l.resolver], onTargets });
    ctrlClick(v);
    moveOn(v);
    l.last().answer([{ path: 'b.ts' }]);
    await settled();
    expect(onTargets).not.toHaveBeenCalled();
  });

  it('drops an answer that comes once the editor tells of another path', async () => {
    const l = later();
    const onTargets = vi.fn();
    let path = 'a.ts';
    const v = editor({ resolvers: [l.resolver], onTargets, context: () => ({ path, files: [] }) });
    ctrlClick(v);
    path = 'c.ts';
    l.last().answer([{ path: 'b.ts' }]);
    await settled();
    expect(onTargets).not.toHaveBeenCalled();
    // The same path still shown: the answer is taken.
    ctrlClick(v);
    l.last().answer([{ path: 'b.ts' }]);
    await vi.waitFor(() => expect(onTargets).toHaveBeenCalledOnce());
  });

  it('takes only the answer of the last spot followed, whichever comes first', async () => {
    const l = later();
    const onTargets = vi.fn();
    const onError = vi.fn();
    const v = editor({ resolvers: [l.resolver], onTargets, onError });
    ctrlClick(v);
    const first = l.last();
    ctrlClick(v);
    const second = l.last();
    first.answer([{ path: 'first.ts' }]);
    await settled();
    expect(onTargets).not.toHaveBeenCalled();
    second.answer([{ path: 'second.ts' }]);
    await vi.waitFor(() => expect(onTargets).toHaveBeenCalledOnce());
    expect(onTargets.mock.calls[0][0]).toEqual([{ path: 'second.ts' }]);
    // The later one answering first: the earlier one, answer or failure, comes too late.
    ctrlClick(v);
    const third = l.last();
    ctrlClick(v);
    l.last().answer([{ path: 'fourth.ts' }]);
    await vi.waitFor(() => expect(onTargets).toHaveBeenCalledTimes(2));
    third.answer([{ path: 'third.ts' }]);
    third.fail('late');
    await settled();
    expect(onTargets.mock.calls.map((c) => c[0][0].path)).toEqual(['second.ts', 'fourth.ts']);
    expect(onError).not.toHaveBeenCalled();
  });

  it('tells of a failure to find where a spot leads, unless the user moved on', async () => {
    const l = later();
    const onError = vi.fn();
    const v = editor({ resolvers: [l.resolver], onError });
    ctrlClick(v);
    l.last().fail('boom');
    await vi.waitFor(() => expect(onError).toHaveBeenCalledExactlyOnceWith('boom'));
    ctrlClick(v);
    v.dispatch({ changes: { from: 0, insert: 'x' } });
    l.last().fail('late');
    await settled();
    expect(onError).toHaveBeenCalledTimes(1);
  });

  it('leaves no failure unhandled without onError', async () => {
    const unhandled = vi.fn();
    process.on('unhandledRejection', unhandled);
    try {
      const l = later();
      ctrlClick(editor({ resolvers: [l.resolver] }));
      l.last().fail('boom');
      await new Promise((r) => setTimeout(r, 20));
      expect(unhandled).not.toHaveBeenCalled();
    } finally {
      process.off('unhandledRejection', unhandled);
    }
  });
});

describe('columns in characters', () => {
  it('turns a column in characters into a UTF-16 offset of the line, and back', () => {
    const line = '😀é x';
    expect(columnOffset(line, 1)).toBe(0);
    expect(columnOffset(line, 2)).toBe(2);
    expect(columnOffset(line, 4)).toBe(4);
    expect(columnOffset(line, 99)).toBe(line.length);
    expect(charColumn(line, 0)).toBe(1);
    expect(charColumn(line, 4)).toBe(4);
    expect(charColumn(line, line.length)).toBe(5);
  });
});
