// What tells apart the tabs of files that have the same name.

import { basename } from '../format';

const folders = (path: string) => path.split('/').slice(0, -1);
/** The `k` folders nearest the file. */
const nearest = (dirs: string[], k: number) => dirs.slice(-k).join('/');

/**
 * For each of `paths` (relative to the source, `/`-separated), the shortest run of its folders, nearest first, that
 * no other open file of the same name has too (`editor` for `src/components/editor/index.ts` next to
 * `src/components/board/index.ts`); '' for a file whose name is alone, and for one at the root.
 */
export function tabHints(paths: readonly string[]): Record<string, string> {
  const byName = new Map<string, string[]>();
  for (const p of paths) byName.set(basename(p), [...(byName.get(basename(p)) ?? []), p]);
  const hints: Record<string, string> = {};
  for (const p of paths) {
    const mine = folders(p);
    const twins = (byName.get(basename(p)) ?? []).filter((q) => q !== p).map(folders);
    // A file whose folders run out first keeps all of them: the others, having more, tell themselves apart.
    let k = 1;
    while (k < mine.length && twins.some((t) => nearest(t, k) === nearest(mine, k))) k++;
    hints[p] = twins.length ? nearest(mine, k) : '';
  }
  return hints;
}
