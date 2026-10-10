// The size of the editor's left column (Fichiers / Recherche), kept in the preferences for all projects.

import { readPref, writePref } from '../prefs';

export const TREE_MIN = 160;
export const TREE_DEFAULT = 240;

const PREF = 'editor.treeWidth';

/** The widest the column may be in an editor area `area` px wide: half of it, and never less than its minimum. */
export const treeMax = (area: number) => Math.max(TREE_MIN, Math.floor(area / 2));

/** The width kept, the usual one when none is (or the preference is not a width). Not held to the area: it varies. */
export function readTreeWidth(): number {
  const n = Number(readPref(PREF));
  return Number.isFinite(n) && n > 0 ? Math.round(n) : TREE_DEFAULT;
}

export const writeTreeWidth = (width: number) => writePref(PREF, String(Math.round(width)));
