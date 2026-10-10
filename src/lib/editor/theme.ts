// The editor's look: the design's colors (tinted by the project, through the CSS variables).

import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags as t } from '@lezer/highlight';

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
  { tag: [t.keyword, t.modifier, t.controlKeyword, t.operatorKeyword, t.definitionKeyword, t.moduleKeyword], color: C.kw },
  { tag: t.heading, color: C.kw, fontWeight: '700' },
  { tag: [t.string, t.special(t.string), t.regexp, t.inserted], color: C.str },
  { tag: [t.number, t.bool, t.null, t.atom], color: C.num },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: C.com, fontStyle: 'italic' },
  { tag: [t.typeName, t.className, t.namespace, t.tagName], color: C.type },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: C.fn },
  { tag: [t.propertyName, t.attributeName, t.labelName], color: C.prop },
  { tag: [t.punctuation, t.bracket, t.operator, t.separator], color: C.pun },
  { tag: t.strong, fontWeight: '700' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.link, color: C.type, textDecoration: 'underline' },
  { tag: t.invalid, color: 'var(--del)' },
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

/** CodeMirror's texts (search panel, merge view…) in French. */
export const PHRASES: Record<string, string> = {
  Accept: 'Accepter',
  Reject: 'Rejeter',
  'Revert this chunk': 'Annuler ce bloc',
  '$ unchanged lines': '$ lignes inchangées',
  Find: 'Rechercher',
  Replace: 'Remplacer',
  next: 'suivant',
  previous: 'précédent',
  all: 'tout',
  'match case': 'respecter la casse',
  'by word': 'mot entier',
  regexp: 'expression régulière',
  replace: 'remplacer',
  'replace all': 'tout remplacer',
  close: 'fermer',
  'current match': 'occurrence courante',
  'replaced $ matches': '$ remplacements',
  'replaced match on line $': 'remplacé à la ligne $',
  'on line': 'à la ligne',
  'Go to line': 'Aller à la ligne',
  go: 'aller',
};
