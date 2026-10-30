import type { DocsDecision } from '../lib/types';

/** A decision of the register for the tests, accepted unless said otherwise. */
export const decision = (number: number | null, o: Partial<DocsDecision> = {}): DocsDecision => ({
  number,
  id: `decision-${number ?? 'proposee'}`,
  title: `Décision ${number ?? 'proposée'}`,
  date: '2026-10-28',
  state: 'accepted',
  origin: '',
  originRef: '',
  pages: [],
  tickets: [],
  documents: [],
  supersedes: [],
  supersededBy: null,
  changes: [],
  acceptedAt: '',
  acceptedBy: '',
  context: '',
  decision: '',
  consequences: '',
  ...o,
});
