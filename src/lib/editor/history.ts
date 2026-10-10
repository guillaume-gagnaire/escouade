// Where the editor was before each jump (a Ctrl+click, F12, a search result), by project and source: Alt+← goes
// back there and Alt+→ forward again, as a browser does with its pages.

import { movedPath } from './tree';

/** A place in a file of a source: `line` and `col` 1-based, the column in characters. */
export interface NavEntry {
  projectId: string;
  source: string;
  path: string;
  line: number;
  col: number;
}

/** Places kept each way: older ones are forgotten. */
const LIMIT = 50;

interface Stacks {
  back: NavEntry[];
  forward: NavEntry[];
}

const same = (a: NavEntry, b: NavEntry) => a.path === b.path && a.line === b.line && a.col === b.col;

class NavHistory {
  private all = new Map<string, Stacks>();

  private of(e: { projectId: string; source: string }): Stacks {
    const k = `${e.projectId}|${e.source}`;
    let s = this.all.get(k);
    if (!s) this.all.set(k, (s = { back: [], forward: [] }));
    return s;
  }

  /** Before a jump, where it leaves from. The way forward is forgotten, as a browser does after a new page. */
  push(entry: NavEntry) {
    const s = this.of(entry);
    const last = s.back.at(-1);
    if (!last || !same(last, entry)) keep(s.back, entry);
    s.forward = [];
  }

  /** The place before `current` whose file still `exists`, null when there is none; `current` becomes the way forward. */
  back(current: NavEntry, exists: (e: NavEntry) => boolean): NavEntry | null {
    const s = this.of(current);
    return move(s.back, s.forward, current, exists);
  }

  /** The place `back` left, whose file still `exists`, null when there is none; `current` goes back on the way back. */
  forward(current: NavEntry, exists: (e: NavEntry) => boolean): NavEntry | null {
    const s = this.of(current);
    return move(s.forward, s.back, current, exists);
  }

  /** `from` (a file, or a folder holding files) renamed `to` in a source: its places follow it, both ways. */
  rename(projectId: string, source: string, from: string, to: string) {
    const s = this.all.get(`${projectId}|${source}`);
    if (!s) return;
    for (const e of [...s.back, ...s.forward]) e.path = movedPath(e.path, from, to) ?? e.path;
  }

  reset() {
    this.all.clear();
  }
}

function keep(stack: NavEntry[], e: NavEntry) {
  stack.push({ ...e });
  if (stack.length > LIMIT) stack.splice(0, stack.length - LIMIT);
}

/**
 * Takes the latest place of `from` that `exists` and is not `current` (the others are dropped on the way: going back
 * always moves), and keeps `current` on `to`.
 */
function move(from: NavEntry[], to: NavEntry[], current: NavEntry, exists: (e: NavEntry) => boolean): NavEntry | null {
  for (let e = from.pop(); e; e = from.pop()) {
    if (!exists(e) || same(e, current)) continue;
    keep(to, current);
    return e;
  }
  return null;
}

export const navHistory = new NavHistory();
