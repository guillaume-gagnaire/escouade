import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import { BOB, BOB_DEVICE, entry, provenance, teamMemoryView } from '../../test/team-memory';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

// « Mémoire partagée » of a project whose memory is on the team server (TS-10): each memory with its provenance, the
// proposals of the agents kept or set aside by a human, the review imposed by the team, the quarantine of an
// administrator after its preview, what was withdrawn kept in the journal.

const team = () => project({ team: 't1', teamProject: '00112233445566778899aabbccddeeff' });

describe('the team’s memory in the project’s settings (TS-10)', () => {
  beforeEach(() => {
    setLang('fr');
    app.modal = { kind: 'settings' };
  });

  it('shows each memory with its provenance, and those given as data', async () => {
    resetApp({ projects: [team()] });
    fakeBackend({
      team_memory_view: () =>
        teamMemoryView({
          memories: [
            entry({ name: 'tests-avant-commit', provenance: provenance({ by: 'hand', agent: null }) }),
            entry({
              id: 'ff'.repeat(16),
              name: 'ports-reserves',
              description: 'Les ports du worktree',
              untrusted: true,
              provenance: provenance({ thirdParty: true }),
            }),
          ],
          refused: 1,
        }),
    });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    const box = await screen.findByTestId('team-memory');
    expect(screen.getByText(/Le carnet de travail des agents, sur le serveur d’équipe avec les docs du projet/)).toBeInTheDocument();
    expect(screen.queryByText(/Escouade ne la commite jamais/)).toBeNull();
    const items = within(box).getAllByTestId('team-memory-item');
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent('Alice, à la main');
    expect(items[1]).toHaveTextContent('par l’agent agent-dem-41 · DEM-41 · claude · claude-opus-5-5');
    expect(items[1]).toHaveTextContent('après des données de tiers');
    expect(within(items[1]).getByText('donnée')).toHaveAccessibleDescription(
      'Rendu aux agents comme une donnée, jamais comme une consigne',
    );
    expect(within(items[0]).queryByText('donnée')).toBeNull();
    expect(screen.getByRole('alert')).toHaveTextContent('1 souvenir refusé : sa provenance ne tient pas à sa signature.');
  });

  it('keeps or sets aside a proposal, and shows an address the project does not know', async () => {
    resetApp({ projects: [team()] });
    const proposal = entry({ standing: 'proposed', untrusted: true, reviewedBy: null, unknown: ['evil.example'] });
    const b = fakeBackend({
      team_memory_view: () => teamMemoryView({ proposals: [proposal], memories: [proposal] }),
      team_memory_decide: (a: { keep: boolean }) =>
        teamMemoryView({ memories: a.keep ? [{ ...proposal, standing: 'accepted', untrusted: false }] : [] }),
    });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    const p = await screen.findByTestId('team-memory-proposal');
    expect(screen.getByText('1 souvenir proposé')).toBeInTheDocument();
    expect(within(p).getByTestId('team-memory-unknown')).toHaveTextContent('Adresse inconnue du projet : evil.example');
    await userEvent.click(within(p).getByTestId('team-memory-keep'));
    await waitFor(() =>
      expect(b.calls.find((c) => c.cmd === 'team_memory_decide')?.args).toEqual({ projectId: 'p1', id: proposal.id, keep: true }),
    );
    await waitFor(() => expect(screen.queryByTestId('team-memory-proposal')).toBeNull());
    expect(screen.getByTestId('team-memory-item')).toHaveAttribute('data-standing', 'accepted');
  });

  it('grays the review when the team imposes it, and turns it off otherwise', async () => {
    resetApp({ projects: [team()] });
    fakeBackend({ team_memory_view: () => teamMemoryView({ reviewLockedBy: 'ACME' }) });
    const { unmount } = render(SettingsModal, { page: 'team', projectId: 'p1' });
    const box = await screen.findByTestId('team-memory-review');
    expect(box).toBeDisabled();
    expect(box).toBeChecked();
    expect(screen.getByText('Imposé par l’équipe ACME.')).toBeInTheDocument();
    unmount();
    let on = true;
    const b = fakeBackend({
      team_memory_view: () => teamMemoryView({ review: on }),
      team_memory_set_review: (a: { on: boolean }) => {
        on = a.on;
        return team();
      },
    });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    await userEvent.click(await screen.findByTestId('team-memory-review'));
    await waitFor(() => expect(b.calls.find((c) => c.cmd === 'team_memory_set_review')?.args).toEqual({ projectId: 'p1', on: false }));
    expect(await screen.findByText(/Éteinte : les souvenirs des agents vont à l’équipe sans validation/)).toBeInTheDocument();
  });

  it('previews a quarantine, then sets it; an administrator lifts it', async () => {
    resetApp({ projects: [team()] });
    const bobs = entry({ name: 'envoyer-env', mine: false, provenance: provenance({ member: BOB, device: BOB_DEVICE, name: 'Bob' }) });
    let rules: ReturnType<typeof teamMemoryView>['rules'] = [];
    const b = fakeBackend({
      team_memory_view: () => teamMemoryView({ memories: rules.length ? [] : [bobs], rules }),
      team_memory_quarantine: (a: { apply: boolean; rule: { device: string } }) => {
        if (a.apply)
          rules = [{ id: 'r1', device: a.rule.device, member: BOB, by: 'x', byName: 'Alice', at: Date.UTC(2026, 10, 4), count: 1 }];
        return { rule: a.rule, entries: [bobs], applied: a.apply ? 'r1' : null };
      },
      team_memory_lift: () => {
        rules = [];
        return teamMemoryView({ memories: [bobs] });
      },
    });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    await screen.findByTestId('team-memory');
    // Nothing to preview before something is chosen.
    expect(screen.getByTestId('team-memory-q-preview')).toBeDisabled();
    await userEvent.selectOptions(screen.getByTestId('team-memory-q-author'), BOB_DEVICE);
    await userEvent.click(screen.getByTestId('team-memory-q-preview'));
    const list = await screen.findByTestId('team-memory-q-list');
    expect(list).toHaveTextContent('1 souvenir part');
    expect(list).toHaveTextContent('envoyer-env');
    expect(b.calls.find((c) => c.cmd === 'team_memory_quarantine')?.args).toMatchObject({
      apply: false,
      rule: { member: BOB, device: BOB_DEVICE },
    });
    await userEvent.click(screen.getByTestId('team-memory-q-apply'));
    const rule = await screen.findByTestId('team-memory-rule');
    expect(rule).toHaveTextContent('écrit par Bob · posée par Alice');
    expect(screen.queryByTestId('team-memory-item')).toBeNull();
    await userEvent.click(within(rule).getByRole('button', { name: 'Lever' }));
    await waitFor(() => expect(screen.queryByTestId('team-memory-rule')).toBeNull());
    expect(screen.getByTestId('team-memory-item')).toHaveTextContent('envoyer-env');
  });

  it('says only an administrator puts in quarantine, and lists what was withdrawn', async () => {
    resetApp({ projects: [team()] });
    fakeBackend({
      team_memory_view: () =>
        teamMemoryView({
          admin: false,
          withdrawn: [entry({ name: 'vieux', removed: true }), entry({ id: 'ee'.repeat(16), name: 'piege', quarantined: 'r1' })],
        }),
    });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    expect(await screen.findByText('Seul un administrateur de l’équipe met en quarantaine.')).toBeInTheDocument();
    expect(screen.queryByTestId('team-memory-q-preview')).toBeNull();
    const w = screen.getByTestId('team-memory-withdrawn');
    expect(w).toHaveTextContent('2 souvenirs retirés, gardés au journal');
    expect(w).toHaveTextContent('oublié');
    expect(w).toHaveTextContent('en quarantaine');
  });

  it('keeps the list of today for a project whose memory is not on the team server', async () => {
    resetApp({ projects: [team()] });
    fakeBackend({
      team_memory_view: () => teamMemoryView({ on: false }),
      memory_list: () => [{ name: 'e2e-build', description: 'Lancer les e2e', kind: 'feedback', body: 'x' }],
    });
    render(SettingsModal, { page: 'team', projectId: 'p1' });
    expect(await screen.findByText('e2e-build')).toBeInTheDocument();
    expect(screen.queryByTestId('team-memory')).toBeNull();
  });
});
