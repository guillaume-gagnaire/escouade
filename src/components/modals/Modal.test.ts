import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import Modal from './Modal.svelte';

const body = createRawSnippet(() => ({ render: () => '<div><input aria-label="Nom" /><button>Valider</button></div>' }));

describe('Modal', () => {
  it('takes the focus when it opens and gives it back when it closes', async () => {
    const outside = document.createElement('textarea');
    document.body.appendChild(outside);
    outside.focus();
    const { unmount } = render(Modal, { title: 'Réglages', onclose: () => {}, children: body });
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    expect(dialog.contains(document.activeElement)).toBe(true);
    unmount();
    expect(document.activeElement).toBe(outside);
    outside.remove();
  });

  it('closes on Escape and keeps Tab inside the dialog', async () => {
    const onclose = vi.fn();
    render(Modal, { title: 'Réglages', onclose, children: body });
    const dialog = screen.getByRole('dialog', { name: 'Réglages' });
    for (let i = 0; i < 5; i++) {
      await userEvent.tab();
      expect(dialog.contains(document.activeElement)).toBe(true);
    }
    await fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it('closes on a click outside, not at the end of a selection dragged out of a field', async () => {
    const onclose = vi.fn();
    render(Modal, { title: 'Commit', onclose, children: body });
    const overlay = screen.getByRole('dialog', { name: 'Commit' }).parentElement!;
    await fireEvent.mouseDown(screen.getByRole('textbox', { name: 'Nom' }));
    await fireEvent.mouseUp(overlay);
    await fireEvent.click(overlay);
    expect(onclose).not.toHaveBeenCalled();
    await fireEvent.mouseDown(overlay);
    await fireEvent.click(overlay);
    expect(onclose).toHaveBeenCalledTimes(1);
  });
});
