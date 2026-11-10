// The pull requests new to the user (ESC-403): open, come into the project's list since the PR tab was last opened, and
// not opened by Escouade (the user saw those born). Each reading of the list, whoever made it (the background reading of
// the project on screen, the view, the « PR et CI » tab, the overview, the tools), tells its numbers (`prsRead`), and
// this keeps what was seen, by project, in the interface's state (it survives a restart).
//
// A host numbers its pull requests in the order they are opened: what was seen is the highest number seen (`mark`), so
// an older one coming back up the list (only the 30 last updated are read) is never new.

/**
 * How often the window offers the backend to read the list of the project on screen, while visible (ESC-403): the
 * backend reads it only when no reading was made for two minutes, and waits longer after a failure.
 */
export const WATCH_TICK_MS = 30_000;

export interface PrSeen {
  /** The highest number seen: at the first reading, then at each opening of the tab, and while it is shown. */
  mark: number;
  /** The open ones of the last reading. */
  open: number[];
  /** Those above `mark` that Escouade opened: kept, so they stay its own once their agent or ticket is gone. */
  own: number[];
  /** Those new at the last opening of the tab, and those come while it was shown: marked in the list until the next. */
  fresh: number[];
}

const highest = (mark: number, numbers: number[]) => numbers.reduce((m, n) => Math.max(m, n), mark);

/** The new ones: open, above what was seen, not Escouade's. */
const pending = (s: PrSeen) => s.open.filter((n) => n > s.mark && !s.own.includes(n));

/**
 * A reading of the list: `numbers`, its open ones; `escouade`, those Escouade opened (its tickets' and its agents');
 * `shown`, the PR tab of the project is on screen (what comes in is seen at once, and marked). The first reading of a
 * project counts nothing.
 */
export function readSeen(s: PrSeen | undefined, numbers: number[], escouade: Set<number>, shown: boolean): PrSeen {
  if (!s) return { mark: highest(0, numbers), open: [...numbers], own: [], fresh: [] };
  const own = [...new Set([...s.own, ...numbers.filter((n) => n > s.mark && escouade.has(n))])];
  const next: PrSeen = { mark: s.mark, open: [...numbers], own, fresh: s.fresh };
  if (!shown) return next;
  return { mark: highest(s.mark, numbers), open: next.open, own: [], fresh: [...new Set([...s.fresh, ...pending(next)])] };
}

/** The PR tab opened: back to zero, the new ones marked in the list until the next opening. */
export function opened(s: PrSeen | undefined): PrSeen | undefined {
  if (!s) return undefined;
  return { mark: highest(s.mark, s.open), open: s.open, own: [], fresh: pending(s) };
}

/** How many the tab's badge shows. */
export const newCount = (s: PrSeen | undefined) => (s ? pending(s).length : 0);

/** Those the list marks « Nouvelle ». */
export const freshIn = (s: PrSeen | undefined) => new Set(s?.fresh ?? []);

const numbers = (v: unknown) => (Array.isArray(v) ? v.filter((n): n is number => Number.isInteger(n)) : []);

/** What was seen of a project, as the interface's state gave it back (anything else: nothing seen). */
export function seenOf(v: unknown): PrSeen | undefined {
  if (!v || typeof v !== 'object') return undefined;
  const o = v as Record<string, unknown>;
  if (!Number.isInteger(o.mark)) return undefined;
  return { mark: o.mark as number, open: numbers(o.open), own: numbers(o.own), fresh: numbers(o.fresh) };
}
