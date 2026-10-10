import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../lib/state.svelte';
import { resetApp } from '../test/ipc';
import Toasts from './Toasts.svelte';

describe('Toasts', () => {
  beforeEach(() => resetApp());
  afterEach(() => vi.useRealTimers());

  it('shows a toast’s button, which does what it says and closes the toast', async () => {
    const onClick = vi.fn();
    render(Toasts);
    app.toast('Escouade 1.6.0 est installée.', 'ok', { label: 'Voir les nouveautés', onClick });
    const button = await screen.findByRole('button', { name: 'Voir les nouveautés' });
    // Its text kept as it is (white space shown): nothing added around it.
    expect(button.parentElement!.textContent).toBe('Escouade 1.6.0 est installée.Voir les nouveautés');
    await userEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
    expect(screen.queryByText('Escouade 1.6.0 est installée.')).toBeNull();
  });

  it('leaves the focus where it was when its button is clicked, and its button still works from the keyboard', async () => {
    const onClick = vi.fn();
    render(Toasts);
    // A dialog's field, which the toast's button closing with its toast must not leave without the focus.
    document.body.insertAdjacentHTML('beforeend', '<textarea aria-label="Message"></textarea>');
    const field = screen.getByLabelText('Message');
    field.focus();
    app.toast('La mise à jour vers 1.6.0 n’a pas pu s’installer.', 'error', { label: 'Réessayer', onClick });
    await userEvent.click(await screen.findByRole('button', { name: 'Réessayer' }));
    expect(onClick).toHaveBeenCalledOnce();
    expect(field).toHaveFocus();
    app.toast('La mise à jour vers 1.6.0 n’a pas pu s’installer.', 'error', { label: 'Réessayer', onClick });
    (await screen.findByRole('button', { name: 'Réessayer' })).focus();
    await userEvent.keyboard('{Enter}');
    expect(onClick).toHaveBeenCalledTimes(2);
    field.remove();
  });

  it('keeps a toast with a button long enough to reach it', async () => {
    vi.useFakeTimers();
    app.toast('Pris en compte');
    app.toast('Escouade 1.6.0 est installée.', 'ok', { label: 'Voir les nouveautés', onClick() {} });
    await vi.advanceTimersByTimeAsync(5_000);
    expect(app.toasts.map((t) => t.text)).toEqual(['Escouade 1.6.0 est installée.']);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(app.toasts).toEqual([]);
  });
});
