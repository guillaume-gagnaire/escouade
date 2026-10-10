import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import MergedBranchesModal from './MergedBranchesModal.svelte';

const MERGED = ['ticket/DEM-1', 'ticket/DEM-2', 'ticket/DEM-3'];

function setup(handlers: Record<string, (args: any) => unknown> = {}) {
  const backend = fakeBackend({ branches_merged: () => MERGED, branch_delete: () => null, ...handlers });
  app.modal = { kind: 'mergedBranches', projectId: 'p1' };
  render(MergedBranchesModal, { projectId: 'p1' });
  return backend;
}

const boxes = () => within(screen.getByRole('group', { name: 'Branches à supprimer' })).getAllByRole('checkbox') as HTMLInputElement[];
const remove = (label: string | RegExp = /^Supprimer \d+ branche/) => screen.getByRole('button', { name: label });
const loaded = () => waitFor(() => expect(boxes().length).toBeGreaterThan(0));

describe('MergedBranchesModal', () => {
  beforeEach(() => resetApp());

  it('lists the merged branches of the project, all ticked, and says what deleting does', async () => {
    const backend = setup();
    expect(screen.getByRole('dialog', { name: 'Branches mergées' })).toBeInTheDocument();
    await loaded();
    expect(backend.called('branches_merged')[0].args).toEqual({ projectId: 'p1' });
    expect(boxes().map((b) => b.labels?.[0]?.textContent?.trim())).toEqual(MERGED);
    expect(boxes().every((b) => b.checked)).toBe(true);
    expect(screen.getByText('Ces branches locales sont déjà dans la base du projet.')).toBeInTheDocument();
    expect(remove()).toHaveTextContent('Supprimer 3 branches');
    // Nothing is deleted before it is asked.
    expect(backend.called('branch_delete')).toHaveLength(0);
  });

  it('counts the branches ticked, in the singular too, and cannot delete none', async () => {
    setup();
    await loaded();
    await userEvent.click(boxes()[0]);
    expect(remove()).toHaveTextContent('Supprimer 2 branches');
    await userEvent.click(boxes()[1]);
    expect(remove()).toHaveTextContent('Supprimer 1 branche');
    expect(remove()).not.toHaveTextContent('branches');
    await userEvent.click(boxes()[2]);
    expect(remove('Supprimer 0 branche')).toBeDisabled();
  });

  it('deletes the branches ticked, none of the others, without forcing any, and says how many went', async () => {
    const backend = setup();
    await loaded();
    await userEvent.click(boxes()[1]);
    await userEvent.click(remove());
    await waitFor(() => expect(app.modal).toBeNull());
    expect(backend.called('branch_delete').map((c) => c.args)).toEqual([
      { projectId: 'p1', name: 'ticket/DEM-1', remote: false, force: false },
      { projectId: 'p1', name: 'ticket/DEM-3', remote: false, force: false },
    ]);
    expect(app.toasts).toEqual([expect.objectContaining({ text: '2 branches supprimées.', kind: 'ok' })]);
  });

  it('says it was one in the singular', async () => {
    setup({ branches_merged: () => ['ticket/DEM-1'] });
    await loaded();
    await userEvent.click(remove());
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['1 branche supprimée.']));
  });

  it('keeps the branches that could not go, with why, and counts the others', async () => {
    const backend = setup({
      branch_delete: (args) => {
        if (args.name === 'ticket/DEM-2') throw 'IN_WORKTREE:a1:refacto-auth';
        return null;
      },
    });
    await loaded();
    await userEvent.click(remove());
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(
      '« ticket/DEM-2 » n’a pas pu être supprimée : La branche « ticket/DEM-2 » est utilisée par l’agent refacto-auth, dans son worktree.',
    );
    expect(app.modal).not.toBeNull();
    expect(app.toasts.map((t) => t.text)).toEqual(['2 branches supprimées.']);
    // The list is read again: what is left is what is still merged.
    await waitFor(() => expect(backend.called('branches_merged')).toHaveLength(2));
  });

  it('keeps the boxes as they were after a partial failure, a new branch being ticked', async () => {
    let reads = 0;
    setup({
      branches_merged: () => (reads++ ? ['ticket/DEM-2', 'ticket/DEM-3', 'ticket/DEM-4'] : MERGED),
      branch_delete: (args) => {
        if (args.name === 'ticket/DEM-2') throw 'IN_WORKTREE:a1:refacto-auth';
        return null;
      },
    });
    await loaded();
    // DEM-3 is left alone on purpose.
    await userEvent.click(boxes()[2]);
    await userEvent.click(remove());
    await screen.findByRole('alert');
    await waitFor(() =>
      expect(boxes().map((b) => b.labels?.[0]?.textContent?.trim())).toEqual(['ticket/DEM-2', 'ticket/DEM-3', 'ticket/DEM-4']),
    );
    // Not ticked again for having been read again: what the user unticked stays so; a branch that came since is ticked.
    expect(boxes().map((b) => b.checked)).toEqual([true, false, true]);
    expect(remove()).toHaveTextContent('Supprimer 2 branches');
  });

  it('does not delete twice while it runs', async () => {
    let finish!: (v: null) => void;
    const backend = setup({ branch_delete: () => new Promise((r) => (finish = r)), branches_merged: () => ['ticket/DEM-1'] });
    await loaded();
    await userEvent.click(remove());
    expect(screen.getByRole('button', { name: 'Suppression…' })).toBeDisabled();
    expect(boxes()[0]).toBeDisabled();
    expect(backend.called('branch_delete')).toHaveLength(1);
    finish(null);
    await waitFor(() => expect(app.modal).toBeNull());
  });

  it('says there is nothing to clean up', async () => {
    setup({ branches_merged: () => [] });
    expect(await screen.findByText('Aucune branche mergée à nettoyer.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Supprimer/ })).toBeNull();
    expect(screen.queryByRole('checkbox')).toBeNull();
    await userEvent.click(screen.getAllByRole('button', { name: 'Fermer' }).at(-1)!);
    expect(app.modal).toBeNull();
  });

  it('says it when the branches cannot be read', async () => {
    setup({
      branches_merged: () => {
        throw 'fatal: not a git repository';
      },
    });
    expect(await screen.findByRole('alert')).toHaveTextContent('fatal: not a git repository');
  });

  it('closes on Escape and on « Annuler » without deleting anything', async () => {
    const backend = setup();
    await loaded();
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
    app.modal = { kind: 'mergedBranches', projectId: 'p1' };
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
    expect(backend.called('branch_delete')).toHaveLength(0);
  });
});

describe('MergedBranchesModal in English', () => {
  beforeEach(() => {
    resetApp();
    setLang('en');
  });

  it('writes the window, the counts and the toast in English', async () => {
    const backend = setup();
    expect(screen.getByRole('dialog', { name: 'Merged branches' })).toBeInTheDocument();
    await waitFor(() => expect(screen.getAllByRole('checkbox').length).toBeGreaterThan(0));
    expect(screen.getByText('These local branches are already in the project’s base.')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Branches to delete' })).toBeInTheDocument();
    await userEvent.click(screen.getAllByRole('checkbox')[0]);
    await userEvent.click(screen.getByRole('button', { name: 'Delete 2 branches' }));
    await waitFor(() => expect(app.toasts.map((t) => t.text)).toEqual(['2 branches deleted.']));
    expect(backend.called('branch_delete')).toHaveLength(2);
  });

  it('says there is nothing to clean up in English', async () => {
    setup({ branches_merged: () => [] });
    expect(await screen.findByText('No merged branches to clean up.')).toBeInTheDocument();
  });
});
