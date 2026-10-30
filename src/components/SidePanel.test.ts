import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
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

  it('keeps its count while the files are read again, back from another tab', async () => {
    let slow = false;
    fakeBackend({
      git_files: () =>
        slow
          ? new Promise((r) => setTimeout(() => r([change('src/a.ts'), change('src/b.ts')]), 400))
          : [change('src/a.ts'), change('src/b.ts')],
      git_log: () => LOG,
    });
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    await screen.findByRole('tab', { name: /Non commités\s*2/ });
    slow = true;
    await userEvent.click(screen.getByRole('tab', { name: 'Historique' }));
    await userEvent.click(screen.getByRole('tab', { name: /Non commités/ }));
    expect(screen.getByRole('tab', { name: /Non commités/ })).toHaveAccessibleName(/Non commités\s*2/);
  });

  it('counts every uncommitted file in its tab, whatever the list shows', async () => {
    fakeBackend({ git_files: () => Array.from({ length: 620 }, (_, i) => change(`src/f${i}.ts`)), git_log: () => LOG });
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    expect(await screen.findByRole('tab', { name: /Non commités\s*620/ })).toBeInTheDocument();
  });

  it('can be closed in the classic layout, where it is optional, and says how it comes back', async () => {
    fakeBackend({ git_files: () => [] });
    app.filesOpen = true;
    render(SidePanel, { project: project(), agent: app.agents.a1 });
    const close = screen.getByRole('button', { name: 'Fermer le panneau' });
    expect(close).toHaveAttribute(
      'data-tip',
      'Fermer le panneau (Ctrl+Maj+B)\nPour le rouvrir : « Fichiers » dans l’en-tête de la conversation',
    );
    await userEvent.click(close);
    expect(app.filesOpen).toBe(false);
  });

  it('opens « PR et CI » on the agent’s pull request, named by its host’s word (ESC-305)', async () => {
    fakeBackend({
      git_files: () => [],
      git_log: () => LOG,
      agent_no_pr: () => ({ reason: 'Cet agent n’a pas de branche à lui.', settings: false }),
    });
    app.agents.a1.prWord = 'MR';
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    const hosts = screen.getByRole('tab', { name: 'MR et CI' });
    await userEvent.click(hosts);
    expect(hosts).toHaveAttribute('aria-selected', 'true');
    expect(app.panelTab).toBe('hosts');
    expect(screen.getByRole('tabpanel')).toHaveTextContent('Pas de MR sur cette branche');
    expect(screen.queryByText(/dans la 2.1/)).toBeNull();
  });

  it('draws its close button with an icon, not a character', async () => {
    fakeBackend({ git_files: () => [] });
    app.filesOpen = true;
    render(SidePanel, { project: project(), agent: app.agents.a1 });
    const close = screen.getByRole('button', { name: 'Fermer le panneau' });
    expect(close.querySelector('svg[data-icon="close"]')).not.toBeNull();
    expect(close.textContent?.trim()).toBe('');
  });

  it('is closed in the split layout by going back to the classic one (ESC-204), kept as the layout is', async () => {
    const backend = fakeBackend({ git_files: () => [] });
    app.ui.layout = 'split';
    // « Fichiers » opened before the layout went side by side: the classic layout comes back without the panel.
    app.filesOpen = true;
    render(SidePanel, { project: project(), agent: app.agents.a1, docked: true });
    const close = screen.getByRole('button', { name: 'Fermer le panneau' });
    expect(close).toHaveAttribute(
      'data-tip',
      'Fermer le panneau (Ctrl+Maj+L)\nPour le rouvrir : la disposition côte à côte, en haut à droite de la conversation',
    );
    await userEvent.click(close);
    expect(app.split).toBe(false);
    expect(app.filesOpen).toBe(false);
    await waitFor(() => expect(backend.called('set_ui').at(-1)?.args.ui.layout).toBe(''));
  });
});

describe('SidePanel in English', () => {
  beforeEach(() => {
    resetApp({ projects: [project()], agents: [agent()] });
    setLang('en');
  });

  it('names its two tabs and its close button in English', async () => {
    fakeBackend({ git_files: () => [change('src/a.ts'), change('src/b.ts')], git_log: () => LOG });
    render(SidePanel, { project: project(), agent: app.agents.a1 });
    expect(await screen.findByRole('tab', { name: /Uncommitted\s*2/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: 'History' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close the panel' })).toHaveAttribute(
      'data-tip',
      'Close the panel (Ctrl+Shift+B)\nTo open it again: “Files” in the header of the conversation',
    );
  });
});
