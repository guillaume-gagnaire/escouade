// The shared Kanban (ESC-190): a ticket shared with the team is a file `.escouade/tickets/<id>.yml` of the project's
// folder (`src-tauri/src/sharing.rs`). What the window draws of it: whose it is, the line of a card of another one of the
// team, the files panel's group, the user's name.

import { t } from './i18n';
import type { Project, Settings, Ticket } from './types';

/** Where the files are, in the project's folder. */
export const SHARED_DIR = '.escouade/tickets';

/** A shared ticket's file, from the project's folder. */
export function sharedFile(ticket: Ticket): string | null {
  return ticket.shared ? `${SHARED_DIR}/${ticket.shared.id}.yml` : null;
}

/** Shared by another one of the team: never started by the autopilot, « Lancer » alone does. */
export function isTheirs(ticket: Ticket): boolean {
  return !!ticket.shared && !ticket.shared.mine;
}

/** Shared by this machine's user. */
export function isMine(ticket: Ticket): boolean {
  return !!ticket.shared?.mine;
}

/** Its author's name, « quelqu’un » when the file names nobody. */
export function authorName(ticket: Ticket): string {
  return ticket.shared?.author.name || t('share.card.someone');
}

/** The hue of a person's disc (0 to 359), from their id: the same everywhere, whatever the name. */
export function hueOf(id: string): number {
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}

/** The letter of a person's disc. */
export function initialOf(name: string): string {
  return (name.trim()[0] ?? '?').toUpperCase();
}

/** This machine's user's name in a project: the project's own, else the app's; empty while none was given. */
export function myName(settings: Settings, project?: Project | null): string {
  return (project?.userName?.trim() || settings.userName?.trim() || '').trim();
}

/** Its agent here holds it: its column is its own, whatever its file says. */
export function heldHere(ticket: Ticket): boolean {
  return !!ticket.agentId && (ticket.column === 'doing' || ticket.column === 'review' || ticket.column === 'pr');
}

/**
 * The line a card of another one's ticket says, by its column, when no agent of this machine holds it: added by them
 * (to do), taken by someone, to test at someone's, done by someone; and of one of ours a teammate took. Null otherwise.
 */
export function theirsLine(ticket: Ticket): string | null {
  const s = ticket.shared;
  // Started here, it is this machine's to tell (its agent, its outcome).
  if (!s || s.problem || heldHere(ticket) || ticket.startedAt) return null;
  // One of ours tells who took it, once a teammate did.
  if (s.mine && (ticket.column === 'todo' || !s.takenBy)) return null;
  const by = s.takenBy?.name || authorName(ticket);
  switch (ticket.column) {
    case 'todo':
      return t('share.card.theirsTodo', { name: authorName(ticket) });
    case 'doing':
      return t('share.card.takenBy', { name: by });
    case 'review':
      return t('share.card.reviewAt', { name: by });
    case 'pr':
      return t('share.card.prAt', { name: by });
    case 'done':
      return t('share.card.doneBy', { name: by });
  }
}

/** Under way or to test on a teammate's machine, as its file says: never started here, it takes no place here. */
export function elsewhere(ticket: Ticket): boolean {
  return !!ticket.shared && !ticket.startedAt && (ticket.column === 'doing' || ticket.column === 'review');
}

/** Someone else took it too, says its file, while this machine's agent holds it (choice 124): their name. */
export function alsoOn(ticket: Ticket, myId: string | undefined): string | null {
  const taken = ticket.shared?.takenBy;
  if (!taken || !heldHere(ticket) || !myId || taken.id === myId) return null;
  return taken.name;
}

/** Whether a path of the files panel is a shared ticket's file (or one of its images and files). */
export function isTicketPath(path: string): boolean {
  const p = path.replace(/\\/g, '/');
  return p.startsWith(`${SHARED_DIR}/`) || p.includes(`/${SHARED_DIR}/`);
}

/** The ticket a path of the folder is the file of, among those of the project. */
export function ticketOfPath(path: string, tickets: Ticket[]): Ticket | undefined {
  const name = path.replace(/\\/g, '/').split('/').pop() ?? '';
  const id = name.endsWith('.yml') ? name.slice(0, -4) : '';
  return id ? tickets.find((x) => x.shared?.id === id) : undefined;
}

/** The tickets of a project the board shows: those « Masquer chez moi » took off stay out unless asked for. */
export function shownOnBoard(tickets: Ticket[], showHidden: boolean): Ticket[] {
  return showHidden ? tickets : tickets.filter((x) => !x.shared?.hidden);
}
