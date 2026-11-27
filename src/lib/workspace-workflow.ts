// The workflows in the « Workspace » (ESC-689): the step of an agent on its card, and the project's workflows the
// floating bar's « Workflows » lists, with the agents in each. What the Workflow tab of the side panel showed before it
// left, worked out from the same reading and the same tracks. Pure: the canvas draws what these give.

import type { Agent, Ticket, Workflow, WorkflowOutcome, WorkflowScan, WorkflowTrack } from './types';
import { labelIn, TICKETS, ticketsWorkflow, ticketTracks, tracksIn, withTickets, workflowTitle } from './workflow-view';

/** Where an agent is in its workflow, as its card says it. */
export interface CardStep {
  workflow: string;
  /** « GSD », the workflow's name (a repository's with it). */
  name: string;
  /** « Planifier », the step's label. */
  step: string;
  /** Its stay in the step: 1 the first time, 2 the second… */
  pass: number;
  /** How its stay goes (absent when nothing is known of it yet). */
  outcome: WorkflowOutcome | null;
  /** It was sent back here (or is being sent back): from which step, and why, as the step wrote it. */
  back: { from: string; reason: string | null } | null;
}

/**
 * The step of an agent on its card: its workflow's, never « Tickets » (the card has its loop), nothing when it is out
 * of any or out of the one it followed (« Hors du workflow »), or in a workflow the project no longer has.
 */
export function cardStepOf(all: Workflow[], tr: WorkflowTrack | undefined | null): CardStep | null {
  if (!tr || tr.outside || !tr.step || !tr.workflow || tr.workflow === TICKETS) return null;
  const w = all.find((x) => x.id === tr.workflow);
  if (!w) return null;
  const mine = tr.passages.filter((p) => p.workflow === w.id);
  const now = [...mine].reverse().find((p) => p.step === tr.step) ?? null;
  // The way back that brought it here: the passage just before its stay, or the stay itself when it is the one going back.
  const at = now ? mine.lastIndexOf(now) : -1;
  const before = at > 0 ? mine[at - 1] : null;
  const sent = now?.outcome === 'back' ? now : before?.outcome === 'back' ? before : null;
  return {
    workflow: w.id,
    name: workflowTitle(w),
    step: labelIn(w, tr.step),
    pass: now?.pass ?? 1,
    outcome: now?.outcome ?? null,
    back: sent ? { from: labelIn(w, sent.step), reason: sent.reason?.trim() || null } : null,
  };
}

/** The tracks of a project: those of its workflows whose agent is still there, then those of « Tickets ». */
export function projectTracks(
  projectId: string,
  read: WorkflowTrack[],
  tickets: Ticket[],
  agents: Record<string, Agent>,
  now: number,
): WorkflowTrack[] {
  return [...read.filter((tr) => agents[tr.agentId]), ...ticketTracks(projectId, tickets, agents, now)];
}

/** A workflow of the project as « Workflows » lists it: the agents in it, those at work first. */
export interface ListedWorkflow {
  workflow: Workflow;
  tracks: WorkflowTrack[];
}

/** The project's workflows as it has them (corrections in, those dismissed left out), then « Tickets » while it has tickets. */
export function projectWorkflows(scan: WorkflowScan | null, tickets: Ticket[], tracks: WorkflowTrack[]): ListedWorkflow[] {
  if (!scan) return [];
  return withTickets(scan.workflows, ticketsWorkflow(tickets)).map((w) => ({
    workflow: w,
    tracks: tracksIn(w, tracks).sort((a, b) => Number(!!a.outside) - Number(!!b.outside)),
  }));
}
