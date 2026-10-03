import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
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
});
