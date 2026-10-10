import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// On macOS, files are linked as folders are: nothing is copied in their place.
vi.mock('../../lib/platform', async (original) => {
  const real = await original<typeof import('../../lib/platform')>();
  return { ...real, IS_MAC: true };
});
vi.mock('../../lib/terminals', () => ({ openAccountLogin: vi.fn(), mountTerminal: vi.fn(), disposeTerminal: vi.fn() }));

import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import AccountModal from './AccountModal.svelte';

describe('AccountModal on macOS', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'account' };
  });

  it('links the files too', async () => {
    fakeBackend({ account_shareable: () => ['settings.json', 'CLAUDE.md', 'skills'] });
    render(AccountModal, {});
    const share = await screen.findByRole('group', { name: 'Partager avec Principal' });
    expect(within(share).getByRole('radio', { name: 'Lier (un changement vaut pour les deux comptes)' })).toBeChecked();
    expect(within(share).getByRole('checkbox', { name: 'settings.json' })).toBeChecked();
    expect(within(share).queryByText('copiés (un lien de fichier demande des droits d’administrateur)')).not.toBeInTheDocument();
  });
});
