import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { menu } from '../../lib/menu.svelte';
import { branchInfo, fakeBackend, resetApp, ticket } from '../../test/ipc';
import TicketForm from './TicketForm.svelte';

const fields = () => [
  screen.getByRole('textbox', { name: 'Titre du ticket' }),
  screen.getByRole('textbox', { name: 'Description' }),
  screen.getByRole('textbox', { name: "Critères d'acceptation" }),
  screen.getByRole('button', { name: '8' }),
];

describe('TicketForm', () => {
  beforeEach(() => resetApp());

  describe('its branch', () => {
    const LIST = [
      branchInfo({ name: 'main', current: true, worktree: 'C:\\code\\demo-api' }),
      branchInfo({ name: 'feat/login' }),
      branchInfo({ name: 'origin/hotfix', remote: true }),
    ];
    const field = () => screen.getByRole('button', { name: /^Branche/ });
    /** The form, the project having `LIST` for branches. */
    const open = (props: Record<string, unknown> = {}) => {
      const backend = fakeBackend({ branch_list: () => LIST });
      const onsubmit = vi.fn();
      render(TicketForm, { projectId: 'p1', onsubmit, oncancel: vi.fn(), ...props });
      return { backend, onsubmit };
    };
    /** Chooses a branch to take up, through the menu and the picker. */
    async function takeUp(name: string) {
      await userEvent.click(field());
      menu.open!.items.find((i) => i.label === 'Reprendre une branche existante…')!.onClick!();
      menu.close();
      const picker = await screen.findByRole('dialog', { name: 'Choisir une branche' });
      await userEvent.click(await within(picker).findByText(name));
    }

    it('says a new ticket gets a branch of its own, named after its key, which it has not got yet', () => {
      open();
      expect(field()).toHaveTextContent('Nouvelle branche ticket/<clé>');
    });

    it('names the branch of a ticket that exists by its key', () => {
      open({ ticket: ticket({ key: 'DEM-12' }) });
      expect(field()).toHaveTextContent('Nouvelle branche ticket/dem-12');
    });

    it('offers to make the ticket’s own branch or to take up one that exists', async () => {
      open();
      await userEvent.click(field());
      expect(field()).toHaveAttribute('aria-haspopup', 'menu');
      expect(menu.open!.items.map((i) => i.label)).toEqual(['Nouvelle branche ticket/<clé>', 'Reprendre une branche existante…']);
    });

    it('says on the entry that takes a branch up that Escouade never deletes it', async () => {
      open();
      await userEvent.click(field());
      expect(menu.open!.items.map((i) => i.title)).toEqual([
        undefined,
        'Escouade ne supprime jamais cette branche, même une fois le ticket validé.',
      ]);
    });

    it('sends the branch chosen in the picker with the ticket, and shows it', async () => {
      const { onsubmit } = open();
      await takeUp('feat/login');
      await waitFor(() => expect(field()).toHaveTextContent('feat/login'));
      expect(screen.queryByRole('dialog', { name: 'Choisir une branche' })).toBeNull();
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Reprendre{Enter}');
      expect(onsubmit).toHaveBeenCalledWith({
        title: 'Reprendre',
        description: '',
        criteria: [],
        maxLoops: 5,
        after: [],
        branch: 'feat/login',
      });
    });

    it('takes a remote branch under its own name', async () => {
      const { onsubmit } = open();
      await takeUp('origin/hotfix');
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Reprendre{Enter}');
      expect(onsubmit.mock.calls[0][0].branch).toBe('origin/hotfix');
    });

    it('cannot take up the branch of the project’s folder, which the picker says', async () => {
      open();
      await userEvent.click(field());
      menu.open!.items[1].onClick!();
      menu.close();
      const picker = await screen.findByRole('dialog', { name: 'Choisir une branche' });
      const main = (await within(picker).findByText('main')).closest('[role="option"]')!;
      expect(main).toHaveAttribute('aria-disabled', 'true');
    });

    it('sends nothing about the branch of a ticket that keeps its own, but gives it up when it had another', async () => {
      const { onsubmit } = open();
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Seul{Enter}');
      expect(onsubmit.mock.calls[0][0]).not.toHaveProperty('branch');
    });

    it('shows the branch an edited ticket takes up, and sends the ticket’s own again when it is given up', async () => {
      const { onsubmit } = open({ ticket: ticket({ branch: 'feat/login' }) });
      expect(field()).toHaveTextContent('feat/login');
      // Kept: sent again as it is.
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
      expect(onsubmit.mock.calls[0][0].branch).toBe('feat/login');
      await userEvent.click(field());
      menu.open!.items[0].onClick!();
      menu.close();
      await waitFor(() => expect(field()).toHaveTextContent('Nouvelle branche ticket/dem-1'));
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
      expect(onsubmit.mock.calls[1][0].branch).toBe('');
    });

    it('asks before leaving when only the branch changed', async () => {
      open();
      await takeUp('feat/login');
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
    });

    it('leaves the form’s Escape to the picker while it is open', async () => {
      const oncancel = vi.fn();
      open({ oncancel });
      await userEvent.click(field());
      menu.open!.items[1].onClick!();
      menu.close();
      await screen.findByRole('dialog', { name: 'Choisir une branche' });
      await fireEvent.keyDown(screen.getByRole('combobox'), { key: 'Escape' });
      await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Choisir une branche' })).toBeNull());
      expect(oncancel).not.toHaveBeenCalled();
    });

    it('is written in English', async () => {
      setLang('en');
      open();
      const english = screen.getByRole('button', { name: /^Branch/ });
      expect(english).toHaveTextContent('New branch ticket/<key>');
      await userEvent.click(english);
      expect(menu.open!.items.map((i) => i.label)).toEqual(['New branch ticket/<key>', 'Take up an existing branch…']);
      expect(menu.open!.items[1].title).toBe('Escouade never deletes this branch, even once the ticket is approved.');
    });
  });

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

describe('TicketForm in English', () => {
  beforeEach(() => {
    resetApp({
      tickets: [ticket({ id: 't2', key: 'DEM-2', title: 'Deux', column: 'doing', createdAt: 2 })],
    });
    setLang('en');
  });

  it('writes its fields, its loops and its dependencies in English', async () => {
    render(TicketForm, { projectId: 'p1', onsubmit: vi.fn(), oncancel: vi.fn() });
    expect(screen.getByRole('textbox', { name: 'Ticket title' })).toHaveAttribute('placeholder', 'Ticket title');
    expect(screen.getByRole('textbox', { name: 'Description' })).toHaveAttribute('placeholder', 'Description (optional)');
    expect(screen.getByRole('textbox', { name: 'Acceptance criteria' })).toHaveAttribute(
      'placeholder',
      'Acceptance criteria, one per line',
    );
    expect(screen.getByRole('group', { name: 'Max loops' })).toBeInTheDocument();
    const after = screen.getByRole('group', { name: 'After' });
    await userEvent.type(within(after).getByRole('searchbox', { name: 'Search for a key' }), 'ZZZ');
    expect(within(after).getByText('No tickets for this key')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add' })).toBeInTheDocument();
  });

  it('says Save for a ticket that is edited, and asks in English before throwing a change away', async () => {
    render(TicketForm, { ticket: ticket(), projectId: 'p1', onsubmit: vi.fn(), oncancel: vi.fn() });
    expect(screen.getByRole('button', { name: 'Save' })).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Ticket title' }), '!');
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(app.modal).toMatchObject({
      kind: 'confirm',
      title: 'Discard your changes?',
      body: 'What you typed in this ticket will not be saved.',
      confirm: 'Discard',
    });
  });

  it('refuses a ticket that already waits for this one, in English', async () => {
    resetApp({
      tickets: [ticket({ id: 't2', key: 'DEM-2', title: 'Deux', createdAt: 2, after: ['t1'] })],
    });
    render(TicketForm, { ticket: ticket(), projectId: 'p1', onsubmit: vi.fn(), oncancel: vi.fn() });
    await userEvent.click(within(screen.getByRole('group', { name: 'After' })).getByRole('checkbox', { name: /^DEM-2 / }));
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});
