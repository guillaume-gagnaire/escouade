import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { Commit, GitLog } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import GitGraph from './GitGraph.svelte';

const c = (hash: string, subject: string, parents: string[] = [], refs: string[] = []): Commit => ({
  hash,
  parents,
  author: 'Ada',
  time: 1790000000,
  refs,
  subject,
});

const LOG: GitLog = {
  commits: [
    c('aaaa1111', 'Merge landing', ['bbbb2222', 'cccc3333'], ['HEAD', 'main', 'tag: v0.1.0']),
    c('cccc3333', 'nouvelle page', ['dddd4444'], ['ccm/landing']),
    c('bbbb2222', 'fix du header', ['dddd4444']),
    c('dddd4444', 'init'),
  ],
  head: 'ccm/landing',
};

const settle = () => new Promise((r) => setTimeout(r, 200));
const row = (subject: string) => screen.getByText(subject).closest('button')!;

describe('GitGraph', () => {
  beforeEach(() =>
    resetApp({
      projects: [project()],
      agents: [agent(), agent({ id: 'a2', name: 'landing', worktree: { path: 'C:/wt', branch: 'ccm/landing', baseBranch: 'main' } })],
    }),
  );

  it('lists every branch’s commits, naming agent branches after their agent', async () => {
    const backend = fakeBackend({ git_log: () => LOG });
    render(GitGraph, { project: project(), agent: app.agents.a2 });
    expect(await screen.findByText('nouvelle page')).toBeInTheDocument();
    expect(backend.called('git_log')[0].args).toEqual({ projectId: 'p1', agentId: 'a2' });
    expect(within(row('nouvelle page')).getByText('landing')).toBeInTheDocument();
    expect(within(row('Merge landing')).getByText('main')).toBeInTheDocument();
    expect(within(row('Merge landing')).getByText('v0.1.0')).toBeInTheDocument();
    expect(within(row('Merge landing')).queryByText('HEAD')).not.toBeInTheDocument();
  });

  it('highlights the agent’s branch', async () => {
    fakeBackend({ git_log: () => LOG });
    render(GitGraph, { project: project(), agent: app.agents.a2 });
    await screen.findByText('nouvelle page');
    expect(row('nouvelle page')).toHaveAttribute('data-mine', 'true');
    expect(row('init')).toHaveAttribute('data-mine', 'true');
    expect(row('fix du header')).toHaveAttribute('data-mine', 'false');
    expect(row('Merge landing')).toHaveAttribute('data-mine', 'false');
  });

  it('opens the diff of a clicked commit', async () => {
    fakeBackend({ git_log: () => LOG });
    render(GitGraph, { project: project(), agent: app.agents.a2 });
    await userEvent.click(await screen.findByText('fix du header'));
    expect(app.modal).toEqual({
      kind: 'diff',
      projectId: 'p1',
      agentId: null,
      paths: [],
      title: 'bbbb222 · fix du header',
      commit: 'bbbb2222',
    });
  });

  it('refreshes when the repository changes', async () => {
    let log = LOG;
    const backend = fakeBackend({ git_log: () => log });
    render(GitGraph, { project: project(), agent: app.agents.a2 });
    await screen.findByText('nouvelle page');
    log = { ...LOG, commits: [c('eeee5555', 'encore un commit', ['aaaa1111'], ['main']), ...LOG.commits] };
    app.gitTick++;
    expect(await screen.findByText('encore un commit')).toBeInTheDocument();
    expect(backend.called('git_log').length).toBe(2);
  });

  it('says why the history cannot be shown', async () => {
    fakeBackend({
      git_log: () => {
        throw new Error('pas un dépôt git');
      },
    });
    render(GitGraph, { project: project(), agent: app.agents.a1 });
    await settle();
    expect(screen.getByText(/pas un dépôt git/)).toBeInTheDocument();
  });
});

describe('GitGraph in English', () => {
  beforeEach(() => {
    resetApp({
      projects: [project()],
      agents: [agent(), agent({ id: 'a2', name: 'landing', worktree: { path: 'C:/wt', branch: 'ccm/landing', baseBranch: 'main' } })],
    });
    setLang('en');
  });

  it('titles the labels of the tags and of the agents’ branches, and the age of a commit, in English', async () => {
    app.now = 1790000000 * 1000 + 5 * 60_000;
    fakeBackend({ git_log: () => LOG });
    render(GitGraph, { project: project(), agent: app.agents.a2 });
    expect(await screen.findByText('Merge landing')).toBeInTheDocument();
    expect(screen.getByText('v0.1.0')).toHaveAttribute('title', 'tag v0.1.0');
    expect(screen.getByText('landing')).toHaveAttribute('title', 'branch ccm/landing of agent landing');
    expect(within(row('Merge landing')).getByText('5 min ago')).toBeInTheDocument();
  });

  it('says there is nothing, in English', async () => {
    fakeBackend({ git_log: () => ({ commits: [], head: null }) });
    render(GitGraph, { project: project(), agent: null });
    expect(await screen.findByText('No commits.')).toBeInTheDocument();
  });
});
