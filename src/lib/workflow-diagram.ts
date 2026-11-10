// The Diagram view of a workflow (ESC-245, choices 196 to 212; `#loops-diagram*` of the preview): the workflow as a
// graph of its steps, laid out in layers by Escouade itself (not by Mermaid, choice 197), and its code in Mermaid for
// « Copier en Mermaid ». Pure: the component draws the shapes these give; every text in them comes from the repository's
// files and is drawn by Svelte as text, never as HTML.

import { t } from './i18n';
import type { Workflow, WorkflowTrack } from './types';
import { currentPhase, doneSteps, labelIn, nextOf, plainLine, stepsOf, TICKETS } from './workflow-view';

/** A workflow as the diagram reads it: its steps in their order, the ways forward and back, the entries, the exit. */
export interface DgModel {
  name: string;
  steps: { id: string; name: string; cmd: string; guessed?: boolean }[];
  /** Forward; two from one step make a choice, each with its condition. */
  go: { from: string; to: string; cond: string }[];
  /** Back to an earlier step, or to the same one again (a loop, `max` rounds). */
  back: { from: string; to: string; cond: string; max: number | null }[];
  entries: { door: string; to: string }[];
  exit: { from: string; cond: string; label: string; again: string } | null;
}

/** How many entries the head of the diagram shows (the tab's head lists them all, choice 144). */
const ENTRIES = 3;

/** The diagram of `w`: `ticketLoops`, the most loops of a ticket, for « Tickets » (its loop on « En cours »). */
export function diagramOf(w: Workflow, ticketLoops = 0): DgModel {
  if (w.kind === 'prompt' && w.promptLoop) {
    const loop = w.promptLoop;
    const steps = loop.steps.map((s) => ({ id: String(s.n), name: s.text, cmd: '' }));
    const go = steps.slice(1).map((s, i) => ({ from: steps[i].id, to: s.id, cond: '' }));
    const back = loop.returns.map((r) => ({ from: r.from, to: r.to, cond: r.say ?? r.condition ?? '', max: r.max }));
    const first = steps[0]?.id;
    const last = steps[steps.length - 1]?.id;
    const doors = [loop.prompt, loop.via].filter((d, i, all) => d && all.indexOf(d) === i);
    const exitSays = w.exit ? plainLine(w.exit) : loop.maxTurns ? t('workflow.dg.exitTurns', { count: loop.maxTurns }) : '';
    return {
      name: w.name,
      steps,
      go,
      back: back.filter((b) => steps.some((s) => s.id === b.from) && steps.some((s) => s.id === b.to)),
      entries: first ? doors.map((door) => ({ door, to: first })) : [],
      exit: last ? { from: last, cond: exitSays, label: t('workflow.dg.exit'), again: t('workflow.dg.nextTurn') } : null,
    };
  }
  const ordered = stepsOf(w);
  const names = new Set(ordered.map((s) => s.name));
  const steps = ordered.map((s) => ({
    id: s.name,
    name: labelIn(w, s.name),
    cmd: s.command || '',
    ...(s.guessed ? { guessed: true } : {}),
  }));
  const say = (cond: string | null, sayIt?: string) => sayIt ?? cond ?? '';
  const go: DgModel['go'] = [];
  const back: DgModel['back'] = [];
  for (const tr of w.transitions) {
    if (!names.has(tr.from) || !names.has(tr.to)) continue;
    if (tr.back) back.push({ from: tr.from, to: tr.to, cond: say(tr.condition, tr.say), max: tr.max });
    else if (!go.some((g) => g.from === tr.from && g.to === tr.to)) go.push({ from: tr.from, to: tr.to, cond: tr.condition ?? '' });
  }
  // A step with no transition written at all still leads to the next one: the rail's order is the workflow's. One with
  // only a way back (a branch that corrects, then returns) leads nowhere else.
  ordered.forEach((s, i) => {
    const next = ordered[i + 1];
    if (next && !w.transitions.some((tr) => tr.from === s.name)) go.push({ from: s.name, to: next.name, cond: '' });
  });
  // « Tickets » loops on « En cours » until the criteria are met, up to the most loops of its tickets.
  if (w.id === TICKETS && ordered[1] && ticketLoops)
    back.unshift({ from: ordered[1].name, to: ordered[1].name, cond: t('workflow.dg.criteriaNotMet'), max: ticketLoops });
  // A condition only where the step has another way out (a choice, or a way back): a single way on needs none.
  for (const g of go)
    if (go.filter((x) => x.from === g.from).length < 2 && !back.some((b) => b.from === g.from && b.to !== g.from)) g.cond = '';
  const first = ordered[0]?.name;
  const commands = new Map(ordered.map((s) => [s.command, s.name]));
  const entries = w.doors
    .filter((d) => d.role === 'entry' && d.command && !commands.has(d.command))
    .slice(0, ENTRIES)
    .map((d) => ({ door: d.command, to: d.enters.find((e) => names.has(e)) ?? first }))
    .filter((e): e is { door: string; to: string } => !!e.to);
  const last = ordered[ordered.length - 1]?.name;
  return {
    name: w.name,
    steps,
    go,
    back,
    entries,
    exit: w.exit && last ? { from: last, cond: plainLine(w.exit), label: t('workflow.dg.exit'), again: t('workflow.dg.nextRound') } : null,
  };
}

// ---------- From an entry: what it reaches (ESC-406) ----------

/** A way into the workflow: what is typed (a command, a skill, the prompt of a loop, else the step's own name) and the step it enters. */
export type DgEntry = DgModel['entries'][number];

/**
 * Every way into `w`, to choose from when no agent is read: its entry doors, each into the step it enters, the first
 * step in front when no door leads to it (alone when the workflow has no door). The diagram's own `entries` stay the
 * three of « Copier en Mermaid ».
 */
export function entriesOf(w: Workflow, m: DgModel): DgEntry[] {
  if (w.kind === 'prompt') return m.entries;
  const first = m.steps[0];
  if (!first) return [];
  const ids = new Set(m.steps.map((s) => s.id));
  const byCmd = new Map(m.steps.filter((s) => s.cmd).map((s) => [s.cmd, s.id]));
  const out: DgEntry[] = [];
  for (const d of w.doors) {
    if (d.role !== 'entry' || !d.command || out.some((e) => e.door === d.command)) continue;
    out.push({ door: d.command, to: d.enters.find((e) => ids.has(e)) ?? byCmd.get(d.command) ?? first.id });
  }
  if (!out.some((e) => e.to === first.id)) out.unshift({ door: first.cmd || first.name, to: first.id });
  return out;
}

/** The steps reached from `to` by the ways forward and the ways back, `to` first; none when `to` is no step. */
export function reachOf(m: DgModel, to: string): Set<string> {
  const reach = new Set<string>();
  if (!m.steps.some((s) => s.id === to)) return reach;
  const todo = [to];
  while (todo.length) {
    const at = todo.shift()!;
    if (reach.has(at)) continue;
    reach.add(at);
    for (const e of [...m.go, ...m.back]) if (e.from === at && !reach.has(e.to)) todo.push(e.to);
  }
  return reach;
}

/**
 * The entry `tr` took: the door of its first passage as typed, without its arguments, else the command of the step it
 * began in. Without a history (« Tickets », a track just read), the first entry that reaches its step, else its step.
 * Null when it cannot be known: no agent, another workflow, steps the workflow no longer has.
 */
export function entryOf(w: Workflow, m: DgModel, tr: WorkflowTrack | null): DgEntry | null {
  if (!tr || tr.workflow !== w.id) return null;
  if (w.kind === 'prompt') return m.entries[0] ?? null;
  const steps = new Map(m.steps.map((s) => [s.id, s]));
  const began = currentPhase(w, tr.passages).find((p) => p.step && steps.has(p.step));
  if (began) {
    const s = steps.get(began.step!)!;
    return { door: began.door?.trim().split(/\s+/)[0] || s.cmd || s.name, to: s.id };
  }
  const on = tr.step ? steps.get(tr.step) : undefined;
  if (!on) return null;
  return entriesOf(w, m).find((e) => reachOf(m, e.to).has(on.id)) ?? { door: on.cmd || on.name, to: on.id };
}

/**
 * The diagram from `entry`: only the steps it reaches, their ways, the exit when its step is reached (the next round
 * only when it goes back to a step reached); `off`, the steps out of this path, folded under the diagram. An entry
 * that leads to no step known keeps the whole diagram. `passed`, the steps the agent went through: one it reached
 * otherwise (a command typed on the way) stays, with where it leads.
 */
export function scopeTo(m: DgModel, entry: DgEntry, passed: string[] = []): { model: DgModel; off: DgModel['steps'] } {
  const reach = reachOf(m, entry.to);
  if (!reach.size) return { model: { ...m, entries: [] }, off: [] };
  for (const id of passed) if (!reach.has(id)) for (const s of reachOf(m, id)) reach.add(s);
  const exit = m.exit && reach.has(m.exit.from) ? { ...m.exit, again: reach.has(m.steps[0].id) ? m.exit.again : '' } : null;
  return {
    model: {
      name: m.name,
      steps: m.steps.filter((s) => reach.has(s.id)),
      go: m.go.filter((g) => reach.has(g.from) && reach.has(g.to)),
      back: m.back.filter((b) => reach.has(b.from) && reach.has(b.to)),
      entries: [entry],
      exit,
    },
    off: m.steps.filter((s) => !reach.has(s.id)),
  };
}

// ---------- Mermaid ----------

/**
 * A text of the repository inside a Mermaid label, between double quotes: whatever could close the quotes, open HTML,
 * a Markdown string or an entity is written as Mermaid's entity codes; a line break is a space; control characters go.
 */
export function mermaidText(s: string): string {
  return (
    s
      .replace(/[\r\n\t]+/g, ' ')
      // eslint-disable-next-line no-control-regex
      .replace(/[\u0000-\u001f\u007f]/g, '')
      .replace(/#/g, '#35;')
      .replace(/"/g, '#quot;')
      .replace(/&/g, '#amp;')
      .replace(/</g, '#lt;')
      .replace(/>/g, '#gt;')
      .replace(/`/g, '#96;')
      .trim()
  );
}

/**
 * The Mermaid code of a workflow, without its agents (they move; a doc or a PR keeps the workflow). The ids are ours
 * (`s1`, `in1`, `out`), never a name of the repository; no `click`, link or style comes from it.
 */
export function toMermaid(m: DgModel): string {
  const id = (s: string) => `s${m.steps.findIndex((x) => x.id === s) + 1}`;
  const q = (s: string) => `"${mermaidText(s)}"`;
  const out = ['flowchart TD', `  %% ${mermaidText(m.name)}`];
  m.entries.forEach((e, i) => out.push(`  in${i + 1}([${q(e.door)}]) --> ${id(e.to)}`));
  for (const s of m.steps) out.push(`  ${id(s.id)}["${mermaidText(s.name)}${s.cmd ? `<br>${mermaidText(s.cmd)}` : ''}"]`);
  for (const g of m.go) out.push(`  ${id(g.from)} -->${g.cond ? `|${q(g.cond)}|` : ''} ${id(g.to)}`);
  for (const b of m.back) {
    const say = [b.cond, b.max ? t('workflow.dg.max', { count: b.max }) : ''].filter(Boolean).join(' · ');
    out.push(`  ${id(b.from)} -.->${say ? `|${q(say)}|` : ''} ${id(b.to)}`);
  }
  if (m.exit) {
    out.push(`  ${id(m.exit.from)} -->${m.exit.cond ? `|${q(m.exit.cond)}|` : ''} out((${q(m.exit.label)}))`);
    if (m.exit.again && m.steps[0]) out.push(`  out -.->|${q(m.exit.again)}| ${id(m.steps[0].id)}`);
  }
  if (m.entries.length) {
    out.push('  classDef door stroke-dasharray: 3 3');
    out.push(`  class ${m.entries.map((_, i) => `in${i + 1}`).join(',')} door`);
  }
  return out.join('\n');
}

// ---------- Where the agents are ----------

/** `seen`: passed by the agent, not done in its pass (a way back undid it). */
export type DgState = 'done' | 'on' | 'next' | 'seen' | '';

/** One stay of the agent in a step, in the order of its path (ESC-406). */
export interface DgPass {
  /** Its number in the path, from 1. */
  n: number;
  step: string;
  name: string;
  /** The nth stay in this step. */
  pass: number;
  /** How it came there: by the entry, a way forward, a way back, or a round of the step's own loop. */
  via: 'entry' | 'go' | 'back' | 'loop';
  from: string | null;
  /** Why it came back or went round: as the step wrote it, else the condition of the workflow. */
  reason: string;
  door: string | null;
  /** The agent is there now. */
  on: boolean;
  ms: number;
}

/**
 * The real path of `tr` in `w`, in the order it went (ESC-406): its passages of the current phase, as the tracking
 * keeps them (they survive a restart), each with how it came there. What is no step of the workflow is skipped.
 */
export function pathOf(w: Workflow, m: DgModel, tr: WorkflowTrack | null): DgPass[] {
  if (!tr || tr.workflow !== w.id) return [];
  const at = new Map(m.steps.map((s, i) => [s.id, i]));
  const ps = currentPhase(w, tr.passages).filter((p) => p.step && at.has(p.step));
  const stays: Record<string, number> = {};
  return ps.map((p, i) => {
    const step = p.step!;
    const prev = ps[i - 1];
    const door = p.door?.trim().split(/\s+/)[0] || null;
    let via: DgPass['via'] = 'entry';
    let reason = '';
    if (prev) {
      const back = m.back.find((b) => b.from === prev.step && b.to === step);
      via = prev.step === step ? 'loop' : back || prev.outcome === 'back' || at.get(step)! < at.get(prev.step!)! ? 'back' : 'go';
      if (via !== 'go') reason = prev.reason ?? back?.cond ?? '';
    }
    return {
      n: i + 1,
      step,
      name: m.steps[at.get(step)!].name,
      pass: (stays[step] = (stays[step] ?? 0) + 1),
      via,
      from: prev?.step ?? null,
      reason,
      door: prev ? door : door || m.steps[at.get(step)!].cmd || null,
      on: i === ps.length - 1 && !tr.outside && tr.step === step,
      ms: p.ms,
    };
  });
}

/**
 * One agent's progress on the diagram: the steps done, the one it is on, the next ones; and its path on it (ESC-406):
 * the numbers of its passages by step, the ways forward and back it took, the rounds of each loop, why it came back.
 */
export interface DgProgress {
  done: string[];
  on: string | null;
  next: string[];
  /** The exit comes next (the end of the cycle). */
  exitNext?: boolean;
  /** The ways back and the loops taken, `from>to`. */
  took: string[];
  /** The numbers of the passages in each step, in their order. */
  seen: Record<string, number[]>;
  /** The number of the passage in course. */
  at: number | null;
  /** The ways forward taken, `from>to`. */
  walked: string[];
  /** The rounds taken of a step's own loop. */
  rounds: Record<string, number>;
  /** Why the last time, by way back or loop (`from>to`). */
  reasons: Record<string, string>;
}

/** No agent read: nothing taken. */
export const NO_PROGRESS: DgProgress = { done: [], on: null, next: [], took: [], seen: {}, at: null, walked: [], rounds: {}, reasons: {} };

/** The progress of `tr` in `w` (of the current phase): done, on, next (each branch of a choice), the ways back taken. */
export function progressOf(w: Workflow, m: DgModel, tr: WorkflowTrack | null): DgProgress {
  if (!tr) return { ...NO_PROGRESS };
  // The steps done and the next as the column says them (ESC-244): the diagram adds the other branches of a choice.
  const done = w.kind === 'prompt' ? [] : doneSteps(w, tr);
  const on = tr.outside ? null : (tr.step ?? null);
  const onId = on && m.steps.some((s) => s.id === on) ? on : null;
  const n = nextOf(w, tr);
  const from = n?.to ? (onId ?? m.go.find((g) => g.to === n.to)?.from ?? null) : null;
  const branches = n?.to ? [n.to, ...m.go.filter((g) => g.from === from && g.to !== n.to).map((g) => g.to)] : [];
  const took = new Set<string>();
  const walked = new Set<string>();
  const seen: DgProgress['seen'] = {};
  const rounds: DgProgress['rounds'] = {};
  const reasons: DgProgress['reasons'] = {};
  const path = pathOf(w, m, tr);
  for (const p of path) {
    (seen[p.step] ??= []).push(p.n);
    const way = `${p.from}>${p.step}`;
    if (p.via === 'go') walked.add(way);
    else if (p.via !== 'entry') {
      took.add(way);
      if (p.reason) reasons[way] = p.reason;
      if (p.via === 'loop') rounds[p.step] = (rounds[p.step] ?? 0) + 1;
    }
  }
  return {
    done,
    on: onId,
    next: branches.filter((x) => !done.includes(x) && x !== onId),
    ...(n?.exit && m.exit ? { exitNext: true } : {}),
    took: [...took],
    seen,
    at: path.find((p) => p.on)?.n ?? null,
    walked: [...walked],
    rounds,
    reasons,
  };
}

// ---------- The layout ----------

/** The width a text takes, in px, in one of the diagram's fonts. */
export type Measure = (text: string, font: 'name' | 'cmd' | 'chip' | 'door' | 'exit') => number;

export interface DgNode {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** As drawn (cut by an ellipsis), and whole. */
  name: string;
  full: string;
  cmd: string;
  state: DgState;
  guessed: boolean;
  /** A loop on itself: its pill, its rounds, its condition. */
  self: { x: number; y: number; max: number | null; cond: string; took: boolean; turns: number } | null;
  dots: { cx: number; cy: number; id: string; cur: boolean; wait: boolean; name: string }[];
  more: string;
  /** Where the count of the agents beyond three ends. */
  moreX: number;
  /** « ensuite » or « en cours », in words at the foot of the step, when there is room. */
  tag: { x: number; y: number; text: string } | null;
  /** The numbers of the agent's passages in this step (ESC-406), the last ones when there are many. */
  beads: { cx: number; cy: number; n: number; on: boolean }[];
  /** How many passages the beads leave out (« +6 »). */
  beadsMore: { x: number; y: number; text: string } | null;
  /** The diamond of a choice. */
  fork: string | null;
  /** Enlarged: its agent by name under it. */
  who: { x: number; y: number; w: number; name: string; all: string; more: string; cur: boolean; wait: boolean } | null;
}

export interface DgChip {
  x: number;
  y: number;
  w: number;
  text: string;
  full: string;
  kind: 'cond' | 'ret' | 'again';
  took: boolean;
}

export interface DgLayout {
  width: number;
  height: number;
  /** The lines, then their heads: `fwd`, `walked`, `door`, `back`, `took`, `again`. */
  edges: { d: string; cls: string }[];
  nodes: DgNode[];
  /** `taken`: the door the agent came in by, drawn full; `label`, « Entrée » in front of it. */
  doors: { x: number; y: number; w: number; text: string; full: string; label: string; taken: boolean }[];
  chips: DgChip[];
  /** Next: the end of the cycle comes after the step the agent is on. */
  exit: { x: number; y: number; w: number; h: number; label: string; next: boolean } | null;
}

export interface DgAgent {
  id: string;
  name: string;
  cur: boolean;
  wait: boolean;
}

/** A text cut by an ellipsis to `max` px. */
export function fit(text: string, max: number, measure: Measure, font: Parameters<Measure>[1]): string {
  if (measure(text, font) <= max) return text;
  let s = text;
  while (s.length > 1 && measure(`${s}…`, font) > max) s = s.slice(0, -1);
  return `${s.trimEnd()}…`;
}

/** The rank of each step: the longest way forward to it (the ways back do not count). */
export function ranks(m: DgModel): { rank: Record<string, number>; rows: DgModel['steps'][] } {
  const rank: Record<string, number> = Object.fromEntries(m.steps.map((s) => [s.id, 0]));
  // A cycle of ways forward (a workflow that writes one) stops at the number of steps.
  for (let k = 0; k < m.steps.length; k++)
    for (const g of m.go)
      if (m.steps.findIndex((s) => s.id === g.to) > m.steps.findIndex((s) => s.id === g.from))
        rank[g.to] = Math.max(rank[g.to], rank[g.from] + 1);
  const rows: DgModel['steps'][] = [];
  for (const s of m.steps) (rows[rank[s.id]] ??= []).push(s);
  return { rank, rows: rows.filter(Boolean) };
}

/**
 * The shapes of the diagram. `dir` TD down the column (its `width`), LR across when enlarged (choice 208). `progress`, the
 * agent read; `agents`, who is on each step; `names`, the agents by name under their step (enlarged).
 */
export function layout(
  m: DgModel,
  {
    dir = 'TD',
    width = 312,
    progress = NO_PROGRESS,
    agents = {},
    names = false,
    measure,
    maxRounds = (n: number) => `×${n}`,
    entryLabel = '',
    tags = { next: '', on: '' },
  }: {
    dir?: 'TD' | 'LR';
    width?: number;
    progress?: DgProgress;
    agents?: Record<string, DgAgent[]>;
    names?: boolean;
    measure: Measure;
    maxRounds?: (n: number) => string;
    /** « Entrée », written in the door. */
    entryLabel?: string;
    /** The words at the foot of the next step and of the one in course. */
    tags?: { next: string; on: string };
  },
): DgLayout {
  const lr = dir === 'LR';
  const { rows } = ranks(m);
  // The rows renumbered without holes.
  const rank: Record<string, number> = {};
  rows.forEach((row, r) => row.forEach((s) => (rank[s.id] = r)));
  const done = new Set(progress.done);
  const took = new Set(progress.took);
  const nexts = new Set(progress.next);
  const walked = new Set(progress.walked);
  const hasPath = Object.keys(progress.seen).length > 0;
  // The one door of a diagram scoped to its entry is the one the agent read came in by.
  const taken = m.entries.length === 1 && (hasPath || !!progress.on || progress.done.length > 0);
  const backs = m.back.filter((b) => b.from !== b.to && rank[b.to] < rank[b.from]);
  const selfs = Object.fromEntries(m.back.filter((b) => b.from === b.to).map((b) => [b.from, b]));
  // A way back goes by the left gutter when the step it leaves has a neighbour on its right.
  const side = (b: DgModel['back'][number]) => {
    const row = rows[rank[b.from]];
    return row.length > 1 && row[row.length - 1].id !== b.from ? 'L' : 'R';
  };
  const lanesR = backs.filter((b) => side(b) === 'R').sort((a, b) => rank[a.from] - rank[a.to] - (rank[b.from] - rank[b.to]));
  const lanesL = backs.filter((b) => side(b) === 'L');
  const again = !!(m.exit && m.exit.again && m.steps.length);
  // Across, a lane holds the condition of its way back on it: wider.
  const LANE = lr ? 26 : 11;
  const NODE_MAIN = lr ? 152 : 46;
  const GAP = lr ? 52 : 34;
  const midEntries = m.entries.filter((e) => rank[e.to] > 0);
  const topEntries = m.entries.filter((e) => rank[e.to] === 0);
  const entryBand = lr && midEntries.length ? 34 : 0;
  const againL = again && !lr ? 1 : 0;
  const againR = again && lr ? 1 : 0;
  const before = (lr ? 14 : 6) + (lanesL.length + againL) * LANE + (lanesL.length || againL ? 8 : 0) + entryBand;
  const after = (lr ? 14 : 6) + (lanesR.length + againR) * LANE + (lanesR.length || againR ? 8 : 0);
  // Across, a step is 56 px high, its agent by name under it, in its slot.
  const crossSize = lr ? 56 + (names ? 26 : 0) : Math.max(width - before - after, 120);
  const crossGap = lr ? 30 : 10;
  const cols = Math.max(1, ...rows.map((r) => r.length));
  const crossTotal = lr ? cols * crossSize + (cols - 1) * crossGap : crossSize;
  type Box = { main: number; mainSize: number; cross: number; crossSize: number };
  const box: Record<string, Box> = {};
  const labelW = entryLabel ? measure(entryLabel, 'chip') + 6 : 0;
  const doorW = (text: string) => measure(text, 'door') + 14 + labelW;
  const midDoorW: Record<string, number> = Object.fromEntries(midEntries.map((e) => [e.to, doorW(e.door)]));
  // Between two ranks: in the column, the condition of a way back takes the top of the gap, a choice's the bottom.
  const forkFrom = new Set(m.go.filter((g) => m.go.filter((x) => x.from === g.from).length > 1).map((g) => g.from));
  const forkGap = new Set([...forkFrom].map((f) => rank[f]));
  const retIn: Record<number, number> = {};
  const retGap = new Map<DgModel['back'][number], number>();
  for (const b of backs) {
    const score = (x: number) => (retIn[x] || 0) * 2 + (forkGap.has(x) ? 1 : 0);
    let best = rank[b.to];
    for (let g = rank[b.to]; g < rank[b.from]; g++) if (score(g) < score(best)) best = g;
    retGap.set(b, best);
    retIn[best] = (retIn[best] || 0) + 1;
  }
  const gapOf = (g: number) => (lr ? GAP : retIn[g] && forkGap.has(g) ? 56 : retIn[g] ? 46 : forkGap.has(g) ? 40 : GAP);
  const splitAt = (g: number) => (lr ? GAP / 2 : retIn[g] ? 30 : gapOf(g) / 2);
  let main = lr ? 12 : 4;
  const doorsAt = main;
  if (topEntries.length) main += lr ? Math.max(...topEntries.map((e) => doorW(e.door))) + 40 : 44;
  const entryOff: Record<string, number> = {};
  const rowMain: number[] = [];
  rows.forEach((row, r) => {
    const off = !lr && row.some((s) => midDoorW[s.id]) ? 26 : 0;
    main += off;
    rowMain[r] = main;
    const slot = lr ? crossSize : (crossSize - crossGap * (row.length - 1)) / row.length;
    const each = lr ? 56 : slot;
    const span = slot * row.length + crossGap * (row.length - 1);
    const start = before + (lr ? (crossTotal - span) / 2 : 0);
    row.forEach((s, i) => {
      box[s.id] = { main, mainSize: NODE_MAIN, cross: start + i * (slot + crossGap), crossSize: each };
      entryOff[s.id] = off;
    });
    main += NODE_MAIN + gapOf(r);
  });
  if (rows.length) main -= gapOf(rows.length - 1);
  // Where an edge comes into a step: its middle, or clear of the door that comes in above it.
  const arrive = (id: string) =>
    box[id].cross + (midDoorW[id] && !lr ? Math.max(box[id].crossSize / 2, midDoorW[id] + 30) : box[id].crossSize / 2);
  let exitBox: Box | null = null;
  if (m.exit && box[m.exit.from]) {
    // Down the column, a condition on the way to the exit leaves the line seen above and under it.
    main += m.exit.cond ? (lr ? Math.max(GAP, Math.min(measure(m.exit.cond, 'chip'), 160) + 36) : 46) : GAP;
    const len = measure(m.exit.label, 'exit') + 44;
    const ex = box[m.exit.from];
    exitBox = lr
      ? { main, mainSize: len, cross: ex.cross + ex.crossSize / 2 - 15, crossSize: 30 }
      : { main, mainSize: 30, cross: before + crossSize / 2 - len / 2, crossSize: len };
    main += exitBox.mainSize;
  }
  main += lr ? 14 : 6;
  const W = lr ? main : width;
  const H = lr ? before + crossTotal + after : main;
  const P = (mm: number, c: number): [number, number] => (lr ? [mm, c] : [c, mm]);
  const R = (b: Box) =>
    lr ? { x: b.main, y: b.cross, w: b.mainSize, h: b.crossSize } : { x: b.cross, y: b.main, w: b.crossSize, h: b.mainSize };
  const r1 = (n: number) => Math.round(n * 10) / 10;
  const path = (pts: [number, number][], r = 6) => {
    // An orthogonal path with rounded corners.
    let d = `M${r1(pts[0][0])} ${r1(pts[0][1])}`;
    for (let i = 1; i < pts.length; i++) {
      const [x, y] = pts[i];
      const next = pts[i + 1];
      if (!next) {
        d += ` L${r1(x)} ${r1(y)}`;
        break;
      }
      const [px, py] = pts[i - 1];
      const k1 = Math.min(r, Math.hypot(x - px, y - py) / 2);
      const k2 = Math.min(r, Math.hypot(next[0] - x, next[1] - y) / 2);
      const [ux, uy, vx, vy] = [Math.sign(x - px), Math.sign(y - py), Math.sign(next[0] - x), Math.sign(next[1] - y)];
      d += ` L${r1(x - ux * k1)} ${r1(y - uy * k1)} Q${r1(x)} ${r1(y)} ${r1(x + vx * k2)} ${r1(y + vy * k2)}`;
    }
    return d;
  };
  const edges: DgLayout['edges'] = [];
  const heads: DgLayout['edges'] = [];
  const arrow = (x: number, y: number, dx: number, dy: number, cls: string) => {
    const s = 4;
    heads.push({
      d: `M${r1(x - dx * s - dy * s)} ${r1(y - dy * s - dx * s)} L${r1(x)} ${r1(y)} L${r1(x - dx * s + dy * s)} ${r1(y - dy * s + dx * s)}`,
      cls: `${cls} head`,
    });
  };
  const chips: DgChip[] = [];
  const chip = (x: number, y: number, text: string, kind: DgChip['kind'], anchor: 'mid' | 'start' | 'end', max: number, tk = false) => {
    const shown = fit(text, Math.max(max, 24), measure, 'chip');
    const w = measure(shown, 'chip') + 12 + (kind === 'ret' ? 13 : 0);
    const left = anchor === 'end' ? x - w : anchor === 'start' ? x : x - w / 2;
    chips.push({ x: r1(left), y: r1(y - 9), w: r1(w), text: shown, full: text, kind, took: tk });
  };
  const doors: DgLayout['doors'] = [];
  const doorCls = taken ? 'door walked' : 'door';
  const door = (x: number, y: number, text: string) => {
    // Down the column, a long door is cut to the width left.
    const shown = lr ? text : fit(text, Math.max(width - x - 14 - labelW - 2, 24), measure, 'door');
    const w = doorW(shown);
    doors.push({ x: r1(x), y: r1(y), w: r1(w), text: shown, full: text, label: entryLabel, taken });
    return w;
  };
  // The entries: at the head, into the first step; one into a later step comes in just above it.
  if (topEntries.length) {
    const first = R(box[topEntries[0].to]);
    if (lr) {
      let y = first.y + first.h / 2 - (topEntries.length * 26 - 6) / 2;
      for (const e of topEntries) {
        const w = door(doorsAt, y, e.door);
        edges.push({
          d: path(
            [
              [doorsAt + w, y + 10],
              [first.x - 14, y + 10],
              [first.x - 14, first.y + first.h / 2],
              [first.x - 1, first.y + first.h / 2],
            ],
            5,
          ),
          cls: doorCls,
        });
        y += 26;
      }
      arrow(first.x - 1, first.y + first.h / 2, 1, 0, doorCls);
    } else {
      const widths = topEntries.map((e) => doorW(e.door));
      const total = widths.reduce((a, b) => a + b, 0) + (widths.length - 1) * 6;
      const cx = first.x + first.w / 2;
      let x = Math.max(first.x, cx - total / 2);
      topEntries.forEach((e, i) => {
        door(x, doorsAt, e.door);
        const mx = x + widths[i] / 2;
        edges.push({
          d: path(
            [
              [mx, doorsAt + 20],
              [mx, doorsAt + 30],
              [cx, doorsAt + 30],
              [cx, first.y - 1],
            ],
            5,
          ),
          cls: doorCls,
        });
        x += widths[i] + 6;
      });
      arrow(cx, first.y - 1, 0, 1, doorCls);
    }
  }
  for (const e of midEntries) {
    const b = R(box[e.to]);
    const w = door(b.x + 8, b.y - (lr ? 32 : 25), e.door);
    edges.push({ d: `M${r1(b.x + 8 + w / 2)} ${r1(b.y - (lr ? 12 : 5))} V${r1(b.y - 1)}`, cls: doorCls });
    arrow(b.x + 8 + w / 2, b.y - 1, 0, 1, doorCls);
  }
  // The ways forward: from the end of a step to the start of the next, turning in the middle of the gap.
  for (const g of m.go) {
    const a = box[g.from];
    const b = box[g.to];
    if (!a || !b || rank[g.to] <= rank[g.from]) continue;
    const am = a.main + a.mainSize;
    const bm = b.main;
    const ac = a.cross + a.crossSize / 2;
    const bc = arrive(g.to);
    const mid = am + splitAt(rank[g.from]);
    // Taken: as the path went; without a history (« Tickets »), between the steps done up to the one in course.
    const went = hasPath ? walked.has(`${g.from}>${g.to}`) : done.has(g.from) && (done.has(g.to) || progress.on === g.to);
    const cls = went ? 'walked' : 'fwd';
    const pts: [number, number][] =
      Math.abs(ac - bc) < 0.5 ? [P(am, ac), P(bm - 1, bc)] : [P(am, ac), P(mid, ac), P(mid, bc), P(bm - 1, bc)];
    edges.push({ d: path(pts), cls });
    arrow(...P(bm - 1, bc), lr ? 1 : 0, lr ? 0 : 1, cls);
    if (g.cond) {
      const [cx, cy] = lr ? P(mid + (bm - mid) / 2, bc) : P(bm - 12 - entryOff[g.to], bc);
      chip(cx, cy, g.cond, 'cond', 'mid', lr ? 160 : Math.max(b.crossSize - 16, 40));
    }
  }
  // The ways back, by a lane of their gutter: out of the side of the step, back into the side of the earlier one.
  const laneX = (b: DgModel['back'][number]) =>
    side(b) === 'R'
      ? before + crossTotal + 8 + lanesR.indexOf(b) * LANE + 3
      : before - entryBand - 8 - againL * LANE - lanesL.indexOf(b) * LANE - 3;
  for (const b of backs) {
    const s = box[b.from];
    const tb = box[b.to];
    const L = laneX(b);
    const right = side(b) === 'R';
    const sEdge = right ? s.cross + s.crossSize : s.cross;
    const tEdge = right ? tb.cross + tb.crossSize : tb.cross;
    const sm = s.main + s.mainSize * (lr ? 0.88 : 0.66);
    const tm = tb.main + tb.mainSize * (lr ? 0.82 : 0.34);
    const tk = took.has(`${b.from}>${b.to}`);
    const cls = tk ? 'took' : 'back';
    const d1 = right ? 1 : -1;
    edges.push({ d: path([P(sm, sEdge), P(sm, L), P(tm, L), P(tm, tEdge + d1)], 7), cls });
    arrow(...P(tm, tEdge + d1), lr ? 0 : -d1, lr ? -d1 : 0, cls);
    // Taken: why the agent came back, as the step wrote it; else the condition of the workflow.
    const say = (tk && progress.reasons[`${b.from}>${b.to}`]) || [b.cond, b.max ? maxRounds(b.max) : ''].filter(Boolean).join(' · ');
    if (!say) continue;
    if (lr) chip(...P(tb.main + tb.mainSize + (s.main - tb.main - tb.mainSize) / 2, L), say, 'ret', 'mid', 220, tk);
    else
      chip(
        right ? L + 5 : L - 5,
        rowMain[retGap.get(b)!] + NODE_MAIN + 12,
        say,
        'ret',
        right ? 'end' : 'start',
        Math.min(170, crossSize - 20),
        tk,
      );
  }
  // The exit, and the next round back to the first step (dashed and neutral).
  let exit: DgLayout['exit'] = null;
  if (exitBox && m.exit) {
    const a = box[m.exit.from];
    const am = a.main + a.mainSize;
    const ac = a.cross + a.crossSize / 2;
    const em = exitBox.main;
    const ec = exitBox.cross + exitBox.crossSize / 2;
    const cls = done.has(m.exit.from) ? 'walked' : 'fwd';
    edges.push({ d: path([P(am, ac), P(em - 1, ec)]), cls });
    arrow(...P(em - 1, ec), lr ? 1 : 0, lr ? 0 : 1, cls);
    if (m.exit.cond) {
      const [cx, cy] = lr ? P(am + (em - am) / 2, ac) : [ac, am + (em - am) / 2 - 1];
      chip(cx, cy, m.exit.cond, 'cond', 'mid', lr ? 160 : Math.min(crossSize - 20, 240));
    }
    const e = R(exitBox);
    exit = { x: r1(e.x), y: r1(e.y), w: r1(e.w), h: r1(e.h), label: m.exit.label, next: !!progress.exitNext };
    if (again) {
      const f = box[m.steps[0].id];
      const L = before - 8 - 3;
      const sm = exitBox.main + exitBox.mainSize / 2;
      const sx = lr ? exitBox.cross + exitBox.crossSize : exitBox.cross;
      const tm = f.main + f.mainSize / 2;
      const Lc = lr ? before + crossTotal + 8 + lanesR.length * LANE + 3 : L;
      const tEdge = lr ? f.cross + f.crossSize : f.cross;
      const s2 = lr ? 1 : -1;
      edges.push({ d: path([P(sm, sx), P(sm, Lc), P(tm, Lc), P(tm, tEdge - s2)], 8), cls: 'again' });
      arrow(...P(tm, tEdge - s2), lr ? 0 : 1, lr ? -1 : 0, 'again');
      if (lr) chip(...P((tm + sm) / 2, Lc), m.exit.again, 'again', 'mid', 160);
      else chip((L + exitBox.cross) / 2, sm, m.exit.again, 'again', 'mid', exitBox.cross - L - 10);
    }
  }
  // The steps: done, on (the agent read), next; a loop on itself; who is on it.
  const nodes: DgNode[] = m.steps.map((s) => {
    const b = R(box[s.id]);
    const passed = progress.seen[s.id] ?? [];
    const state: DgState = progress.on === s.id ? 'on' : done.has(s.id) ? 'done' : nexts.has(s.id) ? 'next' : passed.length ? 'seen' : '';
    const on = agents[s.id] ?? [];
    const loop = selfs[s.id];
    // The head of the step: its name, then the numbers of the agent's passages, the last three beyond four.
    const shownPasses = passed.length > 4 ? passed.slice(-3) : passed;
    const beads = shownPasses.map((n, i) => ({
      cx: r1(b.x + b.w - 14 - (shownPasses.length - 1 - i) * 19),
      cy: r1(b.y + 14),
      n,
      on: n === progress.at,
    }));
    const moreText = passed.length > shownPasses.length ? `+${passed.length - shownPasses.length}` : '';
    const beadsW = beads.length ? beads.length * 19 + 4 + (moreText ? measure(moreText, 'chip') + 6 : 0) : 0;
    const beadsMore = moreText ? { x: r1(b.x + b.w - 14 - beads.length * 19 + 6), y: r1(b.y + 17.6), text: moreText } : null;
    const check = state === 'done' && !beads.length;
    const room = b.w - 24 - (check ? 16 : 0) - beadsW;
    // The foot of the step, from the right: its loop, « ensuite » or « en cours », who is on it; then its command.
    const footY = b.y + b.h - (lr ? 18 : 14);
    let foot = b.x + b.w - 10 - (loop ? 50 : 0);
    const tagText = state === 'on' ? tags.on : state === 'next' ? tags.next : '';
    const tagW = tagText ? measure(tagText, 'chip') : 0;
    const tag = tagText && foot - tagW - b.x > 96 ? { x: r1(foot), y: r1(footY + 3.6), text: tagText } : null;
    if (tag) foot -= tagW + 8;
    const dots = on
      .slice(0, 3)
      .map((a, i) => ({ cx: r1(foot - 4 - i * 9), cy: r1(footY), id: a.id, cur: a.cur, wait: a.wait, name: a.name }));
    const moreX = foot - Math.min(on.length, 3) * 9 - 4;
    if (on.length) foot = moreX - (on.length > 3 ? measure(`+${on.length - 3}`, 'chip') + 4 : 0);
    let who: DgNode['who'] = null;
    if (names && on.length) {
      // Enlarged: the agent read first, else the first; the others by their count. Clear of the ways back.
      const a = on.find((x) => x.cur) ?? on[0];
      const more = on.length > 1 ? `+${on.length - 1}` : '';
      const shown = fit(a.name, b.w * 0.72 - 22 - (more ? 22 : 0), measure, 'chip');
      who = {
        x: r1(b.x),
        y: r1(b.y + b.h + 6),
        w: r1(measure(shown, 'chip') + 22),
        name: shown,
        all: on.map((x) => x.name).join(', '),
        more,
        cur: a.cur,
        wait: a.wait,
      };
    }
    return {
      id: s.id,
      x: r1(b.x),
      y: r1(b.y),
      w: r1(b.w),
      h: r1(b.h),
      name: fit(s.name, room, measure, 'name'),
      full: s.name,
      cmd: s.cmd ? fit(s.cmd, Math.max(foot - b.x - 16, 24), measure, 'cmd') : '',
      state,
      guessed: !!s.guessed,
      self: loop
        ? {
            x: r1(b.x + b.w - 50),
            y: r1(b.y + b.h - (lr ? 26 : 22)),
            max: loop.max,
            cond: progress.reasons[`${s.id}>${s.id}`] ?? loop.cond,
            took: took.has(`${s.id}>${s.id}`),
            turns: progress.rounds[s.id] ?? 0,
          }
        : null,
      dots,
      more: on.length > 3 ? `+${on.length - 3}` : '',
      moreX: r1(moreX),
      tag,
      beads,
      beadsMore,
      fork: forkFrom.has(s.id)
        ? lr
          ? `M${r1(b.x + b.w)} ${r1(b.y + b.h / 2 - 5)} l5 5 l-5 5 l-5 -5 z`
          : `M${r1(b.x + b.w / 2 - 5)} ${r1(b.y + b.h)} l5 -5 l5 5 l-5 5 z`
        : null,
      who,
    };
  });
  return { width: r1(W), height: r1(H), edges: [...edges, ...heads], nodes, doors, chips, exit };
}
