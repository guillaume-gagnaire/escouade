import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import ConfirmModal from '../components/modals/ConfirmModal.svelte';
import { agent, fakeBackend, resetApp } from '../test/ipc';
import { mergeAgent } from './agent-actions';
import { app } from './state.svelte';

const WORKTREE = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\landing', branch: 'ccm/landing', baseBranch: 'main' };
const landing = () => agent({ id: 'a2', name: 'landing', worktree: WORKTREE });

let shown: { unmount: () => void } | null = null;

/** The confirmation currently asked, rendered as the app does (the one before it is gone). */
function asked() {
  shown?.unmount();
  expect(app.modal?.kind).toBe('confirm');
  shown = render(ConfirmModal, { ...(app.modal as any) });
}

describe('mergeAgent', () => {
  beforeEach(() => {
    shown = null;
    resetApp({ agents: [landing()] });
  });

  it('merges into the base without asking more when the project is on it', async () => {
    const backend = fakeBackend({ merge_agent: () => 'Merge effectué en un commit' });
    mergeAgent(app.agents.a2);
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Merger ccm/landing dans main ?', confirm: 'Merger' });
    asked();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Squash (un seul commit)' }));
    await userEvent.click(screen.getByRole('button', { name: 'Merger' }));
    expect(backend.called('merge_agent').map((c) => c.args)).toEqual([{ id: 'a2', squash: true, switchToBase: false }]);
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Merge effectué en un commit', kind: 'ok' });
    expect(app.modal).toBeNull();
  });

  it('asks to switch to the base when the project is on another branch, then merges with the same choice', async () => {
    let refuse = true;
    const backend = fakeBackend({
      merge_agent: () => {
        if (refuse) throw 'NOT_ON_BASE:feature/login:main';
        return 'Merge effectué';
      },
    });
    mergeAgent(app.agents.a2);
    asked();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Squash (un seul commit)' }));
    await userEvent.click(screen.getByRole('button', { name: 'Merger' }));
    // Nothing is shown as an error: the refusal became a question.
    expect(app.toasts).toEqual([]);
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Basculer sur « main » ?',
      body: 'Le projet est sur la branche « feature/login ». Escouade bascule sur « main » puis merge « ccm/landing ».',
      confirm: 'Basculer et merger',
    });
    expect(backend.called('merge_agent')).toHaveLength(1);

    refuse = false;
    asked();
    await userEvent.click(screen.getByRole('button', { name: 'Basculer et merger' }));
    expect(backend.called('merge_agent').map((c) => c.args)).toEqual([
      { id: 'a2', squash: true, switchToBase: false },
      { id: 'a2', squash: true, switchToBase: true },
    ]);
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Merge effectué', kind: 'ok' });
    expect(app.modal).toBeNull();
  });

  it('merges nothing when the switch is declined', async () => {
    const backend = fakeBackend({
      merge_agent: () => {
        throw 'NOT_ON_BASE:feature/login:main';
      },
    });
    mergeAgent(app.agents.a2);
    asked();
    await userEvent.click(screen.getByRole('button', { name: 'Merger' }));
    asked();
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
    expect(backend.called('merge_agent')).toHaveLength(1);
  });

  it('says so when the project is on no branch at all', async () => {
    fakeBackend({
      merge_agent: () => {
        throw 'NOT_ON_BASE::main';
      },
    });
    mergeAgent(app.agents.a2);
    asked();
    await userEvent.click(screen.getByRole('button', { name: 'Merger' }));
    expect(app.modal).toMatchObject({
      title: 'Basculer sur « main » ?',
      body: 'Le projet n’est sur aucune branche (HEAD détachée). Escouade bascule sur « main » puis merge « ccm/landing ».',
      confirm: 'Basculer et merger',
    });
  });

  it('shows git’s own refusal of the switch as it is, and any other error too', async () => {
    let n = 0;
    fakeBackend({
      merge_agent: () => {
        throw ++n === 1
          ? 'NOT_ON_BASE:feature/login:main'
          : 'git switch: error: The following untracked working tree files would be overwritten by checkout:\n\tnotes.txt';
      },
    });
    mergeAgent(app.agents.a2);
    asked();
    await userEvent.click(screen.getByRole('button', { name: 'Merger' }));
    asked();
    await userEvent.click(screen.getByRole('button', { name: 'Basculer et merger' }));
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'error' });
    expect(app.toasts.at(-1)?.text).toContain('notes.txt');
    // It is an error, not another question.
    expect(app.modal).toBeNull();
  });

  it('shows the other refusals of the backend as errors, without proposing a switch', async () => {
    fakeBackend({
      merge_agent: () => {
        throw "Rien à merger : la branche ccm/landing n'a pas de nouveau commit.";
      },
    });
    mergeAgent(app.agents.a2);
    asked();
    await userEvent.click(screen.getByRole('button', { name: 'Merger' }));
    expect(app.toasts.at(-1)).toMatchObject({ text: "Rien à merger : la branche ccm/landing n'a pas de nouveau commit.", kind: 'error' });
    expect(app.modal).toBeNull();
  });

  it('does nothing for an agent without a worktree', () => {
    fakeBackend();
    mergeAgent(agent());
    expect(app.modal).toBeNull();
  });
});
