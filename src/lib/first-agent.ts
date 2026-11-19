// The station of the first agent (ESC-505, docs/ONBOARDING.md § 4.5): where its first turn stands, read from the agent
// and its conversation as they are (nothing kept apart), and the messages proposed to start it. Nothing here sends
// anything: the message goes by the user's click on the composer's own button.

import type { Agent, ConvItem, TurnItem } from './types';

/**
 * Where the first turn stands: `compose` (nothing sent yet), `preparing` (sent, waiting for the agent's worktree to be
 * made), `running`, `permission` and `asking` (a request waits for
 * the user), `done` (a turn ended well), or why it stopped: `limit` (the account's usage limit), `offline` (the harness
 * could not reach its service), `signedOut` (the account is not signed in), `failed` (anything else).
 */
export type FirstTurn =
  'compose' | 'preparing' | 'running' | 'permission' | 'asking' | 'done' | 'limit' | 'offline' | 'signedOut' | 'failed';

export interface FirstTurnView {
  state: FirstTurn;
  /** What the harness said of a turn that failed. */
  error: string | null;
  /** When the usage limit resets (ms), if known. */
  until: number | null;
  /** The last message sent, to put back in the composer after a failure (« Le message reste prêt »). */
  sent: string | null;
}

/** The words of a harness that could not reach its service: no network, no name resolution, a connection refused. */
const OFFLINE =
  /connection error|unable to connect|could not connect|failed to fetch|fetch failed|getaddrinfo|enotfound|econnrefused|econnreset|etimedout|enetunreach|network (is )?unreachable|offline|could not resolve host/i;
/** The words of an account that is not signed in, or whose sign-in is gone. */
const SIGNED_OUT = /invalid api key|please run \/login|not logged in|log ?in again|authentication|oauth|unauthorized|\b401\b/i;

/**
 * Where the agent's first turn stands, from it and its conversation (top-level items, in their order). `waiting`: the
 * messages sent that wait for the agent's worktree (the conversation's `waiting`): they are gone from the composer.
 */
export function firstTurn(agent: Agent, items: ConvItem[], waiting: { text: string }[] = []): FirstTurnView {
  const top = items.filter((i) => !i.parent);
  const users = top.filter((i) => i.kind === 'user');
  const sent = waiting.length ? waiting[waiting.length - 1].text : users.length ? (users[users.length - 1] as { text: string }).text : null;
  const view = (state: FirstTurn, error: string | null = null, until: number | null = null): FirstTurnView => ({
    state,
    error,
    until,
    sent,
  });
  // A request waiting for the user comes first: the explanation goes beside its card.
  for (const id of agent.pending ?? []) {
    const it = top.find((i) => i.id === id) ?? items.find((i) => i.id === id);
    if (it?.kind === 'permission') return view('permission');
    if (it?.kind === 'question') return view('asking');
  }
  if (agent.status === 'running' || agent.status === 'waiting') return view('running');
  if (agent.signIn) return view('signedOut');
  // Sent, and waiting for its worktree (made at the first message, ESC-156): the turn starts once it is ready.
  if (waiting.length || agent.setup) return view('preparing');
  if (!users.length && !agent.prompts) return view('compose');
  // The turn of the last message sent: its end, when it has one.
  const lastUser = top.lastIndexOf(users[users.length - 1]);
  const turn = top.slice(lastUser + 1).find((i): i is TurnItem => i.kind === 'turn');
  if (agent.resumeAt || turn?.limited) return view('limit', null, agent.resumeAt ?? null);
  if (!turn) return users.length ? view('running') : view('compose');
  const error = turn.error?.trim() || null;
  if (turn.signIn || (turn.isError && error && SIGNED_OUT.test(error))) return view('signedOut', error);
  if (turn.isError && error && OFFLINE.test(error)) return view('offline', error);
  if (turn.isError || agent.status === 'error') return view('failed', error);
  return view('done');
}

/** The messages proposed to start the first agent (§ 4.5, choice O5), the first one selected. */
export type Starter = 'explain' | 'tests' | 'bug';

/**
 * The messages proposed for a project: « Lance les tests et dis-moi ce qui casse » only when it has a test command
 * Escouade recognizes that can run here (`tests`, the core's reading of its folder). The demonstration, whose two tests
 * fail on purpose, starts with it.
 */
export function starters(tests: boolean, demo = false): Starter[] {
  if (!tests) return ['explain', 'bug'];
  return demo ? ['tests', 'explain', 'bug'] : ['explain', 'tests', 'bug'];
}
