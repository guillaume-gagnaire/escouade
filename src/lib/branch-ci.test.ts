import { beforeEach, describe, expect, it } from 'vitest';
import { canStart, waitLabel } from './board';
import {
  ansiRuns,
  canSend,
  ciShown,
  ciOfTicket,
  ciPill,
  ciState,
  ciTitle,
  ciTone,
  logLineKind,
  pushedCi,
  redTargetHolds,
  sendTargets,
  targetCiOf,
} from './branch-ci';
import { setLang } from './i18n';
import { app } from './state.svelte';
import { agent, board, branchCi, resetApp, ticket, ticketPr } from '../test/ipc';

const HOUR = 3600_000;

describe('the CI of a branch without a pull request (ESC-63)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
  });

  it('draws the pill: red since when, green, waiting, without a job', () => {
    const now = Date.now();
    expect(ciPill(branchCi({ redSince: now - 2 * HOUR }), now)).toBe('main ✗ depuis 2 h');
    expect(ciPill(branchCi({ checks: 'success', redSince: null }), now)).toBe('main ✓');
    expect(ciPill(branchCi({ checks: 'pending', redSince: null }), now)).toBe('main · CI en cours');
    expect(ciPill(branchCi({ checks: null, redSince: null }), now)).toBe('main · aucune CI');
    expect(ciTone({ checks: 'skipped' })).toBe('success');
    expect(ciState(branchCi())).toBe('1/3 jobs en échec');
    setLang('en');
    expect(ciPill(branchCi({ redSince: now - 2 * HOUR }), now)).toBe('main ✗ for 2 h');
    expect(ciState(branchCi({ checks: 'success', checksTotal: 1 }))).toBe('1 job passed');
  });

  it('says in its tooltip why its last reading failed, the last state kept', () => {
    const now = Date.now();
    const title = ciTitle(branchCi({ error: 'GitHub injoignable', errorSince: now - 10 * 60_000 }), now);
    expect(title).toContain('CI de main : 1/3 jobs en échec');
    expect(title).toContain('GitHub injoignable · depuis 10 min');
  });

  it('holds the autopilot back only when asked and the target is red', () => {
    expect(redTargetHolds(board(), branchCi())).toBe(false);
    expect(redTargetHolds(board({ pauseOnRedTarget: true }), branchCi())).toBe(true);
    expect(redTargetHolds(board({ pauseOnRedTarget: true }), branchCi({ checks: 'pending' }))).toBe(false);
    expect(redTargetHolds(board({ pauseOnRedTarget: true }), null)).toBe(false);
    app.targetCi = { p1: branchCi() };
    expect(targetCiOf({ id: 'p1' })?.branch).toBe('main');
    expect(targetCiOf({ id: 'p2' })).toBeNull();
  });

  it('tells a ticket that waits for a green target, and lets it be launched by hand', () => {
    const s = board({ pauseOnRedTarget: true });
    const t = ticket();
    expect(waitLabel(t, 0, s, 0, null, null, [], true)).toBe('En attente : CI de la cible rouge');
    expect(waitLabel(t, 0, s, 0, null, null, [], false)).toBe("Pris dès qu'une place se libère");
    // Launched by hand: it starts all the same.
    expect(waitLabel({ ...t, forced: true }, 0, s, 0, null, null, [], true)).toBe('Lancement demandé…');
    expect(canStart(t, s, 0, null, null, null, true)).toBe(true);
    expect(canStart(t, s, 0, null, null, null, false)).toBe(false);
  });

  it('shows the pushed branch of a done ticket once read, and opens the CI of a card', () => {
    const pushed = branchCi({ branch: 'ticket/dem-1' });
    expect(pushedCi(ticket({ column: 'done', pushed }))).toEqual(pushed);
    // A branch without any job shows nothing; a reading that failed is said.
    expect(pushedCi(ticket({ column: 'done', pushed: { ...pushed, checks: null } }))).toBeNull();
    expect(pushedCi(ticket({ column: 'done', pushed: { ...pushed, checks: null, error: 'GitHub injoignable' } }))).not.toBeNull();
    expect(ciShown(branchCi({ checks: null }))).toBe(false);
    expect(ciShown(null)).toBe(false);
    expect(pushedCi(ticket({ column: 'todo', pushed }))).toBeNull();
    expect(ciOfTicket(ticket({ column: 'done', pushed }))).toEqual({ kind: 'pushed', id: 't1' });
    expect(ciOfTicket(ticket({ column: 'pr', pr: ticketPr() }))).toEqual({ kind: 'ticketPr', id: 't1' });
    expect(ciOfTicket(ticket())).toBeNull();
  });

  it('sends a failed job to the agents of the project, the most recent first', () => {
    const a = [
      agent({ id: 'a1', name: 'vieux', lastActivity: 1 }),
      agent({ id: 'a2', name: 'récent', lastActivity: 5 }),
      agent({ id: 'a3', name: 'archivé', archived: true, lastActivity: 9 }),
      agent({ id: 'a4', name: 'ailleurs', projectId: 'p2', lastActivity: 9 }),
    ];
    expect(sendTargets(a, 'p1').map((x) => x.name)).toEqual(['récent', 'vieux']);
    const check = { name: 'test', state: 'failure' as const, url: null, job: '7', started: null, finished: null, cancelled: false };
    expect(canSend(check)).toBe(true);
    expect(canSend({ ...check, state: 'success' })).toBe(false);
  });

  it('reads a log as a terminal does: its colours, its errors and its groups', () => {
    expect(ansiRuns('plain')).toEqual([{ text: 'plain', cls: '' }]);
    expect(ansiRuns('\x1b[31mred\x1b[0m then \x1b[1;92mbold green\x1b[39m')).toEqual([
      { text: 'red', cls: 'fg-1' },
      { text: ' then ', cls: '' },
      { text: 'bold green', cls: 'fg-10 b' },
    ]);
    // 256 colours: their arguments skipped; other escapes dropped.
    expect(ansiRuns('\x1b[38;5;196mx\x1b[2Ky')).toEqual([{ text: 'xy', cls: '' }]);
    expect(logLineKind('2026-01-02T08:05:00.1234567Z ##[error]Process completed with exit code 1.')).toBe('error');
    expect(logLineKind('##[group]Run npm test')).toBe('group');
    expect(logLineKind('##[warning]Deprecated')).toBe('warning');
    expect(logLineKind('ok')).toBeNull();
  });
});
