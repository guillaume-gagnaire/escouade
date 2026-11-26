// The workflows in the « Workspace » (ESC-689): the step on an agent's card, its diagram with its path from a click,
// « Workflows » of the floating bar (the project's workflows, « Tickets » with them, their agents, a diagram without an
// agent, « Copier en Mermaid », the settings), and what Escape closes first.
import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { dropConversation } from '../../lib/conversations.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { WorkflowScan, WorkflowTrack } from '../../lib/types';
import { agent, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import { GSD_ID, gsd, passage, scanOf, track } from '../../test/workflows';
import WorkspaceView from './WorkspaceView.svelte';

/** a1 sent back from Verify to Plan, a2 executing, a3 on a ticket and in no workflow of its own. */
const TRACKS = (): WorkflowTrack[] => [
  track('a1', {
    step: 'gsd:plan-phase',
    passages: [
      passage(1, 'gsd:discuss-phase', 0, 4, { door: '/gsd:discuss-phase 3' }),
      passage(2, 'gsd:plan-phase', 4, 9),
      passage(3, 'gsd:execute-phase', 13, 21),
      passage(4, 'gsd:verify-work', 34, 3, { outcome: 'back', reason: '2 écarts : le curseur', backTo: 'gsd:plan-phase' }),
      passage(5, 'gsd:plan-phase', 37, 4, { pass: 2, outcome: 'running' }),
    ],
  }),
  track('a2', { step: 'gsd:execute-phase', passages: [passage(1, 'gsd:execute-phase', 0, 3, { outcome: 'running' })] }),
];

let copied: string[];
function backend(scan: WorkflowScan | null = scanOf(gsd()), tracks: WorkflowTrack[] = TRACKS()) {
  return fakeBackend({
    get_conversation: () => [],
    get_workflows: () => scan,
    effective_workflows: () => scan?.workflows ?? [],
    get_workflow_tracks: () => tracks,
  });
}

function show() {
  app.openWorkspace('p1');
  return render(WorkspaceView, { project: app.projects[0] });
}

const cardOf = (name: string) => screen.getByRole('article', { name: new RegExp(`^${name},`) });

describe('the workflows in the Workspace (ESC-689)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({
      projects: [project()],
      agents: [
        agent({ id: 'a1', name: 'pagination-users', status: 'running' }),
        agent({ id: 'a2', name: 'refacto-auth', status: 'waiting' }),
        agent({ id: 'a3', name: 'dem-5-export', status: 'running', ticketId: 't1' }),
      ],
      tickets: [ticket({ id: 't1', key: 'DEM-5', column: 'doing', agentId: 'a3', iteration: 1 })],
    });
    copied = [];
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: async (text: string) => void copied.push(text) },
      configurable: true,
    });
  });
  afterEach(() => ['a1', 'a2', 'a3'].forEach(dropConversation));

  it('says the step on the card of an agent that follows a workflow, a way back in red with its reason; nothing on the others', async () => {
    backend();
    show();
    const step = await within(cardOf('pagination-users')).findByTestId('card-step');
    expect(step).toHaveTextContent(/^GSD · Plan phase · 2ᵉ passage en cours Renvoyé/);
    expect(step.querySelector('.wf-back')).toHaveTextContent('Renvoyé par Verify work : 2 écarts : le curseur');
    expect(step).toHaveClass('back');
    expect(step).toHaveAttribute('data-tip', expect.stringContaining('Voir le schéma de GSD et le chemin de pagination-users'));
    expect(within(cardOf('refacto-auth')).getByTestId('card-step')).toHaveTextContent(/^GSD · Execute phase en cours$/);
    expect(within(cardOf('refacto-auth')).getByTestId('card-step').querySelector('.wf-back')).toBeNull();
    // On a ticket, in « Tickets » alone: its card has its loop elsewhere, no step here.
    expect(within(cardOf('dem-5-export')).queryByTestId('card-step')).toBeNull();
  });

  it('opens the diagram of the workflow from the step, with the agent’s path, and Escape goes back to the canvas', async () => {
    backend();
    show();
    await userEvent.click(await within(cardOf('pagination-users')).findByTestId('card-step'));
    expect(app.workflowDiagram).toEqual({ projectId: 'p1', workflow: GSD_ID, agentId: 'a1' });
    const big = await screen.findByTestId('workflow-diagram-big');
    expect(big).toHaveTextContent('GSD');
    // The diagram of the old tab, as it was: from the door the agent took, its path told in words.
    expect([...big.querySelectorAll('.dg-door')].map((d) => d.getAttribute('data-door'))).toEqual(['/gsd:discuss-phase']);
    const path = within(big).getByTestId('workflow-path');
    expect(path).toHaveAccessibleName('Chemin de pagination-users, dans l’ordre');
    expect(path.querySelectorAll('li')).toHaveLength(5);
    expect(big.querySelector('.dg-node[data-step="gsd:plan-phase"] circle.ag.cur title')).toHaveTextContent('pagination-users');
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByTestId('workflow-diagram-big')).toBeNull());
    expect(app.workspaceOn).toBe(true);
    await userEvent.keyboard('{Escape}');
    expect(app.workspaceOn).toBe(false);
  });

  it('lists the project’s workflows in « Workflows » of the bar: « Tickets » too, their agents where they are', async () => {
    backend();
    show();
    const entry = screen.getByRole('button', { name: /^Workflows/ });
    await waitFor(() => expect(entry).toHaveTextContent('Workflows3'));
    expect(entry).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(entry);
    expect(entry).toHaveAttribute('aria-expanded', 'true');
    const panel = screen.getByTestId('workspace-workflows');
    expect(panel).toHaveAccessibleName('Workflows');
    const items = [...panel.querySelectorAll<HTMLElement>('li.wf')];
    expect(items.map((li) => li.querySelector('b')?.textContent)).toEqual(['GSD', 'Tickets']);
    expect(items[0].querySelector('.cap')).toHaveTextContent('4 étapes · 2 agents le suivent');
    expect([...items[0].querySelectorAll('.who button')].map((b) => b.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'pagination-users Plan phase · 2ᵉ passage',
      'refacto-auth Execute phase',
    ]);
    expect(items[1].querySelector('.who button')).toHaveTextContent('dem-5-export En cours · boucle 1/5 du ticket');
    // An agent's line opens the diagram with its path.
    await userEvent.click(within(items[1]).getByRole('button', { name: /dem-5-export/ }));
    expect(app.workflowDiagram).toEqual({ projectId: 'p1', workflow: 'escouade:tickets', agentId: 'a3' });
  });

  it('opens a workflow’s diagram without an agent, its door to choose; copies it in Mermaid; leads to its settings', async () => {
    const w = gsd();
    backend(scanOf(w), []);
    show();
    await userEvent.click(screen.getByRole('button', { name: /^Workflows/ }));
    const panel = screen.getByTestId('workspace-workflows');
    await within(panel).findByText('Aucun agent ne le suit', { exact: false });
    const gsdItem = panel.querySelector<HTMLElement>(`li.wf[data-workflow="${GSD_ID}"]`)!;
    await userEvent.click(within(gsdItem).getByRole('button', { name: 'Copier en Mermaid' }));
    await waitFor(() => expect(copied).toHaveLength(1));
    expect(copied[0].split('\n').slice(0, 2)).toEqual(['flowchart TD', '  %% GSD']);
    await userEvent.click(within(gsdItem).getByRole('button', { name: /Schéma/ }));
    expect(app.workflowDiagram).toEqual({ projectId: 'p1', workflow: GSD_ID, agentId: null });
    const big = await screen.findByTestId('workflow-diagram-big');
    const doors = within(big).getByTestId('workflow-entries');
    expect(
      within(doors)
        .getAllByRole('radio')
        .map((r) => r.textContent),
    ).toEqual(w.doors.filter((d) => d.role === 'entry').map((d) => d.command));
    expect(within(big).queryByTestId('workflow-path')).toBeNull();
    await userEvent.click(within(big).getByRole('button', { name: 'Fermer (Échap)' }));
    await waitFor(() => expect(screen.queryByTestId('workflow-diagram-big')).toBeNull());
    // « Workflows » is still open behind it.
    await userEvent.click(within(screen.getByTestId('workspace-workflows')).getByRole('button', { name: 'Réglages du workflow' }));
    expect(app.modal).toMatchObject({ kind: 'settings', page: 'board', projectId: 'p1', at: 'workflow' });
  });

  it('closes « Workflows » on Escape before the canvas, the focus back on its button', async () => {
    backend();
    show();
    const entry = screen.getByRole('button', { name: /^Workflows/ });
    await userEvent.click(entry);
    expect(screen.getByTestId('workspace-workflows')).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByTestId('workspace-workflows')).toBeNull();
    expect(entry).toHaveFocus();
    expect(app.workspaceOn).toBe(true);
  });

  it('says what was looked for when the project has no workflow, and when it reads none', async () => {
    backend(scanOf(), []);
    resetApp({ projects: [project()], agents: [agent({ id: 'a1' })] });
    const view = show();
    await userEvent.click(screen.getByRole('button', { name: /^Workflows/ }));
    const panel = screen.getByTestId('workspace-workflows');
    await within(panel).findByText('Pas de workflow dans demo-api', { exact: false });
    expect(within(panel).getByText('.claude/commands')).toBeInTheDocument();
    await userEvent.click(within(panel).getByRole('button', { name: 'Décrire un workflow…' }));
    expect(app.modal).toMatchObject({ kind: 'settings', at: 'workflow' });
    view.unmount();
    app.modal = null;
    backend({ ...scanOf(), off: true }, []);
    show();
    await userEvent.click(screen.getByRole('button', { name: /^Workflows/ }));
    expect(await screen.findByTestId('workflow-off')).toHaveTextContent('Pas de workflow pour ce projet');
  });

  it('forgets the diagram with the canvas, and opens the canvas for it from elsewhere (a divider of the conversation)', async () => {
    backend();
    const view = show();
    app.openWorkflowDiagram('p1', GSD_ID, 'a2');
    await screen.findByTestId('workflow-diagram-big');
    view.unmount();
    expect(app.workflowDiagram).toBeNull();
    app.closeWorkspace('p1');
    app.openWorkflowDiagram('p1', GSD_ID, 'a2');
    expect(app.workspaceOn).toBe(true);
    expect(app.workflowDiagram).toEqual({ projectId: 'p1', workflow: GSD_ID, agentId: 'a2' });
  });

  it('has no « Workflows » on the canvas of every project, its cards keep their step', async () => {
    backend();
    app.openOverviewCanvas();
    render(WorkspaceView, {});
    expect(screen.queryByRole('button', { name: /^Workflows/ })).toBeNull();
    expect(await within(cardOf('pagination-users')).findByTestId('card-step')).toBeInTheDocument();
  });

  it('speaks English', async () => {
    setLang('en');
    backend();
    show();
    const step = await within(cardOf('pagination-users')).findByTestId('card-step');
    expect(step).toHaveTextContent(/^GSD · Plan phase · pass 2 running Sent back/);
    expect(step.querySelector('.wf-back')).toHaveTextContent('Sent back by Verify work: 2 écarts : le curseur');
    await userEvent.click(screen.getByRole('button', { name: /^Workflows/ }));
    expect(screen.getByTestId('workspace-workflows').querySelector('.cap')).toHaveTextContent('4 steps · 2 agents follow it');
  });
});
