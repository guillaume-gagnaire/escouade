import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { dropConversation } from '../lib/conversations.svelte';
import { app } from '../lib/state.svelte';
import type { ConvItem, PermissionItem, QuestionItem } from '../lib/types';
import { agent, fakeBackend, project, resetApp, ticket } from '../test/ipc';
import Overview from './Overview.svelte';

const NOW = Date.UTC(2026, 9, 10, 12, 0, 0);
const MIN = 60_000;

const permission = (over: Partial<PermissionItem> = {}): PermissionItem => ({
  kind: 'permission',
  id: 'req-1',
  toolUseId: 't1',
  toolName: 'Bash',
  input: { command: 'npm test' },
  canAlways: true,
  defaultNo: false,
  decision: null,
  ts: 1,
  ...over,
});

const question = (over: Partial<QuestionItem> = {}): QuestionItem => ({
  kind: 'question',
  id: 'req-2',
  toolUseId: 't2',
  questions: [{ question: 'Quelle base de données ?', options: [{ label: 'SQLite' }, { label: 'Postgres' }] }],
  answers: null,
  ts: 1,
  ...over,
});

/** A fake backend whose agents' conversations are `convs`, by agent id. */
function backend(convs: Record<string, ConvItem[]> = {}) {
  return fakeBackend({ get_conversation: ({ id }: { id: string }) => convs[id] ?? [] });
}

/** The groups shown, in order, each with the agents of its lines. */
function groups() {
  return screen.getAllByRole('region').map((r) => ({
    title: within(r).getByRole('heading').textContent?.trim(),
    agents: within(r)
      .getAllByRole('listitem')
      .map((li) => li.dataset.agent),
  }));
}

/** The line of an agent: what opens it. */
const line = (id: string) => document.querySelector<HTMLElement>(`[data-agent="${id}"] [role="button"]`)!;

describe('Overview', () => {
  beforeEach(() => {
    for (const id of ['a1', 'a2', 'b1', 'b2', 'c1']) dropConversation(id);
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web', color: 'oklch(0.7 0.1 200)' }), project({ id: 'p3', name: 'vide' })],
    });
    app.now = NOW;
  });

  it('groups every project’s agents, those waiting for an answer at the top, the longest waiting first', () => {
    backend();
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' }), project({ id: 'p3', name: 'vide' })],
      agents: [
        agent({ id: 'a1', status: 'running', createdAt: 1 }),
        agent({ id: 'a2', name: 'tests-e2e', status: 'waiting', lastActivity: NOW - 2 * MIN, createdAt: 2 }),
        agent({ id: 'c1', name: 'vieux', archived: true }),
        agent({ id: 'b2', projectId: 'p2', name: 'landing', status: 'done', createdAt: 4 }),
        agent({ id: 'b1', projectId: 'p2', name: 'api-docs', status: 'waiting', lastActivity: NOW - 9 * MIN, createdAt: 3 }),
      ],
    });
    app.now = NOW;
    render(Overview);
    expect(groups()).toEqual([
      { title: expect.stringContaining('Attend ta réponse'), agents: ['b1', 'a2'] },
      { title: expect.stringContaining('demo-api'), agents: ['a1'] },
      { title: expect.stringContaining('studio-web'), agents: ['b2'] },
    ]);
    // Mixed projects: each waiting line names its own.
    expect(screen.getByRole('region', { name: 'Attend ta réponse' }).querySelector('[data-agent="b1"]')).toHaveTextContent('studio-web');
    // Only they have something to answer.
    expect(within(screen.getByRole('region', { name: 'demo-api' })).queryByRole('button', { name: 'Répondre' })).not.toBeInTheDocument();
  });

  it('shows on each line its ticket, what it does right now, its model, its cost, its context and since when', () => {
    backend();
    resetApp({
      agents: [
        agent({
          id: 'a1',
          status: 'running',
          activity: 'Lit src/db.ts',
          model: 'opus',
          cost: 1.2,
          liveCost: 0.04,
          liveTokens: 300,
          contextTokens: 68_000,
          contextWindow: 200_000,
          lastActivity: NOW - 3 * MIN - 5000,
        }),
        agent({ id: 'a2', name: 'tests-e2e', status: 'done', model: 'sonnet', cost: 0.5, createdAt: 2, lastActivity: NOW - 2 * 3600_000 }),
      ],
      tickets: [ticket({ agentId: 'a1', column: 'doing', iteration: 2, maxLoops: 5 })],
    });
    app.now = NOW;
    render(Overview);
    const busy = screen.getByRole('listitem', { name: /refacto-auth/ });
    expect(busy).toHaveTextContent('▸ DEM-1 · boucle 2/5');
    expect(busy).toHaveTextContent('Lit src/db.ts');
    expect(busy).toHaveTextContent('Opus');
    expect(busy).toHaveTextContent('≈ 1,24 $');
    expect(busy).toHaveTextContent('34 %');
    expect(busy).toHaveTextContent('3 min');
    const idle = screen.getByRole('listitem', { name: /tests-e2e/ });
    expect(idle).toHaveTextContent('Terminé');
    expect(idle).toHaveTextContent('Sonnet');
    expect(idle).toHaveTextContent('0,50 $');
    expect(idle).toHaveTextContent('2 h');
    // No context known before a turn told the model's window.
    expect(idle.querySelector('.ctx')).toHaveTextContent('—');
  });

  it('allows or denies a pending permission on the spot, with its tool and its argument', async () => {
    const be = backend({ a1: [permission()], b1: [permission({ id: 'req-9', input: { command: 'rm -rf dist' } })] });
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [
        agent({ id: 'a1', status: 'waiting', pending: ['req-1'] }),
        agent({ id: 'b1', projectId: 'p2', name: 'api-docs', status: 'waiting', pending: ['req-9'] }),
      ],
    });
    app.openOverview();
    render(Overview);
    const first = screen.getByRole('listitem', { name: /refacto-auth/ });
    await within(first).findByText('npm test');
    expect(first).toHaveTextContent('Bash');
    await userEvent.click(within(first).getByRole('button', { name: 'Autoriser' }));
    expect(be.called('answer_permission').map((c) => c.args)).toEqual([{ id: 'a1', requestId: 'req-1', decision: 'allow', message: null }]);
    const second = await screen.findByRole('listitem', { name: /api-docs/ });
    await within(second).findByText('rm -rf dist');
    await userEvent.click(within(second).getByRole('button', { name: 'Refuser' }));
    expect(be.called('answer_permission').at(-1)?.args).toEqual({ id: 'b1', requestId: 'req-9', decision: 'deny', message: null });
    // Answering opens nothing: the overview stays.
    expect(app.ui.view).toBe('overview');
  });

  it('leaves Ctrl+Enter to the conversation: it neither answers nor opens anything here', async () => {
    const be = backend({ a1: [permission()] });
    resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['req-1'] })] });
    app.openOverview();
    render(Overview);
    await screen.findByText('npm test');
    expect(line('a1')).toHaveFocus();
    await userEvent.keyboard('{Control>}{Enter}{/Control}{Control>}{Shift>}{Enter}{/Shift}{/Control}');
    expect(be.called('answer_permission')).toHaveLength(0);
    expect(app.ui.view).toBe('overview');
  });

  it('gives the focus to the line of an agent answered from the keyboard once its request is gone', async () => {
    backend({ a1: [permission()] });
    resetApp({
      agents: [agent({ id: 'a1', status: 'waiting', pending: ['req-1'] }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 })],
    });
    app.openOverview();
    render(Overview);
    await screen.findByText('npm test');
    await userEvent.tab();
    expect(screen.getByRole('button', { name: 'Autoriser' })).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    // The backend's answer: the agent works again, out of « Attend ta réponse ».
    app.agents.a1 = { ...app.agents.a1, status: 'running', pending: [] };
    await waitFor(() => expect(line('a1')).toHaveFocus());
    expect(line('a1').closest('section')).toHaveAccessibleName('demo-api');
  });

  it('shows a question’s text, and « Répondre » opens its agent', async () => {
    const be = backend({ b1: [question()] });
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [agent({ id: 'a1' }), agent({ id: 'b1', projectId: 'p2', name: 'api-docs', status: 'waiting', pending: ['req-2'] })],
    });
    app.openOverview();
    render(Overview);
    const row = await screen.findByRole('listitem', { name: /api-docs/ });
    await within(row).findByText('Quelle base de données ?');
    expect(within(row).queryByRole('button', { name: 'Autoriser' })).not.toBeInTheDocument();
    await userEvent.click(within(row).getByRole('button', { name: 'Répondre' }));
    expect(app.ui.view).toBe('project');
    expect(app.project?.id).toBe('p2');
    expect(app.agent?.id).toBe('b1');
    expect(be.called('answer_question')).toHaveLength(0);
  });

  it('asks to answer a proposed plan in its agent, not on the spot', async () => {
    backend({ a1: [permission({ toolName: 'ExitPlanMode', input: { plan: '## Plan' } })] });
    resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['req-1'] })] });
    render(Overview);
    const row = screen.getByRole('listitem', { name: /refacto-auth/ });
    await within(row).findByText('Claude propose un plan');
    expect(within(row).queryByRole('button', { name: 'Autoriser' })).not.toBeInTheDocument();
    expect(within(row).getByRole('button', { name: 'Répondre' })).toBeInTheDocument();
  });

  it('opens an agent in its project with a click or Enter, its conversation rather than the editor, and moves with the arrows', async () => {
    backend();
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [agent({ id: 'a1' }), agent({ id: 'b1', projectId: 'p2', name: 'api-docs' })],
    });
    await app.openEditor({ projectId: 'p2', source: 'project' });
    app.openOverview();
    const { unmount } = render(Overview);
    // The keys work at once: the first line has the focus.
    expect(line('a1')).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}');
    expect(line('b1')).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}');
    expect(line('b1')).toHaveFocus();
    await userEvent.keyboard('{ArrowUp}');
    expect(line('a1')).toHaveFocus();
    await userEvent.keyboard('{ArrowDown}{Enter}');
    expect(app.ui.view).toBe('project');
    expect(app.project?.id).toBe('p2');
    expect(app.agent?.id).toBe('b1');
    expect(app.editorOn).toBe(false);
    unmount();
    app.openOverview();
    render(Overview);
    await userEvent.click(screen.getByText('refacto-auth'));
    expect(app.ui.view).toBe('project');
    expect(app.agent?.id).toBe('a1');
  });

  it('goes back to the view it covered with Escape, but not behind a dialog', async () => {
    backend();
    resetApp({ agents: [agent()] });
    app.openStats();
    app.openOverview();
    render(Overview);
    app.modal = { kind: 'newProject' };
    await userEvent.keyboard('{Escape}');
    expect(app.ui.view).toBe('overview');
    app.modal = null;
    await userEvent.keyboard('{Escape}');
    expect(app.ui.view).toBe('stats');
  });

  it('says when there is no agent', () => {
    backend();
    resetApp({ agents: [agent({ archived: true })] });
    render(Overview);
    expect(screen.getByText('Aucun agent pour l’instant.')).toBeInTheDocument();
    expect(screen.queryByRole('region')).not.toBeInTheDocument();
  });
});
