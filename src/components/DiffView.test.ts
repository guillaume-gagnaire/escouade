import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import type { DiffLine } from '../lib/diff';
import { setLang } from '../lib/i18n';
import DiffView from './DiffView.svelte';

const added = (n: number, from = 1): DiffLine[] =>
  Array.from({ length: n }, (_, i) => ({ kind: 'add', text: `ligne ${from + i}`, oldNo: null, newNo: from + i }));
const rows = (container: HTMLElement) => container.querySelectorAll('.urow').length;
const more = (n: number) => screen.getByRole('button', { name: `Afficher ${n} lignes de plus` });

describe('DiffView with a long diff', () => {
  it('shows up to 1 500 lines in full', () => {
    const { container } = render(DiffView, { lines: added(1500), split: false });
    expect(rows(container)).toBe(1500);
    expect(screen.queryByRole('button', { name: /Afficher/ })).not.toBeInTheDocument();
    expect(screen.queryByText(/Diff volumineux/)).not.toBeInTheDocument();
  });

  it('folds a longer one behind its line count, with nothing drawn yet', () => {
    const { container } = render(DiffView, { lines: added(1501), split: false });
    expect(screen.getByText('Diff volumineux (1501 lignes)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Afficher' })).toBeInTheDocument();
    expect(rows(container)).toBe(0);
  });

  it('draws 500 lines when shown, then 500 more at each click, down to the last few', async () => {
    const { container } = render(DiffView, { lines: added(1620), split: false });
    await userEvent.click(screen.getByRole('button', { name: 'Afficher' }));
    expect(rows(container)).toBe(500);
    expect(screen.queryByText(/Diff volumineux/)).not.toBeInTheDocument();
    expect(screen.getByText('ligne 500')).toBeInTheDocument();
    expect(screen.queryByText('ligne 501')).not.toBeInTheDocument();
    await userEvent.click(more(500));
    expect(rows(container)).toBe(1000);
    await userEvent.click(more(500));
    expect(rows(container)).toBe(1500);
    // The last slice says how many lines it brings, and ends the buttons.
    await userEvent.click(more(120));
    expect(rows(container)).toBe(1620);
    expect(screen.getByText('ligne 1620')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Afficher/ })).not.toBeInTheDocument();
  });

  it('cuts side by side in rows, so that a deletion keeps its replacement across a slice', async () => {
    const ctx = (n: number, from: number): DiffLine[] =>
      Array.from({ length: n }, (_, i) => ({ kind: 'ctx', text: `contexte ${from + i}`, oldNo: from + i, newNo: from + i }));
    // The 500th row pairs a deletion with the addition that follows it.
    const lines: DiffLine[] = [
      ...ctx(499, 1),
      { kind: 'del', text: 'supprimée', oldNo: 500, newNo: null },
      { kind: 'add', text: 'ajoutée', oldNo: null, newNo: 500 },
      ...ctx(1100, 501),
    ];
    const { container } = render(DiffView, { lines, split: true });
    await userEvent.click(screen.getByRole('button', { name: 'Afficher' }));
    expect(container.querySelectorAll('.srow')).toHaveLength(500);
    expect(screen.getByText('supprimée').closest('.srow')).toHaveTextContent('ajoutée');
  });

  it('stays unfolded while the diff is refreshed', async () => {
    const { container, rerender } = render(DiffView, { lines: added(1600), split: false });
    await userEvent.click(screen.getByRole('button', { name: 'Afficher' }));
    await rerender({ lines: added(1700), split: false });
    expect(rows(container)).toBe(500);
    expect(screen.queryByText(/Diff volumineux/)).not.toBeInTheDocument();
  });
});

describe('DiffView with the last lines of a long diff', () => {
  it('says one line more in the singular', async () => {
    render(DiffView, { lines: added(1501), split: false });
    await userEvent.click(screen.getByRole('button', { name: 'Afficher' }));
    await userEvent.click(more(500));
    await userEvent.click(more(500));
    expect(screen.getByRole('button', { name: 'Afficher 1 ligne de plus' })).toBeInTheDocument();
  });
});

describe('DiffView keyboard focus', () => {
  it('goes from « Afficher » to the button of the next slice, and stays on it', async () => {
    render(DiffView, { lines: added(1620), split: false });
    await userEvent.tab();
    expect(screen.getByRole('button', { name: 'Afficher' })).toHaveFocus();
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(more(500)).toHaveFocus());
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(more(500)).toHaveFocus());
  });

  it.each([false, true])('falls on the rows once no slice is left (side by side: %s)', async (split) => {
    const { container } = render(DiffView, { lines: added(1520), split });
    await userEvent.tab();
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(more(500)).toHaveFocus());
    await userEvent.keyboard('{Enter}{Enter}');
    await waitFor(() => expect(screen.getByRole('button', { name: 'Afficher 20 lignes de plus' })).toHaveFocus());
    await userEvent.keyboard('{Enter}');
    expect(screen.queryByRole('button', { name: /Afficher/ })).not.toBeInTheDocument();
    await waitFor(() => expect(container.querySelector('.rows')).toHaveFocus());
  });
});

describe('DiffView with a file too large to be sent', () => {
  it('says so and draws nothing', () => {
    const { container } = render(DiffView, { lines: [], split: false, tooLarge: true });
    expect(screen.getByText('Diff trop volumineux pour être affiché.')).toBeInTheDocument();
    expect(rows(container)).toBe(0);
    expect(screen.queryByRole('button', { name: /Afficher/ })).not.toBeInTheDocument();
  });

  it('says so in side by side too', () => {
    render(DiffView, { lines: [], split: true, tooLarge: true });
    expect(screen.getByText('Diff trop volumineux pour être affiché.')).toBeInTheDocument();
  });
});

describe('DiffView in English', () => {
  beforeEach(() => setLang('en'));

  it('folds a long diff behind its line count, then shows it a slice at a time, in English', async () => {
    const { container } = render(DiffView, { lines: added(1520), split: false });
    expect(screen.getByText('Large diff (1520 lines)')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Show' }));
    expect(rows(container)).toBe(500);
    await userEvent.click(screen.getByRole('button', { name: 'Show 500 more lines' }));
    await userEvent.click(screen.getByRole('button', { name: 'Show 500 more lines' }));
    expect(rows(container)).toBe(1500);
    expect(screen.getByRole('button', { name: 'Show 20 more lines' })).toBeInTheDocument();
  });

  it('says a diff is too large to be displayed, in English', () => {
    render(DiffView, { lines: [], split: false, tooLarge: true });
    expect(screen.getByText('This diff is too large to display.')).toBeInTheDocument();
  });

  it('says one more line in the singular', async () => {
    render(DiffView, { lines: added(1501), split: false });
    await userEvent.click(screen.getByRole('button', { name: 'Show' }));
    await userEvent.click(screen.getByRole('button', { name: 'Show 500 more lines' }));
    await userEvent.click(screen.getByRole('button', { name: 'Show 500 more lines' }));
    expect(screen.getByRole('button', { name: 'Show 1 more line' })).toBeInTheDocument();
  });
});
