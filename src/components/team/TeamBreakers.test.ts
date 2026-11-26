import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { denial, lockLine, locks, setLocks } from '../../lib/locks.svelte';
import { app } from '../../lib/state.svelte';
import { bandOf, type Breaker, type BreakersView } from '../../lib/team-breakers';
import { teams, type TeamView } from '../../lib/teams.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import BreakerBand from './BreakerBand.svelte';
import TeamBreakers from './TeamBreakers.svelte';

// The administrator's circuit breaker (TS-15): the band says who cut, what and why, and its scope; the team's window
// shows what is in force, and lets an administrator set one (confirmed with what stops and on how many devices online)
// and lift it; a member sets and lifts nothing.

const TEAM: TeamView = {
  id: 'acme',
  name: 'Acme',
  server: 'team.acme.fr',
  joined: true,
  role: 'admin',
  admins: 2,
  members: 3,
  epoch: 3,
  fingerprint: [],
  head: [],
  escrow: null,
  keysInFile: false,
  recoveryConfirmed: true,
  wordsPending: false,
  recoveryOutdated: false,
  recoveryStale: false,
  alarm: null,
  invitations: [],
  lastError: null,
  snapshotAlerts: [],
  dropAlerts: [],
  admissions: [],
  refusals: [],
  joinWords: [],
};

const CUT: Breaker = {
  id: 'aa'.repeat(16),
  scope: { kind: 'team' },
  what: { kind: 'harness', id: 'claude' },
  reason: 'fuite de clé chez le fournisseur X',
  by: 'Claire Martin',
  member: '01'.repeat(16),
  at: 1_800_000_000,
  serial: 1,
  seq: 1,
};

function view(over: Partial<BreakersView> = {}, active: Breaker[] = [CUT]): BreakersView {
  return {
    team: 'acme',
    name: 'Acme',
    admin: true,
    signerName: 'Claire Martin',
    lastContact: 1_800_000_000,
    withdrawn: [],
    view: { active, lifted: [], refusals: [], alarm: null, serial: active.length },
    ...over,
  };
}

describe('the band of a circuit breaker (TS-15)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    teams.list = [TEAM];
    teams.breakers = {};
    setLocks(undefined);
    app.modal = null;
  });

  it('says who cut, what and why, and its scope; « Voir » opens the team’s breakers', async () => {
    fakeBackend({ team_list: () => [TEAM], team_breakers: () => view(), team_settings: () => null });
    render(BreakerBand);
    const band = await screen.findByTestId('breaker-band');
    expect(band).toHaveTextContent('Arrêt demandé par Claire Martin : fuite de clé chez le fournisseur X');
    expect(band).toHaveTextContent('Retiré : le harness Claude Code · toute l’équipe Acme');
    await userEvent.click(within(band).getByRole('button', { name: 'Voir' }));
    expect(app.modal).toEqual({ kind: 'team', step: 'breakers', teamId: 'acme' });
  });

  it('says nothing without a breaker, and the other breakers by their number', () => {
    expect(bandOf([view({}, [])], null, Date.now())).toBeNull();
    const two = bandOf(
      [view({}, [{ ...CUT, id: 'b', what: { kind: 'agents' }, scope: { kind: 'project', project: 'ab'.repeat(16), name: 'API' } }, CUT])],
      null,
      Date.now(),
    )!;
    expect(two.detail).toBe('Retiré : tous les agents · le projet API de l’équipe Acme · et 1 autre coupe-circuit');
  });

  it('says, under « sans contact depuis N heures », that no new team agent starts, with the last contact', () => {
    const now = 1_800_000_000 * 1000;
    const late = view({ lastContact: 1_800_000_000 - 3 * 3600 }, []);
    expect(bandOf([late], null, now)).toBeNull();
    expect(bandOf([late], 4, now)).toBeNull();
    const band = bandOf([late], 2, now)!;
    expect(band.kind).toBe('contact');
    expect(band.text).toBe('Pas de nouvel agent de l’équipe Acme : son serveur n’a pas répondu depuis plus de 2 h (stratégie de ce poste)');
    expect(band.detail).toMatch(/^Dernier contact : /);
    expect(bandOf([view({ lastContact: null }, [])], 2, now)?.detail).toBe('Aucun contact encore');
  });

  it('is shown by the band under the policy of this device', async () => {
    fakeBackend({ team_list: () => [TEAM], team_breakers: () => view({ lastContact: 1 }, []), team_settings: () => null });
    setLocks({
      locks: { 'team.maxOfflineHours': { key: 'team.maxOfflineHours', value: 24, by: { kind: 'device', source: 'x' }, bounds: [] } },
      rows: {},
      warnings: [],
    });
    expect(locks.current.locks['team.maxOfflineHours'].value).toBe(24);
    render(BreakerBand);
    expect(await screen.findByTestId('breaker-band')).toHaveTextContent('Pas de nouvel agent de l’équipe Acme');
  });
});

describe('the team’s circuit breakers in its window (TS-15)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({
      projects: [project({ id: 'p1', name: 'API', team: 'acme', teamProject: 'ab'.repeat(16) }), project({ id: 'p2', name: 'Perso' })],
    });
    teams.list = [TEAM];
    teams.breakers = {};
  });

  it('shows those in force to a member, who neither sets nor lifts', async () => {
    fakeBackend({ team_breakers: () => view({ admin: false }) });
    render(TeamBreakers, { teamId: 'acme', onback: () => {} });
    const cut = await screen.findByTestId('breaker-active');
    expect(cut).toHaveTextContent('Arrêt demandé par Claire Martin : fuite de clé chez le fournisseur X');
    expect(cut).toHaveTextContent('le harness Claude Code · toute l’équipe Acme');
    expect(screen.queryByRole('button', { name: 'Lever' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Poser un coupe-circuit' })).toBeNull();
    expect(screen.getByText(/Seul un administrateur pose ou lève/)).toBeInTheDocument();
  });

  it('lets an administrator set one, confirmed with what stops and on how many devices online, then lift it', async () => {
    let state = view({}, []);
    const backend = fakeBackend({
      team_breakers: () => state,
      team_breaker_online: () => 3,
      team_breaker_set: () => (state = view()),
      team_breaker_lift: () => (state = view({}, [])),
    });
    render(TeamBreakers, { teamId: 'acme', onback: () => {} });
    const form = await screen.findByRole('region', { name: 'Poser un coupe-circuit' });
    const next = within(form).getByRole('button', { name: 'Continuer' });
    expect(next).toBeDisabled();
    await userEvent.type(within(form).getByRole('textbox', { name: 'Raison' }), 'fuite de clé chez le fournisseur X');
    expect(next).toBeEnabled();
    await userEvent.click(next);
    const confirm = await screen.findByTestId('breaker-confirm');
    expect(confirm).toHaveTextContent('Plus aucun agent des projets de l’équipe ne démarre sur Claude Code');
    expect(confirm).toHaveTextContent('toute l’équipe Acme');
    await waitFor(() => expect(confirm).toHaveTextContent('Sur 3 postes en ligne, dans la seconde ; les autres à leur retour.'));
    await userEvent.click(within(confirm).getByRole('button', { name: 'Couper' }));
    await waitFor(() =>
      expect(backend.called('team_breaker_set')[0]?.args).toEqual({
        team: 'acme',
        ask: {
          scope: { kind: 'team' },
          what: { kind: 'harness', id: 'claude' },
          reason: 'fuite de clé chez le fournisseur X',
          by: 'Claire Martin',
        },
      }),
    );
    const cut = await screen.findByTestId('breaker-active');
    await userEvent.click(within(cut).getByRole('button', { name: 'Lever' }));
    await waitFor(() =>
      expect(backend.called('team_breaker_lift')[0]?.args).toEqual({ team: 'acme', breaker: CUT.id, by: 'Claire Martin' }),
    );
    expect(await screen.findByText('Aucun coupe-circuit en cours.')).toBeInTheDocument();
  });

  it('sets every agent of one project of the team stopped, by the id of its journal', async () => {
    const backend = fakeBackend({ team_breakers: () => view({}, []), team_breaker_online: () => 0, team_breaker_set: () => view() });
    render(TeamBreakers, { teamId: 'acme', onback: () => {} });
    const form = await screen.findByRole('region', { name: 'Poser un coupe-circuit' });
    await userEvent.click(within(form).getByRole('radio', { name: 'Un projet de l’équipe' }));
    await userEvent.click(within(form).getByRole('radio', { name: 'Tous les agents' }));
    await userEvent.type(within(form).getByRole('textbox', { name: 'Raison' }), 'audit');
    await userEvent.click(within(form).getByRole('button', { name: 'Continuer' }));
    const confirm = await screen.findByTestId('breaker-confirm');
    expect(confirm).toHaveTextContent('comme à la sortie de l’app');
    expect(confirm).toHaveTextContent('le projet API de l’équipe Acme');
    await waitFor(() => expect(confirm).toHaveTextContent('Aucun autre poste n’est en ligne'));
    await userEvent.click(within(confirm).getByRole('button', { name: 'Couper' }));
    await waitFor(() =>
      expect(backend.called('team_breaker_set')[0]?.args.ask).toMatchObject({
        scope: { kind: 'project', project: 'ab'.repeat(16), name: 'API' },
        what: { kind: 'agents' },
      }),
    );
  });
});

describe('what the team’s revocation list withdraws, read by the circuit breaker (TS-14, TS-15)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
    teams.list = [TEAM];
    teams.breakers = {};
  });

  it('shows the plugins it withdraws in every version, with the list’s reason', async () => {
    fakeBackend({
      team_breakers: () => view({ admin: false, withdrawn: [{ id: 'fr.acme.rules', reason: 'fuite de jeton' }] }, []),
    });
    render(TeamBreakers, { teamId: 'acme', onback: () => {} });
    const section = await screen.findByTestId('breaker-withdrawn');
    expect(section).toHaveTextContent('Retirés par la liste de révocation de l’équipe');
    expect(section).toHaveTextContent('Le plugin fr.acme.rules : fuite de jeton');
    expect(screen.getByText('Aucun coupe-circuit en cours.')).toBeInTheDocument();
  });

  it('names the list on the lock of a plugin it withdraws', () => {
    const by = { kind: 'withdrawn' as const, team: 'Acme', reason: 'fuite de jeton' };
    expect(lockLine(by)).toBe('Retiré par la liste de révocation de l’équipe Acme : fuite de jeton');
    const l = {
      locks: {
        'plugins.denied': { key: 'plugins.denied', value: null, by: null, bounds: [], denies: [{ by, denied: ['fr.acme.rules'] }] },
      },
      rows: {},
      warnings: [],
    };
    expect(denial('plugins.denied', 'fr.acme.rules', l)).toBe('retiré par la liste de révocation de l’équipe Acme');
    expect(denial('plugins.denied', 'fr.acme.other', l)).toBeNull();
  });
});
