import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app, type Modal } from '../../lib/state.svelte';
import type { BranchInfo } from '../../lib/types';
import { branchInfo, fakeBackend, resetApp } from '../../test/ipc';
import NewBranchModal from './NewBranchModal.svelte';

const LIST: BranchInfo[] = [
  branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api' }),
  branchInfo({ name: 'feat/login' }),
  branchInfo({ name: 'origin/hotfix', remote: true }),
];

const COMMIT = 'a1b2c3d4e5f60718293a4b5c6d7e8f9012345678';

type Handlers = Record<string, (args: any) => unknown>;

/** The window over a project whose branches the fake backend lists. */
function setup(handlers: Handlers = {}, props: { start?: string; resume?: { name: string; start: string; switchTo: boolean } } = {}) {
  const backend = fakeBackend({ branch_list: () => LIST, branch_check: () => null, branch_create: () => null, ...handlers });
  app.modal = { kind: 'newBranch', projectId: 'p1', ...props };
  const view = render(NewBranchModal, { projectId: 'p1', ...props });
  return { backend, ...view };
}

const field = () => screen.getByRole('textbox', { name: 'Nom' });
const from = () => screen.getByRole('combobox', { name: 'À partir de' }) as HTMLSelectElement;
const switchTo = () => screen.getByRole('checkbox', { name: 'et y passer' }) as HTMLInputElement;
const create = () => screen.getByRole('button', { name: 'Créer' });
const loaded = () => waitFor(() => expect(within(screen.getByRole('combobox')).getAllByRole('option').length).toBeGreaterThan(0));

describe('NewBranchModal', () => {
  beforeEach(() => resetApp());

  it('opens on the name, starting from the current branch, and switching to the new one', async () => {
    const { backend } = setup();
    expect(screen.getByRole('dialog', { name: 'Nouvelle branche' })).toBeInTheDocument();
    expect(field()).toHaveFocus();
    expect(field()).toHaveAttribute('placeholder', 'feat/ma-branche');
    expect(switchTo()).toBeChecked();
    expect(create()).toBeDisabled();
    await loaded();
    expect(backend.called('branch_list')[0].args).toEqual({ projectId: 'p1' });
    expect(
      within(from())
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual(['main', 'feat/login', 'origin/hotfix']);
    expect(from().value).toBe('main');
  });

  it('checks the name as it is typed, and says what is wrong under the field', async () => {
    let refusal: string | null = 'Un nom de branche ne peut pas contenir d’espace.';
    const { backend } = setup({
      branch_check: () => {
        if (refusal) throw refusal;
        return null;
      },
    });
    await userEvent.type(field(), 'my branch');
    const problem = await screen.findByText('Un nom de branche ne peut pas contenir d’espace.');
    expect(backend.called('branch_check').at(-1)!.args).toEqual({ projectId: 'p1', name: 'my branch' });
    // Only once the typing paused: not for every key.
    expect(backend.called('branch_check').length).toBeLessThan(4);
    expect(field()).toHaveAttribute('aria-invalid', 'true');
    expect(field().getAttribute('aria-describedby')).toBe(problem.id);
    expect(create()).toBeDisabled();
    refusal = null;
    await userEvent.clear(field());
    await userEvent.type(field(), 'feat/ok');
    await waitFor(() => expect(screen.queryByText('Un nom de branche ne peut pas contenir d’espace.')).toBeNull());
    expect(field()).not.toHaveAttribute('aria-invalid', 'true');
    expect(create()).toBeEnabled();
  });

  it('does not check an empty name, and takes away the message once it is empty again', async () => {
    const { backend } = setup({
      branch_check: () => {
        throw 'La branche « x » existe déjà.';
      },
    });
    await userEvent.type(field(), 'x');
    await screen.findByText('La branche « x » existe déjà.');
    await userEvent.clear(field());
    expect(screen.queryByText('La branche « x » existe déjà.')).toBeNull();
    const calls = backend.called('branch_check').length;
    await new Promise((r) => setTimeout(r, 400));
    expect(backend.called('branch_check')).toHaveLength(calls);
  });

  it('creates the branch and switches to it on Enter, starting from the branch chosen', async () => {
    const { backend } = setup();
    await loaded();
    await userEvent.type(field(), 'feat/search');
    await userEvent.selectOptions(from(), 'origin/hotfix');
    await userEvent.type(field(), '{Enter}');
    await waitFor(() => expect(app.modal).toBeNull());
    expect(backend.called('branch_create').map((c) => c.args)).toEqual([
      { projectId: 'p1', name: 'feat/search', start: 'origin/hotfix', switch: true, stash: false },
    ]);
    expect(app.toasts).toEqual([expect.objectContaining({ text: 'Branche « feat/search » créée : tu es dessus.', kind: 'ok' })]);
  });

  it('creates it without switching when « et y passer » is unticked', async () => {
    const { backend } = setup();
    await loaded();
    await userEvent.type(field(), 'feat/later');
    await userEvent.click(switchTo());
    await userEvent.click(create());
    await waitFor(() => expect(app.modal).toBeNull());
    expect(backend.called('branch_create')[0].args).toMatchObject({ name: 'feat/later', start: 'main', switch: false });
    expect(app.toasts.map((t) => t.text)).toEqual(['Branche « feat/later » créée.']);
  });

  it('starts from the commit the graph gave, which it names', async () => {
    const { backend } = setup({}, { start: COMMIT });
    await loaded();
    expect(from().value).toBe(COMMIT);
    expect(within(from()).getAllByRole('option')[0]).toHaveTextContent('Le commit a1b2c3d');
    await userEvent.type(field(), 'fix/here{Enter}');
    await waitFor(() => expect(backend.called('branch_create')).toHaveLength(1));
    expect(backend.called('branch_create')[0].args.start).toBe(COMMIT);
  });

  it('starts from a branch the graph gave when it is one it lists', async () => {
    setup({}, { start: 'origin/hotfix' });
    await loaded();
    expect(from().value).toBe('origin/hotfix');
    expect(within(from()).getAllByRole('option')).toHaveLength(3);
  });

  it('starts from the current commit on a detached HEAD', async () => {
    const { backend } = setup({ branch_list: () => [branchInfo({ name: 'main' }), branchInfo({ name: 'feat/login' })] });
    await waitFor(() => expect(within(from()).getAllByRole('option')).toHaveLength(3));
    expect(within(from()).getAllByRole('option')[0]).toHaveTextContent('Le commit courant');
    await userEvent.type(field(), 'rescue{Enter}');
    await waitFor(() => expect(backend.called('branch_create')).toHaveLength(1));
    expect(backend.called('branch_create')[0].args.start).toBe('');
  });

  it('asks before putting uncommitted changes aside, then creates with the stash', async () => {
    const { backend, unmount } = setup({
      branch_create: (args) => {
        if (!args.stash) throw 'DIRTY';
        return 'escouade: avant de passer sur feat/x';
      },
    });
    await loaded();
    await userEvent.type(field(), 'feat/x');
    await userEvent.selectOptions(from(), 'feat/login');
    await userEvent.type(field(), '{Enter}');
    await waitFor(() => expect(app.modal?.kind).toBe('confirm'));
    expect(app.modal).toMatchObject({
      title: 'Créer « feat/x » et y passer ?',
      body: 'Il reste des changements non commités dans le dossier du projet.',
      confirm: 'Mettre de côté (stash) et changer',
    });
    const question = app.modal as Extract<Modal, { kind: 'confirm' }>;
    // The window is gone (the question took its place): cancelling brings it back as it was.
    unmount();
    question.onCancel!();
    expect(app.modal).toMatchObject({
      kind: 'newBranch',
      projectId: 'p1',
      resume: { name: 'feat/x', start: 'feat/login', switchTo: true },
    });
    app.modal = question;
    await question.onConfirm(false);
    expect(backend.called('branch_create').map((c) => c.args.stash)).toEqual([false, true]);
    expect(app.toasts.map((t) => t.text)).toEqual([
      'Branche « feat/x » créée : tu es dessus.',
      'Changements mis de côté : « escouade: avant de passer sur feat/x » (git stash).',
    ]);
  });

  it('comes back with what was written', async () => {
    setup({}, { resume: { name: 'feat/back', start: 'feat/login', switchTo: false } });
    await loaded();
    expect(field()).toHaveValue('feat/back');
    expect(from().value).toBe('feat/login');
    expect(switchTo()).not.toBeChecked();
  });

  it('tells an agent that works in the folder, and stays open to create the branch without switching', async () => {
    const { backend } = setup({
      branch_create: (args) => {
        if (args.switch) throw 'AGENT_WORKING:a1:refacto-auth';
        return null;
      },
    });
    await loaded();
    await userEvent.type(field(), 'feat/x{Enter}');
    await waitFor(() => expect(app.toasts).toHaveLength(1));
    expect(app.toasts[0]).toMatchObject({
      text: 'L’agent refacto-auth travaille dans le dossier du projet : attends la fin de son tour.',
      kind: 'error',
    });
    expect(app.modal?.kind).toBe('newBranch');
    expect(field()).toHaveValue('feat/x');
    await userEvent.click(switchTo());
    await userEvent.click(create());
    await waitFor(() => expect(app.modal).toBeNull());
    expect(backend.called('branch_create').map((c) => c.args.switch)).toEqual([true, false]);
  });

  it('shows any other refusal under the form, where it was written', async () => {
    setup({
      branch_create: () => {
        throw '« abc1234 » ne désigne aucun commit.';
      },
    });
    await loaded();
    await userEvent.type(field(), 'feat/x{Enter}');
    expect(await screen.findByRole('alert')).toHaveTextContent('« abc1234 » ne désigne aucun commit.');
    expect(app.modal?.kind).toBe('newBranch');
    expect(create()).toBeEnabled();
  });

  it('creates once while it runs', async () => {
    let finish!: (v: null) => void;
    const { backend } = setup({ branch_create: () => new Promise((r) => (finish = r)) });
    await loaded();
    await userEvent.type(field(), 'feat/x{Enter}');
    await userEvent.type(field(), '{Enter}');
    await fireEvent.click(create());
    expect(backend.called('branch_create')).toHaveLength(1);
    expect(create()).toBeDisabled();
    finish(null);
    await waitFor(() => expect(app.modal).toBeNull());
  });

  it('closes on Escape and on « Annuler » without creating anything', async () => {
    const { backend } = setup();
    await userEvent.type(field(), 'feat/x');
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
    app.modal = { kind: 'newBranch', projectId: 'p1' };
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
    expect(backend.called('branch_create')).toHaveLength(0);
  });
});

describe('NewBranchModal in English', () => {
  beforeEach(() => {
    resetApp();
    setLang('en');
  });

  it('writes the window, its choices and its toast in English', async () => {
    const { backend } = setup();
    expect(screen.getByRole('dialog', { name: 'New branch' })).toBeInTheDocument();
    await loaded();
    const name = screen.getByRole('textbox', { name: 'Name' });
    expect(name).toHaveAttribute('placeholder', 'feat/my-branch');
    expect(screen.getByRole('combobox', { name: 'Start from' })).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'and switch to it' })).toBeChecked();
    expect(screen.getByRole('button', { name: 'Create' })).toBeDisabled();
    await userEvent.type(name, 'feat/x{Enter}');
    await waitFor(() => expect(backend.called('branch_create')).toHaveLength(1));
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['Branch “feat/x” created: you’re on it.']));
  });
});
