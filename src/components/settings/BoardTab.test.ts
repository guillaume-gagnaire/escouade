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
    settingsForm.open({ tab: 'board', projectId: 'p1' });
    setLang('en');
  });

  it('writes what validating a ticket does, and its options', async () => {
    render(BoardTab, { project: app.projects[0] });
    const actions = screen.getByRole('radiogroup', { name: 'When I approve a ticket in “To review”' });
    expect(within(actions).getByRole('radio', { name: /Merge into a branch/ })).toHaveAttribute('aria-checked', 'true');
    expect(within(actions).getByText('Pushes ticket/<key> and opens a PR to the target branch for review.')).toBeInTheDocument();
    expect(within(actions).getByRole('radio', { name: /Leave as is/ })).toBeInTheDocument();
    expect(screen.getByText('Target branch')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Squash' })).toHaveAttribute('aria-pressed', 'true');
    // Opening a pull request has its own option, not the strategy.
    await userEvent.click(within(actions).getByRole('radio', { name: /Open a pull request/ }));
    expect(screen.getByRole('switch', { name: 'Draft PR' })).toBeInTheDocument();
    expect(screen.queryByText('Strategy')).not.toBeInTheDocument();
  });

  it('writes the tests, the commit message and the conflicts', () => {
    render(BoardTab, { project: app.projects[0] });
    expect(screen.getByRole('textbox', { name: 'Test command' })).toHaveAttribute('placeholder', 'e.g. npm test');
    expect(screen.getByRole('switch', { name: 'Re-run the tests first' })).toBeDisabled();
    expect(screen.getByRole('switch', { name: 'Delete the worktree once approved' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Generated commit message' })).toBeInTheDocument();
    expect(screen.getByText('feat: limit login attempts [DEM-42]')).toBeInTheDocument();
    const conflicts = screen.getByRole('group', { name: 'On conflict' });
    expect(
      within(conflicts)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Ask me', 'Let the agent resolve', 'Cancel']);
  });

  it('writes the autopilot and the agents, the percentages as English does', () => {
    render(BoardTab, { project: app.projects[0] });
    expect(screen.getByRole('switch', { name: 'Assign tickets automatically' })).toBeInTheDocument();
    const quota = screen.getByRole('group', { name: 'Pause above quota' });
    expect(
      within(quota)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['80%', '90%', '95%', '100%']);
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
    expect(
      within(screen.getByRole('group', { name: 'Pause au-delà du quota' })).getByRole('button', { name: /^80\s%$/ }),
    ).toBeInTheDocument();
    expect(screen.getByText('feat: limiter les tentatives de connexion [DEM-42]')).toBeInTheDocument();
  });
});
