import type { Provenance, TeamMemoryEntry, TeamMemoryView } from '../lib/team-memory';

// The team's memory (TS-10) for the tests: Alice (an administrator) and her agent on DEM-41, Bob by hand.

export const ALICE = 'a1a1a1a1000000000000000000000000';
export const ALICE_DEVICE = 'ad00ad00000000000000000000000000';
export const BOB = 'b0b0b0b0000000000000000000000000';
export const BOB_DEVICE = 'bd00bd00000000000000000000000000';

export function provenance(over: Partial<Provenance> = {}): Provenance {
  return {
    member: ALICE,
    device: ALICE_DEVICE,
    name: 'Alice',
    at: Date.UTC(2026, 10, 3, 9, 30),
    by: 'agent',
    agent: { name: 'agent-dem-41', ticket: 'DEM-41', harness: 'claude', model: 'claude-opus-5-5' },
    thirdParty: false,
    imported: false,
    ...over,
  };
}

export function entry(over: Partial<TeamMemoryEntry> = {}): TeamMemoryEntry {
  return {
    id: '0123456789abcdef0123456789abcdef',
    name: 'tests-avant-commit',
    description: 'Lancer les tests avant de commiter',
    kind: 'feedback',
    body: 'npm test, puis cargo nextest.',
    provenance: provenance(),
    standing: 'accepted',
    reviewedBy: ALICE,
    removed: false,
    removedBy: null,
    quarantined: null,
    mine: true,
    untrusted: false,
    unknown: [],
    at: Date.UTC(2026, 10, 3, 9, 30),
    ...over,
  };
}

export function teamMemoryView(over: Partial<TeamMemoryView> = {}): TeamMemoryView {
  return {
    on: true,
    review: true,
    reviewLockedBy: null,
    admin: true,
    me: ALICE,
    memories: [],
    proposals: [],
    withdrawn: [],
    rules: [],
    authors: [
      { member: ALICE, device: ALICE_DEVICE, name: 'Alice' },
      { member: BOB, device: BOB_DEVICE, name: 'Bob' },
    ],
    tickets: ['DEM-41'],
    refused: 0,
    ...over,
  };
}
