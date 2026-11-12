// The agents' sidebar of the 2.0 design (lot 3): the order of its cards, what each card says the agent does, and the
// question an agent waits on, answered from the list without leaving the conversation open (docs/TODO-2.0.md,
// « Nouveautés »).

import { capsOf, showsCost } from './engines';
import { fInt, fTok } from './format';
import { t } from './i18n';
import { fSpentUsd, spent } from './spend';
import type { Pending } from './later';
import { SUMMED_UP } from './tools';
import type { Agent, AgentFolder, Project } from './types';

/** What the sidebar lists: a loose agent (`lifted`: it waits, so it went to the top) or a folder. */
export type SidebarItem =
  | { kind: 'agent'; agent: Agent; lifted: boolean }
  | {
      kind: 'folder';
      folder: AgentFolder;
      principal: Agent | null;
      /** Without the principal, in shown order: the waiting ones first. */
      agents: Agent[];
      running: number;
      waiting: number;
    };

/**
 * The sidebar's list of `project` (`agents`: its agents that are not archived), the shown order of the core's
 * `folders::shown`: unknown ids and doubles dropped, what is placed nowhere at the bottom oldest first, the loose agents
 * that wait first, a folder's principal on top of its own, then its waiting agents.
 */
export function sidebarItems(project: Project, agents: Agent[]): SidebarItem[] {
  const byId = new Map(agents.map((a) => [a.id, a]));
  const folders = project.agentFolders ?? [];
  const seen = new Set<string>();
  const members = new Map<string, Agent[]>();
  const principals = new Map<string, Agent | null>();
  for (const f of folders) {
    const mine = f.agents.filter((id) => byId.has(id) && !seen.has(id) && seen.add(id));
    members.set(
      f.id,
      mine.map((id) => byId.get(id)!),
    );
    principals.set(f.id, (f.principal && mine.includes(f.principal) && byId.get(f.principal)) || null);
  }
  const folderIds = new Set(folders.map((f) => f.id));
  const order = (project.agentOrder ?? []).filter((i) => (folderIds.has(i) || byId.has(i)) && !seen.has(i) && seen.add(i));
  for (const f of folders) if (!seen.has(f.id)) order.push(f.id) && seen.add(f.id);
  const unlisted = agents.filter((a) => !seen.has(a.id)).sort((a, b) => a.createdAt - b.createdAt);
  order.push(...unlisted.map((a) => a.id));

  const waits = (i: string) => !folderIds.has(i) && byId.get(i)?.status === 'waiting';
  const out: SidebarItem[] = order.filter(waits).map((i) => ({ kind: 'agent', agent: byId.get(i)!, lifted: true }));
  for (const i of order.filter((i) => !waits(i))) {
    const folder = folders.find((f) => f.id === i);
    if (!folder) {
      out.push({ kind: 'agent', agent: byId.get(i)!, lifted: false });
      continue;
    }
    const principal = principals.get(i) ?? null;
    const mine = members.get(i)!;
    const rest = mine.filter((a) => a !== principal);
    const agents = [...rest.filter((a) => a.status === 'waiting'), ...rest.filter((a) => a.status !== 'waiting')];
    out.push({
      kind: 'folder',
      folder,
      principal,
      agents,
      running: mine.filter((a) => a.status === 'running').length,
      waiting: mine.filter((a) => a.status === 'waiting').length,
    });
  }
  return out;
}

/** The agents of `items`, flat, in shown order (a folder's principal first). */
export function shownAgents(items: SidebarItem[]): Agent[] {
  return items.flatMap((it) => (it.kind === 'agent' ? [it.agent] : [...(it.principal ? [it.principal] : []), ...it.agents]));
}

/** What a folded folder still shows as cards: its agents that wait, and the open one. */
export function foldedCards(item: Extract<SidebarItem, { kind: 'folder' }>, selectedId: string | undefined): Agent[] {
  return [...(item.principal ? [item.principal] : []), ...item.agents].filter((a) => a.status === 'waiting' || a.id === selectedId);
}

/**
 * What the amber pill of a folder's header counts: its agents, its principal included, that wait for an answer and were not
 * seen yet (`attention`, what makes a card blink). Opened, an agent leaves the count, as its card stops blinking; it still
 * waits on top of its folder, and a folded folder still shows it (`foldedCards`).
 */
export function folderAsks(item: Extract<SidebarItem, { kind: 'folder' }>, attention: Record<string, true>): number {
  return [...(item.principal ? [item.principal] : []), ...item.agents].filter((a) => a.status === 'waiting' && attention[a.id]).length;
}

/** The line of a card under its model: what the agent does during a turn, or what its last turns changed. */
export type CardActivity =
  { kind: 'doing'; verb: string; what: string } | { kind: 'thinking' } | { kind: 'result'; files: number } | { kind: 'over'; goes: string };

/**
 * What the card of `a` says it does: during a turn its activity (« Lit src/db.ts », a verb then what it acts on), or that
 * it thinks; after it, the files it changed (`files`, from git), none when it changed nothing. An agent waiting for an
 * answer says its question instead. Its turn over but some of its work going on (`pending`: « Attend »), it says what goes
 * on (« 1 sous-agent, 1 tâche de fond »); a wake-up alone is said by its status.
 */
export function cardActivity(
  a: Agent,
  files: number,
  pending?: Pick<Pending, 'subagents' | 'background' | 'driven'> | null,
): CardActivity | null {
  if (a.status === 'waiting') return null;
  if (pending && (pending.subagents || pending.background || pending.driven.length)) {
    const goes = [
      pending.subagents && t('nav.sidebar.goes.subagents', { count: pending.subagents }),
      pending.background && t('nav.sidebar.goes.background', { count: pending.background }),
      pending.driven.length && t('nav.sidebar.goes.driven', { count: pending.driven.length }),
    ].filter(Boolean);
    return { kind: 'over', goes: goes.join(', ') };
  }
  if (a.status === 'running') {
    if (!a.activity) return { kind: 'thinking' };
    const at = a.activity.indexOf(' ');
    return at < 0
      ? { kind: 'doing', verb: a.activity, what: '' }
      : { kind: 'doing', verb: a.activity.slice(0, at), what: a.activity.slice(at + 1) };
  }
  return files > 0 ? { kind: 'result', files } : null;
}

/** The most characters, and lines, of what a permission acts on that the card shows whole. */
export const LIST_ARG_MAX = 160;
export const LIST_ARG_LINES = 3;
/** The most characters of what the card says besides (why the harness asks, what « always » keeps, what a form is for). */
export const LIST_NOTE_MAX = 240;

/** Questions to answer from the list, one after the other: each with its choices. `note`: what a form is for. */
export interface ListQuestions {
  kind: 'question';
  requestId: string;
  questions: { question: string; options: string[] }[];
  note: string | null;
}

/**
 * A permission to answer from the list: the tool, what it acts on, what the harness says besides (`note`), and what
 * « always » stands for, if offered: `always` (rules kept, OpenCode's pattern told in the note), `session` or `agent` (Cursor CLI).
 */
export interface ListPermission {
  kind: 'permission';
  requestId: string;
  tool: string;
  arg: string;
  note: string | null;
  always: 'always' | 'session' | 'agent' | null;
}

export type ListRequest = ListQuestions | ListPermission;

/** What a permission's choice does: the decision sent. */
export type ListDecision = 'allow' | 'always' | 'session' | 'agent' | 'deny';

/**
 * The request `a` waits on, when it can be answered from its card: only what is read whole there (« Vue d’ensemble »'s
 * rule for permissions). Questions with choices, one or several, one choice each; a permission of a tool whose summary
 * says what it asks, short enough, that the harness would not refuse by default. A question to write, of several
 * choices ticked, a plan, anything cut short or too long: answered in the conversation, which a click on the card opens.
 */
export function listRequest(a: Agent): ListRequest | null {
  const req = a.status === 'waiting' ? a.requests?.[0] : undefined;
  if (!req || req.cut) return null;
  const note = [req.description, req.reason].filter((x): x is string => !!x?.trim()).join('\n') || null;
  if ((note?.length ?? 0) > LIST_NOTE_MAX) return null;
  if (req.kind === 'question') {
    if (!req.questions.length || req.questions.some((q) => !q.options.length || q.multi)) return null;
    return { kind: 'question', requestId: req.id, questions: req.questions.map(({ question, options }) => ({ question, options })), note };
  }
  const arg = req.arg;
  if (req.defaultNo || !SUMMED_UP.has(req.tool) || !arg.trim()) return null;
  if (arg.length > LIST_ARG_MAX || arg.split('\n').length > LIST_ARG_LINES) return null;
  const always = req.canAlways ? (req.alwaysScope === 'session' || req.alwaysScope === 'agent' ? req.alwaysScope : 'always') : null;
  return { kind: 'permission', requestId: req.id, tool: req.tool, arg, note, always };
}

/** The question of `r` to answer now, by the choices already picked (`picks`, by question): -1 once all have one. */
export function listStep(r: ListQuestions, picks: Record<string, string[]>): number {
  return r.questions.findIndex((q) => !picks[q.question]?.length);
}

/** The choices the card shows, numbered 1, 2…: the options of the question to answer, or a permission's decisions. */
export function listChoices(r: ListQuestions, picks: Record<string, string[]>): string[];
export function listChoices(r: ListRequest, picks: Record<string, string[]>): string[] | ListDecision[];
export function listChoices(r: ListRequest, picks: Record<string, string[]>): string[] | ListDecision[] {
  if (r.kind === 'permission') return r.always ? ['allow', r.always, 'deny'] : ['allow', 'deny'];
  return r.questions[listStep(r, picks)]?.options ?? [];
}

/**
 * The choice a key picks on the list: 1, 2… (0, 1… here), by the digit typed or, without Shift, by the key of the digit
 * row (AZERTY types « & é " » there). Never with a modifier (Ctrl+1 goes to a project, Alt+1 answers in the
 * conversation), nor a key held down.
 */
export function listDigit(e: KeyboardEvent): number | null {
  if (e.ctrlKey || e.altKey || e.metaKey || e.repeat) return null;
  const d = /^[1-9]$/.exec(e.key)?.[0] ?? (!e.shiftKey ? /^Digit([1-9])$/.exec(e.code)?.[1] : undefined);
  return d ? Number(d) - 1 : null;
}

/**
 * The agent the keys of the list answer: the one whose card has the focus, else the first one that can be answered there.
 * A card with the focus that waits for what is answered in its conversation takes them: no other agent is answered.
 */
export function listTarget(order: Agent[], focused: string | null): Agent | null {
  const mine = order.find((a) => a.id === focused);
  if (mine?.status === 'waiting') return listRequest(mine) ? mine : null;
  return order.find((a) => listRequest(a)) ?? null;
}

/**
 * What `a` has spent, on its card: its tokens and its cost (« ≈ » when estimated), or its credits and their cost for a
 * harness whose limit is the month's credits (Copilot CLI, a credit costing $0.01): what it counts in. Tokens alone for a
 * harness that tells no cost.
 */
export function cardSpend(a: Agent): string {
  const s = spent(a);
  if (!showsCost(a)) return fTok(s.tokens);
  const credits = Math.round(s.cost * 100);
  const used = capsOf(a.engine)?.quota === 'credits' ? t('nav.sidebar.credits', { count: credits, n: fInt(credits) }) : fTok(s.tokens);
  return `${used} · ${fSpentUsd(s)}`;
}
