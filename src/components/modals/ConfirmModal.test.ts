import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import { resetApp } from '../../test/ipc';
import ConfirmModal from './ConfirmModal.svelte';

describe('ConfirmModal', () => {
  beforeEach(() => resetApp());

  it('offers a third choice', async () => {
    const onConfirm = vi.fn();
    const alt = vi.fn();
    app.modal = {
      kind: 'confirm',
      title: 'Enregistrer ?',
      body: 'b',
      confirm: 'Enregistrer',
      onConfirm,
      alt: { label: 'Ne pas enregistrer', onClick: alt },
    };
    render(ConfirmModal, { ...(app.modal as any) });
    await userEvent.click(screen.getByRole('button', { name: 'Ne pas enregistrer' }));
    expect(alt).toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
    expect(app.modal).toBeNull();
  });

  it('goes back where it came from when cancelled', async () => {
    const onCancel = vi.fn(() => {
      app.modal = { kind: 'settings', resume: true };
    });
    app.modal = { kind: 'confirm', title: 'Fermer ?', body: 'b', confirm: 'Fermer', onConfirm: vi.fn(), onCancel };
    render(ConfirmModal, { ...(app.modal as any) });
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(onCancel).toHaveBeenCalled();
    expect(app.modal).toEqual({ kind: 'settings', resume: true });
  });

  it('leaves the modal its confirmation opened', async () => {
    const onConfirm = vi.fn(() => {
      app.modal = { kind: 'settings', resume: true };
    });
    app.modal = { kind: 'confirm', title: 'Fermer ?', body: 'b', confirm: 'Fermer le projet', onConfirm };
    render(ConfirmModal, { ...(app.modal as any) });
    await userEvent.click(screen.getByRole('button', { name: 'Fermer le projet' }));
    expect(app.modal).toEqual({ kind: 'settings', resume: true });
  });

  it('leaves the modal its third choice opened', async () => {
    const alt = vi.fn(() => {
      app.modal = { kind: 'settings', resume: true };
    });
    app.modal = {
      kind: 'confirm',
      title: 'Enregistrer ?',
      body: 'b',
      confirm: 'Enregistrer',
      onConfirm: vi.fn(),
      alt: { label: 'Ne pas enregistrer', onClick: alt },
    };
    render(ConfirmModal, { ...(app.modal as any) });
    await userEvent.click(screen.getByRole('button', { name: 'Ne pas enregistrer' }));
    expect(app.modal).toEqual({ kind: 'settings', resume: true });
  });
});
