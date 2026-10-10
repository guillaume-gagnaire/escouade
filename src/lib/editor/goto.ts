// Code navigation in the editor. With Ctrl (Cmd on macOS) held, what leads elsewhere is underlined under the mouse,
// with a hand, and a click follows it; F12 follows it from the cursor. Resolvers tell what leads where (the links of
// files in `links.ts`); the editor view makes the jump and keeps the history that Alt+← and Alt+→ walk.

import { Prec, StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView, keymap, ViewPlugin, type Rect } from '@codemirror/view';
import { IS_MAC, primaryKey } from '../platform';

/**
 * Where a link leads: a file from the source's root (with `/`), and a place in it, 1-based, the column counted in
 * characters (code points: an emoji is one, as Rust counts them).
 */
export interface NavTarget {
  path: string;
  line?: number;
  col?: number;
  /** The text of that line, for a list of places to show. */
  text?: string;
}

/** A stretch of text that leads elsewhere; where to is only worked out when it is followed. */
export interface NavSpot {
  from: number;
  to: number;
  resolve: () => Promise<NavTarget[]>;
  /** What it names, for a message (« Aucune définition trouvée pour … »): its text by default. */
  label?: string;
}

/** The place a spot was followed from: its file, from the source's root, line and column (1-based, in characters). */
export interface NavFrom {
  path: string;
  line: number;
  col: number;
}

/** The spot followed: what it names, and where it is on the screen (null when it is not drawn), to show a list under it. */
export interface NavFollowed {
  label: string;
  rect: Rect | null;
}

/** The 1-based column, in characters, of the UTF-16 `offset` in the line `text`. */
export function charColumn(text: string, offset: number): number {
  const before = text.slice(0, offset);
  return before.length - (before.match(/[\uD800-\uDBFF][\uDC00-\uDFFF]/g)?.length ?? 0) + 1;
}

/** The UTF-16 offset in the line `text` of the 1-based column `col` counted in characters, at most the line's end. */
export function columnOffset(text: string, col: number): number {
  let offset = 0;
  for (let n = 1; n < col && offset < text.length; n++) offset += (text.codePointAt(offset) ?? 0) > 0xffff ? 2 : 1;
  return Math.min(offset, text.length);
}

/** An identifier, as most languages write them. */
const IDENTIFIER = /^[\p{L}_$][\p{L}\p{N}_$]*$/u;

/**
 * The identifier at `pos`, as the language cuts words; null on anything else (a space, an operator, a number). What
 * Ctrl+click looks a definition for, and Maj+F12 the uses of.
 */
export function nameAt(state: EditorState, pos: number): { from: number; to: number; name: string } | null {
  const w = state.wordAt(pos);
  const name = w ? state.sliceDoc(w.from, w.to) : '';
  return w && IDENTIFIER.test(name) ? { from: w.from, to: w.to, name } : null;
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
  /**
   * Where a followed spot leads, the place it was followed from and the spot. Only while the user is still there: an
   * answer coming once another file is shown, the text edited or the editor gone is dropped.
   */
  onTargets: (t: NavTarget[], from: NavFrom, spot: NavFollowed) => void;
  /** Where a followed spot leads could not be worked out (dropped too once the user moved on). */
  onError?: (e: unknown) => void;
  /** The file shown, from the source's root, and the source's files; null while they are unknown (nothing leads anywhere). */
  context: () => { path: string; files: readonly string[] } | null;
  /** Alt+← / Alt+→ (Ctrl+- / Ctrl+Maj+- on macOS) and the back and forward buttons of the mouse. */
  onBack?: () => void;
  onForward?: () => void;
  /** Maj+F12: the identifier at the cursor, whose uses are looked for (by their text: in a comment too). */
  onReferences?: (name: string) => void;
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
    const c = o.context();
    const plugin = view.plugin(hover);
    if (!c || !plugin) return;
    const doc = view.state.doc;
    const line = doc.lineAt(pos);
    const from: NavFrom = { path: c.path, line: line.number, col: charColumn(line.text, pos - line.from) };
    const label = s.label ?? view.state.sliceDoc(s.from, s.to);
    const seq = ++plugin.followed;
    // A search may take seconds: the user may have moved on meanwhile (the plugin goes with the file or the editor),
    // or followed another spot, whose answer is the only one to take.
    const here = () => plugin.live && plugin.followed === seq && view.state.doc === doc && o.context()?.path === c.path;
    s.resolve().then(
      (t) => {
        if (here()) o.onTargets(t, from, { label, rect: view.coordsAtPos(s.from) });
      },
      (e) => {
        if (here()) o.onError?.(e);
      },
    );
  }

  const hover = ViewPlugin.fromClass(
    class {
      /** Where the mouse is over the text, for a modifier pressed without moving it. */
      mouse: { x: number; y: number } | null = null;
      /** False once destroyed: with the editor, or when another file's state replaces this one. */
      live = true;
      /** How many spots were followed: only the answer of the last one is taken. */
      followed = 0;

      constructor(readonly view: EditorView) {
        // On the window: the modifier may be pressed while the focus is elsewhere, the mouse over the editor.
        window.addEventListener('keydown', this.key);
        window.addEventListener('keyup', this.key);
        window.addEventListener('blur', this.clear);
      }

      destroy() {
        this.live = false;
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
        {
          key: 'Shift-F12',
          run: (view) => {
            const id = nameAt(view.state, view.state.selection.main.head);
            if (id) o.onReferences?.(id.name);
            return !!o.onReferences;
          },
        },
        { key: mac ? 'Ctrl--' : 'Alt-ArrowLeft', run: run(o.onBack) },
        { key: mac ? 'Ctrl-Shift--' : 'Alt-ArrowRight', run: run(o.onForward) },
      ]),
    ),
    EditorView.baseTheme({ '.cm-goto': { textDecoration: 'underline', cursor: 'pointer' } }),
  ];
}
