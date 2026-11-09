import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { settingsForm } from '../../lib/settings.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import BoardTab from './BoardTab.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

// The tab is also driven through the settings window (SettingsModal.test.ts, in French); here it is alone, in English.
describe('BoardTab in English', () => {
  beforeEach(() => {
    resetApp({ projects: [project()] });
    app.git.p1 = gitInfo();
    fakeBackend({ git_branches: () => ['main', 'release'] });
    settingsForm.open({ page: 'board', projectId: 'p1' });
    setLang('en');
  });

  it('writes what validating a ticket does, and its options', async () => {
    render(BoardTab, { project: app.projects[0] });
    const actions = screen.getByRole('radiogroup', { name: 'When I approve a ticket in “To review”' });
    expect(within(actions).getByRole('radio', { name: /Merge into a branch/ })).toHaveAttribute('aria-checked', 'true');
    expect(
      within(actions).getByText('Pushes ticket/<key>, opens its PR to the target branch and follows it “In review” until it’s merged.'),
    ).toBeInTheDocument();
    expect(within(actions).getByRole('radio', { name: /Leave as is/ })).toBeInTheDocument();
    expect(screen.getByText('Target branch')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Squash' })).toHaveAttribute('aria-pressed', 'true');
    // Opening a pull request has its own option, not the strategy.
    await userEvent.click(within(actions).getByRole('radio', { name: /Open a pull request/ }));
    expect(screen.getByRole('switch', { name: 'Draft PR' })).toBeInTheDocument();
    expect(screen.queryByText('Strategy')).not.toBeInTheDocument();
  });

  it('leaves the answers under the threads off by default, and settles a conflict by a merge unless told rebase (ESC-61)', async () => {
    render(BoardTab, { project: app.projects[0] });
    expect(screen.queryByRole('switch', { name: 'Reply under fixed threads' })).toBeNull();
    const actions = screen.getByRole('radiogroup', { name: 'When I approve a ticket in “To review”' });
    await userEvent.click(within(actions).getByRole('radio', { name: /Open a pull request/ }));
    const reply = screen.getByRole('switch', { name: 'Reply under fixed threads' });
    expect(reply).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText(/Escouade never resolves a thread/)).toBeInTheDocument();
    await userEvent.click(reply);
    expect(settingsForm.project!.board.replyThreads).toBe(true);
    const update = screen.getByRole('group', { name: 'Branch update' });
    expect(within(update).getByRole('button', { name: 'Merge' })).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(update).getByRole('button', { name: 'Rebase' }));
    expect(settingsForm.project!.board.prUpdate).toBe('rebase');
  });

  it('removes the worktree of a done or deleted ticket by default, and says what it keeps (ESC-143)', async () => {
    render(BoardTab, { project: app.projects[0] });
    const cleanup = screen.getByRole('switch', { name: 'Delete the worktree of a done or deleted ticket' });
    expect(cleanup).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByText(/A worktree with uncommitted changes or commits neither merged nor pushed is kept/)).toBeInTheDocument();
    await userEvent.click(cleanup);
    expect(settingsForm.project!.board.cleanup).toBe(false);
  });

  it('leaves the pause on a red target off by default, and turns it on (ESC-63)', async () => {
    render(BoardTab, { project: app.projects[0] });
    const pause = screen.getByRole('switch', { name: 'Pause while the target’s CI is red' });
    expect(pause).toHaveAttribute('aria-checked', 'false');
    expect(screen.getByText(/The autopilot starts no ticket while the CI of the target branch fails/)).toBeInTheDocument();
    await userEvent.click(pause);
    expect(settingsForm.project!.board.pauseOnRedTarget).toBe(true);
  });

  it('writes the tests, the commit message and the conflicts', () => {
    render(BoardTab, { project: app.projects[0] });
    expect(screen.getByRole('textbox', { name: 'Test command' })).toHaveAttribute('placeholder', 'e.g. npm test');
    expect(screen.getByRole('switch', { name: 'Re-run the tests first' })).toBeDisabled();
    expect(screen.getByRole('switch', { name: 'Delete the worktree of a done or deleted ticket' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Generated commit message' })).toBeInTheDocument();
    expect(screen.getByText('feat: limit login attempts [DEM-42]')).toBeInTheDocument();
    const conflicts = screen.getByRole('group', { name: 'On conflict' });
    expect(
      within(conflicts)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Ask me', 'Let the agent resolve', 'Cancel']);
  });

  it('writes the autopilot and the agents; the pause above quota is in Accounts and harnesses (R4)', () => {
    render(BoardTab, { project: app.projects[0] });
    expect(screen.getByRole('switch', { name: 'Assign tickets automatically' })).toBeInTheDocument();
    expect(screen.queryByRole('group', { name: /above quota/ })).toBeNull();
    expect(screen.getByRole('heading', { name: 'Agents' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'In parallel' })).toBeInTheDocument();
    expect(within(screen.getByRole('combobox', { name: 'Model' })).getByRole('option', { name: 'Default (Sonnet)' })).toBeInTheDocument();
    const effort = within(screen.getByRole('combobox', { name: 'Effort' }));
    expect(effort.getByRole('option', { name: 'Default' })).toBeInTheDocument();
    expect(effort.getByRole('option', { name: 'Low' })).toBeInTheDocument();
  });

  it('follows the language when it changes, the choices built in the script included', () => {
    render(BoardTab, { project: app.projects[0] });
    expect(screen.getByRole('radio', { name: /Merge into a branch/ })).toBeInTheDocument();
    setLang('fr');
    flushSync();
    expect(screen.getByRole('radio', { name: /Merger dans une branche/ })).toBeInTheDocument();
    expect(within(screen.getByRole('group', { name: 'En parallèle' })).getAllByRole('button').length).toBeGreaterThan(0);
    expect(screen.getByText('feat: limiter les tentatives de connexion [DEM-42]')).toBeInTheDocument();
  });

  it('ends with the project’s workflow (R6: Workflow left the project’s settings for its Kanban)', () => {
    render(BoardTab, { project: app.projects[0] });
    const titles = screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent);
    expect(titles.at(-1)).toBe('Workflow');
    expect(titles.slice(0, -1)).toEqual(['When I approve a ticket in “To review”', 'Before and after', 'Autopilot', 'Agents']);
    expect(document.querySelector('#sg-workflow')).toHaveAttribute('data-anchor', 'workflow');
  });
});
