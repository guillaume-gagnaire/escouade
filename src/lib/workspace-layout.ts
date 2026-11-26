// What the « Workspace » shows of each agent, and where (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md,
// « Les altitudes »). Pure: the view draws what it gives.

import { bgRows, isRunning, ownerOf, type BgRow } from './bgtasks';
import { drivenAgents } from './later';
import { planView } from './plan';
import { sidebarSubs, subagentInfo } from './subagents';
import type { Agent, ConvItem, KitUse, SubAgent, Ticket } from './types';
import type { Box, Size } from './workspace-camera';
import { landed, type Landing } from './workspace-landing';
import { testsOf, type TestRun } from './workspace-tests';

export type Placed<T> = T & { x: number; y: number; w: number; h: number };

/**
 * A skill or a plugin of an agent, its subagents' calls added; `runningIn`: the subagent's call a call of it runs in;
 * `firstAt`, `lastAt`: its first and its latest call, of all its threads.
 */
export type ToolCard = {
  kind: 'skill' | 'plugin';
  name: string;
  calls: number;
  running: boolean;
  runningIn: string | null;
  firstAt: number;
  lastAt: number;
};

/**
 * The name a tool is shown by. Claude Code names the server of a plugin `plugin_<plugin>_<server>`: its server, or
 * « <plugin> · <server> » when they differ (a plugin's name has no `_`). Any other name is shown whole.
 */
export function toolName(t: Pick<ToolCard, 'kind' | 'name'>): string {
  if (t.kind !== 'plugin' || !t.name.startsWith('plugin_')) return t.name;
  const rest = t.name.slice('plugin_'.length);
  const i = rest.indexOf('_');
  if (i <= 0 || i === rest.length - 1) return rest;
  const [plugin, server] = [rest.slice(0, i), rest.slice(i + 1)];
  return plugin === server ? server : `${plugin} · ${server}`;
}

/**
 * The tools of `agent` and of its subagents, one card a tool, in the order of their first use (a new one comes last:
 * it never moves the others; the agent's thread before its subagents' at the same time).
 */
export function toolCards(agent: Agent): ToolCard[] {
  const canvas = agent.canvas;
  if (!canvas) return [];
  const cards = new Map<string, ToolCard>();
  const threads: [string | null, KitUse[]][] = [[null, canvas.tools], ...canvas.subs.map((s): [string, KitUse[]] => [s.call, s.tools])];
  for (const [call, tools] of threads)
    for (const k of tools) {
      const key = `${k.kind}:${k.name}`;
      const first = k.firstAt ?? 0;
      const card = cards.get(key) ?? { kind: k.kind, name: k.name, calls: 0, running: false, runningIn: null, firstAt: first, lastAt: 0 };
      card.calls += k.calls;
      card.firstAt = Math.min(card.firstAt, first);
      card.lastAt = Math.max(card.lastAt, k.lastAt);
      if (k.running > 0 && !card.running) [card.running, card.runningIn] = [true, call];
      cards.set(key, card);
    }
  // A stable sort: same time, the order of the threads.
  return [...cards.values()].sort((a, b) => a.firstAt - b.firstAt);
}

// Near: the compass. Each agent is a frame, the agent's card in it and what belongs to it around (spec, « La boussole,
// côté par côté »). The sizes are left to who builds (taken from the mock-ups, to tune with use).

export type Side = 'north' | 'west' | 'centre' | 'east' | 'south';

export const CARD = { w: 360, h: 128 } as const;
export const SUB = { w: 240, h: 100 } as const;
export const BG = { w: 240, h: 60 } as const;
/**
 * The conversation's plan banner (`PlanBanner`), north of the card, as measured in the window: its head and gauge
 * (`head`, 10 px less folded: no gap before a list), a row of 30 px a task in a list padded by 6 px, 232 px at most and
 * scrolling past it (its CSS).
 */
export const PLAN = { w: 560, head: 72, folded: 62, row: 30, pad: 6, list: 232 } as const;
export const GAP = 48;
/** A test card, stuck under the card that runs it: its head, one line of command and its button « Sortie », its height fixed. */
export const TEST = { h: 92, gap: 8 } as const;
/** The landing zone, empty: its height until it is measured. */
export const SOUTH = { h: 132 } as const;
/**
 * A tool's card (« de la doc, pas un agent »): above the agent's card, two to a row around an aisle in the middle of the
 * card, where the plan's link runs; `row`: between two rows; `drop`: from the last row to the card, for a flow to show.
 */
export const TOOL = { w: 168, h: 60, aisle: 24, row: 12, drop: 32 } as const;
/** The tool cards shown, the most recently called; the others in a pile (« + n outils ») that unfolds them. */
export const TOOLS_SHOWN = 8;
/** The pile of the tools: `hidden` of them folded in it, or `open` (it folds them back). */
export type ToolPile = { hidden: number; open: boolean };
/**
 * Mid-height (below 60 %, down to 30 %): a side folded in one pile (« Outils ×4 », « Équipe ×4 », « Livrables ×3 ») that a click
 * unfolds; `running` when one of its items runs. The plan is folded to its head and its gauge (`PLAN.folded`).
 */
export type Level = 'mid' | 'near';
export const PILE = { w: 184, h: 64 } as const;
/** The west and south piles on the card's axis: the plan's line goes through their middle, not along an edge. */
const PILE_X = (CARD.w - PILE.w) / 2;
export type PileSide = 'west' | 'east' | 'south';
export type Pile = { side: PileSide; count: number; running: boolean };
/** The pile of the ended subagents, and the gap between two cards of the team. */
const FOLDED_H = 40;
const STACK_GAP = 16;
/** The plan keeps 30 subagents at most: at that many, more may have ended. */
const PLAN_SUBS_MAX = 30;

export type TeamItem =
  | { kind: 'sub'; id: string; sub: SubAgent }
  | { kind: 'bg'; id: string; row: BgRow }
  | { kind: 'folded'; id: 'folded'; done: number; failed: number; capped: boolean };

/** `run` alone moves: a moving link always means « it runs right now ». */
export type LinkState = 'run' | 'done' | 'failed' | 'plan' | 'launched';
/** `d`: the SVG path, in the world's coordinates. */
export type Link = { id: string; state: LinkState; d: string };
/** A link of a frame, from (x1, y1) to (x2, y2) in the frame's coordinates: a curve, or a straight line. */
export type Edge = {
  id: string;
  state: LinkState;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  straight?: boolean;
  /** Straight segments through these points: the way around the cards. */
  via?: [number, number][];
};

/** `west`: the tools, drawn above the card (the user's choice); the name is the side's, for the keyboard's order. */
export type Frame = {
  agentId: string;
  w: number;
  h: number;
  centre: Box;
  north: Box | null;
  west: Placed<ToolCard>[];
  /** The pile of the tools past `TOOLS_SHOWN`, after them; none with fewer. */
  westMore: Placed<ToolPile> | null;
  team: Placed<TeamItem>[];
  /** The latest test run of each card, stuck under it (`under`: 'centre' or the subagent's id). */
  tests: Placed<TestRun>[];
  /** The landing zone: below all the rest, as wide as the frame; always there near, none at mid-height when folded. */
  south: Box | null;
  /** At mid-height, its sides folded, in the order west, east, south; an empty side has none. */
  piles: Placed<Pile>[];
  /** At mid-height, its plan folded to its head and its gauge. */
  northMini: boolean;
  links: Edge[];
};
/**
 * `teamOpen`: its pile of ended subagents unfolded; `seenAt`: when it was unfolded last; `centreH`: its card's height;
 * `planOpen`: its plan's banner unfolded, as in its conversation (default); `teamH`: the height measured of a card of
 * its team, by its id (none or 0: its size by default); `toolsOpen`: its pile of tools unfolded.
 */
export type FrameOpts = {
  teamOpen?: boolean;
  seenAt?: number;
  centreH?: number;
  planOpen?: boolean;
  teamH?: Record<string, number>;
  toolsOpen?: boolean;
  /** The board's test command (`testCommand`): a run of it is a test run. */
  testCommand?: string;
  /** The landing zone's height measured. */
  southH?: number;
  /** The plan's banner's height measured, open (the reader may have sized its list): its size by default without. */
  northH?: number;
  /** Its altitude: at `mid`, the sides not in `unfolded` are piles (never those of an agent that waits). */
  level?: Level;
  unfolded?: Side[];
  /** What lands in its zone: at mid-height, what its pile counts. */
  landing?: Landing[];
};

/** The highest a card grows (an agent that waits, its questions in it), at a zoom of 1: past it, its inset scrolls. */
export const cardMaxH = (windowH: number): number => Math.round(0.7 * windowH);

/**
 * The highest the inset of a card of `cardH` grows, in the world's pixels: the card held at 70 % of the window on screen
 * at the camera's `zoom` (never less than a few lines). It reads the zoom alone, never the layout: no loop.
 */
export const askMaxH = (windowH: number, zoom: number, cardH: number): number => Math.max(80, Math.round(cardMaxH(windowH) / zoom) - cardH);

/** It waits for the user: a question or a permission. Such an agent is unfolded at every altitude. */
export const waits = (agent: Agent): boolean => agent.status === 'waiting' || agent.pending.length > 0;

/** A subagent's name on the canvas: its type, else its title (its plan's row, else its call in the conversation). */
export function subName(agent: Agent, items: ConvItem[], call: string): string {
  const info = subagentInfo(agent, items, call);
  return info?.kind ?? info?.title ?? call;
}

/**
 * The east of `agent`: its subagents running, its background tasks running, then the ended subagents folded in one pile
 * (unless unfolded). A failure since the pile was last unfolded stays out of it, just before it: the app keeps no « seen »
 * by subagent.
 */
export function teamOf(agent: Agent, opts: FrameOpts): TeamItem[] {
  const plan = agent.plan;
  const subs = plan?.agents ?? [];
  const item = (sub: SubAgent): TeamItem => ({ kind: 'sub', id: sub.id, sub });
  const out: TeamItem[] = subs.filter(isRunning).map(item);
  // The test run in the background a card shows as its test card, under it; another one running stays in the team.
  const carded = new Set(testsOf(agent, opts.testCommand ?? '').flatMap((r) => (r.opens?.kind === 'task' ? [r.opens.task] : [])));
  out.push(
    ...bgRows(plan)
      .filter((row) => isRunning(row) && !carded.has(row.id))
      .map((row): TeamItem => ({ kind: 'bg', id: row.id, row })),
  );
  // In the sidebar's order, the latest ended first.
  const ended: SubAgent[] = sidebarSubs(agent)?.finished ?? [];
  if (opts.teamOpen) return [...out, ...ended.map(item)];
  const seenAt = opts.seenAt ?? 0;
  const unseen = (s: SubAgent) => s.status === 'failed' && (s.endedAt ?? s.startedAt) > seenAt;
  out.push(...ended.filter(unseen).map(item));
  const pile = ended.filter((s) => !unseen(s));
  if (pile.length) {
    const failed = pile.filter((s) => s.status === 'failed').length;
    out.push({ kind: 'folded', id: 'folded', done: pile.length - failed, failed, capped: subs.length >= PLAN_SUBS_MAX });
  }
  return out;
}

const defaultSize = (i: TeamItem): Size => (i.kind === 'sub' ? SUB : i.kind === 'bg' ? BG : { w: SUB.w, h: FOLDED_H });

/** A curve from (x1, y1), leaving to the right, to (x2, y2), arriving from the left. */
function curve(x1: number, y1: number, x2: number, y2: number): string {
  const mx = (x1 + x2) / 2;
  return `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`;
}

/** The link of an edge of a frame drawn at (dx, dy) of the world. */
function linkOf(e: Edge, dx: number, dy: number): Link {
  const [x1, y1, x2, y2] = [e.x1 + dx, e.y1 + dy, e.x2 + dx, e.y2 + dy];
  if (e.via) {
    const pts = e.via.map(([x, y]) => `L ${x + dx} ${y + dy}`).join(' ');
    return { id: e.id, state: e.state, d: `M ${x1} ${y1} ${pts} L ${x2} ${y2}` };
  }
  return { id: e.id, state: e.state, d: e.straight ? `M ${x1} ${y1} L ${x2} ${y2}` : curve(x1, y1, x2, y2) };
}

/**
 * The frames computed, by agent object and altitude: one replaced (every update of the core replaces it) is computed
 * again; crossing back to an altitude finds its frame.
 */
const frames = new WeakMap<Agent, Map<string, { key: string; frame: Frame }>>();

/**
 * The compass of `agent`, in its own coordinates: the plan's band on top (with tasks), its card below, its team in a
 * column on the right. A subagent whose parent is in the team, and a background task whose subagent is, lie right of it.
 */
export function frameOf(agent: Agent, opts: FrameOpts = {}): Frame {
  const key = `${!!opts.teamOpen}|${opts.seenAt ?? 0}|${opts.centreH ?? ''}|${opts.planOpen !== false}|${JSON.stringify(opts.teamH ?? {})}|${!!opts.toolsOpen}|${opts.testCommand ?? ''}|${opts.southH ?? ''}|${opts.northH ?? ''}|${opts.level ?? ''}|${opts.unfolded ?? ''}|${opts.landing?.length ?? 0}:${opts.landing?.filter((l) => !landed(l)).length ?? 0}`;
  let byLevel = frames.get(agent);
  if (!byLevel) frames.set(agent, (byLevel = new Map()));
  const known = byLevel.get(opts.level ?? '');
  if (known?.key === key) return known.frame;
  const frame = computeFrame(agent, opts);
  byLevel.set(opts.level ?? '', { key, frame });
  return frame;
}

function computeFrame(agent: Agent, opts: FrameOpts): Frame {
  const mid = opts.level === 'mid' && !waits(agent);
  const folded = (side: Side) => mid && !opts.unfolded?.includes(side);
  const piles: Placed<Pile>[] = [];
  const pileAt = (side: PileSide, count: number, running: boolean, x: number, y: number) => {
    const pile = { side, count, running, x, y, ...PILE };
    if (count) piles.push(pile);
    return count ? pile : null;
  };
  const tasks = planView(agent)?.rows ?? [];
  let north: Box | null = null;
  const northMini = !!tasks.length && folded('north');
  if (tasks.length) {
    const h =
      opts.planOpen === false || northMini
        ? PLAN.folded
        : opts.northH || PLAN.head + Math.min(tasks.length * PLAN.row + PLAN.pad, PLAN.list);
    north = { x: 0, y: 0, w: PLAN.w, h };
  }
  const top = north ? north.h + GAP : 0;
  const tools = toolCards(agent);
  const westPile = folded('west')
    ? pileAt(
        'west',
        tools.length,
        tools.some((t) => t.running),
        PILE_X,
        top,
      )
    : null;
  const all = folded('west') ? [] : tools;
  let shown = all;
  let pile: ToolPile | null = null;
  if (all.length > TOOLS_SHOWN) {
    if (opts.toolsOpen) pile = { hidden: 0, open: true };
    else {
      // The ones running first: a call that runs keeps its card, and its flow.
      const recent = new Set([...all].sort((a, b) => Number(b.running) - Number(a.running) || b.lastAt - a.lastAt).slice(0, TOOLS_SHOWN));
      shown = all.filter((t) => recent.has(t));
      pile = { hidden: all.length - TOOLS_SHOWN, open: false };
    }
  }
  const slot = (i: number) => ({
    x: (i % 2) * (TOOL.w + TOOL.aisle),
    y: top + Math.floor(i / 2) * (TOOL.h + TOOL.row),
    w: TOOL.w,
    h: TOOL.h,
  });
  const west: Placed<ToolCard>[] = shown.map((t, i) => ({ ...t, ...slot(i) }));
  const westMore = pile && { ...pile, ...slot(west.length) };
  const rows = Math.ceil((west.length + (westMore ? 1 : 0)) / 2);
  const centreY = westPile ? top + PILE.h + TOOL.drop : rows ? top + rows * (TOOL.h + TOOL.row) - TOOL.row + TOOL.drop : top;
  const centre: Box = { x: 0, y: centreY, w: CARD.w, h: opts.centreH ?? CARD.h };
  const links: Edge[] = [];
  if (north) {
    const x = CARD.w / 2;
    links.push({ id: `${agent.id}:plan`, state: 'plan', x1: x, y1: centre.y, x2: x, y2: north.h, straight: true });
  }
  const pileLink = (p: Placed<Pile>, e: Omit<Edge, 'id' | 'state'>) =>
    links.push({ id: `${agent.id}:pile:${p.side}`, state: p.running ? 'run' : 'plan', ...e });
  if (westPile) {
    const x = CARD.w / 2;
    pileLink(westPile, { x1: x, y1: westPile.y + PILE.h, x2: x, y2: centre.y, straight: true });
  }

  let items = teamOf(agent, opts);
  if (folded('east')) {
    // Its subagents and its background tasks, the ended ones too.
    const all = teamOf(agent, { ...opts, teamOpen: true }).filter((i) => i.kind !== 'folded');
    const running = all.some((i) => i.kind === 'bg' || (i.kind === 'sub' && i.sub.status === 'running'));
    const pile = pileAt('east', all.length, running, CARD.w + 2 * GAP, centre.y);
    if (pile) pileLink(pile, { x1: centre.x + centre.w, y1: centre.y + centre.h / 2, x2: pile.x, y2: pile.y + PILE.h / 2 });
    items = [];
  }
  const ids = new Set(items.map((i) => i.id));
  const parentOf = (i: TeamItem): string | undefined => {
    const p = i.kind === 'sub' ? i.sub.parent : i.kind === 'bg' ? ownerOf(agent.plan, i.row)?.id : undefined;
    return p && p !== i.id && ids.has(p) ? p : undefined;
  };
  const placed = new Map<string, Placed<TeamItem>>();
  /** The right end of each row the column started, by the item that started it. */
  const rowEnd = new Map<string, { root: string; x: number }>();
  const colX = CARD.w + 2 * GAP;
  let colY = centre.y;
  const sizeOf = (i: TeamItem): Size => {
    const s = defaultSize(i);
    return opts.teamH?.[i.id] ? { w: s.w, h: opts.teamH[i.id] } : s;
  };
  const runs = new Map(testsOf(agent, opts.testCommand ?? '').map((r) => [r.under, r]));
  /** How far a card's test card reaches below it. */
  const below = (id: string) => (runs.has(id) ? TEST.gap + TEST.h : 0);
  const place = (i: TeamItem, x: number, y: number) => placed.set(i.id, { ...i, ...sizeOf(i), x, y });
  // Down the column, a row at a time: a root, then each child right of its parent, as long as one is placed; the next
  // root under the lowest card of the row, or its test card.
  for (const root of items) {
    if (parentOf(root)) continue;
    place(root, colX, colY);
    rowEnd.set(root.id, { root: root.id, x: colX + sizeOf(root).w });
    let bottom = colY + sizeOf(root).h + below(root.id);
    for (let more = true; more;) {
      more = false;
      for (const i of items) {
        const p = parentOf(i);
        if (placed.has(i.id) || !p || !placed.has(p)) continue;
        const row = rowEnd.get(p)!;
        const end = rowEnd.get(row.root)!;
        const y = placed.get(p)!.y;
        place(i, end.x + GAP, y);
        end.x += GAP + sizeOf(i).w;
        rowEnd.set(i.id, row);
        bottom = Math.max(bottom, y + sizeOf(i).h + below(i.id));
        more = true;
      }
    }
    colY = bottom + STACK_GAP;
  }
  const team = items.map((i) => placed.get(i.id)!);
  // No test card for a subagent that has no card (folded in the pile, forgotten by the plan).
  const tests: Placed<TestRun>[] = [];
  for (const [under, r] of runs) {
    const sub = placed.get(under);
    const card = under === 'centre' ? centre : sub?.kind === 'sub' ? sub : undefined;
    if (!card) continue;
    tests.push({ ...r, x: card.x, y: card.y + card.h + TEST.gap, w: card.w, h: TEST.h });
  }

  for (const i of team) {
    const p = parentOf(i);
    const from = p ? placed.get(p)! : centre;
    const state: LinkState =
      i.kind === 'bg'
        ? 'run'
        : i.kind === 'folded'
          ? 'done'
          : i.sub.status === 'running'
            ? 'run'
            : i.sub.status === 'failed'
              ? 'failed'
              : 'done';
    links.push({ id: `${agent.id}:team:${i.id}`, state, x1: from.x + from.w, y1: from.y + from.h / 2, x2: i.x, y2: i.y + i.h / 2 });
  }

  // A flow from a tool only while one of its calls runs, under no card: from the tool's side on the aisle, down the aisle
  // (off its middle, the plan's link), then to the agent's card, or along the gap above the card and down right of it to
  // the subagent it runs in when that one has a card.
  const aisle = TOOL.w + TOOL.aisle / 2;
  const gapY = centre.y - TOOL.drop / 2;
  for (const t of west) {
    if (!t.running) continue;
    const id = `${agent.id}:tool:${t.kind}:${t.name}`;
    const left = t.x === 0;
    const [x1, y1] = [left ? t.x + t.w : t.x, t.y + t.h / 2];
    const lane = aisle + (left ? -6 : 6);
    const to = t.runningIn ? placed.get(t.runningIn) : undefined;
    if (to) {
      const x = CARD.w + GAP;
      const y2 = to.y + to.h / 2;
      links.push({
        id,
        state: 'run',
        x1,
        y1,
        x2: to.x,
        y2,
        via: [
          [lane, y1],
          [lane, gapY],
          [x, gapY],
          [x, y2],
        ],
      });
    } else links.push({ id, state: 'run', x1, y1, x2: lane, y2: centre.y, via: [[lane, y1]] });
  }

  const boxes: Box[] = [centre, ...(north ? [north] : []), ...west, ...(westMore ? [westMore] : []), ...team, ...tests, ...piles];
  let w = Math.max(...boxes.map((b) => b.x + b.w));
  const southY = Math.max(...boxes.map((b) => b.y + b.h)) + GAP;
  // One line down to it, from the agent's card or its test card.
  const own = tests.find((r) => r.under === 'centre') ?? centre;
  const x = CARD.w / 2;
  let south: Box | null = null;
  let h = southY - GAP;
  if (folded('south')) {
    const landing = opts.landing ?? [];
    const pile = pileAt(
      'south',
      landing.length,
      landing.some((l) => !landed(l)),
      PILE_X,
      southY,
    );
    if (pile) {
      pileLink(pile, { x1: x, y1: own.y + own.h, x2: x, y2: pile.y, straight: true });
      w = Math.max(w, PILE_X + PILE.w);
      h = pile.y + PILE.h;
    }
  } else {
    south = { x: 0, y: southY, w, h: opts.southH || SOUTH.h };
    links.push({ id: `${agent.id}:south`, state: 'plan', x1: x, y1: own.y + own.h, x2: x, y2: south.y, straight: true });
    h = south.y + south.h;
  }
  return { agentId: agent.id, w, h, centre, north, west, westMore, team, tests, south, piles, northMini, links };
}

/** The agents another of them launched (`drivenAgents`), each linked from the first that did, in the order given. */
export function launchedOf(agents: Agent[], all: Record<string, Agent>, tickets: Record<string, Ticket>): [from: string, to: string][] {
  const shown = new Set(agents.map((a) => a.id));
  const linked = new Set<string>();
  const out: [string, string][] = [];
  for (const a of agents)
    for (const d of drivenAgents(a.plan, all, tickets, a.id)) {
      if (d.id === a.id || !shown.has(d.id) || linked.has(d.id)) continue;
      linked.add(d.id);
      out.push([a.id, d.id]);
    }
  return out;
}

/** A column of the canvas of every project: a head with the project's name, then its agents. */
export const COLUMN = { gap: 160, head: 56 } as const;

/**
 * The frames one under the other from (x0, y0), in order; their links in the world's coordinates, and a dotted link
 * from an agent to each it launched, down the left of the cards.
 */
export function layoutNear(
  frames: Frame[],
  launched: [string, string][] = [],
  x0 = 0,
  y0 = 0,
): { items: Placed<Frame>[]; links: Link[]; w: number; h: number } {
  let y = y0;
  const items = frames.map((f) => {
    const placed = { ...f, x: x0, y };
    y += f.h + GAP;
    return placed;
  });
  const links: Link[] = items.flatMap((f) => f.links.map((e) => linkOf(e, f.x, f.y)));
  const byId = new Map(items.map((f) => [f.agentId, f]));
  for (const [from, to] of launched) {
    const a = byId.get(from);
    const b = byId.get(to);
    if (!a || !b) continue;
    const mid = (f: Placed<Frame>) => f.y + f.centre.y + f.centre.h / 2;
    const [ya, yb, lane] = [mid(a), mid(b), x0 - GAP / 2];
    const via: [number, number][] = [
      [lane, ya],
      [lane, yb],
    ];
    links.push(linkOf({ id: `launched:${from}:${to}`, state: 'launched', x1: x0, y1: ya, x2: x0, y2: yb, via }, 0, 0));
  }
  const w = Math.max(0, ...frames.map((f) => f.w));
  return { items, links, w, h: items.length ? y - GAP - y0 : 0 };
}

/** The projects side by side, in order, each a column of its agents (under a head with `heads`). */
export function layoutColumns(
  columns: { projectId: string; frames: Frame[]; launched: [string, string][] }[],
  heads: boolean,
): { items: Placed<Frame>[]; links: Link[]; heads: Placed<{ projectId: string }>[]; w: number; h: number } {
  const out = { items: [] as Placed<Frame>[], links: [] as Link[], heads: [] as Placed<{ projectId: string }>[], w: 0, h: 0 };
  const top = heads ? COLUMN.head : 0;
  let x = 0;
  for (const c of columns) {
    const col = layoutNear(c.frames, c.launched, x, top);
    if (heads) out.heads.push({ projectId: c.projectId, x, y: 0, w: col.w, h: COLUMN.head });
    out.items.push(...col.items);
    out.links.push(...col.links);
    out.w = x + col.w;
    out.h = Math.max(out.h, top + col.h);
    x += col.w + COLUMN.gap;
  }
  return out;
}
