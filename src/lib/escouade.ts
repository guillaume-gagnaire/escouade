// The ```escouade block an agent ends its turns with: its report of the criteria, what it has done
// so far, and how to launch its worktree for a test. Read as the backend reads it (`board::parse_report`),
// so that the card shows what the board keeps.

import type { RecipeProcess, RecipeStep, TestRecipe } from './types';

export interface Report {
  /** n from 1; null when the block has no list of criteria (a launch recipe or a progress alone). */
  criteria: { n: number; ok: boolean; note: string }[] | null;
  recipe: TestRecipe | null;
  /** The features in place, from `avancement` (or `progress`): up to 8 short lines; null when the block gave none. */
  progress: string[] | null;
}

export type Segment = { kind: 'md'; text: string } | { kind: 'report'; report: Report };

const BLOCK = /```escouade[^\n]*\n([\s\S]*?)```/g;

/** The most items of a progress list that are kept, and the most UTF-8 bytes of each. */
const PROGRESS_ITEMS = 8;
const PROGRESS_ITEM_BYTES = 120;

type Obj = Record<string, unknown>;

const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v);

/** What the first of the keys the object has holds, even null: the French key and its English alias. */
function pick(o: Obj, ...keys: string[]): unknown {
  const key = keys.find((k) => Object.hasOwn(o, k));
  return key === undefined ? undefined : o[key];
}

const utf8 = new TextEncoder();

/** Text on one line: control characters dropped, whitespace collapsed and trimmed, cut at `max` bytes with « … ». */
function oneLine(s: string, max: number): string {
  const line = s
    .replace(/(?!\p{White_Space})\p{Cc}/gu, '')
    .split(/\p{White_Space}+/u)
    .filter(Boolean)
    .join(' ');
  if (utf8.encode(line).length <= max) return line;
  let out = '';
  let bytes = 0;
  for (const ch of line) {
    bytes += utf8.encode(ch).length;
    if (bytes > max) break;
    out += ch;
  }
  return `${out}…`;
}

function progressOf(v: unknown): string[] | null {
  if (!Array.isArray(v)) return null;
  const items = v
    .filter((item): item is string => typeof item === 'string')
    .map((item) => oneLine(item, PROGRESS_ITEM_BYTES))
    .filter(Boolean)
    .slice(0, PROGRESS_ITEMS);
  return items.length ? items : null;
}

/** A criterion's number (from 1): a number, or a string of digits. */
function criterionNumber(n: unknown): number | null {
  const v = typeof n === 'string' ? (/^\+?\d+$/.test(n.trim()) ? Number(n.trim()) : NaN) : n;
  return typeof v === 'number' && Number.isSafeInteger(v) && v >= 1 ? v : null;
}

/** The criteria of a list: those with a valid `n`; null for what is not a list, or one of which nothing is usable. */
function criteriaOf(list: unknown): Report['criteria'] {
  if (!Array.isArray(list)) return null;
  const reported: NonNullable<Report['criteria']> = [];
  for (const c of list) {
    if (!isObj(c)) continue;
    const n = criterionNumber(c.n);
    if (n !== null) reported.push({ n, ok: c.ok === true, note: typeof c.note === 'string' ? c.note.trim() : '' });
  }
  // Nothing usable in a list that was not empty must not read as "no criterion reached".
  return reported.length || !list.length ? reported : null;
}

/** A text of the recipe: absent or null is empty; anything but text makes the recipe unreadable (null). */
const str = (v: unknown): string | null => (v == null ? '' : typeof v === 'string' ? v : null);

/** Items of a list of the recipe: absent or null is empty; null when it is not a list or an item is unreadable. */
function listOf<T>(v: unknown, read: (item: unknown) => T | null): T[] | null {
  if (v == null) return [];
  if (!Array.isArray(v)) return null;
  const parsed = v.map(read);
  return parsed.every((i) => i !== null) ? (parsed as T[]) : null;
}

function toStep(s: unknown): RecipeStep | null {
  if (!isObj(s)) return null;
  const command = str(pick(s, 'command', 'commande'));
  const dir = str(pick(s, 'dir', 'dossier'));
  return command === null || dir === null ? null : { command, dir };
}

function toProcess(p: unknown): RecipeProcess | null {
  if (!isObj(p)) return null;
  const name = str(pick(p, 'name', 'nom'));
  const command = str(pick(p, 'command', 'commande'));
  const dir = str(pick(p, 'dir', 'dossier'));
  const url = str(p.url);
  // Values are texts whatever the agent wrote (`"PORT": 4101`); a null one is skipped.
  if (p.env != null && !isObj(p.env)) return null;
  const env = Object.fromEntries(
    Object.entries(p.env ?? {})
      .filter(([, v]) => v !== null)
      .map(([k, v]) => [k, typeof v === 'string' ? v : JSON.stringify(v)]),
  );
  return name === null || command === null || dir === null || url === null ? null : { name, command, dir, env, url };
}

/** A folder of the worktree, written relative: no drive, no root, no `..`. */
function safeDir(dir: string): boolean {
  const d = dir.trim();
  return !(/^[\\/]/.test(d) || /^[\u0000-\u007f]:/.test(d) || d.split(/[\\/]/).includes('..'));
}

/** The launch recipe, or null when it is unreadable, leaves the worktree, has a step without a command or nothing to run. */
function toRecipe(l: unknown): TestRecipe | null {
  if (!isObj(l)) return null;
  const prepare = listOf(pick(l, 'prepare', 'preparation'), toStep);
  const processes = listOf(pick(l, 'processes', 'processus'), toProcess);
  const open = str(pick(l, 'open', 'ouvrir'));
  if (!prepare || !processes || open === null) return null;
  const steps = [...prepare, ...processes];
  return steps.length && steps.every((s) => safeDir(s.dir) && s.command.trim()) ? { prepare, processes, open } : null;
}

/** The content of a block, if it is a report: criteria (`criteres` or `criteria`), progress (`avancement` or `progress`) or a recipe. */
export function parseReport(json: string): Report | null {
  let v: unknown;
  try {
    v = JSON.parse(json);
  } catch {
    return null;
  }
  if (!isObj(v)) return null;
  const criteria = criteriaOf(pick(v, 'criteres', 'criteria'));
  const recipe = toRecipe(v.lancement);
  const progress = progressOf(pick(v, 'avancement', 'progress'));
  return criteria || recipe || progress ? { criteria, recipe, progress } : null;
}

/** A message cut around its readable blocks (an unreadable one stays a code block). */
export function splitEscouade(text: string): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  const block = new RegExp(BLOCK);
  for (let m = block.exec(text); m; m = block.exec(text)) {
    const report = parseReport(m[1]);
    if (!report) {
      // Unreadable: maybe a mention of the fence in the text, whose match ran up to the opening fence of the real
      // block. Read again from just after its own opening fence.
      block.lastIndex = m.index + 3;
      continue;
    }
    const before = text.slice(last, m.index);
    if (before.trim()) out.push({ kind: 'md', text: before });
    out.push({ kind: 'report', report });
    last = m.index + m[0].length;
  }
  const rest = text.slice(last);
  if (rest.trim() || !out.length) out.push({ kind: 'md', text: rest });
  return out;
}
