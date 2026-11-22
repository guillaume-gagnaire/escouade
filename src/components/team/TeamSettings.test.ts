import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { file, settings, version } from '../../test/team-settings';
import { app } from '../../lib/state.svelte';
import { teams, type TeamView } from '../../lib/teams.svelte';
import type { TeamSettingsView } from '../../lib/team-settings';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import TeamSettings from './TeamSettings.svelte';
import TeamSettingsBand from './TeamSettingsBand.svelte';

const TEAM: TeamView = {
  id: 'acme',
  name: 'Acme',
  server: 'team.acme.fr',
  joined: true,
  role: 'member',
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
  snapshotAlerts: [],
  lastError: null,
};

const V1 = version(1, file({ enforced: { 'telemetry.enabled': false } }));
const V2 = version(2, file({ enforced: { 'telemetry.enabled': false, 'permissions.modes': ['plan', 'default'] } }), {
  status: { kind: 'applied' },
  note: 'Pas de mode auto.',
});

function member(over: Partial<TeamSettingsView> = {}): TeamSettingsView {
  return settings([V2, V1], over, {
    quorum: true,
    admins: 2,
    effective: {
      defaults: {},
      enforced: { 'telemetry.enabled': false, 'permissions.modes': ['plan', 'default'] },
      accepted: {},
      unknown: [],
      reasons: { 'permissions.modes': 'Pas de mode auto.' },
      pending: [
        { part: 'enforced', key: 'proxy.authorities', value: '/etc/acme/ca.pem', sort: 'sensitive', fingerprint: 'aa'.repeat(32) },
        { part: 'proposed', key: 'worktrees.setup', value: ['npm ci'], sort: 'executable', fingerprint: 'bb'.repeat(32) },
      ],
    },
  });
}

describe('the team’s settings on a member’s device (TS-13)', () => {
  beforeEach(() => {
    resetApp({ projects: [project({ id: 'p1', name: 'Site', team: 'acme' }), project({ id: 'p2', name: 'Perso' })] });
    teams.list = [TEAM];
    teams.settings = {};
  });

  it('shows the version in force, what it imposes, the history with each change, and marks it seen', async () => {
    const backend = fakeBackend({ team_settings: () => member(), team_settings_seen: () => null });
    render(TeamSettings, { teamId: 'acme', onback: () => {} });
    expect(await screen.findByTestId('team-settings-force')).toHaveTextContent('Version 2 · signée par Claire Martin');
    const imposed = screen.getByRole('region', { name: 'Ce que l’équipe impose' });
    expect(within(imposed).getByText('permissions.modes')).toBeInTheDocument();
    expect(within(imposed).getByText('Pas de mode auto.')).toBeInTheDocument();
    expect(screen.getByText(/Quorum : deux administrateurs/)).toBeInTheDocument();
    await waitFor(() => expect(backend.called('team_settings_seen')[0]?.args).toEqual({ team: 'acme', number: 2 }));
    // The history: each version, its status, its change.
    const history = screen.getByTestId('team-settings-history');
    expect(within(history).getByText('En vigueur')).toBeInTheDocument();
    expect(within(history).getByText('Remplacée')).toBeInTheDocument();
    await userEvent.click(within(history).getAllByRole('button', { name: 'Voir le changement' })[0]);
    expect(within(history).getByTestId('team-settings-diff')).toHaveTextContent('permissions.modes');
    expect(within(history).getByTestId('team-settings-diff')).toHaveTextContent('ajouté : ["plan","default"]');
    // Its projects on this device: the team's ticked, the others not.
    expect(screen.getByRole('checkbox', { name: 'Site' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Perso' })).not.toBeChecked();
    // A member proposes nothing.
    expect(screen.queryByRole('region', { name: 'Proposer une version' })).toBeNull();
  });

  it('asks the consent of a sensitive key with what it allows, and reads a proposed command at its fingerprint', async () => {
    let view = member();
    const backend = fakeBackend({
      team_settings: () => view,
      team_settings_seen: () => null,
      team_settings_consent: (a: { key: string }) => {
        view = {
          ...view,
          view: {
            ...view.view,
            effective: { ...view.view.effective, pending: view.view.effective.pending.filter((p) => p.key !== a.key) },
          },
        };
        return view;
      },
    });
    render(TeamSettings, { teamId: 'acme', onback: () => {} });
    const asks = await screen.findAllByTestId('team-consent');
    expect(asks).toHaveLength(2);
    expect(asks[0]).toHaveTextContent('Ce certificat permet de lire le trafic TLS d’Escouade.');
    expect(asks[1]).toHaveTextContent('npm ci');
    expect(asks[1]).toHaveTextContent('Empreinte bbbbbbbbbbbb');
    await userEvent.click(within(asks[0]).getByRole('button', { name: 'Accepter sur ce poste' }));
    expect(backend.called('team_settings_consent')[0].args).toEqual({
      team: 'acme',
      part: 'enforced',
      key: 'proxy.authorities',
      fingerprint: 'aa'.repeat(32),
    });
    await waitFor(() => expect(screen.getAllByTestId('team-consent')).toHaveLength(1));
    await userEvent.click(screen.getByRole('button', { name: 'J’ai relu, accepter' }));
    expect(backend.called('team_settings_consent')[1].args.fingerprint).toBe('bb'.repeat(32));
  });

  it('lets an administrator read a version with its diff, sign it, and cosign another’s', async () => {
    const proposal = version(3, file({ enforced: { 'proxy.url': 'http://p' } }), {
      status: { kind: 'proposed' },
      need: 2,
      by: 'Bob',
      digest: 'c'.repeat(64),
    });
    const admin = settings(
      [proposal, V2, V1],
      { admin: true, signerName: 'Alice Martin', yaml: 'version: 1\n' },
      { quorum: true, admins: 2 },
    );
    const backend = fakeBackend({
      team_settings: () => admin,
      team_settings_seen: () => null,
      team_settings_check: () => ({
        file: file({ enforced: { 'telemetry.enabled': false, 'permissions.modes': ['plan'], 'proxy.url': 'http://p' } }),
        warnings: [],
        error: null,
        need: 2,
      }),
      team_settings_propose: () => admin,
      team_settings_cosign: () => admin,
    });
    render(TeamSettings, { teamId: 'acme', onback: () => {} });
    const propose = await screen.findByRole('region', { name: 'Proposer une version' });
    await userEvent.click(within(propose).getByRole('button', { name: 'Relire le changement' }));
    const diff = await within(propose).findByTestId('team-settings-diff');
    expect(diff).toHaveTextContent('changé : ["plan","default"] → ["plan"]');
    expect(diff).toHaveTextContent('proxy.url');
    expect(within(propose).getByText(/attendra la signature d’un second administrateur/)).toBeInTheDocument();
    await userEvent.click(within(propose).getByRole('button', { name: 'Signer et publier' }));
    expect(backend.called('team_settings_propose')[0].args).toMatchObject({ team: 'acme', by: 'Alice Martin' });
    // Bob's version waits for a second administrator: Alice cosigns it.
    await userEvent.click(screen.getByRole('button', { name: 'Cosigner' }));
    expect(backend.called('team_settings_cosign')[0].args).toEqual({ team: 'acme', digest: 'c'.repeat(64) });
  });

  it('refuses to sign what never enters, and says why', async () => {
    const admin = settings([V2, V1], { admin: true, signerName: 'Alice' });
    fakeBackend({
      team_settings: () => admin,
      team_settings_seen: () => null,
      team_settings_check: () => ({
        file: null,
        warnings: [],
        error: 'enforced.permissions.modes : le mode bypassPermissions n’entre jamais dans le fichier d’équipe, même signé.',
        need: 1,
      }),
    });
    render(TeamSettings, { teamId: 'acme', onback: () => {} });
    const propose = await screen.findByRole('region', { name: 'Proposer une version' });
    await userEvent.click(within(propose).getByRole('button', { name: 'Relire le changement' }));
    expect(await within(propose).findByRole('alert')).toHaveTextContent('bypassPermissions n’entre jamais');
    expect(within(propose).queryByRole('button', { name: 'Signer et publier' })).toBeNull();
  });
});

describe('the band of the team’s settings', () => {
  beforeEach(() => {
    resetApp();
    teams.list = [TEAM];
    teams.settings = {};
    app.modal = null;
  });

  it('says the change, who signed it, and opens the settings at « Voir »', async () => {
    const backend = fakeBackend({
      team_list: () => [TEAM],
      team_settings: () => settings([V2, V1], { seen: 1 }),
      team_settings_seen: () => null,
    });
    render(TeamSettingsBand);
    const band = await screen.findByTestId('team-settings-band');
    expect(band).toHaveTextContent(/^L’équipe Acme a changé 1 réglage · Claire Martin, /);
    await userEvent.click(within(band).getByRole('button', { name: 'Voir' }));
    expect(app.modal).toEqual({ kind: 'team', step: 'settings', teamId: 'acme' });
    expect(backend.called('team_settings_seen')[0].args).toEqual({ team: 'acme', number: 2 });
  });

  it('is not there when the member saw the version in force', async () => {
    fakeBackend({ team_list: () => [TEAM], team_settings: () => settings([V2, V1], { seen: 2 }) });
    render(TeamSettingsBand);
    await waitFor(() => expect(teams.settings.acme).toBeDefined());
    expect(screen.queryByTestId('team-settings-band')).toBeNull();
  });
});
