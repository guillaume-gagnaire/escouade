import { EditorView as CodeMirror } from '@codemirror/view';
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { buffers } from '../../lib/editor/buffers.svelte';
import { trees } from '../../lib/editor/trees.svelte';
import { menu } from '../../lib/menu.svelte';
import { handleShortcut } from '../../lib/shortcuts';
import { app } from '../../lib/state.svelte';
import { openTerminal } from '../../lib/terminals';
import { agent, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import '../../test/pointer';
import QuickOpen from '../QuickOpen.svelte';
import EditorView from './EditorView.svelte';

// The xterm.js instance a terminal draws on is not one of jsdom's: only where it is asked to open is looked at.
vi.mock('../../lib/terminals', () => ({
  openTerminal: vi.fn((projectId: string, shell: string, name: string) => Promise.resolve({ id: 't1', projectId, name, shell })),
  disposeTerminal() {},
}));

const text = (t: string, hash = 'h1') => ({ kind: 'text', text: t, size: t.length, hash, eol: 'lf', bom: false });

function backend(over: Record<string, (a: any) => unknown> = {}) {
  return fakeBackend({
    fs_tree: () => ({ root: 'C:/code/demo-api', files: ['README.md', 'src/app.ts'], truncated: false }),
    git_files: () => [{ path: 'src/app.ts', status: 'M', add: 1, del: 0, agentId: null, inWorktree: false }],
    fs_read: (a) => text(a.path === 'README.md' ? '# demo\n' : 'const a = 2;\n'),
    fs_base: () => ({ reference: 'HEAD', text: 'const a = 1;\n' }),
    fs_write: () => 'h2',
    set_unsaved: () => null,
    ...over,
  });
}

describe('EditorView', () => {
  beforeEach(() => {
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo({ modified: 1 });
  });

  it('opens on the first changed file, with its tree, tab and comparison', async () => {
    backend();
    await app.openEditor({ source: 'project' });
    render(EditorView, { project: project() });
    expect(await screen.findByRole('tab', { name: /app\.ts/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('treeitem', { name: /src/ })).toHaveAttribute('aria-expanded', 'true');
    expect(await screen.findByText('1 ligne modifiée vs HEAD')).toBeInTheDocument();
    expect(screen.getByText('TypeScript')).toBeInTheDocument();
    expect(screen.getByText('Enregistré')).toBeInTheDocument();
  });

  it('saves what was typed', async () => {
    const be = backend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('tab', { name: /app\.ts/ });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.kind).toBe('text');
    buffers.edit(key, 'const a = 3;\n');
    expect(await screen.findByText(/● Non enregistré/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(be.called('fs_write')[0].args).toMatchObject({ path: 'src/app.ts', text: 'const a = 3;\n', expectedHash: 'h1' });
    expect(await screen.findByText('Enregistré')).toBeInTheDocument();
  });

  it('warns when the file changed on disk, and reloads it', async () => {
    backend({ fs_write: () => Promise.reject('changed') });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.kind).toBe('text');
    buffers.edit(key, 'mine\n');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Ce fichier a changé sur le disque.');
    await userEvent.click(screen.getByRole('button', { name: 'Recharger' }));
    expect(buffers.all[key].text).toBe('const a = 2;\n');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('asks before closing an unsaved tab', async () => {
    backend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.kind).toBe('text');
    buffers.edit(key, 'mine\n');
    await userEvent.click(screen.getByRole('button', { name: 'Fermer app.ts' }));
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Enregistrer « app.ts » ?', confirm: 'Enregistrer' });
    await (app.modal as any).alt.onClick();
    expect(app.editor.p1.places.project.open).toEqual([]);
    expect(buffers.all[key]).toBeUndefined();
  });

  it('goes back to the conversation', async () => {
    backend();
    await app.openEditor({ source: 'project' });
    render(EditorView, { project: project() });
    await userEvent.click(screen.getByRole('button', { name: '← Conversation' }));
    expect(app.editorOn).toBe(false);
  });

  it('reads a tab again when it is shown again', async () => {
    const be = backend();
    await app.openEditor({ source: 'project', path: 'README.md' });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('tab', { name: /app\.ts/ });
    // The first refresh (tree, status, open files) is over once the comparison shows.
    await screen.findByText('1 ligne modifiée vs HEAD');
    const reads = () => be.called('fs_read').filter((c) => c.args.path === 'README.md').length;
    await userEvent.click(screen.getByRole('tab', { name: /README\.md/ }));
    await expect.poll(reads).toBe(1);
    await userEvent.click(screen.getByRole('tab', { name: /app\.ts/ }));
    await userEvent.click(screen.getByRole('tab', { name: /README\.md/ }));
    await expect.poll(reads).toBe(2);
  });

  it('tells the tabs of files of the same name apart by their folder', async () => {
    backend({
      fs_tree: () => ({ root: 'C:/code/demo-api', files: ['src/board/index.ts', 'src/editor/index.ts', 'README.md'], truncated: false }),
      git_files: () => [],
    });
    await app.openEditor({ source: 'project', path: 'src/editor/index.ts' });
    await app.openEditor({ source: 'project', path: 'src/board/index.ts' });
    await app.openEditor({ source: 'project', path: 'README.md' });
    render(EditorView, { project: project() });
    expect(await screen.findByRole('tab', { name: /index\.ts · editor/ })).toHaveAttribute('title', 'src/editor/index.ts');
    expect(screen.getByRole('tab', { name: /index\.ts · board/ })).toHaveAttribute('title', 'src/board/index.ts');
    expect(screen.getByRole('tab', { name: /README\.md/ })).not.toHaveTextContent('·');
  });

  it('says what a file without text is', async () => {
    backend({ fs_read: () => ({ kind: 'binary', text: null, size: 10, hash: '', eol: 'lf', bom: false }) });
    await app.openEditor({ source: 'project', path: 'logo.png' });
    render(EditorView, { project: project() });
    expect(await screen.findByText('Fichier binaire : pas d’aperçu.')).toBeInTheDocument();
  });

  it('does not open a file once the view was left', async () => {
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    backend({
      fs_tree: async () => {
        await gate;
        return { root: 'C:/code/demo-api', files: ['README.md', 'src/app.ts'], truncated: false };
      },
    });
    await app.openEditor({ source: 'project' });
    const { unmount } = render(EditorView, { project: project() });
    // The view is gone (back to the conversation, or another project) before the tree arrives.
    unmount();
    app.closeEditor('p1');
    release();
    await expect.poll(() => trees.get('p1', 'project')).toBeDefined();
    await new Promise((r) => setTimeout(r, 30));
    expect(app.editorOn).toBe(false);
    expect(app.editor.p1.places.project.open).toEqual([]);
  });

  it('keeps an empty editor empty while git events come in', async () => {
    const be = backend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    // The first refresh (tree, status, open files) is over once the comparison shows.
    await screen.findByText('1 ligne modifiée vs HEAD');
    await userEvent.click(screen.getByRole('button', { name: 'Fermer app.ts' }));
    expect(screen.getByText('Sélectionne un fichier dans l’arborescence.')).toBeInTheDocument();
    const listed = be.called('git_files').length;
    app.gitTick++;
    await expect.poll(() => be.called('git_files').length).toBe(listed + 1);
    await new Promise((r) => setTimeout(r, 50));
    expect(screen.getByText('Sélectionne un fichier dans l’arborescence.')).toBeInTheDocument();
    expect(screen.queryByRole('tab')).not.toBeInTheDocument();
  });

  it('saves from the close prompt, then closes the tab', async () => {
    const be = backend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.kind).toBe('text');
    buffers.edit(key, 'mine\n');
    await userEvent.click(screen.getByRole('button', { name: 'Fermer app.ts' }));
    await (app.modal as any).onConfirm(false);
    expect(be.called('fs_write')[0].args).toMatchObject({ path: 'src/app.ts', text: 'mine\n', expectedHash: 'h1' });
    expect(app.editor.p1.places.project.open).toEqual([]);
    expect(buffers.all[key]).toBeUndefined();
  });

  it('shows why a save from the close prompt was refused, on the tab it concerns', async () => {
    let onDisk = 'h1';
    backend({
      fs_read: (a) => text(a.path === 'README.md' ? '# demo\n' : 'const a = 2;\n', a.path === 'README.md' ? onDisk : 'h1'),
      fs_write: () => {
        onDisk = 'h9';
        return Promise.reject('changed');
      },
    });
    await app.openEditor({ source: 'project', path: 'README.md' });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const readme = buffers.key('p1', 'project', 'README.md');
    await userEvent.click(screen.getByRole('tab', { name: /README\.md/ }));
    await expect.poll(() => buffers.all[readme]?.kind).toBe('text');
    buffers.edit(readme, 'mine\n');
    // README.md goes to the background: its banner is not on screen.
    await userEvent.click(screen.getByRole('tab', { name: /app\.ts/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Fermer README.md' }));
    await (app.modal as any).onConfirm(false);
    expect(await screen.findByRole('alert')).toHaveTextContent('Ce fichier a changé sur le disque.');
    expect(screen.getByRole('tab', { name: /README\.md/ })).toHaveAttribute('aria-selected', 'true');
    expect(app.editor.p1.places.project.open).toContain('README.md');
  });

  it('saves over what changed on disk with "Garder ma version"', async () => {
    const be = backend({ fs_write: (a) => (a.expectedHash === null ? 'h3' : Promise.reject('changed')) });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.kind).toBe('text');
    buffers.edit(key, 'mine\n');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Ce fichier a changé sur le disque.');
    await userEvent.click(screen.getByRole('button', { name: 'Garder ma version' }));
    await expect.poll(() => be.called('fs_write').length).toBe(2);
    expect(be.called('fs_write')[1].args).toMatchObject({ path: 'src/app.ts', text: 'mine\n', expectedHash: null });
    await expect.poll(() => screen.queryByRole('alert')).toBeNull();
    expect(await screen.findByText('Enregistré')).toBeInTheDocument();
  });

  it('closes the tab of a deleted file from its banner', async () => {
    let gone = false;
    backend({ fs_read: () => (gone ? Promise.reject('fichier introuvable') : text('const a = 2;\n')) });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.kind).toBe('text');
    gone = true;
    await buffers.refresh(key);
    expect(await screen.findByRole('alert')).toHaveTextContent('Ce fichier a été supprimé.');
    await userEvent.click(screen.getByRole('button', { name: 'Fermer' }));
    expect(app.editor.p1.places.project.open).toEqual([]);
    expect(buffers.all[key]).toBeUndefined();
  });

  it('creates a deleted file again with "Enregistrer", even without a change', async () => {
    let gone = false;
    const be = backend({
      fs_read: () => (gone ? Promise.reject('fichier introuvable') : text('const a = 2;\n')),
      // As the backend does: a write expecting the file it read is refused once that file is gone.
      fs_write: (a) => {
        if (gone && a.expectedHash) return Promise.reject('deleted');
        gone = false;
        return 'h5';
      },
    });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.kind).toBe('text');
    gone = true;
    await buffers.refresh(key);
    expect(await screen.findByRole('alert')).toHaveTextContent('Ce fichier a été supprimé.');
    const save = screen.getByRole('button', { name: 'Enregistrer' });
    expect(save).toBeEnabled();
    await userEvent.click(save);
    await expect.poll(() => be.called('fs_write').length).toBe(1);
    expect(be.called('fs_write')[0].args).toMatchObject({ path: 'src/app.ts', text: 'const a = 2;\n', expectedHash: null });
    await expect.poll(() => screen.queryByRole('alert')).toBeNull();
  });

  it('reads a file again that was missing, when its tab is shown again', async () => {
    let there = false;
    backend({ fs_read: (a) => (a.path === 'README.md' && !there ? Promise.reject('fichier introuvable') : text('# demo\n')) });
    await app.openEditor({ source: 'project', path: 'README.md' });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByText('1 ligne modifiée vs HEAD');
    await userEvent.click(screen.getByRole('tab', { name: /README\.md/ }));
    expect(await screen.findByText('Ce fichier n’existe pas (ou plus).')).toBeInTheDocument();
    there = true;
    await userEvent.click(screen.getByRole('tab', { name: /app\.ts/ }));
    await userEvent.click(screen.getByRole('tab', { name: /README\.md/ }));
    await expect.poll(() => buffers.all[buffers.key('p1', 'project', 'README.md')]?.kind).toBe('text');
  });

  it('names the worktree folder of an agent source', async () => {
    resetApp({
      agents: [agent({ worktree: { path: 'C:\\code\\demo-api\\.claude\\worktrees\\wt-x', branch: 'escouade/wt-x', baseBranch: 'main' } })],
    });
    backend();
    await app.openEditor({ source: 'a1', path: 'README.md' });
    render(EditorView, { project: project() });
    expect(await screen.findByText('.claude/worktrees/wt-x')).toBeInTheDocument();
  });

  it('never says "branche" without a branch name', async () => {
    app.git.p1 = gitInfo({ branch: '' });
    backend();
    await app.openEditor({ source: 'project', path: 'README.md' });
    render(EditorView, { project: project() });
    expect(await screen.findByText('branche · projet')).toBeInTheDocument();
  });

  it('closes the tab of the source it was asked on, even if the source changed meanwhile', async () => {
    backend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.kind).toBe('text');
    buffers.edit(key, 'mine\n');
    await userEvent.click(screen.getByRole('button', { name: 'Fermer app.ts' }));
    await app.openEditor({ projectId: 'p1', source: 'a1' });
    await (app.modal as any).alt.onClick();
    expect(app.editor.p1.places.project.open).toEqual([]);
    expect(buffers.all[key]).toBeUndefined();
  });

  it('tells that the tree could not be read once, not at every git event', async () => {
    const be = backend({ fs_tree: () => Promise.reject('boom') });
    await app.openEditor({ source: 'project', path: 'README.md' });
    render(EditorView, { project: project() });
    await expect.poll(() => app.toasts.map((t) => t.text)).toEqual(['boom']);
    const listed = be.called('git_files').length;
    app.gitTick++;
    await expect.poll(() => be.called('git_files').length).toBe(listed + 1);
    await new Promise((r) => setTimeout(r, 50));
    expect(app.toasts).toHaveLength(1);
  });

  it('brings the line asked for into view once, not each time its tab is shown again', async () => {
    backend({ fs_read: (a) => text(a.path === 'README.md' ? '# demo\n' : 'a\nb\nc\nd\n') });
    await app.openEditor({ source: 'project', path: 'README.md' });
    await app.openEditor({ source: 'project', path: 'src/app.ts', line: 3 });
    const { container } = render(EditorView, { project: project() });
    const shown = () => container.querySelector('.cm-content')?.textContent;
    expect(await screen.findByText('Ln 3, Col 1')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('tab', { name: /README\.md/ }));
    await expect.poll(shown).toBe('# demo');
    await userEvent.click(screen.getByRole('tab', { name: /app\.ts/ }));
    await expect.poll(shown).toBe('abcd');
    await new Promise((r) => setTimeout(r, 30));
    expect(screen.getByText('Ln 1, Col 1')).toBeInTheDocument();
    expect(app.editor.p1.reveal).toBeNull();
  });

  it('does not show the previous file’s comparison on the one just opened', async () => {
    backend({ fs_base: (a) => ({ reference: 'HEAD', text: a.path === 'README.md' ? '# demo\n' : 'const a = 1;\n' }) });
    await app.openEditor({ source: 'project', path: 'README.md' });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByText('1 ligne modifiée vs HEAD');
    await userEvent.click(screen.getByRole('tab', { name: /README\.md/ }));
    await screen.findByText('Identique à HEAD');
    // Both files are loaded now: back on app.ts, README.md’s comparison must not stay until app.ts’s is computed.
    await userEvent.click(screen.getByRole('tab', { name: /app\.ts/ }));
    expect(screen.queryByText('Identique à HEAD')).not.toBeInTheDocument();
    expect(await screen.findByText('1 ligne modifiée vs HEAD')).toBeInTheDocument();
  });
});

describe('EditorView changes in the text, and comparison with the disk', () => {
  beforeEach(() => {
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo({ modified: 1 });
  });

  const REFERENCE = 'a\nb\nc\nd\ne\nf\n';
  const SAVED = 'a\nB\nc\nd\ne\nf\n';
  /** What the user typed on line 2, and what the agent wrote on line 6 meanwhile. */
  const MINE = 'a\nmine\nc\nd\ne\nf\n';
  const AGENT = 'a\nB\nc\nd\ne\nagent\n';

  /** The lines of the other version drawn above the blocks that differ from it. */
  const removed = (c: HTMLElement) => [...c.querySelectorAll('.cm-deletedChunk .cm-deletedLine')].map((l) => l.textContent);
  const codeOf = (c: HTMLElement) => CodeMirror.findFromDOM(c.querySelector('.cm-editor') as HTMLElement)!;
  /** Types MINE over SAVED in the editor: `mine` for `B` on line 2. */
  const typeMine = (c: HTMLElement) => codeOf(c).dispatch({ changes: { from: 2, to: 3, insert: 'mine' }, userEvent: 'input.type' });

  /** src/app.ts open, one line changed since HEAD; the disk as `disk` says, written to as the backend does. */
  async function opened(base: { reference: string; text: string | null } | null = { reference: 'HEAD', text: REFERENCE }) {
    const disk = { text: SAVED, hash: 'h1' };
    let writes = 0;
    const be = backend({
      fs_read: (a) => text(a.path === 'README.md' ? '# demo\n' : disk.text, a.path === 'README.md' ? 'r1' : disk.hash),
      fs_base: () => base,
      fs_write: (a) => {
        if (a.expectedHash !== null && a.expectedHash !== disk.hash) return Promise.reject('changed');
        Object.assign(disk, { text: a.text, hash: `w${++writes}` });
        return disk.hash;
      },
    });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    const key = buffers.key('p1', 'project', 'src/app.ts');
    await expect.poll(() => buffers.all[key]?.base).toEqual(base);
    return { be, disk, key, container };
  }

  /** The agent writes AGENT while the user typed MINE: the banner is up; then « Comparer ». */
  async function comparing() {
    const o = await opened();
    await expect.poll(() => o.container.querySelector('.cm-content')?.textContent).toBe('aBcdef');
    typeMine(o.container);
    expect(buffers.all[o.key].text).toBe(MINE);
    Object.assign(o.disk, { text: AGENT, hash: 'h2' });
    await buffers.refresh(o.key);
    await userEvent.click(within(await screen.findByRole('alert')).getByRole('button', { name: 'Comparer' }));
    await expect.poll(() => screen.getByRole('alert').textContent).toContain('Comparaison avec la version du disque.');
    return o;
  }

  it('shows the changes against the reference in the text, follows what is typed, and hides them', async () => {
    const { key, container } = await opened();
    await screen.findByText('1 ligne modifiée vs HEAD');
    const toggle = screen.getByRole('button', { name: 'Voir les changements' });
    expect(toggle).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelector('.cm-deletedChunk')).toBeNull();
    await userEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-pressed', 'true');
    // A toggle keeps its name: pressed says it is on.
    expect(toggle).toHaveTextContent(/^Voir les changements$/);
    expect(removed(container)).toEqual(['b']);
    // Still a text to type in.
    codeOf(container).dispatch({ changes: { from: 0, to: 1, insert: 'A' }, userEvent: 'input.type' });
    expect(buffers.all[key].text).toBe('A\nB\nc\nd\ne\nf\n');
    expect(removed(container)).toEqual(['a', 'b']);
    await userEvent.click(toggle);
    expect(container.querySelector('.cm-deletedChunk')).toBeNull();
    expect(screen.getByRole('button', { name: 'Voir les changements' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('puts a block of the reference back with « Annuler ce bloc », unsaved, and Ctrl+Z takes it away again', async () => {
    const { be, key } = await opened();
    await userEvent.click(await screen.findByRole('button', { name: 'Voir les changements' }));
    await userEvent.click(screen.getByRole('button', { name: 'Annuler ce bloc' }));
    expect(buffers.all[key].text).toBe(REFERENCE);
    expect(await screen.findByText(/● Non enregistré/)).toBeInTheDocument();
    expect(be.called('fs_write')).toHaveLength(0);
    // The focus is in the text: Ctrl+Z goes to it.
    await userEvent.keyboard('{Control>}z{/Control}');
    expect(buffers.all[key].text).toBe(SAVED);
    expect(await screen.findByText('Enregistré')).toBeInTheDocument();
  });

  it('has no « Voir les changements » for a file the reference does not have', async () => {
    await opened({ reference: 'HEAD', text: null });
    expect(await screen.findByText('Nouveau fichier · absent de HEAD')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Voir les changements' })).not.toBeInTheDocument();
  });

  it('has no « Voir les changements » outside a repository', async () => {
    await opened(null);
    expect(screen.getByText('Ln 1, Col 1')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Voir les changements' })).not.toBeInTheDocument();
  });

  it('follows a text reloaded from disk while the changes are shown', async () => {
    const { disk, key, container } = await opened();
    await userEvent.click(await screen.findByRole('button', { name: 'Voir les changements' }));
    expect(removed(container)).toEqual(['b']);
    Object.assign(disk, { text: 'a\nb\nc\nd\ne\nF\n', hash: 'h2' });
    app.gitTick++;
    await expect.poll(() => buffers.all[key].text).toBe('a\nb\nc\nd\ne\nF\n');
    await expect.poll(() => removed(container)).toEqual(['f']);
    expect(screen.getByRole('button', { name: 'Voir les changements' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('keeps what was typed when the agent writes while the changes are shown, and shows them again once the choice is made', async () => {
    const { disk, key, container } = await opened();
    await userEvent.click(await screen.findByRole('button', { name: 'Voir les changements' }));
    typeMine(container);
    Object.assign(disk, { text: AGENT, hash: 'h2' });
    app.gitTick++;
    expect(await screen.findByRole('alert')).toHaveTextContent('Ce fichier a changé sur le disque.');
    expect(buffers.all[key].text).toBe(MINE);
    expect(removed(container)).toEqual(['b']);
    await userEvent.click(screen.getByRole('button', { name: 'Comparer' }));
    await expect.poll(() => removed(container)).toEqual(['B', 'agent']);
    await userEvent.click(screen.getByRole('button', { name: 'Recharger' }));
    await expect.poll(() => screen.queryByRole('alert')).toBeNull();
    // Against the reference again, as the button still says.
    expect(removed(container)).toEqual(['b', 'f']);
    expect(screen.getByRole('button', { name: 'Voir les changements' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('compares with the version on disk from its banner, takes a block of it and keeps the text merged', async () => {
    const { be, disk, key, container } = await opened();
    await expect.poll(() => container.querySelector('.cm-content')?.textContent).toBe('aBcdef');
    typeMine(container);
    Object.assign(disk, { text: AGENT, hash: 'h2' });
    await buffers.refresh(key);
    const banner = await screen.findByRole('alert');
    expect(banner).toHaveTextContent('Ce fichier a changé sur le disque.');
    expect(
      within(banner)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Recharger', 'Comparer', 'Garder ma version']);
    await userEvent.click(within(banner).getByRole('button', { name: 'Comparer' }));
    await expect.poll(() => screen.getByRole('alert').textContent).toContain('Comparaison avec la version du disque.');
    expect(
      within(screen.getByRole('alert'))
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Recharger', 'Garder ma version']);
    // What the disk has, block by block; the comparison with the reference gives way to it.
    expect(removed(container)).toEqual(['B', 'agent']);
    expect(screen.queryByRole('button', { name: 'Voir les changements' })).not.toBeInTheDocument();
    expect(buffers.all[key].text).toBe(MINE);
    const take = screen.getAllByRole('button', { name: 'Prendre ce bloc' });
    expect(take).toHaveLength(2);
    await userEvent.click(take[1]);
    expect(buffers.all[key].text).toBe('a\nmine\nc\nd\ne\nagent\n');
    // The banner stays until the user chooses.
    expect(screen.getByRole('alert')).toHaveTextContent('Comparaison avec la version du disque.');
    expect(be.called('fs_write')).toHaveLength(0);
    await userEvent.click(screen.getByRole('button', { name: 'Garder ma version' }));
    await expect.poll(() => be.called('fs_write').length).toBe(1);
    expect(be.called('fs_write')[0].args).toMatchObject({ text: 'a\nmine\nc\nd\ne\nagent\n', expectedHash: 'h2' });
    await expect.poll(() => screen.queryByRole('alert')).toBeNull();
    expect(container.querySelector('.cm-deletedChunk')).toBeNull();
    expect(await screen.findByText('Enregistré')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Voir les changements' })).toBeInTheDocument();
  });

  const NEWER = 'Le fichier a encore changé sur le disque : la comparaison montre sa nouvelle version.';

  it('keeps what was typed while the agent writes again during the comparison, says so, and holds « Garder ma version » back a moment', async () => {
    const { disk, key, container } = await comparing();
    Object.assign(disk, { text: 'a\nB\nc\nd\ne\nagain\n', hash: 'h3' });
    app.gitTick++;
    await expect.poll(() => removed(container)).toEqual(['B', 'again']);
    expect(buffers.all[key].text).toBe(MINE);
    expect(screen.getByRole('alert')).toHaveTextContent(NEWER);
    expect(app.toasts.map((t) => t.text)).toEqual([NEWER]);
    // A click aimed at the version shown before is not taken.
    const keep = screen.getByRole('button', { name: 'Garder ma version' });
    expect(keep).toBeDisabled();
    await expect.poll(() => keep, { timeout: 2000 }).toBeEnabled();
    expect(screen.getByRole('alert')).toHaveTextContent(NEWER);
    // Typing in the comparison: the newer version has been seen.
    codeOf(container).dispatch({ changes: { from: 0, insert: 'x' }, userEvent: 'input.type' });
    await expect.poll(() => screen.getByRole('alert').textContent).toContain('Comparaison avec la version du disque.');
    expect(app.toasts).toHaveLength(1);
  });

  it('does not write over a version of the disk the comparison has not shown, and shows it instead', async () => {
    const { be, disk, key, container } = await comparing();
    // Written once the comparison was read, before anything read the file again.
    Object.assign(disk, { text: 'a\nB\nc\nd\ne\nagain\n', hash: 'h3' });
    await userEvent.click(screen.getByRole('button', { name: 'Garder ma version' }));
    await expect.poll(() => removed(container)).toEqual(['B', 'again']);
    expect(be.called('fs_write')).toHaveLength(1);
    expect(disk).toEqual({ text: 'a\nB\nc\nd\ne\nagain\n', hash: 'h3' });
    expect(buffers.all[key].text).toBe(MINE);
    expect(screen.getByRole('alert')).toHaveTextContent(NEWER);
    expect(screen.getByRole('button', { name: 'Garder ma version' })).toBeDisabled();
    expect(app.toasts.map((t) => t.text)).toEqual([
      'Le fichier a encore changé sur le disque : rien n’est enregistré, la comparaison montre sa nouvelle version.',
    ]);
  });

  it('says that nothing was saved when « Garder ma version » finds the disk back at the version opened', async () => {
    const { be, disk, key } = await comparing();
    Object.assign(disk, { text: SAVED, hash: 'h1' });
    await userEvent.click(screen.getByRole('button', { name: 'Garder ma version' }));
    await expect.poll(() => screen.queryByRole('alert')).toBeNull();
    expect(app.toasts.map((t) => t.text)).toEqual(['Rien n’a été enregistré : le fichier est revenu à la version que tu avais ouverte.']);
    expect(be.called('fs_write')).toHaveLength(1);
    expect(disk).toEqual({ text: SAVED, hash: 'h1' });
    expect(buffers.all[key].text).toBe(MINE);
    expect(screen.getByText(/● Non enregistré/)).toBeInTheDocument();
  });

  it('reloads from the comparison: the version on disk replaces what was typed, and the comparison is over', async () => {
    const { key, container } = await comparing();
    await userEvent.click(screen.getByRole('button', { name: 'Recharger' }));
    expect(buffers.all[key].text).toBe(AGENT);
    await expect.poll(() => screen.queryByRole('alert')).toBeNull();
    expect(container.querySelector('.cm-deletedChunk')).toBeNull();
  });
});

describe('EditorView tree, as VS Code’s explorer', () => {
  beforeEach(() => {
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo({ modified: 1 });
  });

  /** A backend whose tree lists the files created through it; `over` replaces commands, given the files on disk. */
  function creating(over: Record<string, (a: any) => unknown> | ((files: string[]) => Record<string, (a: any) => unknown>) = {}) {
    const files = ['README.md', 'src/app.ts'];
    return backend({
      fs_tree: () => ({ root: 'C:/code/demo-api', files: [...files], truncated: false }),
      fs_create: (a) => void files.push(a.path),
      fs_read: () => text(''),
      ...(typeof over === 'function' ? over(files) : over),
    });
  }
  const field = () => screen.findByRole('textbox', { name: 'Nom du nouveau fichier' });
  const item = (name: RegExp) => screen.getByRole('treeitem', { name });

  it('creates a file next to the one shown, folders included, and opens it', async () => {
    const be = creating();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), 'lib/util.ts{Enter}');
    expect(be.called('fs_create').map((c) => c.args)).toEqual([{ projectId: 'p1', agentId: null, path: 'src/lib/util.ts' }]);
    expect(await screen.findByRole('tab', { name: /util\.ts/ })).toHaveAttribute('aria-selected', 'true');
    expect(await screen.findByRole('treeitem', { name: /util\.ts/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.queryByRole('textbox', { name: 'Nom du nouveau fichier' })).not.toBeInTheDocument();
    expect(app.toasts).toEqual([]);
  });

  it('creates it in the folder of the row last clicked', async () => {
    const be = creating();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(item(/README/));
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), 'a.ts{Enter}');
    await screen.findByRole('tab', { name: /a\.ts/ });
    await userEvent.click(item(/src/));
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    expect(item(/src/)).toHaveAttribute('aria-expanded', 'true');
    await userEvent.type(await field(), 'b.ts{Enter}');
    expect(be.called('fs_create').map((c) => c.args.path)).toEqual(['a.ts', 'src/b.ts']);
  });

  it('creates a file in a folder from its menu, in its folder from a file’s, at the root from the free space', async () => {
    creating();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    const pick = async (target: Element, label: string) => {
      await fireEvent.contextMenu(target);
      menu.open!.items.find((i) => i.label === label)!.onClick!();
      menu.close();
    };
    await userEvent.click(screen.getByRole('button', { name: 'Tout réduire' }));
    expect(app.editor.p1.places.project.expanded).toEqual({});
    await pick(item(/src/), 'Nouveau fichier…');
    expect(item(/src/)).toHaveAttribute('aria-expanded', 'true');
    expect(item(/src/).nextElementSibling).toContainElement(await field());
    await userEvent.keyboard('{Escape}');
    await pick(item(/app\.ts/), 'Nouveau fichier…');
    expect(item(/src/).nextElementSibling).toContainElement(await field());
    await userEvent.keyboard('{Escape}');
    await pick(screen.getByRole('tree'), 'Nouveau fichier…');
    expect(screen.getByRole('tree').firstElementChild).toContainElement(await field());
  });

  describe('terminal here', () => {
    beforeEach(() => {
      vi.mocked(openTerminal).mockClear();
      app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
    });

    const pick = async (target: Element) => {
      await fireEvent.contextMenu(target);
      menu.open!.items.find((i) => i.label === 'Ouvrir un terminal ici')!.onClick!();
      menu.close();
    };

    it('opens one in a folder from its menu, in the folder of a file from its menu, at the root from the free space', async () => {
      creating();
      await app.openEditor({ source: 'project', path: 'src/app.ts' });
      render(EditorView, { project: project() });
      await screen.findByRole('treeitem', { name: /app\.ts/ });
      await pick(item(/src/));
      expect(openTerminal).toHaveBeenLastCalledWith('p1', 'pwsh', 'pwsh · src', { agentId: null, subdir: 'src' });
      await pick(item(/app\.ts/));
      expect(openTerminal).toHaveBeenLastCalledWith('p1', 'pwsh', 'pwsh · src', { agentId: null, subdir: 'src' });
      // A file at the root, and the free space: the project itself.
      await pick(item(/README/));
      expect(openTerminal).toHaveBeenLastCalledWith('p1', 'pwsh', 'pwsh · demo-api', { agentId: null });
      await pick(screen.getByRole('tree'));
      expect(openTerminal).toHaveBeenLastCalledWith('p1', 'pwsh', 'pwsh · demo-api', { agentId: null });
      expect(openTerminal).toHaveBeenCalledTimes(4);
    });

    it('names the folder by its own name, from deep in the tree', async () => {
      backend({ fs_tree: () => ({ root: 'C:/code/demo-api', files: ['src/lib/ui/button.ts'], truncated: false }) });
      await app.openEditor({ source: 'project', path: 'src/lib/ui/button.ts' });
      render(EditorView, { project: project() });
      await screen.findByRole('treeitem', { name: /button\.ts/ });
      await pick(item(/ui/));
      expect(openTerminal).toHaveBeenLastCalledWith('p1', 'pwsh', 'pwsh · ui', { agentId: null, subdir: 'src/lib/ui' });
    });

    it('opens it in the source shown: the worktree of the agent being browsed', async () => {
      const wt = { path: 'C:/code/demo-api/.claude/worktrees/wt', branch: 'escouade/wt', baseBranch: 'main' };
      resetApp({ agents: [agent(), agent({ id: 'a2', name: 'wt-agent', worktree: wt })] });
      app.shells = [{ id: 'pwsh', label: 'PowerShell 7', path: 'pwsh.exe' }];
      creating();
      await app.openEditor({ source: 'a2', path: 'src/app.ts' });
      render(EditorView, { project: project() });
      await screen.findByRole('treeitem', { name: /app\.ts/ });
      await pick(item(/src/));
      expect(openTerminal).toHaveBeenLastCalledWith('p1', 'pwsh', 'pwsh · src', { agentId: 'a2', subdir: 'src' });
      // At the root of its worktree, it is named after the agent.
      await pick(screen.getByRole('tree'));
      expect(openTerminal).toHaveBeenLastCalledWith('p1', 'pwsh', 'pwsh · wt-agent', { agentId: 'a2' });
    });

    it('keeps the entry before the copies, apart from them', async () => {
      creating();
      await app.openEditor({ source: 'project', path: 'src/app.ts' });
      render(EditorView, { project: project() });
      const row = await screen.findByRole('treeitem', { name: /app\.ts/ });
      await fireEvent.contextMenu(row);
      const items = menu.open!.items;
      expect(items.map((i) => i.label)).toEqual([
        'Nouveau fichier…',
        'Ouvrir un terminal ici',
        '',
        'Copier le chemin',
        'Copier le chemin relatif',
      ]);
      expect(items[2].separator).toBe(true);
      menu.close();
    });
  });

  it('tells why a name is refused, by the tree or by the disk', async () => {
    creating({ fs_create: () => Promise.reject('src/b.ts existe déjà') });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), 'APP.ts');
    expect(screen.getByRole('alert')).toHaveTextContent('« APP.ts » existe déjà à cet endroit.');
    await userEvent.clear(await field());
    await userEvent.type(await field(), 'b.ts{Enter}');
    expect(await screen.findByRole('alert')).toHaveTextContent('src/b.ts existe déjà');
    expect(app.editor.p1.places.project.open).toEqual(['src/app.ts']);
  });

  it('says so when the file created is one git ignores, which the tree does not show', async () => {
    creating({ fs_create: () => null });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(item(/README/));
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), '.env{Enter}');
    expect(await screen.findByRole('tab', { name: /\.env/ })).toBeInTheDocument();
    expect(app.toasts.map((t) => t.text)).toEqual(['.env est ignoré par git : l’arborescence ne le montre pas.']);
  });

  it('shows the ignored files the project copies into its worktrees greyed, opens them, and knows them when naming a file', async () => {
    const be = creating({
      fs_tree: () => ({ root: 'C:/code/demo-api', files: ['.env', 'README.md', 'src/app.ts'], truncated: false, ignored: ['.env'] }),
      fs_read: (a) => text(a.path === '.env' ? 'SECRET=1\n' : '# demo\n'),
    });
    await app.openEditor({ source: 'project', path: 'README.md' });
    render(EditorView, { project: project() });
    const env = await screen.findByRole('treeitem', { name: /\.env/ });
    expect(env).toHaveAttribute('title', 'Ignoré par git');
    expect(item(/README/)).toHaveAttribute('title', 'README.md');
    await userEvent.click(env);
    expect(await screen.findByRole('tab', { name: /\.env/ })).toHaveAttribute('aria-selected', 'true');
    expect(be.called('fs_read').map((c) => c.args.path)).toContain('.env');
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), '.env');
    expect(screen.getByRole('alert')).toHaveTextContent('« .env » existe déjà à cet endroit (ignoré par git).');
  });

  it('copies the path of a file, from the root of its source or whole', async () => {
    creating();
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    const row = await screen.findByRole('treeitem', { name: /app\.ts/ });
    for (const label of ['Copier le chemin relatif', 'Copier le chemin']) {
      await fireEvent.contextMenu(row);
      menu.open!.items.find((i) => i.label === label)!.onClick!();
      menu.close();
    }
    expect(writeText.mock.calls).toEqual([['src/app.ts'], ['C:\\code\\demo-api\\src\\app.ts']]);
  });

  it('opens a file created in a folder typed in another case under the name the disk gives it', async () => {
    // As Windows does: `SRC/` is the `src/` already there.
    creating((files) => ({ fs_create: (a) => void files.push(a.path.replace(/^SRC\//, 'src/')) }));
    await app.openEditor({ source: 'project', path: 'README.md' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /README/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), 'SRC/new.ts{Enter}');
    await expect.poll(() => app.editor.p1.places.project.active).toBe('src/new.ts');
    expect(app.editor.p1.places.project.open).toEqual(['README.md', 'src/new.ts']);
    expect(app.toasts).toEqual([]);
  });

  it('puts the cursor in the file created', async () => {
    creating();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), 'b.ts{Enter}');
    await expect.poll(() => document.activeElement?.closest('.cm-editor')).not.toBeNull();
  });

  it('starts no other file while one is being created, and tells a refusal that comes once its field is gone', async () => {
    let refuse!: (e: string) => void;
    const be = creating({ fs_create: () => new Promise((_, reject) => (refuse = reject)) });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), 'b.ts{Enter}');
    await userEvent.click(item(/README/));
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    expect(screen.getAllByRole('textbox', { name: 'Nom du nouveau fichier' })).toHaveLength(1);
    expect(item(/src/).nextElementSibling).toContainElement(await field());
    // The source changes: the field goes, the refusal is told otherwise.
    app.editor.p1.source = 'other';
    await expect.poll(() => screen.queryByRole('textbox', { name: 'Nom du nouveau fichier' })).toBeNull();
    refuse('Accès refusé');
    await expect.poll(() => app.toasts.map((t) => t.text)).toEqual(['Création impossible : Accès refusé']);
    expect(be.called('fs_create')).toHaveLength(1);
  });

  it('names the file in the nearest folder the tree shows when the one asked for is not there', async () => {
    creating();
    // A file git ignores, opened from the conversation: its folder is not in the tree.
    await app.openEditor({ source: 'project', path: 'build/out/x.js' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /README/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    expect(screen.getByRole('tree').firstElementChild).toContainElement(await field());
  });

  it('does not take a file for one git ignores when the tree could not be read again', async () => {
    let reads = 0;
    creating({
      fs_tree: () =>
        reads++ ? Promise.reject('git occupé') : { root: 'C:/code/demo-api', files: ['README.md', 'src/app.ts'], truncated: false },
    });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), 'b.ts{Enter}');
    expect(await screen.findByRole('tab', { name: /b\.ts/ })).toBeInTheDocument();
    expect(app.toasts).toEqual([]);
  });

  it('does not take a file for one git ignores when the tree could not list it', async () => {
    creating({
      fs_tree: () => ({ root: 'C:/code/demo-api', files: ['README.md', 'src/app.ts'], truncated: true }),
      fs_create: () => null,
    });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    await userEvent.type(await field(), 'b.ts{Enter}');
    expect(await screen.findByRole('tab', { name: /b\.ts/ })).toBeInTheDocument();
    expect(app.toasts).toEqual([]);
  });

  it('reads the tree again on demand', async () => {
    const be = creating();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    const before = be.called('fs_tree').length;
    await userEvent.click(screen.getByRole('button', { name: 'Actualiser' }));
    await expect.poll(() => be.called('fs_tree').length).toBe(before + 1);
  });

  it('gives up the file being named when the source changes', async () => {
    const wt = { path: 'C:/code/demo-api/.claude/worktrees/wt', branch: 'escouade/wt', baseBranch: 'main' };
    resetApp({ agents: [agent(), agent({ id: 'a2', name: 'wt', worktree: wt })] });
    app.git.p1 = gitInfo({ modified: 1 });
    creating();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    await userEvent.click(screen.getByRole('button', { name: 'Nouveau fichier' }));
    expect(await field()).toBeInTheDocument();
    await app.openEditor({ source: 'a2' });
    await expect.poll(() => screen.queryByRole('textbox', { name: 'Nom du nouveau fichier' })).toBeNull();
    await app.openEditor({ source: 'project' });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    expect(screen.queryByRole('textbox', { name: 'Nom du nouveau fichier' })).not.toBeInTheDocument();
  });
});

describe('EditorView navigation', () => {
  beforeEach(() => {
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo({ modified: 1 });
  });

  const APP = "import { u } from './util';\nimport { m } from './missing';\nconsole.log(u); // see src/util.ts:2:5\n";
  const UTIL = 'export const u = 1;\nexport const v = 2;\n';
  const files = ['src/app.ts', 'src/util.ts', 'tsconfig.json'];

  function navBackend(over: Record<string, (a: any) => unknown> = {}) {
    return backend({
      fs_tree: () => ({ root: 'C:/code/demo-api', files, truncated: false }),
      git_files: () => [],
      fs_read: (a) => text(a.path === 'src/util.ts' ? UTIL : a.path === 'tsconfig.json' ? '{}' : APP),
      ...over,
    });
  }

  /** The CodeMirror view, once it shows the text starting with `start`. */
  async function shown(container: HTMLElement, start: string): Promise<CodeMirror> {
    await expect.poll(() => container.querySelector('.cm-content')?.textContent?.startsWith(start)).toBe(true);
    return CodeMirror.findFromDOM(container.querySelector('.cm-editor') as HTMLElement)!;
  }

  /** A Ctrl+click at `pos` (jsdom has no layout to find it from the mouse). */
  function ctrlClick(view: CodeMirror, pos: number) {
    vi.spyOn(view, 'posAtCoords').mockReturnValue(pos);
    const o = { bubbles: true, cancelable: true, button: 0, detail: 1, ctrlKey: true };
    view.contentDOM.dispatchEvent(new MouseEvent('mousedown', o));
    view.contentDOM.dispatchEvent(new MouseEvent('mouseup', o));
  }
  const press = (view: CodeMirror, key: string, init: KeyboardEventInit = {}) =>
    view.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
  const active = () => app.editor.p1.places.project.active;

  it('follows an import with Ctrl+click, comes back to it with Alt+← and goes forward again with Alt+→', async () => {
    navBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    const at = APP.indexOf('util');
    ctrlClick(await shown(container, 'import'), at);
    await expect.poll(active).toBe('src/util.ts');
    press(await shown(container, 'export'), 'ArrowLeft', { altKey: true });
    await expect.poll(active).toBe('src/app.ts');
    expect(await screen.findByText(`Ln 1, Col ${at + 1}`)).toBeInTheDocument();
    press(await shown(container, 'import'), 'ArrowRight', { altKey: true });
    await expect.poll(active).toBe('src/util.ts');
  });

  it('does not open what a link leads to once the editor is closed', async () => {
    navBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container, unmount } = render(EditorView, { project: project() });
    ctrlClick(await shown(container, 'import'), APP.indexOf('util'));
    // Closed before the answer: the editor leaves the screen with its view.
    app.closeEditor();
    unmount();
    await new Promise((r) => setTimeout(r, 30));
    expect(app.editor.p1).toMatchObject({ on: false, places: { project: { active: 'src/app.ts' } } });
  });

  it('says when the file a link leads to is not in the tree, and stays', async () => {
    navBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    ctrlClick(await shown(container, 'import'), APP.indexOf('missing'));
    await expect.poll(() => app.toasts.map((t) => t.text)).toEqual(['Fichier introuvable : src/missing']);
    expect(active()).toBe('src/app.ts');
  });

  it('follows a path at the cursor with F12, to its line and column', async () => {
    navBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    const view = await shown(container, 'import');
    view.dispatch({ selection: { anchor: APP.indexOf('src/util.ts') + 2 } });
    press(view, 'F12');
    await expect.poll(active).toBe('src/util.ts');
    expect(await screen.findByText('Ln 2, Col 5')).toBeInTheDocument();
  });

  it('follows the aliases of the source’s tsconfig.json', async () => {
    const tsconfig = '{\n  // aliases\n  "compilerOptions": { "paths": { "@/*": ["src/*"] } }\n}';
    const be = navBackend({
      fs_read: (a) => text(a.path === 'src/util.ts' ? UTIL : a.path === 'tsconfig.json' ? tsconfig : "import { u } from '@/util';\n"),
    });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    const view = await shown(container, 'import');
    await expect.poll(() => be.called('fs_read').map((c) => c.args.path)).toContain('tsconfig.json');
    // Read in the background: followed once it is there (and clicked only while app.ts is shown).
    await expect
      .poll(() => {
        if (active() === 'src/app.ts') ctrlClick(view, "import { u } from '@/u".length);
        return active();
      })
      .toBe('src/util.ts');
  });

  it('goes to the definition of an identifier of the file with Ctrl+click, and back with Alt+←', async () => {
    const MAIN = 'function helper() {}\n\nconst x = helper();\n';
    navBackend({ fs_read: (a) => text(a.path === 'tsconfig.json' ? '{}' : MAIN) });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    const view = await shown(container, 'function');
    ctrlClick(view, MAIN.lastIndexOf('helper') + 1);
    expect(await screen.findByText('Ln 1, Col 10')).toBeInTheDocument();
    press(view, 'ArrowLeft', { altKey: true });
    expect(await screen.findByText('Ln 3, Col 12')).toBeInTheDocument();
  });

  it('lists the definitions a search finds in the tree, its folder first, and opens the one picked where it is', async () => {
    const texts: Record<string, string> = {
      'src/app.ts': 'render(1);\n',
      'lib/render.ts': '// draws\n\n\nexport function render(n: number) {}\n',
      'tsconfig.json': '{}',
    };
    const be = navBackend({
      fs_tree: () => ({
        root: 'C:/code/demo-api',
        files: ['lib/render.ts', 'src/app.ts', 'src/draw.ts', 'tsconfig.json'],
        truncated: false,
      }),
      fs_read: (a) => text(texts[a.path] ?? ''),
      code_search: () => ({
        matches: [
          { path: 'lib/render.ts', line: 4, col: 8, text: 'export function render(n: number) {}', offset: 0, ranges: [[7, 23]] },
          { path: 'src/draw.ts', line: 2, col: 8, text: 'export const render = (n: number) => n;', offset: 0, ranges: [[7, 20]] },
          // Gone since: not offered.
          { path: 'src/gone.ts', line: 1, col: 1, text: 'function render() {}', offset: 0, ranges: [[0, 16]] },
        ],
        truncated: false,
        timedOut: false,
      }),
    });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    ctrlClick(await shown(container, 'render'), 2);
    const list = await screen.findByRole('listbox', { name: 'Définitions de « render »' });
    expect(
      within(list)
        .getAllByRole('option')
        .map((o) => o.textContent?.replace(/\s+/g, ' ').trim()),
    ).toEqual(['src/draw.ts:2 export const render = (n: number) => n;', 'lib/render.ts:4 export function render(n: number) {}']);
    expect(be.called('code_search')[0].args).toMatchObject({ projectId: 'p1', agentId: null, query: { regex: true, caseSensitive: true } });
    await userEvent.keyboard('{ArrowDown}{Enter}');
    await expect.poll(active).toBe('lib/render.ts');
    expect(await screen.findByText('Ln 4, Col 17')).toBeInTheDocument();
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    // Through the history, as any jump.
    press(await shown(container, '// draws'), 'ArrowLeft', { altKey: true });
    await expect.poll(active).toBe('src/app.ts');
    expect(await screen.findByText('Ln 1, Col 3')).toBeInTheDocument();
  });

  it('says when no definition of an identifier is found, and stays', async () => {
    navBackend({
      fs_read: (a) => text(a.path === 'tsconfig.json' ? '{}' : 'nowhere();\n'),
      code_search: () => ({ matches: [], truncated: false, timedOut: false }),
    });
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    ctrlClick(await shown(container, 'nowhere'), 2);
    await expect.poll(() => app.toasts.map((t) => t.text)).toEqual(['Aucune définition trouvée pour « nowhere ».']);
    expect(active()).toBe('src/app.ts');
  });

  it('skips, going back, a place whose file is gone from the tree', async () => {
    navBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    ctrlClick(await shown(container, 'import'), APP.indexOf('util'));
    await expect.poll(active).toBe('src/util.ts');
    trees.all['p1|project'] = { root: 'C:/code/demo-api', files: ['src/util.ts'], truncated: false, ignored: [] };
    press(await shown(container, 'export'), 'ArrowLeft', { altKey: true });
    await new Promise((r) => setTimeout(r, 30));
    expect(active()).toBe('src/util.ts');
  });
});

describe('EditorView search through the files', () => {
  beforeEach(() => {
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo({ modified: 1 });
  });

  const APP = 'const total = sum(1, 2);\nconsole.log(total);\n';
  const UTIL = 'export const v = 2;\nexport function sum(a, b) {}\n';
  const FOUND = {
    matches: [{ path: 'src/util.ts', line: 2, col: 17, text: 'export function sum(a, b) {}', offset: 0, ranges: [[16, 19]] }],
    truncated: false,
    timedOut: false,
  };

  function searchBackend(over: Record<string, (a: any) => unknown> = {}) {
    return backend({
      fs_tree: () => ({ root: 'C:/code/demo-api', files: ['src/app.ts', 'src/util.ts'], truncated: false }),
      git_files: () => [],
      fs_read: (a) => text(a.path === 'src/util.ts' ? UTIL : APP),
      code_search: () => FOUND,
      ...over,
    });
  }

  async function shown(container: HTMLElement, start: string): Promise<CodeMirror> {
    await expect.poll(() => container.querySelector('.cm-content')?.textContent?.startsWith(start)).toBe(true);
    return CodeMirror.findFromDOM(container.querySelector('.cm-editor') as HTMLElement)!;
  }
  const field = () => screen.getByRole('textbox', { name: 'Rechercher' });
  const queries = (be: ReturnType<typeof backend>) => be.called('code_search').map((c) => c.args);
  const query = (pattern: string, o: Record<string, boolean> = {}) => ({
    pattern,
    regex: false,
    caseSensitive: false,
    wholeWord: false,
    maxResults: 2000,
    ...o,
  });
  const ctrlShiftF = () => handleShortcut(new KeyboardEvent('keydown', { key: 'F', ctrlKey: true, shiftKey: true }), false);

  it('shows the files or the search in the left column, from its two buttons', async () => {
    searchBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    const files = screen.getByRole('button', { name: 'Fichiers' });
    const find = screen.getByRole('button', { name: 'Rechercher dans les fichiers' });
    expect([files, find].map((b) => b.getAttribute('aria-pressed'))).toEqual(['true', 'false']);
    expect(screen.queryByRole('textbox', { name: 'Rechercher' })).not.toBeInTheDocument();
    await userEvent.click(find);
    expect([files, find].map((b) => b.getAttribute('aria-pressed'))).toEqual(['false', 'true']);
    await vi.waitFor(() => expect(field()).toHaveFocus());
    expect(screen.queryByRole('tree', { name: 'Fichiers' })).not.toBeInTheDocument();
    await userEvent.click(files);
    expect(screen.getByRole('tree', { name: 'Fichiers' })).toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: 'Rechercher' })).not.toBeInTheDocument();
  });

  it('keeps both views in one column of its own, not taken for the sidebar’s (aside.side)', async () => {
    searchBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    await screen.findByRole('treeitem', { name: /app\.ts/ });
    expect(container.querySelector('aside.side')).toBeNull();
    const column = container.querySelector('aside.editor-side') as HTMLElement;
    expect(within(column).getByRole('tree', { name: 'Fichiers' })).toBeInTheDocument();
    expect(within(column).getByRole('textbox', { name: 'Rechercher', hidden: true })).toBeInTheDocument();
  });

  it('opens the search with Ctrl+Shift+F, the selection of the code in its field when it holds on one line', async () => {
    const be = searchBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    const view = await shown(container, 'const');
    view.focus();
    view.dispatch({ selection: { anchor: APP.indexOf('sum'), head: APP.indexOf('sum') + 3 } });
    expect(ctrlShiftF()).toBe(true);
    await vi.waitFor(() => expect(field()).toHaveFocus());
    expect(field()).toHaveValue('sum');
    await expect.poll(() => queries(be)).toEqual([{ projectId: 'p1', agentId: null, query: query('sum') }]);
    expect(await screen.findByRole('treeitem', { name: 'Ligne 2 : export function sum(a, b) {}' })).toBeInTheDocument();
    // Over two lines, or with the focus elsewhere than on the code, the selection is not taken.
    view.focus();
    view.dispatch({ selection: { anchor: 0, head: APP.indexOf('console') + 3 } });
    ctrlShiftF();
    await vi.waitFor(() => expect(field()).toHaveFocus());
    view.dispatch({ selection: { anchor: 0, head: 5 } });
    ctrlShiftF();
    expect(field()).toHaveValue('sum');
    expect(queries(be)).toHaveLength(1);
  });

  it('finds the uses of the word at the cursor with Shift+F12, whole and in its case, as written', async () => {
    const be = searchBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { container } = render(EditorView, { project: project() });
    const view = await shown(container, 'const');
    // Read as an expression before: the word is then looked for as written all the same.
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher dans les fichiers' }));
    await userEvent.click(screen.getByRole('button', { name: 'Expression régulière' }));
    expect(screen.getByRole('button', { name: 'Expression régulière' })).toHaveAttribute('aria-pressed', 'true');
    view.focus();
    view.dispatch({ selection: { anchor: APP.indexOf('total') + 2 } });
    view.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { key: 'F12', shiftKey: true, bubbles: true, cancelable: true }));
    await vi.waitFor(() => expect(field()).toHaveFocus());
    expect(field()).toHaveValue('total');
    const pressed = (name: string) => screen.getByRole('button', { name }).getAttribute('aria-pressed');
    expect(['Mot entier', 'Respecter la casse', 'Expression régulière'].map(pressed)).toEqual(['true', 'true', 'false']);
    await expect
      .poll(() => queries(be))
      .toEqual([{ projectId: 'p1', agentId: null, query: query('total', { caseSensitive: true, wholeWord: true }) }]);
  });

  it('opens a line found at its line and column, and comes back with Alt+←', async () => {
    searchBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts', line: 2, col: 9 });
    const { container } = render(EditorView, { project: project() });
    await shown(container, 'const');
    expect(await screen.findByText('Ln 2, Col 9')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rechercher dans les fichiers' }));
    await userEvent.type(field(), 'sum');
    await userEvent.click(await screen.findByRole('treeitem', { name: 'Ligne 2 : export function sum(a, b) {}' }));
    await expect.poll(() => app.editor.p1.places.project.active).toBe('src/util.ts');
    expect(await screen.findByText('Ln 2, Col 17')).toBeInTheDocument();
    const util = await shown(container, 'export');
    util.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', altKey: true, bubbles: true, cancelable: true }));
    await expect.poll(() => app.editor.p1.places.project.active).toBe('src/app.ts');
    expect(await screen.findByText('Ln 2, Col 9')).toBeInTheDocument();
  });

  it('searches the source shown, and again in another one', async () => {
    const be = searchBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    render(EditorView, { project: project() });
    await userEvent.click(await screen.findByRole('button', { name: 'Rechercher dans les fichiers' }));
    await userEvent.type(field(), 'sum');
    await screen.findByRole('treeitem', { name: /util\.ts/ });
    await app.openEditor({ projectId: 'p1', source: 'a1' });
    await expect.poll(() => queries(be).at(-1)).toEqual({ projectId: 'p1', agentId: 'a1', query: query('sum') });
    expect(field()).toHaveValue('sum');
  });

  it('keeps the search while the editor is closed, to find it again as it was', async () => {
    const be = searchBackend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { unmount } = render(EditorView, { project: project() });
    await userEvent.click(await screen.findByRole('button', { name: 'Rechercher dans les fichiers' }));
    await userEvent.type(field(), 'sum');
    await screen.findByRole('treeitem', { name: /util\.ts/ });
    unmount();
    render(EditorView, { project: project() });
    expect(field()).toHaveValue('sum');
    expect(screen.getByRole('treeitem', { name: /util\.ts/ })).toBeInTheDocument();
    expect(queries(be)).toHaveLength(1);
  });
});

describe('EditorView and « Ouvrir un fichier »', () => {
  beforeEach(() => {
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo({ modified: 1 });
  });

  it('opens the file picked, the place left kept in the history for Alt+← to come back to', async () => {
    backend();
    await app.openEditor({ source: 'project', path: 'src/app.ts', line: 1, col: 7 });
    const { container } = render(EditorView, { project: project() });
    expect(await screen.findByText('Ln 1, Col 7')).toBeInTheDocument();
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 'p', ctrlKey: true }), false)).toBe(true);
    expect(app.modal).toEqual({ kind: 'quickOpen', projectId: 'p1', source: 'project' });
    render(QuickOpen, { projectId: 'p1', source: 'project' });
    await userEvent.type(await screen.findByRole('combobox', { name: 'Ouvrir un fichier' }), 'readme');
    await screen.findByRole('option', { name: /README/ });
    await userEvent.keyboard('{Enter}');
    await expect.poll(() => app.editor.p1.places.project.active).toBe('README.md');
    await expect.poll(() => container.querySelector('.cm-content')?.textContent?.startsWith('# demo')).toBe(true);
    const view = CodeMirror.findFromDOM(container.querySelector('.cm-editor') as HTMLElement)!;
    view.contentDOM.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', altKey: true, bubbles: true, cancelable: true }));
    await expect.poll(() => app.editor.p1.places.project.active).toBe('src/app.ts');
    expect(await screen.findByText('Ln 1, Col 7')).toBeInTheDocument();
  });

  it('is not answered by an editor that is gone: the palette opens the file itself', async () => {
    backend();
    await app.openEditor({ source: 'project', path: 'src/app.ts' });
    const { unmount } = render(EditorView, { project: project() });
    await screen.findByRole('tab', { name: /app\.ts/ });
    unmount();
    app.modal = { kind: 'quickOpen', projectId: 'p1', source: 'project' };
    render(QuickOpen, { projectId: 'p1', source: 'project' });
    await userEvent.type(await screen.findByRole('combobox', { name: 'Ouvrir un fichier' }), 'readme');
    await screen.findByRole('option', { name: /README/ });
    await userEvent.keyboard('{Enter}');
    await expect.poll(() => app.editor.p1.places.project.active).toBe('README.md');
  });
});

describe('EditorView left column width', () => {
  const Real = globalThis.ResizeObserver;
  /** The editor area reports being `area` px wide, as the window's size gives it. */
  function areaOf(area: number) {
    globalThis.ResizeObserver = class {
      constructor(private cb: ResizeObserverCallback) {}
      observe() {
        this.cb([{ contentRect: { width: area } } as ResizeObserverEntry], this as unknown as ResizeObserver);
      }
      unobserve() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
  }
  beforeEach(() => {
    localStorage.clear();
    // jsdom has no pointer capture.
    Element.prototype.setPointerCapture = vi.fn();
    resetApp({ agents: [agent()] });
    app.git.p1 = gitInfo({ modified: 1 });
    backend();
  });
  afterEach(() => {
    globalThis.ResizeObserver = Real;
    localStorage.clear();
  });

  const handle = () => screen.getByRole('separator', { name: 'Largeur de la colonne des fichiers' });
  const column = (container: HTMLElement) => container.querySelector('aside.editor-side') as HTMLElement;
  async function drag(from: number, to: number) {
    await fireEvent.pointerDown(handle(), { clientX: from, pointerId: 1, button: 0 });
    await fireEvent.pointerMove(handle(), { clientX: to, pointerId: 1, buttons: 1 });
    await fireEvent.pointerUp(handle(), { pointerId: 1 });
  }

  it('is 240 px wide until it is resized, with a handle between it and the code', async () => {
    areaOf(1000);
    await app.openEditor({ source: 'project' });
    const { container } = render(EditorView, { project: project() });
    expect(column(container)).toHaveStyle({ width: '240px' });
    expect(handle()).toHaveAttribute('aria-orientation', 'vertical');
    expect(handle()).toHaveAttribute('aria-valuenow', '240');
    expect(handle()).toHaveAttribute('aria-valuemin', '160');
    expect(handle()).toHaveAttribute('aria-valuemax', '500');
    expect(column(container).nextElementSibling).toBe(handle());
    expect(handle().nextElementSibling?.tagName).toBe('SECTION');
  });

  it('follows the handle when it is dragged, and keeps the width for the next time', async () => {
    areaOf(1000);
    await app.openEditor({ source: 'project' });
    const first = render(EditorView, { project: project() });
    await drag(300, 380);
    expect(column(first.container)).toHaveStyle({ width: '320px' });
    expect(handle()).toHaveAttribute('aria-valuenow', '320');
    expect(localStorage.getItem('escouade.editor.treeWidth')).toBe('320');
    first.unmount();
    const again = render(EditorView, { project: project() });
    expect(column(again.container)).toHaveStyle({ width: '320px' });
  });

  it('does not write the preference at every step of the drag, only where it is let go', async () => {
    areaOf(1000);
    await app.openEditor({ source: 'project' });
    const { container } = render(EditorView, { project: project() });
    await fireEvent.pointerDown(handle(), { clientX: 300, pointerId: 1, button: 0 });
    await fireEvent.pointerMove(handle(), { clientX: 340, pointerId: 1, buttons: 1 });
    expect(column(container)).toHaveStyle({ width: '280px' });
    expect(localStorage.getItem('escouade.editor.treeWidth')).toBeNull();
    await fireEvent.pointerUp(handle(), { pointerId: 1 });
    expect(localStorage.getItem('escouade.editor.treeWidth')).toBe('280');
  });

  it('stops at 160 px and at half of the editor area', async () => {
    areaOf(800);
    await app.openEditor({ source: 'project' });
    const { container } = render(EditorView, { project: project() });
    await drag(300, -600);
    expect(column(container)).toHaveStyle({ width: '160px' });
    await drag(300, 1500);
    expect(column(container)).toHaveStyle({ width: '400px' });
    expect(handle()).toHaveAttribute('aria-valuemax', '400');
    expect(localStorage.getItem('escouade.editor.treeWidth')).toBe('400');
  });

  it('goes back to 240 px on a double click', async () => {
    areaOf(1000);
    localStorage.setItem('escouade.editor.treeWidth', '420');
    await app.openEditor({ source: 'project' });
    const { container } = render(EditorView, { project: project() });
    expect(column(container)).toHaveStyle({ width: '420px' });
    await fireEvent.dblClick(handle());
    expect(column(container)).toHaveStyle({ width: '240px' });
    expect(localStorage.getItem('escouade.editor.treeWidth')).toBe('240');
  });

  it('moves by 16 px with the arrow keys, and to the bounds with Home and End', async () => {
    areaOf(1000);
    await app.openEditor({ source: 'project' });
    const { container } = render(EditorView, { project: project() });
    handle().focus();
    await userEvent.keyboard('{ArrowRight}');
    expect(column(container)).toHaveStyle({ width: '256px' });
    expect(localStorage.getItem('escouade.editor.treeWidth')).toBe('256');
    await userEvent.keyboard('{ArrowLeft}{ArrowLeft}');
    expect(column(container)).toHaveStyle({ width: '224px' });
    await userEvent.keyboard('{End}');
    expect(column(container)).toHaveStyle({ width: '500px' });
    await userEvent.keyboard('{Home}');
    expect(column(container)).toHaveStyle({ width: '160px' });
    expect(handle()).toHaveFocus();
  });

  it('shows a width kept that is too wide for the area at the most the area gives, and keeps what was chosen', async () => {
    areaOf(600);
    localStorage.setItem('escouade.editor.treeWidth', '400');
    await app.openEditor({ source: 'project' });
    const { container } = render(EditorView, { project: project() });
    expect(column(container)).toHaveStyle({ width: '300px' });
    expect(handle()).toHaveAttribute('aria-valuenow', '300');
    expect(localStorage.getItem('escouade.editor.treeWidth')).toBe('400');
  });

  it('takes the usual width for a preference that is not a width', async () => {
    areaOf(1000);
    localStorage.setItem('escouade.editor.treeWidth', 'large');
    await app.openEditor({ source: 'project' });
    const { container } = render(EditorView, { project: project() });
    expect(column(container)).toHaveStyle({ width: '240px' });
  });
});
