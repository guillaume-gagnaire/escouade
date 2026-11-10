// An agent's cache as the backend keeps it (ESC-398), for the tests.
import { localDay } from '../lib/heat.svelte';
import type { CacheHeat } from '../lib/types';

export const AT = new Date(2026, 10, 9, 14, 0).getTime();

export function heat(over: Partial<CacheHeat> = {}): CacheHeat {
  return {
    at: AT,
    ttl: 300,
    broken: null,
    read: 129_000,
    written: 9_000,
    cached: 138_000,
    fresh: 4_000,
    context: 142_000,
    saved: 0.36,
    warm: 0.05,
    cold: 0.47,
    model: 'gpt-6-luna',
    day: localDay(AT),
    savedToday: 3.1,
    ...over,
  };
}
