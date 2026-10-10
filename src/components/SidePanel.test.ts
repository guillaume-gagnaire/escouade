import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import SidePanel from './SidePanel.svelte';

const change = (path: string) => ({ path, status: 'M', add: 1, del: 0, agentId: 'a1' });
const LOG = {
  commits: [{ hash: 'aaaa1111', parents: [], author: 'Ada', time: 1790000000, refs: ['HEAD', 'main'], subject: 'premier commit' }],
  head: 'main',
};

describe('SidePanel', () => {
  beforeEach(() => resetApp({ projects: [project()], agents: [agent()] }));

  it('switches between the uncommitted files and the history', async () => {
    fakeBackend({ git_files: () => [change('src/a.ts'), change('src/b.ts')], git_log: () => LOG });
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(await screen.findByRole('tab', { name: /Non commités\s*2/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByText('a.ts')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('tab', { name: 'Historique' }));
    expect(await screen.findByText('premier commit')).toBeInTheDocument();
    expect(screen.queryByText('a.ts')).not.toBeInTheDocument();
    expect(app.panelTab).toBe('history');
  });

  it('counts every uncommitted file in its tab, whatever the list shows', async () => {
    fakeBackend({ git_files: () => Array.from({ length: 620 }, (_, i) => change(`src/f${i}.ts`)), git_log: () => LOG });
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(await screen.findByRole('tab', { name: /Non commités\s*620/ })).toBeInTheDocument();
  });

  it('can be closed in the classic layout, where it is optional', async () => {
    fakeBackend({ git_files: () => [] });
    app.filesOpen = true;
    render(SidePanel, { project: project(), agent: app.agents.a1 });
    await userEvent.click(screen.getByTitle('Fermer'));
    expect(app.filesOpen).toBe(false);
  });

  it('cannot be closed in the split layout', () => {
    fakeBackend({ git_files: () => [] });
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(screen.queryByTitle('Fermer')).not.toBeInTheDocument();
  });
});
