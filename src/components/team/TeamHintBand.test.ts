import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { hintBand } from '../../lib/teamboard';
import { teams, type TeamView } from '../../lib/teams.svelte';
import type { Project, TeamHint } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import TeamHintBand from './TeamHintBand.svelte';

// The band of the team a repository names (TS-7): « demander une invitation » on a device of no such team (it opens the
// screen of the entry, the address filled in, nothing else: no call goes to that address), « Rattacher à l'équipe » on
// a device of it, never on a project attached, and « Ne plus proposer » until the file names another team.

const TEAM = '0123456789abcdef0123456789abcdef';
const hint = (over: Partial<TeamHint> = {}): TeamHint => ({
  server: 'https://team.acme.fr',
  host: 'team.acme.fr',
  team: TEAM,
  project: null,
  name: 'Acme',
  fingerprint: 'f1',
  ...over,
});
const named = (h: TeamHint | null, over: Partial<Project> = {}): Project =>
  project({ shared: { local: [], trusted: '', file: null, team: h }, ...over });
const team = (over: Partial<TeamView> = {}) => ({ id: TEAM, name: 'ACME', server: 'team.acme.fr', joined: true, ...over }) as TeamView;

describe('the band of the team a repository names (TS-7)', () => {
  beforeEach(() => {
    setLang('fr');
    app.modal = null;
  });

  it('says what to offer, from the file and the teams of this device', () => {
    expect(hintBand(named(null), [])).toBeNull();
    expect(hintBand(named(hint()), [])?.kind).toBe('invite');
    expect(hintBand(named(hint()), [team()])).toMatchObject({ kind: 'attach', teamId: TEAM });
    // Attached already: none.
    expect(hintBand(named(hint(), { team: TEAM }), [team()])).toBeNull();
    // The same id on another server, or a team not joined yet: an invitation, never « Rattacher ».
    expect(hintBand(named(hint({ host: 'team.evil.fr' })), [team()])?.kind).toBe('invite');
    expect(hintBand(named(hint()), [team({ joined: false })])?.kind).toBe('invite');
    // Another team than the device's.
    expect(hintBand(named(hint({ team: 'ff'.repeat(16) })), [team()])?.kind).toBe('invite');
    // Dismissed: until the file names another team.
    const dismissed = named(hint(), {});
    dismissed.shared!.teamDismissed = 'f1';
    expect(hintBand(dismissed, [])).toBeNull();
    dismissed.shared!.team = hint({ fingerprint: 'f2', host: 'other.fr' });
    expect(hintBand(dismissed, [])?.kind).toBe('invite');
  });

  it('asks for an invitation without a call to the address of the file', async () => {
    const b = fakeBackend({ team_list: () => [] });
    resetApp({ projects: [named(hint({ name: 'Acme Corp' }))] });
    render(TeamHintBand);
    const band = await screen.findByTestId('team-hint-band');
    expect(band).toHaveTextContent('Ce projet appartient à l’équipe Acme Corp sur team.acme.fr, d’après le dépôt.');
    expect(band).toHaveTextContent('Escouade ne joint ce serveur qu’à ton geste');
    await userEvent.click(screen.getByRole('button', { name: 'Demander une invitation' }));
    expect(app.modal).toEqual({ kind: 'team', step: 'join', expected: 'team.acme.fr' });
    // Nothing but the list of this device's teams was asked.
    expect(b.calls.map((c) => c.cmd).filter((c) => c !== 'team_list')).toEqual([]);
  });

  it('attaches the project on a device of the team, and « Ne plus proposer » dismisses it', async () => {
    const b = fakeBackend({
      team_list: () => [team()],
      project_set_team: () => named(hint(), { team: TEAM }),
      project_team_hint_dismiss: () => named(hint(), { shared: { local: [], trusted: '', file: null, team: hint(), teamDismissed: 'f1' } }),
    });
    resetApp({ projects: [named(hint())] });
    render(TeamHintBand);
    await waitFor(() => expect(screen.getByTestId('team-hint-band')).toHaveAttribute('data-kind', 'attach'));
    expect(screen.getByTestId('team-hint-band')).toHaveTextContent('Ce projet est de ton équipe Acme, d’après le dépôt.');
    await userEvent.click(screen.getByRole('button', { name: 'Rattacher à l’équipe' }));
    expect(b.calls.find((c) => c.cmd === 'project_set_team')?.args).toEqual({ project: 'p1', team: TEAM });
    await userEvent.click(screen.getByRole('button', { name: 'Ne plus proposer pour ce projet' }));
    expect(b.calls.find((c) => c.cmd === 'project_team_hint_dismiss')?.args).toEqual({ project: 'p1' });
  });
});
