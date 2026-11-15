import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../../test/ipc';
import VscodeView from './VscodeView.svelte';

// The workbench of VS Code does not start in jsdom: what the view asks of it is looked at.
const wb = vi.hoisted(() => ({
  workbench: vi.fn(async () => {}),
  attach: vi.fn(),
  detach: vi.fn(),
  openFile: vi.fn(async () => {}),
}));
vi.mock('../../lib/vscode/workbench', () => wb);

describe('VscodeView', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({
      agents: [agent({ worktree: { path: 'C:\\code\\demo-api\\.claude\\worktrees\\refacto-auth', branch: 'refacto-auth' } } as never)],
    });
    fakeBackend({ fs_tree: () => ({ root: 'C:\\code\\demo-api', files: [], truncated: false, ignored: [] }) });
    app.settings.editorFontSize = 15;
    for (const f of Object.values(wb)) f.mockClear();
  });

  it('lends its place to the workbench, on the project’s folder, and opens the file asked for', async () => {
    await app.openEditor({ projectId: 'p1', source: 'project', path: 'src/app.ts' });
    const { unmount } = render(VscodeView, { project: project() });
    expect(screen.getByText('Chargement de la vue VS Code…')).toBeTruthy();
    await waitFor(() => expect(wb.attach).toHaveBeenCalled());
    expect(wb.workbench).toHaveBeenCalledWith({ projectId: 'p1', agentId: null }, 'demo-api', 15);
    expect(wb.attach.mock.calls[0][0]).toBe(screen.getByTestId('vscode-view').querySelector('.vscode-place'));
    await waitFor(() => expect(wb.openFile).toHaveBeenCalledWith({ projectId: 'p1', agentId: null }, 'src/app.ts'));
    expect(screen.queryByText('Chargement de la vue VS Code…')).toBeNull();
    unmount();
    expect(wb.detach).toHaveBeenCalledWith(wb.attach.mock.calls[0][0]);
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

  it('says why the workbench did not start', async () => {
    wb.workbench.mockRejectedValueOnce(new Error('boom'));
    await app.openEditor({ projectId: 'p1', source: 'project' });
    render(VscodeView, { project: project() });
    expect(await screen.findByRole('alert')).toHaveTextContent('La vue VS Code n’a pas démarré : Error: boom');
    expect(wb.attach).not.toHaveBeenCalled();
  });
});
