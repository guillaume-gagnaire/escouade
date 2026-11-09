import { describe, expect, it } from 'vitest';
import { alertsOf, type AlertsInput } from './settings-alerts.svelte';

const none: AlertsInput = { harnesses: [], orphans: [], forges: [], ticketAccounts: [], plugins: [], runs: {} };
const plugin = (over: Partial<AlertsInput['plugins'][number]> = {}): AlertsInput['plugins'][number] => ({
  id: 'dev.x',
  name: 'X',
  state: { kind: 'ready' },
  choices: { enabled: true } as AlertsInput['plugins'][number]['choices'],
  declarative: false,
  ...over,
});

describe('the amber dots of the settings bar', () => {
  it('says nothing when all is well', () => {
    expect(alertsOf({ ...none, harnesses: [{ name: 'Claude Code', hasAccount: true }] })).toEqual({});
  });

  it('says a harness without an account on Comptes et harness', () => {
    const a = alertsOf({ ...none, harnesses: [{ name: 'Codex', hasAccount: false }] });
    expect(a.accounts).toEqual(['Codex n’a aucun compte : aucun agent ni ticket n’y démarre']);
  });

  it('says a project no account of its harness serves, on the project and on its page Projet', () => {
    const a = alertsOf({ ...none, orphans: [{ id: 'p1', harness: 'Codex' }] });
    expect(a['project:p1']).toEqual([expect.stringContaining('Aucun compte Codex ne sert ce projet')]);
    expect(a['p1:project']).toEqual(a['project:p1']);
  });

  it('says a source whose account was disconnected is paused, on the project and on its page Tickets importés (ESC-397)', () => {
    const link = (account?: string) => ({ service: 'jira' as const, container: 'MOB', name: 'MOB — Mobile', states: {}, account });
    const a = alertsOf({
      ...none,
      ticketAccounts: [{ service: 'jira', id: 'jira', connected: true, label: 'ada', unread: false }] as never,
      sources: [
        { id: 'p1', links: [link('jira-gone')] },
        { id: 'p2', links: [link()] },
      ],
    });
    expect(a['p1:tickets']).toEqual(['Source Jira « MOB — Mobile » en pause : son compte est déconnecté']);
    expect(a['project:p1']).toEqual(a['p1:tickets']);
    expect(a['p2:tickets']).toBeUndefined();
  });

  it('says a token expired or unread on Services connectés', () => {
    const a = alertsOf({
      ...none,
      forges: [
        { label: '@ada · github.com', status: { health: 'expired', missing: [], error: null } },
        { label: 'gitlab.com', status: { health: 'connected', missing: [], error: null } },
      ],
      ticketAccounts: [
        { label: 'ada@atlas.dev', connected: true, unread: true },
        { label: 'linear', connected: false, unread: true },
      ],
    });
    expect(a.services).toEqual(['Jeton expiré ou révoqué · @ada · github.com', 'Jeton illisible dans le trousseau · ada@atlas.dev']);
  });

  it('says a plugin stopped or disabled by Escouade, not one switched off', () => {
    const a = alertsOf({
      ...none,
      plugins: [
        plugin({ id: 'a', name: 'Trello' }),
        plugin({ id: 'b', name: 'Jira', state: { kind: 'disabled', reason: 'retiré' } }),
        plugin({ id: 'c', name: 'Off', choices: { enabled: false } as AlertsInput['plugins'][number]['choices'] }),
        plugin({ id: 'd', name: 'Off2', choices: { enabled: false } as AlertsInput['plugins'][number]['choices'] }),
      ],
      runs: {
        a: { phase: 'stopped', stop: { kind: 'crashed', times: 3 }, restarts: 3 },
        d: { phase: 'stopped', stop: { kind: 'crashed', times: 3 }, restarts: 3 },
      },
    });
    expect(a.plugins).toEqual(['Trello : Arrêté après 3 plantages', 'Jira : Désactivé par Escouade']);
  });

  it('says nothing of a plugin stopped on purpose', () => {
    const a = alertsOf({ ...none, plugins: [plugin()], runs: { 'dev.x': { phase: 'stopped', stop: { kind: 'asked' }, restarts: 0 } } });
    expect(a.plugins).toBeUndefined();
  });
});
