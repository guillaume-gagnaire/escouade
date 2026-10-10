// The editor's look: the design's colors (tinted by the project, through the CSS variables).

import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';
import { t, type Key } from '../i18n';

const C = {
  kw: 'var(--accent)',
  str: 'oklch(0.8 0.1 145)',
  num: 'oklch(0.82 0.11 75)',
  com: 'var(--dim)',
  type: 'oklch(0.82 0.08 215)',
  fn: 'oklch(0.87 0.08 95)',
  prop: 'oklch(0.84 0.06 270)',
  pun: 'var(--muted)',
  txt: 'var(--text)',
};

const highlight = HighlightStyle.define([
  {
    tag: [tags.keyword, tags.modifier, tags.controlKeyword, tags.operatorKeyword, tags.definitionKeyword, tags.moduleKeyword],
    color: C.kw,
  },
  { tag: tags.heading, color: C.kw, fontWeight: '700' },
  { tag: [tags.string, tags.special(tags.string), tags.regexp, tags.inserted], color: C.str },
  { tag: [tags.number, tags.bool, tags.null, tags.atom], color: C.num },
  { tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment], color: C.com, fontStyle: 'italic' },
  { tag: [tags.typeName, tags.className, tags.namespace, tags.tagName], color: C.type },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.macroName], color: C.fn },
  { tag: [tags.propertyName, tags.attributeName, tags.labelName], color: C.prop },
  { tag: [tags.punctuation, tags.bracket, tags.operator, tags.separator], color: C.pun },
  { tag: tags.strong, fontWeight: '700' },
  { tag: tags.emphasis, fontStyle: 'italic' },
  { tag: tags.link, color: C.type, textDecoration: 'underline' },
  { tag: tags.invalid, color: 'var(--del)' },
]);

export const editorTheme = [
  EditorView.theme(
    {
      '&': { height: '100%', backgroundColor: 'var(--term)', color: C.txt, fontSize: '12.5px' },
      '.cm-scroller': { fontFamily: 'var(--mono)', lineHeight: '20px' },
      '.cm-content': { caretColor: 'var(--accent)', padding: '12px 0 60px' },
      '&.cm-focused .cm-cursor': { borderLeftColor: 'var(--accent)' },
      '.cm-gutters': { backgroundColor: 'var(--term)', color: 'var(--dim)', border: 'none' },
      '.cm-lineNumbers .cm-gutterElement': { padding: '0 14px 0 8px' },
      '.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--text)' },
      '.cm-activeLine': { backgroundColor: 'color-mix(in oklch, var(--accent) 9%, transparent)' },
      // As specific as the base theme's rules (it has one per focus), or its colors win over ours.
      '.cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, ::selection': {
        backgroundColor: 'color-mix(in oklch, var(--accent) 28%, transparent)',
      },
      '.cm-change-gutter': { width: '4px' },
      '.cm-change': { height: '100%', position: 'relative' },
      '.cm-change.changed::before': {
        content: '""',
        position: 'absolute',
        left: '0',
        top: '2px',
        bottom: '2px',
        width: '3px',
        borderRadius: '0 2px 2px 0',
        background: 'var(--add)',
      },
      '.cm-change.deleted::after': {
        content: '""',
        position: 'absolute',
        left: '0',
        top: '-3px',
        borderLeft: '5px solid var(--del)',
        borderTop: '3px solid transparent',
        borderBottom: '3px solid transparent',
      },
      // The changes shown in the text (« Voir les changements », « Comparer »), in the colors of the gutter's marks:
      // the lines of the other version above each block, then the block. As specific as the merge view's own rules,
      // or its colors win over ours.
      '&.cm-merge-b .cm-changedLine': { backgroundColor: 'color-mix(in oklch, var(--add) 8%, transparent)' },
      '&.cm-merge-b .cm-changedText': { background: 'color-mix(in oklch, var(--add) 24%, transparent)' },
      '.cm-deletedChunk': { backgroundColor: 'color-mix(in oklch, var(--del) 8%, transparent)' },
      '.cm-deletedChunk .cm-deletedText': { background: 'color-mix(in oklch, var(--del) 28%, transparent)' },
      '.cm-deletedChunk .cm-blockAction': {
        height: '20px',
        margin: '0',
        padding: '0 8px',
        border: '1px solid var(--line2)',
        borderRadius: 'var(--r-sm)',
        backgroundColor: 'var(--elev)',
        color: 'var(--text)',
        fontFamily: 'var(--ui)',
        fontSize: '11px',
        fontWeight: '600',
        lineHeight: '18px',
        cursor: 'pointer',
      },
      '.cm-deletedChunk .cm-blockAction:hover': { borderColor: 'var(--accent)' },
      '.cm-panels': { backgroundColor: 'var(--elev)', color: 'var(--text)' },
      '.cm-panels.cm-panels-top': { borderBottom: '1px solid var(--line)' },
      '.cm-searchMatch': { backgroundColor: 'color-mix(in oklch, var(--wait) 30%, transparent)' },
      '&.cm-focused .cm-matchingBracket': { backgroundColor: 'var(--elev2)', outline: '1px solid var(--line2)' },
    },
    { dark: true },
  ),
  syntaxHighlighting(highlight),
];

/** The texts CodeMirror asks for (search panel, merge view…), by the English phrase it asks with, and the key of their text. */
const PHRASE_KEYS = {
  Accept: 'editor.cm.accept',
  Reject: 'editor.cm.reject',
  'Revert this chunk': 'editor.compare.revertBlock',
  '$ unchanged lines': 'editor.cm.unchangedLines',
  Find: 'editor.cm.find',
  Replace: 'editor.cm.replace',
  next: 'editor.cm.next',
  previous: 'editor.cm.previous',
  all: 'editor.cm.all',
  'match case': 'editor.cm.matchCase',
  'by word': 'editor.cm.byWord',
  regexp: 'editor.cm.regexp',
  replace: 'editor.cm.replaceVerb',
  'replace all': 'editor.cm.replaceAll',
  close: 'editor.cm.close',
  'current match': 'editor.cm.currentMatch',
  'replaced $ matches': 'editor.cm.replacedMatches',
  'replaced match on line $': 'editor.cm.replacedOnLine',
  'on line': 'editor.cm.onLine',
  'Go to line': 'editor.cm.goToLine',
  go: 'editor.cm.go',
} as const satisfies Record<string, Key>;

/** CodeMirror's texts (search panel, merge view…) in the language of the interface, for `EditorState.phrases`. */
export function phrases(): Record<string, string> {
  return Object.fromEntries(Object.entries(PHRASE_KEYS).map(([phrase, key]) => [phrase, t(key)]));
}
