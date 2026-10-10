import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// The ticket window on macOS: Cmd instead of Ctrl, written the Mac way.
vi.mock('../../lib/platform', async (original) => {
  const real = await original<typeof import('../../lib/platform')>();
  return {
    ...real,
    IS_MAC: true,
    primaryKey: (e: KeyboardEvent, mac = true) => real.primaryKey(e, mac),
    keyLabel: (shortcut: string, mac = true) => real.keyLabel(shortcut, mac),
  };
});

import { app } from '../../lib/state.svelte';
import { resetApp } from '../../test/ipc';
import TicketForm from './TicketForm.svelte';

describe('TicketForm on macOS', () => {
  beforeEach(() => resetApp());

  function show() {
    const onsubmit = vi.fn();
    app.modal = { kind: 'ticket', projectId: 'p1', onSubmit: onsubmit };
    render(TicketForm, { projectId: 'p1', onsubmit });
    return onsubmit;
  }

  it('adds the ticket with Cmd+Enter, not with Ctrl+Enter, and writes the key the Mac way', async () => {
    const onsubmit = show();
    expect(screen.getByText('⌘Entrée pour ajouter')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Ajouter' })).toHaveAttribute('aria-keyshortcuts', 'Meta+Enter');
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Un ticket');
    await userEvent.click(screen.getByRole('textbox', { name: 'Description' }));
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    expect(onsubmit).not.toHaveBeenCalled();
    await userEvent.keyboard('{Meta>}{Enter}{/Meta}');
    expect(onsubmit).toHaveBeenCalledTimes(1);
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ title: 'Un ticket' }));
  });
});
