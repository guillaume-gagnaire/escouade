import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import type { DiffLine } from '../../lib/diff';
import { app } from '../../lib/state.svelte';
import { resetApp } from '../../test/ipc';
import PatchView from './PatchView.svelte';

const added = (n: number): DiffLine[] =>
  Array.from({ length: n }, (_, i) => ({ kind: 'add', text: `ligne ${i + 1}`, oldNo: null, newNo: i + 1 }));

describe('PatchView', () => {
  beforeEach(() => {
    resetApp();
    app.diffSplit = false;
  });

  it('shows 400 lines, then says how many are left', () => {
    render(PatchView, { lines: added(403) });
    expect(screen.getByText('ligne 400')).toBeInTheDocument();
    expect(screen.queryByText('ligne 401')).not.toBeInTheDocument();
    expect(screen.getByText('… 3 lignes de plus')).toBeInTheDocument();
  });

  it('says one line left in the singular', () => {
    render(PatchView, { lines: added(401) });
    expect(screen.getByText('… 1 ligne de plus')).toBeInTheDocument();
  });

  it('says nothing when everything is shown', () => {
    render(PatchView, { lines: added(400) });
    expect(screen.queryByText(/de plus/)).not.toBeInTheDocument();
  });
});
