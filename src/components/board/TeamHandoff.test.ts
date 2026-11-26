// Passing an agent (TS-17): the window of the passage says what goes before the gesture (the branch pushed and where,
// the turn interrupted, the uncommitted work as a commit « en cours » or an encrypted patch, the conversation encrypted
// on the server), lists the teammates and the team's machines, and says a refusal with nothing to do; the card says
// « Alice te passe DEM-41 · Prendre » or « Passé à Bob · Annuler le passage »; the box « À toi » offers « Prendre »; a
// teammate without the agent's harness picks theirs.
import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));
vi.mock('../../lib/previews/pdf', () => ({ PdfPasswordError: class extends Error {}, openPdf: async () => null }));

import { setLang } from '../../lib/i18n';
import { gesturesOf, type InboxItem } from '../../lib/inbox/views';
import { app } from '../../lib/state.svelte';
import { handoffLine, handoffOffered } from '../../lib/teamboard';
import type { HandoffPlan, Project, TeamHandoff, TeamTicket, Ticket } from '../../lib/types';
import { board, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import InboxViewPane from '../inbox/InboxViewPane.svelte';
import { viewTitle } from '../inbox/view-title';
import TeamHandoffModal from './TeamHandoffModal.svelte';
import TeamHandoffTakeModal from './TeamHandoffTakeModal.svelte';
import TeamTicketBand from './TeamTicketBand.svelte';

const teamProject = (): Project =>
  project({ id: 'p1', team: 'acme', teamProject: 'ab'.repeat(16), board: board({ autopilot: true, maxParallel: 2 }) });

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
    teammate: false,
    conflicts: [],
    dismissed: [],
    refused: null,
    ...over,
  };
}

function handoff(over: Partial<TeamHandoff> = {}): TeamHandoff {
  return {
    object: '03'.repeat(16),
    fromName: 'Alice Martin',
    fromDevice: 'aa'.repeat(16),
    toName: 'Bob Durand',
    toDevice: 'bb'.repeat(16),
    incoming: true,
    outgoing: false,
    machine: false,
    branch: 'ticket/dem-41',
    remote: 'origin',
    patch: true,
    engine: 'claude',
    agent: 'DEM-41',
    at: 1,
    agentId: null,
    ...over,
  };
}

const ofTeam = (t: Partial<TeamTicket> = {}, over: Partial<Ticket> = {}) =>
  ticket({ id: 't41', projectId: 'p1', key: 'DEM-41', title: 'La passation', column: 'doing', team: team(t), ...over });

function plan(over: Partial<HandoffPlan> = {}): HandoffPlan {
  return {
    ticketId: 't41',
    key: 'DEM-41',
    title: 'La passation',
    agentId: 'a1',
    agentName: 'DEM-41',
    engine: 'claude',
    branch: 'ticket/dem-41',
    remote: 'origin',
    dirty: [' M src/app.ts', '?? src/passation.ts'],
    running: true,
    session: false,
    targets: [
      { device: 'bb'.repeat(16), name: 'Bob Durand', machine: false, mine: false },
      { device: 'cc'.repeat(16), name: 'serveur-acme', machine: true, mine: false },
    ],
    refusal: null,
    ...over,
  };
}

describe('passing an agent (TS-17)', () => {
  beforeEach(() => {
    cleanup();
    setLang('fr');
    resetApp({ projects: [teamProject()] });
  });

  it('says on the card who passes it, and to whom', () => {
    expect(handoffLine(ofTeam({ handoff: handoff() }))).toEqual({ text: 'Alice Martin te passe DEM-41', tone: 'incoming' });
    expect(handoffLine(ofTeam({ handoff: handoff({ incoming: false, outgoing: true }) }))).toEqual({
      text: 'Passé à Bob Durand · en attente qu’il le prenne',
      tone: 'outgoing',
    });
    expect(handoffLine(ofTeam({ handoff: handoff({ incoming: false, outgoing: true, machine: true, toName: 'serveur' }) }))?.text).toBe(
      'Passé à la machine serveur · en attente qu’il le prenne',
    );
    expect(handoffLine(ofTeam({ handoff: handoff({ incoming: false }) }))?.tone).toBe('between');
    expect(handoffLine(ofTeam())).toBeNull();
    setLang('en');
    expect(handoffLine(ofTeam({ handoff: handoff() }))?.text).toBe('Alice Martin passes you DEM-41');
  });

  it('is offered from the card of a ticket this machine holds, with its agent, of a team project only', () => {
    const mine = { name: 'Bob', device: 'bb', mine: true, since: 0, until: 0, interrupted: false, unreachableMin: null };
    const held = ofTeam({ holder: mine, token: 3 }, { agentId: 'a1' });
    expect(handoffOffered(held, teamProject())).toBe(true);
    expect(handoffOffered(held, project({ id: 'p1' }))).toBe(false);
    expect(handoffOffered({ ...held, agentId: null }, teamProject())).toBe(false);
    expect(handoffOffered(ofTeam({ holder: { ...mine, mine: false } }, { agentId: 'a1' }), teamProject())).toBe(false);
    expect(handoffOffered(ofTeam({ holder: mine, handoff: handoff() }, { agentId: 'a1' }), teamProject())).toBe(false);
  });

  it('« Prendre » on the card takes it on the agent’s harness; « Annuler le passage » takes it back', async () => {
    const backend = fakeBackend({ ticket_team_handoff_engines: () => ({ preferred: 'claude', engines: ['claude'] }) });
    const { container } = render(TeamTicketBand, {
      ticket: ofTeam({
        handoff: handoff(),
        holder: { name: 'Bob', device: 'bb', mine: true, since: 0, until: 0, interrupted: false, unreachableMin: null },
      }),
      project: teamProject(),
    });
    expect(container.querySelector('[data-team-handoff="incoming"]')).toHaveTextContent('Alice Martin te passe DEM-41');
    // The claim the server handed says nothing of who works on it: no holder line meanwhile.
    expect(container.querySelector('[data-team-holder]')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Prendre' }));
    await waitFor(() =>
      expect(backend.called('ticket_team_handoff_take')).toEqual([
        { cmd: 'ticket_team_handoff_take', args: { id: 't41', engine: 'claude' } },
      ]),
    );
    cleanup();
    render(TeamTicketBand, { ticket: ofTeam({ handoff: handoff({ incoming: false, outgoing: true }) }), project: teamProject() });
    await userEvent.click(screen.getByRole('button', { name: 'Annuler le passage' }));
    expect(backend.called('ticket_team_handoff_cancel')).toEqual([{ cmd: 'ticket_team_handoff_cancel', args: { id: 't41' } }]);
  });

  it('a teammate without the agent’s harness picks theirs: never the account that passes', async () => {
    fakeBackend({ ticket_team_handoff_engines: () => ({ preferred: null, engines: ['codex', 'opencode'] }) });
    render(TeamTicketBand, { ticket: ofTeam({ handoff: handoff() }), project: teamProject() });
    await userEvent.click(screen.getByRole('button', { name: 'Prendre' }));
    await waitFor(() => expect(app.modal?.kind).toBe('teamHandoffTake'));
    const modal = app.modal as Extract<typeof app.modal, { kind: 'teamHandoffTake' }>;
    expect(modal.engines).toEqual(['codex', 'opencode']);
    const picked = vi.fn();
    cleanup();
    app.tickets = { t41: ofTeam({ handoff: handoff() }) };
    render(TeamHandoffTakeModal, { ticketId: 't41', engines: ['codex', 'opencode'], onPick: picked });
    expect(
      screen.getByText('Claude Code n’est pas sur ce poste pour ce projet : l’agent repart sur le tien, avec la passation.'),
    ).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: /Codex/ }));
    expect(picked).toHaveBeenCalledWith('codex');
  });

  it('the window says what goes before the gesture, and passes to the one chosen', async () => {
    const backend = fakeBackend({ ticket_team_handoff_plan: () => plan() });
    render(TeamHandoffModal, { ticketId: 't41' });
    await screen.findByText('Passer DEM-41');
    expect(screen.getByTestId('team-handoff-push')).toHaveTextContent('La branche ticket/dem-41 sera poussée sur origin.');
    expect(screen.getByTestId('team-handoff-push')).toHaveTextContent('Rien n’est poussé avant « Passer ».');
    expect(screen.getByTestId('team-handoff-turn')).toHaveTextContent('son tour sera interrompu avant de passer');
    expect(screen.getByText('2 fichiers non commités')).toBeTruthy();
    expect(screen.getByText(/effacée une fois reprise/)).toBeTruthy();
    const targets = screen.getAllByTestId('team-handoff-target');
    expect(targets.map((x) => x.getAttribute('data-machine'))).toEqual(['false', 'true']);
    expect(screen.getByText('Machines de l’équipe')).toBeTruthy();
    // To the team's machine, as a commit « en cours ».
    await userEvent.click(screen.getByText('serveur-acme'));
    await userEvent.click(screen.getByTestId('team-handoff-commit'));
    await userEvent.click(screen.getByTestId('team-handoff-submit'));
    await screen.findByTestId('team-handoff-done');
    expect(backend.called('ticket_team_handoff')).toEqual([
      { cmd: 'ticket_team_handoff', args: { id: 't41', to: 'cc'.repeat(16), wip: 'commit' } },
    ]);
  });

  it('says that the session of Claude Code goes with it, encrypted, only when it does (TS-17b)', async () => {
    fakeBackend({ ticket_team_handoff_plan: () => plan({ session: true }) });
    render(TeamHandoffModal, { ticketId: 't41' });
    expect(await screen.findByTestId('team-handoff-session')).toHaveTextContent(
      'Sa session de Claude Code part avec lui, chiffrée ; s’il ne peut pas la reprendre, il repart avec la passation.',
    );
    cleanup();
    setLang('en');
    fakeBackend({ ticket_team_handoff_plan: () => plan({ session: true }) });
    render(TeamHandoffModal, { ticketId: 't41' });
    expect(await screen.findByTestId('team-handoff-session')).toHaveTextContent('Its Claude Code session goes with it, encrypted');
    cleanup();
    setLang('fr');
    // Another harness, or no session: nothing said of it.
    fakeBackend({ ticket_team_handoff_plan: () => plan({ engine: 'codex' }) });
    render(TeamHandoffModal, { ticketId: 't41' });
    await screen.findByText('Passer DEM-41');
    expect(screen.queryByTestId('team-handoff-session')).toBeNull();
  });

  it('a refusal says why and offers nothing; a failure keeps the window', async () => {
    fakeBackend({
      ticket_team_handoff_plan: () =>
        plan({ refusal: 'Cet agent est d’un projet personnel : seul l’agent d’un ticket d’un projet de l’équipe se passe à un collègue.' }),
    });
    render(TeamHandoffModal, { ticketId: 't41' });
    expect(await screen.findByTestId('team-handoff-refusal')).toHaveTextContent('projet personnel');
    expect(screen.queryByTestId('team-handoff-submit')).toBeNull();
    cleanup();
    fakeBackend({
      ticket_team_handoff_plan: () => plan({ dirty: [], running: false }),
      ticket_team_handoff: () => {
        throw 'L’hébergeur de la branche ticket/dem-41 ne répond pas : rien n’est passé, l’agent et son travail restent ici.';
      },
    });
    render(TeamHandoffModal, { ticketId: 't41' });
    expect(await screen.findByText('Aucun travail non commité.')).toBeTruthy();
    expect(screen.getByTestId('team-handoff-turn')).toHaveTextContent('L’agent est arrêté entre deux tours.');
    await userEvent.click(screen.getByTestId('team-handoff-submit'));
    expect(await screen.findByTestId('team-handoff-failure')).toHaveTextContent('rien n’est passé');
  });

  it('is in the box « À toi » of the teammate, with « Prendre »', async () => {
    const v: InboxItem = {
      key: 'handoff:t41',
      source: 'handoff',
      mark: null,
      group: 'today',
      projectId: 'p1',
      projectName: 'demo',
      private: false,
      title: 'La passation',
      since: Date.now(),
      agentId: null,
      agentName: 'DEM-41',
      ticketId: 't41',
      ticketKey: 'DEM-41',
      snoozedUntil: null,
      request: null,
      column: 'doing',
      blocked: null,
      conflict: false,
      partial: false,
      decision: null,
      files: 0,
      number: null,
      url: null,
      word: null,
      by: 'Alice Martin',
      engine: 'claude',
      branch: 'ticket/dem-41',
    };
    expect(viewTitle(v)).toBe('Alice Martin te passe DEM-41 · La passation');
    expect(gesturesOf(v).map((g) => g.id)).toEqual(['take']);
    const backend = fakeBackend({
      ticket_team_handoff_engines: () => ({ preferred: 'claude', engines: ['claude'] }),
      inbox_views: () => ({ enabled: true, views: [], blocking: 0, total: 0 }),
    });
    render(InboxViewPane, { view: v });
    expect(screen.getByText(/Escouade récupère la branche ticket\/dem-41/)).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: 'Prendre' }));
    await waitFor(() =>
      expect(backend.called('inbox_view_act')).toEqual([
        { cmd: 'inbox_view_act', args: { key: 'handoff:t41', act: { act: 'take', engine: 'claude' } } },
      ]),
    );
  });
});
