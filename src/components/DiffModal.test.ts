import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import DiffModal from './DiffModal.svelte';

export const DIFF = `diff --git a/src/auth.ts b/src/auth.ts
--- a/src/auth.ts
+++ b/src/auth.ts
@@ -10,2 +10,2 @@
 const a = 1;
-const b = 2;
+const b = 3;
diff --git a/new.txt b/new.txt
new file mode 100644
--- /dev/null
+++ b/new.txt
@@ -0,0 +1 @@
+hello
`;

const props = { projectId: 'p1', agentId: 'a1', paths: [], title: 'Modifications de refacto-auth' };

describe('DiffModal', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    localStorage.removeItem('escouade.diffSplit');
    app.diffSplit = false;
  });

  it('shows the first file, and another one when picked in the list', async () => {
    const backend = fakeBackend({ git_diff: () => DIFF });
    render(DiffModal, props);
    expect(await screen.findByText('const b = 3;')).toBeInTheDocument();
    expect(backend.called('git_diff')[0].args).toEqual({ projectId: 'p1', agentId: 'a1', paths: [] });
    await userEvent.click(screen.getByRole('button', { name: /new\.txt/ }));
    expect(screen.getByText('hello')).toBeInTheDocument();
    expect(screen.queryByText('const b = 3;')).not.toBeInTheDocument();
  });

  it('shows old and new lines side by side and remembers the choice', async () => {
    fakeBackend({ git_diff: () => DIFF });
    render(DiffModal, props);
    await screen.findByText('const b = 3;');
    await userEvent.click(screen.getByRole('button', { name: 'Côte à côte' }));
    const row = screen.getByText('const b = 2;').closest('.srow')!;
    expect(row).toHaveTextContent('const b = 3;');
    expect(localStorage.getItem('escouade.diffSplit')).toBe('1');
    expect(app.diffSplit).toBe(true);
  });

  it('closes with Escape', async () => {
    fakeBackend({ git_diff: () => DIFF });
    app.modal = { kind: 'diff', ...props };
    render(DiffModal, props);
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
  });
});

describe('DiffModal for the whole project', () => {
  const diffOf = (path: string, line: string) => `diff --git a/${path} b/${path}
--- a/${path}
+++ b/${path}
@@ -1 +1 @@
-old
+${line}
`;
  const whole = { projectId: 'p1', agentId: null, paths: [], title: 'Modifications de demo-api', wholeProject: true };
  beforeEach(() => resetApp({ projects: [project()], agents: [agent({ id: 'a2', name: 'tests-e2e' })] }));

  it('shows the project’s files and each worktree’s, the latter named after their agent', async () => {
    const backend = fakeBackend({
      git_project_diff: () => [
        { agentId: null, inWorktree: false, diff: diffOf('README.md', 'racine') },
        { agentId: 'a2', inWorktree: true, diff: diffOf('README.md', 'worktree') + diffOf('src/wt.ts', 'seulement ici') },
      ],
    });
    render(DiffModal, whole);
    expect(await screen.findByText('racine')).toBeInTheDocument();
    expect(backend.called('git_project_diff')[0].args).toEqual({ projectId: 'p1' });
    expect(backend.called('git_diff')).toHaveLength(0);
    expect(screen.getByText('3 fichiers')).toBeInTheDocument();
    // Three rows, although two share a path: the same file in two checkouts is two files.
    const rows = screen.getAllByRole('button', { name: /^M / });
    expect(rows.map((r) => r.textContent)).toEqual([
      expect.stringMatching(/^M README\.md\s*\+1−1$/),
      expect.stringMatching(/^M README\.md\s*tests-e2e\s*\+1−1$/),
      expect.stringMatching(/^M src\/wt\.ts\s*tests-e2e\s*\+1−1$/),
    ]);
    await userEvent.click(rows[1]);
    expect(screen.getByText('worktree')).toBeInTheDocument();
    expect(screen.queryByText('racine')).not.toBeInTheDocument();
    await userEvent.click(rows[2]);
    expect(screen.getByText('seulement ici')).toBeInTheDocument();
  });

  it('names a file’s agent even when it is the only file', async () => {
    fakeBackend({ git_project_diff: () => [{ agentId: 'a2', inWorktree: true, diff: diffOf('src/wt.ts', 'seul') }] });
    render(DiffModal, whole);
    expect(await screen.findByText('seul')).toBeInTheDocument();
    expect(screen.getByText('tests-e2e')).toBeInTheDocument();
    expect(screen.queryByText('Aucune différence.')).not.toBeInTheDocument();
  });

  it('says there is no difference only when no checkout has any', async () => {
    fakeBackend({ git_project_diff: () => [] });
    render(DiffModal, whole);
    expect(await screen.findByText('Aucune différence.')).toBeInTheDocument();
  });
});

describe('DiffModal for a commit', () => {
  beforeEach(() => resetApp({ projects: [project()] }));

  it('shows what the commit changed', async () => {
    const backend = fakeBackend({ git_show: () => DIFF });
    render(DiffModal, { projectId: 'p1', agentId: null, paths: [], title: 'a1b2c3d ajoute les tests', commit: 'a1b2c3d4' });
    expect(await screen.findByText('const b = 3;')).toBeInTheDocument();
    expect(backend.called('git_show')[0].args).toEqual({ projectId: 'p1', hash: 'a1b2c3d4' });
    expect(backend.called('git_diff')).toHaveLength(0);
  });
});
