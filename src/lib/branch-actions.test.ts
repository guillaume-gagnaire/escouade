import { waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { app } from './state.svelte';
import { agent, branchInfo, fakeBackend, resetApp } from '../test/ipc';
import {
  branchRefusal,
  deletable,
  deleteBranch,
  refusalText,
  remoteCopy,
  startAgentOn,
  switchBranch,
  tellRefusal,
  worktreeReason,
} from './branch-actions';

describe('branchRefusal', () => {
  it('reads the codes the backend refuses a branch operation with', () => {
    expect(branchRefusal('DIRTY')).toEqual({ kind: 'dirty' });
    expect(branchRefusal('IN_WORKTREE:0a1b2c3d4e5f:refacto-auth')).toEqual({
      kind: 'inWorktree',
      agentId: '0a1b2c3d4e5f',
      agent: 'refacto-auth',
    });
    expect(branchRefusal('AGENT_WORKING:0a1b2c3d4e5f:fix: login')).toEqual({
      kind: 'agentWorking',
      agentId: '0a1b2c3d4e5f',
      agent: 'fix: login',
    });
    expect(branchRefusal('UNMERGED:3')).toEqual({ kind: 'unmerged', commits: 3 });
    // None of the commits is lost, yet the branch is not in the base.
    expect(branchRefusal('UNMERGED:0')).toEqual({ kind: 'unmerged', commits: 0 });
  });

  it('leaves everything else to be told as it is', () => {
    for (const other of [
      'Il reste des changements non commités',
      'DIRTY: oups',
      'UNMERGED:x',
      'UNMERGED:3 commits',
      'not IN_WORKTREE:a:b',
      new Error('DIRTY'),
      null,
    ]) {
      expect(branchRefusal(other)).toBeNull();
    }
  });
});

describe('refusalText', () => {
  it('writes a sentence for each code, in the language of the interface, and leaves the rest as it came', () => {
    expect(refusalText('AGENT_WORKING:a1:fix', 'feat')).toBe(
      'L’agent fix travaille dans le dossier du projet : attends la fin de son tour.',
    );
    expect(refusalText('IN_WORKTREE:a1:fix', 'feat')).toBe('La branche « feat » est utilisée par l’agent fix, dans son worktree.');
    expect(refusalText('UNMERGED:1', 'feat')).toBe('1 commit n’est dans aucune autre branche.');
    expect(refusalText('UNMERGED:0', 'feat')).toBe(
      'Elle n’est pas mergée dans la base du projet, mais ses commits sont dans une autre branche.',
    );
    expect(refusalText('Le nom est pris.', 'feat')).toBe('Le nom est pris.');
    expect(refusalText('DIRTY', 'feat')).toBe('DIRTY');
    setLang('en');
    expect(refusalText('IN_WORKTREE:a1:fix', 'feat')).toBe('The branch “feat” is used by agent fix, in its worktree.');
    expect(refusalText('UNMERGED:2', 'feat')).toBe('2 commits are in no other branch.');
  });
});

describe('switchBranch', () => {
  beforeEach(() => {
    resetApp();
  });

  it('switches the folder and says nothing when nothing was put aside', async () => {
    const backend = fakeBackend({ branch_switch: () => null });
    expect(await switchBranch('p1', 'feat/x')).toBe('switched');
    expect(backend.called('branch_switch')[0].args).toEqual({ projectId: 'p1', name: 'feat/x', stash: false });
    expect(app.toasts).toEqual([]);
    expect(app.modal).toBeNull();
  });

  it('asks before putting uncommitted changes aside, then switches with the stash', async () => {
    const backend = fakeBackend({
      branch_switch: (args) => {
        if (!args.stash) throw 'DIRTY';
        return 'escouade: avant de passer sur feat/x';
      },
    });
    expect(await switchBranch('p1', 'feat/x')).toBe('asked');
    expect(app.toasts).toEqual([]);
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Changer pour « feat/x » ?',
      body: 'Il reste des changements non commités dans le dossier du projet.',
      confirm: 'Mettre de côté (stash) et changer',
    });
    if (app.modal?.kind !== 'confirm') throw new Error('no confirmation');
    expect(app.modal.danger).toBeFalsy();
    await app.modal.onConfirm(false);
    expect(backend.called('branch_switch').map((c) => c.args.stash)).toEqual([false, true]);
    expect(app.toasts).toEqual([
      expect.objectContaining({ text: 'Changements mis de côté : « escouade: avant de passer sur feat/x » (git stash).', kind: 'ok' }),
    ]);
  });

  it('lets the caller get out of the way before it asks, and only then', async () => {
    fakeBackend({
      branch_switch: () => {
        throw 'DIRTY';
      },
    });
    const seen: unknown[] = [];
    expect(await switchBranch('p1', 'feat/x', false, () => void seen.push(app.modal))).toBe('asked');
    // Not yet open when the caller was told.
    expect(seen).toEqual([null]);
    expect(app.modal?.kind).toBe('confirm');
    app.modal = null;
    fakeBackend({
      branch_switch: () => {
        throw 'AGENT_WORKING:a1:fix';
      },
    });
    expect(await switchBranch('p1', 'feat/x', false, () => void seen.push('released'))).toBe('refused');
    expect(seen).toEqual([null]);
  });

  it('tells an agent that works in the folder, and the agent that holds the branch, by their names', async () => {
    let refusal = 'AGENT_WORKING:a1:refacto-auth';
    fakeBackend({
      branch_switch: () => {
        throw refusal;
      },
    });
    expect(await switchBranch('p1', 'feat/x')).toBe('refused');
    expect(app.toasts.map((t) => [t.text, t.kind])).toEqual([
      ['L’agent refacto-auth travaille dans le dossier du projet : attends la fin de son tour.', 'error'],
    ]);
    app.toasts = [];
    refusal = 'IN_WORKTREE:a1:refacto-auth';
    expect(await switchBranch('p1', 'escouade/refacto-auth')).toBe('refused');
    expect(app.toasts.map((t) => [t.text, t.kind])).toEqual([
      ['La branche « escouade/refacto-auth » est utilisée par l’agent refacto-auth, dans son worktree.', 'error'],
    ]);
    expect(app.modal).toBeNull();
  });

  it('shows git’s own refusal as it comes', async () => {
    fakeBackend({
      branch_switch: () => {
        throw 'Ce changement de branche écraserait des fichiers non suivis du dossier : a.txt.';
      },
    });
    expect(await switchBranch('p1', 'feat/x')).toBe('refused');
    expect(app.toasts).toEqual([expect.objectContaining({ text: expect.stringContaining('a.txt'), kind: 'error' })]);
  });

  it('writes its texts in English too', async () => {
    setLang('en');
    fakeBackend({
      branch_switch: (args) => {
        if (!args.stash) throw 'DIRTY';
        return 'escouade: before switching to feat/x';
      },
    });
    await switchBranch('p1', 'feat/x');
    expect(app.modal).toMatchObject({
      title: 'Switch to “feat/x”?',
      body: 'There are uncommitted changes in the project’s folder.',
      confirm: 'Stash and switch',
    });
    if (app.modal?.kind !== 'confirm') throw new Error('no confirmation');
    await app.modal.onConfirm(false);
    expect(app.toasts[0].text).toBe('Changes stashed: “escouade: before switching to feat/x” (git stash).');
    tellRefusal('AGENT_WORKING:a1:fix', 'feat/x');
    expect(app.toasts[1].text).toBe('Agent fix is working in the project’s folder: wait for the end of its turn.');
  });
});

describe('what can be deleted', () => {
  it('never offers the branch of the folder, nor one a worktree holds', () => {
    expect(deletable(branchInfo({ name: 'feat' }))).toBe(true);
    expect(deletable(branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api' }))).toBe(false);
    expect(deletable(branchInfo({ name: 'escouade/x', worktree: 'C:\\code\\demo-api\\.claude\\worktrees\\x', agent: 'a1' }))).toBe(false);
    expect(deletable(branchInfo({ name: 'elsewhere', worktree: 'D:\\other' }))).toBe(false);
    expect(deletable(branchInfo({ name: 'origin/feat', remote: true }))).toBe(true);
  });

  it('offers the remote copy only when it has the branch’s own name', () => {
    expect(remoteCopy(branchInfo({ name: 'feat', upstream: 'origin/feat' }))).toBe('origin/feat');
    expect(remoteCopy(branchInfo({ name: 'feat/x', upstream: 'origin/feat/x' }))).toBe('origin/feat/x');
    expect(remoteCopy(branchInfo({ name: 'feat', upstream: 'origin/main' }))).toBeNull();
    expect(remoteCopy(branchInfo({ name: 'feat', upstream: 'origin/feat', upstreamGone: true }))).toBeNull();
    expect(remoteCopy(branchInfo({ name: 'feat' }))).toBeNull();
    expect(remoteCopy(branchInfo({ name: 'origin/feat', remote: true }))).toBeNull();
  });
});

describe('worktreeReason', () => {
  beforeEach(() => resetApp({ agents: [agent({ id: 'a2', name: 'refacto-auth' })] }));

  it('names the agent whose worktree holds a branch, or the folder of a worktree that is no agent’s', () => {
    expect(worktreeReason(branchInfo({ name: 'escouade/x', worktree: 'C:/wt', agent: 'a2' }))).toBe(
      'La branche « escouade/x » est utilisée par l’agent refacto-auth, dans son worktree.',
    );
    expect(worktreeReason(branchInfo({ name: 'elsewhere', worktree: 'D:/other' }))).toBe(
      'La branche « elsewhere » est prise par le worktree D:/other.',
    );
    // An agent the app no longer knows is still told apart from a missing one.
    expect(worktreeReason(branchInfo({ name: 'gone', worktree: 'C:/wt2', agent: 'zz' }))).toBe(
      'La branche « gone » est utilisée par l’agent ?, dans son worktree.',
    );
  });

  it('has nothing to say of the branch of the folder, nor of one no worktree holds', () => {
    expect(worktreeReason(branchInfo({ name: 'main', current: true, worktree: 'C:/code/demo-api' }))).toBeNull();
    expect(worktreeReason(branchInfo({ name: 'feat' }))).toBeNull();
  });

  it('writes it in English', () => {
    setLang('en');
    expect(worktreeReason(branchInfo({ name: 'escouade/x', worktree: 'C:/wt', agent: 'a2' }))).toBe(
      'The branch “escouade/x” is used by agent refacto-auth, in its worktree.',
    );
  });
});

describe('deleteBranch', () => {
  beforeEach(() => {
    resetApp();
  });

  const confirmation = () => {
    if (app.modal?.kind !== 'confirm') throw new Error('no confirmation');
    return app.modal;
  };

  it('asks once for a branch that is merged, then deletes it', async () => {
    const backend = fakeBackend({ branch_delete: () => null });
    deleteBranch('p1', branchInfo({ name: 'feat/x' }));
    expect(confirmation()).toMatchObject({
      title: 'Supprimer la branche « feat/x » ?',
      body: 'La branche « feat/x » sera supprimée en local.',
      confirm: 'Supprimer',
    });
    expect(confirmation().option).toBeUndefined();
    expect(backend.called('branch_delete')).toHaveLength(0);
    await confirmation().onConfirm(false);
    expect(backend.called('branch_delete').map((c) => c.args)).toEqual([{ projectId: 'p1', name: 'feat/x', remote: false, force: false }]);
    expect(app.toasts).toEqual([expect.objectContaining({ text: 'Branche « feat/x » supprimée.', kind: 'ok' })]);
  });

  it('offers to delete its remote copy too, off by default', async () => {
    const backend = fakeBackend({ branch_delete: () => null });
    deleteBranch('p1', branchInfo({ name: 'feat/x', upstream: 'origin/feat/x', ahead: 1 }));
    expect(confirmation().option).toEqual({ label: 'Supprimer aussi origin/feat/x', value: false });
    await confirmation().onConfirm(true);
    expect(backend.called('branch_delete')[0].args).toEqual({ projectId: 'p1', name: 'feat/x', remote: true, force: false });
  });

  it('says why a remote copy stayed', async () => {
    fakeBackend({
      branch_delete: () => '« origin/main » est la branche distante de la base du projet : seule la branche locale est supprimée.',
    });
    deleteBranch('p1', branchInfo({ name: 'main2', upstream: 'origin/main2' }));
    await confirmation().onConfirm(true);
    expect(app.toasts).toEqual([expect.objectContaining({ text: expect.stringContaining('seule la branche locale'), kind: 'info' })]);
  });

  it('counts the commits that no other branch has, and deletes only once asked again', async () => {
    const backend = fakeBackend({
      branch_delete: (args) => {
        if (!args.force) throw 'UNMERGED:2';
        return null;
      },
    });
    deleteBranch('p1', branchInfo({ name: 'feat/x' }));
    await confirmation().onConfirm(false);
    expect(backend.called('branch_delete')).toHaveLength(1);
    expect(app.toasts).toEqual([]);
    expect(confirmation()).toMatchObject({
      title: 'Supprimer « feat/x » quand même ?',
      body: '2 commits ne sont dans aucune autre branche.',
      confirm: 'Supprimer quand même',
      danger: true,
    });
    await confirmation().onConfirm(false);
    expect(backend.called('branch_delete').map((c) => c.args.force)).toEqual([false, true]);
    expect(app.toasts).toEqual([expect.objectContaining({ text: 'Branche « feat/x » supprimée.', kind: 'ok' })]);
  });

  it('keeps the choice of the remote copy when it asks again', async () => {
    const backend = fakeBackend({
      branch_delete: (args) => {
        if (!args.force) throw 'UNMERGED:1';
        return null;
      },
    });
    deleteBranch('p1', branchInfo({ name: 'feat/x', upstream: 'origin/feat/x' }));
    await confirmation().onConfirm(true);
    expect(confirmation().body).toBe('1 commit n’est dans aucune autre branche.');
    await confirmation().onConfirm(false);
    expect(backend.called('branch_delete').map((c) => [c.args.remote, c.args.force])).toEqual([
      [true, false],
      [true, true],
    ]);
  });

  it('says so when the commits are all in another branch, though the branch is not in the base', async () => {
    fakeBackend({
      branch_delete: () => {
        throw 'UNMERGED:0';
      },
    });
    deleteBranch('p1', branchInfo({ name: 'feat/x' }));
    await confirmation().onConfirm(false);
    expect(confirmation().body).toBe('Elle n’est pas mergée dans la base du projet, mais ses commits sont dans une autre branche.');
    expect(confirmation().danger).toBe(true);
  });

  it('asks for a remote branch alone as for something others see, and never deletes it unasked', async () => {
    const backend = fakeBackend({ branch_delete: () => null });
    deleteBranch('p1', branchInfo({ name: 'origin/feat/x', remote: true }));
    expect(confirmation()).toMatchObject({
      title: 'Supprimer la branche distante « origin/feat/x » ?',
      body: 'Elle sera supprimée du dépôt distant, pour tout le monde.',
      danger: true,
    });
    expect(backend.called('branch_delete')).toHaveLength(0);
    await confirmation().onConfirm(false);
    expect(backend.called('branch_delete')[0].args).toEqual({ projectId: 'p1', name: 'origin/feat/x', remote: false, force: false });
  });

  it('tells a refusal that is not about merging', async () => {
    fakeBackend({
      branch_delete: () => {
        throw 'IN_WORKTREE:a1:refacto-auth';
      },
    });
    deleteBranch('p1', branchInfo({ name: 'feat/x' }));
    await confirmation().onConfirm(false);
    await waitFor(() =>
      expect(app.toasts).toEqual([
        expect.objectContaining({
          text: 'La branche « feat/x » est utilisée par l’agent refacto-auth, dans son worktree.',
          kind: 'error',
        }),
      ]),
    );
  });

  it('writes its dialogs in English', () => {
    setLang('en');
    fakeBackend({ branch_delete: () => null });
    deleteBranch('p1', branchInfo({ name: 'feat/x', upstream: 'origin/feat/x' }));
    expect(confirmation()).toMatchObject({
      title: 'Delete the branch “feat/x”?',
      body: 'The branch “feat/x” will be deleted locally.',
      confirm: 'Delete',
      option: { label: 'Also delete origin/feat/x', value: false },
    });
  });
});

describe('startAgentOn', () => {
  beforeEach(() => resetApp({ agents: [] }));

  it('starts an agent on the branch and shows it at once, like a new agent', async () => {
    const backend = fakeBackend({ create_agent_on_branch: (a: any) => agent({ id: 'a9', name: 'agent-9', projectId: a.projectId }) });
    app.editor = { p1: { open: true } } as any;
    expect(await startAgentOn('p1', 'origin/feat/login')).toBe(true);
    expect(backend.called('create_agent_on_branch')[0].args).toEqual({ projectId: 'p1', branch: 'origin/feat/login', model: null });
    expect(app.agents.a9.name).toBe('agent-9');
    expect(app.ui.selectedAgent.p1).toBe('a9');
    expect(app.toasts).toEqual([]);
  });

  it('tells in a toast why the branch was refused, in words or by its code, and starts nothing', async () => {
    const refusals = [
      ['C’est la branche du dossier du projet : un agent sans worktree y travaille déjà, ou change de branche d’abord.'],
      ['IN_WORKTREE:a1:refacto-auth', 'La branche « feat/login » est utilisée par l’agent refacto-auth, dans son worktree.'],
    ];
    for (const [refusal, text = refusal] of refusals) {
      resetApp({ agents: [] });
      fakeBackend({
        create_agent_on_branch: () => {
          throw refusal;
        },
      });
      expect(await startAgentOn('p1', 'feat/login')).toBe(false);
      expect(app.toasts).toEqual([expect.objectContaining({ text, kind: 'error' })]);
      expect(Object.keys(app.agents)).toEqual([]);
    }
  });
});
