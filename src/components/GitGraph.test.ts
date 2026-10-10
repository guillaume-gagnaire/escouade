import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import type { Commit, GitLog } from '../lib/types';
import { agent, branchInfo, fakeBackend, project, resetApp } from '../test/ipc';
import ContextMenu from './ContextMenu.svelte';
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

describe('GitGraph menu', () => {
  const MENU_LOG: GitLog = {
    commits: [
      c('aaaa1111', 'Merge landing', ['bbbb2222', 'cccc3333'], ['HEAD', 'main', 'origin/main', 'tag: v0.1.0']),
      c('cccc3333', 'nouvelle page', ['eeee5555'], ['ccm/landing']),
      c('bbbb2222', 'fix du header', ['eeee5555'], ['feat/login', 'origin/feat/login', 'hotfix']),
      c('eeee5555', 'sans branche', ['ffff6666']),
      c('ffff6666', 'une seule branche', ['aabb7777'], ['release']),
      c('aabb7777', 'distante seule', ['dddd4444'], ['origin/only']),
      c('dddd4444', 'init', [], ['tag: v0.0.1']),
    ],
    head: 'main',
  };
  const BRANCHES = [
    branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api', upstream: 'origin/main' }),
    branchInfo({ name: 'feat/login', upstream: 'origin/feat/login' }),
    branchInfo({ name: 'hotfix' }),
    branchInfo({ name: 'release' }),
    branchInfo({ name: 'ccm/landing', worktree: 'C:/wt', agent: 'a2' }),
    branchInfo({ name: 'origin/main', remote: true, trackedBy: 'main' }),
    branchInfo({ name: 'origin/feat/login', remote: true, trackedBy: 'feat/login' }),
    branchInfo({ name: 'origin/only', remote: true }),
  ];

  beforeEach(() => {
    resetApp({
      projects: [project()],
      agents: [agent(), agent({ id: 'a2', name: 'landing', worktree: { path: 'C:/wt', branch: 'ccm/landing', baseBranch: 'main' } })],
    });
    menu.close();
  });

  let view: ReturnType<typeof render>;
  /** The graph on `MENU_LOG`, the repository having `branches` (an Error: they cannot be read). */
  async function setup(branches: unknown = BRANCHES, handlers: Record<string, (args: any) => unknown> = {}) {
    const backend = fakeBackend({
      git_log: () => MENU_LOG,
      branch_list: () => {
        if (branches instanceof Error) throw branches;
        return branches;
      },
      ...handlers,
    });
    view = render(GitGraph, { project: project(), agent: app.agents.a2 });
    // The branches come with the history: the menu has them when the rows do.
    await screen.findByText('init');
    await waitFor(() => expect(backend.called('branch_list').length).toBeGreaterThan(0));
    return backend;
  }
  /** The repository with its HEAD detached: no branch is the folder's. */
  const DETACHED = BRANCHES.map((b) => (b.current ? { ...b, current: false, worktree: null } : b));
  /** What the open menu says, a separator being « — ». */
  const entries = () => menu.open!.items.map((i) => (i.separator ? '—' : i.label));
  const entry = (label: string, nth = 0) => menu.open!.items.filter((i) => i.label === label)[nth];
  const rightClick = (subject: string, at = {}) => fireEvent.contextMenu(row(subject), at);

  describe('what it offers', () => {
    it('reads the branches along with the history, and again when the repository changes', async () => {
      const backend = await setup();
      expect(backend.called('branch_list')[0].args).toEqual({ projectId: 'p1' });
      const before = backend.called('branch_list').length;
      app.gitTick++;
      await waitFor(() => expect(backend.called('branch_list').length).toBe(before + 1));
    });

    it('offers, on a commit no branch is on, to make one there and to compare it with the current branch', async () => {
      await setup();
      await rightClick('sans branche');
      expect(entries()).toEqual(['Créer une branche ici…', '—', 'Comparer avec la branche courante']);
    });

    it('offers for each local branch of the commit to switch to it, and to delete it, the delete entries naming theirs when there are several', async () => {
      await setup();
      await rightClick('fix du header');
      expect(entries()).toEqual([
        'Créer une branche ici…',
        '—',
        'Passer sur feat/login',
        'Passer sur hotfix',
        'Comparer avec la branche courante',
        '—',
        'Supprimer la branche…',
        'Supprimer la branche…',
      ]);
      // The remote branch of the commit gets neither: nothing is deleted from the remote repository from here.
      expect(menu.open!.items.filter((i) => i.danger).map((i) => [i.label, i.hint])).toEqual([
        ['Supprimer la branche…', 'feat/login'],
        ['Supprimer la branche…', 'hotfix'],
      ]);
    });

    it('leaves the name out of the delete entry of a commit that has one branch', async () => {
      await setup();
      await rightClick('une seule branche');
      expect(entries()).toEqual([
        'Créer une branche ici…',
        '—',
        'Passer sur release',
        'Comparer avec la branche courante',
        '—',
        'Supprimer la branche…',
      ]);
      expect(entry('Supprimer la branche…').hint).toBeUndefined();
      expect(entry('Supprimer la branche…').danger).toBe(true);
    });

    it('offers nothing but a new branch on the commit of the current branch: no switching, no deleting, no comparing with itself', async () => {
      await setup();
      await rightClick('Merge landing');
      expect(entries()).toEqual(['Créer une branche ici…']);
    });

    it('compares a commit that only a remote branch is on, and neither switches to it nor deletes it', async () => {
      await setup();
      await rightClick('distante seule');
      expect(entries()).toEqual(['Créer une branche ici…', '—', 'Comparer avec la branche courante']);
    });

    it('says why a branch an agent’s worktree holds can be neither switched to nor deleted', async () => {
      await setup();
      await rightClick('nouvelle page');
      expect(entries()).toEqual([
        'Créer une branche ici…',
        '—',
        'Passer sur ccm/landing',
        'Comparer avec la branche courante',
        '—',
        'Supprimer la branche…',
      ]);
      const why = 'La branche « ccm/landing » est utilisée par l’agent landing, dans son worktree.';
      expect(entry('Passer sur ccm/landing')).toMatchObject({ disabled: true, title: why });
      expect(entry('Supprimer la branche…')).toMatchObject({ disabled: true, title: why, danger: true });
      expect(entry('Comparer avec la branche courante').disabled).toBeFalsy();
    });

    it('says it too of a branch that a worktree of no agent holds', async () => {
      await setup([...BRANCHES.filter((b) => b.name !== 'release'), branchInfo({ name: 'release', worktree: 'D:/elsewhere' })]);
      await rightClick('une seule branche');
      expect(entry('Passer sur release')).toMatchObject({
        disabled: true,
        title: 'La branche « release » est prise par le worktree D:/elsewhere.',
      });
    });

    it('enables what nothing holds, without a reason', async () => {
      await setup();
      await rightClick('fix du header');
      expect(entry('Passer sur hotfix').disabled).toBeFalsy();
      expect(entry('Passer sur hotfix').title).toBeUndefined();
      expect(entry('Supprimer la branche…').disabled).toBeFalsy();
    });

    it('compares with HEAD, and says so, when the folder is on no branch', async () => {
      await setup(DETACHED);
      await rightClick('sans branche');
      expect(entries()).toEqual(['Créer une branche ici…', '—', 'Comparer avec HEAD']);
      // HEAD's own commit has nothing to be compared with, though `main` there can be switched to.
      menu.close();
      await rightClick('Merge landing');
      expect(entries()).toEqual(['Créer une branche ici…', '—', 'Passer sur main', '—', 'Supprimer la branche…']);
    });

    it('still offers to make a branch and to compare when the branches cannot be read', async () => {
      await setup(new Error('boom'));
      expect(screen.getByText('init')).toBeInTheDocument();
      await rightClick('fix du header');
      expect(entries()).toEqual(['Créer une branche ici…', '—', 'Comparer avec HEAD']);
    });

    it('keeps the last list it read when a later read fails, but not the list of another project', async () => {
      let reads = 0;
      const backend = await setup(BRANCHES, {
        branch_list: () => {
          if (reads++) throw new Error('transient');
          return BRANCHES;
        },
      });
      app.gitTick++;
      await waitFor(() => expect(backend.called('branch_list').length).toBe(2));
      await waitFor(() => expect(backend.called('git_log').length).toBe(2));
      await settle();
      await rightClick('fix du header');
      expect(entries()).toContain('Passer sur hotfix');
      // Another project: what was read for the first says nothing of it.
      menu.close();
      const other = project({ id: 'p2', name: 'other' });
      app.projects.push(other);
      view.rerender({ project: other, agent: null });
      await waitFor(() => expect(backend.called('branch_list').length).toBe(3));
      await settle();
      await rightClick('fix du header');
      expect(entries()).toEqual(['Créer une branche ici…', '—', 'Comparer avec HEAD']);
    });

    it('does as much when the backend has no list to give', async () => {
      await setup(null);
      await rightClick('fix du header');
      expect(entries()).toEqual(['Créer une branche ici…', '—', 'Comparer avec HEAD']);
    });
  });

  describe('what it does', () => {
    it('opens the window of a new branch that starts at the commit', async () => {
      await setup();
      await rightClick('fix du header');
      entry('Créer une branche ici…').onClick!();
      expect(app.modal).toEqual({ kind: 'newBranch', projectId: 'p1', start: 'bbbb2222' });
    });

    it('switches the project’s folder to the branch', async () => {
      const backend = await setup(BRANCHES, { branch_switch: () => null });
      await rightClick('fix du header');
      entry('Passer sur hotfix').onClick!();
      await waitFor(() => expect(backend.called('branch_switch')).toHaveLength(1));
      expect(backend.called('branch_switch')[0].args).toEqual({ projectId: 'p1', name: 'hotfix', stash: false });
      expect(app.modal).toBeNull();
    });

    it('asks before putting uncommitted changes aside, and tells any other refusal', async () => {
      let refusal = 'DIRTY';
      await setup(BRANCHES, {
        branch_switch: () => {
          throw refusal;
        },
      });
      await rightClick('fix du header');
      entry('Passer sur hotfix').onClick!();
      await waitFor(() => expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Changer pour « hotfix » ?' }));
      app.modal = null;
      refusal = 'AGENT_WORKING:a1:refacto-auth';
      entry('Passer sur feat/login').onClick!();
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error' }));
      expect(app.toasts.at(-1)!.text).toContain('L’agent refacto-auth travaille');
    });

    it('asks before deleting the branch', async () => {
      await setup();
      await rightClick('fix du header');
      entry('Supprimer la branche…', 1).onClick!();
      expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Supprimer la branche « hotfix » ?', confirm: 'Supprimer' });
    });

    it('opens the diff window on what differs between the current branch and the branch of the commit', async () => {
      await setup();
      await rightClick('fix du header');
      entry('Comparer avec la branche courante').onClick!();
      expect(app.modal).toEqual({
        kind: 'diff',
        projectId: 'p1',
        agentId: null,
        paths: [],
        title: 'main ↔ feat/login',
        refs: { from: 'main', to: 'feat/login' },
      });
    });

    it('compares with the commit when no branch is on it, titled with its short hash', async () => {
      await setup();
      await rightClick('sans branche');
      entry('Comparer avec la branche courante').onClick!();
      expect(app.modal).toEqual({
        kind: 'diff',
        projectId: 'p1',
        agentId: null,
        paths: [],
        title: 'main ↔ eeee555',
        refs: { from: 'main', to: 'eeee5555' },
      });
    });

    it('compares with the commit that only a tag is on', async () => {
      await setup();
      await rightClick('init');
      entry('Comparer avec la branche courante').onClick!();
      expect(app.modal).toMatchObject({ title: 'main ↔ dddd444', refs: { from: 'main', to: 'dddd4444' } });
    });

    it('compares with a remote branch when only one is on the commit', async () => {
      await setup();
      await rightClick('distante seule');
      entry('Comparer avec la branche courante').onClick!();
      expect(app.modal).toMatchObject({ title: 'main ↔ origin/only', refs: { from: 'main', to: 'origin/only' } });
    });

    it('compares HEAD with the commit when the folder is on no branch', async () => {
      await setup(DETACHED);
      await rightClick('fix du header');
      entry('Comparer avec HEAD').onClick!();
      expect(app.modal).toMatchObject({ title: 'HEAD ↔ feat/login', refs: { from: 'HEAD', to: 'feat/login' } });
    });
  });

  describe('how it opens', () => {
    it('opens on the right click, where the pointer is, and not the diff of the commit', async () => {
      await setup();
      const notPrevented = await rightClick('fix du header', { clientX: 120, clientY: 80 });
      expect(notPrevented).toBe(false);
      expect(menu.open).toMatchObject({ x: 120, y: 80 });
      expect(menu.open!.keyboard).toBeFalsy();
      expect(app.modal).toBeNull();
    });

    it('opens on the menu key, and on Shift+F10, under the row that has the focus', async () => {
      await setup();
      const button = row('fix du header');
      button.focus();
      await fireEvent.keyDown(button, { key: 'ContextMenu' });
      expect(menu.open!.keyboard).toBe(true);
      expect(entries()[0]).toBe('Créer une branche ici…');
      menu.close();
      await userEvent.keyboard('{Shift>}{F10}{/Shift}');
      expect(menu.open!.keyboard).toBe(true);
      expect(entries()).toContain('Passer sur hotfix');
      expect(app.modal).toBeNull();
    });

    it('does not open on F10 alone, nor on another key', async () => {
      await setup();
      row('fix du header').focus();
      await userEvent.keyboard('{F10}a{Shift>}b{/Shift}');
      expect(menu.open).toBeNull();
    });

    it('is worked by the keyboard from end to end: the arrows, Enter, and the focus given back to the row', async () => {
      const backend = await setup(BRANCHES, { branch_switch: () => null });
      render(ContextMenu);
      const button = row('fix du header');
      button.focus();
      await userEvent.keyboard('{Shift>}{F10}{/Shift}');
      expect(await screen.findByRole('menuitem', { name: 'Créer une branche ici…' })).toHaveFocus();
      await userEvent.keyboard('{ArrowDown}');
      expect(screen.getByRole('menuitem', { name: 'Passer sur feat/login' })).toHaveFocus();
      await userEvent.keyboard('{ArrowDown}{Enter}');
      await waitFor(() => expect(backend.called('branch_switch')).toHaveLength(1));
      expect(backend.called('branch_switch')[0].args).toEqual({ projectId: 'p1', name: 'hotfix', stash: false });
      await waitFor(() => expect(button).toHaveFocus());
      expect(menu.open).toBeNull();
    });

    it('leaves a click on the row to the diff of the commit', async () => {
      await setup();
      await userEvent.click(screen.getByText('fix du header'));
      expect(menu.open).toBeNull();
      expect(app.modal).toMatchObject({ kind: 'diff', commit: 'bbbb2222' });
    });
  });

  describe('in English', () => {
    beforeEach(() => setLang('en'));

    it('writes the entries, and the title of the comparison, in English', async () => {
      await setup();
      await rightClick('fix du header');
      expect(entries()).toEqual([
        'Create a branch here…',
        '—',
        'Switch to feat/login',
        'Switch to hotfix',
        'Compare with the current branch',
        '—',
        'Delete branch…',
        'Delete branch…',
      ]);
      entry('Compare with the current branch').onClick!();
      expect(app.modal).toMatchObject({ title: 'main ↔ feat/login' });
      menu.close();
      await rightClick('nouvelle page');
      expect(entry('Switch to ccm/landing').title).toBe('The branch “ccm/landing” is used by agent landing, in its worktree.');
    });

    it('says it compares with HEAD when the folder is on no branch', async () => {
      await setup(DETACHED);
      await rightClick('sans branche');
      expect(entries()).toEqual(['Create a branch here…', '—', 'Compare with HEAD']);
    });
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
