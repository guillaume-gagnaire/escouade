import { describe, expect, it } from 'vitest';
import { agent, board, ticket } from '../test/ipc';
import source from './board.ts?raw';
import {
  APPROVE_LABEL,
  canStart,
  columnTickets,
  commitPreview,
  cycleRefusal,
  doneMeta,
  keyList,
  keyPrefix,
  launchAnyway,
  pauseLabel,
  placesLabel,
  queueIndices,
  quotaUntil,
  settingsSummary,
  ticketTag,
  waitingFor,
  waitLabel,
} from './board';
import type { Ticket } from './types';

describe('board labels', () => {
  it('orders each column', () => {
    const list = [
      ticket({ id: 'b', rank: 2 }),
      ticket({ id: 'a', rank: 1 }),
      ticket({ id: 'old', column: 'done', doneAt: 1 }),
      ticket({ id: 'new', column: 'done', doneAt: 2 }),
    ];
    expect(columnTickets(list, 'todo').map((t) => t.id)).toEqual(['a', 'b']);
    expect(columnTickets(list, 'done').map((t) => t.id)).toEqual(['new', 'old']);
  });

  it('counts the free places, or tells the quota', () => {
    expect(placesLabel(0, 2, null)).toBe('2 places libres');
    expect(placesLabel(1, 2, null)).toBe('1 place libre');
    expect(placesLabel(2, 2, null)).toBe('Toutes les places sont prises');
    const at = new Date(2026, 9, 3, 15, 0).getTime();
    expect(placesLabel(0, 2, at)).toBe('Quota atteint — reprise à 15:00');
    expect(quotaUntil([agent({ resumeAt: 5 }), agent({ id: 'a2', resumeAt: 3 }), agent({ id: 'a3', resumeAt: 1, archived: true })])).toBe(
      3,
    );
    expect(quotaUntil([agent()])).toBeNull();
  });

  it('tells why the autopilot is paused, and until when', () => {
    const now = new Date(2026, 9, 3, 13, 30).getTime();
    const at = new Date(2026, 9, 3, 14, 0).getTime();
    expect(pauseLabel({ reason: 'week', pct: 96, until: at }, now)).toBe('Pilote auto en pause : quota hebdo à 96 % (reprise à 14:00)');
    expect(pauseLabel({ reason: 'fiveHour', pct: 100, until: at }, now)).toBe(
      'Pilote auto en pause : quota de 5 h à 100 % (reprise à 14:00)',
    );
    const about = new Date(2026, 9, 3, 14, 30).getTime();
    expect(pauseLabel({ reason: 'limit', pct: null, until: about }, now)).toBe(
      "Pilote auto en pause : limite d'usage atteinte (reprise vers 14:30)",
    );
    // A weekly window that ends another day says which, its use as the status bar rounds it.
    const monday = new Date(2026, 9, 5, 9, 0).getTime();
    expect(pauseLabel({ reason: 'week', pct: 95.6, until: monday }, now)).toBe(
      'Pilote auto en pause : quota hebdo à 96 % (reprise le lundi 5 octobre à 09:00)',
    );
    // While it holds, no ticket is to launch by hand, and none is promised a place.
    const pause = { reason: 'limit' as const, pct: null, until: about };
    expect(canStart(ticket(), board({ autopilot: false }), 0, null, null, pause)).toBe(false);
    expect(waitLabel(ticket(), 0, board(), 0, null, pause)).toBe('En attente : pilote auto en pause');
    expect(waitLabel(ticket(), 1, board(), 2, null, pause)).toBe('En attente : pilote auto en pause');
    expect(waitLabel(ticket({ forced: true }), 0, board({ autopilot: false }), 0, null, pause)).toBe('En attente : pilote auto en pause');
    // The target branch first: the pause over, it still holds them back.
    expect(waitLabel(ticket(), 0, board(), 0, 'Branche cible main introuvable — aucun ticket ne démarre', pause)).toBe(
      'En attente de la branche cible',
    );
  });

  it('sums up what validating does, and names the button', () => {
    expect(settingsSummary(board(), 'main')).toBe('merge squash → main');
    expect(settingsSummary(board({ action: 'pr' }), 'main')).toBe('PR → main');
    expect(settingsSummary(board({ action: 'push' }), 'main')).toBe('push ticket/*');
    expect(settingsSummary(board({ action: 'keep' }), 'main')).toBe("laisser en l'état");
    expect(APPROVE_LABEL).toEqual({ merge: 'Valider et merger', pr: 'Valider + PR', push: 'Valider et pousser', keep: 'Valider' });
  });

  it('tells a ticket to do when it will start', () => {
    const s = board();
    expect(waitLabel(ticket(), 0, s, 2)).toBe("Pris dès qu'une place se libère");
    expect(waitLabel(ticket(), 1, s, 2)).toBe("En attente d'une place (2/2)");
    expect(waitLabel(ticket(), 0, board({ autopilot: false }), 0)).toBe('Pilote auto désactivé');
    expect(waitLabel(ticket({ forced: true }), 0, board({ autopilot: false }), 0)).toBe('Lancement demandé…');
    expect(canStart(ticket(), board({ autopilot: false }), 1, null)).toBe(true);
    expect(canStart(ticket(), board({ autopilot: false }), 2, null)).toBe(false);
    expect(canStart(ticket(), board(), 0, null)).toBe(false);
    expect(canStart(ticket(), board({ autopilot: false }), 0, 5)).toBe(false);
  });

  it('never promises a place while the target branch cannot start a ticket', () => {
    const issue = 'Branche cible main introuvable — aucun ticket ne démarre';
    expect(waitLabel(ticket(), 0, board(), 0, issue)).toBe('En attente de la branche cible');
    expect(waitLabel(ticket({ forced: true }), 0, board({ autopilot: false }), 0, issue)).toBe('En attente de la branche cible');
    expect(canStart(ticket(), board({ autopilot: false }), 0, null, issue)).toBe(false);
    expect(waitLabel(ticket(), 0, board(), 0, null)).toBe("Pris dès qu'une place se libère");
  });

  it('previews the commit message with the project key', () => {
    expect(keyPrefix('Écoute-api')).toBe('ECO');
    expect(keyPrefix('42')).toBe('TIC');
    expect(keyPrefix('Éléphant')).toBe('ELE');
    expect(commitPreview(board(), 'atlas')).toBe('feat: limiter les tentatives de connexion [ATL-42]');
    expect(commitPreview(board({ conventional: false, prefix: 'ZZZ' }), 'atlas')).toBe('ZZZ-42 Limiter les tentatives de connexion');
  });

  it('tags the agent of a ticket and sums a finished one up', () => {
    expect(ticketTag(ticket({ column: 'doing', iteration: 2 }))).toBe('DEM-1 · boucle 2/5');
    expect(ticketTag(ticket({ column: 'review' }))).toBe('DEM-1 · à tester');
    expect(ticketTag(ticket())).toBeNull();
    expect(ticketTag(undefined)).toBeNull();
    expect(doneMeta(ticket({ iteration: 3, cost: 0.42 }))).toBe('3 boucles · 0,42 $');
  });

  it('counts every loop of a finished ticket, those of its rounds sent back included', () => {
    // Sent back once after two loops, done in one: three loops, though its last round says 1.
    expect(doneMeta(ticket({ column: 'done', iteration: 1, loops: 3, cost: 1.5 }))).toBe('3 boucles · 1,50 $');
    // Saved before the loops were counted: its last round's.
    expect(doneMeta(ticket({ column: 'done', iteration: 2, loops: 0, cost: 1.5 }))).toBe('2 boucles · 1,50 $');
  });

  it('writes the accent-stripping pattern without any literal combining mark (they get lost when copied)', () => {
    expect(source).not.toMatch(/\p{M}/u);
  });
});

describe('dependencies between tickets', () => {
  const all = (...list: Ticket[]) => Object.fromEntries(list.map((t) => [t.id, t]));

  it('names the tickets a ticket comes after that are not done yet, in its order', () => {
    const t3 = ticket({ id: 't3', key: 'DEM-3', column: 'doing' });
    const t4 = ticket({ id: 't4', key: 'DEM-4', column: 'done' });
    const t6 = ticket({ id: 't6', key: 'DEM-6', column: 'review' });
    // One gone (deleted meanwhile) holds nothing back.
    const t5 = ticket({ id: 't5', key: 'DEM-5', after: ['t6', 't4', 'gone', 't3'] });
    expect(waitingFor(t5, all(t3, t4, t5, t6))).toEqual(['DEM-6', 'DEM-3']);
    expect(waitingFor(ticket(), all(t3))).toEqual([]);
    expect(keyList(['DEM-3'])).toBe('DEM-3');
    expect(keyList(['DEM-3', 'DEM-4'])).toBe('DEM-3 et DEM-4');
    expect(keyList(['DEM-3', 'DEM-4', 'DEM-6'])).toBe('DEM-3, DEM-4 et DEM-6');
  });

  it('tells a ticket to do that waits for others, after the target branch and before the pause, unless launched by hand', () => {
    const issue = 'Branche cible main introuvable — aucun ticket ne démarre';
    const pause = { reason: 'limit' as const, pct: null, until: 1 };
    expect(waitLabel(ticket(), 0, board(), 0, null, null, ['DEM-3'])).toBe('⏸ après DEM-3');
    expect(waitLabel(ticket(), 1, board(), 2, null, null, ['DEM-3', 'DEM-4'])).toBe('⏸ après DEM-3 et DEM-4');
    expect(waitLabel(ticket(), 0, board({ autopilot: false }), 0, null, null, ['DEM-3'])).toBe('⏸ après DEM-3');
    expect(waitLabel(ticket(), 0, board(), 0, null, pause, ['DEM-3'])).toBe('⏸ après DEM-3');
    expect(waitLabel(ticket(), 0, board(), 0, issue, pause, ['DEM-3'])).toBe('En attente de la branche cible');
    // Launched by hand, it starts all the same: only what holds every ticket back holds it.
    expect(waitLabel(ticket({ forced: true }), 0, board({ autopilot: false }), 0, null, null, ['DEM-3'])).toBe('Lancement demandé…');
    expect(waitLabel(ticket({ forced: true }), 0, board(), 0, null, pause, ['DEM-3'])).toBe('En attente : pilote auto en pause');
  });

  it('asks before a ticket that waits is launched by hand', () => {
    expect(launchAnyway('DEM-5', ['DEM-3'])).toBe('DEM-5 attend DEM-3, pas encore terminé. Le lancer quand même ?');
    expect(launchAnyway('DEM-5', ['DEM-3', 'DEM-4'])).toBe('DEM-5 attend DEM-3 et DEM-4, pas encore terminés. Le lancer quand même ?');
  });

  it('passes over the tickets that wait in the queue of « À faire », as the autopilot does, unless launched by hand', () => {
    const t3 = ticket({ id: 't3', key: 'DEM-3', column: 'doing' });
    const a = ticket({ id: 'a', after: ['t3'] });
    const b = ticket({ id: 'b', rank: 2 });
    const c = ticket({ id: 'c', rank: 3, after: ['t3'], forced: true });
    const d = ticket({ id: 'd', rank: 4, after: ['b'] });
    expect(queueIndices([a, b, c, d], all(t3, a, b, c, d))).toEqual({ a: -1, b: 0, c: 1, d: -1 });
  });

  it('refuses a dependency that already waits for the ticket, directly or not', () => {
    const t3 = ticket({ id: 't3', key: 'DEM-3', after: ['t4'] });
    const t4 = ticket({ id: 't4', key: 'DEM-4', after: ['t5'] });
    const t5 = ticket({ id: 't5', key: 'DEM-5' });
    const t6 = ticket({ id: 't6', key: 'DEM-6', after: ['t7'] });
    // A loop already there (a hand-edited file) does not hang the search.
    const t7 = ticket({ id: 't7', key: 'DEM-7', after: ['t6'] });
    const list = all(t3, t4, t5, t6, t7);
    expect(cycleRefusal(t5, t3, list)).toBe('DEM-3 attend déjà DEM-5 (directement ou non).');
    expect(cycleRefusal(t5, t4, list)).toBe('DEM-4 attend déjà DEM-5 (directement ou non).');
    expect(cycleRefusal(t5, t6, list)).toBeNull();
    // DEM-3 after DEM-5 as well is no loop: it waits for it already.
    expect(cycleRefusal(t3, t5, list)).toBeNull();
  });
});
