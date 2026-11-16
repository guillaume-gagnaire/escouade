// The Kanban of a team's project (TS-6, `src-tauri/src/teamboard.rs`): what the window says of a ticket of the team —
// who holds it and since when, whether their machine still answers, an interrupted claim, the conflicts this machine
// lost, « Sur ce poste seulement ».

import { invoke } from '@tauri-apps/api/core';
import { fTime } from './format';
import { t } from './i18n';
import type { Project, TeamConflict, Ticket } from './types';

export const teamBoardApi = {
  /** « Partager son Kanban avec l’équipe » (`teamProject` null) or « Rejoindre » a teammate's by its id. */
  link: (project: string, teamProject: string | null) => invoke<Project>('project_link_team_board', { project, teamProject }),
  /** « Reprendre » a ticket whose holder's lease ran out. */
  resume: (id: string) => invoke<Ticket>('ticket_team_resume', { id }),
  /** « Reprendre le mien » / « Garder le mien » (`keepMine`), « Garder le sien » / « Prendre le sien ». */
  settle: (id: string, field: string, keepMine: boolean) => invoke<Ticket>('ticket_team_settle', { id, field, keepMine }),
  assign: (id: string, me: boolean) => invoke<Ticket>('ticket_team_assign', { id, me }),
  local: (id: string, local: boolean) => invoke<Ticket>('ticket_team_local', { id, local }),
};

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
