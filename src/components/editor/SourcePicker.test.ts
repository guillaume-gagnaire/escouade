import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { agent, gitInfo, project } from '../../test/ipc';
import SourcePicker from './SourcePicker.svelte';

describe('SourcePicker', () => {
  const wt = agent({
    id: 'a2',
    name: 'refacto',
    worktree: { path: 'C:\\code\\demo-api\\.claude\\worktrees\\refacto', branch: 'escouade/refacto', baseBranch: 'main' },
  });

  it('offers the project branch and each worktree, with their changes', async () => {
    const onpick = vi.fn();
    render(SourcePicker, {
      project: project(),
      source: 'project',
      agents: [wt],
      git: gitInfo({ modified: 6, added: 1, agents: { a2: 4 } }),
      onpick,
    });
    await userEvent.click(screen.getByRole('button', { name: /Source : main/ }));
    expect(screen.getByRole('menuitemradio', { name: /main/ })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('menuitemradio', { name: /main/ })).toHaveTextContent('Δ 3');
    expect(screen.getByRole('menuitemradio', { name: /refacto/ })).toHaveTextContent('4 modif.');
    await userEvent.click(screen.getByRole('menuitemradio', { name: /refacto/ }));
    expect(onpick).toHaveBeenCalledWith('a2');
    expect(screen.queryByRole('menuitemradio')).not.toBeInTheDocument();
  });

  it('does not count the worktrees’ changes on the project branch', async () => {
    render(SourcePicker, {
      project: project(),
      source: 'project',
      agents: [wt],
      git: gitInfo({ modified: 4, agents: { a2: 4 } }),
      onpick: () => {},
    });
    await userEvent.click(screen.getByRole('button', { name: /Source : main/ }));
    expect(screen.getByRole('menuitemradio', { name: /main/ })).toHaveTextContent('propre');
    expect(screen.getByRole('menuitemradio', { name: /main/ })).not.toHaveTextContent('Δ');
    expect(screen.getByRole('menuitemradio', { name: /refacto/ })).toHaveTextContent('4 modif.');
  });

  it('shows the real folder of a worktree, not the agent’s name', async () => {
    const renamed = agent({
      id: 'a3',
      name: 'nouveau-nom',
      model: 'opus',
      worktree: {
        path: 'C:\\code\\demo-api\\.claude\\worktrees\\dossier-initial',
        branch: 'escouade/dossier-initial',
        baseBranch: 'main',
      },
    });
    render(SourcePicker, { project: project(), source: 'project', agents: [renamed], git: gitInfo(), onpick: () => {} });
    await userEvent.click(screen.getByRole('button', { name: /Source : main/ }));
    const option = screen.getByRole('menuitemradio', { name: /nouveau-nom/ });
    expect(option).toHaveTextContent('.claude/worktrees/dossier-initial · opus');
    expect(option).not.toHaveTextContent('worktrees/nouveau-nom');
  });
});

describe('SourcePicker in English', () => {
  it('says the source, the branch and the changes of each place in English', async () => {
    setLang('en');
    const wt = agent({
      id: 'a2',
      name: 'refacto',
      worktree: { path: 'C:\\code\\demo-api\\.claude\\worktrees\\refacto', branch: 'escouade/refacto', baseBranch: 'main' },
    });
    render(SourcePicker, {
      project: project(),
      source: 'project',
      agents: [wt, agent({ id: 'a3', name: 'tidy' })],
      git: gitInfo({ modified: 6, added: 1, agents: { a2: 4 } }),
      onpick: () => {},
    });
    const button = screen.getByRole('button', { name: /Source: main/ });
    expect(button).toHaveTextContent('branch');
    await userEvent.click(button);
    expect(screen.getByRole('menu', { name: 'Browse' })).toBeInTheDocument();
    expect(screen.getByRole('menuitemradio', { name: /main/ })).toHaveTextContent('Project branch · ');
    expect(screen.getByRole('menuitemradio', { name: /main/ })).toHaveTextContent('Δ 3');
    expect(screen.getByRole('menuitemradio', { name: /refacto/ })).toHaveTextContent('4 changes');
    expect(screen.getByRole('menuitemradio', { name: /tidy/ })).toHaveTextContent('clean');
  });
});
