import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { buffers } from '../../lib/editor/buffers.svelte';
import { trees } from '../../lib/editor/trees.svelte';
import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import EditorView from './EditorView.svelte';

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
