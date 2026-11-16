import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { page, REOPEN_KEY, reopening } from '../../lib/vscode/load';
import { viewUnsaved } from '../../lib/vscode/unsaved.svelte';
import { viewShown } from '../../lib/vscode/shown.svelte';
import { agent, fakeBackend, project, resetApp } from '../../test/ipc';
import VscodeView from './VscodeView.svelte';

// The workbench of VS Code does not start in jsdom: what the view asks of it is looked at.
const wb = vi.hoisted(() => ({
  workbench: vi.fn(async () => {}),
  attach: vi.fn(),
  detach: vi.fn(),
  openFile: vi.fn(async () => {}),
  focus: vi.fn(async () => {}),
  hasTabs: vi.fn(() => false),
  setFontSize: vi.fn(),
  openTabs: vi.fn(() => [{ projectId: 'p1', source: 'project', path: 'src/app.ts' }]),
}));
vi.mock('../../lib/vscode/workbench', () => wb);
// The French texts of VS Code (lib/vscode/load.ts): not looked at here.
vi.mock('@codingame/monaco-vscode-language-pack-fr/translations/main.i18n.json.js', () => ({ default: [] }));
vi.mock('@codingame/monaco-vscode-language-pack-fr', () => ({}));

const PROJECT = { projectId: 'p1', agentId: null };

describe('VscodeView', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({
      agents: [agent({ worktree: { path: 'C:\\code\\demo-api\\.claude\\worktrees\\refacto-auth', branch: 'refacto-auth' } } as never)],
    });
    fakeBackend({
      fs_tree: () => ({ root: 'C:\\code\\demo-api', files: ['src/app.ts'], truncated: false, ignored: [] }),
      set_unsaved: () => null,
    });
    app.settings.editorFontSize = 15;
    for (const f of Object.values(wb)) f.mockClear();
    wb.hasTabs.mockReturnValue(false);
    viewUnsaved.set([]);
    app.modal = null;
    // A file opened is a tab, as in the workbench.
    wb.openFile.mockImplementation(async () => void wb.hasTabs.mockReturnValue(true));
  });

  it('lends its place to the workbench, on the project’s folder, and opens the file asked for', async () => {
    await app.openEditor({ projectId: 'p1', source: 'project', path: 'src/app.ts' });
    const { unmount } = render(VscodeView, { project: project() });
    expect(screen.getByText('Chargement de la vue VS Code…')).toBeTruthy();
    await waitFor(() => expect(wb.attach).toHaveBeenCalled());
    expect(wb.workbench).toHaveBeenCalledWith(PROJECT, 'demo-api', 15);
    expect(wb.attach.mock.calls[0][0]).toBe(screen.getByTestId('vscode-view').querySelector('.vscode-place'));
    await waitFor(() => expect(wb.openFile).toHaveBeenCalledWith(PROJECT, 'src/app.ts', expect.objectContaining({ path: 'src/app.ts' })));
    expect(screen.queryByText('Chargement de la vue VS Code…')).toBeNull();
    unmount();
    expect(wb.detach).toHaveBeenCalledWith(wb.attach.mock.calls[0][0]);
  });

  it('opens a file asked for at its line and column, each request once, even the file it shows', async () => {
    await app.openEditor({ projectId: 'p1', source: 'project', path: 'src/app.ts', line: 7, col: 3 });
    const { unmount } = render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.openFile).toHaveBeenCalledWith(PROJECT, 'src/app.ts', expect.objectContaining({ line: 7, col: 3 })));
    // Taken: the request is not left for a view drawn again.
    await waitFor(() => expect(app.editor.p1.reveal).toBeNull());
    expect(wb.openFile).toHaveBeenCalledTimes(1);
    // The same file again, at another line: asked again.
    await app.openEditor({ projectId: 'p1', source: 'project', path: 'src/app.ts', line: 12 });
    await waitFor(() => expect(wb.openFile).toHaveBeenLastCalledWith(PROJECT, 'src/app.ts', expect.objectContaining({ line: 12 })));
    expect(wb.openFile).toHaveBeenCalledTimes(2);
    // Drawn again (a pane moved), with tabs already: nothing opened, nothing moved.
    unmount();
    wb.hasTabs.mockReturnValue(true);
    render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.attach).toHaveBeenCalledTimes(2));
    await new Promise((r) => setTimeout(r, 20));
    expect(wb.openFile).toHaveBeenCalledTimes(2);
  });

  it('takes the keyboard once shown', async () => {
    await app.openEditor({ projectId: 'p1', source: 'project' });
    render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.focus).toHaveBeenCalledTimes(1));
    expect(wb.openFile).not.toHaveBeenCalled();
  });

  it('shows the agent’s worktree as a folder of its own when it is picked', async () => {
    await app.openEditor({ projectId: 'p1', source: 'project' });
    render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.attach).toHaveBeenCalled());
    await userEvent.click(screen.getByTestId('vscode-view').querySelector<HTMLElement>('.vscode-bar .src')!);
    await userEvent.click(screen.getAllByRole('menuitemradio')[1]);
    await waitFor(() =>
      expect(wb.workbench).toHaveBeenLastCalledWith({ projectId: 'p1', agentId: 'a1' }, expect.stringContaining('refacto-auth'), 15),
    );
    expect(app.editor.p1?.source).toBe('a1');
  });

  it('asks before « Conversation » closes it with files modified, and closes at once without', async () => {
    await app.openEditor({ projectId: 'p1', source: 'project' });
    render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.attach).toHaveBeenCalled());
    viewUnsaved.set([{ projectId: 'p1', source: 'project', path: 'src/app.ts', uri: 'file:///c:/code/demo-api/src/app.ts' }]);
    await userEvent.click(screen.getByTestId('vscode-back'));
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Enregistrer les modifications de « app.ts » ?',
      confirm: 'Enregistrer',
      alt: { label: 'Ne pas enregistrer' },
    });
    expect(app.editor.p1.on).toBe(true);
    viewUnsaved.set([]);
    app.modal = null;
    await userEvent.click(screen.getByTestId('vscode-back'));
    expect(app.modal).toBeNull();
    expect(app.editor.p1.on).toBe(false);
  });

  it('says why the workbench did not start', async () => {
    wb.workbench.mockRejectedValueOnce(new Error('boom'));
    await app.openEditor({ projectId: 'p1', source: 'project' });
    render(VscodeView, { project: project() });
    expect(await screen.findByRole('alert')).toHaveTextContent('La vue VS Code n’a pas démarré : Error: boom');
    expect(wb.attach).not.toHaveBeenCalled();
  });

  it('takes « Taille du texte de l’éditeur » at once', async () => {
    await app.openEditor({ projectId: 'p1', source: 'project' });
    render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.setFontSize).toHaveBeenCalledWith(15));
    app.settings.editorFontSize = 18;
    await waitFor(() => expect(wb.setFontSize).toHaveBeenLastCalledWith(18));
  });

  it('offers to reload the window when Escouade changes language, the files modified asked about first, its tabs kept', async () => {
    const reload = vi.spyOn(page, 'reload').mockImplementation(() => {});
    await app.openEditor({ projectId: 'p1', source: 'project' });
    render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.attach).toHaveBeenCalled());
    // Loaded in French: nothing to say.
    expect(screen.queryByTestId('vscode-lang-note')).toBeNull();
    setLang('en');
    expect(await screen.findByTestId('vscode-lang-note')).toHaveTextContent('VS Code changes language when the window reloads.');
    viewUnsaved.set([{ projectId: 'p1', source: 'project', path: 'src/app.ts', uri: 'file:///c:/code/demo-api/src/app.ts' }]);
    await userEvent.click(screen.getByTestId('vscode-reload'));
    // Asked first, as any close: nothing reloaded until it is answered.
    expect(app.modal).toMatchObject({ kind: 'confirm', confirm: 'Save' });
    expect(reload).not.toHaveBeenCalled();
    viewUnsaved.set([]);
    app.modal = null;
    await userEvent.click(screen.getByTestId('vscode-reload'));
    expect(reload).toHaveBeenCalledTimes(1);
    expect(JSON.parse(sessionStorage.getItem(REOPEN_KEY) ?? '[]')).toEqual([{ projectId: 'p1', source: 'project', path: 'src/app.ts' }]);
    sessionStorage.removeItem(REOPEN_KEY);
    // Back in French: the note goes.
    setLang('fr');
    await waitFor(() => expect(screen.queryByTestId('vscode-lang-note')).toBeNull());
    reload.mockRestore();
  });

  it('opens again the tabs kept by a reload, the one shown last', async () => {
    reopening.tabs = [
      { projectId: 'p1', source: 'a1', path: 'src/b.ts' },
      { projectId: 'p1', source: 'project', path: 'src/app.ts' },
    ];
    await app.openEditor({ projectId: 'p1', source: 'project' });
    render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.openFile).toHaveBeenCalledTimes(2));
    expect(wb.openFile.mock.calls.map((c) => c.slice(0, 2))).toEqual([
      [{ projectId: 'p1', agentId: 'a1' }, 'src/b.ts'],
      [PROJECT, 'src/app.ts'],
    ]);
    expect(wb.workbench).toHaveBeenCalledWith({ projectId: 'p1', agentId: 'a1' }, expect.stringContaining('refacto-auth'), 15);
    expect(reopening.tabs).toEqual([]);
  });

  it('says how the file shown differs from its base, and compares it from its bar (ESC-476)', async () => {
    const compare = vi.fn(async () => {});
    viewShown.serve(compare);
    viewShown.set(null);
    render(VscodeView, { project: project() });
    await waitFor(() => expect(wb.attach).toHaveBeenCalled());
    expect(screen.queryByTestId('vscode-changes')).toBeNull();
    const file = { projectId: 'p1', source: 'a1', path: 'src/app.ts', count: 5, reference: 'main', isNew: false };
    viewShown.set(file);
    await waitFor(() => expect(screen.getByTestId('vscode-changes').textContent?.trim()).toBe('5 lignes modifiées par rapport à main'));
    await userEvent.click(screen.getByTestId('vscode-compare'));
    expect(compare).toHaveBeenCalledTimes(1);
    viewShown.set({ ...file, count: 1 });
    await waitFor(() => expect(screen.getByTestId('vscode-changes').textContent?.trim()).toBe('1 ligne modifiée par rapport à main'));
    viewShown.set({ ...file, isNew: true });
    await waitFor(() => expect(screen.getByTestId('vscode-changes').textContent?.trim()).toBe('Nouveau fichier · absent de main'));
    // The same as its base: said, nothing to compare.
    viewShown.set({ ...file, count: 0, reference: 'HEAD' });
    await waitFor(() => expect(screen.getByTestId('vscode-changes').textContent?.trim()).toBe('Identique à HEAD'));
    expect(screen.queryByTestId('vscode-compare')).toBeNull();
    // A file of another project: not this view's to tell.
    viewShown.set({ ...file, projectId: 'p2' });
    await waitFor(() => expect(screen.queryByTestId('vscode-changes')).toBeNull());
    viewShown.set(null);
  });
});
