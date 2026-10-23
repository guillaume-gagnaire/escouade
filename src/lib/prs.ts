// The pull requests of an ordinary agent (ESC-64): who may open one, what its card and its header show of them, and the
// two corrections handed to the agent on a click (never by themselves).

import { t } from './i18n';
import { api } from './ipc';
import { openOutside } from './links';
import type { MenuItem } from './menu.svelte';
import { app } from './state.svelte';
import type { Agent, PrFix, PullRequest } from './types';

/** How many of an agent's pull requests its card shows: the ones to merge first (the backend's order). */
export const CARD_PRS = 3;

/** How often the pull requests of the agent shown are read again (the backend reads them at most every 20 s). */
export const PRS_EVERY_MS = 60_000;

/** The code host's word: « PR », « MR » on GitLab. */
export const prWord = (a: Agent) => a.prWord ?? 'PR';

/** The branch it works on, other than the project's folder's: its worktree's, else the one of the worktree it went into by itself (ESC-161). */
export const ownBranch = (a: Agent) => a.worktree?.branch ?? a.foundWorktree?.branch ?? null;

/** An ordinary agent (not a ticket's, which its board opens) with a branch of its own (`ownBranch`). Its pull requests are read. */
export const hasOwnBranch = (a: Agent) => !a.ticketId && !a.archived && !!ownBranch(a);

/**
 * Whether its pull requests are to be read: such an agent, or one without a worktree of its own, which may have gone into
 * one by itself since it was last read (ESC-161: the backend finds it from its session).
 */
export const mayHavePrs = (a: Agent) => !a.ticketId && !a.archived;

/**
 * Such an agent whose project's code host opens pull requests (`prWord`, set by the backend once the project has a remote
 * on a host with a client and an account): « Ouvrir une PR » is offered. None without a remote, on a host without a
 * client yet, or without an account.
 */
export const canOpenPr = (a: Agent) => hasOwnBranch(a) && !!a.prWord;

/** Its pull request still open, if any: « Voir la PR #n » rather than opening another. */
export const openPrOf = (a: Agent) => a.prs.find((p) => p.state === 'open') ?? null;

/** What its card shows. */
export const cardPrs = (a: Agent) => (a.ticketId ? [] : a.prs.slice(0, CARD_PRS));

/** The CI of an open one failed: the agent can be handed the failed checks. */
export const canFixChecks = (p: PullRequest) => p.state === 'open' && p.checks === 'failure';

/** A review asks for changes, or threads are still open: the agent can be handed them. */
export const canFixReviews = (p: PullRequest) => p.state === 'open' && (p.changesRequested || p.openThreads > 0);

/** Its state in a word: a draft is told as such while open. */
export const stateWord = (p: PullRequest) => t(`prs.state.${p.state === 'open' && p.draft ? 'draft' : p.state}`);

/** The tone of its number on the card: what to look at first (a failure, a request, then the rest). */
export function prTone(p: PullRequest): 'fail' | 'wait' | 'ok' | 'merged' | 'dim' {
  if (p.state === 'merged') return 'merged';
  if (p.state === 'closed' || p.draft) return 'dim';
  if (p.checks === 'failure' || p.mergeable === false) return 'fail';
  if (p.changesRequested || p.checks === 'pending') return 'wait';
  return 'ok';
}

/** The glyph before its number: merged, its CI failed, passed, running, or none. */
export function prGlyph(p: PullRequest): string {
  if (p.state === 'merged') return '⇡';
  return { failure: '✗', success: '✓', pending: '●', skipped: '○' }[p.checks ?? 'skipped'];
}

/** Its tooltip: number and title, then its state, its CI and its reviews in words. */
export function prTip(word: string, p: PullRequest): string {
  const parts = [stateWord(p), t(`prs.checks.${p.checks ?? 'none'}`)];
  if (p.state === 'open') {
    if (p.approvals) parts.push(t('prs.approvals', { count: p.approvals }));
    if (p.changesRequested) parts.push(t('prs.changesRequested'));
    if (p.openThreads) parts.push(t('prs.threads', { count: p.openThreads }));
    if (p.mergeable === false) parts.push(t('prs.conflict', { base: p.base }));
  }
  return `${t('prs.tipHead', { word, number: p.number, title: p.title })}\n${parts.join(' · ')}`;
}

/** Opens it in the system's browser. */
export const openPr = (p: PullRequest) => openOutside(p.url);

/** « Ouvrir une PR… »: its window, the title and the description proposed; « Voir la PR #n » once one is open. */
export function askOpenPr(a: Agent) {
  const open = openPrOf(a);
  if (open) openPr(open);
  else app.modal = { kind: 'pullRequest', agentId: a.id };
}

/** The item of the menus (its card's, the branch picker's): none for an agent that cannot have one. */
export function prMenuItem(a: Agent, before?: () => unknown): MenuItem | null {
  if (!canOpenPr(a)) return null;
  const open = openPrOf(a);
  const word = prWord(a);
  return {
    label: open ? t('prs.see', { word, number: open.number }) : t('prs.open', { word }),
    onClick: async () => {
      await before?.();
      askOpenPr(a);
    },
  };
}

/** « L'agent corrige » / « L'agent corrige les retours »: on the user's click, the host's say goes to the agent. */
export async function fixPr(a: Agent, p: PullRequest, fix: PrFix) {
  const sent = await app.run(api.agentPrFix(a.id, p.number, fix));
  if (sent !== undefined) app.toast(t('prs.fixSent', { agent: a.name }), 'ok');
}

/** Reads the agent's pull requests again, quietly: a host that does not answer shows what was last read. */
export function refreshPrs(a: Agent | null | undefined, now = false) {
  if (!a || !mayHavePrs(a)) return;
  api.agentPrsRefresh(a.id, now).catch(() => {});
}
