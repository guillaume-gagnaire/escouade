import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { recentFiles, setEditorJump } from '../lib/editor/quick-open';
import { trees } from '../lib/editor/trees.svelte';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import QuickOpen from './QuickOpen.svelte';

const FILES = ['.env', '.gitignore', 'README.md', 'src/app.ts', 'src/components/ConvSearch.svelte', 'src/lib/format.ts'];

/** The palette over the files of `source`, whose tree the fake backend answers with. */
function setup(over: { files?: string[]; ignored?: string[]; source?: string; fsTree?: () => unknown; label?: string } = {}) {
  const source = over.source ?? 'project';
  app.modal = { kind: 'quickOpen', projectId: 'p1', source };
  const backend = fakeBackend({
    fs_tree:
      over.fsTree ??
      (() => ({ root: 'C:/code/demo-api', files: over.files ?? FILES, truncated: false, ignored: over.ignored ?? ['.env'] })),
    fs_read: () => ({ kind: 'text', text: '', size: 0, hash: 'h', eol: 'lf', bom: false }),
    fs_base: () => null,
  });
  render(QuickOpen, { projectId: 'p1', source });
  return { backend, field: screen.getByRole('combobox', { name: over.label ?? 'Ouvrir un fichier' }) };
}

const names = () => screen.queryAllByRole('option').map((o) => o.querySelector('.name')?.textContent);
const selected = () => screen.getAllByRole('option').findIndex((o) => o.getAttribute('aria-selected') === 'true');
const active = () => app.editor.p1?.places.project?.active;
const loaded = () => waitFor(() => expect(names()).toHaveLength(FILES.length));

describe('QuickOpen', () => {
  beforeEach(() =>
    resetApp({
      projects: [project()],
      agents: [agent({ worktree: { path: 'C:/wt/a1', branch: 'agent/a1', baseBranch: 'main' } })],
    }),
  );

  it('opens over everything on its field, and reads the files of its source', async () => {
    const { backend, field } = setup({ source: 'a1' });
    expect(screen.getByRole('dialog', { name: 'Ouvrir un fichier' })).toHaveAttribute('aria-modal', 'true');
    expect(field).toHaveFocus();
    expect(field).toHaveAttribute('placeholder', 'Nom du fichier, ou nom:42 pour une ligne');
    await loaded();
    expect(backend.called('fs_tree').map((c) => c.args)).toEqual([{ projectId: 'p1', agentId: 'a1' }]);
    expect(screen.getByText('worktree · refacto-auth')).toBeInTheDocument();
  });

  it('says the project when the files are those of its own checkout', async () => {
    setup();
    await loaded();
    expect(screen.getByText('demo-api')).toBeInTheDocument();
  });

  it('puts the files opened last first when nothing is typed, the others after them', async () => {
    recentFiles.note('p1', 'project', 'src/lib/format.ts');
    recentFiles.note('p1', 'project', 'README.md');
    recentFiles.note('p1', 'project', 'vanished.ts');
    setup();
    await loaded();
    expect(names().slice(0, 4)).toEqual(['README.md', 'format.ts', '.env', '.gitignore']);
  });

  it('leaves out of the recent files the one the editor shows: Enter goes back to the one before', async () => {
    await app.openEditor({ source: 'project', path: 'src/lib/format.ts' });
    await app.openEditor({ source: 'project', path: 'README.md' });
    setup();
    await loaded();
    expect(names().slice(0, 2)).toEqual(['format.ts', '.env']);
  });

  it('writes the name in bold and its folder after it, in grey', async () => {
    const { field } = setup();
    await userEvent.type(field, 'app');
    const [option] = await screen.findAllByRole('option');
    expect(option.querySelector('.name')).toHaveTextContent(/^app\.ts$/);
    expect(option.querySelector('.dir')).toHaveTextContent(/^src$/);
    // A file at the root has no folder to say.
    await userEvent.clear(field);
    await userEvent.type(field, 'readme');
    await waitFor(() => expect(names()).toEqual(['README.md']));
    expect(screen.getAllByRole('option')[0].querySelector('.dir')).toBeNull();
  });

  it('ranks what is typed: the name first, then the letters in order', async () => {
    const { field } = setup();
    await userEvent.type(field, 'format');
    await waitFor(() => expect(names()).toEqual(['format.ts']));
    await userEvent.clear(field);
    await userEvent.type(field, 'cvsrch');
    await waitFor(() => expect(names()).toEqual(['ConvSearch.svelte']));
    await userEvent.clear(field);
    await userEvent.type(field, 'zzz');
    await waitFor(() => expect(names()).toEqual([]));
    expect(screen.getByRole('status')).toHaveTextContent('Aucun fichier ne correspond.');
    expect(field).toHaveAttribute('aria-expanded', 'false');
  });

  it('finds the ignored files the project copies, and says they are ignored', async () => {
    const { field } = setup();
    await userEvent.type(field, '.env');
    const [option] = await screen.findAllByRole('option');
    expect(option.querySelector('.name')).toHaveTextContent('.env');
    expect(option).toHaveTextContent('ignoré par git');
    await userEvent.clear(field);
    await userEvent.type(field, 'readme');
    await waitFor(() => expect(names()).toEqual(['README.md']));
    expect(screen.getAllByRole('option')[0]).not.toHaveTextContent('ignoré par git');
  });

  it('moves through the results with ↑ ↓, round the ends, and tells the screen reader which one', async () => {
    const { field } = setup();
    await loaded();
    expect(selected()).toBe(0);
    expect(field).toHaveAttribute('aria-activedescendant', screen.getAllByRole('option')[0].id);
    await userEvent.keyboard('{ArrowUp}');
    expect(selected()).toBe(FILES.length - 1);
    await userEvent.keyboard('{ArrowDown}{ArrowDown}');
    expect(selected()).toBe(1);
    expect(field).toHaveAttribute('aria-activedescendant', screen.getAllByRole('option')[1].id);
  });

  it('opens the chosen file with Enter, at its top, and closes', async () => {
    const { field } = setup();
    await userEvent.type(field, 'app');
    await screen.findAllByRole('option');
    await userEvent.keyboard('{Enter}');
    expect(app.modal).toBeNull();
    await waitFor(() => expect(active()).toBe('src/app.ts'));
    expect(app.editor.p1.reveal).toMatchObject({ path: 'src/app.ts', line: 1 });
    expect(app.editor.p1.source).toBe('project');
    expect(app.editorOn).toBe(true);
  });

  it('opens the file picked with ↓ Enter', async () => {
    setup();
    await loaded();
    await userEvent.keyboard('{ArrowDown}{Enter}');
    await waitFor(() => expect(active()).toBe('.gitignore'));
    expect(app.modal).toBeNull();
  });

  it('opens the file clicked', async () => {
    setup();
    await loaded();
    await userEvent.click(screen.getByRole('option', { name: /format\.ts/ }));
    await waitFor(() => expect(active()).toBe('src/lib/format.ts'));
    expect(app.modal).toBeNull();
  });

  it('opens the file at the line typed after its name', async () => {
    const { field } = setup();
    await userEvent.type(field, 'app:42');
    await waitFor(() => expect(names()).toEqual(['app.ts']));
    expect(screen.getByRole('status')).toHaveTextContent('ligne 42');
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(active()).toBe('src/app.ts'));
    expect(app.editor.p1.reveal).toMatchObject({ path: 'src/app.ts', line: 42 });
    expect(app.editor.p1.reveal?.col).toBeUndefined();
  });

  it('opens it at its column too, when given as an error of a stack trace gives it', async () => {
    const { field } = setup();
    await userEvent.type(field, 'src/lib/format.ts:7:3{Enter}');
    await waitFor(() => expect(active()).toBe('src/lib/format.ts'));
    expect(app.editor.p1.reveal).toMatchObject({ path: 'src/lib/format.ts', line: 7, col: 3 });
  });

  it('leaves the editor to open the file when it is on screen, for its history to keep the place left', async () => {
    const jump = vi.fn();
    const unset = setEditorJump(jump);
    try {
      await app.openEditor({ source: 'project', path: 'README.md' });
      const { field } = setup();
      await userEvent.type(field, 'app:12');
      await waitFor(() => expect(names()).toEqual(['app.ts']));
      await userEvent.keyboard('{Enter}');
      await waitFor(() => expect(jump).toHaveBeenCalledTimes(1));
      expect(jump).toHaveBeenCalledWith({ path: 'src/app.ts', line: 12 });
      // It opened nothing itself: the editor does.
      expect(app.editor.p1.places.project.open).toEqual(['README.md']);
    } finally {
      unset();
    }
  });

  it('opens the file itself when no editor is on screen, or when the one that is shows another source', async () => {
    const jump = vi.fn();
    const unset = setEditorJump(jump);
    try {
      const { field } = setup();
      await userEvent.type(field, 'app{Enter}');
      await waitFor(() => expect(active()).toBe('src/app.ts'));
      app.closeEditor();
      expect(jump).not.toHaveBeenCalled();

      // The editor is open, on the worktree: the palette of the project’s files is not its to answer.
      await app.openEditor({ source: 'a1', path: 'x.ts' });
      app.modal = { kind: 'quickOpen', projectId: 'p1', source: 'project' };
      await userEvent.clear(field);
      await userEvent.type(field, 'format{Enter}');
      await waitFor(() => expect(active()).toBe('src/lib/format.ts'));
      expect(jump).not.toHaveBeenCalled();
    } finally {
      unset();
    }
  });

  it('closes on Escape and on a click outside, and opens nothing', async () => {
    setup();
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
    expect(app.editor.p1).toBeUndefined();
    app.modal = { kind: 'quickOpen', projectId: 'p1', source: 'project' };
    await userEvent.click(document.querySelector('.overlay')!);
    expect(app.modal).toBeNull();
  });

  it('does nothing on Enter with no file to open', async () => {
    const { field } = setup();
    await userEvent.type(field, 'zzz{Enter}');
    expect(app.modal).toEqual({ kind: 'quickOpen', projectId: 'p1', source: 'project' });
    expect(app.editor.p1).toBeUndefined();
  });

  it('shows 50 files at most, and tells to be more precise', async () => {
    const many = Array.from({ length: 80 }, (_, i) => `src/file${String(i).padStart(2, '0')}.ts`);
    const { field } = setup({ files: many, ignored: [] });
    await userEvent.type(field, 'file');
    await waitFor(() => expect(names()).toHaveLength(50));
    expect(screen.getByRole('status')).toHaveTextContent('Les 50 premiers résultats : précise ta recherche.');
  });

  it('says it is reading the files, then how many there are', async () => {
    let answer: (v: unknown) => void = () => {};
    setup({ fsTree: () => new Promise((r) => (answer = r)) });
    expect(screen.getByRole('status')).toHaveTextContent('Chargement des fichiers…');
    answer({ root: 'C:/code/demo-api', files: FILES, truncated: true, ignored: [] });
    await loaded();
    expect(screen.getByRole('status')).toHaveTextContent('6 fichiers · liste tronquée');
  });

  it('groups the thousands of the files it counts', async () => {
    const files = Array.from({ length: 1234 }, (_, i) => `src/file${i}.ts`);
    setup({ files, ignored: [] });
    // The space of the grouping is Intl’s: `toHaveTextContent` reads any space as one.
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('1 234 fichiers'));
  });

  it('shows the files it already has while it reads them again, and tells when it cannot', async () => {
    trees.all['p1|project'] = { root: 'C:/code/demo-api', files: ['old.ts'], truncated: false, ignored: [] };
    setup({ fsTree: () => Promise.reject('git est introuvable') });
    expect(names()).toEqual(['old.ts']);
    expect(await screen.findByRole('alert')).toHaveTextContent('git est introuvable');
    expect(names()).toEqual(['old.ts']);
  });
});

describe('QuickOpen in English', () => {
  beforeEach(() => {
    resetApp({
      projects: [project()],
      agents: [agent({ worktree: { path: 'C:/wt/a1', branch: 'agent/a1', baseBranch: 'main' } })],
    });
    setLang('en');
  });

  it('names its field, its place and its hints in English, and counts the files', async () => {
    const files = Array.from({ length: 1234 }, (_, i) => `src/file${i}.ts`);
    const { field } = setup({ files, ignored: ['src/file1.ts'], source: 'a1', label: 'Open a file' });
    expect(screen.getByRole('dialog', { name: 'Open a file' })).toBeInTheDocument();
    expect(field).toHaveAttribute('placeholder', 'File name, or name:42 for a line');
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
    expect(screen.getByText('worktree · refacto-auth')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('1,234 files'));
    expect(screen.getByRole('listbox', { name: 'Files' })).toBeInTheDocument();
    expect(document.querySelector('.keys')).toHaveTextContent('choose · Enter open · Esc close');
    await userEvent.type(field, 'file1.ts:42');
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent('The first 50 results: narrow down your search. · opens at line 42'),
    );
    expect(screen.getByText('ignored by git')).toBeInTheDocument();
    await userEvent.clear(field);
    await userEvent.type(field, 'file1233');
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('1 result'));
    await userEvent.clear(field);
    await userEvent.type(field, 'zzz');
    expect(screen.getByRole('status')).toHaveTextContent('No file matches.');
  });
});
