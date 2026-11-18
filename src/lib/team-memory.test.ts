import { describe, expect, it } from 'vitest';
import { BOB, BOB_DEVICE, entry, provenance } from '../test/team-memory';
import { author, dayStart, takes } from './team-memory';

describe('the team’s memory (TS-10)', () => {
  it('says who wrote a memory: the agent with its ticket, harness and model, or the member by hand', () => {
    expect(author(provenance())).toEqual({ who: 'Alice', agent: 'agent-dem-41 · DEM-41 · claude · claude-opus-5-5' });
    expect(author(provenance({ by: 'hand', agent: null }))).toEqual({ who: 'Alice', agent: null });
    expect(author(provenance({ name: '  ', by: 'hand', agent: null })).who).toBe('a1a1a1a1');
  });

  it('takes out what the core’s rule takes: a ticket’s agent, a member or a device, since a date', () => {
    const alices = entry();
    const bobs = entry({ provenance: provenance({ member: BOB, device: BOB_DEVICE, agent: null, by: 'hand', at: Date.UTC(2026, 10, 5) }) });
    expect(takes({}, alices)).toBe(false);
    expect(takes({ ticket: 'dem-41' }, alices)).toBe(true);
    expect(takes({ ticket: 'DEM-41' }, bobs)).toBe(false);
    expect(takes({ member: BOB }, bobs)).toBe(true);
    expect(takes({ device: BOB_DEVICE, since: Date.UTC(2026, 10, 4) }, bobs)).toBe(true);
    expect(takes({ device: BOB_DEVICE, since: Date.UTC(2026, 10, 6) }, bobs)).toBe(false);
  });

  it('reads a day typed in a date field', () => {
    expect(dayStart('2026-11-03')).toBe(new Date(2026, 10, 3).getTime());
    expect(dayStart('')).toBeNull();
    expect(dayStart('3 novembre')).toBeNull();
  });
});
