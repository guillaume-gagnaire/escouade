import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import type { BranchInfo } from '../../lib/types';
import { agent, board, branchInfo, fakeBackend, gitInfo, project, resetApp } from '../../test/ipc';
import BranchPicker from './BranchPicker.svelte';

const MAIN = branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api', upstream: 'origin/main', ahead: 2, behind: 5 });
const FEAT = branchInfo({ name: 'feat/login', upstream: 'origin/feat/login' });
const HELD = branchInfo({
  name: 'escouade/refacto-auth',
  worktree: 'C:\\code\\demo-api\\.claude\\worktrees\\a1',
  agent: 'a1',
});
const REMOTE_FEAT = branchInfo({ name: 'origin/feat/login', remote: true, trackedBy: 'feat/login' });
const REMOTE_HOTFIX = branchInfo({ name: 'origin/hotfix', remote: true });
const LIST: BranchInfo[] = [MAIN, FEAT, HELD, REMOTE_FEAT, REMOTE_HOTFIX];

const tracked = { upstream: 'origin/main', hasRemote: true, ahead: 2, behind: 5 };

/** The picker over a project whose branches the fake backend lists. */
function setup(over: { list?: BranchInfo[]; handlers?: Record<string, (args: any) => unknown>; git?: Parameters<typeof gitInfo>[0] } = {}) {
  const backend = fakeBackend({ branch_list: () => over.list ?? LIST, ...over.handlers });
  app.git = { p1: gitInfo(over.git ?? tracked) };
  const onclose = vi.fn();
  render(BranchPicker, { projectId: 'p1', onclose });
  return { backend, onclose, field: screen.getByRole('combobox') };
}

const names = () => screen.queryAllByRole('option').map((o) => o.querySelector('.name')?.textContent);
const loaded = (count = LIST.length) => waitFor(() => expect(names()).toHaveLength(count));
const option = (name: string) => screen.getAllByRole('option').find((o) => o.querySelector('.name')?.textContent === name)!;
const cursor = () => screen.getAllByRole('option').findIndex((o) => o.getAttribute('aria-selected') === 'true');

describe('BranchPicker', () => {
  beforeEach(() => {
    resetApp({ agents: [agent({ id: 'a1', name: 'refacto-auth' })] });
    menu.close();
  });

  it('opens on its search field, the local branches first with the current one on top, then the remote ones', async () => {
    const { backend, field } = setup();
    expect(screen.getByRole('dialog', { name: 'Branches' })).toBeInTheDocument();
    expect(field).toHaveFocus();
    await loaded();
    expect(backend.called('branch_list')[0].args).toEqual({ projectId: 'p1' });
    const local = within(screen.getByRole('group', { name: 'Locales' }));
    expect(local.getAllByRole('option').map((o) => o.querySelector('.name')?.textContent)).toEqual([
      'main',
      'feat/login',
      'escouade/refacto-auth',
    ]);
    const remote = within(screen.getByRole('group', { name: 'Distantes' }));
    expect(remote.getAllByRole('option').map((o) => o.querySelector('.name')?.textContent)).toEqual(['origin/feat/login', 'origin/hotfix']);
  });

  it('ticks the current branch and gives each its commits ahead and behind its remote branch', async () => {
    setup();
    await loaded();
    expect(option('main')).toHaveAttribute('aria-current', 'true');
    expect(option('main').querySelector('.chk')).toHaveTextContent('✓');
    expect(option('main')).toHaveTextContent('↑2');
    expect(option('main')).toHaveTextContent('↓5');
    expect(option('main')).toHaveTextContent('2 commits à pousser');
    expect(option('main')).toHaveTextContent('5 commits à tirer');
    expect(option('feat/login')).not.toHaveAttribute('aria-current');
    expect(option('feat/login').querySelector('.chk')!.textContent).toBe('');
    expect(option('feat/login')).not.toHaveTextContent('↑');
    expect(option('feat/login')).not.toHaveTextContent('↓');
  });

  it('says a branch whose remote branch was deleted, and a remote branch a local one tracks', async () => {
    setup({ list: [MAIN, branchInfo({ name: 'old', upstream: 'origin/old', upstreamGone: true }), REMOTE_FEAT] });
    await loaded(3);
    expect(option('old')).toHaveTextContent('distante supprimée');
    expect(option('origin/feat/login')).toHaveTextContent('suivie par feat/login');
  });

  it('finds a branch from what is typed, even from letters scattered through its name', async () => {
    const { field } = setup();
    await loaded();
    await userEvent.type(field, 'login');
    expect(names()).toEqual(['feat/login', 'origin/feat/login']);
    await userEvent.clear(field);
    await userEvent.type(field, 'flg');
    expect(names()).toEqual(['feat/login', 'origin/feat/login']);
    await userEvent.clear(field);
    await userEvent.type(field, 'hotfix');
    expect(names()).toEqual(['origin/hotfix']);
    expect(screen.queryByRole('group', { name: 'Locales' })).toBeNull();
    await userEvent.clear(field);
    await userEvent.type(field, 'zzz');
    expect(names()).toEqual([]);
    expect(screen.getByText('Aucune branche ne correspond.')).toBeInTheDocument();
  });

  it('moves with the arrows and wraps around, the chosen row being the one the search field points at', async () => {
    const { field } = setup();
    await loaded();
    expect(cursor()).toBe(0);
    expect(field).toHaveAttribute('aria-expanded', 'true');
    expect(field).toHaveAttribute('aria-activedescendant', screen.getAllByRole('option')[0].id);
    await userEvent.keyboard('{ArrowDown}{ArrowDown}');
    expect(cursor()).toBe(2);
    expect(field).toHaveAttribute('aria-activedescendant', screen.getAllByRole('option')[2].id);
    await userEvent.keyboard('{ArrowUp}{ArrowUp}{ArrowUp}');
    expect(cursor()).toBe(LIST.length - 1);
    await userEvent.keyboard('{ArrowDown}');
    expect(cursor()).toBe(0);
  });

  it('follows the mouse', async () => {
    setup();
    await loaded();
    await fireEvent.mouseMove(option('origin/hotfix'));
    expect(cursor()).toBe(4);
  });

  it('closes on Escape, and on a click away from it', async () => {
    const { onclose } = setup();
    await loaded();
    await userEvent.keyboard('{Escape}');
    expect(onclose).toHaveBeenCalledTimes(1);
    await userEvent.click(document.querySelector('.backdrop')!);
    expect(onclose).toHaveBeenCalledTimes(2);
  });

  it('is over the dialogs when it is opened from one, under them else', async () => {
    setup();
    await loaded();
    expect(screen.getByRole('dialog')).not.toHaveClass('over');
    cleanup();
    fakeBackend({ branch_list: () => LIST });
    render(BranchPicker, { projectId: 'p1', onclose: vi.fn(), mode: 'pick', onpick: vi.fn(), over: true });
    await loaded();
    expect(screen.getByRole('dialog')).toHaveClass('over');
    expect(document.querySelector('.backdrop')).toHaveClass('over');
  });

  it('leaves Escape to the context menu while one is open', async () => {
    const { onclose } = setup();
    await loaded();
    menu.open = { x: 0, y: 0, items: [{ label: 'x' }] };
    await userEvent.keyboard('{Escape}');
    expect(onclose).not.toHaveBeenCalled();
  });

  describe('switching', () => {
    it('switches to the chosen branch on Enter, and closes', async () => {
      const { backend, onclose } = setup({ handlers: { branch_switch: () => null } });
      await loaded();
      await userEvent.keyboard('{ArrowDown}{Enter}');
      await waitFor(() => expect(onclose).toHaveBeenCalled());
      expect(backend.called('branch_switch').map((c) => c.args)).toEqual([{ projectId: 'p1', name: 'feat/login', stash: false }]);
      expect(app.toasts).toEqual([]);
    });

    it('switches on a click, and to a remote branch by its name', async () => {
      const { backend, onclose } = setup({ handlers: { branch_switch: () => null } });
      await loaded();
      await userEvent.click(option('origin/hotfix'));
      await waitFor(() => expect(onclose).toHaveBeenCalled());
      expect(backend.called('branch_switch')[0].args).toEqual({ projectId: 'p1', name: 'origin/hotfix', stash: false });
    });

    it('switches to the best match of what is typed', async () => {
      const { backend, field } = setup({ handlers: { branch_switch: () => null } });
      await loaded();
      await userEvent.type(field, 'hotfix{Enter}');
      await waitFor(() => expect(backend.called('branch_switch')).toHaveLength(1));
      expect(backend.called('branch_switch')[0].args.name).toBe('origin/hotfix');
    });

    it('does nothing but close for the branch the folder is already on', async () => {
      const { backend, onclose } = setup();
      await loaded();
      await userEvent.keyboard('{Enter}');
      expect(onclose).toHaveBeenCalled();
      expect(backend.called('branch_switch')).toHaveLength(0);
    });

    it('asks before putting uncommitted changes aside, then hands over to the question', async () => {
      const { backend, onclose } = setup({
        handlers: {
          branch_switch: (args) => {
            if (!args.stash) throw 'DIRTY';
            return 'escouade: avant de passer sur feat/login';
          },
        },
      });
      await loaded();
      await userEvent.click(option('feat/login'));
      await waitFor(() => expect(onclose).toHaveBeenCalled());
      expect(app.modal).toMatchObject({
        kind: 'confirm',
        body: 'Il reste des changements non commités dans le dossier du projet.',
        confirm: 'Mettre de côté (stash) et changer',
      });
      if (app.modal?.kind !== 'confirm') throw new Error('no question');
      await app.modal.onConfirm(false);
      expect(backend.called('branch_switch').map((c) => c.args.stash)).toEqual([false, true]);
      expect(app.toasts.map((t) => t.text)).toEqual([
        'Changements mis de côté : « escouade: avant de passer sur feat/login » (git stash).',
      ]);
    });

    it('tells an agent that works in the folder in an error toast, and stays open for another choice', async () => {
      const { backend, onclose } = setup({
        handlers: {
          branch_switch: () => {
            throw 'AGENT_WORKING:a1:refacto-auth';
          },
        },
      });
      await loaded();
      await userEvent.click(option('feat/login'));
      await waitFor(() => expect(app.toasts).toHaveLength(1));
      expect(app.toasts[0]).toMatchObject({
        text: 'L’agent refacto-auth travaille dans le dossier du projet : attends la fin de son tour.',
        kind: 'error',
      });
      expect(onclose).not.toHaveBeenCalled();
      expect(app.modal).toBeNull();
      // The list is read again: what was refused may come from what changed.
      await waitFor(() => expect(backend.called('branch_list')).toHaveLength(2));
    });

    it('takes no second switch while one runs', async () => {
      let finish!: (v: null) => void;
      const { backend } = setup({ handlers: { branch_switch: () => new Promise((r) => (finish = r)) } });
      await loaded();
      await userEvent.click(option('feat/login'));
      await userEvent.click(option('origin/hotfix'));
      expect(backend.called('branch_switch')).toHaveLength(1);
      expect(screen.getByRole('status')).toHaveTextContent('Changement de branche…');
      finish(null);
    });
  });

  describe('a branch an agent works on', () => {
    it('says which agent has it, cannot be activated, and gives the reason on hover and on the cursor', async () => {
      const { backend, onclose } = setup();
      await loaded();
      const held = option('escouade/refacto-auth');
      const reason = 'La branche « escouade/refacto-auth » est utilisée par l’agent refacto-auth, dans son worktree.';
      expect(held).toHaveTextContent('utilisée par l’agent refacto-auth');
      expect(held).toHaveAttribute('aria-disabled', 'true');
      expect(held).toHaveAttribute('title', reason);
      // On the keyboard: the cursor reaches it, says why, and Enter does nothing.
      await userEvent.keyboard('{ArrowDown}{ArrowDown}');
      expect(cursor()).toBe(2);
      expect(held).toHaveAttribute('aria-describedby');
      expect(document.getElementById(held.getAttribute('aria-describedby')!)).toHaveTextContent(reason);
      await userEvent.keyboard('{Enter}');
      expect(backend.called('branch_switch')).toHaveLength(0);
      expect(onclose).not.toHaveBeenCalled();
      // And with the mouse.
      await userEvent.click(held);
      expect(backend.called('branch_switch')).toHaveLength(0);
      expect(onclose).not.toHaveBeenCalled();
      expect(screen.getByRole('combobox')).toHaveFocus();
      expect(option('feat/login')).not.toHaveAttribute('aria-disabled');
    });

    it('holds back the remote branch of a local branch an agent has, too', async () => {
      setup({
        list: [MAIN, HELD, branchInfo({ name: 'origin/escouade/refacto-auth', remote: true, trackedBy: 'escouade/refacto-auth' })],
      });
      await loaded(3);
      const remote = option('origin/escouade/refacto-auth');
      expect(remote).toHaveAttribute('aria-disabled', 'true');
      expect(remote).toHaveTextContent('utilisée par l’agent refacto-auth');
    });

    it('also holds back a branch another worktree has', async () => {
      const { backend } = setup({ list: [MAIN, branchInfo({ name: 'spike', worktree: 'D:\\other\\spike' })] });
      await loaded(2);
      const spike = option('spike');
      expect(spike).toHaveAttribute('aria-disabled', 'true');
      expect(spike).toHaveTextContent('dans un autre worktree');
      expect(spike).toHaveAttribute('title', 'La branche « spike » est prise par le worktree D:\\other\\spike.');
      await userEvent.click(spike);
      expect(backend.called('branch_switch')).toHaveLength(0);
    });
  });

  describe('the foot', () => {
    it('opens the new branch window and the merged branches one, over the picker', async () => {
      const { onclose } = setup();
      await loaded();
      await userEvent.click(screen.getByRole('button', { name: 'Nouvelle branche…' }));
      expect(onclose).toHaveBeenCalledTimes(1);
      await waitFor(() => expect(app.modal).toEqual({ kind: 'newBranch', projectId: 'p1' }));
      await userEvent.click(screen.getByRole('button', { name: 'Branches mergées…' }));
      expect(onclose).toHaveBeenCalledTimes(2);
      await waitFor(() => expect(app.modal).toEqual({ kind: 'mergedBranches', projectId: 'p1' }));
    });

    it('offers to pull and push only what there is to pull or push, and to fetch', async () => {
      setup({ git: { ...tracked, ahead: 2, behind: 0 } });
      await loaded();
      const pull = screen.getByRole('button', { name: /Récupérer/ });
      expect(pull).toBeDisabled();
      expect(pull).toHaveTextContent('↓0');
      const push = screen.getByRole('button', { name: /Pousser/ });
      expect(push).toBeEnabled();
      expect(push).toHaveTextContent('↑2');
      expect(screen.getByRole('button', { name: /Fetch/ })).toHaveTextContent('maintenant');
      expect(screen.getByRole('button', { name: /Fetch/ })).toBeEnabled();
    });

    it('pulls, pushes and fetches the project’s checkout, and says what came of it', async () => {
      const backend = fakeBackend({
        branch_list: () => LIST,
        git_pull: () => '5 commits tirés',
        git_push: () => '2 commits poussés',
        git_fetch: () => 'Fetch terminé',
      });
      app.git = { p1: gitInfo(tracked) };
      const onclose = vi.fn();
      render(BranchPicker, { projectId: 'p1', onclose });
      await loaded();
      await userEvent.click(screen.getByRole('button', { name: /Récupérer/ }));
      await userEvent.click(screen.getByRole('button', { name: /Pousser/ }));
      await userEvent.click(screen.getByRole('button', { name: /Fetch/ }));
      expect(backend.called('git_pull')[0].args).toEqual({ projectId: 'p1' });
      expect(backend.called('git_push')[0].args).toEqual({ projectId: 'p1' });
      expect(backend.called('git_fetch')[0].args).toEqual({ projectId: 'p1' });
      expect(onclose).toHaveBeenCalledTimes(3);
      await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['5 commits tirés', '2 commits poussés', 'Fetch terminé']));
    });

    it('publishes a branch that tracks none, or whose remote branch is gone', async () => {
      setup({ git: { hasRemote: true, branch: 'feat/x' } });
      await loaded();
      expect(screen.getByRole('button', { name: /Récupérer/ })).toBeDisabled();
      expect(screen.queryByRole('button', { name: /Pousser/ })).toBeNull();
      expect(screen.getByRole('button', { name: 'Publier' })).toBeEnabled();
    });

    it('has no sync for a repository without a remote, nor on a detached HEAD', async () => {
      setup({ git: { hasRemote: false } });
      await loaded();
      expect(screen.getByRole('button', { name: 'Nouvelle branche…' })).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /Récupérer|Pousser|Publier|Fetch/ })).toBeNull();
    });

    it('lets the buttons of the foot be reached with Tab from the search field', async () => {
      setup();
      await loaded();
      await userEvent.tab();
      expect(screen.getByRole('button', { name: 'Nouvelle branche…' })).toHaveFocus();
    });
  });

  describe('deleting', () => {
    it('opens a menu on the right click of a branch, with the one way to delete it, which asks first', async () => {
      const { onclose } = setup();
      await loaded();
      await fireEvent.contextMenu(option('feat/login'));
      expect(menu.open!.items.map((i) => [i.label, i.danger])).toEqual([
        ['Lancer un agent sur cette branche', undefined],
        ['', undefined],
        ['Supprimer la branche…', true],
      ]);
      menu.open!.items[2].onClick!();
      expect(onclose).toHaveBeenCalled();
      await waitFor(() => expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Supprimer la branche « feat/login » ?' }));
    });

    it('offers it on the keyboard too: the menu key, or Shift+F10, on the row the cursor is on', async () => {
      const { field } = setup();
      await loaded();
      await userEvent.keyboard('{ArrowDown}');
      await fireEvent.keyDown(field, { key: 'ContextMenu' });
      expect(menu.open!.items.map((i) => i.label)).toEqual(['Lancer un agent sur cette branche', '', 'Supprimer la branche…']);
      menu.close();
      await fireEvent.keyDown(field, { key: 'F10', shiftKey: true });
      expect(menu.open!.items.map((i) => i.label)).toEqual(['Lancer un agent sur cette branche', '', 'Supprimer la branche…']);
      menu.close();
      await fireEvent.keyDown(field, { key: 'F10' });
      expect(menu.open).toBeNull();
    });

    it('names a remote branch as such', async () => {
      setup();
      await loaded();
      await fireEvent.contextMenu(option('origin/hotfix'));
      expect(menu.open!.items.map((i) => i.label)).toEqual(['Lancer un agent sur cette branche', '', 'Supprimer la branche distante…']);
    });

    it('starts an agent on the branch from the menu, the picker gone first', async () => {
      const { backend, onclose } = setup({
        handlers: { create_agent_on_branch: (a: any) => agent({ id: 'a9', name: 'agent-9', projectId: a.projectId }) },
      });
      await loaded();
      await fireEvent.contextMenu(option('origin/hotfix'));
      menu.open!.items[0].onClick!();
      expect(onclose).toHaveBeenCalled();
      await waitFor(() => expect(backend.called('create_agent_on_branch')).toHaveLength(1));
      expect(backend.called('create_agent_on_branch')[0].args).toEqual({ projectId: 'p1', branch: 'origin/hotfix', model: null });
      await waitFor(() => expect(app.ui.selectedAgent.p1).toBe('a9'));
    });

    it('tells in a toast why an agent could not be started there', async () => {
      setup({
        handlers: {
          create_agent_on_branch: () => {
            throw 'IN_WORKTREE:a1:refacto-auth';
          },
        },
      });
      await loaded();
      await fireEvent.contextMenu(option('feat/login'));
      menu.open!.items[0].onClick!();
      await waitFor(() =>
        expect(app.toasts).toEqual([
          expect.objectContaining({
            text: 'La branche « feat/login » est utilisée par l’agent refacto-auth, dans son worktree.',
            kind: 'error',
          }),
        ]),
      );
    });

    it('never offers it for the current branch, nor for one a worktree holds', async () => {
      setup();
      await loaded();
      await fireEvent.contextMenu(option('main'));
      expect(menu.open).toBeNull();
      await fireEvent.contextMenu(option('escouade/refacto-auth'));
      expect(menu.open).toBeNull();
      await userEvent.keyboard('{ArrowDown}{ArrowDown}');
      await fireEvent.keyDown(screen.getByRole('combobox'), { key: 'ContextMenu' });
      expect(menu.open).toBeNull();
    });
  });

  it('says it when the branches cannot be read', async () => {
    setup({
      handlers: {
        branch_list: () => {
          throw 'fatal: not a git repository';
        },
      },
    });
    expect(await screen.findByRole('alert')).toHaveTextContent('fatal: not a git repository');
    expect(names()).toEqual([]);
  });

  it('reads the branches again when the project’s git state changes, keeping the cursor on its branch', async () => {
    const { backend } = setup();
    await loaded();
    await userEvent.keyboard('{ArrowDown}');
    expect(cursor()).toBe(1);
    app.git = { p1: gitInfo({ ...tracked, branch: 'feat/login' }) };
    await waitFor(() => expect(backend.called('branch_list')).toHaveLength(2));
    await loaded();
    expect(cursor()).toBe(1);
  });

  it('shows a detached HEAD as no branch checked out', async () => {
    setup({ list: [branchInfo({ name: 'main' }), FEAT], git: { branch: '(detached)', hasRemote: true } });
    await loaded(2);
    expect(screen.queryByText('✓')).toBeNull();
    expect(screen.getAllByRole('option').some((o) => o.hasAttribute('aria-current'))).toBe(false);
  });

  it('keeps the list short, and says how many branches the search will find', async () => {
    const many = Array.from({ length: 320 }, (_, i) => branchInfo({ name: `old/${String(i).padStart(3, '0')}` }));
    const { field } = setup({ list: [MAIN, ...many] });
    await loaded(300);
    expect(screen.getByText('21 autres branches : tape pour les chercher.')).toBeInTheDocument();
    await userEvent.type(field, 'old/319');
    expect(names()).toEqual(['old/319']);
    expect(screen.queryByText(/autres? branches?/)).toBeNull();
  });

  it('says there is no branch in a repository that has none yet', async () => {
    setup({ list: [] });
    expect(await screen.findByText('Aucune branche.')).toBeInTheDocument();
  });
});

/** The picker to choose the branch of an agent or of a ticket. */
function pick(over: { list?: BranchInfo[] } = {}) {
  const backend = fakeBackend({ branch_list: () => over.list ?? LIST, branch_switch: () => null });
  app.git = { p1: gitInfo(tracked) };
  const [onclose, onpick] = [vi.fn(), vi.fn()];
  render(BranchPicker, { projectId: 'p1', onclose, mode: 'pick', onpick });
  return { backend, onclose, onpick, field: screen.getByRole('combobox') };
}

describe('BranchPicker to pick a branch', () => {
  beforeEach(() => {
    resetApp({ agents: [agent({ id: 'a1', name: 'refacto-auth' })] });
    menu.close();
  });

  it('lists the branches under a title of its own, with none of what is done to branches', async () => {
    const { field } = pick();
    await loaded();
    expect(screen.getByRole('dialog', { name: 'Choisir une branche' })).toBeInTheDocument();
    expect(field).toHaveFocus();
    expect(names()).toEqual(['main', 'feat/login', 'escouade/refacto-auth', 'origin/feat/login', 'origin/hotfix']);
    for (const label of [/Nouvelle branche/, /Branches mergées/, /Récupérer/, /Pousser/, /Fetch/]) {
      expect(screen.queryByRole('button', { name: label })).toBeNull();
    }
  });

  it('gives the branch clicked, a remote one under its own name, and does not switch to it', async () => {
    const { backend, onclose, onpick } = pick();
    await loaded();
    await userEvent.click(option('feat/login'));
    expect(onpick).toHaveBeenCalledWith(FEAT);
    expect(onclose).toHaveBeenCalled();
    onpick.mockClear();
    await userEvent.click(option('origin/hotfix'));
    expect(onpick).toHaveBeenCalledWith(REMOTE_HOTFIX);
    expect(backend.called('branch_switch')).toHaveLength(0);
  });

  it('picks with the keyboard too: the arrows, then Enter', async () => {
    const { onpick } = pick();
    await loaded();
    await userEvent.keyboard('{ArrowDown}{Enter}');
    expect(onpick).toHaveBeenCalledWith(FEAT);
  });

  it('cannot pick the branch of the project’s folder, which says why, nor the remote branch it tracks', async () => {
    const list = [MAIN, FEAT, branchInfo({ name: 'origin/main', remote: true, trackedBy: 'main' })];
    const { onpick, onclose } = pick({ list });
    await loaded(3);
    const why = 'C’est la branche du dossier du projet : un agent sans worktree y travaille déjà, ou change de branche d’abord.';
    for (const name of ['main', 'origin/main']) {
      expect(option(name)).toHaveAttribute('aria-disabled', 'true');
      expect(option(name)).toHaveAttribute('title', why);
      await userEvent.click(option(name));
    }
    expect(onpick).not.toHaveBeenCalled();
    expect(onclose).not.toHaveBeenCalled();
    // The cursor on it: the reason is under the list, and Enter does nothing.
    await userEvent.hover(option('main'));
    expect(screen.getByText(why, { selector: 'p' })).toBeInTheDocument();
    await userEvent.keyboard('{Enter}');
    expect(onpick).not.toHaveBeenCalled();
  });

  it('cannot pick the board’s target branch, nor the remote branch that tracks it, which says why', async () => {
    app.projects = [project({ board: board({ target: 'feat/login' }) })];
    const { onpick, onclose } = pick();
    await loaded();
    const why = 'C’est la branche cible du Kanban : un agent n’y travaille pas directement.';
    for (const name of ['feat/login', 'origin/feat/login']) {
      expect(option(name)).toHaveAttribute('aria-disabled', 'true');
      expect(option(name)).toHaveAttribute('title', why);
      await userEvent.click(option(name));
    }
    expect(onpick).not.toHaveBeenCalled();
    expect(onclose).not.toHaveBeenCalled();
    // The others are still there to pick, a remote branch no local one tracks included.
    await userEvent.click(option('origin/hotfix'));
    expect(onpick).toHaveBeenCalledWith(REMOTE_HOTFIX);
  });

  it('says it is the target, not the folder’s branch, when the folder is on it', async () => {
    app.projects = [project({ board: board({ target: 'main' }) })];
    const { onpick } = pick();
    await loaded();
    expect(option('main')).toHaveAttribute('aria-disabled', 'true');
    expect(option('main')).toHaveAttribute('title', 'C’est la branche cible du Kanban : un agent n’y travaille pas directement.');
    await userEvent.click(option('main'));
    expect(onpick).not.toHaveBeenCalled();
  });

  it('keeps the target branch for the other picker: only the choice of an agent’s branch refuses it', async () => {
    app.projects = [project({ board: board({ target: 'feat/login' }) })];
    setup();
    await loaded();
    expect(option('feat/login')).not.toHaveAttribute('aria-disabled', 'true');
    expect(option('feat/login')).not.toHaveAttribute('title');
  });

  it('cannot pick a branch a worktree holds either', async () => {
    const { onpick } = pick();
    await loaded();
    expect(option('escouade/refacto-auth')).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(option('escouade/refacto-auth'));
    expect(onpick).not.toHaveBeenCalled();
  });

  it('opens no menu on a right click', async () => {
    pick();
    await loaded();
    await fireEvent.contextMenu(option('feat/login'));
    expect(menu.open).toBeNull();
  });

  it('writes its title and its reason in English', async () => {
    setLang('en');
    pick();
    await loaded();
    expect(screen.getByRole('dialog', { name: 'Choose a branch' })).toBeInTheDocument();
    expect(option('main')).toHaveAttribute(
      'title',
      'This is the branch of the project’s folder: an agent without a worktree already works on it, or switch branches first.',
    );
  });

  it('writes the reason of the target branch in English', async () => {
    setLang('en');
    app.projects = [project({ board: board({ target: 'feat/login' }) })];
    pick();
    await loaded();
    expect(option('feat/login')).toHaveAttribute('title', 'It’s the board’s target branch: an agent doesn’t work on it directly.');
  });
});

describe('BranchPicker in English', () => {
  beforeEach(() => {
    resetApp({ agents: [agent({ id: 'a1', name: 'refacto-auth' })] });
    menu.close();
    setLang('en');
  });

  it('writes the groups, the labels, the foot and the reasons in English', async () => {
    const { field } = setup();
    await loaded();
    expect(screen.getByRole('dialog', { name: 'Branches' })).toBeInTheDocument();
    expect(field).toHaveAttribute('placeholder', 'Search for a branch…');
    expect(screen.getByRole('group', { name: 'Local' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Remote' })).toBeInTheDocument();
    expect(option('escouade/refacto-auth')).toHaveTextContent('used by agent refacto-auth');
    expect(option('escouade/refacto-auth')).toHaveAttribute(
      'title',
      'The branch “escouade/refacto-auth” is used by agent refacto-auth, in its worktree.',
    );
    expect(option('main')).toHaveTextContent('2 commits to push');
    expect(option('main')).toHaveTextContent('5 commits to pull');
    expect(screen.getByRole('button', { name: 'New branch…' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Merged branches…' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Pull/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Push/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Fetch/ })).toHaveTextContent('now');
    await userEvent.type(field, 'zzz');
    expect(screen.getByText('No branch matches.')).toBeInTheDocument();
  });

  it('writes the menu of a branch in English', async () => {
    setup();
    await loaded();
    await fireEvent.contextMenu(option('feat/login'));
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Start an agent on this branch', '', 'Delete branch…']);
  });
});
