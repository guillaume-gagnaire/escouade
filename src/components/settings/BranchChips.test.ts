import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend, resetApp } from '../../test/ipc';
import BranchChips from './BranchChips.svelte';

/** The height of a row of chips, its gap included, and of the list, as the test lays them out (jsdom lays nothing out). */
const ROW = 34;
const LIST = 114;

describe('BranchChips', () => {
  beforeEach(() => {
    resetApp();
    // One chip per row.
    vi.spyOn(HTMLElement.prototype, 'offsetTop', 'get').mockImplementation(function (this: HTMLElement) {
      return this.classList.contains('chip') ? Array.from(this.parentElement!.children).indexOf(this) * ROW : 0;
    });
    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function (this: HTMLElement) {
      return this.classList.contains('chip') ? 28 : LIST;
    });
    vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(LIST);
  });
  afterEach(() => vi.restoreAllMocks());

  const many = Array.from({ length: 40 }, (_, i) => `feat/${String(i).padStart(2, '0')}`);

  it('lists the project’s local branches as chips, and tells the one picked', async () => {
    const backend = fakeBackend({ git_branches: () => ['main', 'release'] });
    const onpick = vi.fn();
    render(BranchChips, { projectId: 'p1', value: 'main', label: 'Branche cible', onpick });
    const group = screen.getByRole('group', { name: 'Branche cible' });
    // Before the branches are read: the one chosen.
    expect(
      within(group)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['⎇ main']);
    await userEvent.click(await within(group).findByRole('button', { name: '⎇ release' }));
    expect(onpick).toHaveBeenCalledWith('release');
    expect(within(group).getByRole('button', { name: '⎇ main' })).toHaveAttribute('aria-pressed', 'true');
    expect(backend.called('git_branches')).toEqual([{ cmd: 'git_branches', args: { projectId: 'p1' } }]);
  });

  it('puts a choice that is no branch ahead of them, chosen while none is named', async () => {
    fakeBackend({ git_branches: () => ['main'] });
    const onpick = vi.fn();
    render(BranchChips, { projectId: 'p1', value: '', label: 'Branche de départ', first: 'Branche courante', onpick });
    const group = screen.getByRole('group', { name: 'Branche de départ' });
    await within(group).findByRole('button', { name: '⎇ main' });
    const first = within(group).getAllByRole('button')[0];
    expect(first).toHaveTextContent('Branche courante');
    expect(first).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(within(group).getByRole('button', { name: '⎇ main' }));
    await userEvent.click(first);
    expect(onpick.mock.calls).toEqual([['main'], ['']]);
  });

  it('scrolls its own list to the branch chosen, rows below the first ones', async () => {
    fakeBackend({ git_branches: () => many });
    render(BranchChips, { projectId: 'p1', value: 'feat/30', label: 'Branche cible', onpick: () => {} });
    const group = screen.getByRole('group', { name: 'Branche cible' });
    await within(group).findByRole('button', { name: '⎇ feat/39' });
    await vi.waitFor(() => expect(group.scrollTop).toBe(30 * ROW));
  });

  it('leaves the list where it is when the branch chosen is in sight', async () => {
    fakeBackend({ git_branches: () => many });
    render(BranchChips, { projectId: 'p1', value: 'feat/01', label: 'Branche cible', onpick: () => {} });
    const group = screen.getByRole('group', { name: 'Branche cible' });
    await within(group).findByRole('button', { name: '⎇ feat/39' });
    await new Promise((r) => setTimeout(r, 0));
    expect(group.scrollTop).toBe(0);
  });

  it('reads the branches of another project when it is given one', async () => {
    const backend = fakeBackend({ git_branches: (a: any) => (a.projectId === 'p1' ? ['main'] : ['trunk']) });
    const view = render(BranchChips, { projectId: 'p1', value: 'main', label: 'Branche cible', onpick: () => {} });
    await vi.waitFor(() => expect(backend.called('git_branches')).toHaveLength(1));
    await view.rerender({ projectId: 'p2', value: 'trunk' });
    expect(await screen.findByRole('button', { name: '⎇ trunk' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByRole('button', { name: '⎇ main' })).toBeNull();
    // Each project's branches are read once.
    await view.rerender({ projectId: 'p1', value: 'main' });
    expect(screen.getByRole('button', { name: '⎇ main' })).toBeInTheDocument();
    expect(backend.called('git_branches')).toHaveLength(2);
  });
});
