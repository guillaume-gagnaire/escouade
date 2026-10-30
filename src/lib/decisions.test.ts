import { describe, expect, it } from 'vitest';
import {
  decisionChangedLine,
  decisionChanges,
  decisionCounts,
  decisionDay,
  decisionDayWords,
  decisionKey,
  decisionLabel,
  decisionNo,
  decisionRef,
  decisionOrigin,
  filterDecisions,
  cites,
  refLabel,
  replacementsOf,
} from './decisions';
import { decision } from '../test/decisions';
import type { DocsTree } from './types';

const TREE: DocsTree = {
  branch: '',
  pages: 2,
  existingSpec: [],
  subjects: [
    {
      name: 'paiement',
      title: 'Paiement',
      pages: [{ path: 'paiement/echeances', title: 'Échéances', compared: '', tickets: [], decisions: [], gaps: 0, draft: false }],
    },
    {
      name: 'adhesion',
      title: 'Adhésion en ligne',
      pages: [{ path: 'adhesion/foyer', title: 'Foyer', compared: '', tickets: [], decisions: [], gaps: 0, draft: false }],
    },
  ],
};

// 28 October 2026, 15:00, local, in milliseconds (as `app.now`).
const NOW = new Date(2026, 9, 28, 15).getTime();

describe('the register of the decisions', () => {
  it('numbers, counts and filters them, the latest first', () => {
    const list = [decision(4), decision(9, { state: 'superseded', supersededBy: 13 }), decision(13), decision(15, { state: 'proposed' })];
    expect(decisionNo(4)).toBe('004');
    expect(decisionNo(1234)).toBe('1234');
    expect(decisionCounts(list)).toEqual({ all: 4, accepted: 2, proposed: 1, superseded: 1 });
    expect(filterDecisions(list, 'all').map((d) => d.number)).toEqual([15, 13, 9, 4]);
    expect(filterDecisions(list, 'accepted').map((d) => d.number)).toEqual([13, 4]);
    expect(filterDecisions(list, 'superseded').map((d) => d.number)).toEqual([9]);
    // The list itself is left in its order.
    expect(list.map((d) => d.number)).toEqual([4, 9, 13, 15]);
  });

  it('cites a proposal by its id, as it has no number until accepted, and lists the proposals first', () => {
    const tarif = decision(null, { id: 'tarif-famille', state: 'proposed', date: '2026-10-29' });
    const older = decision(null, { id: 'relances', state: 'proposed', date: '2026-10-20' });
    const list = [decision(4), tarif, decision(13), older];
    expect(decisionRef(tarif)).toBe('tarif-famille');
    expect(decisionRef(list[2])).toBe(13);
    expect([decisionLabel(tarif), decisionLabel(list[0])]).toEqual(['tarif-famille', '004']);
    expect([refLabel(7), refLabel('relances')]).toEqual(['007', 'relances']);
    expect(decisionKey(list[0])).toBe('4');
    expect(cites('tarif-famille', tarif) && cites(13, list[2]) && !cites(4, tarif)).toBe(true);
    expect(filterDecisions(list, 'all').map(decisionRef)).toEqual(['tarif-famille', 'relances', 13, 4]);
    // A proposal is replaced by nothing.
    expect(replacementsOf(tarif, list)).toEqual([]);
  });

  it('says a day as the register does: today, yesterday, else its day and month', () => {
    expect(decisionDay('2026-10-28', NOW)).toBe('Auj.');
    expect(decisionDay('2026-10-27', NOW)).toBe('Hier');
    expect(decisionDay('2026-10-24', NOW)).toBe('24/10');
    expect(decisionDay('2025-12-02', NOW)).toBe('02/12/25');
    expect(decisionDay('2026-10-27T18:00:00+02:00', NOW)).toBe('Hier');
    expect(decisionDay('bientôt', NOW)).toBe('bientôt');
    expect(decisionDayWords('2026-10-27T09:00:00+02:00', NOW)).toBe('hier');
    expect(decisionDayWords('2026-10-02', NOW)).toBe('le 02/10');
  });

  it('says where each comes from', () => {
    expect(decisionOrigin(decision(1, { origin: 'brainstorm', originRef: 'Tarif famille' }))).toBe('Brainstorm « Tarif famille »');
    expect(decisionOrigin(decision(1, { origin: 'Discussion', originRef: 'Prestataire' }))).toBe('Discussion « Prestataire »');
    expect(decisionOrigin(decision(1, { origin: 'ticket', originRef: 'DEM-31' }))).toBe('Ticket DEM-31');
    expect(decisionOrigin(decision(1, { origin: 'document', originRef: 'Cahier des charges v3' }))).toBe('Cahier des charges v3');
    expect(decisionOrigin(decision(1, { origin: 'document' }))).toBe('Un document');
    expect(decisionOrigin(decision(1))).toBe('');
    expect(decisionOrigin(decision(1, { origin: 'atelier', originRef: '22/10' }))).toBe('atelier · 22/10');
  });

  it('says what each changed in the spec, by the titles of its pages and subjects', () => {
    const d = decision(14, {
      changes: [
        { page: 'adhesion/foyer', added: 4, removed: 0 },
        { page: 'paiement/echeances', added: 2, removed: 1 },
        { page: 'notifications/sms', added: 1, removed: 0 },
      ],
    });
    expect(decisionChanges(d, TREE)).toEqual([
      { path: 'adhesion/foyer', added: 4, removed: 0, title: 'Foyer', subject: 'Adhésion en ligne', known: true },
      { path: 'paiement/echeances', added: 2, removed: 1, title: 'Échéances', subject: 'Paiement', known: true },
      { path: 'notifications/sms', added: 1, removed: 0, title: 'notifications/sms', subject: 'notifications', known: false },
    ]);
    expect(decisionChangedLine(d, TREE)).toBe('Adhésion en ligne, Paiement, notifications');
    // Without changes, the pages it names; replaced, by which.
    const old = decision(9, { pages: ['paiement/echeances'], state: 'superseded', supersededBy: 13 });
    expect(decisionChanges(old, TREE)[0]).toMatchObject({ title: 'Échéances', added: null, removed: null });
    expect(decisionChangedLine(old, TREE)).toBe('Paiement · par 013');
    expect(decisionChangedLine(decision(2, { state: 'superseded', supersededBy: 3 }), null)).toBe('par 003');
    expect(decisionChangedLine(decision(2), null)).toBe('');
  });

  it('finds the proposals that would replace an accepted one', () => {
    const list = [decision(9), decision(13, { state: 'proposed', supersedes: [9] }), decision(14, { supersedes: [9] })];
    expect(replacementsOf(list[0], list).map((d) => d.number)).toEqual([13]);
    expect(replacementsOf(list[1], list)).toEqual([]);
  });
});
