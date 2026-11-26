import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app } from '../../lib/state.svelte';
import { teams, type MachineView, type TeamView } from '../../lib/teams.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import TeamModal from './TeamModal.svelte';

const SIX = ['alpha', 'bravo', 'charlie', 'delta', 'echo', 'foxtrot'];
const MACHINE: MachineView = { id: '7f'.repeat(16), devices: ['ab'.repeat(16)], me: false };

function team(over: Partial<TeamView> = {}): TeamView {
  return {
    id: '00112233445566778899aabbccddeeff',
    name: 'ACME',
    server: 'team.acme.fr',
    joined: true,
    role: 'admin',
    admins: 1,
    members: 2,
    machines: [MACHINE],
    epoch: 3,
    fingerprint: SIX,
    head: SIX,
    escrow: null,
    keysInFile: false,
    recoveryConfirmed: true,
    wordsPending: false,
    recoveryOutdated: false,
    recoveryStale: false,
    alarm: null,
    invitations: [],
    snapshotAlerts: [],
    dropAlerts: [],
    nullAlerts: [],
    admissions: [],
    refusals: [],
    joinWords: [],
    lastError: null,
    ...over,
  };
}

const PAIR = { code: 'K7Q4M-2XZ8P', server: 'team.acme.fr', expiresAt: 1_900_000_000, state: 'waiting' };

describe('« Machines de l’équipe » (OE-1)', () => {
  beforeEach(() => {
    resetApp();
    teams.list = [];
    teams.loaded = false;
    app.modal = { kind: 'team' };
  });

  it('ranks the machines apart from the people, says what they can and cannot sign, and lets an administrator remove one', async () => {
    const backend = fakeBackend({
      team_list: () => [team()],
      team_machine_remove: () => team({ machines: [] }),
    });
    render(TeamModal);
    const card = await screen.findByRole('region', { name: 'ACME' });
    expect(within(card).getByText('2 membres')).toBeInTheDocument();
    const machines = within(card).getByRole('group', { name: 'Machines de l’équipe' });
    expect(within(machines).getByText('Machine 7f7f7f7f')).toBeInTheDocument();
    expect(within(machines).getByText('1 appareil')).toBeInTheDocument();
    expect(within(machines).getByText(/Rattachées à l’équipe, pas à une personne/)).toBeInTheDocument();
    expect(within(machines).getByText(/prendre des tickets/)).toBeInTheDocument();
    expect(within(machines).getByText(/un membre, un rôle, un réglage de l’équipe, un coupe-circuit, un plugin privé/)).toBeInTheDocument();
    await userEvent.click(within(machines).getByRole('button', { name: 'Retirer' }));
    const confirm = within(machines).getByRole('alertdialog');
    expect(confirm).toHaveTextContent('Retirer la machine 7f7f7f7f de l’équipe ?');
    await userEvent.click(within(confirm).getByRole('button', { name: 'Retirer' }));
    expect(backend.called('team_machine_remove')[0].args).toEqual({ team: team().id, machine: MACHINE.id });
    expect(await within(card).findByText('Aucune machine dans cette équipe.')).toBeInTheDocument();
  });

  it('shows a member the machines without the gestures of an administrator', async () => {
    fakeBackend({ team_list: () => [team({ role: 'member' })] });
    render(TeamModal);
    const machines = await screen.findByRole('group', { name: 'Machines de l’équipe' });
    expect(within(machines).getByText('Machine 7f7f7f7f')).toBeInTheDocument();
    expect(within(machines).queryByRole('button', { name: 'Retirer' })).toBeNull();
    expect(within(machines).queryByRole('button', { name: 'Inscrire une machine de l’équipe' })).toBeNull();
  });

  it('says on the machine itself that it is the team’s, and hides the section from a member of a team without one', async () => {
    fakeBackend({
      team_list: () => [
        team({ role: 'service', machines: [{ ...MACHINE, me: true }] }),
        team({ id: 'ff'.repeat(16), name: 'Autre', role: 'member', machines: [] }),
      ],
    });
    render(TeamModal);
    const card = await screen.findByRole('region', { name: 'ACME' });
    expect(within(card).getAllByText('Machine de l’équipe')[0]).toBeInTheDocument();
    expect(within(card).getByText('cette machine')).toBeInTheDocument();
    const other = screen.getByRole('region', { name: 'Autre' });
    expect(within(other).queryByRole('group', { name: 'Machines de l’équipe' })).toBeNull();
  });

  it('enrolls a machine by a code under the role of service, with the command to type on it', async () => {
    const backend = fakeBackend({
      team_list: () => [team({ machines: [] })],
      team_pair_start: () => PAIR,
      team_pair_step: () => PAIR,
      team_pair_cancel: () => null,
    });
    render(TeamModal);
    const machines = await screen.findByRole('group', { name: 'Machines de l’équipe' });
    await userEvent.click(within(machines).getByRole('button', { name: 'Inscrire une machine de l’équipe' }));
    expect(await screen.findByTestId('machine-code')).toHaveTextContent('K7Q4M-2XZ8P');
    expect(backend.called('team_pair_start')[0].args).toEqual({ team: team().id, service: true });
    expect(screen.getByTestId('machine-command')).toHaveTextContent('escouade-server team pair team.acme.fr K7Q4M-2XZ8P');
    expect(screen.getByText(/sous le rôle de service/)).toBeInTheDocument();
  });
});
