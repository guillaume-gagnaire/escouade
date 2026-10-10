// The text compared with another version of the file, in the text itself (« Voir les changements », « Comparer »):
// above each block that differs, the lines the other version has there, and a button that puts them in the text.

import { Change, diff, unifiedMergeView } from '@codemirror/merge';
import { Compartment, EditorState, Facet, type Extension, type StateEffect, type TransactionSpec } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { t } from '../i18n';
import { DIFF_TIMEOUT, lineTokens } from './changes';

/** What the text is compared with: the reference version (HEAD, or where a worktree's branch left its base), or the file on disk. */
export type CompareWith = 'reference' | 'disk';

export interface Comparison {
  /** The text of the other version. */
  original: string;
  against: CompareWith;
}

/** What the button of a block says, for the version it takes the block from; read for each button drawn, in the language of the moment. */
const blockLabel = (against: CompareWith) => t(against === 'reference' ? 'editor.compare.revertBlock' : 'editor.compare.takeBlock');

/** The comparison an editor shows, null when it shows none. */
const shown = Facet.define<Comparison, Comparison | null>({ combine: (values) => values[0] ?? null });

/** The buttons of a block: only the one taking the other version's block, as keeping the text as it is needs none. */
function blockButton(against: CompareWith) {
  return (type: 'accept' | 'reject', action: (e: MouseEvent) => void): HTMLElement => {
    if (type === 'accept') return document.createElement('span');
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'cm-blockAction';
    button.textContent = blockLabel(against);
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

/** Each line with its line break: the pieces of a text, back to back. */
const linesOf = (text: string) => text.match(/[^\n]*\n|[^\n]+$/g) ?? [];

/** Where each piece starts in the text, and where the last one ends. */
function starts(pieces: readonly string[]): number[] {
  const at = [0];
  for (const p of pieces) at.push(at[at.length - 1] + p.length);
  return at;
}

/**
 * The merge view's own limit on the changes scanned, past which a diff gives a cruder answer at once, and the
 * gutter's limit on the time one takes. Each call to `diff` gets its own 300 ms: a stretch of lines between two
 * anchors (the same few lines over and over give none, so a stretch can be the whole file), then each run of
 * characters.
 */
const BOUNDED = { scanLimit: 500, timeout: DIFF_TIMEOUT };

/**
 * The diff of `a` and `b` within the limit, its changes moved to where the two strings start in a longer one. The
 * merge view's diff sometimes gives an empty change: its common-prefix check compares the start of one range with
 * the end of the other (`fromA == toB`, a slip), misses a prefix, and the shortcut that follows splits off nothing.
 * Left out, as the merge view would draw it as a block with nothing to put back.
 */
function boundedDiff(a: string, b: string, fromA: number, fromB: number, out: Change[]) {
  for (const c of diff(a, b, BOUNDED)) {
    if (c.fromA < c.toA || c.fromB < c.toB) out.push(new Change(c.fromA + fromA, c.toA + fromA, c.fromB + fromB, c.toB + fromB));
  }
}

/** Of `pairs`, in the order of their second item, the longest run whose first items increase too. */
function longestIncreasing(pairs: readonly [number, number][]): [number, number][] {
  // The last pair of the best run of each length found so far, and the pair before each one in its run.
  const tails: number[] = [];
  const before: number[] = new Array(pairs.length);
  for (let k = 0; k < pairs.length; k++) {
    let lo = 0;
    let hi = tails.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (pairs[tails[mid]][0] < pairs[k][0]) lo = mid + 1;
      else hi = mid;
    }
    before[k] = lo > 0 ? tails[lo - 1] : -1;
    tails[lo] = k;
  }
  const run: [number, number][] = [];
  for (let k = tails.length ? tails[tails.length - 1] : -1; k >= 0; k = before[k]) run.push(pairs[k]);
  return run.reverse();
}

/**
 * The runs of lines that differ between two texts, one token a line (see `lineTokens`). Anchored on the lines found
 * once in each text and in the same order in both, as patience diff does, each stretch between two anchors diffed
 * within the limit: a lockfile's lines are nearly all unique, so its changes, however many, are found one by one,
 * and a block rewritten, which has no anchor, is one run at once.
 */
function lineRuns(a: string, b: string): Change[] {
  // Where each line is, -1 for a line found more than once.
  const inA = new Map<string, number>();
  for (let i = 0; i < a.length; i++) inA.set(a[i], inA.has(a[i]) ? -1 : i);
  const inB = new Map<string, number>();
  for (let j = 0; j < b.length; j++) inB.set(b[j], inB.has(b[j]) ? -1 : j);
  const pairs: [number, number][] = [];
  for (let j = 0; j < b.length; j++) {
    const i = inA.get(b[j]) ?? -1;
    if (i >= 0 && inB.get(b[j]) === j) pairs.push([i, j]);
  }
  const runs: Change[] = [];
  let i0 = 0;
  let j0 = 0;
  for (const [i, j] of [...longestIncreasing(pairs), [a.length, b.length]]) {
    if (i > i0 || j > j0) boundedDiff(a.slice(i0, i), b.slice(j0, j), i0, j0, runs);
    i0 = i + 1;
    j0 = j + 1;
  }
  return runs;
}

const isHighSurrogate = (code: number) => code >= 0xd800 && code <= 0xdbff;
const isLowSurrogate = (code: number) => code >= 0xdc00 && code <= 0xdfff;

/** Whether `c` only adds text (`'b'`), only removes some (`'a'`), or does both (null). */
const side = (c: Change) => (c.fromA === c.toA ? 'b' : c.fromB === c.toB ? 'a' : null);

/**
 * The changes that only add (or only remove) text, each moved forward one character at a time while its first
 * character is the one just after it, as far as the text in common before the next change goes: where taking the
 * common start off two texts puts one. Diffed line by line, then run by run, a change can be left at the start of
 * the same characters instead (the line diff's heuristics are not minimal, and a run's diff knows nothing past the
 * run); begun at an empty line on both sides, a line break added or removed there is taken by the merge view for the
 * end of the line above and drawn as an empty block, with nothing to put back. Moved forward, a change never begins
 * with the character that follows it, so never there. A move stops short of the middle of a surrogate pair.
 *
 * Taken from the last change to the first, so that each one is held back by where the next one is once moved, not
 * where it was: held back short of that, a change can be left on the empty line the next one has just left. Moved up
 * against the next change, both adding (or both removing) text, the two are one change, which the merge view would
 * join anyway, and that one moves on past the text the next one could not pass; up against a change of the other
 * kind, the merge view joins them into a change that replaces text, which it draws from the right line.
 */
function slideForward(changes: readonly Change[], a: string, b: string): Change[] {
  // The changes done, the last first; the one done last bounds the move of the one before it.
  const done: Change[] = [];
  for (let n = changes.length - 1; n >= 0; n--) {
    let c = changes[n];
    const only = side(c);
    if (only) {
      for (;;) {
        const next = done.length ? done[done.length - 1] : null;
        const endA = next ? next.fromA : a.length;
        const endB = next ? next.fromB : b.length;
        // The text that has the change, and where the change is in it.
        const [text, from, to] = only === 'b' ? [b, c.fromB, c.toB] : [a, c.fromA, c.toA];
        let by = 0;
        while (c.toA + by < endA && c.toB + by < endB && text.charCodeAt(from + by) === text.charCodeAt(to + by)) by++;
        if (by && isLowSurrogate(text.charCodeAt(from + by)) && isHighSurrogate(text.charCodeAt(from + by - 1))) by--;
        if (by) c = new Change(c.fromA + by, c.toA + by, c.fromB + by, c.toB + by);
        if (!next || c.toA !== next.fromA || c.toB !== next.fromB || side(next) !== only) break;
        done.pop();
        c = new Change(c.fromA, next.toA, c.fromB, next.toB);
      }
    }
    done.push(c);
  }
  return done.reverse();
}

/**
 * The diff of the merge view: line by line, then character by character within each run of lines that differ. A
 * character diff of the whole text either gives up on a big file (one block from its first change to its last: a
 * lockfile's scattered changes) or, unbounded, takes its whole timeout at each key typed in a big block rewritten,
 * which the merge view diffs again each time. Line by line, both are quick; within a run, the limit keeps a big one
 * quick and only cruder.
 */
export function blockDiff(a: string, b: string): readonly Change[] {
  const linesA = linesOf(a);
  const linesB = linesOf(b);
  const tokens = lineTokens(linesA, linesB);
  const out: Change[] = [];
  if (tokens) {
    const atA = starts(linesA);
    const atB = starts(linesB);
    for (const run of lineRuns(tokens[0], tokens[1])) {
      const fromA = atA[run.fromA];
      const fromB = atB[run.fromB];
      boundedDiff(a.slice(fromA, atA[run.toA]), b.slice(fromB, atB[run.toB]), fromA, fromB, out);
    }
  } else {
    boundedDiff(a, b, 0, 0, out);
  }
  return slideForward(out, a, b);
}

function compareView(c: Comparison): Extension {
  return [
    shown.of(c),
    unifiedMergeView({
      original: c.original,
      // The gutter has its own marks, of the lines changed since the reference version.
      gutter: false,
      mergeControls: blockButton(c.against),
      diffConfig: { override: blockDiff },
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

/**
 * The effect drawing the comparison shown again (null when there is none), for its buttons to say their
 * labels in the language now in use: they are drawn once, with their block.
 */
export function relabelComparison(state: EditorState, slot: Compartment): StateEffect<unknown> | null {
  const now = state.facet(shown);
  return now ? slot.reconfigure(compareView(now)) : null;
}
