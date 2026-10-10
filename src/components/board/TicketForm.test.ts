import { render, screen, within } from '@testing-library/svelte';
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
    render(TicketForm, { projectId: 'p1', onsubmit: vi.fn(), oncancel });
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
    render(TicketForm, { projectId: 'p1', ticket: ticket(), onsubmit: vi.fn(), oncancel });
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(oncancel).toHaveBeenCalledTimes(1);
    expect(app.modal).toBeNull();
  });

  it('keeps Enter in the title to submit, and in the textareas to add a line', async () => {
    const onsubmit = vi.fn();
    render(TicketForm, { projectId: 'p1', ticket: ticket(), onsubmit, oncancel: vi.fn() });
    await userEvent.type(screen.getByRole('textbox', { name: "Critères d'acceptation" }), '{Enter}Un de plus');
    expect(onsubmit).not.toHaveBeenCalled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
    expect(onsubmit).toHaveBeenCalledWith({
      title: 'Ajouter le fichier',
      description: '',
      criteria: ['Le fichier existe', 'Tests verts', 'Un de plus'],
      maxLoops: 5,
      after: [],
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
      render(TicketForm, { projectId: 'p1', onsubmit: vi.fn(), oncancel });
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Un début');
      await userEvent.keyboard('{Escape}');
      asked();
      expect(oncancel).not.toHaveBeenCalled();
      await (app.modal as any).onConfirm(false);
      expect(oncancel).toHaveBeenCalledTimes(1);
    });

    it('asks before « Annuler » leaves the form, which stays as it was if the confirmation is dismissed', async () => {
      const oncancel = vi.fn();
      render(TicketForm, { projectId: 'p1', onsubmit: vi.fn(), oncancel });
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
      render(TicketForm, { projectId: 'p1', ticket: ticket(), onsubmit: vi.fn(), oncancel });
      await change();
      await userEvent.keyboard('{Escape}');
      asked();
      expect(oncancel).not.toHaveBeenCalled();
    });

    it('does not ask when what was changed is back to what it was', async () => {
      const oncancel = vi.fn();
      render(TicketForm, { projectId: 'p1', ticket: ticket(), onsubmit: vi.fn(), oncancel });
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
        render(TicketForm, { projectId: 'p1', onsubmit: vi.fn(), oncancel: vi.fn() });
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

describe('TicketForm « Après »', () => {
  beforeEach(() =>
    resetApp({
      tickets: [
        ticket({ id: 't2', key: 'DEM-2', title: 'Deux', column: 'doing', createdAt: 2 }),
        ticket({ id: 't3', key: 'DEM-3', title: 'Trois', createdAt: 3, after: ['t5'] }),
        ticket({ id: 't4', key: 'DEM-4', title: 'Fini', column: 'done', createdAt: 4 }),
        ticket({ id: 't5', key: 'DEM-5', title: 'Cinq', createdAt: 5 }),
        ticket({ id: 'x1', projectId: 'p2', key: 'AUT-1', title: 'Ailleurs', createdAt: 1 }),
      ],
    }),
  );

  const group = () => screen.getByRole('group', { name: 'Après' });
  const listed = () =>
    within(group())
      .queryAllByRole('checkbox')
      .map((c) => c.closest('label')!.textContent!.replace(/\s+/g, ' ').trim());
  const box = (key: string) => within(group()).getByRole('checkbox', { name: new RegExp(`^${key} `) });
  const search = () => within(group()).getByRole('searchbox', { name: 'Rechercher une clé' });

  it('lists the other tickets of the project not done yet, by key and title, and sends those checked in their order', async () => {
    const onsubmit = vi.fn();
    render(TicketForm, { projectId: 'p1', onsubmit, oncancel: vi.fn() });
    expect(listed()).toEqual(['DEM-2 Deux', 'DEM-3 Trois', 'DEM-5 Cinq']);
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Nouveau');
    await userEvent.click(box('DEM-5'));
    await userEvent.click(box('DEM-2'));
    expect(box('DEM-5')).toBeChecked();
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }));
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ title: 'Nouveau', after: ['t5', 't2'] }));
  });

  it('finds a ticket by its key, an imported one by its key there too, and keeps those checked in sight', async () => {
    app.tickets.t2 = {
      ...app.tickets.t2,
      external: { service: 'jira', id: '1', key: 'ATL-1287', container: 'ATL', url: 'https://x', error: null },
    };
    render(TicketForm, { projectId: 'p1', onsubmit: vi.fn(), oncancel: vi.fn() });
    await userEvent.click(box('DEM-3'));
    await userEvent.type(search(), 'dem-5');
    expect(listed()).toEqual(['DEM-3 Trois', 'DEM-5 Cinq']);
    await userEvent.clear(search());
    await userEvent.type(search(), 'atl-12');
    expect(listed()).toEqual(['DEM-2 Deux', 'DEM-3 Trois']);
    await userEvent.click(box('DEM-3'));
    await userEvent.clear(search());
    await userEvent.type(search(), 'ZZZ');
    expect(listed()).toEqual([]);
    expect(within(group()).getByText('Aucun ticket pour cette clé')).toBeInTheDocument();
  });

  it('refuses a ticket that already waits for this one, directly or not, and lists not the ticket itself', async () => {
    const onsubmit = vi.fn();
    render(TicketForm, { projectId: 'p1', ticket: app.tickets.t5, onsubmit, oncancel: vi.fn() });
    expect(listed()).toEqual(['DEM-2 Deux', 'DEM-3 Trois']);
    await userEvent.click(box('DEM-3'));
    expect(within(group()).getByRole('alert')).toHaveTextContent('DEM-3 attend déjà DEM-5 (directement ou non).');
    expect(box('DEM-3')).not.toBeChecked();
    // Another one is taken, and the refusal goes.
    await userEvent.click(box('DEM-2'));
    expect(box('DEM-2')).toBeChecked();
    expect(within(group()).queryByRole('alert')).not.toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ after: ['t2'] }));
  });

  it('keeps what an edited ticket comes after, those done included, and asks before leaving once that changed', async () => {
    const onsubmit = vi.fn();
    const oncancel = vi.fn();
    render(TicketForm, { projectId: 'p1', ticket: ticket({ after: ['t4', 't2'] }), onsubmit, oncancel });
    expect(box('DEM-2')).toBeChecked();
    // Unchecked then checked again: nothing changed.
    await userEvent.click(box('DEM-2'));
    await userEvent.click(box('DEM-2'));
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
    expect(oncancel).toHaveBeenCalledTimes(1);
    await userEvent.click(box('DEM-2'));
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
    // DEM-4, done, is not listed but stays.
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ after: ['t4'] }));
  });

  it('shows no « Après » when no other ticket of the project is left to wait for', () => {
    resetApp({ tickets: [ticket({ id: 't4', column: 'done' }), ticket({ id: 'x1', projectId: 'p2' })] });
    render(TicketForm, { projectId: 'p1', onsubmit: vi.fn(), oncancel: vi.fn() });
    expect(screen.queryByRole('group', { name: 'Après' })).not.toBeInTheDocument();
  });
});
