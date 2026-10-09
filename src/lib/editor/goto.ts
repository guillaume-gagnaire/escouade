// Code navigation in the editor. With Ctrl (Cmd on macOS) held, what leads elsewhere is underlined under the mouse,
// with a hand, and a click follows it; F12 follows it from the cursor. Resolvers tell what leads where (the links of
// files in `links.ts`); the editor view makes the jump and keeps the history that Alt+← and Alt+→ walk.

import { Prec, StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView, keymap, ViewPlugin } from '@codemirror/view';
import { IS_MAC, primaryKey } from '../platform';

/** Where a link leads: a file from the source's root (with `/`), and a place in it (1-based). */
export interface NavTarget {
  path: string;
  line?: number;
  col?: number;
}

/** A stretch of text that leads elsewhere; where to is only worked out when it is followed. */
export interface NavSpot {
  from: number;
  to: number;
  resolve: () => Promise<NavTarget[]>;
}

/** What a resolver looks at: a position in the file `path` of a source, whose tree has `files`. */
export interface NavContext {
  state: EditorState;
  pos: number;
  path: string;
  files: readonly string[];
}

export type NavResolver = (ctx: NavContext) => NavSpot | null;

/** The spot of the first resolver that finds one at `ctx.pos`. */
export function spotAt(resolvers: readonly NavResolver[], ctx: NavContext): NavSpot | null {
  for (const r of resolvers) {
    const s = r(ctx);
    if (s) return s;
  }
  return null;
}

export interface GotoOptions {
  resolvers: NavResolver[];
  /** Where a followed spot leads, and the place it was followed from (1-based). */
  onTargets: (t: NavTarget[], from: { line: number; col: number }) => void;
  /** The file shown, from the source's root, and the source's files; null while they are unknown (nothing leads anywhere). */
  context: () => { path: string; files: readonly string[] } | null;
  /** Alt+← / Alt+→ (Ctrl+- / Ctrl+Maj+- on macOS) and the back and forward buttons of the mouse. */
  onBack?: () => void;
  onForward?: () => void;
  mac?: boolean;
}

interface Stretch {
  from: number;
  to: number;
}

const setUnderline = StateEffect.define<Stretch | null>();
const LINK = Decoration.mark({ class: 'cm-goto' });

const underline = StateField.define<Stretch | null>({
  create: () => null,
  update(v, tr) {
    for (const e of tr.effects) if (e.is(setUnderline)) return e.value;
    // The text moved under it: the next move of the mouse finds the spot again.
    return tr.docChanged ? null : v;
  },
  provide: (f) => EditorView.decorations.from(f, (v) => (v ? Decoration.set(LINK.range(v.from, v.to)) : Decoration.none)),
});

/** The mouse's back and forward buttons. */
const BACK_BUTTON = 3;
const FORWARD_BUTTON = 4;

export function gotoExtension(o: GotoOptions): Extension {
  const mac = o.mac ?? IS_MAC;
  // Ctrl+Alt is AltGr on French keyboards, and Alt+click adds a cursor: neither navigates.
  const held = (e: MouseEvent | KeyboardEvent) => primaryKey(e, mac) && !e.altKey && !e.shiftKey;

  function spot(view: EditorView, pos: number): NavSpot | null {
    const c = o.context();
    const s = c ? spotAt(o.resolvers, { state: view.state, pos, path: c.path, files: c.files }) : null;
    return s && s.to > s.from ? s : null;
  }

  function follow(view: EditorView, s: NavSpot, pos: number) {
    const line = view.state.doc.lineAt(pos);
    const from = { line: line.number, col: pos - line.from + 1 };
    void s.resolve().then((t) => o.onTargets(t, from));
  }

  const hover = ViewPlugin.fromClass(
    class {
      /** Where the mouse is over the text, for a modifier pressed without moving it. */
      mouse: { x: number; y: number } | null = null;

      constructor(readonly view: EditorView) {
        // On the window: the modifier may be pressed while the focus is elsewhere, the mouse over the editor.
        window.addEventListener('keydown', this.key);
        window.addEventListener('keyup', this.key);
        window.addEventListener('blur', this.clear);
      }

      destroy() {
        window.removeEventListener('keydown', this.key);
        window.removeEventListener('keyup', this.key);
        window.removeEventListener('blur', this.clear);
      }

      key = (e: KeyboardEvent) => {
        if (!e.repeat) this.show(held(e));
      };

      clear = () => this.show(false);

      /** Underlines the spot under the mouse while the modifier is `on`, nothing otherwise. */
      show(on: boolean) {
        const pos = on && this.mouse ? this.view.posAtCoords(this.mouse) : null;
        const s = pos === null ? null : spot(this.view, pos);
        const now = this.view.state.field(underline, false) ?? null;
        if (now?.from === s?.from && now?.to === s?.to) return;
        this.view.dispatch({ effects: setUnderline.of(s && { from: s.from, to: s.to }) });
      }
    },
    {
      eventHandlers: {
        mousemove(e) {
          this.mouse = { x: e.clientX, y: e.clientY };
          this.show(held(e));
        },
        mouseleave() {
          this.mouse = null;
          this.show(false);
        },
        mousedown(e, view) {
          if (e.button !== 0 || !held(e)) return false;
          const pos = view.posAtCoords({ x: e.clientX, y: e.clientY });
          const s = pos === null ? null : spot(view, pos);
          // Nothing to follow there: the click places the cursor, as without the modifier.
          if (!s || pos === null) return false;
          e.preventDefault();
          follow(view, s, pos);
          return true;
        },
        mouseup(e) {
          const go = e.button === BACK_BUTTON ? o.onBack : e.button === FORWARD_BUTTON ? o.onForward : undefined;
          if (!go) return false;
          // Else the webview could take it for its own history.
          e.preventDefault();
          go();
          return true;
        },
      },
    },
  );

  const run = (f: (() => void) | undefined) => () => {
    f?.();
    return !!f;
  };

  return [
    underline,
    hover,
    // Over the default keymap, where Alt+← moves over a syntax element.
    Prec.high(
      keymap.of([
        {
          key: 'F12',
          run: (view) => {
            const pos = view.state.selection.main.head;
            const s = spot(view, pos);
            if (s) follow(view, s, pos);
            return true;
          },
        },
        { key: mac ? 'Ctrl--' : 'Alt-ArrowLeft', run: run(o.onBack) },
        { key: mac ? 'Ctrl-Shift--' : 'Alt-ArrowRight', run: run(o.onForward) },
      ]),
    ),
    EditorView.baseTheme({ '.cm-goto': { textDecoration: 'underline', cursor: 'pointer' } }),
  ];
}
