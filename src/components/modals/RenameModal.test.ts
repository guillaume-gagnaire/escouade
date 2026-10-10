import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import RenameModal from './RenameModal.svelte';

describe('RenameModal', () => {
  it('renames with the trimmed value, then closes', async () => {
    const onSubmit = vi.fn();
    app.modal = { kind: 'rename', title: 'Renommer le projet', value: 'demo', onSubmit };
    render(RenameModal, { title: 'Renommer le projet', value: 'demo', onSubmit });
    const field = screen.getByDisplayValue('demo');
    await userEvent.clear(field);
    await userEvent.type(field, '  studio  ');
    await userEvent.click(screen.getByRole('button', { name: 'Renommer' }));
    expect(onSubmit).toHaveBeenCalledWith('studio');
    expect(app.modal).toBeNull();
  });

  it('writes its buttons in English', async () => {
    setLang('en');
    const onSubmit = vi.fn();
    app.modal = { kind: 'rename', title: 'Rename the project', value: 'demo', onSubmit };
    render(RenameModal, { title: 'Rename the project', value: 'demo', onSubmit });
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rename' }));
    expect(onSubmit).toHaveBeenCalledWith('demo');
  });
});
