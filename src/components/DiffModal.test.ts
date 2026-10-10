import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
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

describe('DiffModal with large diffs', () => {
  beforeEach(() => resetApp({ projects: [project()] }));
  const header = (path: string) => `diff --git a/${path} b/${path}\nindex 1111111..2222222 100644\n--- a/${path}\n+++ b/${path}\n`;
  const flagged = (path: string) => `${header(path)}Diff too large\n`;
  const long = (path: string, n: number) =>
    `${header(path)}@@ -0,0 +1,${n} @@\n${Array.from({ length: n }, (_, i) => `+ligne ${i + 1}`).join('\n')}\n`;

  it('says a file too large to be sent cannot be shown, and shows the others', async () => {
    fakeBackend({ git_diff: () => flagged('pnpm-lock.yaml') + DIFF });
    render(DiffModal, props);
    expect(await screen.findByText('Diff trop volumineux pour être affiché.')).toBeInTheDocument();
    // It has no counts, unlike the files whose diff came.
    const rows = screen.getAllByRole('button', { name: /^M / });
    expect(rows.map((r) => r.textContent)).toEqual([
      expect.stringMatching(/^M pnpm-lock\.yaml\s*$/),
      expect.stringMatching(/^M src\/auth\.ts\s*\+1−1$/),
    ]);
    expect(document.querySelector('.fhead')).not.toHaveTextContent('+0');
    await userEvent.click(screen.getByRole('button', { name: /new\.txt/ }));
    expect(screen.getByText('hello')).toBeInTheDocument();
    expect(screen.queryByText('Diff trop volumineux pour être affiché.')).not.toBeInTheDocument();
  });

  it('says it too for a commit', async () => {
    fakeBackend({ git_show: () => flagged('dump.sql') });
    render(DiffModal, { projectId: 'p1', agentId: null, paths: [], title: 'a1b2c3d import', commit: 'a1b2c3d4' });
    expect(await screen.findByText('Diff trop volumineux pour être affiché.')).toBeInTheDocument();
  });

  it('folds each long file on its own', async () => {
    fakeBackend({ git_diff: () => long('a.lock', 2000) + long('b.lock', 1600) });
    const { container } = render(DiffModal, props);
    expect(await screen.findByText('Diff volumineux (2001 lignes)')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Afficher' }));
    expect(container.querySelectorAll('.urow')).toHaveLength(500);
    await userEvent.click(screen.getByRole('button', { name: /b\.lock/ }));
    expect(screen.getByText('Diff volumineux (1601 lignes)')).toBeInTheDocument();
    expect(container.querySelectorAll('.urow')).toHaveLength(0);
  });

  it('keeps the keyboard inside the window when a long file is unfolded', async () => {
    fakeBackend({ git_diff: () => long('a.lock', 2000) });
    render(DiffModal, props);
    const show = await screen.findByRole('button', { name: 'Afficher' });
    show.focus();
    await userEvent.keyboard('{Enter}');
    const next = await screen.findByRole('button', { name: 'Afficher 500 lignes de plus' });
    await waitFor(() => expect(next).toHaveFocus());
    expect(screen.getByRole('dialog').contains(document.activeElement)).toBe(true);
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

describe('DiffModal in English', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    localStorage.removeItem('escouade.diffSplit');
    app.diffSplit = false;
    setLang('en');
  });

  it('writes its head, its switch and its close button in English', async () => {
    fakeBackend({ git_diff: () => DIFF });
    render(DiffModal, props);
    expect(await screen.findByText('const b = 3;')).toBeInTheDocument();
    expect(screen.getByText('2 files')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Unified' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Side by side' })).toBeInTheDocument();
    expect(screen.getByTitle('Close (Esc)')).toBeInTheDocument();
  });

  it('says a diff is empty or binary, in English', async () => {
    fakeBackend({ git_diff: () => '' });
    const { unmount } = render(DiffModal, props);
    expect(await screen.findByText('No differences.')).toBeInTheDocument();
    unmount();
    fakeBackend({ git_diff: () => 'diff --git a/logo.png b/logo.png\nBinary files a/logo.png and b/logo.png differ\n' });
    render(DiffModal, props);
    expect(await screen.findByText('Binary file.')).toBeInTheDocument();
  });

  it('says a diff is too large to be displayed, in English', async () => {
    const header =
      'diff --git a/pnpm-lock.yaml b/pnpm-lock.yaml\nindex 1111111..2222222 100644\n--- a/pnpm-lock.yaml\n+++ b/pnpm-lock.yaml\n';
    fakeBackend({ git_diff: () => `${header}Diff too large\n` });
    render(DiffModal, props);
    expect(await screen.findByText('This diff is too large to display.')).toBeInTheDocument();
  });
});
