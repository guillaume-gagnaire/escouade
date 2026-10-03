import { describe, expect, it } from 'vitest';
import { agent, board, ticket } from '../test/ipc';
import source from './board.ts?raw';
import {
  APPROVE_LABEL,
  canStart,
  columnTickets,
  commitPreview,
  doneMeta,
  keyPrefix,
  placesLabel,
  quotaUntil,
  settingsSummary,
  ticketTag,
  waitLabel,
} from './board';

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

  it('writes the accent-stripping pattern without any literal combining mark (they get lost when copied)', () => {
    expect(source).not.toMatch(/\p{M}/u);
  });
});
