import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi, type Mock } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app, type TicketFormDraft } from '../../lib/state.svelte';
import { menu } from '../../lib/menu.svelte';
import type { Ticket, TicketDraft } from '../../lib/types';
import { branchInfo, fakeBackend, resetApp, ticket } from '../../test/ipc';
import TicketForm from './TicketForm.svelte';

interface Props {
  ticket?: Ticket;
  onsubmit?: (d: TicketDraft) => boolean | void | Promise<boolean | void>;
  resume?: TicketFormDraft;
}

/** The form in its window, as App shows it: `app.modal` is the dialog it is, and closing it empties `app.modal`. */
function show(props: Props = {}) {
  const onsubmit = (props.onsubmit ?? vi.fn()) as Mock<(d: TicketDraft) => boolean | void | Promise<boolean | void>>;
  app.modal = { kind: 'ticket', projectId: 'p1', ticket: props.ticket, onSubmit: onsubmit as never, resume: props.resume };
  const view = render(TicketForm, { projectId: 'p1', ticket: props.ticket, onsubmit, resume: props.resume });
  return { ...view, onsubmit };
}

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
    const open = (props: Props = {}) => {
      const backend = fakeBackend({ branch_list: () => LIST });
      const { onsubmit } = show(props);
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

    it('leaves the window’s Escape to the picker while it is open', async () => {
      open();
      await userEvent.click(field());
      menu.open!.items[1].onClick!();
      menu.close();
      await screen.findByRole('dialog', { name: 'Choisir une branche' });
      await fireEvent.keyDown(screen.getByRole('combobox'), { key: 'Escape' });
      await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Choisir une branche' })).toBeNull());
      expect(app.modal).toMatchObject({ kind: 'ticket' });
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

  describe('in its window', () => {
    const title = () => screen.getByRole('textbox', { name: 'Titre du ticket' });
    const ctrlEnter = () => userEvent.keyboard('{Control>}{Enter}{/Control}');

    it('is a large dialog named after what it does, over the whole window', () => {
      show();
      expect(screen.getByRole('dialog', { name: 'Nouveau ticket' })).toHaveAttribute('aria-modal', 'true');
      // Wide enough for long text, tall enough for the description: the dialog's own size, not the column's.
      expect(screen.getByRole('dialog')).toHaveStyle({ width: '760px' });
      expect(screen.getByRole('dialog')).toHaveClass('tall');
    });

    it('names the ticket being edited by its key', () => {
      show({ ticket: ticket() });
      expect(screen.getByRole('dialog', { name: 'Modifier DEM-1' })).toBeInTheDocument();
    });

    it('puts the focus in the title when it opens', () => {
      show();
      expect(title()).toHaveFocus();
    });

    it('gives the description a large field and the criteria a smaller one, both resizable', () => {
      show();
      const [description, criteria] = [
        screen.getByRole('textbox', { name: 'Description' }),
        screen.getByRole('textbox', { name: "Critères d'acceptation" }),
      ];
      expect(description).toHaveAttribute('rows', '12');
      expect(criteria).toHaveAttribute('rows', '6');
      // The title is a field of one line, above the others.
      expect(title().tagName).toBe('INPUT');
    });

    it('names its two big fields with a label of their own, besides the placeholder', () => {
      show();
      expect(screen.getByLabelText('Description')).toBe(screen.getByRole('textbox', { name: 'Description' }));
      expect(screen.getByLabelText("Critères d'acceptation")).toBe(screen.getByRole('textbox', { name: "Critères d'acceptation" }));
    });

    it('keeps a long description whole, over many lines, and sends it', async () => {
      const onsubmit = vi.fn();
      show({ onsubmit });
      const long = Array.from({ length: 80 }, (_, i) => `Ligne ${i + 1} : ` + 'du texte assez long pour passer à la ligne '.repeat(4)).join(
        '\n',
      );
      await userEvent.type(title(), 'Long');
      const field = screen.getByRole('textbox', { name: 'Description' });
      // Typed key by key, 80 lines would take long: pasted, as a long text usually comes.
      await userEvent.click(field);
      await userEvent.paste(long);
      expect(field).toHaveValue(long);
      await userEvent.keyboard('{Control>}{Enter}{/Control}');
      expect(onsubmit).toHaveBeenCalledTimes(1);
      expect(onsubmit.mock.calls[0][0].description).toBe(long.trim());
    });

    it('grows the description with what is typed in it, never shrinks it under the user’s hand', async () => {
      show();
      const field = screen.getByRole('textbox', { name: 'Description' });
      let content = 100;
      Object.defineProperty(field, 'scrollHeight', { configurable: true, get: () => content });
      Object.defineProperty(field, 'clientHeight', { configurable: true, get: () => 100 });
      Object.defineProperty(field, 'offsetHeight', { configurable: true, get: () => 102 });
      await userEvent.type(field, 'a');
      // Fits: left as it is.
      expect(field.style.height).toBe('');
      content = 340;
      await userEvent.type(field, 'b');
      // Its content plus its borders (offset minus client).
      expect(field.style.height).toBe('342px');
    });

    it.each([
      ['the title', () => title()],
      ['the description', () => screen.getByRole('textbox', { name: 'Description' })],
      ['the criteria', () => screen.getByRole('textbox', { name: "Critères d'acceptation" })],
      ['a number of loops', () => screen.getByRole('button', { name: '8' })],
      ['the branch', () => screen.getByRole('button', { name: /^Branche/ })],
    ])('adds the ticket with Ctrl+Enter from %s', async (_, from) => {
      const onsubmit = vi.fn();
      show({ onsubmit });
      await userEvent.type(title(), 'Un ticket');
      from().focus();
      await ctrlEnter();
      expect(onsubmit).toHaveBeenCalledTimes(1);
      expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ title: 'Un ticket' }));
    });

    it('adds the ticket with Ctrl+Enter from the dependencies too', async () => {
      resetApp({ tickets: [ticket({ id: 't2', key: 'DEM-2', title: 'Deux', createdAt: 2 })] });
      const onsubmit = vi.fn();
      show({ onsubmit });
      await userEvent.type(title(), 'Un ticket');
      screen.getByRole('searchbox', { name: 'Rechercher une clé' }).focus();
      await ctrlEnter();
      expect(onsubmit).toHaveBeenCalledTimes(1);
    });

    it('does not add a ticket without a title with Ctrl+Enter, and writes no new line in the description for it', async () => {
      const onsubmit = vi.fn();
      show({ onsubmit });
      const description = screen.getByRole('textbox', { name: 'Description' });
      await userEvent.click(description);
      await ctrlEnter();
      expect(onsubmit).not.toHaveBeenCalled();
      expect(description).toHaveValue('');
    });

    it('does not add the ticket with Enter alone in a textarea, nor with Shift or Alt held besides Ctrl', async () => {
      const onsubmit = vi.fn();
      show({ onsubmit });
      await userEvent.type(title(), 'Un ticket');
      const description = screen.getByRole('textbox', { name: 'Description' });
      await userEvent.click(description);
      await userEvent.keyboard('{Enter}');
      expect(description).toHaveValue('\n');
      await userEvent.keyboard('{Control>}{Shift>}{Enter}{/Shift}{/Control}');
      await userEvent.keyboard('{Alt>}{Control>}{Enter}{/Control}{/Alt}');
      expect(onsubmit).not.toHaveBeenCalled();
    });

    it('is not submitted twice by a Ctrl+Enter pressed again while it saves', async () => {
      let done!: (ok: boolean) => void;
      const onsubmit = vi.fn(() => new Promise<boolean>((r) => (done = r)));
      show({ onsubmit });
      await userEvent.type(title(), 'Un ticket');
      await ctrlEnter();
      await ctrlEnter();
      expect(onsubmit).toHaveBeenCalledTimes(1);
      expect(screen.getByRole('button', { name: 'Ajouter' })).toBeDisabled();
      done(false);
      await waitFor(() => expect(screen.getByRole('button', { name: 'Ajouter' })).toBeEnabled());
    });

    it('says which key adds the ticket, and which saves a ticket that is edited', () => {
      const { unmount } = show();
      expect(screen.getByText('Ctrl+Entrée pour ajouter')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Ajouter' })).toHaveAttribute('aria-keyshortcuts', 'Control+Enter');
      unmount();
      show({ ticket: ticket() });
      expect(screen.getByText('Ctrl+Entrée pour enregistrer')).toBeInTheDocument();
    });

    it('closes once the ticket is saved, and stays open, what was typed in it, when it was not', async () => {
      const onsubmit = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);
      show({ onsubmit });
      await userEvent.type(title(), 'Un ticket');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }));
      expect(onsubmit).toHaveBeenCalledTimes(1);
      await waitFor(() => expect(screen.getByRole('button', { name: 'Ajouter' })).toBeEnabled());
      expect(app.modal).toMatchObject({ kind: 'ticket' });
      expect(title()).toHaveValue('Un ticket');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }));
      await waitFor(() => expect(app.modal).toBeNull());
    });

    it('does not close, once saved, a dialog that took its place meanwhile, nor comes back to be saved twice', async () => {
      let done!: (ok: boolean) => void;
      show({ onsubmit: () => new Promise<boolean>((r) => (done = r)) });
      await userEvent.type(title(), 'Un ticket');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }));
      // « Quitter Escouade ? » takes its place while it saves.
      app.modal = { kind: 'confirm', title: 'Quitter', body: '', confirm: 'Quitter', onConfirm: () => {} };
      done(true);
      await new Promise((r) => setTimeout(r));
      expect(app.modal).toMatchObject({ kind: 'confirm' });
    });

    it('gives the focus back to what had it before, once it is closed', async () => {
      const trigger = document.createElement('button');
      document.body.append(trigger);
      trigger.focus();
      const { unmount } = show();
      expect(title()).toHaveFocus();
      unmount();
      expect(trigger).toHaveFocus();
      trigger.remove();
    });

    it('leaves with its cross as « Annuler » does', async () => {
      show();
      await userEvent.click(screen.getByRole('button', { name: 'Fermer' }));
      expect(app.modal).toBeNull();
    });
  });

  it('is left with Escape from every field, not only the title', async () => {
    show();
    for (const [i, field] of fields().entries()) {
      app.modal = { kind: 'ticket', projectId: 'p1', onSubmit: () => true };
      // Focused, not clicked: a click on a number of loops would be a change.
      field.focus();
      await userEvent.keyboard('{Escape}');
      expect(app.modal, `Escape in field ${i}`).toBeNull();
    }
  });

  it('is left at once with « Annuler » when nothing changed', async () => {
    show({ ticket: ticket() });
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toBeNull();
  });

  it('keeps Enter in the title to submit, and in the textareas to add a line', async () => {
    const onsubmit = vi.fn();
    show({ ticket: ticket(), onsubmit });
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
      show();
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Un début');
      await userEvent.keyboard('{Escape}');
      asked();
      // Agreeing closes the window (App's test), cancelling brings the form back.
      expect(app.modal).toMatchObject({ kind: 'confirm', onCancel: expect.any(Function) });
    });

    it('asks before « Annuler » leaves the form, which comes back as it was if the confirmation is dismissed', async () => {
      const self = (show(), app.modal);
      await userEvent.type(screen.getByRole('textbox', { name: 'Description' }), 'Pour plus tard');
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      asked();
      // The confirmation takes the window's place: the form is gone, its draft kept in the dialog it was.
      cleanup();
      expect(self).toMatchObject({ kind: 'ticket', resume: { description: 'Pour plus tard' } });
      (app.modal as any).onCancel();
      expect(app.modal).toBe(self);
    });

    it('comes back with the draft it had, still asking before it is left', async () => {
      const resume: TicketFormDraft = {
        title: 'Un début',
        description: 'Pour plus tard',
        criteria: 'Un critère\nUn autre',
        maxLoops: 8,
        after: [],
        branch: 'feat/x',
      };
      const onsubmit = vi.fn();
      show({ onsubmit, resume });
      expect(screen.getByRole('textbox', { name: 'Titre du ticket' })).toHaveValue('Un début');
      expect(screen.getByRole('textbox', { name: 'Description' })).toHaveValue('Pour plus tard');
      expect(screen.getByRole('textbox', { name: "Critères d'acceptation" })).toHaveValue('Un critère\nUn autre');
      expect(screen.getByRole('button', { name: '8' })).toHaveAttribute('aria-pressed', 'true');
      expect(screen.getByRole('button', { name: /^Branche/ })).toHaveTextContent('feat/x');
      await userEvent.keyboard('{Escape}');
      asked();
    });

    it('does not ask when the draft it comes back with is the ticket as it was', async () => {
      const resume: TicketFormDraft = {
        title: 'Ajouter le fichier',
        description: '',
        criteria: 'Le fichier existe\nTests verts',
        maxLoops: 5,
        after: [],
        branch: '',
      };
      show({ ticket: ticket(), resume });
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toBeNull();
    });

    it.each([
      ['its title', () => userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), ' bis')],
      ['its description', () => userEvent.type(screen.getByRole('textbox', { name: 'Description' }), 'Un mot')],
      ['its criteria', () => userEvent.type(screen.getByRole('textbox', { name: "Critères d'acceptation" }), '{Enter}Un de plus')],
      ['its loops', () => userEvent.click(screen.getByRole('button', { name: '8' }))],
    ])('asks when %s of a ticket being edited changed', async (_, change) => {
      show({ ticket: ticket() });
      await change();
      await userEvent.keyboard('{Escape}');
      asked();
    });

    it('does not ask when what was changed is back to what it was', async () => {
      show({ ticket: ticket() });
      await userEvent.click(screen.getByRole('button', { name: '8' }));
      await userEvent.click(screen.getByRole('button', { name: '5' }));
      await userEvent.type(screen.getByRole('textbox', { name: 'Description' }), 'x{Backspace}');
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toBeNull();
    });

    it('does not leave while the ticket is being saved', async () => {
      show({ onsubmit: () => new Promise<boolean>(() => {}) });
      await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Un début');
      await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }));
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toMatchObject({ kind: 'ticket' });
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
    show({ onsubmit });
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
    show();
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
    show({ ticket: app.tickets.t5, onsubmit });
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
    show({ ticket: ticket({ after: ['t4', 't2'] }), onsubmit });
    expect(box('DEM-2')).toBeChecked();
    // Unchecked then checked again: nothing changed.
    await userEvent.click(box('DEM-2'));
    await userEvent.click(box('DEM-2'));
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
    app.modal = { kind: 'ticket', projectId: 'p1', onSubmit: () => true };
    await userEvent.click(box('DEM-2'));
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
    // DEM-4, done, is not listed but stays.
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ after: ['t4'] }));
  });

  it('shows no « Après » when no other ticket of the project is left to wait for', () => {
    resetApp({ tickets: [ticket({ id: 't4', column: 'done' }), ticket({ id: 'x1', projectId: 'p2' })] });
    show();
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
    show();
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

  it('names its window and says the key that adds the ticket in English', () => {
    const { unmount } = show();
    expect(screen.getByRole('dialog', { name: 'New ticket' })).toBeInTheDocument();
    expect(screen.getByText('Ctrl+Enter to add')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
    unmount();
    show({ ticket: ticket() });
    expect(screen.getByRole('dialog', { name: 'Edit DEM-1' })).toBeInTheDocument();
    expect(screen.getByText('Ctrl+Enter to save')).toBeInTheDocument();
  });

  it('says Save for a ticket that is edited, and asks in English before throwing a change away', async () => {
    show({ ticket: ticket() });
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
    show({ ticket: ticket() });
    await userEvent.click(within(screen.getByRole('group', { name: 'After' })).getByRole('checkbox', { name: /^DEM-2 / }));
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});
