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
