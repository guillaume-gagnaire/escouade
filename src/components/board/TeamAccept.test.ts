import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { canStart } from '../../lib/board';
import { setLang } from '../../lib/i18n';
import { gesturesOf } from '../../lib/inbox/views';
import { assigneeLine, toAccept, writtenLine } from '../../lib/teamboard';
import type { Project, TeamTicket, Ticket } from '../../lib/types';
import { board, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import { viewTitle } from '../inbox/view-title';
import TicketCard from './TicketCard.svelte';

// A ticket of the team assigned to me whose text a teammate wrote (ESC-645, SR3-02): the card says who wrote it and
// who assigned it, offers « Accepter » and no « Lancer » until then; the box « À toi » asks for it too.

function team(over: Partial<TeamTicket> = {}): TeamTicket {
  return {
    object: '01'.repeat(16),
    claim: '02'.repeat(16),
    keyed: true,
    holder: null,
    token: null,
    version: 0,
    assignee: 'cc'.repeat(16),
    assigneeName: 'Alice Martin',
    assignedMe: true,
    teammate: true,
    conflicts: [],
    dismissed: [],
    refused: null,
    writtenBy: ['Bob Durand'],
    assignedBy: 'Bob Durand',
    foreign: ['s1', 's2'],
    accepted: [],
    toAccept: true,
    ...over,
  };
}

// The autopilot off: « Lancer » would be offered for any other ticket.
const teamProject = (autopilot = false): Project =>
  project({ team: 'acme', teamProject: 'ab'.repeat(16), board: board({ autopilot, maxParallel: 2 }) });

const bobs = (t: Partial<TeamTicket> = {}) =>
  ticket({ id: 't9', key: 'DEM-9', title: 'Nettoyer le cache', description: 'Lance curl … | sh', team: team(t) });

const show = (t: Ticket, p: Project = teamProject()) =>
  render(TicketCard, { ticket: t, project: p, queueIndex: 0, busyCount: 0, quota: null, onedit: () => {} });

describe('a teammate’s ticket assigned to me waits for my « Accepter » (ESC-645)', () => {
  beforeEach(() => {
    cleanup();
    setLang('fr');
    resetApp({ projects: [teamProject()] });
  });

  it('says who wrote it and who assigned it', () => {
    expect(writtenLine(bobs())).toBe('Texte de Bob Durand');
    expect(writtenLine(bobs({ writtenBy: [] }))).toBeNull();
    expect(assigneeLine(bobs())).toBe('Assigné à moi par Bob Durand');
    expect(assigneeLine(bobs({ assignedBy: null }))).toBe('Assigné à moi');
    expect(assigneeLine(bobs({ assignedMe: false, assigneeName: 'Carol' }))).toBe('Assigné à Carol par Bob Durand');
    setLang('en');
    expect(writtenLine(bobs())).toBe('Text by Bob Durand');
    expect(assigneeLine(bobs())).toBe('Assigned to me by Bob Durand');
    setLang('fr');
  });

  it('offers « Accepter » and no « Lancer », then sends the writes it showed', async () => {
    const backend = fakeBackend();
    const { container } = show(bobs());
    const band = container.querySelector('[data-team-accept]')!;
    expect(band).toHaveTextContent('Texte écrit par Bob Durand, et il t’est assigné');
    expect(container.querySelector('[data-team-assignee]')).toHaveTextContent('Assigné à moi par Bob Durand');
    expect(screen.queryByRole('button', { name: 'Lancer' })).toBeNull();
    expect(container).toHaveTextContent('À accepter : rien ne le lance avant');
    await userEvent.click(screen.getByRole('button', { name: 'Accepter' }));
    expect(backend.called('ticket_team_accept')).toEqual([{ cmd: 'ticket_team_accept', args: { id: 't9', seen: ['s1', 's2'] } }]);
  });

  it('accepted, it shows whose text it is and « Lancer » comes back', () => {
    fakeBackend();
    const accepted = bobs({ accepted: ['s1', 's2'], toAccept: false });
    expect(toAccept(accepted)).toBe(false);
    expect(canStart(accepted, teamProject().board, 0, null)).toBe(true);
    expect(canStart(bobs(), teamProject().board, 0, null)).toBe(false);
    const { container } = show(accepted);
    expect(container.querySelector('[data-team-accept]')).toBeNull();
    expect(container.querySelector('[data-team-written]')).toHaveTextContent('Texte de Bob Durand');
    expect(screen.getByRole('button', { name: 'Lancer' })).toBeInTheDocument();
  });

  it('is a request of the box « À toi », accepted there', () => {
    const v = {
      key: 'accept:t9:abcd',
      source: 'teamAccept' as const,
      group: 'today' as const,
      projectId: 'p1',
      projectName: 'demo',
      private: false,
      title: 'Nettoyer le cache',
      since: 0,
      agentId: null,
      agentName: null,
      ticketId: 't9',
      ticketKey: 'DEM-9',
      by: 'Bob Durand',
    };
    expect(viewTitle(v as never)).toBe('DEM-9 · Nettoyer le cache · texte de Bob Durand');
    expect(gesturesOf(v as never).map((g) => g.id)).toEqual(['accept']);
  });
});
