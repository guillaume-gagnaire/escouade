import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { agentGitOf, branchPrs, switchRefusal } from '../lib/agent-branch';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import type { AgentGit } from '../lib/types';
import { agent, branchInfo, fakeBackend, gitInfo, pullRequest as pr, resetApp } from '../test/ipc';
import StatusBar from './StatusBar.svelte';

const { openUrl } = vi.hoisted(() => ({ openUrl: vi.fn(() => Promise.resolve()) }));
vi.mock('../lib/opener', () => ({ openUrl }));

const WT = { path: 'C:\\wt\\login', branch: 'escouade/login', baseBranch: 'main', existing: false };

function own(over: Partial<AgentGit> = {}): AgentGit {
  return { branch: 'escouade/login', upstream: null, upstreamGone: false, ahead: 0, behind: 0, hasRemote: true, ...over };
}

describe('the branch of the agent open', () => {
  it('is its own folder’s, as git tells it; none for an agent in the project’s folder or archived', () => {
    const g = gitInfo({ agentGit: { a1: own({ branch: 'feat/review' }) } });
    expect(agentGitOf(agent({ worktree: WT }), g)?.branch).toBe('feat/review');
    expect(agentGitOf(agent({ id: 'a2' }), g)).toBeNull();
    expect(agentGitOf(agent({ worktree: WT, archived: true }), g)).toBeNull();
    expect(agentGitOf(null, g)).toBeNull();
  });

  it('is switched from the picker for an ordinary agent in a worktree of Escouade’s only', () => {
    expect(switchRefusal(agent({ worktree: WT }))).toBeNull();
    expect(switchRefusal(agent({ worktree: WT, ticketId: 't1' }))).toBe(
      'La branche d’un agent de ticket suit son ticket : elle ne change pas.',
    );
    expect(switchRefusal(agent({ worktree: { ...WT, repos: [{ repo: 'api', baseBranch: 'main' }] } }))).toMatch(/plusieurs dépôts/);
    expect(switchRefusal(agent({ foundWorktree: { path: 'C:/p/.claude/worktrees/a', branch: 'feat/a' } }))).toMatch(
      /pas un worktree d’Escouade/,
    );
  });

  it('has the pull requests of that branch, each repository’s for several', () => {
    const a = agent({ prs: [pr({ number: 1, head: 'feat/a' }), pr({ number: 2, head: 'escouade/login' })] });
    expect(branchPrs(a, 'escouade/login').map((p) => p.number)).toEqual([2]);
    const multi = agent({ prs: [pr({ number: 3, head: 'x', repo: 'api' })] });
    expect(branchPrs(multi, '').map((p) => p.number)).toEqual([3]);
  });
});

describe('StatusBar while an agent is open', () => {
  const button = (name: RegExp) => screen.getByRole('button', { name });
  const LIST = [
    branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api' }),
    branchInfo({ name: 'escouade/login', worktree: WT.path, agent: 'a1' }),
    branchInfo({ name: 'escouade/other', worktree: 'C:\\wt\\other', agent: 'a2' }),
    branchInfo({ name: 'feat/review' }),
  ];

  beforeEach(() => {
    resetApp({ agents: [agent({ worktree: WT }), agent({ id: 'a2', name: 'autre', worktree: { ...WT, branch: 'escouade/other' } })] });
    app.ui.selectedAgent = { p1: 'a1' };
    menu.close();
    openUrl.mockClear();
  });
  afterEach(() => setLang('fr'));

  it('shows the branch of its worktree rather than the project’s folder’s, and says whose it is', async () => {
    fakeBackend();
    app.git = {
      p1: gitInfo({ hasRemote: true, upstream: 'origin/main', agentGit: { a1: own({ upstream: 'origin/escouade/login', ahead: 2 }) } }),
    };
    render(StatusBar);
    const b = button(/^escouade\/login/);
    expect(b).toHaveAccessibleName('escouade/login 2 commits à pousser 0 commit à tirer');
    expect(b.dataset.tip?.split('\n')[0]).toBe('Branche de l’agent refacto-auth, dans son worktree');
    // The board over its conversation: the project's folder's.
    app.board = { p1: true };
    await tick();
    expect(button(/^main/)).toHaveAccessibleName('main 0 commit à pousser 0 commit à tirer');
    app.board = {};
    await tick();
    expect(button(/^escouade\/login/)).toBeInTheDocument();
    // An agent in the project's folder: the folder's, as before.
    app.ui.selectedAgent = { p1: 'a2' };
    app.git = { p1: gitInfo({ agentGit: { a1: own() } }) };
    await tick();
    expect(button(/^main/)).toHaveTextContent(/^main$/);
  });

  it('shows its pull request apart, which opens in the browser, and how many more it has', async () => {
    fakeBackend();
    app.agents.a1 = {
      ...app.agents.a1,
      prWord: 'PR',
      prs: [pr({ number: 12, checks: 'failure' }), pr({ number: 9, state: 'merged' }), pr({ number: 4, head: 'escouade/old' })],
    };
    app.git = { p1: gitInfo({ agentGit: { a1: own() } }) };
    render(StatusBar);
    const chip = button(/^Ouvrir la PR #12/);
    expect(chip).toHaveAccessibleName('Ouvrir la PR #12 dans le navigateur (ouverte)');
    expect(chip).toHaveTextContent('#12+1 autre');
    expect(chip).toHaveClass('fail');
    expect(chip.dataset.tip).toContain('PR #12 · Connexion par jeton');
    await userEvent.click(chip);
    expect(openUrl).toHaveBeenCalledWith('https://github.com/acme/demo/pull/7');
    // By Entrée too, and a refusal said with its reason (ESC-402).
    chip.focus();
    await userEvent.keyboard('{Enter}');
    expect(openUrl).toHaveBeenCalledTimes(2);
    openUrl.mockImplementationOnce(() => Promise.reject('aucun navigateur'));
    await userEvent.click(chip);
    await waitFor(() =>
      expect(app.toasts.at(-1)).toMatchObject({ text: 'La PR #12 ne s’ouvre pas dans le navigateur : aucun navigateur', kind: 'error' }),
    );
    // Another branch: none of the old one's.
    app.git = { p1: gitInfo({ agentGit: { a1: own({ branch: 'feat/review' }) } }) };
    await tick();
    expect(screen.queryByRole('button', { name: /^Ouvrir la PR/ })).toBeNull();
  });

  it('shows the pull request of the project’s folder’s branch for an agent working there', async () => {
    fakeBackend();
    app.ui.selectedAgent = { p1: 'a2' };
    app.agents.a2 = { ...app.agents.a2, worktree: null, prs: [pr({ number: 5, head: 'dev' })] };
    app.git = { p1: gitInfo({ branch: 'dev' }) };
    render(StatusBar);
    expect(button(/^Ouvrir la PR #5/)).toBeInTheDocument();
  });

  it('switches its worktree from the picker, never to the folder’s branch nor another agent’s', async () => {
    const backend = fakeBackend({ branch_list: () => LIST, agent_branch_switch: () => 'feat/review' });
    app.git = { p1: gitInfo({ agentGit: { a1: own() } }) };
    render(StatusBar);
    await userEvent.click(button(/^escouade\/login/));
    const dialog = await screen.findByRole('dialog', { name: 'Branche de refacto-auth' });
    const option = (name: string) => within(dialog).getByText(name, { selector: '.name' }).closest('[role="option"]')!;
    await waitFor(() => expect(option('feat/review')).toBeInTheDocument());
    expect(option('escouade/login')).toHaveAttribute('aria-current', 'true');
    expect(option('main')).toHaveAttribute('aria-disabled', 'true');
    expect(option('main').getAttribute('data-tip')).toBe('C’est la branche du dossier du projet : un worktree ne peut pas la prendre.');
    expect(option('escouade/other')).toHaveAttribute('aria-disabled', 'true');
    // Nothing of the project's folder's own: no new branch, no clean-up.
    expect(within(dialog).queryByRole('button', { name: 'Nouvelle branche…' })).toBeNull();
    await userEvent.click(option('main'));
    expect(backend.called('agent_branch_switch')).toHaveLength(0);
    await userEvent.click(option('feat/review'));
    await waitFor(() => expect(backend.called('agent_branch_switch')[0].args).toEqual({ id: 'a1', name: 'feat/review' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(app.toasts.at(-1)).toMatchObject({ text: 'refacto-auth passe sur feat/review.', kind: 'ok' });
    expect(backend.called('branch_switch')).toHaveLength(0);
  });

  it('never offers the board’s target, and opens the project’s folder’s picker on demand', async () => {
    const backend = fakeBackend({ branch_list: () => LIST });
    app.projects = [{ ...app.projects[0], board: { ...app.projects[0].board, target: 'feat/review' } }];
    app.git = { p1: gitInfo({ agentGit: { a1: own() } }) };
    render(StatusBar);
    await userEvent.click(button(/^escouade\/login/));
    const dialog = await screen.findByRole('dialog', { name: 'Branche de refacto-auth' });
    const review = (await within(dialog).findByText('feat/review')).closest('[role="option"]')!;
    expect(review).toHaveAttribute('aria-disabled', 'true');
    expect(review.getAttribute('data-tip')).toBe('C’est la branche cible du Kanban : un agent n’y travaille pas directement.');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Dossier du projet…' }));
    const folder = await screen.findByRole('dialog', { name: 'Branches' });
    expect(within(folder).getByRole('button', { name: 'Nouvelle branche…' })).toBeInTheDocument();
    expect(backend.called('branch_list')).toHaveLength(2);
  });

  it('says why the switch was refused, and keeps the picker open', async () => {
    fakeBackend({
      branch_list: () => LIST,
      agent_branch_switch: () => {
        throw 'L’agent refacto-auth travaille : attends la fin de son tour.';
      },
    });
    app.git = { p1: gitInfo({ agentGit: { a1: own() } }) };
    render(StatusBar);
    await userEvent.click(button(/^escouade\/login/));
    const dialog = await screen.findByRole('dialog', { name: 'Branche de refacto-auth' });
    await userEvent.click(await within(dialog).findByText('feat/review'));
    // Said in the picker, which stays open: a toast would cover its foot in a narrow window (ESC-238).
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('L’agent refacto-auth travaille : attends la fin de son tour.');
    expect(app.toasts).toEqual([]);
    expect(screen.getByRole('dialog', { name: 'Branche de refacto-auth' })).toBeInTheDocument();
    // Gone once the cursor moves to another branch.
    await userEvent.keyboard('{ArrowDown}');
    await waitFor(() => expect(within(dialog).queryByRole('alert')).toBeNull());
  });

  it('opens on the agent’s branch, not on a branch it can’t take (ESC-238)', async () => {
    fakeBackend({ branch_list: () => LIST });
    app.git = { p1: gitInfo({ agentGit: { a1: own() } }) };
    render(StatusBar);
    await userEvent.click(button(/^escouade\/login/));
    const dialog = await screen.findByRole('dialog', { name: 'Branche de refacto-auth' });
    const option = (name: string) => within(dialog).getByText(name, { selector: '.name' }).closest('[role="option"]')!;
    await waitFor(() => expect(option('escouade/login')).toHaveAttribute('aria-selected', 'true'));
    expect(option('main')).toHaveAttribute('aria-selected', 'false');
    expect(within(dialog).queryByText('C’est la branche du dossier du projet : un worktree ne peut pas la prendre.')).toBeNull();
    // A search puts its best match first, as before.
    await userEvent.type(within(dialog).getByRole('combobox'), 'review');
    await waitFor(() => expect(option('feat/review')).toHaveAttribute('aria-selected', 'true'));
  });

  it('syncs its worktree, not the project’s folder', async () => {
    const backend = fakeBackend({ branch_list: () => LIST, agent_git_sync: () => '1 commit poussé' });
    app.git = { p1: gitInfo({ agentGit: { a1: own({ upstream: 'origin/escouade/login', ahead: 1 }) } }) };
    render(StatusBar);
    await userEvent.click(button(/^escouade\/login/));
    const dialog = await screen.findByRole('dialog', { name: 'Branche de refacto-auth' });
    await userEvent.click(within(dialog).getByRole('button', { name: /Pousser/ }));
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['1 commit poussé']));
    expect(backend.called('agent_git_sync')[0].args).toEqual({ id: 'a1', op: 'push' });
    expect(backend.called('git_push')).toHaveLength(0);
  });

  it('lets a ticket’s agent sync but not switch', async () => {
    fakeBackend({ branch_list: () => LIST });
    app.agents.a1 = { ...app.agents.a1, ticketId: 't1' };
    app.git = { p1: gitInfo({ agentGit: { a1: own({ upstream: 'origin/escouade/login' }) } }) };
    render(StatusBar);
    await userEvent.click(button(/^escouade\/login/));
    const dialog = await screen.findByRole('dialog', { name: 'Branche de refacto-auth' });
    const review = (await within(dialog).findByText('feat/review')).closest('[role="option"]')!;
    expect(review).toHaveAttribute('aria-disabled', 'true');
    expect(review.getAttribute('data-tip')).toBe('La branche d’un agent de ticket suit son ticket : elle ne change pas.');
    expect(within(dialog).getByRole('button', { name: /Fetch/ })).toBeEnabled();
  });

  it('lists the branch of each repository of an agent of several, its switch not offered yet', async () => {
    fakeBackend();
    const repo = (name: string, branch: string) => ({
      repo: name,
      branch,
      upstream: null,
      ahead: 0,
      behind: 0,
      hasRemote: true,
      hasCommit: true,
      modified: 0,
      added: 0,
      deleted: 0,
      gone: false,
    });
    app.agents.a1 = {
      ...app.agents.a1,
      worktree: {
        ...WT,
        repos: [
          { repo: 'api', baseBranch: 'main' },
          { repo: 'front', baseBranch: 'main' },
        ],
      },
    };
    app.git = { p1: gitInfo({ agentGit: { a1: own({ repos: [repo('api', 'escouade/login'), repo('front', 'escouade/login')] }) } }) };
    render(StatusBar);
    const b = button(/^escouade\/login/);
    expect(b).toHaveTextContent('escouade/login· 2 dépôts');
    await userEvent.click(b);
    await waitFor(() => expect(menu.open).toBeTruthy());
    const items = menu.open!.items;
    expect(items.filter((i) => !i.heading).map((i) => [i.label, i.detail, i.disabled])).toEqual([
      ['api', 'escouade/login · à jour', true],
      ['front', 'escouade/login · à jour', true],
    ]);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('writes it in English', async () => {
    setLang('en');
    fakeBackend({ branch_list: () => LIST });
    app.agents.a1 = { ...app.agents.a1, prWord: 'PR', prs: [pr({ number: 12 })] };
    app.git = { p1: gitInfo({ agentGit: { a1: own() } }) };
    render(StatusBar);
    expect(button(/^escouade\/login/).dataset.tip?.split('\n')[0]).toBe('Branch of agent refacto-auth, in its worktree');
    expect(button(/^Open PR #12/)).toBeInTheDocument();
    await userEvent.click(button(/^escouade\/login/));
    await screen.findByRole('dialog', { name: 'Branch of refacto-auth' });
  });
});
