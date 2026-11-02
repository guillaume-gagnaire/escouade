// What the Workflow tab shows (ESC-194, choices 141 to 154), worked out from the reading of a project's workflows
// (ESC-192) and the tracks of its agents (ESC-193): the rail of steps and its ways back, the doors, where each agent is,
// one agent's history. Pure: the tab draws what these give.

import { t } from './i18n';
import type {
  Agent,
  Column,
  Ticket,
  Workflow,
  WorkflowPassage,
  WorkflowScan,
  WorkflowStep,
  WorkflowTrack,
  WorkflowTransition,
} from './types';

/** A step's name as the rail writes it: `gsd:discuss-phase` → « Discuss phase » (the corrections rename it). */
export function stepLabel(name: string): string {
  const base = name.slice(Math.max(name.lastIndexOf(':'), name.lastIndexOf('/')) + 1).replace(/\.md$/i, '');
  const words = base.replace(/[-_]+/g, ' ').trim();
  return words ? words[0].toLocaleUpperCase() + words.slice(1) : name;
}

/** The label of a step of `w` by its name: the user's own name as is, a step as read by its command (a step gone keeps its name). */
export function labelIn(w: Workflow | null, name: string | null): string {
  if (!name) return '';
  const loop = w?.promptLoop?.steps.find((s) => String(s.n) === name || s.text === name);
  if (loop) return loop.text;
  // Named by the user (Réglages › Projets › Workflow): as they wrote it.
  return w?.steps.find((s) => s.name === name)?.named ? name : stepLabel(name);
}

/** The workflow's name in the menu: a repository's own carries the repository's (choice 155). */
export const workflowTitle = (w: Workflow) => (w.repo ? t('workflow.inRepo', { repo: w.repo, name: w.name }) : w.name);

/**
 * The workflow the tab shows: the one chosen in its menu, else the open agent's, else the one most agents follow, else
 * the first read.
 */
export function pickWorkflow(scan: WorkflowScan | null, tracks: WorkflowTrack[], openAgent: string | null, chosen: string | null) {
  const all = scan?.workflows ?? [];
  if (!all.length) return null;
  const byId = (id: string | null | undefined) => (id ? all.find((w) => w.id === id) : undefined);
  // The open agent's own workflow before « Tickets », which it may follow too (choice 159).
  const ofOpen = tracks
    .filter((tr) => tr.agentId === openAgent)
    .sort((a, b) => Number(a.workflow === TICKETS) - Number(b.workflow === TICKETS));
  const mine = ofOpen.map((tr) => byId(tr.workflow)).find(Boolean);
  if (byId(chosen)) return byId(chosen)!;
  if (mine) return mine;
  // « Tickets » by the count of its agents never hides the project's own workflow.
  const read = all.filter((w) => w.id !== TICKETS);
  const count = (w: Workflow) => tracks.filter((tr) => tr.workflow === w.id).length;
  return [...(read.length ? read : all)].sort((a, b) => count(b) - count(a))[0];
}

/** « phase 3 sur 6 », as a state file of the workflow says it. */
export function phaseOf(w: Workflow): { done: number; total: number; file: string } | null {
  for (const f of w.stateFiles) if (f.says?.phase) return { ...f.says.phase, file: f.path };
  return null;
}

/** The steps in their order on the rail. */
export const stepsOf = (w: Workflow) => [...w.steps].sort((a, b) => a.order - b.order);

/** The doors in the head of the tab: those that are no step of the rail, the most launched first, the entries before the rest. */
export function doorsOf(w: Workflow, tracks: WorkflowTrack[]) {
  const steps = new Set(w.steps.map((s) => s.command));
  const launched = (cmd: string) => tracks.reduce((n, tr) => n + tr.passages.filter((p) => p.door?.split(/\s/)[0] === cmd).length, 0);
  const rank = { entry: 0, step: 1, other: 2 } as const;
  return w.doors
    .filter((d) => !steps.has(d.command) && d.command)
    .map((d) => ({ door: d, n: launched(d.command) }))
    .sort((a, b) => b.n - a.n || rank[a.door.role] - rank[b.door.role] || a.door.command.localeCompare(b.door.command))
    .map((x) => x.door);
}

/** A file by its name alone: `.planning/phases/{phase}/PLAN.md` → `PLAN.md`. */
export const fileName = (p: string) => p.split(/[\\/]/).pop() || p;

/** A line the workflow wrote (its exit), without the backticks of its Markdown. */
export const plainLine = (line: string) => line.replace(/`([^`]*)`/g, '$1');

/** How many doors the head shows before « +n ». */
export const DOORS_SHOWN = 3;

/** What a step of the rail says under its name: its subagents and their rounds, the files it writes, its ways back. */
export function stepDetail(w: Workflow, s: WorkflowStep) {
  const self = w.transitions.find((tr) => tr.back && tr.from === s.name && tr.to === s.name);
  const back = w.transitions
    .filter((tr) => tr.back && tr.from === s.name && tr.to !== s.name)
    .map((tr): { to: string; condition: string | null; say?: string } => ({
      to: tr.to,
      condition: tr.condition,
      ...(tr.say ? { say: tr.say } : {}),
    }));
  return {
    agents: s.agents.map((a) => a.name),
    rounds: self?.max ?? null,
    writes: [...new Set(s.writes.map(fileName))].slice(0, 2),
    back,
  };
}

/**
 * The ways back drawn in the gutter, by the index of their steps on the rail: from a later step to an earlier one
 * (a step back to itself is said in its detail, « jusqu'à 3 tours »).
 */
export function backArcs(w: Workflow, names: string[]): [number, number][] {
  const seen = new Set<string>();
  const out: [number, number][] = [];
  for (const tr of w.transitions) {
    const from = names.indexOf(tr.from);
    const to = names.indexOf(tr.to);
    if (!tr.back || from < 0 || to < 0 || to >= from || seen.has(`${from},${to}`)) continue;
    seen.add(`${from},${to}`);
    out.push([from, to]);
  }
  return out;
}

/** The tracks of the agents that follow `w`, in the workflow now or out of it for a while. */
export const tracksIn = (w: Workflow, tracks: WorkflowTrack[]) => tracks.filter((tr) => tr.workflow === w.id);

/** The passages of one workflow, the current phase only: since the last time the agent came in by the first step. */
export function currentPhase(w: Workflow, passages: WorkflowPassage[]): WorkflowPassage[] {
  const mine = passages.filter((p) => p.workflow === w.id);
  const first = stepsOf(w)[0]?.name;
  let start = 0;
  for (let i = mine.length - 1; i > 0; i--)
    if (mine[i].step === first && mine[i - 1].step !== first) {
      start = i;
      break;
    }
  return mine.slice(start);
}

/**
 * The steps an agent has done in its pass (ESC-244), in their order: those its history went through since it came in by
 * the first step, a way back undoing the steps from the one it goes back to; never the one it is on. Without a history
 * (« Tickets », a track just read), the steps before its own.
 */
export function doneSteps(w: Workflow, tr: WorkflowTrack): string[] {
  if (tr.workflow !== w.id) return [];
  const steps = stepsOf(w);
  const idx = (name: string | null) => steps.findIndex((s) => s.name === name);
  const phase = currentPhase(w, tr.passages);
  if (!phase.some((p) => p.step)) return steps.slice(0, Math.max(idx(tr.step), 0)).map((s) => s.name);
  const done = new Set<string>();
  for (const p of phase) {
    const i = idx(p.step);
    if (i < 0) continue;
    for (const s of steps.slice(i)) done.delete(s.name);
    if (p.outcome === 'done') done.add(p.step!);
  }
  if (!tr.outside && tr.step) done.delete(tr.step);
  return steps.filter((s) => done.has(s.name)).map((s) => s.name);
}

/** What comes after an agent's step: the next step (or the exit, at the end of the cycle), and its ways back, if any. */
export interface NextStep {
  to: string | null;
  exit: string | null;
  backs: { to: string; condition: string | null; say?: string }[];
}

/**
 * The next of an agent in `w` (ESC-244): the ordinary way on from its step, else the step after it, else the exit; and
 * the ways back from its step, the other end of a choice. Between two steps, from the last one (the step it was sent
 * back to, after a way back). Null outside the workflow, in a loop of one prompt (its turns say it), at the very end.
 */
export function nextOf(w: Workflow, tr: WorkflowTrack): NextStep | null {
  if (tr.workflow !== w.id || tr.outside || w.kind === 'prompt') return null;
  const steps = stepsOf(w);
  let from = tr.step;
  if (!from) {
    const last = [...currentPhase(w, tr.passages)].reverse().find((p) => p.step);
    if (!last) return null;
    if (last.outcome === 'back' && last.backTo) return { to: last.backTo, exit: null, backs: [] };
    from = last.step;
  }
  const i = steps.findIndex((s) => s.name === from);
  if (i < 0) return null;
  const to = w.transitions.find((x) => !x.back && x.from === from && x.to !== from)?.to ?? steps[i + 1]?.name ?? null;
  const backs = stepDetail(w, steps[i]).back;
  const exit = to ? null : w.exit;
  return to || exit || backs.length ? { to, exit, backs } : null;
}

/** « Ensuite : Livrer, ou retour à Planifier si écarts »: a condition said in the line when it is a few words, else in the tooltip. */
export function nextSays(w: Workflow, n: NextStep | null): string {
  if (!n) return '';
  const short = (c: string | null) => !!c && c.length <= 24 && !/[\n`]/.test(c);
  const back = (b: NextStep['backs'][number]) =>
    short(b.condition)
      ? t('workflow.backIf', { step: labelIn(w, b.to), condition: b.condition! })
      : t('workflow.backTo', { step: labelIn(w, b.to) });
  const first = n.to ? labelIn(w, n.to) : n.exit ? t('workflow.theExit') : null;
  const all = [...(first ? [first] : []), ...n.backs.map(back)];
  return t('workflow.next', {
    what:
      all[0] +
      all
        .slice(1)
        .map((x) => t('workflow.orElse', { what: x }))
        .join(''),
  });
}

/** The barrettes of an agent's row: one per step, done in its pass, on for its own. */
export function pipsOf(w: Workflow, tr: WorkflowTrack): ('done' | 'on' | '')[] {
  const done = new Set(doneSteps(w, tr));
  return stepsOf(w).map((s) => (s.name === tr.step && !tr.outside ? 'on' : done.has(s.name) ? 'done' : ''));
}

/** How long a passage took, or has taken so far. */
export const passageMs = (p: WorkflowPassage, now: number) => (p.outcome === 'running' ? Math.max(p.ms, now - p.at) : p.ms);

/** The strip in time of an agent's history: a shade of ink per step, red for a failure or a way back, green for now. */
export function stripOf(w: Workflow, passages: WorkflowPassage[], now: number) {
  const names = stepsOf(w).map((s) => s.name);
  return passages.map((p) => {
    const i = p.step ? names.indexOf(p.step) : -1;
    const cls =
      p.outcome === 'running'
        ? 'on'
        : p.outcome === 'back' || p.outcome === 'failed'
          ? 'bad'
          : p.step === null
            ? 'out'
            : `s${(Math.max(i, 0) % 4) + 1}`;
    return { cls, flex: Math.max(passageMs(p, now) / 60_000, 0.4), step: p.step };
  });
}

/** A piece of a line of the history: in mono (a name of the workflow's own) or not. */
export interface Bit {
  text: string;
  mono?: boolean;
}

/** What was done in a passage, said short: « 2 gsd-executor », « skill brainstorming », « STATE.md écrit ». */
export function didOf(p: WorkflowPassage): Bit[] {
  const agents = new Map<string, number>();
  const out: Bit[] = [];
  if (p.door) out.push({ text: p.door, mono: true });
  for (const d of p.did) {
    const [kind, ...rest] = d.split(':');
    const what = rest.join(':');
    if (kind === 'agent') agents.set(what, (agents.get(what) ?? 0) + 1);
  }
  for (const [name, n] of agents) out.push({ text: n > 1 ? `${n} ${name}` : name, mono: true });
  for (const d of p.did) {
    const [kind, ...rest] = d.split(':');
    const what = rest.join(':');
    if (kind === 'skill') out.push({ text: what, mono: true });
    if (kind === 'file') out.push({ text: t('workflow.written', { file: what.split('/').pop() ?? what }) });
  }
  return out;
}

/** One line of an agent's history. */
export interface HistoryRow {
  key: string;
  at: number;
  cls: '' | 'on' | 'bad' | 'back' | 'out';
  /** The line of a way back: the restart icon in place of the dot. */
  backLine: boolean;
  what: string;
  sub: Bit[];
  ms: number | null;
  cost: number | null;
}

/** The lines of an agent's history in `w`, oldest first: a line per passage, and one more for each way back. */
export function historyRows(w: Workflow, passages: WorkflowPassage[], now: number, costKnown = true): HistoryRow[] {
  const rows: HistoryRow[] = [];
  for (const p of passages) {
    const label = p.step ? labelIn(w, p.step) : t('workflow.outside');
    const num = { ms: passageMs(p, now), cost: costKnown ? p.cost : null };
    if (p.step === null) {
      rows.push({ key: `${p.n}`, at: p.at, cls: 'out', backLine: false, what: label, sub: p.doing ? [{ text: p.doing }] : [], ...num });
      continue;
    }
    const cls = p.outcome === 'running' ? 'on' : p.outcome === 'back' || p.outcome === 'failed' ? 'bad' : '';
    const what =
      p.outcome === 'failed'
        ? t('workflow.failed', { step: label })
        : p.outcome === 'stopped'
          ? t('workflow.stopped', { step: label })
          : label;
    const sub = p.outcome === 'back' && p.reason ? [{ text: p.reason }] : didOf(p);
    rows.push({ key: `${p.n}`, at: p.at, cls, backLine: false, what, sub, ...num });
    if (p.outcome === 'back' && p.backTo) {
      const by: Bit[] = p.decidedBy ? [{ text: p.decidedBy.split('/').pop() ?? p.decidedBy, mono: true }] : [];
      rows.push({
        key: `${p.n}-back`,
        at: p.ended ?? p.at,
        cls: 'back',
        backLine: true,
        what: t('workflow.backLine', { step: labelIn(w, p.backTo) }),
        sub: by.length ? [{ text: t('workflow.decidedBy', { file: by[0].text }), mono: false }] : [],
        ms: null,
        cost: null,
      });
    }
  }
  return rows;
}

/** The turns of a prompt loop, for its grid: done (ticked or not), now, and to come up to its maximum. */
export function turnsGrid(tr: WorkflowTrack | null, max: number | null): { n: number; cls: 'ok' | 'bad' | 'on' | 'todo'; next: boolean }[] {
  const done = (tr?.turns ?? []).map((u) => ({
    n: u.n,
    cls: u.outcome === 'running' ? ('on' as const) : u.ticked === false || u.outcome === 'failed' ? ('bad' as const) : ('ok' as const),
    next: false,
  }));
  const last = done.length ? done[done.length - 1].n : 0;
  // The first turn to come is the next (ESC-244), as « Ensuite » says it of a step.
  const todo = Array.from({ length: Math.max(0, (max ?? 0) - last) }, (_, i) => ({ n: last + i + 1, cls: 'todo' as const, next: i === 0 }));
  return [...done, ...todo];
}

/** The boxes of a prompt loop's list: from the last turn that read them, else from the reading. */
export function checkedOf(w: Workflow, tr: WorkflowTrack | null) {
  const turn = [...(tr?.turns ?? [])].reverse().find((u) => u.checked);
  return turn?.checked ?? w.promptLoop?.list?.says?.checked ?? null;
}

/** The count of the Workflow tab: the agents that follow a workflow of the project, once each (in « Tickets » too). */
export const followers = (tracks: WorkflowTrack[]) => new Set(tracks.filter((tr) => tr.workflow !== null).map((tr) => tr.agentId)).size;

// ---------- Where the card, the conversation and the Kanban meet the workflow (ESC-197, choices 145 and 159) ----------

/** The id of Escouade's own workflow, the loop of the tickets: no reading has it, the Kanban does. */
export const TICKETS = 'escouade:tickets';

/** The columns of the Kanban, the steps of « Tickets » in their order; « En revue » (ESC-59) while a ticket is there. */
const COLUMNS: Column[] = ['todo', 'doing', 'review', 'done'];
const WITH_PR: Column[] = ['todo', 'doing', 'review', 'pr', 'done'];

/**
 * « Tickets », the loop of the Kanban drawn as a workflow of Escouade's (choice 159): À faire → En cours (loop n/max, the
 * work then the check of the criteria) → À tester → Terminé, « Renvoyer » its way back; the autopilot its door. There
 * when the project's Kanban has tickets. Its steps are named as the columns, in the window's language.
 */
export function ticketsWorkflow(tickets: Ticket[]): Workflow | null {
  if (!tickets.length) return null;
  const name = (c: Column) => t(`board.columns.${c}`);
  const max = Math.max(...tickets.map((tk) => tk.maxLoops));
  const pr = tickets.some((tk) => tk.column === 'pr');
  const step = (c: Column, order: number): WorkflowStep => ({
    name: name(c),
    command: c === 'todo' ? t('workflow.ticketsDoor') : '',
    order,
    description: '',
    path: '',
    agents: [],
    reads: [],
    writes: [],
    named: true,
    ...(c === 'doing' ? { note: t('workflow.ticketsDoing', { count: max }) } : {}),
  });
  const go = (from: Column, to: Column): WorkflowTransition => ({
    from: name(from),
    to: name(to),
    back: false,
    condition: null,
    max: null,
  });
  return {
    id: TICKETS,
    name: t('workflow.ticketsName'),
    kind: 'cycle',
    harness: null,
    repo: '',
    doors: [],
    steps: (pr ? WITH_PR : COLUMNS).map(step),
    transitions: [
      go('todo', 'doing'),
      go('doing', 'review'),
      ...(pr ? [go('review', 'pr'), go('pr', 'done')] : [go('review', 'done')]),
      {
        from: name('review'),
        to: name('doing'),
        back: true,
        condition: null,
        max: null,
        say: t('workflow.sendBack', { step: name('doing') }),
      },
    ],
    stateFiles: [],
    exit: null,
    promptLoop: null,
  };
}

/**
 * Where each agent of a ticket is in « Tickets »: on its ticket's column, its loop beside it. An agent of a ticket that
 * follows the project's workflow too is in both (choice 159).
 */
export function ticketTracks(projectId: string, tickets: Ticket[], agents: Record<string, Agent>, now: number): WorkflowTrack[] {
  const out: WorkflowTrack[] = [];
  for (const tk of tickets) {
    const a = tk.agentId ? agents[tk.agentId] : undefined;
    if (!a || tk.projectId !== projectId || tk.column === 'todo') continue;
    const since = tk.startedAt ?? tk.createdAt;
    out.push({
      agentId: a.id,
      projectId,
      workflow: TICKETS,
      step: t(`board.columns.${tk.column}`),
      version: '',
      outside: null,
      subagents: [],
      passages: [],
      turns: [],
      support: { partial: false, missing: [] },
      since,
      ms: Math.max(0, (tk.doneAt ?? now) - since),
      cost: tk.cost,
      ticketLoop: tk.column === 'doing' ? [tk.iteration, tk.maxLoops] : null,
      updated: since,
      behind: false,
    });
  }
  return out;
}

/** The workflows of a project as the tab offers them: those read (corrections in), then « Tickets ». */
export const withTickets = (read: Workflow[], tickets: Workflow | null) => (tickets ? [...read, tickets] : read);

/** « GSD · Vérifier »: where an agent is in a workflow, as its card and the conversation say it; null out of any. */
export function stepTitle(all: Workflow[], workflow: string | null, step: string | null): string | null {
  const w = all.find((x) => x.id === workflow);
  if (!w || !step) return null;
  return `${workflowTitle(w)} · ${labelIn(w, step)}`;
}

/** The step an agent's card shows in the place of its plan: its workflow's (never « Tickets », the card has its loop). */
export const cardStep = (all: Workflow[], tr: WorkflowTrack | undefined | null) =>
  tr && !tr.outside && tr.workflow !== TICKETS ? stepTitle(all, tr.workflow, tr.step) : null;

/** « Ensuite : Exécuter »: what comes after the step on the card (ESC-244), in its tooltip; as `cardStep`, never « Tickets ». */
export function cardNext(all: Workflow[], tr: WorkflowTrack | undefined | null): string | null {
  const w = tr && tr.workflow !== TICKETS ? all.find((x) => x.id === tr.workflow) : undefined;
  return (w && nextSays(w, nextOf(w, tr!))) || null;
}

/** A divider of the conversation: the agent entering a step, by its door, at its time. */
export interface StepDivider {
  key: string;
  workflow: string;
  title: string;
  door: string | null;
  at: number;
}

type Timed = { id: string; kind: string; ts?: number; text?: string };

/**
 * The dividers of a conversation (choice 145), by the item each goes before: one at each entry in a step of a workflow
 * the project has. A step entered by its door goes before the message that typed it, else before the call (subagent,
 * skill, file) that signalled it; the fact is seen just after its item, within a moment.
 */
export function stepDividers(items: Timed[], passages: WorkflowPassage[], all: Workflow[]): Map<string, StepDivider[]> {
  const out = new Map<string, StepDivider[]>();
  // The fact is applied as its item comes: a moment after its time, never long.
  const SLACK = 2000;
  /** The last item between `from` and `to` that `ok` takes. */
  const last = (from: number, to: number, ok: (it: Timed) => boolean) => {
    let found: Timed | null = null;
    for (const it of items) if (it.ts !== undefined && it.ts >= from && it.ts <= to && ok(it)) found = it;
    return found;
  };
  for (const p of passages) {
    const title = p.step ? stepTitle(all, p.workflow, p.step) : null;
    if (!title) continue;
    const word = p.door?.trim().split(/\s/)[0];
    const anchor =
      (word ? last(p.at - 60_000, p.at + SLACK, (it) => it.kind === 'user' && !!it.text?.trimStart().startsWith(word)) : null) ??
      last(p.at - SLACK, p.at + 200, (it) => it.kind === 'tool') ??
      items.find((it) => it.ts !== undefined && it.ts >= p.at - SLACK);
    if (!anchor) continue;
    const list = out.get(anchor.id) ?? [];
    list.push({ key: `${p.workflow}#${p.n}`, workflow: p.workflow, title, door: p.door, at: p.at });
    out.set(anchor.id, list);
  }
  return out;
}
