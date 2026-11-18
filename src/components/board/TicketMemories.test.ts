import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import { entry, provenance } from '../../test/team-memory';
import TicketMemories from './TicketMemories.svelte';

// « Ce ticket a laissé 2 souvenirs · Garder · Écarter » (TS-10): the memories a ticket's agent proposed, on its card to
// validate; each kept or set aside by a human, never by the validation itself.

const team = () => project({ team: 't1', teamProject: '00112233445566778899aabbccddeeff' });

describe('the memories a ticket left (TS-10)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [team()] });
  });

  it('says how many, keeps them all, and is gone once none waits', async () => {
    let left = [
      entry({ id: 'aa'.repeat(16), name: 'un', standing: 'proposed', untrusted: true }),
      entry({
        id: 'bb'.repeat(16),
        name: 'deux',
        standing: 'proposed',
        untrusted: true,
        unknown: ['evil.example'],
        provenance: provenance({ thirdParty: true }),
      }),
    ];
    const b = fakeBackend({
      team_memory_of_ticket: () => left,
      team_memory_decide: (a: { id: string }) => {
        left = left.filter((e) => e.id !== a.id);
        return {};
      },
    });
    render(TicketMemories, { project: team(), ticket: ticket({ key: 'DEM-41', column: 'review' }) });
    const box = await screen.findByTestId('ticket-memories');
    expect(box).toHaveTextContent('Ce ticket a laissé 2 souvenirs');
    expect(b.calls.find((c) => c.cmd === 'team_memory_of_ticket')?.args).toEqual({ projectId: 'p1', key: 'DEM-41' });
    await userEvent.click(screen.getByRole('button', { name: 'Voir' }));
    expect(screen.getAllByTestId('ticket-memory')).toHaveLength(2);
    expect(box).toHaveTextContent('Adresse inconnue du projet : evil.example');
    expect(box).toHaveTextContent('après des données de tiers');
    await userEvent.click(screen.getByTestId('ticket-memories-keep'));
    await waitFor(() => expect(screen.queryByTestId('ticket-memories')).toBeNull());
    expect(b.calls.filter((c) => c.cmd === 'team_memory_decide').map((c) => c.args.keep)).toEqual([true, true]);
  });

  it('sets one aside, and shows nothing for a project of no team', async () => {
    let left = [entry({ id: 'aa'.repeat(16), name: 'un', standing: 'proposed' })];
    const b = fakeBackend({
      team_memory_of_ticket: () => left,
      team_memory_decide: () => {
        left = [];
        return {};
      },
    });
    const { unmount } = render(TicketMemories, { project: team(), ticket: ticket({ key: 'DEM-41', column: 'review' }) });
    await userEvent.click(await screen.findByTestId('ticket-memories-reject'));
    await waitFor(() => expect(b.calls.find((c) => c.cmd === 'team_memory_decide')?.args).toMatchObject({ keep: false }));
    unmount();
    const c = fakeBackend({ team_memory_of_ticket: () => left });
    render(TicketMemories, { project: project(), ticket: ticket({ key: 'DEM-41', column: 'review' }) });
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTestId('ticket-memories')).toBeNull();
    expect(c.calls.some((x) => x.cmd === 'team_memory_of_ticket')).toBe(false);
  });
});
