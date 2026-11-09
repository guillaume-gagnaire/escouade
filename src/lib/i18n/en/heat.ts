import { defineZone } from '../types';

// The temperature of an agent's cache (ESC-398): on its card, in its conversation's header, in the status bar.
export default defineZone('heat', {
  degrees: '{n} °C',
  unknown: '— °C',
  label: 'Cache',
  aria: {
    warm: 'Cache: {n} °C, expires in {left}',
    expired: 'Cache: 0 °C, expired: the next message reads everything again at full price',
    broken: 'Cache: 0 °C, broken ({why}): the next message reads everything again at full price',
    unknown: 'Cache: — °C, {engine} says nothing of how long its cache lives',
    none: 'Cache: — °C, {engine} has said nothing of its cache yet',
  },
  why: {
    model: 'model changed',
    compaction: 'context compacted',
    prompt: 'system prompt changed',
    account: 'account changed',
    conversation: 'new conversation',
  },
  left: { minutes: '{m} min {s}', seconds: '{s} s', hours: '{h} h {m}' },
  detail: {
    title: 'Cache: {n} °C',
    titleUnknown: 'Cache: — °C',
    warm: 'Last turn {ago} · expires in {left} ({engine}’s cache lives {life} without activity).',
    expired: 'Last turn {ago} · expired ({engine}’s cache lives {life} without activity).',
    broken: 'Last turn {ago} · broken: {why}.',
    unknownLife: 'Last turn {ago} · {engine} doesn’t say how long its cache lives.',
    life: { minutes: '{n} min', hours: { one: '{count} hour', other: '{count} hours' } },
    read: 'Read from the cache (warm)',
    written: 'Written to the cache (cold)',
    cached: 'Read or written to the cache',
    fresh: 'New, full price',
    saved: 'Saved on this turn',
    next: 'If the next message starts cold: ≈ {tokens} tokens read again, ≈ {cold} instead of ≈ {warm}.',
    nextTokens: 'If the next message starts cold: ≈ {tokens} tokens read again at full price.',
  },
  today: 'Cache today: ≈ {amount} saved',
  todayHint: 'What the agents’ cache saved today, from public prices',
});
