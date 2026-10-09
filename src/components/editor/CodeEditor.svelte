<script lang="ts">
  import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import { bracketMatching, indentOnInput, indentUnit } from '@codemirror/language';
  import { search, searchKeymap } from '@codemirror/search';
  import { Compartment, EditorState, Transaction, type Extension } from '@codemirror/state';
  import { drawSelection, EditorView, highlightActiveLine, highlightActiveLineGutter, keymap, lineNumbers } from '@codemirror/view';
  import { onDestroy, onMount, untrack } from 'svelte';
  import type { LineChanges } from '../../lib/editor/changes';
  import {
    charColumn,
    columnOffset,
    gotoExtension,
    type NavFollowed,
    type NavFrom,
    type NavResolver,
    type NavTarget,
  } from '../../lib/editor/goto';
  import { changeGutter, setChanges } from '../../lib/editor/gutter';
  import { reloadChange } from '../../lib/editor/reload';
  import { editorTheme, PHRASES } from '../../lib/editor/theme';

  // `docKey` names the file shown: another one replaces the whole editor state. `version` changes
  // when the same file's text was replaced from disk.
  let {
    docKey,
    text,
    version,
    language = null,
    indent,
    changes,
    reveal = null,
    nav = null,
    onchange,
    oncursor,
    onrevealed,
    ontargets,
    onnaverror,
    onback,
    onforward,
  }: {
    docKey: string;
    text: string;
    version: number;
    language?: Extension | null;
    indent: { tabs: boolean; size: number };
    changes: LineChanges;
    /** A line to bring into view, the cursor on `col` (in characters) when given. */
    reveal?: { line: number; col?: number; seq: number } | null;
    /** What leads elsewhere in the file `path` of a source whose tree has `files` (resolvers read for each file shown). */
    nav?: { path: string; files: readonly string[]; resolvers: NavResolver[] } | null;
    onchange: (text: string) => void;
    /** The cursor's line and column, 1-based, the column in characters as links and searches count it. */
    oncursor: (pos: { line: number; col: number }) => void;
    /** The line of request `seq` is in view: the request is done, it must not move the cursor again. */
    onrevealed?: (seq: number) => void;
    /** A link followed (Ctrl+click, F12): where it leads, where it was followed from, and the spot. */
    ontargets?: (targets: NavTarget[], from: NavFrom, spot: NavFollowed) => void;
    /** Where a followed link leads could not be worked out. */
    onnaverror?: (e: unknown) => void;
    onback?: () => void;
    onforward?: () => void;
  } = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  const lang = new Compartment();
  const ind = new Compartment();
  /** A text replaced from outside is not the user typing. */
  let applying = false;
  let shownKey = untrack(() => docKey);
  let shownVersion = untrack(() => version);

  const indentExt = (i: { tabs: boolean; size: number }) => [
    indentUnit.of(i.tabs ? '\t' : ' '.repeat(i.size)),
    EditorState.tabSize.of(i.tabs ? 4 : i.size),
  ];

  function reportCursor(state: EditorState) {
    const head = state.selection.main.head;
    const line = state.doc.lineAt(head);
    oncursor({ line: line.number, col: charColumn(state.sliceDoc(line.from, head), head - line.from) });
  }

  function makeState(doc: string) {
    return EditorState.create({
      doc,
      extensions: [
        EditorState.allowMultipleSelections.of(true),
        // Ctrl+click (Cmd+click) follows a link: Alt+click (Option+click) adds a cursor, as in VS Code.
        EditorView.clickAddsSelectionRange.of((e) => e.altKey),
        gotoExtension({
          resolvers: untrack(() => nav?.resolvers ?? []),
          context: () => (nav ? { path: nav.path, files: nav.files } : null),
          onTargets: (t, from, spot) => ontargets?.(t, from, spot),
          onError: (e) => onnaverror?.(e),
          onBack: () => onback?.(),
          onForward: () => onforward?.(),
        }),
        changeGutter(),
        lineNumbers(),
        highlightActiveLineGutter(),
        history(),
        drawSelection(),
        highlightActiveLine(),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        search({ top: true }),
        EditorState.phrases.of(PHRASES),
        keymap.of([...closeBracketsKeymap, ...defaultKeymap, ...searchKeymap, ...historyKeymap, indentWithTab]),
        editorTheme,
        lang.of(untrack(() => language) ?? []),
        ind.of(indentExt(untrack(() => indent))),
        EditorView.updateListener.of((u) => {
          if (u.docChanged && !applying) onchange(u.state.doc.toString());
          if (u.selectionSet || u.docChanged) reportCursor(u.state);
        }),
      ],
    });
  }

  onMount(() => {
    view = new EditorView({ state: makeState(untrack(() => text)), parent: host });
    view.dispatch({ effects: setChanges.of(untrack(() => changes)) });
    reportCursor(view.state);
  });
  onDestroy(() => view?.destroy());

  $effect(() => {
    const k = docKey;
    const v = version;
    untrack(() => {
      if (!view) return;
      if (k !== shownKey) {
        shownKey = k;
        shownVersion = v;
        view.setState(makeState(text));
        view.dispatch({ effects: setChanges.of(changes) });
        reportCursor(view.state);
      } else if (v !== shownVersion) {
        shownVersion = v;
        // Only what differs is replaced, outside of the history: the selections follow the text they were on,
        // and undo never brings back what was there before the reload.
        const change = reloadChange(view.state.doc.toString(), view.state.toText(text).toString());
        if (change) {
          applying = true;
          try {
            view.dispatch({ changes: change, annotations: Transaction.addToHistory.of(false) });
          } finally {
            applying = false;
          }
        }
      }
    });
  });

  $effect(() => {
    const l = language;
    untrack(() => view?.dispatch({ effects: lang.reconfigure(l ?? []) }));
  });

  $effect(() => {
    const i = indent;
    untrack(() => view?.dispatch({ effects: ind.reconfigure(indentExt(i)) }));
  });

  $effect(() => {
    const c = changes;
    untrack(() => view?.dispatch({ effects: setChanges.of(c) }));
  });

  $effect(() => {
    const r = reveal;
    if (!r) return;
    untrack(() => {
      if (!view) return;
      const line = view.state.doc.line(Math.min(Math.max(1, r.line), view.state.doc.lines));
      const pos = line.from + columnOffset(line.text, r.col ?? 1);
      view.dispatch({ selection: { anchor: pos }, effects: EditorView.scrollIntoView(pos, { y: 'center' }) });
      view.focus();
      onrevealed?.(r.seq);
    });
  });
</script>

<div class="code" bind:this={host}></div>

<style>
  .code {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .code :global(.cm-editor) {
    height: 100%;
  }
  .code :global(.cm-editor.cm-focused) {
    outline: none;
  }
</style>
