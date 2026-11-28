/**
 * The tab « Kanban » of the phone (MO-7, docs/MOBILE.md § 3.7, M8): one project at a time, with its space; its column
 * chosen in segments; its tickets, and their simple gestures — « Valider » a ticket « À tester » and « Lancer » one
 * « À faire » under the system's lock (a merge follows, an agent starts), « Renvoyer à l'agent » with a word of what to
 * change, « L'agent résout » a conflict. Neither writing a ticket nor moving one by hand: that is the computer's work.
 *
 * It reads each space's projects and the chosen project's board (`board_glance`, served to a phone: neither the
 * tickets' text nor their conversations) through the space's own connection, every few seconds while the tab is shown.
 * The project chosen is kept on the phone, an id and a space (no content).
 */

import { t } from '$shared/lib/i18n';
import type { GlanceProject } from '$shared/lib/spaces.svelte';
import type { Column } from '$shared/lib/types';
import { confirmByLock } from './lock';
import { callIn, phone, stateOf } from './phone-spaces.svelte';

/** How often the board shown is read again, and the spaces' projects. */
export const BOARD_EVERY = 3000;
export const PROJECTS_EVERY = 15_000;

/** A ticket, as a line says it (`glance::BoardTicket`). */
export interface BoardTicket {
  id: string;
  key: string;
  title: string;
  column: Column;
  agent: string | null;
  iteration: number;
  maxLoops: number;
  /** Its criteria, met or not: the squares of its line. */
  criteria: boolean[];
  partial: boolean;
  blocked: string | null;
  conflict: boolean;
  /** The validation step running (« Merge… »). */
  step: string | null;
}

export type BoardCounts = Record<Column, number>;

/** A project's board, as `board_glance` gives it. */
export interface BoardGlance {
  projects: GlanceProject[];
  projectId: string | null;
  counts: BoardCounts;
  tickets: BoardTicket[];
}

/** The project shown, and its space. */
export interface Choice {
  space: string;
  project: string;
}

/** What a line offers. */
export type Gesture = 'approve' | 'reject' | 'start' | 'resolve';

export const kanban = $state<{
  /** The project shown; none before a space said its projects. */
  choice: Choice | null;
  /** Each space's projects, as it said them last. */
  lists: Record<string, GlanceProject[]>;
  /** The board of the project shown, as its space said it last. */
  board: BoardGlance | null;
  /** The column chosen; none: the first that matters (`firstColumn`). */
  column: Column | null;
  /** The line unfolded (its gestures as buttons), or swiped open. */
  open: string | null;
  /** The line whose « Renvoyer » asks what to change. */
  rejecting: string | null;
  /** The lines whose gesture is under way: each its own (« Valider » answers once its merge and its cleaning are done, and
   * another line's gesture goes meanwhile). */
  busy: string[];
  /** What the last gesture failed on. */
  error: string | null;
}>({ choice: null, lists: {}, board: null, column: null, open: null, rejecting: null, busy: [], error: null });

// ---------- what the screen reads (pure) ----------

/** The segments: « En revue » only when the project holds one. */
export function segmentsOf(counts: BoardCounts): Column[] {
  return counts.pr > 0 ? ['todo', 'doing', 'review', 'pr', 'done'] : ['todo', 'doing', 'review', 'done'];
}

/** The column shown at first: what waits for the user (« À tester »), else what is under way, else what is to do. */
export function firstColumn(counts: BoardCounts): Column {
  if (counts.review > 0) return 'review';
  if (counts.doing > 0) return 'doing';
  return 'todo';
}

/** The column shown: the one chosen when it is still a segment, else the first that matters. */
export function shownColumn(counts: BoardCounts, chosen: Column | null): Column {
  return chosen && segmentsOf(counts).includes(chosen) ? chosen : firstColumn(counts);
}

/** A column's tickets, in the board's order (the core gives them so). */
export const ticketsIn = (board: BoardGlance, column: Column): BoardTicket[] => board.tickets.filter((x) => x.column === column);

/** What a line offers: nothing while its validation runs. */
export function gesturesOf(ticket: BoardTicket): Gesture[] {
  if (ticket.step) return [];
  if (ticket.conflict) return ['resolve'];
  if (ticket.column === 'review') return ['reject', 'approve'];
  if (ticket.column === 'todo') return ['start'];
  return [];
}

/** The criteria met, for the squares' label. */
export const metOf = (ticket: BoardTicket): number => ticket.criteria.filter(Boolean).length;

/** The project to show: the one kept when its space still has it, else the first project of the first space. */
export function chooseFrom(lists: Record<string, GlanceProject[]>, order: string[], kept: Choice | null): Choice | null {
  if (kept && lists[kept.space]?.some((p) => p.id === kept.project)) return kept;
  for (const space of order) {
    const first = lists[space]?.[0];
    if (first) return { space, project: first.id };
  }
  return null;
}

// ---------- the choice kept ----------

const KEY = 'escouade.mobile.kanban';

function readKept(): Choice | null {
  try {
    const raw = localStorage.getItem(KEY);
    const v = raw ? (JSON.parse(raw) as Partial<Choice>) : null;
    return v && typeof v.space === 'string' && typeof v.project === 'string' ? { space: v.space, project: v.project } : null;
  } catch {
    return null;
  }
}

function keep(choice: Choice): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(choice));
  } catch {
    // A private window: the choice lasts as long as the page.
  }
}

// ---------- reading ----------

const nameOf = (space: string): string => phone.spaces.find((s) => s.id === space)?.name ?? space;

/** Each space that is open asked its projects; the project shown chosen when none is yet. */
export async function readProjects(): Promise<void> {
  await Promise.all(
    phone.spaces
      .filter((s) => stateOf(s) === 'open')
      .map(async (s) => {
        try {
          const g = await callIn<BoardGlance>(s.id, 'board_glance', {});
          kanban.lists[s.id] = g.projects;
        } catch {
          // Cut meanwhile: its chip and its band say so; its projects stay as they were.
        }
      }),
  );
  const known = new Set(phone.spaces.map((s) => s.id));
  for (const id of Object.keys(kanban.lists)) if (!known.has(id)) delete kanban.lists[id];
  const next = chooseFrom(
    kanban.lists,
    phone.spaces.map((s) => s.id),
    kanban.choice ?? readKept(),
  );
  if (next && (next.space !== kanban.choice?.space || next.project !== kanban.choice?.project)) {
    kanban.choice = next;
    kanban.board = null;
    kanban.column = null;
    await readBoard();
  } else if (!next) {
    kanban.choice = null;
    kanban.board = null;
  }
}

/** The board of the project shown, read again. */
export async function readBoard(): Promise<void> {
  const choice = kanban.choice;
  if (!choice) return;
  try {
    const g = await callIn<BoardGlance>(choice.space, 'board_glance', { projectId: choice.project });
    // Another project chosen meanwhile: this answer is not its own.
    if (kanban.choice !== choice) return;
    kanban.lists[choice.space] = g.projects;
    if (g.projectId !== choice.project) {
      // The project is gone from its space: the first of the space, as it answered.
      kanban.choice = g.projectId ? { space: choice.space, project: g.projectId } : null;
      kanban.column = null;
    }
    kanban.board = g.projectId ? g : null;
  } catch {
    // Cut: the board stays as it was said last, the band says the space is cut.
  }
}

/** Another project shown (the sheet of the projects). */
export function choose(choice: Choice): void {
  kanban.choice = choice;
  kanban.board = null;
  kanban.column = null;
  kanban.open = null;
  kanban.rejecting = null;
  kanban.error = null;
  keep(choice);
  void readBoard();
}

/** Another column shown. */
export function showColumn(column: Column): void {
  kanban.column = column;
  kanban.open = null;
  kanban.rejecting = null;
}

/** The tab shown: its projects and its board read now, then every few seconds; the returned function stops it. */
export function watchBoard(every = BOARD_EVERY, projectsEvery = PROJECTS_EVERY): () => void {
  void readProjects();
  // Before a project is shown (the spaces still connecting), the projects are asked at each pass.
  const board = setInterval(() => void (kanban.choice ? readBoard() : readProjects()), every);
  const projects = setInterval(() => void readProjects(), projectsEvery);
  return () => {
    clearInterval(board);
    clearInterval(projects);
  };
}

// ---------- the gestures ----------

/** A gesture's command, the lock asked first when `locked` says what it does. */
async function act(ticket: BoardTicket, cmd: string, args: Record<string, unknown>, locked: string | null): Promise<boolean> {
  const choice = kanban.choice;
  if (!choice || kanban.busy.includes(ticket.id)) return false;
  kanban.error = null;
  if (locked && !(await confirmByLock(t('mobile.lock.reason', { what: locked, space: nameOf(choice.space) })))) return false;
  kanban.busy.push(ticket.id);
  try {
    await callIn(choice.space, cmd, { id: ticket.id, ...args });
    // Another line opened meanwhile stays open.
    if (kanban.open === ticket.id) kanban.open = null;
    if (kanban.rejecting === ticket.id) kanban.rejecting = null;
    return true;
  } catch (e) {
    kanban.error = t('mobile.kanban.failed', { key: ticket.key, reason: String(e) });
    return false;
  } finally {
    kanban.busy = kanban.busy.filter((id) => id !== ticket.id);
    await readBoard();
  }
}

/** « Valider »: under the lock, a merge follows. */
export const approve = (ticket: BoardTicket): Promise<boolean> =>
  act(ticket, 'ticket_approve', {}, t('mobile.kanban.lockApprove', { key: ticket.key }));

/** « Renvoyer à l'agent » with what to change: no lock (as refusing). */
export function reject(ticket: BoardTicket, comment: string): Promise<boolean> {
  const said = comment.trim();
  if (!said) return Promise.resolve(false);
  return act(ticket, 'ticket_reject', { comment: said }, null);
}

/** « Lancer »: under the lock, an agent starts. */
export const start = (ticket: BoardTicket): Promise<boolean> =>
  act(ticket, 'ticket_start', {}, t('mobile.kanban.lockStart', { key: ticket.key }));

/** « L'agent résout » a conflict: under the lock, its agent takes it up. */
export const resolve = (ticket: BoardTicket): Promise<boolean> =>
  act(ticket, 'ticket_resolve_conflict', {}, t('mobile.kanban.lockResolve', { key: ticket.key }));
