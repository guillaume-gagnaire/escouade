import { describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { kpis, niceMax, shown } from './stats';
import type { StatsView } from './types';

const view = (over: Partial<StatsView>): StatsView => ({
  range: 'day',
  buckets: Array.from({ length: 14 }, (_, i) => ({ label: String(i), start: i, input: 0, cache: 0, output: 0, cost: 0, prompts: 0 })),
  tokens: 0,
  tokensPrev: 0,
  cost: 0,
  costAll: 0,
  firstTs: null,
  prompts: 0,
  byProject: [],
  byModel: [],
  byAgent: [],
  byTicket: [],
  byAccount: [],
  accounts: [],
  ...over,
});

describe('kpis', () => {
  it('computes the evolution against the previous period', () => {
    expect(kpis(view({ tokens: 1080, tokensPrev: 1000 })).delta).toBe(8);
    expect(kpis(view({ tokens: 500, tokensPrev: 1000 })).delta).toBe(-50);
  });

  it('has no evolution without a previous period', () => {
    expect(kpis(view({ tokens: 1000, tokensPrev: 0 })).delta).toBeNull();
  });

  it('averages cost and tokens per prompt, and prompts per bucket', () => {
    const k = kpis(view({ tokens: 46_000, cost: 3, prompts: 2 }));
    expect(k.costPerPrompt).toBe(1.5);
    expect(k.tokensPerPrompt).toBe(23_000);
    expect(k.promptsPerBucket).toBeCloseTo(2 / 14);
  });

  it('does not divide by zero without prompts', () => {
    const k = kpis(view({ cost: 3 }));
    expect(k.costPerPrompt).toBeNull();
    expect(k.tokensPerPrompt).toBeNull();
  });

  it('describes the span of the range', () => {
    expect(kpis(view({ range: 'week' })).span).toBe('12 dernières semaines');
  });

  it('describes the span of the range in English, and reads an unknown range as the days', () => {
    setLang('en');
    expect(kpis(view({ range: 'month' })).span).toBe('Last 12 months');
    expect(kpis(view({ range: 'year' })).span).toBe('Last 14 days');
  });
});

describe('niceMax', () => {
  it.each([
    [0, 1],
    [7, 10],
    [1.5e6, 2e6],
    [2.2e6, 2.5e6],
    [3.4e6, 5e6],
    [9.1e6, 1e7],
    [1e6, 1e6],
  ])('%d → %d', (max, want) => expect(niceMax(max)).toBe(want));
});

describe('shown', () => {
  const list = Array.from({ length: 25 }, (_, i) => i);

  it('keeps the first 20 lines until everything is asked for', () => {
    expect(shown(list, false)).toEqual(list.slice(0, 20));
    expect(shown(list, true)).toEqual(list);
  });

  it('keeps a short list whole', () => {
    expect(shown(list.slice(0, 20), false)).toHaveLength(20);
    expect(shown([], false)).toEqual([]);
  });
});
