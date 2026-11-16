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

import { setLang } from '../../lib/i18n';
import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import { assigneeLine, conflictLine, heldElsewhere, holderLine, isTeamBoard, localOnly } from '../../lib/teamboard';
import type { Project, TeamHolder, TeamTicket, Ticket } from '../../lib/types';
import { board, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import TicketCard from './TicketCard.svelte';
import TicketForm from './TicketForm.svelte';

// The Kanban of a team's project (TS-6): the card says who holds a ticket and since when, an interrupted claim and
// « Reprendre », the conflicts this machine lost, « Sur ce poste seulement »; the form's « Ne pas partager ».

const SINCE = new Date(2026, 10, 16, 14, 2).getTime();

function holder(over: Partial<TeamHolder> = {}): TeamHolder {
  return {
    name: 'Alice Martin',
    device: 'aa'.repeat(16),
    mine: false,
    since: SINCE,
    until: SINCE + 600_000,
    interrupted: false,
    unreachableMin: null,
    ...over,
  };
}

function team(over: Partial<TeamTicket> = {}): TeamTicket {
  return {
    object: '01'.repeat(16),
    claim: '02'.repeat(16),
    keyed: true,
    holder: null,
    token: null,
    version: 0,
    assignee: null,
    assigneeName: null,
    assignedMe: false,
    teammate: true,
    conflicts: [],
    dismissed: [],
    refused: null,
    ...over,
  };
}

const teamProject = (): Project =>
  project({ team: 'acme', teamProject: 'ab'.repeat(16), board: board({ autopilot: true, maxParallel: 2 }) });

const ofTeam = (over: Partial<Ticket> = {}, t: Partial<TeamTicket> = {}) =>
  ticket({ id: 't7', key: 'DEM-7', title: 'La prise atomique', team: team(t), ...over });

const show = (t: Ticket, p: Project = teamProject()) =>
  render(TicketCard, { ticket: t, project: p, queueIndex: 0, busyCount: 0, quota: null, onedit: () => {} });

describe('what the card says of a ticket of the team (TS-6)', () => {
  beforeEach(() => {
    cleanup();
    setLang('fr');
    resetApp({ projects: [teamProject()] });
  });

  it('says who holds it and since when, and whether their machine answers', () => {
    expect(holderLine(ofTeam({}, { holder: holder() }))).toMatchObject({
      text: 'Pris par Alice Martin · depuis 14:02',
      tone: 'held',
      unreachable: null,
    });
    expect(holderLine(ofTeam({}, { holder: holder({ unreachableMin: 12 }) }))?.unreachable).toBe('Injoignable depuis 12 min');
    expect(holderLine(ofTeam({}, { holder: holder({ mine: true }) }))).toMatchObject({
      text: 'Pris par ce poste · depuis 14:02',
      tone: 'here',
    });
    expect(holderLine(ofTeam({}, { holder: holder({ name: '' }) }))?.text).toBe('Pris par un collègue · depuis 14:02');
    expect(holderLine(ofTeam())).toBeNull();
    expect(heldElsewhere(ofTeam({}, { holder: holder() }))).toBe(true);
    expect(heldElsewhere(ofTeam({}, { holder: holder({ interrupted: true }) }))).toBe(false);
    setLang('en');
    expect(holderLine(ofTeam({}, { holder: holder() }))?.text).toBe('Taken by Alice Martin · since 2:02 PM');
    setLang('fr');
  });

  it('shows the holder on the card, and no « Lancer » while a teammate holds it', () => {
    fakeBackend();
    const { container } = show(ofTeam({}, { holder: holder({ unreachableMin: 12 }) }));
    const band = container.querySelector('[data-team-holder="held"] .text')!;
    expect(band).toHaveTextContent('Pris par Alice Martin · depuis 14:02 · Injoignable depuis 12 min');
    expect(screen.queryByRole('button', { name: 'Lancer' })).toBeNull();
  });

  it('offers « Lancer » for a ticket of the team the autopilot was not given, and it goes through the claim', async () => {
    const backend = fakeBackend();
    show(ofTeam());
    await userEvent.click(screen.getByRole('button', { name: 'Lancer' }));
    expect(backend.called('ticket_start')).toEqual([{ cmd: 'ticket_start', args: { id: 't7' } }]);
    cleanup();
    // Assigned to me: the autopilot takes it, as one of this machine.
    show(ofTeam({}, { assignedMe: true }));
    expect(screen.queryByRole('button', { name: 'Lancer' })).toBeNull();
  });

  it('leaves « Valider » to the machine that took it', () => {
    fakeBackend();
    const { container } = show(ofTeam({ column: 'review' }));
    expect(container.querySelector('[data-team-away]')).toHaveTextContent('Sur le poste qui l’a pris : il se valide là-bas.');
    expect(screen.queryByRole('button', { name: 'Valider' })).toBeNull();
    cleanup();
    // Started here: this machine's to approve.
    const here = show(ofTeam({ column: 'review', startedAt: 5 }));
    expect(here.container.querySelector('[data-team-away]')).toBeNull();
  });

  it('says an interrupted claim, and « Reprendre » takes it up by hand', async () => {
    const backend = fakeBackend();
    const { container } = show(ofTeam({ column: 'doing' }, { holder: holder({ interrupted: true }) }));
    expect(container.querySelector('[data-team-holder="interrupted"]')).toHaveTextContent(
      'Interrompu chez Alice Martin : son bail a expiré. Le pilote auto ne le reprend pas.',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Reprendre' }));
    expect(backend.called('ticket_team_resume')).toEqual([{ cmd: 'ticket_team_resume', args: { id: 't7' } }]);
  });

  it('shows the conflicts this machine lost, field by field, and settles them', async () => {
    const backend = fakeBackend();
    const conflicts = [
      { field: 'title', by: 'Bob Durand', stamp: 's1', mine: 'Le mien', theirs: 'Le sien', unmerged: false },
      { field: 'description', by: 'Bob Durand', stamp: 's2', mine: 'a', theirs: 'b', unmerged: true },
    ];
    const { container } = show(ofTeam({}, { conflicts }));
    expect(container.querySelector('[data-team-conflict="title"]')).toHaveTextContent('Bob Durand a changé le titre en même temps');
    expect(container.querySelector('[data-team-conflict="description"]')).toHaveTextContent(
      'Bob Durand a changé la description en même temps, sur les mêmes lignes : elles ne se fusionnent pas.',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Reprendre le mien' }));
    await userEvent.click(screen.getByRole('button', { name: 'Prendre le sien' }));
    expect(backend.called('ticket_team_settle').map((c) => c.args)).toEqual([
      { id: 't7', field: 'title', keepMine: true },
      { id: 't7', field: 'description', keepMine: false },
    ]);
    expect(conflictLine(conflicts[0]).theirs).toBe('Garder le sien');
    expect(conflictLine(conflicts[1]).mine).toBe('Garder le mien');
  });

  it('says a move the server refused, and whom it is assigned to', () => {
    fakeBackend();
    const { container } = show(
      ofTeam(
        {},
        {
          refused: 'Repris par Bob Durand pendant une coupure : ton déplacement n’est pas parti.',
          assignee: 'cc'.repeat(16),
          assigneeName: 'Bob Durand',
        },
      ),
    );
    expect(container.querySelector('[data-team-refused]')).toHaveTextContent('Repris par Bob Durand pendant une coupure');
    expect(container.querySelector('[data-team-assignee]')).toHaveTextContent('Assigné à Bob Durand');
    expect(assigneeLine(ofTeam({}, { assignee: 'cc'.repeat(16), assignedMe: true }))).toBe('Assigné à moi');
    expect(assigneeLine(ofTeam())).toBeNull();
  });

  it('marks a ticket kept on this machine, and its menu offers to share it', async () => {
    const backend = fakeBackend();
    const home = ticket({ id: 't8', key: 'DEM-L8', title: 'Brouillon', teamLocal: true });
    expect(localOnly(home, teamProject())).toBe(true);
    expect(localOnly(home, project())).toBe(false);
    const { container } = show(home);
    expect(container.querySelector('[data-team-local]')).toHaveTextContent('Sur ce poste seulement');
    const card = container.querySelector('.card') as HTMLElement;
    await userEvent.pointer({ keys: '[MouseRight]', target: card });
    const share = menu.open!.items.find((i) => i.label === 'Partager avec l’équipe');
    expect(share).toBeTruthy();
    share!.onClick!();
    expect(backend.called('ticket_team_local')).toEqual([{ cmd: 'ticket_team_local', args: { id: 't8', local: false } }]);
  });

  it('offers « Me l’assigner » and « Ne pas partager » in the menu of a ticket of the team', async () => {
    const backend = fakeBackend();
    const { container } = show(ofTeam());
    await userEvent.pointer({ keys: '[MouseRight]', target: container.querySelector('.card') as HTMLElement });
    menu.open!.items.find((i) => i.label === 'Me l’assigner')!.onClick!();
    menu.open!.items.find((i) => i.label === 'Ne pas partager')!.onClick!();
    expect(backend.called('ticket_team_assign')).toEqual([{ cmd: 'ticket_team_assign', args: { id: 't7', me: true } }]);
    expect(backend.called('ticket_team_local')).toEqual([{ cmd: 'ticket_team_local', args: { id: 't7', local: true } }]);
    expect(menu.open!.items.some((i) => i.label === 'Partager avec l’équipe' && !i.separator)).toBe(false);
  });
});

describe('the form of a ticket of a team’s Kanban (TS-6)', () => {
  beforeEach(() => {
    cleanup();
    setLang('fr');
    resetApp({ projects: [teamProject()] });
  });

  it('offers « Ne pas partager » for a new ticket, never the file of ESC-190', async () => {
    fakeBackend();
    const onsubmit = vi.fn(() => true);
    render(TicketForm, { projectId: 'p1', onsubmit });
    expect(screen.queryByRole('switch', { name: 'Partager avec l’équipe' })).toBeNull();
    const sw = screen.getByRole('switch', { name: 'Ne pas partager' });
    expect(sw).toHaveAttribute('aria-checked', 'false');
    await userEvent.type(screen.getByRole('textbox', { name: /Titre/ }), 'Un brouillon');
    await userEvent.click(sw);
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }));
    const sent = (onsubmit.mock.calls as unknown as [Record<string, unknown>][])[0][0];
    expect(sent).toMatchObject({ title: 'Un brouillon', teamLocal: true });
    expect(sent).not.toHaveProperty('shared');
    expect(isTeamBoard(app.projects[0])).toBe(true);
  });

  it('says a ticket of the team is the team’s', () => {
    fakeBackend();
    render(TicketForm, { projectId: 'p1', ticket: ofTeam(), onsubmit: () => true });
    expect(screen.queryByRole('switch', { name: 'Ne pas partager' })).toBeNull();
    expect(screen.getByText('Ticket de l’équipe')).toBeInTheDocument();
  });
});
