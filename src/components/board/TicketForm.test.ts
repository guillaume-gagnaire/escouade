import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ticket } from '../../test/ipc';
import TicketForm from './TicketForm.svelte';

describe('TicketForm', () => {
  it('is left with Escape from every field, not only the title', async () => {
    const oncancel = vi.fn();
    render(TicketForm, { onsubmit: vi.fn(), oncancel });
    const fields = [
      screen.getByRole('textbox', { name: 'Titre du ticket' }),
      screen.getByRole('textbox', { name: 'Description' }),
      screen.getByRole('textbox', { name: "Critères d'acceptation" }),
      screen.getByRole('button', { name: '8' }),
    ];
    for (const [i, field] of fields.entries()) {
      await userEvent.click(field);
      await userEvent.keyboard('{Escape}');
      expect(oncancel, `Escape in field ${i}`).toHaveBeenCalledTimes(i + 1);
    }
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
});
