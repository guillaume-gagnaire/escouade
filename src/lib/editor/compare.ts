// The text compared with another version of the file, in the text itself (« Voir les changements », « Comparer »):
// above each block that differs, the lines the other version has there, and a button that puts them in the text.

import { unifiedMergeView } from '@codemirror/merge';
import { Compartment, EditorState, Facet, type Extension, type TransactionSpec } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { DIFF_TIMEOUT } from './changes';

/** What the text is compared with: the reference version (HEAD, or where a worktree's branch left its base), or the file on disk. */
export type CompareWith = 'reference' | 'disk';

export interface Comparison {
  /** The text of the other version. */
  original: string;
  against: CompareWith;
}

/** What the button of a block does, said for the version it takes the block from. */
const ACTION: Record<CompareWith, string> = { reference: 'Annuler ce bloc', disk: 'Prendre ce bloc' };

/** The comparison an editor shows, null when it shows none. */
const shown = Facet.define<Comparison, Comparison | null>({ combine: (values) => values[0] ?? null });

/** The buttons of a block: only the one taking the other version's block, as keeping the text as it is needs none. */
function blockButton(label: string) {
  return (type: 'accept' | 'reject', action: (e: MouseEvent) => void): HTMLElement => {
    if (type === 'accept') return document.createElement('span');
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'cm-blockAction';
    button.textContent = label;
    // Pressed with the mouse, it leaves the focus in the text, where Ctrl+Z undoes what it did.
    button.onmousedown = (e) => e.preventDefault();
    // A click, or Enter or Space once it has the focus (Escape then Tab from the text): the button goes with its
    // block, so the focus goes back to the text.
    button.onclick = (e) => {
      const host = button.closest<HTMLElement>('.cm-editor');
      const view = host && EditorView.findFromDOM(host);
      action(e);
      view?.focus();
    };
    return button;
  };
}

function compareView(c: Comparison): Extension {
  return [
    shown.of(c),
    unifiedMergeView({
      original: c.original,
      // The gutter has its own marks, of the lines changed since the reference version.
      gutter: false,
      mergeControls: blockButton(ACTION[c.against]),
      // The merge view's own limit on the changes scanned, and the gutter's on the time a big file's diff takes.
      diffConfig: { scanLimit: 500, timeout: DIFF_TIMEOUT },
    }),
  ];
}

/** The compartment `slot` holding the comparison `c` (nothing for null), for a new editor state. */
export const comparison = (slot: Compartment, c: Comparison | null): Extension => slot.of(c ? compareView(c) : []);

/**
 * The transaction bringing the comparison `c` (none for null) into an editor whose compartment `slot` holds it; null
 * when it is the one shown already, so that a value given again does not draw its blocks again.
 */
export function showComparison(state: EditorState, slot: Compartment, c: Comparison | null): TransactionSpec | null {
  const now = state.facet(shown);
  if (now === c || (now && c && now.against === c.against && now.original === c.original)) return null;
  return { effects: slot.reconfigure(c ? compareView(c) : []) };
}
