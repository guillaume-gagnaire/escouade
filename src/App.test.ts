import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import { app } from './lib/state.svelte';
import type { InitialState } from './lib/types';
import { agent, fakeBackend, project, resetApp, SETTINGS } from './test/ipc';

const DIFF = `diff --git a/src/auth.ts b/src/auth.ts
--- a/src/auth.ts
+++ b/src/auth.ts
@@ -1 +1 @@
-const b = 2;
+const b = 3;
`;

function start(layout: '' | 'split') {
  const initial: InitialState = {
    projects: [project()],
    agents: [agent()],
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

  it('keeps the files panel closed in the classic layout until asked for', async () => {
    start('');
    expect(await screen.findByRole('main')).toBeInTheDocument();
    expect(screen.queryByText('Non commités')).not.toBeInTheDocument();
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
    expect(await screen.findByRole('dialog', { name: 'Réglages du tableau' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toBeInTheDocument();
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
