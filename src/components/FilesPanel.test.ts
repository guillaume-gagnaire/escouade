import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { commitAgentPrompt } from '../lib/agent-actions';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import type { FileChange } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import FilesPanel from './FilesPanel.svelte';

const change = (path: string, agentId: string | null = null, inWorktree = false): FileChange => ({
  path,
  status: 'M',
  add: 3,
  del: 1,
  agentId,
  inWorktree,
});
// A file's row (its diff), not the button that opens the file in the editor.
const row = (file: RegExp) => new RegExp(`^[AMD] ${file.source}`);
const settle = () => new Promise((r) => setTimeout(r, 200));
const diffOf = (path: string, line: string) => `diff --git a/${path} b/${path}
--- a/${path}
+++ b/${path}
@@ -1 +1 @@
-old
+${line}
`;

describe('FilesPanel', () => {
  beforeEach(() => resetApp({ projects: [project()], agents: [agent(), agent({ id: 'a2', name: 'tests-e2e' })] }));

  it('lists the agent’s files, then the whole project with the owning agent', async () => {
    const backend = fakeBackend({
      git_files: (a: any) => (a.agentId ? [change('src/auth.ts', 'a1')] : [change('src/auth.ts', 'a1'), change('README.md', 'a2')]),
    });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByText('auth.ts')).toBeInTheDocument();
    expect(backend.called('git_files').at(-1)?.args).toEqual({ projectId: 'p1', agentId: 'a1' });
    await userEvent.click(screen.getByRole('button', { name: 'Tout le projet' }));
    expect(await screen.findByText('README.md')).toBeInTheDocument();
    expect(screen.getByText('tests-e2e')).toBeInTheDocument();
  });

  it('does not refetch when the agent object is merely updated', async () => {
    const backend = fakeBackend({ git_files: () => [change('src/auth.ts', 'a1')] });
    const { rerender } = render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await settle();
    const before = backend.called('git_files').length;
    await rerender({ project: project(), agent: { ...app.agents.a1, tokens: 42 } });
    await settle();
    expect(backend.called('git_files').length).toBe(before);
  });

  it('ignores a slow response superseded by a newer one', async () => {
    let calls = 0;
    fakeBackend({
      git_files: () => {
        calls++;
        return calls === 1 ? new Promise((r) => setTimeout(() => r([change('old.ts', 'a1')]), 400)) : [change('new.ts', 'a1')];
      },
    });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await new Promise((r) => setTimeout(r, 150));
    app.gitTick++;
    await new Promise((r) => setTimeout(r, 600));
    expect(screen.getByText('new.ts')).toBeInTheDocument();
    expect(screen.queryByText('old.ts')).not.toBeInTheDocument();
  });
});

describe('FilesPanel « Commit… »', () => {
  beforeEach(() => resetApp({ projects: [project()], agents: [agent()] }));

  it('asks the agent to commit by default, and needs one for it', async () => {
    const backend = fakeBackend({ git_files: () => [change('src/auth.ts', 'a1')] });
    const { unmount } = render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await screen.findByText('auth.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    expect(backend.called('send_message')[0].args).toMatchObject({ id: 'a1', text: commitAgentPrompt() });
    expect(app.modal).toBeNull();
    unmount();

    app.filesScope = 'project';
    fakeBackend({ git_files: () => [change('README.md')] });
    render(FilesPanel, { project: project(), agent: null });
    await screen.findByText('README.md');
    expect(screen.getByRole('button', { name: 'Commit tout…' })).toBeDisabled();
  });

  it('opens the direct commit when the project says so, even without an agent', async () => {
    const direct = project({ commitMode: 'direct' });
    let backend = fakeBackend({ git_files: () => [change('src/auth.ts', 'a1')] });
    const { unmount } = render(FilesPanel, { project: direct, agent: app.agents.a1 });
    await screen.findByText('auth.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    expect(app.modal).toEqual({ kind: 'commit', projectId: 'p1', agentId: 'a1' });
    expect(backend.called('send_message')).toHaveLength(0);
    unmount();

    app.modal = null;
    app.filesScope = 'project';
    backend = fakeBackend({ git_files: () => [change('README.md')] });
    render(FilesPanel, { project: direct, agent: null });
    await screen.findByText('README.md');
    await userEvent.click(screen.getByRole('button', { name: 'Commit tout…' }));
    expect(app.modal).toEqual({ kind: 'commit', projectId: 'p1', agentId: null });
    expect(backend.called('send_message')).toHaveLength(0);
  });
});

describe('FilesPanel « Voir le diff »', () => {
  const wt = { path: 'C:/wt', branch: 'ccm/wt', baseBranch: 'main' };
  beforeEach(() => resetApp({ projects: [project()], agents: [agent(), agent({ id: 'a2', name: 'wt-agent', worktree: wt })] }));

  it('covers the project and every agent’s worktree for the whole project', async () => {
    fakeBackend({ git_files: () => [change('README.md', null), change('src/wt.ts', 'a2', true)] });
    app.filesScope = 'project';
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await screen.findByText('wt.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Voir le diff' }));
    expect(app.modal).toMatchObject({ kind: 'diff', projectId: 'p1', title: 'Modifications de demo-api', wholeProject: true });
  });

  it('reads one checkout for an agent: its worktree, or the files it edited in the project’s', async () => {
    fakeBackend({ git_files: () => [change('src/auth.ts', 'a1'), change('new.txt', 'a1')] });
    const { unmount } = render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await screen.findByText('auth.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Voir le diff' }));
    expect(app.modal).toMatchObject({
      kind: 'diff',
      agentId: 'a1',
      paths: ['src/auth.ts', 'new.txt'],
      title: 'Modifications de refacto-auth',
    });
    expect(app.modal).not.toHaveProperty('wholeProject', true);
    unmount();

    fakeBackend({ git_files: () => [change('src/wt.ts', 'a2', true)] });
    render(FilesPanel, { project: project(), agent: app.agents.a2 });
    await screen.findByText('wt.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Voir le diff' }));
    expect(app.modal).toMatchObject({ kind: 'diff', agentId: 'a2', paths: [], title: 'Modifications de wt-agent' });
    expect(app.modal).not.toHaveProperty('wholeProject', true);
  });
});

describe('FilesPanel docked in the split layout', () => {
  beforeEach(() =>
    resetApp({
      projects: [project()],
      agents: [agent(), agent({ id: 'a2', name: 'wt-agent', worktree: { path: 'C:/wt', branch: 'ccm/wt', baseBranch: 'main' } })],
    }),
  );

  it('shows the first file’s diff right away, and another file’s diff in place when picked', async () => {
    const backend = fakeBackend({
      git_files: () => [change('src/auth.ts', 'a1'), change('new.txt', 'a1')],
      git_diff: (a: any) => diffOf(a.paths[0], `contenu de ${a.paths[0]}`),
    });
    render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(await screen.findByText('contenu de src/auth.ts')).toBeInTheDocument();
    expect(backend.called('git_diff').at(-1)?.args).toEqual({ projectId: 'p1', agentId: null, paths: ['src/auth.ts'] });
    await userEvent.click(screen.getByRole('button', { name: row(/new\.txt/) }));
    expect(await screen.findByText('contenu de new.txt')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: row(/new\.txt/) })).toHaveAttribute('aria-current', 'true');
    expect(app.modal).toBeNull();
    expect(screen.queryByTitle('Fermer')).not.toBeInTheDocument();
  });

  it('keeps the picked file across refreshes and falls back to the first one when it is gone', async () => {
    let files = [change('src/auth.ts', 'a1'), change('new.txt', 'a1')];
    let version = 1;
    fakeBackend({ git_files: () => files, git_diff: (a: any) => diffOf(a.paths[0], `${a.paths[0]} v${version}`) });
    render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    await userEvent.click(await screen.findByRole('button', { name: row(/new\.txt/) }));
    expect(await screen.findByText('new.txt v1')).toBeInTheDocument();
    version = 2;
    app.gitTick++;
    expect(await screen.findByText('new.txt v2')).toBeInTheDocument();
    files = [change('src/auth.ts', 'a1')];
    app.gitTick++;
    expect(await screen.findByText('src/auth.ts v2')).toBeInTheDocument();
  });

  it('reads a project-wide file from the worktree of the agent that owns it', async () => {
    const backend = fakeBackend({
      git_files: (a: any) => (a.agentId ? [] : [change('src/wt.ts', 'a2', true), change('README.md', 'a1')]),
      git_diff: (a: any) => diffOf(a.paths[0], 'x'),
    });
    app.filesScope = 'project';
    render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    await screen.findAllByText('x');
    expect(backend.called('git_diff').at(-1)?.args).toEqual({ projectId: 'p1', agentId: 'a2', paths: ['src/wt.ts'] });
    await userEvent.click(screen.getByRole('button', { name: row(/README\.md/) }));
    await settle();
    expect(backend.called('git_diff').at(-1)?.args).toEqual({ projectId: 'p1', agentId: null, paths: ['README.md'] });
  });

  it('switches the diff to side by side', async () => {
    fakeBackend({ git_files: () => [change('src/auth.ts', 'a1')], git_diff: () => diffOf('src/auth.ts', 'new') });
    render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    await screen.findByText('new');
    await userEvent.click(screen.getByRole('button', { name: 'Côte à côte' }));
    expect(app.diffSplit).toBe(true);
    expect(screen.getByText('old').closest('.srow')).toHaveTextContent('new');
  });

  it('ignores a slow diff superseded by a newer one', async () => {
    let calls = 0;
    fakeBackend({
      git_files: () => [change('src/auth.ts', 'a1')],
      git_diff: () => {
        calls++;
        return calls === 1
          ? new Promise((r) => setTimeout(() => r(diffOf('src/auth.ts', 'ancien')), 500))
          : diffOf('src/auth.ts', 'récent');
      },
    });
    render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    await new Promise((r) => setTimeout(r, 200));
    app.gitTick++;
    await new Promise((r) => setTimeout(r, 700));
    expect(screen.getByText('récent')).toBeInTheDocument();
    expect(screen.queryByText('ancien')).not.toBeInTheDocument();
  });
});

describe('FilesPanel docked with a binary file', () => {
  beforeEach(() => resetApp({ projects: [project()], agents: [agent()] }));

  it('says the file is binary instead of showing an empty diff', async () => {
    fakeBackend({
      git_files: () => [change('logo.png', 'a1')],
      git_diff: () => 'diff --git a/logo.png b/logo.png\nBinary files a/logo.png and b/logo.png differ\n',
    });
    render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(await screen.findByText('Fichier binaire.')).toBeInTheDocument();
  });
});

describe('FilesPanel docked: the file being read stays put', () => {
  beforeEach(() => resetApp({ projects: [project()], agents: [agent(), agent({ id: 'a2', name: 'tests-e2e' })] }));

  it('keeps showing the default file when the agent touches one that sorts before it', async () => {
    let files = [change('src/z.ts', 'a1')];
    fakeBackend({ git_files: () => files, git_diff: (a: any) => diffOf(a.paths[0], `diff de ${a.paths[0]}`) });
    render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(await screen.findByText('diff de src/z.ts')).toBeInTheDocument();
    files = [change('src/a.ts', 'a1'), change('src/z.ts', 'a1')];
    app.gitTick++;
    expect(await screen.findByRole('button', { name: row(/a\.ts/) })).toBeInTheDocument();
    await settle();
    expect(screen.getByText('diff de src/z.ts')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: row(/z\.ts/) })).toHaveAttribute('aria-current', 'true');
  });

  it('never asks for another agent’s file when switching agents', async () => {
    const backend = fakeBackend({
      git_files: (a: any) => [change(a.agentId === 'a1' ? 'src/one.ts' : 'src/two.ts', a.agentId, true)],
      git_diff: (a: any) => diffOf(a.paths[0], `diff de ${a.paths[0]}`),
    });
    const { rerender } = render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(await screen.findByText('diff de src/one.ts')).toBeInTheDocument();
    await rerender({ project: project(), agent: app.agents.a2, docked: true });
    expect(await screen.findByText('diff de src/two.ts')).toBeInTheDocument();
    const pairs = backend.called('git_diff').map((c) => `${c.args.agentId}:${c.args.paths[0]}`);
    expect(pairs).not.toContain('a2:src/one.ts');
  });
});

describe('FilesPanel with thousands of files', () => {
  beforeEach(() => resetApp({ projects: [project()], agents: [agent()] }));
  const many = (n: number) => Array.from({ length: n }, (_, i) => change(`src/f${i}.ts`, 'a1'));
  const rowsOf = (container: HTMLElement) => container.querySelectorAll('.filerow').length;

  it('lists 500 files at most, then says how many more there are', async () => {
    fakeBackend({ git_files: () => many(620) });
    const { container } = render(FilesPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByText('f0.ts')).toBeInTheDocument();
    expect(rowsOf(container)).toBe(500);
    expect(screen.getByText('f499.ts')).toBeInTheDocument();
    expect(screen.queryByText('f500.ts')).not.toBeInTheDocument();
    expect(screen.getByText('… et 120 autres fichiers')).toBeInTheDocument();
  });

  it('says one more file in the singular', async () => {
    fakeBackend({ git_files: () => many(501) });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByText('… et 1 autre fichier')).toBeInTheDocument();
  });

  it('lists 500 files without a remainder', async () => {
    fakeBackend({ git_files: () => many(500) });
    const { container } = render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await screen.findByText('f499.ts');
    expect(rowsOf(container)).toBe(500);
    expect(screen.queryByText(/autres? fichiers?/)).not.toBeInTheDocument();
  });

  it('still reads the diff of every file, not only the listed ones', async () => {
    fakeBackend({ git_files: () => many(620) });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await screen.findByText('f0.ts');
    // By its text, not its role: working out the accessible name of the 500 rows' buttons takes
    // seconds on a slow machine.
    await userEvent.click(screen.getByText('Voir le diff', { selector: 'button' }));
    expect(app.modal).toMatchObject({ kind: 'diff', agentId: 'a1' });
    expect((app.modal as Extract<typeof app.modal, { kind: 'diff' }>).paths).toHaveLength(620);
  });
});

describe('FilesPanel docked with a large diff', () => {
  beforeEach(() => resetApp({ projects: [project()], agents: [agent()] }));
  const header =
    'diff --git a/pnpm-lock.yaml b/pnpm-lock.yaml\nindex 1111111..2222222 100644\n--- a/pnpm-lock.yaml\n+++ b/pnpm-lock.yaml\n';
  const longDiff = (n: number) => `${header}@@ -0,0 +1,${n} @@\n${Array.from({ length: n }, (_, i) => `+ligne ${i + 1}`).join('\n')}\n`;

  it('says a file too large to be sent cannot be shown', async () => {
    fakeBackend({ git_files: () => [change('pnpm-lock.yaml', 'a1')], git_diff: () => `${header}Diff too large\n` });
    render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(await screen.findByText('Diff trop volumineux pour être affiché.')).toBeInTheDocument();
    // No counts to show: the list has the file’s own.
    expect(screen.getByLabelText('Diff de pnpm-lock.yaml')).not.toHaveTextContent('+0');
  });

  it('folds a long diff, and keeps it open while the agent keeps editing', async () => {
    fakeBackend({ git_files: () => [change('pnpm-lock.yaml', 'a1')], git_diff: () => longDiff(2000) });
    const { container } = render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    await userEvent.click(await screen.findByRole('button', { name: 'Afficher' }));
    expect(container.querySelectorAll('.urow')).toHaveLength(500);
    app.gitTick++;
    await settle();
    expect(container.querySelectorAll('.urow')).toHaveLength(500);
    expect(screen.getByRole('button', { name: 'Afficher 500 lignes de plus' })).toBeInTheDocument();
  });

  it('folds a long diff again for another file', async () => {
    fakeBackend({ git_files: () => [change('a.lock', 'a1'), change('b.lock', 'a1')], git_diff: () => longDiff(2000) });
    const { container } = render(FilesPanel, { project: project(), agent: app.agents.a1, docked: true });
    await userEvent.click(await screen.findByRole('button', { name: 'Afficher' }));
    expect(container.querySelectorAll('.urow')).toHaveLength(500);
    await userEvent.click(screen.getByRole('button', { name: row(/b\.lock/) }));
    expect(await screen.findByText('Diff volumineux (2001 lignes)')).toBeInTheDocument();
    expect(container.querySelectorAll('.urow')).toHaveLength(0);
  });
});

describe('FilesPanel context menu', () => {
  beforeEach(() => {
    resetApp({
      projects: [project()],
      agents: [agent(), agent({ id: 'a2', name: 'wt-agent', worktree: { path: 'C:/wt', branch: 'ccm/wt', baseBranch: 'main' } })],
    });
    menu.close();
  });
  const entries = () => menu.open?.items.filter((i) => !i.separator) ?? [];
  const click = (label: string) => entries().find((i) => i.label === label)!.onClick!();
  const rightClick = async (file: RegExp) => fireEvent.contextMenu(await screen.findByRole('button', { name: row(file) }));

  it('acts in the checkout a row comes from, even while another agent’s list is loading', async () => {
    let release: (files: FileChange[]) => void = () => {};
    const backend = fakeBackend({
      git_files: (a: any) => (a.agentId === 'a2' ? [change('src/wt.ts', 'a2', true)] : new Promise((r) => (release = r))),
    });
    const { rerender } = render(FilesPanel, { project: project(), agent: app.agents.a2 });
    await screen.findByRole('button', { name: row(/wt\.ts/) });
    await rerender({ project: project(), agent: app.agents.a1 });
    await settle(); // a1's list is not there yet: the rows are still a2's
    await rightClick(/wt\.ts/);
    expect(entries().map((i) => i.label)).toEqual(['Ouvrir dans l’éditeur', 'Abandonner les modifications…']);
    click('Abandonner les modifications…');
    await (app.modal as Extract<typeof app.modal, { kind: 'confirm' }>).onConfirm(false);
    expect(backend.called('git_discard')[0].args).toEqual({ projectId: 'p1', agentId: 'a2', path: 'src/wt.ts' });
    release([]);
  });

  it('reverts a modified file once confirmed', async () => {
    const backend = fakeBackend({ git_files: () => [change('src/auth.ts', 'a1')] });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await rightClick(/auth\.ts/);
    click('Abandonner les modifications…');
    expect(backend.called('git_discard')).toHaveLength(0);
    expect(app.modal).toMatchObject({ kind: 'confirm', danger: true, title: 'Abandonner les modifications de « auth.ts » ?' });
    await (app.modal as Extract<typeof app.modal, { kind: 'confirm' }>).onConfirm(false);
    expect(backend.called('git_discard')[0].args).toEqual({ projectId: 'p1', agentId: null, path: 'src/auth.ts' });
  });

  it('deletes a new file once confirmed, and restores a deleted one right away', async () => {
    const backend = fakeBackend({
      git_files: () => [
        { ...change('new.txt', 'a1'), status: 'A' },
        { ...change('gone.ts', 'a1'), status: 'D' },
      ],
    });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await rightClick(/new\.txt/);
    click('Supprimer le fichier…');
    expect(app.modal).toMatchObject({ kind: 'confirm', danger: true, title: 'Supprimer « new.txt » ?' });
    app.modal = null;
    await rightClick(/gone\.ts/);
    // A deleted file cannot be opened.
    expect(entries().find((i) => i.label === 'Ouvrir dans l’éditeur')?.disabled).toBe(true);
    click('Restaurer le fichier');
    expect(app.modal).toBeNull();
    expect(backend.called('git_discard')[0].args).toEqual({ projectId: 'p1', agentId: null, path: 'gone.ts' });
  });
});

describe('FilesPanel editor entry', () => {
  it('opens a file in the editor from its row button, and still shows its diff on click', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a2', branch: 'escouade/a2', baseBranch: 'main' };
    resetApp({ agents: [agent({ id: 'a2', name: 'wt', worktree: wt })] });
    app.ui.selectedAgent.p1 = 'a2';
    fakeBackend({ git_files: () => [{ path: 'src/wt.ts', status: 'M', add: 1, del: 0, agentId: 'a2', inWorktree: true }] });
    render(FilesPanel, { project: project(), agent: app.agents.a2 });
    await userEvent.click(await screen.findByRole('button', { name: 'Ouvrir wt.ts dans l’éditeur' }));
    expect(app.editor.p1).toMatchObject({ on: true, source: 'a2' });
    expect(app.editor.p1.places.a2.active).toBe('src/wt.ts');
    await userEvent.click(screen.getByRole('button', { name: /wt\.ts.*src/ }));
    expect(app.modal).toMatchObject({ kind: 'diff', paths: ['src/wt.ts'] });
  });

  it('opens a file of the project checkout on the project, from its menu too, and offers nothing for a deleted file', async () => {
    resetApp({ agents: [agent()] });
    menu.close();
    fakeBackend({
      git_files: () => [change('src/auth.ts', 'a1'), { ...change('gone.ts', 'a1'), status: 'D' }],
    });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await userEvent.click(await screen.findByRole('button', { name: 'Ouvrir auth.ts dans l’éditeur' }));
    expect(app.editor.p1).toMatchObject({ on: true, source: 'project' });
    expect(app.editor.p1.places.project.active).toBe('src/auth.ts');
    expect(screen.queryByRole('button', { name: 'Ouvrir gone.ts dans l’éditeur' })).not.toBeInTheDocument();
    app.closeEditor();
    await fireEvent.contextMenu(screen.getByRole('button', { name: row(/auth\.ts/) }));
    menu.open!.items.find((i) => i.label === 'Ouvrir dans l’éditeur')!.onClick!();
    expect(app.editorOn).toBe(true);
  });

  it('opens the same menu from a right click on the row’s editor button', async () => {
    resetApp({ agents: [agent()] });
    menu.close();
    fakeBackend({ git_files: () => [change('src/auth.ts', 'a1')] });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await fireEvent.contextMenu(await screen.findByRole('button', { name: 'Ouvrir auth.ts dans l’éditeur' }));
    expect(menu.open?.items.filter((i) => !i.separator).map((i) => i.label)).toEqual([
      'Ouvrir dans l’éditeur',
      'Abandonner les modifications…',
    ]);
  });
});

describe('FilesPanel in English', () => {
  beforeEach(() => {
    resetApp({ projects: [project()], agents: [agent(), agent({ id: 'a2', name: 'tests-e2e' })] });
    menu.close();
    setLang('en');
  });
  const many = (n: number) => Array.from({ length: n }, (_, i) => change(`src/f${i}.ts`, 'a1'));

  it('writes its scopes, its hints and its buttons in English', async () => {
    fakeBackend({ git_files: () => [change('src/auth.ts', 'a1')] });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByText('auth.ts')).toBeInTheDocument();
    expect(screen.getByText('Files changed by this agent (from its edits)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Show diff' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Commit…' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Open auth.ts in editor' })).toHaveAttribute('title', 'Open in editor');
    await userEvent.click(screen.getByRole('button', { name: 'Whole project' }));
    expect(screen.getByText('All the project’s agents · attributed by worktree')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Commit all…' })).toBeInTheDocument();
  });

  it('says it is empty, in English', async () => {
    fakeBackend({ git_files: () => [] });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByText('No files changed by this agent.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Whole project' }));
    expect(await screen.findByText('No uncommitted changes.')).toBeInTheDocument();
  });

  it('counts the files it does not list with the plural and the digits of English', async () => {
    fakeBackend({ git_files: () => many(1502) });
    const { unmount } = render(FilesPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByText('… and 1,002 more files')).toBeInTheDocument();
    unmount();
    fakeBackend({ git_files: () => many(501) });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByText('… and 1 more file')).toBeInTheDocument();
  });

  it('names the worktree of an agent, and the merge, in English', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\landing', branch: 'ccm/landing', baseBranch: 'main' };
    resetApp({ agents: [agent({ id: 'a3', name: 'landing', worktree: wt })] });
    fakeBackend({ git_files: () => [change('src/auth.ts', 'a3', true)] });
    render(FilesPanel, { project: project(), agent: app.agents.a3 });
    expect(await screen.findByText('worktree .claude/worktrees/landing · isolated from the other agents')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Merge ccm/landing → main…' })).toBeInTheDocument();
  });

  it('asks in English before discarding the changes of a file, deleting a new one', async () => {
    fakeBackend({
      git_files: () => [
        change('src/auth.ts', 'a1'),
        { ...change('new.txt', 'a1'), status: 'A' },
        { ...change('gone.ts', 'a1'), status: 'D' },
      ],
    });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    const entries = () => menu.open!.items.filter((i) => !i.separator);
    const rightClick = async (file: RegExp) => fireEvent.contextMenu(await screen.findByRole('button', { name: row(file) }));
    await rightClick(/auth\.ts/);
    expect(entries().map((i) => i.label)).toEqual(['Open in editor', 'Discard changes…']);
    entries()[1].onClick!();
    expect(app.modal).toMatchObject({
      title: 'Discard the changes to “auth.ts”?',
      body: 'src/auth.ts goes back to its state at the last commit: its uncommitted changes are lost.',
      confirm: 'Discard changes',
    });
    await rightClick(/new\.txt/);
    expect(entries()[1].label).toBe('Delete the file…');
    entries()[1].onClick!();
    expect(app.modal).toMatchObject({ title: 'Delete “new.txt”?', confirm: 'Delete' });
    await rightClick(/gone\.ts/);
    expect(entries()[1].label).toBe('Restore the file');
  });

  it('sends the agent the commit request in English, and says so', async () => {
    // The request follows the language of the texts for Claude, the toast the interface's.
    app.lang = { ui: 'en', system: 'fr', claude: 'en' };
    const backend = fakeBackend({ git_files: () => [change('src/auth.ts', 'a1')] });
    render(FilesPanel, { project: project(), agent: app.agents.a1 });
    await screen.findByText('auth.ts');
    await userEvent.click(screen.getByRole('button', { name: 'Commit…' }));
    await settle();
    expect(backend.called('send_message')[0].args.text).toMatch(/^Commit the changes you made in this repository/);
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Commit request sent to refacto-auth', kind: 'ok' });
  });
});
