import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import { resetApp, ticket } from '../../test/ipc';
import TicketForm from './TicketForm.svelte';

const fields = () => [
  screen.getByRole('textbox', { name: 'Titre du ticket' }),
  screen.getByRole('textbox', { name: 'Description' }),
  screen.getByRole('textbox', { name: "Critères d'acceptation" }),
  screen.getByRole('button', { name: '8' }),
];

describe('TicketForm', () => {
  beforeEach(() => resetApp());

  it('is left with Escape from every field, not only the title', async () => {
    const oncancel = vi.fn();
    render(TicketForm, { onsubmit: vi.fn(), oncancel });
    for (const [i, field] of fields().entries()) {
      // Focused, not clicked: a click on a number of loops would be a change.
      field.focus();
      await userEvent.keyboard('{Escape}');
      expect(oncancel, `Escape in field ${i}`).toHaveBeenCalledTimes(i + 1);
    }
    expect(app.modal).toBeNull();
  });

  it('is left at once with « Annuler » when nothing changed', async () => {
    const oncancel = vi.fn();
    render(TicketForm, { ticket: ticket(), onsubmit: vi.fn(), oncancel });
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(oncancel).toHaveBeenCalledTimes(1);
    expect(app.modal).toBeNull();
  });

  it('keeps Enter in the title to submit, and in the textareas to add a line', async () => {
    const onsubmit = vi.fn();
    render(TicketForm, { ticket: ticket(), onsubmit, oncancel: vi.fn() });
    await userEvent.type(screen.getByRole('textbox', { name: "Critères d'acceptation" }), '{Enter}Un de plus');
    expect(onsubmit).not.toHaveBeenCalled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
    expect(onsubmit).toHaveBeenCalledWith({
      title: 'Ajouter le fichier',
      description: '',
      criteria: ['Le fichier existe', 'Tests verts', 'Un de plus'],
      maxLoops: 5,
    });
  });

  describe('with changes', () => {
    const asked = () =>
      expect(app.modal).toMatchObject({
        kind: 'confirm',
        title: 'Abandonner les modifications ?',
        body: 'Ce que tu as saisi dans ce ticket ne sera pas enregistré.',
        confirm: 'Abandonner',
        danger: true,
      });

    it('asks before Escape leaves a new ticket in which something was typed, and leaves once confirmed', async () => {
      const oncancel = vi.fn();
      render(TicketForm, { onsubmit: vi.fn(), oncancel });
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Un début');
      await userEvent.keyboard('{Escape}');
      asked();
      expect(oncancel).not.toHaveBeenCalled();
      await (app.modal as any).onConfirm(false);
      expect(oncancel).toHaveBeenCalledTimes(1);
    });

    it('asks before « Annuler » leaves the form, which stays as it was if the confirmation is dismissed', async () => {
      const oncancel = vi.fn();
      render(TicketForm, { onsubmit: vi.fn(), oncancel });
      await userEvent.type(screen.getByRole('textbox', { name: 'Description' }), 'Pour plus tard');
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      asked();
      expect(oncancel).not.toHaveBeenCalled();
      // The confirmation closes (« Annuler » there): the form and what was typed are still on screen.
      app.modal = null;
      expect(screen.getByRole('textbox', { name: 'Description' })).toHaveValue('Pour plus tard');
      expect(oncancel).not.toHaveBeenCalled();
    });

    it.each([
      ['its title', () => userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), ' bis')],
      ['its description', () => userEvent.type(screen.getByRole('textbox', { name: 'Description' }), 'Un mot')],
      ['its criteria', () => userEvent.type(screen.getByRole('textbox', { name: "Critères d'acceptation" }), '{Enter}Un de plus')],
      ['its loops', () => userEvent.click(screen.getByRole('button', { name: '8' }))],
    ])('asks when %s of a ticket being edited changed', async (_, change) => {
      const oncancel = vi.fn();
      render(TicketForm, { ticket: ticket(), onsubmit: vi.fn(), oncancel });
      await change();
      await userEvent.keyboard('{Escape}');
      asked();
      expect(oncancel).not.toHaveBeenCalled();
    });

    it('does not ask when what was changed is back to what it was', async () => {
      const oncancel = vi.fn();
      render(TicketForm, { ticket: ticket(), onsubmit: vi.fn(), oncancel });
      await userEvent.click(screen.getByRole('button', { name: '8' }));
      await userEvent.click(screen.getByRole('button', { name: '5' }));
      await userEvent.type(screen.getByRole('textbox', { name: 'Description' }), 'x{Backspace}');
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toBeNull();
      expect(oncancel).toHaveBeenCalledTimes(1);
    });

    it('keeps the Escape that asked from reaching the window, whose listener would dismiss the confirmation at once', async () => {
      const onkeydown = vi.fn();
      window.addEventListener('keydown', onkeydown);
      try {
        render(TicketForm, { onsubmit: vi.fn(), oncancel: vi.fn() });
        await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Un début');
        onkeydown.mockClear();
        await userEvent.keyboard('{Escape}');
        asked();
        expect(onkeydown).not.toHaveBeenCalled();
      } finally {
        window.removeEventListener('keydown', onkeydown);
      }
    });
  });
});
