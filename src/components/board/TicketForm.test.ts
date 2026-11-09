import { cleanup, createEvent, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app, type TicketFormDraft } from '../../lib/state.svelte';
import { menu } from '../../lib/menu.svelte';
import type { Account, AttachedFile, Ticket, TicketDraft, ToldModel } from '../../lib/types';
import { BARE_ENGINE, branchInfo, CLAUDE_ENGINE, fakeBackend, project, resetApp, ticket, usableEngines } from '../../test/ipc';
import TicketForm from './TicketForm.svelte';

/** The group « Après » (`name`), its popover opened: its search and the tickets to check are in it. */
function openAfter(name: string) {
  const group = screen.getByRole('group', { name });
  const button = within(group).getByRole('button', { name: new RegExp(`^${name} `) });
  if (button.getAttribute('aria-expanded') !== 'true') {
    button.click();
    flushSync();
  }
  return group;
}

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
        engine: null,
        account: '',
        model: '',
        effort: '',
        // A new ticket says whether it is shared (ESC-190).
        shared: false,
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
    /** The window's height: jsdom's is 768, which is a short window for this form. */
    const heightOf = (px: number) => {
      window.innerHeight = px;
      window.dispatchEvent(new Event('resize'));
    };
    beforeEach(() => heightOf(1000));
    afterEach(() => heightOf(768));
    const title = () => screen.getByRole('textbox', { name: 'Titre du ticket' });
    const ctrlEnter = () => userEvent.keyboard('{Control>}{Enter}{/Control}');

    it('is a large dialog named after what it does, over the whole window', () => {
      show();
      expect(screen.getByRole('dialog', { name: 'Nouveau ticket' })).toHaveAttribute('aria-modal', 'true');
      // Wide enough for long text, tall enough for the description: the dialog's own size, not the column's.
      expect(screen.getByRole('dialog').style.getPropertyValue('--w')).toBe('760px');
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

    it('gives the criteria a field of their own rows, the description the room left (its style)', () => {
      show();
      const [description, criteria] = [
        screen.getByRole('textbox', { name: 'Description' }),
        screen.getByRole('textbox', { name: "Critères d'acceptation" }),
      ];
      // No rows of its own, nor a height set by its text (ESC-189): the window's room decides, a long text scrolls in it.
      expect(description).not.toHaveAttribute('rows');
      expect(criteria).toHaveAttribute('rows', '6');
      // The title is a field of one line, above the others.
      expect(title().tagName).toBe('INPUT');
    });

    it('takes a row from the criteria in a short window, so that loops, branch and dependencies show at once', () => {
      heightOf(700);
      show();
      const criteria = screen.getByRole('textbox', { name: "Critères d'acceptation" });
      expect(criteria).toHaveAttribute('rows', '5');
      // Taller again with the window.
      heightOf(900);
      return waitFor(() => expect(criteria).toHaveAttribute('rows', '6'));
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

    it('leaves the height of the description to the window, whatever its text (ESC-189)', async () => {
      show();
      const field = screen.getByRole('textbox', { name: 'Description' });
      Object.defineProperty(field, 'scrollHeight', { configurable: true, get: () => 4000 });
      Object.defineProperty(field, 'clientHeight', { configurable: true, get: () => 100 });
      await userEvent.type(field, 'ab');
      // A height of its text would push the fields under it out of the window, or draw it over them.
      expect(field.style.height).toBe('');
      expect(field.style.minHeight).toBe('');
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
      within(openAfter('Après')).getByRole('searchbox', { name: 'Rechercher une clé' }).focus();
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

    it('adds the ticket once for a Ctrl+Enter held down, whose repeats would add the next one', async () => {
      const onsubmit = vi.fn();
      show({ onsubmit });
      await userEvent.type(title(), 'Un ticket');
      const description = screen.getByRole('textbox', { name: 'Description' });
      description.focus();
      // A repeat is held back, and writes no line either.
      const repeat = await fireEvent.keyDown(description, { key: 'Enter', ctrlKey: true, repeat: true });
      expect(repeat).toBe(false);
      expect(onsubmit).not.toHaveBeenCalled();
      await fireEvent.keyDown(description, { key: 'Enter', ctrlKey: true });
      expect(onsubmit).toHaveBeenCalledTimes(1);
    });

    it('adds the ticket from the title with Enter alone, not for a key held down nor one that confirms an input method', async () => {
      const onsubmit = vi.fn();
      show({ onsubmit });
      await userEvent.type(title(), 'Un ticket');
      await fireEvent.keyDown(title(), { key: 'Enter', repeat: true });
      await fireEvent.keyDown(title(), { key: 'Enter', isComposing: true });
      expect(onsubmit).not.toHaveBeenCalled();
      await fireEvent.keyDown(title(), { key: 'Enter' });
      expect(onsubmit).toHaveBeenCalledTimes(1);
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
      expect(document.querySelector('.hint')).toHaveTextContent('Ctrl+Entrée pour ajouter');
      // The key drawn as one.
      expect(document.querySelector('.hint kbd')).toHaveTextContent('Ctrl+Entrée');
      expect(screen.getByRole('button', { name: 'Ajouter' })).toHaveAttribute('aria-keyshortcuts', 'Control+Enter');
      unmount();
      show({ ticket: ticket() });
      expect(document.querySelector('.hint')).toHaveTextContent('Ctrl+Entrée pour enregistrer');
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
      engine: null,
      account: '',
      model: '',
      effort: '',
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

describe('TicketForm, its images', () => {
  beforeEach(() => resetApp());
  afterEach(() => setLang('fr'));
  const png = [137, 80, 78, 71, 13, 10, 26, 10];
  const b64 = btoa(String.fromCharCode(...png));
  const capture = (name = 'capture.png', type = 'image/png') => new File([new Uint8Array(png)], name, { type });
  const paste = (target: HTMLElement, clipboardData: { files: File[]; types: string[] }) => {
    const e = createEvent.paste(target, { clipboardData });
    fireEvent(target, e);
    return e.defaultPrevented;
  };
  const drop = (target: HTMLElement, files: File[]) => {
    const e = createEvent.drop(target, { dataTransfer: { files, types: ['Files'] } });
    fireEvent(target, e);
    return e.defaultPrevented;
  };
  const title = () => screen.getByRole('textbox', { name: 'Titre du ticket' });
  const description = () => screen.getByRole('textbox', { name: 'Description' });
  /** The images of a ticket that has two, and what the backend reads of them. */
  const IMAGES = [
    { file: 'a1.png', name: 'capture.png', mediaType: 'image/png' },
    { file: 'b2.gif', name: 'schema.gif', mediaType: 'image/gif' },
  ];
  const READ = [
    { ...IMAGES[0], data: b64 },
    { ...IMAGES[1], data: 'R0lG' },
  ];

  it('shows an image pasted in the description under it, and adds the ticket with it', async () => {
    fakeBackend();
    const { onsubmit } = show({ onsubmit: vi.fn(() => true) });
    await userEvent.type(title(), 'Le bouton');
    expect(paste(description(), { files: [capture()], types: ['Files'] })).toBe(true);
    const list = await screen.findByRole('list', { name: 'Images du ticket' });
    expect(within(list).getByAltText('capture.png')).toHaveAttribute('src', `data:image/png;base64,${b64}`);
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter' }));
    expect(onsubmit).toHaveBeenCalledWith(
      expect.objectContaining({ title: 'Le bouton', images: [{ name: 'capture.png', mediaType: 'image/png', file: '', data: b64 }] }),
    );
  });

  it('takes an image pasted in any field of the window, or dropped on it', async () => {
    const backend = fakeBackend();
    show();
    expect(paste(title(), { files: [capture('a.png')], types: ['Files'] })).toBe(true);
    expect(drop(screen.getByRole('textbox', { name: "Critères d'acceptation" }), [capture('b.gif', 'image/gif')])).toBe(true);
    expect(await screen.findByAltText('a.png (1/2)')).toBeInTheDocument();
    expect(await screen.findByAltText('b.gif (2/2)')).toBeInTheDocument();
    expect(backend.called('clipboard_image')).toHaveLength(0);
  });

  it('attaches an image picked from the file dialog, which offers every file', async () => {
    fakeBackend();
    show();
    const input = screen.getByLabelText('Joindre un fichier', { selector: 'input' });
    expect(input).not.toHaveAttribute('accept');
    await userEvent.upload(input, capture('maquette.webp', 'image/webp'));
    expect(await screen.findByAltText('maquette.webp')).toBeInTheDocument();
  });

  it('leaves the text pasted to its field, even with an image of it (the cells of a spreadsheet)', async () => {
    const backend = fakeBackend();
    show();
    expect(paste(description(), { files: [], types: ['text/plain'] })).toBe(false);
    expect(paste(description(), { files: [capture()], types: ['Files', 'text/plain', 'text/html'] })).toBe(false);
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByRole('list', { name: 'Images du ticket' })).not.toBeInTheDocument();
    expect(backend.called('clipboard_image')).toHaveLength(0);
  });

  it('takes an image pasted while the window itself has the focus, or dropped on its head or foot', async () => {
    fakeBackend();
    show();
    const dialog = screen.getByRole('dialog');
    // A click on a thumbnail or between the fields gives the focus to the window.
    dialog.focus();
    expect(paste(dialog, { files: [capture('a.png')], types: ['Files'] })).toBe(true);
    expect(drop(screen.getByRole('button', { name: 'Annuler' }), [capture('b.png')])).toBe(true);
    expect(drop(screen.getByText('Nouveau ticket'), [capture('c.png')])).toBe(true);
    for (const name of ['a.png (1/3)', 'b.png (2/3)', 'c.png (3/3)']) expect(await screen.findByAltText(name)).toBeInTheDocument();
  });

  it('says which images of the ticket are no longer in its folder', async () => {
    fakeBackend({ ticket_images: () => READ.slice(0, 1) });
    show({ ticket: ticket({ images: IMAGES }) });
    expect(await screen.findByAltText('capture.png')).toBeInTheDocument();
    await waitFor(() =>
      expect(app.toasts.at(-1)?.text).toBe("schema.gif n'est plus dans le dossier du ticket : il en est retiré à l'enregistrement."),
    );
    expect(screen.queryByRole('img', { name: /schema.gif/ })).not.toBeInTheDocument();
  });

  it('leaves without asking when only an image gone from the ticket’s folder was dropped: the user changed nothing', async () => {
    fakeBackend({ ticket_images: () => READ.slice(0, 1) });
    show({ ticket: ticket({ images: IMAGES }) });
    await waitFor(() => expect(app.toasts.at(-1)?.text).toContain('schema.gif'));
    await waitFor(() => expect(screen.queryByRole('img', { name: /schema.gif/ })).not.toBeInTheDocument());
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
  });

  it('takes an image gone from the ticket’s folder off the ticket at any save, as its toast says', async () => {
    fakeBackend({ ticket_images: () => READ.slice(0, 1) });
    const { onsubmit } = show({ ticket: ticket({ images: IMAGES }), onsubmit: vi.fn(() => true) });
    await waitFor(() => expect(app.toasts.at(-1)?.text).toContain('schema.gif'));
    await userEvent.type(title(), ' bis');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(onsubmit).toHaveBeenCalledWith(
      expect.objectContaining({ images: [{ name: 'capture.png', mediaType: 'image/png', file: 'a1.png', data: '' }] }),
    );
  });

  it('takes the image WebKitGTK leaves out of a paste (Linux) from the system clipboard, and is not added meanwhile', async () => {
    let give: (png: ArrayBuffer) => void = () => {};
    fakeBackend({ clipboard_image: () => new Promise((r) => (give = r)) });
    const { onsubmit } = show();
    await userEvent.type(title(), 'Le bouton');
    paste(description(), { files: [], types: [] });
    expect(screen.getByRole('button', { name: 'Ajouter' })).toBeDisabled();
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    expect(onsubmit).not.toHaveBeenCalled();
    give(new Uint8Array(png).buffer);
    expect(await screen.findByAltText('image.png')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Ajouter' })).toBeEnabled();
  });

  it('keeps 20 images of a paste of 21, and says the 21st was not added', async () => {
    fakeBackend();
    show();
    paste(description(), { files: Array.from({ length: 21 }, (_, n) => capture(`image-${n + 1}.png`)), types: ['Files'] });
    await waitFor(() => expect(app.toasts.at(-1)?.text).toBe("image-21.png n'est pas ajoutée : un ticket prend 20 images au plus."));
    const list = screen.getByRole('list', { name: 'Images du ticket' });
    expect(within(list).getAllByRole('img')).toHaveLength(20);
    expect(screen.queryByAltText(/image-21\.png/)).not.toBeInTheDocument();
  });

  it('takes the room of its images from the description by its style, not by its rows (ESC-189)', async () => {
    fakeBackend();
    window.innerHeight = 700;
    window.dispatchEvent(new Event('resize'));
    try {
      show();
      paste(description(), { files: [capture()], types: ['Files'] });
      await screen.findByAltText('capture.png');
      expect(description()).not.toHaveAttribute('rows');
      expect(description().style.height).toBe('');
      await userEvent.click(screen.getByRole('button', { name: 'Retirer capture.png' }));
      expect(screen.queryByAltText('capture.png')).not.toBeInTheDocument();
    } finally {
      window.innerHeight = 768;
      window.dispatchEvent(new Event('resize'));
    }
  });

  it('takes an image away with its cross', async () => {
    fakeBackend();
    show();
    paste(description(), { files: [capture('a.png'), capture('b.png')], types: ['Files'] });
    await screen.findByAltText('b.png (2/2)');
    await userEvent.click(screen.getByRole('button', { name: 'Retirer a.png (1/2)' }));
    expect(screen.queryByAltText(/a\.png/)).not.toBeInTheDocument();
    expect(screen.getByAltText('b.png')).toBeInTheDocument();
  });

  it('tells screenshots all named image.png apart by their rank, which goes once only one is left', async () => {
    fakeBackend();
    show();
    paste(description(), { files: [capture('image.png'), capture('image.png')], types: ['Files'] });
    const list = await screen.findByRole('list', { name: 'Images du ticket' });
    await waitFor(() => expect(within(list).getAllByRole('img')).toHaveLength(2));
    expect(
      within(list)
        .getAllByRole('img')
        .map((i) => i.getAttribute('alt')),
    ).toEqual(['image.png (1/2)', 'image.png (2/2)']);
    expect(
      within(list)
        .getAllByRole('button')
        .map((b) => b.getAttribute('aria-label')),
    ).toEqual(['Retirer image.png (1/2)', 'Retirer image.png (2/2)']);
    // The tooltip names it as well.
    expect(within(list).getByAltText('image.png (2/2)').closest('li')).toHaveAttribute('data-tip', 'image.png (2/2)');
    await userEvent.click(screen.getByRole('button', { name: 'Retirer image.png (1/2)' }));
    expect(within(list).getByAltText('image.png')).toBeInTheDocument();
    expect(within(list).getByRole('button', { name: 'Retirer image.png' })).toBeInTheDocument();
    expect(within(list).queryByAltText(/\(\d\/\d\)/)).not.toBeInTheDocument();
  });

  it('keeps the focus in the form when an image is taken away: on the next one, else the description', async () => {
    fakeBackend();
    const { onsubmit } = show({ onsubmit: vi.fn(() => true) });
    await userEvent.type(title(), 'Le bouton');
    paste(description(), { files: [capture('a.png'), capture('b.png')], types: ['Files'] });
    await screen.findByAltText('b.png (2/2)');
    await userEvent.click(screen.getByRole('button', { name: 'Retirer a.png (1/2)' }));
    expect(screen.getByRole('button', { name: 'Retirer b.png' })).toHaveFocus();
    await userEvent.click(screen.getByRole('button', { name: 'Retirer b.png' }));
    expect(description()).toHaveFocus();
    // Ctrl+Enter still adds the ticket.
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    expect(onsubmit).toHaveBeenCalledTimes(1);
  });

  it('asks before leaving a new ticket an image was pasted in', async () => {
    fakeBackend();
    show();
    paste(description(), { files: [capture()], types: ['Files'] });
    await screen.findByAltText('capture.png');
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
  });

  it('shows the images of a ticket being edited, and saves those kept by their file', async () => {
    const backend = fakeBackend({ ticket_images: () => READ });
    const { onsubmit } = show({ ticket: ticket({ images: IMAGES }), onsubmit: vi.fn(() => true) });
    expect(await screen.findByAltText('capture.png (1/2)')).toHaveAttribute('src', `data:image/png;base64,${b64}`);
    expect(screen.getByAltText('schema.gif (2/2)')).toHaveAttribute('src', 'data:image/gif;base64,R0lG');
    expect(backend.called('ticket_images')).toEqual([{ cmd: 'ticket_images', args: { id: 't1' } }]);
    await userEvent.click(screen.getByRole('button', { name: 'Retirer schema.gif (2/2)' }));
    paste(description(), { files: [capture('nouvelle.png')], types: ['Files'] });
    await screen.findByAltText('nouvelle.png (2/2)');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(onsubmit).toHaveBeenCalledWith(
      expect.objectContaining({
        images: [
          { name: 'capture.png', mediaType: 'image/png', file: 'a1.png', data: '' },
          { name: 'nouvelle.png', mediaType: 'image/png', file: '', data: b64 },
        ],
      }),
    );
  });

  it('shows the place of the images of a ticket being edited while they are read, and leaves without asking', async () => {
    fakeBackend({ ticket_images: () => new Promise(() => {}) });
    show({ ticket: ticket({ images: IMAGES }) });
    expect(screen.getByRole('img', { name: 'Chargement de capture.png (1/2)…' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Chargement de schema.gif (2/2)…' })).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
  });

  it('keeps the images of a ticket saved without touching them, even before they are read', async () => {
    fakeBackend({ ticket_images: () => new Promise(() => {}) });
    const { onsubmit } = show({ ticket: ticket({ images: IMAGES }), onsubmit: vi.fn(() => true) });
    await userEvent.type(title(), ' bis');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
    expect(onsubmit).toHaveBeenCalledTimes(1);
    expect(onsubmit.mock.calls[0][0]).not.toHaveProperty('images');
  });

  it('asks before leaving a ticket being edited one of whose images was taken away', async () => {
    fakeBackend({ ticket_images: () => READ });
    show({ ticket: ticket({ images: IMAGES }) });
    await userEvent.click(await screen.findByRole('button', { name: 'Retirer capture.png (1/2)' }));
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
  });

  it('keeps its images while another dialog takes its place, and comes back with them', async () => {
    fakeBackend();
    const self = (show(), app.modal);
    paste(description(), { files: [capture()], types: ['Files'] });
    await screen.findByAltText('capture.png');
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    cleanup();
    const resume = (self as { resume?: TicketFormDraft }).resume!;
    expect(resume.images).toEqual([expect.objectContaining({ name: 'capture.png', data: b64 })]);
    show({ resume });
    expect(screen.getByAltText('capture.png')).toHaveAttribute('src', `data:image/png;base64,${b64}`);
  });

  it('still sees no change in an image gone from the ticket’s folder once back from a dialog that took its place', async () => {
    fakeBackend({ ticket_images: () => READ.slice(0, 1) });
    const self = (show({ ticket: ticket({ images: IMAGES }) }), app.modal);
    await waitFor(() => expect(screen.queryByRole('img', { name: /schema\.gif/ })).not.toBeInTheDocument());
    // « Quitter Escouade ? » takes the window's place, then is cancelled.
    cleanup();
    const resume = (self as { resume?: TicketFormDraft }).resume!;
    show({ ticket: ticket({ images: IMAGES }), resume });
    expect(screen.getByAltText('capture.png')).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
  });

  it('is written in English', () => {
    setLang('en');
    fakeBackend({ ticket_images: () => new Promise(() => {}) });
    show({ ticket: ticket({ images: IMAGES }) });
    expect(screen.getByLabelText('Attach a file', { selector: 'input' })).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Ticket images' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Loading capture.png (1/2)…' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Remove capture.png (1/2)' })).toBeInTheDocument();
  });
});

const MB = 1024 * 1024;

describe('TicketForm, its files', () => {
  beforeEach(() => resetApp());
  afterEach(() => setLang('fr'));
  const PNG = [137, 80, 78, 71, 13, 10, 26, 10];
  const capture = (name = 'capture.png') => new File([new Uint8Array(PNG)], name, { type: 'image/png' });
  const zip = (name = 'sources.zip') => new File([new Uint8Array([80, 75, 3, 4])], name, { type: 'application/zip' });
  /** A file that says it has `size` bytes, without holding them. */
  const sized = (name: string, size: number, type = '') => {
    const f = new File(['x'], name, { type });
    Object.defineProperty(f, 'size', { value: size });
    return f;
  };
  /** The copy of a file in the drafts, as the backend gives it. */
  const copyOf = (name: string, over: Partial<AttachedFile> = {}): AttachedFile => ({
    name,
    size: 4,
    origin: false,
    path: `/data/attachments/_drafts/1f2e3d4c/${name}`,
    claudePath: '',
    ...over,
  });
  /** The backend of a copy: each file gets its handle, named after it, and comes back as its copy. */
  const copying = (over: Record<string, (a: any) => unknown> = {}) => ({
    attach_open: ({ name }: { name: string }) => `h:${name}`,
    attach_finish: ({ handle }: { handle: string }) => copyOf(handle.slice(2)),
    ...over,
  });
  /** The copy of a big file whose second chunk waits for `release`. */
  const stalled = () => {
    let release: () => void = () => {};
    let chunks = 0;
    const backend = fakeBackend(copying({ attach_chunk: () => (++chunks === 2 ? new Promise<void>((r) => (release = r)) : undefined) }));
    const stuck = () => waitFor(() => expect(backend.called('attach_chunk')).toHaveLength(2));
    return { backend, stuck, release: () => release() };
  };
  const title = () => screen.getByRole('textbox', { name: 'Titre du ticket' });
  const description = () => screen.getByRole('textbox', { name: 'Description' });
  const add = () => screen.getByRole('button', { name: 'Ajouter' });
  const drop = (files: File[], target: HTMLElement = description()) => {
    const e = createEvent.drop(target, { dataTransfer: { files, types: ['Files'] } });
    fireEvent(target, e);
    return e.defaultPrevented;
  };
  const paste = (target: HTMLElement, clipboardData: { files: File[]; types: string[] }) => {
    const e = createEvent.paste(target, { clipboardData });
    fireEvent(target, e);
    return e.defaultPrevented;
  };
  /** The copies are done: no chip is waiting for its path. */
  const copied = (backend: ReturnType<typeof fakeBackend>, count = 1) =>
    waitFor(() => {
      expect(backend.called('attach_finish')).toHaveLength(count);
      expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    });
  const FILES = [
    { name: 'budget.xlsx', file: 'ab12cd34/budget.xlsx', size: 2048 },
    { name: 'sources.zip', file: 'ef56ab78/sources.zip', size: 4096 },
  ];

  describe('a file that is no image of the ticket', () => {
    it('is copied to the drafts, shown as a chip, and added to the ticket by its copy', async () => {
      const backend = fakeBackend(copying());
      const { onsubmit } = show({ onsubmit: vi.fn(() => true) });
      await userEvent.type(title(), 'Le bouton');
      expect(drop([zip()])).toBe(true);
      const list = await screen.findByRole('list', { name: 'Fichiers du ticket' });
      expect(within(list).getByText('sources.zip')).toBeInTheDocument();
      expect(within(list).getByText('1 Ko')).toBeInTheDocument();
      await copied(backend);
      expect(backend.called('attach_open')[0].args).toEqual({ owner: 'draft', name: 'sources.zip', size: 4 });
      expect([...(backend.called('attach_chunk')[0].args as unknown as Uint8Array)]).toEqual([80, 75, 3, 4]);
      expect(backend.called('attach_finish')[0].args).toEqual({ handle: 'h:sources.zip' });
      await userEvent.click(add());
      expect(onsubmit).toHaveBeenCalledTimes(1);
      expect(onsubmit.mock.calls[0][0]).toEqual({
        title: 'Le bouton',
        description: '',
        criteria: [],
        maxLoops: 5,
        after: [],
        files: [{ name: 'sources.zip', file: '', path: copyOf('sources.zip').path }],
        engine: null,
        account: '',
        model: '',
        effort: '',
        shared: false,
      });
    });

    it.each([
      ['dropped on the window', () => drop([zip()], screen.getByText('Nouveau ticket'))],
      ['pasted from the clipboard', () => paste(description(), { files: [zip()], types: ['Files'] })],
    ])('is taken when it is %s, as an image is', async (_, give) => {
      const backend = fakeBackend(copying());
      show();
      expect(give()).toBe(true);
      expect(await screen.findByText('sources.zip')).toBeInTheDocument();
      await copied(backend);
    });

    it('is taken from the file dialog, which offers every file', async () => {
      const backend = fakeBackend(copying());
      show();
      const input = screen.getByLabelText('Joindre un fichier', { selector: 'input' });
      expect(input).not.toHaveAttribute('accept');
      expect(input).toHaveAttribute('multiple');
      await userEvent.upload(input, [zip(), new File(['PK'], 'budget.xlsx')]);
      expect(await screen.findByText('budget.xlsx')).toBeInTheDocument();
      expect(screen.getByText('sources.zip')).toBeInTheDocument();
      await copied(backend, 2);
    });

    it('is a PDF or a text as well, which the chat would send in its message: a ticket copies them all', async () => {
      const backend = fakeBackend(copying());
      show();
      drop([new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' }), new File(['# Notes'], 'notes.md')]);
      expect(await screen.findByText('notes.md')).toBeInTheDocument();
      expect(screen.getByText('rapport.pdf')).toBeInTheDocument();
      await copied(backend, 2);
      expect(backend.called('attach_open').map((c) => c.args.owner)).toEqual(['draft', 'draft']);
    });

    it('is an image over 5 MB, which the chat would refuse, or one whose content is no image', async () => {
      const backend = fakeBackend(copying());
      show();
      drop([sized('photo.png', 5 * MB + 1, 'image/png'), new File(['bonjour'], 'faux.png', { type: 'image/png' })]);
      expect(await screen.findByText('faux.png')).toBeInTheDocument();
      expect(screen.getByText('photo.png')).toBeInTheDocument();
      await copied(backend, 2);
      expect(backend.called('attach_open')[0].args).toEqual({ owner: 'draft', name: 'photo.png', size: 5 * MB + 1 });
      expect(screen.queryByRole('list', { name: 'Images du ticket' })).not.toBeInTheDocument();
      expect(app.toasts).toEqual([]);
    });

    it('is empty, 0 byte, and still attached', async () => {
      const backend = fakeBackend(copying());
      show();
      drop([new File([], 'vide.txt')]);
      expect(await screen.findByText('vide.txt')).toBeInTheDocument();
      await copied(backend);
      expect(backend.called('attach_chunk')).toHaveLength(0);
    });

    it('is never attached where it lies: a ticket copies, and its path is not even asked for', async () => {
      const backend = fakeBackend(copying());
      app.settings.attachOrigin = true;
      show();
      drop([zip()]);
      await screen.findByText('sources.zip');
      await copied(backend);
      expect(backend.called('attach_origin')).toHaveLength(0);
    });
  });

  describe('an image the chat reads', () => {
    it('stays an image of the ticket, 5 MB or less, with its preview and no copy', async () => {
      const backend = fakeBackend(copying());
      const { onsubmit } = show({ onsubmit: vi.fn(() => true) });
      await userEvent.type(title(), 'Le bouton');
      // An image that says it weighs 5 MB, the limit.
      const limit = capture('limite.png');
      Object.defineProperty(limit, 'size', { value: 5 * MB });
      drop([capture(), limit]);
      expect(await screen.findByAltText('capture.png (1/2)')).toBeInTheDocument();
      expect(await screen.findByAltText('limite.png (2/2)')).toBeInTheDocument();
      expect(screen.queryByRole('list', { name: 'Fichiers du ticket' })).not.toBeInTheDocument();
      expect(backend.called('attach_open')).toHaveLength(0);
      await userEvent.click(add());
      expect(onsubmit.mock.calls[0][0]).toHaveProperty('images');
      expect(onsubmit.mock.calls[0][0]).not.toHaveProperty('files');
    });

    it('goes with the files of the same drop, each to its list', async () => {
      const backend = fakeBackend(copying());
      const { onsubmit } = show({ onsubmit: vi.fn(() => true) });
      await userEvent.type(title(), 'Les deux');
      drop([capture(), zip()]);
      expect(await screen.findByAltText('capture.png')).toBeInTheDocument();
      expect(await screen.findByText('sources.zip')).toBeInTheDocument();
      await copied(backend);
      await userEvent.click(add());
      expect(onsubmit.mock.calls[0][0].images).toHaveLength(1);
      expect(onsubmit.mock.calls[0][0].files).toEqual([{ name: 'sources.zip', file: '', path: copyOf('sources.zip').path }]);
    });
  });

  describe('the chip of a file', () => {
    it('says its name and size, and is a chip of the chat: its icon by type', async () => {
      const backend = fakeBackend(copying());
      show();
      drop([zip('sources.zip'), new File(['PK'], 'budget.xlsx')]);
      const list = await screen.findByRole('list', { name: 'Fichiers du ticket' });
      await waitFor(() => expect(within(list).getAllByRole('listitem')).toHaveLength(2));
      expect(list.querySelector('[data-icon="archive"]')).not.toBeNull();
      expect(list.querySelector('[data-icon="sheet"]')).not.toBeNull();
      await copied(backend, 2);
    });

    it('shows how far its copy went, and does not let the ticket be added meanwhile', async () => {
      const { release, stuck } = stalled();
      const { onsubmit } = show({ onsubmit: vi.fn(() => true) });
      await userEvent.type(title(), 'Le film');
      drop([sized('film.mkv', 10 * MB, 'video/x-matroska')]);
      const bar = await screen.findByRole('progressbar', { name: 'Copie de film.mkv' });
      await stuck();
      expect(bar).toHaveAttribute('aria-valuenow', '40');
      expect(add()).toBeDisabled();
      await userEvent.keyboard('{Control>}{Enter}{/Control}');
      expect(onsubmit).not.toHaveBeenCalled();
      release();
      await waitFor(() => expect(add()).toBeEnabled());
      await userEvent.click(add());
      expect(onsubmit).toHaveBeenCalledTimes(1);
    });

    it('is not added while its copy has not even begun', async () => {
      let open: (handle: string) => void = () => {};
      fakeBackend(copying({ attach_open: () => new Promise((r) => (open = r)) }));
      show();
      await userEvent.type(title(), 'Le film');
      drop([zip()]);
      expect(await screen.findByRole('progressbar', { name: 'Copie de sources.zip' })).toHaveAttribute('aria-valuenow', '0');
      expect(add()).toBeDisabled();
      open('h:sources.zip');
      await waitFor(() => expect(add()).toBeEnabled());
    });

    it('erases its copy when it is taken away, and keeps the focus in the form', async () => {
      const backend = fakeBackend(copying());
      show();
      drop([zip('a.zip'), zip('b.zip')]);
      await screen.findByText('b.zip');
      await copied(backend, 2);
      await userEvent.click(screen.getByRole('button', { name: 'Retirer a.zip' }));
      expect(screen.queryByText('a.zip')).not.toBeInTheDocument();
      expect(backend.called('attach_discard').map((c) => c.args)).toEqual([{ owner: 'draft', path: copyOf('a.zip').path }]);
      expect(screen.getByRole('button', { name: 'Retirer b.zip' })).toHaveFocus();
      await userEvent.click(screen.getByRole('button', { name: 'Retirer b.zip' }));
      expect(backend.called('attach_discard')).toHaveLength(2);
      expect(description()).toHaveFocus();
      expect(screen.queryByRole('list', { name: 'Fichiers du ticket' })).not.toBeInTheDocument();
    });

    it('stops its copy when it is taken away while it goes on, without a word', async () => {
      const { backend, release, stuck } = stalled();
      show();
      drop([sized('film.mkv', 10 * MB, 'video/x-matroska')]);
      await stuck();
      await userEvent.click(screen.getByRole('button', { name: 'Retirer film.mkv' }));
      expect(screen.queryByText('film.mkv')).not.toBeInTheDocument();
      release();
      await waitFor(() => expect(backend.called('attach_cancel')).toHaveLength(1));
      expect(backend.called('attach_cancel')[0].args).toEqual({ handle: 'h:film.mkv' });
      expect(backend.called('attach_finish')).toHaveLength(0);
      expect(backend.called('attach_discard')).toHaveLength(0);
      expect(app.toasts).toEqual([]);
    });

    it('is taken away, and the reason told, when its copy fails: the ticket can still be added', async () => {
      fakeBackend(
        copying({
          attach_open: () => {
            throw 'film.mkv dépasse 100 Mo';
          },
        }),
      );
      const { onsubmit } = show({ onsubmit: vi.fn(() => true) });
      await userEvent.type(title(), 'Le film');
      drop([sized('film.mkv', 2 * MB)]);
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'film.mkv dépasse 100 Mo' }));
      expect(screen.queryByText('film.mkv')).not.toBeInTheDocument();
      await waitFor(() => expect(add()).toBeEnabled());
      await userEvent.click(add());
      expect(onsubmit.mock.calls[0][0]).not.toHaveProperty('files');
    });

    it('is refused over the maximum size of the settings, with nothing copied, and warned of over their threshold', async () => {
      app.settings.attachMaxMb = 3;
      app.settings.attachWarnMb = 1;
      const backend = fakeBackend(copying());
      show();
      drop([sized('enorme.mkv', 4 * MB)]);
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'enorme.mkv dépasse 3 Mo' }));
      expect(screen.queryByText('enorme.mkv')).not.toBeInTheDocument();
      expect(backend.called('attach_open')).toHaveLength(0);
      drop([sized('film.mkv', 2 * MB), zip()]);
      expect(await screen.findByRole('img', { name: 'Gros fichier : 2 Mo, copié sur le disque' })).toBeInTheDocument();
      // A small one is not warned of.
      expect(await screen.findByText('sources.zip')).toBeInTheDocument();
      expect(screen.getAllByRole('img', { name: /^Gros fichier/ })).toHaveLength(1);
      await copied(backend, 2);
    });

    it('takes 20 files at most, those of the ticket counted, and names the one that is left out', async () => {
      const backend = fakeBackend(copying());
      const have = Array.from({ length: 18 }, (_, n) => ({ name: `a${n}.zip`, file: `ab12cd34/a${n}.zip`, size: 4 }));
      show({ ticket: ticket({ files: have }) });
      drop([zip('x.zip'), zip('y.zip'), zip('z.zip')]);
      await waitFor(() => expect(app.toasts.at(-1)?.text).toBe("z.zip n'est pas ajouté : un ticket prend 20 fichiers au plus."));
      expect(within(screen.getByRole('list', { name: 'Fichiers du ticket' })).getAllByRole('listitem')).toHaveLength(20);
      expect(screen.queryByText('z.zip')).not.toBeInTheDocument();
      await copied(backend, 2);
      expect(backend.called('attach_open').map((c) => c.args.name)).toEqual(['x.zip', 'y.zip']);
    });

    it('is no part of the 20 images: each list has its own 20', async () => {
      fakeBackend(copying());
      show();
      paste(description(), { files: Array.from({ length: 20 }, (_, n) => capture(`image-${n + 1}.png`)), types: ['Files'] });
      await waitFor(() => expect(within(screen.getByRole('list', { name: 'Images du ticket' })).getAllByRole('img')).toHaveLength(20));
      drop([zip()]);
      expect(await screen.findByText('sources.zip')).toBeInTheDocument();
      expect(app.toasts).toEqual([]);
    });

    it('takes the room of its chips from the description by its style, not by its rows (ESC-189)', async () => {
      fakeBackend(copying());
      window.innerHeight = 700;
      window.dispatchEvent(new Event('resize'));
      try {
        show();
        drop([zip()]);
        await screen.findByText('sources.zip');
        expect(description()).not.toHaveAttribute('rows');
        expect(description().style.height).toBe('');
        await userEvent.click(screen.getByRole('button', { name: 'Retirer sources.zip' }));
        expect(screen.queryByText('sources.zip')).not.toBeInTheDocument();
      } finally {
        window.innerHeight = 768;
        window.dispatchEvent(new Event('resize'));
      }
    });
  });

  describe('a ticket that has files', () => {
    it('shows them as chips in its form, with their name and size', () => {
      fakeBackend(copying());
      show({ ticket: ticket({ files: FILES }) });
      const list = screen.getByRole('list', { name: 'Fichiers du ticket' });
      expect(within(list).getByText('budget.xlsx')).toBeInTheDocument();
      expect(within(list).getByText('2 Ko')).toBeInTheDocument();
      expect(within(list).getByText('sources.zip')).toBeInTheDocument();
      expect(within(list).getByText('4 Ko')).toBeInTheDocument();
      expect(within(list).getAllByRole('button', { name: /^Retirer / })).toHaveLength(2);
      expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    });

    it('asks nothing of the backend to show them, and leaves without asking when nothing changed', async () => {
      const backend = fakeBackend(copying());
      show({ ticket: ticket({ files: FILES }) });
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toBeNull();
      expect(backend.calls).toEqual([]);
    });

    it('sends no files when it is saved without touching them', async () => {
      fakeBackend(copying());
      const { onsubmit } = show({ ticket: ticket({ files: FILES }), onsubmit: vi.fn(() => true) });
      await userEvent.type(title(), ' bis');
      await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
      expect(onsubmit).toHaveBeenCalledTimes(1);
      expect(onsubmit.mock.calls[0][0]).not.toHaveProperty('files');
    });

    it('keeps those left by their file, and adds the new ones by the path of their copy', async () => {
      const backend = fakeBackend(copying());
      const { onsubmit } = show({ ticket: ticket({ files: FILES }), onsubmit: vi.fn(() => true) });
      await userEvent.click(screen.getByRole('button', { name: 'Retirer budget.xlsx' }));
      // Not erased now: the ticket's file goes when the ticket is saved.
      expect(backend.called('attach_discard')).toHaveLength(0);
      drop([zip('nouveau.zip')]);
      await screen.findByText('nouveau.zip');
      await copied(backend);
      await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
      expect(onsubmit.mock.calls[0][0].files).toEqual([
        { name: 'sources.zip', file: 'ef56ab78/sources.zip', path: '' },
        { name: 'nouveau.zip', file: '', path: copyOf('nouveau.zip').path },
      ]);
      expect(backend.called('attach_discard')).toHaveLength(0);
    });

    it('sends an empty list when every file was taken away, which takes them off the ticket', async () => {
      fakeBackend(copying());
      const { onsubmit } = show({ ticket: ticket({ files: FILES.slice(0, 1) }), onsubmit: vi.fn(() => true) });
      await userEvent.click(screen.getByRole('button', { name: 'Retirer budget.xlsx' }));
      await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));
      expect(onsubmit.mock.calls[0][0].files).toEqual([]);
    });

    it('asks before leaving once a file was taken away from it, or put in', async () => {
      fakeBackend(copying());
      show({ ticket: ticket({ files: FILES }) });
      await userEvent.click(screen.getByRole('button', { name: 'Retirer budget.xlsx' }));
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
    });

    it('does not ask when what was taken away is back to what it was', async () => {
      const backend = fakeBackend(copying());
      show({ ticket: ticket({ files: FILES.slice(0, 1) }) });
      drop([zip()]);
      await screen.findByText('sources.zip');
      await copied(backend);
      await userEvent.click(screen.getByRole('button', { name: 'Retirer sources.zip' }));
      await userEvent.keyboard('{Escape}');
      expect(app.modal).toBeNull();
    });
  });

  describe('closing the form without saving', () => {
    /** « Annuler » asks first, and the confirmation of it is the one that gives up. */
    const giveUp = async () => {
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
      return (app.modal as any).onConfirm();
    };

    it('asks before leaving a new ticket a file was attached to', async () => {
      const backend = fakeBackend(copying());
      show();
      drop([zip()]);
      await screen.findByText('sources.zip');
      await copied(backend);
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
      // Nothing is erased before the user agrees.
      expect(backend.called('attach_discard')).toHaveLength(0);
    });

    it('erases the copies this form made once the user gives up, not the files the ticket had', async () => {
      const backend = fakeBackend(copying());
      show({ ticket: ticket({ files: FILES }) });
      drop([zip('un.zip'), zip('deux.zip')]);
      await screen.findByText('deux.zip');
      await copied(backend, 2);
      await giveUp();
      expect(backend.called('attach_discard').map((c) => c.args)).toEqual([
        { owner: 'draft', path: copyOf('un.zip').path },
        { owner: 'draft', path: copyOf('deux.zip').path },
      ]);
    });

    it('erases nothing of a copy taken away already, and nothing when the form had no copy', async () => {
      const backend = fakeBackend(copying());
      show({ ticket: ticket({ files: FILES }) });
      await userEvent.click(screen.getByRole('button', { name: 'Retirer budget.xlsx' }));
      await giveUp();
      expect(backend.called('attach_discard')).toHaveLength(0);
    });

    it('stops a copy that goes on, which erases what it wrote', async () => {
      const { backend, release, stuck } = stalled();
      show();
      drop([sized('film.mkv', 10 * MB, 'video/x-matroska')]);
      await stuck();
      await giveUp();
      release();
      await waitFor(() => expect(backend.called('attach_cancel')).toHaveLength(1));
      expect(backend.called('attach_finish')).toHaveLength(0);
      expect(backend.called('attach_discard')).toHaveLength(0);
      expect(app.toasts).toEqual([]);
    });

    it('does not erase a copy when the confirmation is dismissed: the form comes back with its files', async () => {
      const backend = fakeBackend(copying());
      const self = (show(), app.modal);
      await userEvent.type(title(), 'Un début');
      drop([zip()]);
      await screen.findByText('sources.zip');
      await copied(backend);
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      cleanup();
      (app.modal as any).onCancel();
      expect(app.modal).toBe(self);
      const { onsubmit } = show({ resume: (self as { resume?: TicketFormDraft }).resume, onsubmit: vi.fn(() => true) });
      expect(screen.getByRole('button', { name: 'Retirer sources.zip' })).toBeInTheDocument();
      expect(backend.called('attach_discard')).toHaveLength(0);
      await userEvent.click(add());
      expect(onsubmit.mock.calls[0][0].files).toEqual([{ name: 'sources.zip', file: '', path: copyOf('sources.zip').path }]);
    });

    it('shows a copy that went on while another dialog had its place, done once the form is back', async () => {
      const { release, stuck } = stalled();
      const self = (show(), app.modal);
      await userEvent.type(title(), 'Le film');
      drop([sized('film.mkv', 10 * MB, 'video/x-matroska')]);
      await stuck();
      await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
      cleanup();
      const { onsubmit } = show({ resume: (self as { resume?: TicketFormDraft }).resume, onsubmit: vi.fn(() => true) });
      expect(screen.getByRole('progressbar', { name: 'Copie de film.mkv' })).toHaveAttribute('aria-valuenow', '40');
      expect(add()).toBeDisabled();
      release();
      await waitFor(() => expect(screen.queryByRole('progressbar')).not.toBeInTheDocument());
      await userEvent.click(add());
      expect(onsubmit.mock.calls[0][0].files).toEqual([{ name: 'film.mkv', file: '', path: copyOf('film.mkv').path }]);
    });

    it('keeps what it sent once it is saved: nothing is erased', async () => {
      const backend = fakeBackend(copying());
      show({ onsubmit: vi.fn(() => true) });
      await userEvent.type(title(), 'Le bouton');
      drop([zip()]);
      await screen.findByText('sources.zip');
      await copied(backend);
      await userEvent.click(add());
      await waitFor(() => expect(app.modal).toBeNull());
      expect(backend.called('attach_discard')).toHaveLength(0);
    });
  });

  it('is written in English', async () => {
    setLang('en');
    const backend = fakeBackend(copying());
    show({ ticket: ticket({ files: FILES }) });
    expect(screen.getByLabelText('Attach a file', { selector: 'input' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Attach a file' })).toBeInTheDocument();
    const list = screen.getByRole('list', { name: 'Ticket files' });
    expect(within(list).getByRole('button', { name: 'Remove budget.xlsx' })).toBeInTheDocument();
    drop([sized('film.mkv', 2 * MB)]);
    await screen.findByText('film.mkv');
    await copied(backend);
    await userEvent.click(screen.getByRole('button', { name: 'Remove film.mkv' }));
    app.settings.attachMaxMb = 1;
    drop([sized('enorme.mkv', 2 * MB)]);
    await waitFor(() => expect(app.toasts.at(-1)?.text).toBe('enorme.mkv is over 1 MB'));
    app.settings.attachMaxMb = 0;
    drop(Array.from({ length: 19 }, (_, n) => zip(`f${n}.zip`)));
    // The ticket has 2, the first 18 of these come to 20, the 19th has no place.
    await waitFor(() => expect(app.toasts.at(-1)?.text).toBe('f18.zip wasn’t added: a ticket takes 20 files at most.'));
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

  /** « Après », its popover opened (the list and its search are in it). */
  const group = () => openAfter('Après');
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
    // Escape closes the popover alone, its button getting the focus back; once more, the window.
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog', { name: 'Après' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^Après / })).toHaveFocus();
    expect(app.modal).not.toBeNull();
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toBeNull();
    app.modal = { kind: 'ticket', projectId: 'p1', onSubmit: () => true };
    await userEvent.click(box('DEM-2'));
    await userEvent.keyboard('{Escape}');
    await userEvent.keyboard('{Escape}');
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Abandonner les modifications ?' });
    // DEM-4, done, is not listed but stays.
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ after: ['t4'] }));
  });

  it('says on its button the tickets checked, « Aucun » without any, and closes its popover on a click elsewhere', async () => {
    show({ ticket: ticket({ after: ['t4', 't5'] }) });
    const button = () => screen.getByRole('button', { name: /^Après / });
    // DEM-4, done, holds nothing back: not said.
    expect(button()).toHaveAccessibleName('Après DEM-5');
    expect(button()).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(box('DEM-5'));
    expect(button()).toHaveAccessibleName('Après Aucun');
    await userEvent.click(box('DEM-2'));
    await userEvent.click(box('DEM-3'));
    expect(button()).toHaveAccessibleName('Après DEM-2, DEM-3');
    await userEvent.click(screen.getByRole('textbox', { name: 'Titre du ticket' }));
    expect(screen.queryByRole('dialog', { name: 'Après' })).not.toBeInTheDocument();
    expect(button()).toHaveAttribute('aria-expanded', 'false');
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
    expect(screen.getByRole('textbox', { name: 'Ticket title' })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Description' })).toHaveAttribute('placeholder', 'Description (optional)');
    expect(screen.getByRole('textbox', { name: 'Acceptance criteria' })).toHaveAttribute(
      'placeholder',
      'Acceptance criteria, one per line',
    );
    expect(screen.getByRole('group', { name: 'Max loops' })).toBeInTheDocument();
    const after = openAfter('After');
    await userEvent.type(within(after).getByRole('searchbox', { name: 'Search for a key' }), 'ZZZ');
    expect(within(after).getByText('No tickets for this key')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Add' })).toBeInTheDocument();
  });

  it('names its window and says the key that adds the ticket in English', () => {
    const { unmount } = show();
    expect(screen.getByRole('dialog', { name: 'New ticket' })).toBeInTheDocument();
    expect(document.querySelector('.hint')).toHaveTextContent('Ctrl+Enter to add');
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
    unmount();
    show({ ticket: ticket() });
    expect(screen.getByRole('dialog', { name: 'Edit DEM-1' })).toBeInTheDocument();
    expect(document.querySelector('.hint')).toHaveTextContent('Ctrl+Enter to save');
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
    await userEvent.click(within(openAfter('After')).getByRole('checkbox', { name: /^DEM-2 / }));
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});

describe('TicketForm: its harness (ESC-29)', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  });
  const field = () => screen.queryByRole('button', { name: /^Harness/ });

  it('has no « Harness » field while a single harness is found', () => {
    fakeBackend();
    show();
    expect(field()).toBeNull();
  });

  it('runs on the project’s harness by default, and sends the one chosen with the ticket', async () => {
    fakeBackend();
    usableEngines('claude', 'codex');
    const { onsubmit } = show();
    expect(field()).toHaveAccessibleName('Harness Celui du projet (Claude Code)');
    await userEvent.click(field()!);
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Celui du projet (Claude Code)', 'Claude Code', 'Harness nu']);
    menu.open!.items[2].onClick!();
    menu.close();
    await waitFor(() => expect(field()).toHaveAccessibleName('Harness Harness nu'));
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Sur un autre{Enter}');
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ title: 'Sur un autre', engine: 'codex' }));
  });

  it('shows the harness of an edited ticket, and gives it back to the project’s', async () => {
    fakeBackend();
    usableEngines('claude', 'codex');
    const { onsubmit } = show({ ticket: ticket({ engine: 'codex' }) });
    expect(field()).toHaveAccessibleName('Harness Harness nu');
    await userEvent.click(field()!);
    menu.open!.items[0].onClick!();
    menu.close();
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ engine: null }));
  });

  it('offers only the harnesses that can be used, and keeps the ticket’s saying why it cannot any more (ESC-391)', async () => {
    fakeBackend();
    usableEngines('claude', 'codex');
    app.settings = { ...app.settings, accounts: app.settings.accounts.filter((a) => a.engine !== 'codex') };
    // Codex found without an account: no field.
    const { unmount } = show();
    expect(field()).toBeNull();
    unmount();
    show({ ticket: ticket({ engine: 'codex' }) });
    expect(field()).toHaveAccessibleName('Harness Harness nu (aucun compte)');
    await userEvent.click(field()!);
    expect(menu.open!.items.map((i) => i.label)).toEqual(['Celui du projet (Claude Code)', 'Claude Code', 'Harness nu (aucun compte)']);
    menu.close();
  });
});

describe('TicketForm: its account, model and effort (ESC-153)', () => {
  const account = (over: Partial<Account>): Account => ({
    id: 'principal',
    name: 'Principal',
    engine: 'claude',
    configDir: '',
    claudePath: '',
    active: true,
    projects: [],
    ...over,
  });
  const told = (id: string, label: string, efforts: string[]): ToldModel => ({
    id,
    label,
    efforts,
    defaultEffort: null,
    images: true,
    default: false,
  });
  const ALL = ['low', 'medium', 'high', 'xhigh', 'max'];
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings = {
      ...app.settings,
      accounts: [
        account({}),
        account({ id: 'pro', name: 'Pro', configDir: 'C:\\pro' }),
        // Reserved to another project, and another harness's: never offered here.
        account({ id: 'team', name: 'Équipe', configDir: 'C:\\team', projects: ['p2'] }),
        account({ id: 'cx', name: 'Codex perso', engine: 'codex', configDir: 'C:\\cx' }),
      ],
    };
    app.usage = { ...app.usage, current: { claude: 'principal', codex: 'cx' } };
    app.accountModels = {
      principal: {
        version: '2.1.302',
        models: [],
        catalog: [told('opus', 'Opus', ALL), told('sonnet', 'Sonnet', ALL), told('haiku', 'Haiku', [])],
      },
      pro: { version: '2.1.302', models: [], catalog: [told('sonnet', 'Sonnet', ALL), told('haiku', 'Haiku', [])] },
    };
    app.projects = [project({ board: { ...project().board, model: 'opus', effort: 'medium' } })];
  });
  const field = (name: string) => screen.getByRole('button', { name: new RegExp(`^${name} `) });
  /** Picks `label` in the menu of the field `name`. */
  async function pick(name: string, label: string) {
    await userEvent.click(field(name));
    const item = menu.open!.items.find((i) => i.label === label);
    expect(item, `${label} in ${menu.open!.items.map((i) => i.label).join(', ')}`).toBeDefined();
    item!.onClick!();
    menu.close();
  }
  async function labels(name: string) {
    await userEvent.click(field(name));
    const all = menu.open!.items.map((i) => i.label);
    menu.close();
    return all;
  }

  it('is on « Auto » for each, saying what it gives: the current account, the Kanban’s model and effort', () => {
    fakeBackend();
    show();
    expect(field('Compte')).toHaveAccessibleName('Compte Auto · Principal');
    expect(field('Modèle')).toHaveAccessibleName('Modèle Auto · Opus (Kanban)');
    expect(field('Effort')).toHaveAccessibleName('Effort Auto · Moyen (Kanban)');
  });

  it('says the account’s defaults when the Kanban has none', () => {
    fakeBackend();
    app.projects = [project()];
    app.settings.accounts![0] = account({ defaultModel: 'sonnet', defaultEffort: 'high' });
    show();
    expect(field('Modèle')).toHaveAccessibleName('Modèle Auto · Sonnet (compte)');
    expect(field('Effort')).toHaveAccessibleName('Effort Auto · Élevé (compte)');
  });

  it('lists the accounts of its harness its project may use', async () => {
    fakeBackend();
    show();
    expect(await labels('Compte')).toEqual(['Auto · Principal', 'Principal', 'Pro']);
  });

  it('lists the models of the account chosen, and of the current one on « Auto »', async () => {
    fakeBackend();
    show();
    expect(await labels('Modèle')).toEqual(['Auto · Opus (Kanban)', 'Opus', 'Sonnet', 'Haiku']);
    await pick('Compte', 'Pro');
    expect(field('Compte')).toHaveAccessibleName('Compte Pro');
    expect(await labels('Modèle')).toEqual(['Auto · Opus (Kanban)', 'Sonnet', 'Haiku']);
  });

  it('sets back to « Auto » a model the account chosen has not, and says so', async () => {
    fakeBackend();
    show();
    await pick('Modèle', 'Opus');
    expect(field('Modèle')).toHaveAccessibleName('Modèle Opus');
    await pick('Compte', 'Pro');
    expect(field('Modèle')).toHaveAccessibleName('Modèle Auto · Opus (Kanban)');
    expect(screen.getByRole('status')).toHaveTextContent('« Modèle » remis sur « Auto » : Pro n’a pas Opus.');
  });

  it('lists the efforts of the model, greys « Effort » out with why for one that takes none, and sets back what it had', async () => {
    fakeBackend();
    show();
    expect(await labels('Effort')).toEqual(['Auto · Moyen (Kanban)', 'Bas', 'Moyen', 'Élevé', 'Très élevé', 'Max']);
    await pick('Effort', 'Élevé');
    await pick('Modèle', 'Haiku');
    expect(field('Effort')).toBeDisabled();
    expect(field('Effort')).toHaveAccessibleDescription('Haiku ne prend pas d’effort');
    expect(screen.getByRole('status')).toHaveTextContent('« Effort » remis sur « Auto » : Haiku ne prend pas l’effort Élevé.');
  });

  it('sets the account back to « Auto » when the harness changes, and says so', async () => {
    fakeBackend();
    usableEngines('claude', 'codex');
    show();
    await pick('Compte', 'Pro');
    await userEvent.click(screen.getByRole('button', { name: /^Harness/ }));
    menu.open!.items.find((i) => i.label === 'Harness nu')!.onClick!();
    menu.close();
    await waitFor(() => expect(field('Compte')).toHaveAccessibleName('Compte Auto · Codex perso'));
    expect(screen.getByRole('status')).toHaveTextContent('« Compte » remis sur « Auto » : Pro ne sert pas ce ticket sur ce harness.');
    expect(await labels('Compte')).toEqual(['Auto · Codex perso', 'Codex perso']);
  });

  it('sends them with the ticket, empty for « Auto », and shows those of an edited ticket', async () => {
    fakeBackend();
    const { onsubmit } = show();
    await pick('Compte', 'Pro');
    await pick('Modèle', 'Sonnet');
    await pick('Effort', 'Max');
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), 'Sur Pro{Enter}');
    expect(onsubmit).toHaveBeenCalledWith(expect.objectContaining({ account: 'pro', model: 'sonnet', effort: 'max' }));
    cleanup();
    const edited = show({ ticket: ticket({ account: 'pro', model: 'sonnet', effort: 'low' }) });
    expect(field('Compte')).toHaveAccessibleName('Compte Pro');
    expect(field('Modèle')).toHaveAccessibleName('Modèle Sonnet');
    expect(field('Effort')).toHaveAccessibleName('Effort Bas');
    await pick('Compte', 'Auto · Principal');
    await userEvent.type(screen.getByRole('textbox', { name: 'Titre du ticket' }), '{Enter}');
    expect(edited.onsubmit).toHaveBeenCalledWith(expect.objectContaining({ account: '', model: 'sonnet', effort: 'low' }));
  });
});
