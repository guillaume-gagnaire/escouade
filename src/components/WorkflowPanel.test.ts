import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { readPref, removePref, writePref } from '../lib/prefs';
import { app } from '../lib/state.svelte';
import type { WorkflowScan, WorkflowTrack } from '../lib/types';
import { workflows } from '../lib/workflows.svelte';
import { agent, fakeBackend, project, resetApp, ticket } from '../test/ipc';
import { getByTip } from '../test/tip';
import { GSD_ID, gsd, paginationUsers, passage, ralph, scanOf, track } from '../test/workflows';
import SidePanel from './SidePanel.svelte';

const AGENTS = [
  agent({ id: 'a1', name: 'pagination-users', status: 'running' }),
  agent({ id: 'a2', name: 'refacto-auth', status: 'waiting' }),
  agent({ id: 'a3', name: 'dem-5-export', status: 'running', ticketId: 't1' }),
];

function backend(scan: WorkflowScan, tracks: WorkflowTrack[]) {
  return fakeBackend({
    get_workflows: () => scan,
    get_workflow_tracks: () => tracks,
    git_files: () => [],
    git_log: () => ({ commits: [], head: 'main' }),
  });
}

const gsdTracks = () => [
  paginationUsers('a1'),
  track('a2', {
    step: 'gsd:execute-phase',
    subagents: ['gsd-executor'],
    passages: [passage(1, 'gsd:execute-phase', 0, 3, { outcome: 'running' })],
  }),
  track('a3', {
    step: 'gsd:plan-phase',
    ticketLoop: [1, 5],
    passages: [passage(1, 'gsd:plan-phase', 0, 3, { outcome: 'running', pass: 2 })],
  }),
];

async function open(scan: WorkflowScan, tracks: WorkflowTrack[], openAgent = 'a1') {
  const be = backend(scan, tracks);
  render(SidePanel, { project: project(), agent: app.agents[openAgent], docked: true });
  await userEvent.click(await screen.findByRole('tab', { name: /^Workflow/ }));
  return be;
}

describe('the Workflow tab', () => {
  beforeEach(() => resetApp({ projects: [project()], agents: AGENTS, tickets: [ticket({ id: 't1', key: 'DEM-5', column: 'doing' })] }));

  it('sits between Historique and PR et CI, with the count of the agents that follow a workflow', async () => {
    backend(scanOf(gsd()), gsdTracks());
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    const tabs = await screen.findAllByRole('tab');
    await waitFor(() => expect(tabs.map((t) => t.textContent?.trim())).toEqual(['Non commités 0', 'Historique', 'Workflow 3', 'PR et CI']));
  });

  it('draws the rail: the steps, their doors, subagents, files and ways back, the agents on them', async () => {
    await open(scanOf(gsd()), gsdTracks());
    const tab = await screen.findByTestId('workflow');
    await within(tab).findByText('Verify work', { selector: '.wf-name' });
    expect(within(tab).getByText('4 étapes', { exact: false })).toHaveTextContent(
      '4 étapes · phase 3 sur 6 d’après .planning/STATE.md · lu à 10:12',
    );
    const steps = [...tab.querySelectorAll('.wf-step')];
    expect(steps.map((s) => s.querySelector('.wf-name')?.textContent)).toEqual([
      'Discuss phase',
      'Plan phase',
      'Execute phase',
      'Verify work',
    ]);
    expect(steps[1].querySelector('.wf-detail')).toHaveTextContent('gsd-phase-researchergsd-plannergsd-plan-checker jusqu’à 3 tours');
    expect(steps[0].querySelector('.wf-detail')).toHaveTextContent('écrit CONTEXT.md');
    expect(steps[3].querySelector('.wf-detail')).toHaveTextContent('retour à Plan phase');
    // The open agent's step stands out, its pill is current.
    expect(steps[3]).toHaveClass('here', 'busy');
    expect(steps[0]).not.toHaveClass('busy');
    expect(within(steps[3] as HTMLElement).getByRole('button', { name: 'pagination-users' })).toHaveAttribute('aria-current', 'true');
    expect(within(steps[2] as HTMLElement).getByRole('button', { name: 'refacto-auth' })).toHaveClass('wait');
    // The way back drawn in the gutter, and the next round dashed from the end.
    await waitFor(() => expect(tab.querySelectorAll('.wf-edges path')).toHaveLength(2));
    expect(tab.querySelector('.wf-edges path.outer')).not.toBeNull();
    expect(tab.querySelector('.wf-end')).toHaveTextContent('The milestone is complete');
  });

  it('says where the chosen agent is in its pass: the steps done ticked, its own, and « Ensuite » (ESC-244)', async () => {
    await open(scanOf(gsd()), gsdTracks());
    const tab = await screen.findByTestId('workflow');
    await within(tab).findByText('Agents dans GSD');
    const steps = () => [...tab.querySelectorAll('.wf-step')];
    const cls = (cl: string) => steps().map((s) => s.classList.contains(cl));
    // pagination-users, on Verify after a way back: Discuss, Plan and Execute done in this pass, the exit next or back to Plan.
    expect(cls('done')).toEqual([true, true, true, false]);
    expect(steps()[0].querySelector('.wf-node [data-icon="check"]')).not.toBeNull();
    expect(steps()[0]).toHaveTextContent('(fait)');
    expect(steps()[3]).toHaveClass('here');
    expect(tab.querySelector('.wf-end')).toHaveClass('next');
    expect(tab.querySelector('.wf-end .wf-next')).toHaveTextContent('Ensuite');
    expect(cls('alt')).toEqual([false, true, false, false]);
    expect(getByTip(steps()[1] as HTMLElement, 'If gaps are found, run /gsd:plan-phase --gaps')).toHaveTextContent('ou retour');
    // Each agent's line says its next.
    const rows = [...tab.querySelectorAll('.wf-ag')];
    expect(rows.map((r) => r.querySelector('.nx')?.textContent)).toEqual([
      'Ensuite : la sortie, ou retour à Plan phase',
      'Ensuite : Verify work',
      'Ensuite : Execute phase',
    ]);
    expect(rows[0].querySelector('.nx')).toHaveAttribute('data-tip', 'If gaps are found, run /gsd:plan-phase --gaps');
    // Another agent chosen in the list: the rail says where it is.
    await userEvent.click(rows[1] as HTMLElement);
    await waitFor(() => expect(steps()[2]).toHaveClass('here'));
    expect(cls('done')).toEqual([false, false, false, false]);
    expect(cls('next')).toEqual([false, false, false, true]);
    expect(steps()[3].querySelector('.wf-next')).toHaveTextContent('Ensuite');
    expect(tab.querySelector('.wf-end')).not.toHaveClass('next');
  });

  it('opens an agent from its pill', async () => {
    await open(scanOf(gsd()), gsdTracks());
    await userEvent.click(await screen.findByRole('button', { name: 'refacto-auth' }));
    expect(app.ui.selectedAgent.p1).toBe('a2');
  });

  it('puts a door in the open agent’s composer, without sending it; « +2 » shows the rest', async () => {
    const be = await open(scanOf(gsd()), gsdTracks());
    const gates = await screen.findByText('Portes d’entrée');
    const head = gates.parentElement!;
    expect([...head.querySelectorAll('button.gate')].map((b) => b.textContent)).toEqual([
      '/gsd:autonomous',
      '/gsd:next',
      '/gsd:progress',
      '+2',
    ]);
    await userEvent.click(within(head).getByRole('button', { name: '+2' }));
    expect(within(head).getByRole('button', { name: '/gsd:quick' })).toBeInTheDocument();
    await userEvent.click(within(head).getByRole('button', { name: '/gsd:next' }));
    expect(app.composerPut).toEqual({ agentId: 'a1', text: '/gsd:next' });
    expect(be.calls.filter((c) => c.cmd === 'send_message')).toEqual([]);
  });

  it('lists the agents in the workflow: step, subagent, pass, ticket loop, time and cost', async () => {
    await open(scanOf(gsd()), gsdTracks());
    const list = (await screen.findByText('Agents dans GSD')).nextElementSibling as HTMLElement;
    expect(screen.getByText('Agents dans GSD')).toHaveTextContent('3 en cours');
    const rows = [...list.querySelectorAll('.wf-ag')];
    expect(rows.map((r) => r.querySelector('.where')?.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'Verify work · gsd-verifier · 2ᵉ passage',
      'Execute phase · gsd-executor',
      'Plan phase · 2ᵉ passage · boucle 1/5 du ticket',
    ]);
    expect(rows[2]).toHaveTextContent('DEM-5');
    expect(rows[0]).toHaveAttribute('aria-current', 'true');
    expect(rows[0].querySelectorAll('.pips i.done')).toHaveLength(3);
    expect(rows[0].querySelector('.nums')).toHaveTextContent('48m 00s · 3,91 $');
  });

  it('shows the open agent’s history, the way back with its reason; another agent’s by a click, without leaving the conversation', async () => {
    await open(scanOf(gsd()), gsdTracks());
    const head = await screen.findByText('Historique de pagination-users');
    expect(head).toHaveTextContent('phase 3 ·');
    const tab = screen.getByTestId('workflow');
    const evs = [...tab.querySelectorAll('.wf-hist .ev')];
    expect(evs.map((e) => e.querySelector('.what b')?.textContent)).toEqual([
      'Discuss phase',
      'Plan phase',
      'Execute phase',
      'Verify work',
      'Retour à Plan phase',
      'Plan phase',
      'Execute phase',
      'Verify work',
    ]);
    expect(evs[3]).toHaveClass('bad');
    expect(evs[3]).toHaveTextContent('2 écarts');
    expect(evs[4]).toHaveClass('back');
    expect(evs[4]).toHaveTextContent('prévu par verify-work.md');
    expect(evs[7]).toHaveClass('on');
    expect(tab.querySelectorAll('.wf-strip i')).toHaveLength(7);
    await userEvent.click(screen.getByRole('button', { name: /refacto-auth.*Execute phase/ }));
    expect(screen.getByText('Historique de refacto-auth')).toBeInTheDocument();
    expect(app.ui.selectedAgent.p1 ?? 'a1').toBe('a1');
  });

  it('keeps the current phase first, « Tout voir » for the ones before', async () => {
    const tr = paginationUsers('a1');
    tr.passages = [passage(0, 'gsd:discuss-phase', -60, 5), passage(9, 'gsd:verify-work', -50, 5), ...tr.passages];
    await open(scanOf(gsd()), [tr]);
    await screen.findByText('Historique de pagination-users');
    const tab = screen.getByTestId('workflow');
    expect(tab.querySelectorAll('.wf-hist .ev')).toHaveLength(8);
    await userEvent.click(screen.getByRole('button', { name: 'Tout voir' }));
    expect(tab.querySelectorAll('.wf-hist .ev')).toHaveLength(10);
  });

  it('folds the rail to a line a step, kept for the project', async () => {
    await open(scanOf(gsd()), gsdTracks());
    await userEvent.click(await screen.findByRole('button', { name: 'Replier les étapes' }));
    const tab = screen.getByTestId('workflow');
    expect(tab.querySelector('.wf-graph')).toHaveClass('folded');
    expect(screen.queryByText('Portes d’entrée')).toBeNull();
    expect(screen.queryByText('Agents dans GSD')).toBeNull();
    expect(screen.getByText('Historique de pagination-users')).toBeInTheDocument();
    expect(workflows.isFolded('p1')).toBe(true);
    expect(localStorage.getItem('escouade.workflowFolded.p1') ?? '1').toBe('1');
    await userEvent.click(screen.getByRole('button', { name: 'Déplier les étapes' }));
    expect(tab.querySelector('.wf-graph')).not.toHaveClass('folded');
  });

  it('says an agent outside the workflow, a partial follow and a workflow that changed', async () => {
    const tracks = [
      paginationUsers('a1'),
      track('a2', {
        outside: 'lance la suite e2e à ta demande',
        passages: [
          passage(1, 'gsd:execute-phase', 0, 3),
          passage(2, null, 3, 8, { outcome: 'running', doing: 'lance la suite e2e à ta demande' }),
        ],
      }),
      track('a3', { step: 'gsd:verify-work', behind: true, support: { partial: true, missing: ['subagents', 'cost'] }, passages: [] }),
    ];
    app.agents.a3 = { ...app.agents.a3, engine: 'codex' };
    await open(scanOf(gsd()), tracks);
    const tab = await screen.findByTestId('workflow');
    await within(tab).findByText('Agents dans GSD');
    expect(screen.getByText('Agents dans GSD')).toHaveTextContent('2 en cours · 1 sorti');
    const out = screen.getByRole('button', { name: /refacto-auth.*Hors du workflow/ });
    expect(out).toHaveClass('off');
    expect(out).toHaveTextContent('Hors du workflow depuis');
    expect(out).toHaveTextContent('lance la suite e2e à ta demande');
    // Its pill stays on the step it left, greyed.
    const exec = tab.querySelectorAll('.wf-step')[2] as HTMLElement;
    expect(within(exec).getByRole('button', { name: 'refacto-auth' })).toHaveClass('off');
    const partial = tab.querySelectorAll('.wf-ag')[2] as HTMLElement;
    expect(partial).toHaveTextContent('Suivi partiel');
    expect(partial).toHaveTextContent('Codex : il manque les sous-agents, le coût par étape.');
    expect(within(tab).getByRole('status')).toHaveTextContent('Le workflow a changé à 10:12. dem-5-export garde sa version');
    await userEvent.click(within(tab).getByRole('button', { name: 'Revoir les étapes' }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects', projectId: 'p1' });
  });

  it('draws a guessed step dashed', async () => {
    const w = gsd();
    w.steps.push({ ...w.steps[3], name: 'gsd:ship', command: '/gsd:ship', order: 4, agents: [], writes: [], guessed: true });
    await open(scanOf(w), gsdTracks());
    const tab = await screen.findByTestId('workflow');
    const guess = (await within(tab).findByText('Ship')).closest('.wf-step')!;
    expect(guess).toHaveClass('guess');
    expect(guess).toHaveTextContent('deviné : nommé par verify-work.md, jamais vu en cours');
  });

  it('shows a loop of one prompt by its turns and its checklist', async () => {
    const turn = (n: number, ticked: boolean | null, outcome: 'done' | 'running' = 'done') => ({
      n,
      at: new Date(2026, 9, 26, 12, n).getTime(),
      ended: null,
      ms: 60_000,
      cost: 0.2,
      checked: { done: 3 + n, total: 6 },
      ticked,
      outcome,
    });
    const tr = track('a1', { workflow: ralph().id, turns: [turn(1, true), turn(2, false), turn(3, null, 'running')] });
    await open(scanOf(ralph()), [tr]);
    const tab = await screen.findByTestId('workflow');
    await within(tab).findByText('Un prompt relancé à chaque tour', { exact: false });
    expect(within(tab).getByRole('img', { name: '6 cases cochées sur 6' })).toBeInTheDocument();
    expect([...tab.querySelectorAll('.wf-turns i')].map((i) => i.classList[0])).toEqual(['ok', 'bad', 'on', ...Array(17).fill('todo')]);
    expect(within(tab).getByText('Tour 3 · en cours', { selector: '.txt b' })).toBeInTheDocument();
    expect(within(tab).getByText('Sortie : plus de case vide, ou 20 tours.')).toBeInTheDocument();
    // The turn after the one running is the next (ESC-244).
    expect([...tab.querySelectorAll('.wf-turns i.next')].map((i) => i.textContent)).toEqual(['4']);
    expect(within(tab).getByText('Ensuite : tour 4')).toBeInTheDocument();
    expect([...tab.querySelectorAll('.wf-step .wf-name')].map((n) => n.textContent)).toHaveLength(4);
    expect(tab.querySelectorAll('.wf-step')[2]).toHaveTextContent('retour à Implement it, and only it.');
    expect([...tab.querySelectorAll('.wf-hist .what b')].map((b) => b.textContent)).toEqual([
      'Tour 3 · en cours',
      'Tour 2 · non coché',
      'Tour 1 · coché',
    ]);
    expect(tab.querySelectorAll('.wf-hist .ev.bad')).toHaveLength(1);
    // No fold for a loop: its turns are its rail.
    expect(screen.queryByRole('button', { name: 'Replier les étapes' })).toBeNull();
  });

  it('says what was looked for when the project has no workflow, and the two ways out', async () => {
    // No ticket either: « Tickets » would be there.
    app.tickets = {};
    const scan = scanOf();
    scan.untraced = [{ name: 'GSD', harness: 'claude', steps: 4 }];
    await open(scan, []);
    expect(await screen.findByRole('heading', { name: 'Pas de workflow dans demo-api' })).toBeInTheDocument();
    expect(screen.getByText('.claude/commands').parentElement).toHaveTextContent('rien');
    expect(screen.getByText('GSD').parentElement).toHaveTextContent('jamais lancé ici');
    expect(screen.getByRole('tab', { name: 'Workflow' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Analyser avec un agent…' }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects', projectId: 'p1' });
  });

  it('says « Pas de workflow pour ce projet » and where it is undone (ESC-309)', async () => {
    app.tickets = {};
    await open({ ...scanOf(), off: true }, []);
    expect(await screen.findByRole('heading', { name: 'Pas de workflow pour ce projet' })).toBeInTheDocument();
    expect(screen.getByTestId('workflow-off')).toHaveTextContent(
      'Escouade ne lit ni n’écrit les workflows de demo-api et n’y suit aucun agent. Pour le défaire : Réglages › Projets › Workflow, « Lire les workflows ».',
    );
    expect(screen.queryByRole('button', { name: 'Analyser avec un agent…' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir les réglages du workflow' }));
    expect(app.modal).toEqual({ kind: 'settings', tab: 'projects', projectId: 'p1' });
  });

  it('chooses among several workflows in its menu, a repository’s named by it', async () => {
    await open(scanOf(gsd(), { ...ralph(), repo: 'web' }), gsdTracks());
    await userEvent.click(await screen.findByRole('button', { name: 'Workflow' }));
    const menu = screen.getByRole('menu');
    expect(
      within(menu)
        .getAllByRole('menuitemradio')
        .map((b) => b.textContent),
    ).toEqual(['GSD', 'web · Ralph', 'Tickets']);
    await userEvent.click(within(menu).getByRole('menuitemradio', { name: 'web · Ralph' }));
    expect(workflows.chosen.p1).toBe(ralph().id);
    expect(await screen.findByText('Un prompt relancé à chaque tour', { exact: false })).toBeInTheDocument();
  });

  it('follows the tracks as they come', async () => {
    await open(scanOf(gsd()), [paginationUsers('a1')]);
    await screen.findByText('Agents dans GSD');
    workflows.take(track('a2', { step: 'gsd:discuss-phase', passages: [passage(1, 'gsd:discuss-phase', 0, 1, { outcome: 'running' })] }));
    const tab = screen.getByTestId('workflow');
    await waitFor(() =>
      expect(within(tab.querySelectorAll('.wf-step')[0] as HTMLElement).getByRole('button', { name: 'refacto-auth' })).toBeInTheDocument(),
    );
    expect(screen.getByRole('tab', { name: /^Workflow/ })).toHaveTextContent('Workflow 2');
    void GSD_ID;
  });

  it('shows the workflow as the project has it: a step renamed by the user under its name, its agent on it', async () => {
    // What effective_workflows gives once « Plan phase » was renamed « Plan : revue » in Réglages › Projets › Workflow
    // (ESC-195); the tracking, corrected too, puts the agent on « Plan : revue ».
    const w = gsd();
    const corrected = {
      ...w,
      steps: w.steps.map((st) => (st.name === 'gsd:plan-phase' ? { ...st, name: 'Plan : revue', also: ['gsd:plan-phase'] } : st)),
    };
    corrected.transitions = w.transitions.map((tr) => ({
      ...tr,
      from: tr.from === 'gsd:plan-phase' ? 'Plan : revue' : tr.from,
      to: tr.to === 'gsd:plan-phase' ? 'Plan : revue' : tr.to,
    }));
    const tr = track('a1', { step: 'Plan : revue', passages: [passage(1, 'Plan : revue', 0, 3, { outcome: 'running' })] });
    fakeBackend({ get_workflows: () => scanOf(w), effective_workflows: () => [corrected], get_workflow_tracks: () => [tr] });
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    await userEvent.click(await screen.findByRole('tab', { name: /^Workflow/ }));
    const tab = await screen.findByTestId('workflow');
    const names = async () => [...tab.querySelectorAll('.wf-step .wf-name')].map((n) => n.textContent);
    await waitFor(async () => expect(await names()).toEqual(['Discuss phase', 'Plan : revue', 'Execute phase', 'Verify work']));
    const plan = tab.querySelectorAll('.wf-step')[1] as HTMLElement;
    expect(plan).toHaveClass('here');
    expect(within(plan).getByRole('button', { name: 'pagination-users' })).toHaveAttribute('aria-current', 'true');
    // Its way back says the new name too, and so do the agent's row and its history.
    expect(tab.querySelectorAll('.wf-step')[3]).toHaveTextContent('retour à Plan : revue');
    expect(tab.querySelector('.wf-ag .where')).toHaveTextContent('Plan : revue');
    expect(tab.querySelector('.wf-hist .what b')).toHaveTextContent('Plan : revue');
  });

  it('leaves out a workflow the user dismissed: the tab then says it has none', async () => {
    app.tickets = {};
    const scan = scanOf(gsd(), ralph());
    fakeBackend({ get_workflows: () => scan, effective_workflows: () => [ralph()], get_workflow_tracks: () => [] });
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    await userEvent.click(await screen.findByRole('tab', { name: /^Workflow/ }));
    expect(await screen.findByText('Un prompt relancé à chaque tour', { exact: false })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Workflow' }));
    expect(
      within(screen.getByRole('menu'))
        .getAllByRole('menuitemradio')
        .map((b) => b.textContent),
    ).toEqual(['Ralph']);
  });

  it('says it has no workflow when the only one was dismissed, and reads them again when the corrections change', async () => {
    app.tickets = {};
    let effective = [] as ReturnType<typeof gsd>[];
    const be = fakeBackend({ get_workflows: () => scanOf(gsd()), effective_workflows: () => effective, get_workflow_tracks: () => [] });
    const { rerender } = render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    await userEvent.click(await screen.findByRole('tab', { name: /^Workflow/ }));
    expect(await screen.findByRole('heading', { name: 'Pas de workflow dans demo-api' })).toBeInTheDocument();
    // « Ce n'est pas un workflow » undone in the settings: the project comes back with its corrections changed.
    effective = [gsd()];
    const before = be.called('effective_workflows').length;
    await rerender({
      project: project({ workflowFixes: [{ workflow: GSD_ID, steps: [{ name: 'Discuter', from: ['gsd:discuss-phase'] }] }] }),
      agent: app.agents.a1,
      docked: true,
    });
    await waitFor(() => expect(be.called('effective_workflows').length).toBeGreaterThan(before));
    expect(await screen.findByText('Discuss phase', { selector: '.wf-name' })).toBeInTheDocument();
  });

  it('speaks English', async () => {
    setLang('en');
    await open(scanOf(gsd()), gsdTracks());
    expect(await screen.findByText('Entry points')).toBeInTheDocument();
    expect(screen.getByText('Agents in GSD')).toBeInTheDocument();
    expect(screen.getByText('History of pagination-users')).toBeInTheDocument();
    expect(getByTip(document.body, 'Fold the steps')).toBeInTheDocument();
  });
});

describe('the workflow « Tickets » of Escouade, in the same tab (ESC-197, choice 159)', () => {
  beforeEach(() =>
    resetApp({
      projects: [project()],
      agents: AGENTS,
      tickets: [
        ticket({ id: 't1', key: 'DEM-5', column: 'doing', agentId: 'a3', iteration: 1, maxLoops: 5 }),
        ticket({ id: 't2', key: 'DEM-6', column: 'todo' }),
      ],
    }),
  );

  it('is in the menu beside the project’s workflow, an agent of a ticket in both', async () => {
    await open(scanOf(gsd()), gsdTracks(), 'a3');
    const tab = screen.getByTestId('workflow');
    // The agent of DEM-5 follows GSD: GSD first, with its ticket and its loop.
    await within(tab).findByText('Agents dans GSD');
    expect(within(tab).getByText('DEM-5')).toBeInTheDocument();
    expect(tab.querySelector('.wf-step[data-step="gsd:plan-phase"] .who[aria-current]')).toHaveTextContent('dem-5-export');
    // Counted once in the tab, though in two workflows.
    expect(screen.getByRole('tab', { name: /^Workflow/ })).toHaveTextContent('Workflow 3');
    await userEvent.click(within(tab).getByRole('button', { name: 'Workflow' }));
    await userEvent.click(within(screen.getByRole('menu')).getByRole('menuitemradio', { name: 'Tickets' }));
    // Its columns, the autopilot its door, « Renvoyer » its way back; the agent on « En cours ».
    expect([...tab.querySelectorAll('.wf-step .wf-name')].map((n) => n.textContent)).toEqual([
      'À faire',
      'En cours',
      'À tester',
      'Terminé',
    ]);
    expect(within(tab).getByText('pilote auto')).toHaveClass('plain');
    expect(within(tab).getByText('jusqu’à 5 boucles : le travail, puis la vérification des critères')).toBeInTheDocument();
    expect(within(tab).getByText('Renvoyer : retour à En cours')).toBeInTheDocument();
    expect(tab.querySelector('.wf-step[data-step="En cours"] .who')).toHaveTextContent('dem-5-export');
    expect(within(tab).getByText('Agents dans Tickets')).toBeInTheDocument();
    expect(within(tab).getByText('boucle 1/5 du ticket', { exact: false })).toBeInTheDocument();
    // À faire done, En cours its own, À tester next (ESC-244).
    expect(
      [...tab.querySelectorAll('.wf-step')].map((s) =>
        s.classList.contains('done') ? 'done' : s.classList.contains('here') ? 'here' : s.classList.contains('next') ? 'next' : '',
      ),
    ).toEqual(['done', 'here', 'next', '']);
    expect(tab.querySelector('.wf-ag .nx')).toHaveTextContent('Ensuite : À tester');
    // No history of its own.
    expect(within(tab).queryByText(/^Historique de/)).toBeNull();
  });

  it('shows « Tickets » alone when the project has no workflow of its own', async () => {
    await open(scanOf(), [], 'a3');
    const tab = screen.getByTestId('workflow');
    await within(tab).findByText('Le workflow des tickets d’Escouade, d’après le Kanban', { exact: false });
    expect(within(tab).queryByText(/^Pas de workflow/)).toBeNull();
    setLang('en');
    await within(tab).findByText('Escouade’s workflow of the tickets, from the Kanban', { exact: false });
    expect([...tab.querySelectorAll('.wf-step .wf-name')].map((n) => n.textContent)).toEqual(['To do', 'In progress', 'To review', 'Done']);
    expect(within(tab).getByText('Send back: back to In progress')).toBeInTheDocument();
    setLang('fr');
  });
});

describe('the Diagram view of the tab (ESC-245)', () => {
  let copied: string[];
  beforeEach(() => {
    resetApp({ projects: [project()], agents: AGENTS, tickets: [ticket({ id: 't1', key: 'DEM-5', column: 'doing' })] });
    removePref('workflowView');
    copied = [];
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: async (text: string) => void copied.push(text) },
      configurable: true,
    });
  });

  it('switches from the column to the diagram, and keeps the choice for the whole app', async () => {
    await open(scanOf(gsd()), gsdTracks());
    const tab = await screen.findByTestId('workflow');
    // The column first.
    await within(tab).findByText('Verify work', { selector: '.wf-name' });
    expect(within(tab).getByRole('tab', { name: 'Colonne' })).toHaveAttribute('aria-selected', 'true');
    await userEvent.click(within(tab).getByRole('tab', { name: 'Diagramme' }));
    const dg = await within(tab).findByTestId('workflow-diagram');
    expect(tab.querySelector('.wf-step')).toBeNull();
    expect(within(tab).getByRole('tab', { name: 'Diagramme' })).toHaveAttribute('aria-selected', 'true');
    expect(readPref('workflowView')).toBe('diagram');
    // The agents and the history stay under it.
    expect(within(tab).getByText('Agents dans GSD', { exact: false })).toBeInTheDocument();
    expect(dg.querySelectorAll('.dg-node')).toHaveLength(4);
    // After a restart (the choice of this run forgotten, the kept one read), the diagram again; back to the column, kept too.
    workflows.view = null;
    expect(workflows.viewOf()).toBe('diagram');
    await userEvent.click(within(tab).getByRole('tab', { name: 'Colonne' }));
    await within(tab).findByText('Verify work', { selector: '.wf-name' });
    expect(readPref('workflowView')).toBe('column');
  });

  it('draws the steps, where the open agent is, the loop, the way back it took, the exit', async () => {
    writePref('workflowView', 'diagram');
    await open(scanOf(gsd()), gsdTracks());
    const dg = await screen.findByTestId('workflow-diagram');
    await waitFor(() => expect(dg.querySelectorAll('.dg-node')).toHaveLength(4));
    const nodes = [...dg.querySelectorAll('.dg-node')];
    expect(nodes.map((n) => n.getAttribute('aria-label'))).toEqual([
      'Discuss phase, faite',
      'Plan phase, faite',
      'Execute phase, faite',
      'Verify work, en cours',
    ]);
    // The loop on Plan, ×3; the way back from Verify, taken by pagination-users.
    expect(nodes[1].querySelector('.dg-self')).toHaveTextContent('×3');
    expect(dg.querySelectorAll('path.e.took:not(.head)')).toHaveLength(1);
    expect(dg.querySelector('.dg-chip.ret.took title')).toHaveTextContent('If gaps are found');
    // Each agent a dot on its step, the open one current.
    expect(nodes[3].querySelector('circle.ag.cur title')).toHaveTextContent('pagination-users');
    expect(nodes[2].querySelector('circle.ag.wait title')).toHaveTextContent('refacto-auth');
    expect(dg.querySelector('.dg-exit')).toHaveTextContent('Sortie');
    expect([...dg.querySelectorAll('.dg-door')].map((d) => d.textContent)).toContain('/gsd:autonomous');
  });

  it('copies the workflow in Mermaid, without its agents', async () => {
    await open(scanOf(gsd()), gsdTracks());
    const tab = await screen.findByTestId('workflow');
    await userEvent.click(await within(tab).findByRole('button', { name: 'Copier en Mermaid' }));
    await waitFor(() => expect(copied).toHaveLength(1));
    expect(copied[0].split('\n').slice(0, 3)).toEqual(['flowchart TD', '  %% GSD', '  in1(["/gsd:autonomous"]) --> s1']);
    expect(copied[0]).not.toContain('pagination-users');
    expect(app.toasts.at(-1)).toMatchObject({
      kind: 'ok',
      text: `Code Mermaid copié : ${copied[0].split('\n').length} lignes, sans les agents`,
    });
  });

  it('keeps a trapped step name of the repository a text: nothing runs, the diagram holds, the copy is escaped', async () => {
    writePref('workflowView', 'diagram');
    const trap = '<img src=x onerror="window.__pwned=1">"]-->x click s1';
    const w = gsd();
    w.steps[0] = { ...w.steps[0], name: trap, named: true };
    w.transitions[0] = { ...w.transitions[0], from: trap };
    await open(scanOf(w), gsdTracks());
    const dg = await screen.findByTestId('workflow-diagram');
    await waitFor(() => expect(dg.querySelectorAll('.dg-node')).toHaveLength(4));
    expect(document.querySelector('img')).toBeNull();
    expect((window as unknown as { __pwned?: number }).__pwned).toBeUndefined();
    expect(dg.querySelector('.dg-node title')).toHaveTextContent(trap);
    await userEvent.click(screen.getByRole('button', { name: 'Copier en Mermaid' }));
    await waitFor(() => expect(copied).toHaveLength(1));
    expect(copied[0]).toContain(
      's1["#lt;img src=x onerror=#quot;window.__pwned=1#quot;#gt;#quot;]--#gt;x click s1<br>/gsd:discuss-phase"]',
    );
  });

  it('enlarges over the work panel, zooms, and closes by Échap', async () => {
    writePref('workflowView', 'diagram');
    await open(scanOf(gsd()), gsdTracks());
    await screen.findByTestId('workflow-diagram');
    await userEvent.click(screen.getByRole('button', { name: 'Agrandir le diagramme' }));
    const big = await screen.findByTestId('workflow-diagram-big');
    expect(within(big).getByTestId('workflow-zoom')).toHaveTextContent('100 %');
    await userEvent.click(within(big).getByRole('button', { name: 'Zoom avant (Ctrl +)' }));
    expect(within(big).getByTestId('workflow-zoom')).toHaveTextContent('110 %');
    within(big).getByRole('region').focus();
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByTestId('workflow-diagram-big')).toBeNull());
  });
});
