// What an agent has used, including its running turn: the CLI reports a turn's exact cost only
// when it ends, until then the backend estimates it from list prices.

import { fInt, fPct, fUsd } from './format';
import { t } from './i18n';
import type { Agent } from './types';

export interface Spent {
  tokens: number;
  cost: number;
  /** Part of it comes from a running turn: the cost is an estimate. */
  estimated: boolean;
}

export function spent(a: Pick<Agent, 'tokens' | 'cost' | 'liveTokens' | 'liveCost'>): Spent {
  const liveTokens = a.liveTokens ?? 0;
  return { tokens: a.tokens + liveTokens, cost: a.cost + (a.liveCost ?? 0), estimated: liveTokens > 0 || (a.liveCost ?? 0) > 0 };
}

export function fSpentUsd(s: { cost: number; estimated: boolean }): string {
  // A non-breaking space, as in fUsd: « ≈ » never ends a line alone.
  return (s.estimated ? '≈\u00a0' : '') + fUsd(s.cost);
}

/** Why the cost of a running turn is an estimate (read where it is shown, so that it follows the language). */
export const estimateHint = () => t('shell.spend.estimateHint');

export interface ContextUse {
  /** Percent of the model's window. */
  pct: number;
  /** In the language of the interface when `contextUse` is called: call it where it is shown. */
  title: string;
  /** Nearly full: Claude Code compacts it soon. */
  full: boolean;
}

/** How full an agent's context is, out of the window of its conversation's model: null until a turn told the window. */
export function contextUse(a: Pick<Agent, 'contextTokens' | 'contextWindow'>): ContextUse | null {
  const size = a.contextWindow;
  if (!size) return null;
  const pct = Math.round((a.contextTokens / size) * 100);
  return {
    pct,
    title: t('shell.spend.context', { pct: fPct(pct), used: fInt(a.contextTokens), size: fInt(size) }),
    full: pct >= 80,
  };
}
