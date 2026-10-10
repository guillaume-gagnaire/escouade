import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../lib/state.svelte';
import type { Agent, PendingRequest } from '../lib/types';
import { agent, fakeBackend, project, resetApp, ticket } from '../test/ipc';
import Overview from './Overview.svelte';

const NOW = Date.UTC(2026, 9, 10, 12, 0, 0);
const MIN = 60_000;
/** How long the buttons of a request just shown wait, and a little more. */
const HELD = 550;

/** A permission waiting, as the agent's view sums it up. */
const permission = (over: Partial<PendingRequest> = {}): PendingRequest => ({
  id: 'req-1',
  kind: 'permission',
  tool: 'Bash',
  arg: 'npm test',
  description: null,
  reason: null,
  questions: [],
  cut: false,
  ...over,
});

const question = (over: Partial<PendingRequest> = {}): PendingRequest => ({
  id: 'req-2',
  kind: 'question',
  tool: 'AskUserQuestion',
  arg: '',
  description: null,
  reason: null,
  questions: [{ question: 'Quelle base de données ?', options: ['SQLite', 'Postgres'] }],
  cut: false,
  ...over,
});

/** An agent waiting on `requests`, the first asked first. */
const asking = (requests: PendingRequest[], over: Partial<Agent> = {}) =>
  agent({ status: 'waiting', pending: requests.map((r) => r.id), requests, ...over });

const backend = () => fakeBackend();

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

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

  it('allows or denies a pending permission on the spot, with its tool and its argument, from the agents’ view alone', async () => {
    const be = backend();
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [
        asking([permission()], { id: 'a1' }),
        asking([permission({ id: 'req-9', arg: 'rm -rf dist' })], { id: 'b1', projectId: 'p2', name: 'api-docs' }),
      ],
    });
    app.openOverview();
    render(Overview);
    const first = screen.getByRole('listitem', { name: /refacto-auth/ });
    expect(first).toHaveTextContent('Bash');
    expect(first).toHaveTextContent('npm test');
    await userEvent.click(within(first).getByRole('button', { name: 'Autoriser' }));
    expect(be.called('answer_permission').map((c) => c.args)).toEqual([{ id: 'a1', requestId: 'req-1', decision: 'allow', message: null }]);
    const second = screen.getByRole('listitem', { name: /api-docs/ });
    expect(second).toHaveTextContent('rm -rf dist');
    await userEvent.click(within(second).getByRole('button', { name: 'Refuser' }));
    expect(be.called('answer_permission').at(-1)?.args).toEqual({ id: 'b1', requestId: 'req-9', decision: 'deny', message: null });
    // Answering opens nothing: the overview stays.
    expect(app.ui.view).toBe('overview');
    // What a request is comes with its agent: no conversation is read for it.
    expect(be.called('get_conversation')).toHaveLength(0);
  });

  it('keeps showing the request asked first when another one comes', async () => {
    const be = backend();
    resetApp({ agents: [asking([permission()], { id: 'a1' })] });
    render(Overview);
    const row = screen.getByRole('listitem', { name: /refacto-auth/ });
    app.agents.a1 = asking([permission(), permission({ id: 'req-2', arg: 'rm -rf dist' })], { id: 'a1' });
    await Promise.resolve();
    expect(row).toHaveTextContent('npm test');
    expect(row).not.toHaveTextContent('rm -rf dist');
    // Nothing changed under the pointer: its buttons stay usable.
    await userEvent.click(within(row).getByRole('button', { name: 'Autoriser' }));
    expect(be.called('answer_permission').map((c) => c.args.requestId)).toEqual(['req-1']);
  });

  it('holds the buttons for 500 ms when another request takes the place of the one answered, and answers the one shown', async () => {
    const be = backend();
    resetApp({ agents: [asking([permission(), permission({ id: 'req-2', arg: 'rm -rf dist' })], { id: 'a1' })] });
    render(Overview);
    const row = screen.getByRole('listitem', { name: /refacto-auth/ });
    await userEvent.click(within(row).getByRole('button', { name: 'Autoriser' }));
    // The backend's answer: the next request is the one waiting now.
    app.agents.a1 = asking([permission({ id: 'req-2', arg: 'rm -rf dist' })], { id: 'a1' });
    await waitFor(() => expect(row).toHaveTextContent('rm -rf dist'));
    const allow = within(row).getByRole('button', { name: 'Autoriser' });
    expect(allow).toBeDisabled();
    expect(within(row).getByRole('button', { name: 'Refuser' })).toBeDisabled();
    await userEvent.click(allow);
    expect(be.called('answer_permission')).toHaveLength(1);
    await wait(HELD);
    expect(allow).toBeEnabled();
    await userEvent.click(allow);
    expect(be.called('answer_permission').map((c) => c.args.requestId)).toEqual(['req-1', 'req-2']);
  });

  it('shows a command over four lines at most, with why Claude asks, and sends one it cannot show whole to its agent', async () => {
    const be = backend();
    const short = 'npm ci\nnpm test';
    const long = ['cd app', 'npm ci', 'npm run build', 'rm -rf dist/old', 'git push --force', 'echo fin'].join('\n');
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' }), project({ id: 'p3', name: 'docs' })],
      agents: [
        asking([permission({ arg: short, description: 'Installe et teste', reason: 'Commande hors de la liste' })], { id: 'a1' }),
        asking([permission({ id: 'req-5', arg: long })], { id: 'b1', projectId: 'p2', name: 'api-docs', lastActivity: 2 }),
        asking([permission({ id: 'req-6', arg: 'x'.repeat(2000), cut: true })], {
          id: 'c1',
          projectId: 'p3',
          name: 'guide',
          lastActivity: 3,
        }),
      ],
    });
    render(Overview);
    const fits = screen.getByRole('listitem', { name: /refacto-auth/ });
    // Its lines as they run.
    expect(fits.querySelector('.arg')?.textContent).toBe(short);
    expect(fits).toHaveTextContent('Installe et teste');
    expect(fits).toHaveTextContent('Commande hors de la liste');
    expect(within(fits).getByRole('button', { name: 'Autoriser' })).toBeInTheDocument();
    // Longer than its four lines, or cut by the backend: answered where it is whole.
    for (const [id, name] of [
      ['b1', 'api-docs'],
      ['c1', 'guide'],
    ]) {
      const row = screen.getByRole('listitem', { name });
      expect(row).toHaveTextContent('La suite se lit dans la conversation.');
      expect(within(row).queryByRole('button', { name: 'Autoriser' })).not.toBeInTheDocument();
      expect(within(row).queryByRole('button', { name: 'Refuser' })).not.toBeInTheDocument();
      await userEvent.click(within(row).getByRole('button', { name: 'Répondre' }));
      expect(app.agent?.id).toBe(id);
      app.openOverview();
    }
    expect(be.called('answer_permission')).toHaveLength(0);
  });

  it('sends to its agent a command of one line that wraps past the four shown', async () => {
    backend();
    resetApp({ agents: [asking([permission({ arg: `npm run e2e ${'--grep connexion '.repeat(60)}` })], { id: 'a1' })] });
    // Laid out taller than its four lines (jsdom lays nothing out).
    const size = (arg: number) =>
      function (this: Element) {
        return this.classList.contains('arg') ? arg : 0;
      };
    const tall = vi.spyOn(Element.prototype, 'scrollHeight', 'get').mockImplementation(size(180));
    const box = vi.spyOn(Element.prototype, 'clientHeight', 'get').mockImplementation(size(72));
    try {
      render(Overview);
      const row = screen.getByRole('listitem', { name: /refacto-auth/ });
      await waitFor(() => expect(within(row).queryByRole('button', { name: 'Autoriser' })).not.toBeInTheDocument());
      expect(within(row).getByRole('button', { name: 'Répondre' })).toBeInTheDocument();
    } finally {
      tall.mockRestore();
      box.mockRestore();
    }
  });

  it('leaves Ctrl+Enter to the conversation: it neither answers nor opens anything here', async () => {
    const be = backend();
    resetApp({ agents: [asking([permission()], { id: 'a1' })] });
    app.openOverview();
    render(Overview);
    expect(line('a1')).toHaveFocus();
    await userEvent.keyboard('{Control>}{Enter}{/Control}{Control>}{Shift>}{Enter}{/Shift}{/Control}');
    expect(be.called('answer_permission')).toHaveLength(0);
    expect(app.ui.view).toBe('overview');
  });

  it('gives the focus to the line of an agent answered from the keyboard once its request is gone', async () => {
    backend();
    resetApp({ agents: [asking([permission()], { id: 'a1' }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 })] });
    app.openOverview();
    render(Overview);
    await userEvent.tab();
    expect(screen.getByRole('button', { name: 'Autoriser' })).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    // The backend's answer: the agent works again, out of « Attend ta réponse ».
    app.agents.a1 = { ...app.agents.a1, status: 'running', pending: [], requests: [] };
    await waitFor(() => expect(line('a1')).toHaveFocus());
    expect(line('a1').closest('section')).toHaveAccessibleName('demo-api');
  });

  it('shows a question’s text, and « Répondre » opens its agent', async () => {
    const be = backend();
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [agent({ id: 'a1' }), asking([question()], { id: 'b1', projectId: 'p2', name: 'api-docs' })],
    });
    app.openOverview();
    render(Overview);
    const row = screen.getByRole('listitem', { name: /api-docs/ });
    expect(within(row).getByText('Quelle base de données ?')).toBeInTheDocument();
    expect(within(row).queryByRole('button', { name: 'Autoriser' })).not.toBeInTheDocument();
    await userEvent.click(within(row).getByRole('button', { name: 'Répondre' }));
    expect(app.ui.view).toBe('project');
    expect(app.project?.id).toBe('p2');
    expect(app.agent?.id).toBe('b1');
    expect(be.called('answer_question')).toHaveLength(0);
  });

  it('asks to answer a proposed plan in its agent, not on the spot', () => {
    backend();
    resetApp({ agents: [asking([permission({ tool: 'ExitPlanMode', arg: '## Plan' })], { id: 'a1' })] });
    render(Overview);
    const row = screen.getByRole('listitem', { name: /refacto-auth/ });
    expect(within(row).getByText('Claude propose un plan')).toBeInTheDocument();
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
