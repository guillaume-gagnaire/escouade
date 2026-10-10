// Derived KPIs of the Stats tab.

import { t } from './i18n';
import type { StatsView } from './types';

export type Range = 'day' | 'week' | 'month';

export interface Kpis {
  span: string;
  /** Token evolution vs the previous period, in percent (null when there is no reference). */
  delta: number | null;
  costPerPrompt: number | null;
  tokensPerPrompt: number | null;
  promptsPerBucket: number;
}

export function kpis(v: StatsView): Kpis {
  // A range the backend does not know is read as the default one, the days.
  const range: Range = v.range === 'week' || v.range === 'month' ? v.range : 'day';
  return {
    span: t(`stats.span.${range}`),
    delta: v.tokensPrev > 0 ? Math.round(((v.tokens - v.tokensPrev) / v.tokensPrev) * 100) : null,
    costPerPrompt: v.prompts > 0 ? v.cost / v.prompts : null,
    tokensPerPrompt: v.prompts > 0 ? v.tokens / v.prompts : null,
    promptsPerBucket: v.buckets.length ? v.prompts / v.buckets.length : 0,
  };
}

/** Lines of the lists of agents and tickets before « Tout voir ». */
export const ROWS = 20;

export function shown<T>(list: T[], all: boolean): T[] {
  return all ? list : list.slice(0, ROWS);
}

/** Rounds an axis maximum up to 1, 2, 2.5 or 5 × 10^n. */
export function niceMax(max: number): number {
  if (max <= 0) return 1;
  const mag = Math.pow(10, Math.floor(Math.log10(max)));
  const n = max / mag;
  const step = n <= 1 ? 1 : n <= 2 ? 2 : n <= 2.5 ? 2.5 : n <= 5 ? 5 : 10;
  return step * mag;
}
