import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { app } from './lib/state.svelte';
import type { Agent, InitialState, Project } from './lib/types';
import { agent, fakeBackend, project, resetApp, SETTINGS } from './test/ipc';

const DIFF = `diff --git a/src/auth.ts b/src/auth.ts
--- a/src/auth.ts
+++ b/src/auth.ts
@@ -1 +1 @@
-const b = 2;
+const b = 3;
`;

function start(
  layout: '' | 'split',
  over: { projects?: Project[]; agents?: Agent[]; handlers?: Record<string, (args: any) => unknown> } = {},
) {
  const initial: InitialState = {
    projects: over.projects ?? [project()],
    agents: over.agents ?? [agent()],
    ui: { activeProject: 'p1', view: 'project', selectedAgent: {}, layout },
    settings: SETTINGS,
    usage: { fiveHour: null, sevenDay: null, todayCost: 0, updatedAt: 0 },
    git: {},
    shells: [],
    terminals: [],
    tickets: [],
    claudeFound: true,
    version: '0.1.0',
    models: [],
  };
  fakeBackend({
    subscribe: () => initial,
    get_conversation: () => [],
    git_files: () => [{ path: 'src/auth.ts', status: 'M', add: 1, del: 1, agentId: 'a1' }],
    git_diff: () => DIFF,
    git_branches: () => ['main'],
    ...over.handlers,
  });
  // Nothing rendered from a previous test's state until the snapshot is in.
  resetApp();
  app.ready = false;
  return render(App);
}

describe('App layout', () => {
  it('shows the uncommitted files and the diff next to the conversation in the split layout', async () => {
    start('split');
    expect(await screen.findByText('const b = 3;')).toBeInTheDocument();
    expect(screen.getByRole('main')).toBeInTheDocument();
  });

  it('opens the direct commit from the files panel when the project commits directly', async () => {
    start('split', {
      projects: [project({ commitMode: 'direct' })],
      handlers: {
        commit_preview: () => ({
          files: [{ path: 'src/auth.ts', status: 'M', add: 1, del: 1, agentId: 'a1', inWorktree: false }],
          leftOut: [],
        }),
        commit_propose: () => 'fix(auth): un jeton plus court',
      },
    });
    await screen.findByText('const b = 3;');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    const dialog = await screen.findByRole('dialog', { name: 'Commit' });
    const field = within(dialog).getByRole('textbox', { name: 'Message' });
    await waitFor(() => expect(field).toHaveValue('fix(auth): un jeton plus court'));
    expect(field).toHaveFocus();
  });

  it('keeps the files panel closed in the classic layout until asked for', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    expect(screen.queryByText('Non commités')).not.toBeInTheDocument();
  });

  it('shows the files of the agents’ worktrees in the diff of the whole project, as the list does', async () => {
    const diffOf = (path: string, line: string) =>
      `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n@@ -1 +1 @@\n-old\n+${line}\n`;
    const change = (path: string, agentId: string | null, inWorktree: boolean) => ({
      path,
      status: 'M',
      add: 1,
      del: 1,
      agentId,
      inWorktree,
    });
    const wt = { path: 'C:/wt', branch: 'escouade/a2', baseBranch: 'main' };
    start('', {
      agents: [agent(), agent({ id: 'a2', name: 'tests-e2e', worktree: wt })],
      handlers: {
        git_files: () => [change('README.md', null, false), change('src/wt.ts', 'a2', true)],
        // What « Voir le diff » used to read: the project's checkout only.
        git_diff: () => diffOf('README.md', 'racine'),
        git_project_diff: () => [
          { agentId: null, inWorktree: false, diff: diffOf('README.md', 'racine') },
          { agentId: 'a2', inWorktree: true, diff: diffOf('src/wt.ts', 'dans le worktree') },
        ],
      },
    });
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.filesScope = 'project';
    app.filesOpen = true;
    await screen.findByText('wt.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Voir le diff' }));
    const dialog = await screen.findByRole('dialog', { name: 'Modifications de demo-api' });
    const rows = await within(dialog).findAllByRole('button', { name: /^M / });
    expect(rows.map((r) => r.textContent)).toEqual([expect.stringMatching(/README\.md/), expect.stringMatching(/src\/wt\.ts\s*tests-e2e/)]);
    await userEvent.click(rows[1]);
    expect(within(dialog).getByText('dans le worktree')).toBeInTheDocument();
  });

  it('shows the board of the project in place of its agent, and the agent again when asked for', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'À faire' })).not.toBeInTheDocument();
    app.openBoard('p1');
    expect(await screen.findByRole('region', { name: 'À faire' })).toBeInTheDocument();
    expect(screen.getAllByRole('main')).toHaveLength(1);
    app.closeBoard('p1');
    await expect.poll(() => screen.queryByRole('region', { name: 'À faire' })).toBeNull();
  });

  it('opens the board settings from the board header', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.openBoard('p1');
    await userEvent.click(await screen.findByRole('button', { name: /Après validation/ }));
    expect(await screen.findByRole('dialog', { name: 'Réglages' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Kanban' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toBeInTheDocument();
  });

  it('comes back to the settings, unsaved changes kept, from closing a project there', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.modal = { kind: 'settings' };
    await userEvent.click(await screen.findByRole('tab', { name: 'Réseau' }));
    await userEvent.type(screen.getByPlaceholderText('aucun'), 'http://proxy:3128');
    await userEvent.click(screen.getByRole('tab', { name: 'Projets' }));
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet…' }));
    await userEvent.click(await screen.findByRole('button', { name: 'Annuler' }));
    expect(await screen.findByRole('tab', { name: 'Projets' })).toHaveAttribute('aria-selected', 'true');
    await userEvent.click(screen.getByRole('tab', { name: 'Réseau' }));
    expect(screen.getByPlaceholderText('aucun')).toHaveValue('http://proxy:3128');
    // Closed for good: still the settings, still the draft.
    await userEvent.click(screen.getByRole('tab', { name: 'Projets' }));
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet…' }));
    await userEvent.click(await screen.findByRole('button', { name: 'Fermer le projet' }));
    expect(await screen.findByText('Aucun projet ouvert.')).toBeInTheDocument();
    expect(app.projects).toEqual([]);
    await userEvent.click(screen.getByRole('tab', { name: 'Réseau' }));
    expect(screen.getByPlaceholderText('aucun')).toHaveValue('http://proxy:3128');
  });

  it('opens « Rechercher dans les conversations » with Ctrl+K, and the message of the result chosen', async () => {
    const a2 = agent({ id: 'a2', name: 'pagination', createdAt: 2 });
    const said = (id: string, text: string) => ({ kind: 'user', id, text, images: 0, ts: 1, queued: false });
    start('', {
      agents: [agent(), a2],
      handlers: {
        get_conversation: ({ id }: { id: string }) =>
          id === 'a2' ? [said('u0', 'Bonjour'), said('u1', 'Ajoute la pagination'), said('u2', 'Merci')] : [],
        search_conversations: () => ({
          hits: [
            {
              agentId: 'a2',
              projectId: 'p1',
              agentName: 'pagination',
              archived: false,
              eventIndex: 0,
              itemId: 'u1',
              snippet: 'Ajoute la pagination',
              mark: [10, 20],
              at: Date.now(),
            },
          ],
          capped: false,
          timedOut: false,
        }),
      },
    });
    expect(await screen.findByRole('main')).toBeInTheDocument();
    await userEvent.keyboard('{Control>}k{/Control}');
    const field = await screen.findByRole('combobox', { name: 'Rechercher dans les conversations' });
    await userEvent.type(field, 'pagination');
    await screen.findByRole('option', { name: /Ajoute la pagination/ });
    await userEvent.keyboard('{Enter}');
    await expect.poll(() => screen.queryByRole('dialog', { name: 'Rechercher dans les conversations' })).toBeNull();
    expect(app.agent?.id).toBe('a2');
    await expect.poll(() => [...document.querySelectorAll<HTMLElement>('.found')].map((e) => e.dataset.item)).toEqual(['u1']);
    expect(document.querySelector('.found')).toHaveTextContent('Ajoute la pagination');
  });

  it('counts an agent as seen once the board that hid it is closed', async () => {
    const focus = vi.spyOn(document, 'hasFocus').mockReturnValue(true);
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    app.openBoard('p1');
    await screen.findByRole('region', { name: 'À faire' });
    app.attention.a1 = true;
    app.closeBoard('p1');
    await expect.poll(() => app.attention.a1).toBeUndefined();
    focus.mockRestore();
  });
});
