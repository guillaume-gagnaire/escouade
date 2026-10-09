import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { gotoExtension, spotAt, type GotoOptions, type NavResolver } from './goto';

/** `def` (4 to 7 in `abc def`) leads to b.ts. */
const def: NavResolver = ({ pos }) => (pos >= 4 && pos <= 7 ? { from: 4, to: 7, resolve: async () => [{ path: 'b.ts' }] } : null);

let view: EditorView | undefined;
afterEach(() => view?.destroy());

/** An editor showing `abc def` with the navigation, the mouse taken to be over position `at` (jsdom has no layout). */
function editor(o: Partial<GotoOptions> = {}, at = 5): EditorView {
  const ext = gotoExtension({ resolvers: [def], onTargets: () => {}, context: () => ({ path: 'a.ts', files: ['b.ts'] }), ...o });
  view = new EditorView({ state: EditorState.create({ doc: 'abc def', extensions: ext }), parent: document.body });
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
    await vi.waitFor(() => expect(onTargets).toHaveBeenCalledExactlyOnceWith([{ path: 'b.ts' }], { line: 1, col: 6 }));
    const press = (init: KeyboardEventInit) =>
      v.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }));
    press({ key: 'ArrowLeft', altKey: true });
    expect(onBack).not.toHaveBeenCalled();
    press({ key: '-', keyCode: 189, ctrlKey: true });
    expect(onBack).toHaveBeenCalledTimes(1);
    press({ key: '_', keyCode: 189, ctrlKey: true, shiftKey: true });
    expect(onForward).toHaveBeenCalledTimes(1);
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
