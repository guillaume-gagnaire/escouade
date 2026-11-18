// The Kanban of a team's project (TS-6, `src-tauri/src/teamboard.rs`): what the window says of a ticket of the team —
// who holds it and since when, whether their machine still answers, an interrupted claim, the conflicts this machine
// lost, « Sur ce poste seulement ».

import { invoke } from './transport';
import { fTime } from './format';
import { t } from './i18n';
import type { Column, Project, TeamConflict, TeamHint, Ticket } from './types';

/** A ticket of the preview of « Verser à l'équipe » (TS-7). */
export interface PourItem {
  /** The id of its file (`<id>.yml`), or of this machine's ticket. */
  id: string;
  key: string;
  title: string;
  column: Column;
  /** Imported from Jira, GitHub…: it keeps its key. */
  external: boolean;
  /** Already in the team's Kanban: not written again. */
  poured: boolean;
  problem: string | null;
}

/** The docs of `.escouade/` in the preview (TS-9's docs on the team server): `pending` they go with « Verser »;
 * `server` the team's are there already; `repo` the team keeps them in the repository; `private`; `none`. */
export interface PourDocs {
  state: 'pending' | 'server' | 'repo' | 'private' | 'none';
  pages: string[];
  decisions: string[];
  documents: string[];
  /** The memory of `.escouade/memory/` (TS-10): `pending` when it goes with « Verser »; its memories' names then. */
  memory?: 'pending' | 'server' | 'repo' | 'private' | 'none';
  memories?: string[];
}

export interface PourPreview {
  files: PourItem[];
  locals: PourItem[];
  folder: string;
  docs: PourDocs;
}

export interface Poured {
  imported: number;
  already: number;
  locals: number;
  /** The files frozen, that may be removed from the repository. */
  files: string[];
  folder: string;
  /** The docs' files of `.escouade/` imported to the server, left in the repository. */
  docs: string[];
}

export const teamBoardApi = {
  /** « Partager son Kanban avec l’équipe » (`teamProject` null) or « Rejoindre » a teammate's by its id. */
  link: (project: string, teamProject: string | null) => invoke<Project>('project_link_team_board', { project, teamProject }),
  /** « Reprendre » a ticket whose holder's lease ran out. */
  resume: (id: string) => invoke<Ticket>('ticket_team_resume', { id }),
  /** « Reprendre le mien » / « Garder le mien » (`keepMine`), « Garder le sien » / « Prendre le sien ». */
  settle: (id: string, field: string, keepMine: boolean) => invoke<Ticket>('ticket_team_settle', { id, field, keepMine }),
  assign: (id: string, me: boolean) => invoke<Ticket>('ticket_team_assign', { id, me }),
  local: (id: string, local: boolean) => invoke<Ticket>('ticket_team_local', { id, local }),
  /** What « Verser à l'équipe » would pour (TS-7). */
  pourPreview: (project: string) => invoke<PourPreview>('project_team_pour_preview', { project }),
  /** « Verser à l'équipe »: the tickets of the files (`files`), those of this machine named, and the docs (`docs`). */
  pour: (project: string, files: boolean, tickets: string[], docs: boolean) =>
    invoke<Poured>('project_team_pour', { project, files, tickets, docs }),
  /** « Ne plus proposer pour ce projet » on the band of the team a repository names. */
  dismissHint: (project: string) => invoke<Project>('project_team_hint_dismiss', { project }),
};

/** A ticket of a project by its key, or by a key it had before it was poured into its team's Kanban (TS-7). */
export function ticketByKey(tickets: Iterable<Ticket>, projectId: string, key: string): Ticket | undefined {
  const all = [...tickets].filter((x) => x.projectId === projectId);
  const k = key.toLowerCase();
  return all.find((x) => x.key.toLowerCase() === k) ?? all.find((x) => (x.formerKeys ?? []).some((f) => f.toLowerCase() === k));
}

/** The number of the docs' files that would go. */
export function docsCount(d: PourDocs): number {
  const docs = d.state === 'pending' ? d.pages.length + d.decisions.length + d.documents.length : 0;
  return docs + (d.memory === 'pending' ? (d.memories ?? []).length : 0);
}

/** Whether « Verser » has docs or a memory to take (its box shown). */
export function docsPending(d: PourDocs): boolean {
  return d.state === 'pending' || d.memory === 'pending';
}

/** The files to pour: those that can be read and are not in the team's Kanban yet. */
export function filesToPour(p: PourPreview): PourItem[] {
  return p.files.filter((f) => !f.poured && !f.problem);
}

/** The band of the team a repository names (TS-7), as this device shows it: `invite` (this device is not in that team,
 * on that server), `attach` (it is: « Rattacher à l'équipe »), or none (attached, dismissed, no key). */
export type HintBand = { kind: 'invite' | 'attach'; hint: TeamHint; teamId: string | null };

export function hintBand(project: Project | null | undefined, teams: { id: string; server: string; joined: boolean }[]): HintBand | null {
  const hint = project?.shared?.team;
  if (!project || !hint) return null;
  if (project.team === hint.team) return null;
  if (project.shared?.teamDismissed === hint.fingerprint) return null;
  // The same team on the same server: an id alone, from a repository, never makes it one's own.
  const mine = teams.find((x) => x.id === hint.team && x.server.toLowerCase() === hint.host.toLowerCase() && x.joined);
  return mine ? { kind: 'attach', hint, teamId: mine.id } : { kind: 'invite', hint, teamId: null };
}

/** Whether a project's Kanban is a team's. */
export function isTeamBoard(project: Project | null | undefined): boolean {
  return !!project?.team && !!project.teamProject;
}

/** A ticket of a team's Kanban that stays on this machine (« Ne pas partager »). */
export function localOnly(ticket: Ticket, project: Project | null | undefined): boolean {
  return isTeamBoard(project) && !!ticket.teamLocal;
}

export interface HolderLine {
  text: string;
  /** `here` (this machine), `held` (a teammate), `interrupted` (their lease ran out). */
  tone: 'here' | 'held' | 'interrupted';
  /** « Injoignable depuis 12 min », when their machine renews nothing. */
  unreachable: string | null;
  /** « Reprendre » is offered. */
  resumable: boolean;
}

/** What the card says of who holds a ticket of the team; null while nobody does. */
export function holderLine(ticket: Ticket): HolderLine | null {
  const h = ticket.team?.holder;
  if (!h) return null;
  const name = h.name || t('teamBoard.card.someone');
  const unreachable = h.unreachableMin ? t('teamBoard.card.unreachable', { count: h.unreachableMin }) : null;
  if (h.interrupted && !h.mine) {
    return { text: t('teamBoard.card.interrupted', { name }), tone: 'interrupted', unreachable, resumable: true };
  }
  const since = fTime(h.since);
  if (h.mine) return { text: t('teamBoard.card.heldHere', { since }), tone: 'here', unreachable: null, resumable: false };
  return { text: t('teamBoard.card.heldBy', { name, since }), tone: 'held', unreachable, resumable: false };
}

/** The field a conflict is of, said in a sentence. */
export function fieldName(field: string): string {
  const known = ['title', 'description', 'criteria', 'column', 'rank', 'key', 'maxLoops', 'progress', 'partial', 'iteration', 'assignee'];
  return known.includes(field) ? t(`teamBoard.card.field.${field}` as 'teamBoard.card.field.title') : field;
}

/** What a band of conflict says (« Bob a changé le titre en même temps »), and its two buttons. */
export function conflictLine(c: TeamConflict): { text: string; mine: string; theirs: string } {
  if (c.unmerged) {
    return {
      text: t('teamBoard.card.unmerged', { name: c.by }),
      mine: t('teamBoard.card.keepMine'),
      theirs: t('teamBoard.card.takeTheirs'),
    };
  }
  return {
    text: t('teamBoard.card.conflict', { name: c.by, field: fieldName(c.field) }),
    mine: t('teamBoard.card.keepMineAgain'),
    theirs: t('teamBoard.card.keepTheirs'),
  };
}

/** Whom the ticket is assigned to, as the card says it; null when to nobody. */
export function assigneeLine(ticket: Ticket): string | null {
  const x = ticket.team;
  if (!x?.assignee) return null;
  if (x.assignedMe) return t('teamBoard.card.assignedMe');
  return t('teamBoard.card.assigned', { name: x.assigneeName || t('teamBoard.card.someone') });
}

/** Under way, to test or in review on another machine of the team: its agent and its branch are there. */
export function teamElsewhere(ticket: Ticket): boolean {
  return !!ticket.team && !ticket.startedAt && (ticket.column === 'doing' || ticket.column === 'review' || ticket.column === 'pr');
}

/** Started on a teammate's machine (it holds it, or held it): this machine's « Lancer » is not offered. */
export function heldElsewhere(ticket: Ticket): boolean {
  const h = ticket.team?.holder;
  return !!h && !h.mine && !h.interrupted;
}
