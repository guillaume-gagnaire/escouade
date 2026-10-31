import { createEvent, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { conversationOf } from '../lib/conversations.svelte';
import { giveBack } from '../lib/drafts';
import { fileOrigins } from '../lib/file-origin';
import { cancelHeld, editHeld, heldOf, sendNow } from '../lib/outbox.svelte';
import { setLang } from '../lib/i18n';
import { readPref, writePref } from '../lib/prefs';
import { app } from '../lib/state.svelte';
import type { AttachedFile } from '../lib/types';
import { agent, BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, resetApp } from '../test/ipc';
import Composer from './Composer.svelte';

// Where a file lies on the disk is the system's to say: no file has a known path unless a test gives one.
vi.mock('../lib/file-origin', () => ({ fileOrigins: vi.fn() }));
beforeEach(() => {
  vi.mocked(fileOrigins).mockImplementation(async (files) => files.map(() => null));
});

function setup(over: Parameters<typeof agent>[0] = {}, items: unknown[] = []) {
  const a = agent({ id: `c${Math.random()}`, ...over });
  resetApp({ agents: [a] });
  const backend = fakeBackend({
    get_conversation: () => items,
    file_suggestions: () => ['src/middleware/auth.ts', 'src/app.ts'],
    get_commands: () => [
      { name: 'compact', description: 'Compacte le contexte' },
      { name: 'commit', description: 'Crée un commit' },
    ],
  });
  render(Composer, { agent: app.agents[a.id] });
  return { a, backend, textarea: screen.getByRole('textbox') as HTMLTextAreaElement };
}

describe('Composer', () => {
  beforeEach(() => resetApp());

  it('takes a door of the Workflow tab at the head of its message, in place of a command there, without sending it (ESC-194)', async () => {
    const { a, backend, textarea } = setup();
    app.composerPut = { agentId: 'other', text: '/gsd:next' };
    await Promise.resolve();
    expect(textarea.value).toBe('');
    app.composerPut = { agentId: a.id, text: '/gsd:next' };
    await waitFor(() => expect(textarea.value).toBe('/gsd:next '));
    expect(app.composerPut).toBeNull();
    await userEvent.type(textarea, 'phase 3');
    app.composerPut = { agentId: a.id, text: '/gsd:verify-work' };
    await waitFor(() => expect(textarea.value).toBe('/gsd:verify-work phase 3'));
    expect(backend.called('send_message')).toEqual([]);
  });

  it('offers the xhigh effort level in the effort menu and applies it to the agent', async () => {
    const { a, backend } = setup({ model: 'opus', effort: 'high' });
    await userEvent.click(screen.getByRole('button', { name: 'Effort : Élevé' }));
    expect(screen.getByRole('menuitemradio', { name: /^Élevé/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('menuitemradio', { name: /Très élevé/ }));
    expect(backend.called('set_agent_options').at(-1)?.args).toMatchObject({ id: a.id, effort: 'xhigh' });
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('picks the model from a menu rather than a row of buttons', async () => {
    const { a, backend } = setup({ model: 'sonnet' });
    expect(screen.queryByRole('button', { name: 'Opus' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Modèle : Sonnet' }));
    expect(screen.getByRole('menuitemradio', { name: /Sonnet/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('menuitemradio', { name: /Opus/ }));
    expect(backend.called('set_agent_options').at(-1)?.args).toMatchObject({ id: a.id, model: 'opus' });
    expect(screen.getByRole('button', { name: 'Modèle : Opus' })).toBeInTheDocument();
  });

  it('names each model with the version Claude Code runs for it', async () => {
    setup({ model: 'sonnet' });
    app.models = [
      { value: 'opus', resolvedModel: 'claude-opus-5-5' },
      { value: 'sonnet', resolvedModel: 'claude-sonnet-5-5' },
    ];
    await userEvent.click(await screen.findByRole('button', { name: 'Modèle : Sonnet 5.5' }));
    expect(screen.getByRole('menuitemradio', { name: /Opus 5\.5/ })).toBeInTheDocument();
  });

  it('keeps a single menu open at a time', async () => {
    setup({ model: 'sonnet', mode: 'auto' });
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    await userEvent.click(screen.getByRole('button', { name: /^Mode/ }));
    expect(screen.getAllByRole('menu')).toHaveLength(1);
    expect(screen.getByRole('menuitemradio', { name: /Plan/ })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /^Mode/ }));
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('closes an open menu with Escape without interrupting Claude, even from the text field', async () => {
    const { backend, textarea } = setup({ status: 'running' });
    await userEvent.click(screen.getByRole('button', { name: /^Effort/ }));
    textarea.focus();
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(backend.called('interrupt')).toHaveLength(0);
    await userEvent.keyboard('{Escape}');
    expect(backend.called('interrupt')).toHaveLength(1);
  });

  it('gives the focus back to the text field after a pick', async () => {
    const { textarea } = setup({ model: 'sonnet' });
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    await userEvent.click(screen.getByRole('menuitemradio', { name: /Opus/ }));
    expect(document.activeElement).toBe(textarea);
  });

  it('closes an open menu when a dialog opens over it', async () => {
    setup();
    await userEvent.click(screen.getByRole('button', { name: /^Mode/ }));
    app.modal = { kind: 'settings' };
    await waitFor(() => expect(screen.queryByRole('menu')).not.toBeInTheDocument());
  });

  it('keeps what is being typed when the agent is updated by the backend', async () => {
    const a = agent({ id: `k${Math.random()}`, status: 'running' });
    resetApp({ agents: [a] });
    fakeBackend({ get_conversation: () => [] });
    const { rerender } = render(Composer, { agent: a });
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    await userEvent.type(textarea, 'Pense aussi aux tests');
    // Every tool call / token update delivers a new agent object to the composer.
    await rerender({ agent: { ...a, tokens: 1234, status: 'waiting' } });
    expect(textarea.value).toBe('Pense aussi aux tests');
  });

  it('restores the draft of an agent after switching away and back', async () => {
    const a = agent({ id: `d${Math.random()}` });
    const b = agent({ id: `e${Math.random()}` });
    resetApp({ agents: [a, b] });
    fakeBackend({ get_conversation: () => [] });
    const first = render(Composer, { agent: app.agents[a.id] });
    await userEvent.type(screen.getByRole('textbox'), 'brouillon de A');
    first.unmount(); // App re-creates the composer for each agent ({#key})
    const second = render(Composer, { agent: app.agents[b.id] });
    expect((screen.getByRole('textbox') as HTMLTextAreaElement).value).toBe('');
    second.unmount();
    render(Composer, { agent: app.agents[a.id] });
    expect((screen.getByRole('textbox') as HTMLTextAreaElement).value).toBe('brouillon de A');
  });

  describe('draft kept in the local preferences', () => {
    const kept = (id: string) => readPref(`draft.${id}`);

    it('is written once the typing pauses', async () => {
      const { a, textarea } = setup();
      await userEvent.type(textarea, 'Pense aux tests');
      expect(kept(a.id)).toBeNull();
      await waitFor(() => expect(kept(a.id)).toBe('Pense aux tests'));
    });

    it('comes back when the app restarts, from the preferences alone', () => {
      const a = agent({ id: `r${Math.random()}` });
      resetApp({ agents: [a] });
      fakeBackend({ get_conversation: () => [] });
      // Nothing in memory for this agent: only what an earlier run kept.
      writePref(`draft.${a.id}`, 'Reprends la migration\navec les tests');
      render(Composer, { agent: app.agents[a.id] });
      expect(screen.getByRole('textbox')).toHaveValue('Reprends la migration\navec les tests');
    });

    it('is emptied as the message is sent', async () => {
      const { a, backend, textarea } = setup();
      await userEvent.type(textarea, 'Ajoute des tests');
      await waitFor(() => expect(kept(a.id)).toBe('Ajoute des tests'));
      await userEvent.keyboard('{Enter}');
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      expect(kept(a.id)).toBeNull();
    });

    it('is written again when the message is refused and given back', async () => {
      const { a, textarea } = setup();
      fakeBackend({
        send_message: () => {
          throw new Error('Claude Code est introuvable');
        },
      });
      await userEvent.type(textarea, 'Résume{Enter}');
      await waitFor(() => expect(app.toasts.at(-1)?.text).toMatch(/introuvable/));
      expect(textarea).toHaveValue('Résume');
      await waitFor(() => expect(kept(a.id)).toBe('Résume'));
    });

    it('is kept for each agent, and not for the attachments', async () => {
      const a = agent({ id: `p${Math.random()}` });
      const b = agent({ id: `q${Math.random()}` });
      resetApp({ agents: [a, b] });
      fakeBackend({ get_conversation: () => [] });
      const { rerender } = render(Composer, { agent: app.agents[a.id] });
      await userEvent.upload(
        screen.getByLabelText('Joindre un fichier', { selector: 'input' }),
        new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' }),
      );
      await screen.findByText('rapport.pdf');
      await userEvent.type(screen.getByRole('textbox'), 'pour A');
      await rerender({ agent: app.agents[b.id] });
      await userEvent.type(screen.getByRole('textbox'), 'pour B');
      await waitFor(() => expect(kept(b.id)).toBe('pour B'));
      expect(kept(a.id)).toBe('pour A');
      expect(Object.keys(localStorage).filter((k) => localStorage.getItem(k)?.includes('rapport.pdf'))).toEqual([]);
    });
  });

  it('ignores Escape while a dialog is open', async () => {
    const { backend, textarea } = setup({ status: 'running' });
    app.modal = { kind: 'settings' };
    textarea.focus();
    await fireEvent.keyDown(textarea, { key: 'Escape' });
    expect(backend.called('interrupt')).toHaveLength(0);
  });

  it('brings back the last message of the user’s on ↑, not what Claude Code passed on', async () => {
    const { a, textarea } = setup({}, [
      { kind: 'user', id: 'u1', text: 'Lance les tests', images: 0, ts: 1, queued: false },
      {
        kind: 'user',
        id: 'u2',
        origin: 'remote',
        text: '<task-notification>\n<status>completed</status>\n</task-notification>',
        images: 0,
        ts: 2,
        queued: false,
      },
      { kind: 'event', id: 'e1', source: 'task', text: '<task-notification></task-notification>', ts: 3 },
    ]);
    await waitFor(() => expect(conversationOf(a.id).loaded).toBe(true));
    textarea.focus();
    await userEvent.keyboard('{ArrowUp}');
    expect(textarea).toHaveValue('Lance les tests');
  });

  it('sends the typed message on Enter and clears the field', async () => {
    const { a, backend, textarea } = setup();
    await userEvent.type(textarea, 'Ajoute des tests{Enter}');
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    expect(backend.called('send_message')[0].args).toMatchObject({ id: a.id, text: 'Ajoute des tests', attachments: [] });
    expect(textarea.value).toBe('');
  });

  it('shrinks the text field back once a long message is sent', async () => {
    const { textarea, backend } = setup();
    // jsdom has no layout: the content height follows the text.
    Object.defineProperty(textarea, 'scrollHeight', { configurable: true, get: () => (textarea.value ? 240 : 44) });
    await userEvent.type(textarea, 'Un long message');
    expect(textarea.style.height).toBe('240px');
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    await waitFor(() => expect(textarea.style.height).toBe('44px'));
  });

  it('keeps Shift+Enter as a newline', async () => {
    const { backend, textarea } = setup();
    await userEvent.type(textarea, 'ligne 1{Shift>}{Enter}{/Shift}ligne 2');
    expect(textarea.value).toBe('ligne 1\nligne 2');
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('completes @file mentions from the backend suggestions', async () => {
    const { a, backend, textarea } = setup();
    await userEvent.type(textarea, 'Relis @auth');
    await screen.findByText('auth.ts');
    expect(backend.called('file_suggestions').at(-1)?.args).toEqual({ id: a.id, query: 'auth' });
    await userEvent.keyboard('{Enter}');
    expect(textarea.value).toBe('Relis @src/middleware/auth.ts ');
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('completes a slash command in the middle of a message, and nowhere in a path', async () => {
    const { textarea } = setup();
    await userEvent.type(textarea, 'Relis src/co');
    expect(screen.queryAllByRole('option')).toEqual([]);
    await userEvent.clear(textarea);
    await userEvent.type(textarea, 'Relis le diff puis /com');
    const options = await screen.findAllByRole('option');
    expect(options.map((o) => o.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      '/compact Compacte le contexte',
      '/commit Crée un commit',
    ]);
    await userEvent.keyboard('{ArrowDown}{Tab}');
    expect(textarea.value).toBe('Relis le diff puis /commit ');
  });

  it('answers a pending question with the free text instead of sending a message', async () => {
    const q = {
      kind: 'question',
      id: 'req-1',
      toolUseId: 't1',
      questions: [{ question: 'Quelle base ?', options: [] }],
      answers: null,
      ts: 1,
    };
    const { a, backend, textarea } = setup({ status: 'waiting', pending: ['req-1'] }, [q]);
    await waitFor(() => expect(conversationOf(a.id).loaded).toBe(true));
    await userEvent.type(textarea, 'PostgreSQL{Enter}');
    await waitFor(() => expect(backend.called('answer_question')).toHaveLength(1));
    expect(backend.called('answer_question')[0].args).toEqual({ id: a.id, requestId: 'req-1', answers: { 'Quelle base ?': 'PostgreSQL' } });
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('keeps attached files when the text answers a pending question', async () => {
    const q = {
      kind: 'question',
      id: 'req-1',
      toolUseId: 't1',
      questions: [{ question: 'Quelle base ?', options: [] }],
      answers: null,
      ts: 1,
    };
    const { a, textarea } = setup({ status: 'waiting', pending: ['req-1'] }, [q]);
    await waitFor(() => expect(conversationOf(a.id).loaded).toBe(true));
    const png = new File([new Uint8Array([137, 80, 78, 71])], 'capture.png', { type: 'image/png' });
    await userEvent.upload(screen.getByLabelText('Joindre un fichier', { selector: 'input' }), png);
    await screen.findByAltText('capture.png');
    await userEvent.type(textarea, 'PostgreSQL{Enter}');
    await waitFor(() => expect(app.toasts.at(-1)?.text).toMatch(/fichiers joints/));
    expect(screen.getByAltText('capture.png')).toBeInTheDocument();
  });

  it('attaches a PDF picked from the file dialog and sends it with the message', async () => {
    const { a, backend, textarea } = setup();
    const pdf = new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' });
    await userEvent.upload(screen.getByLabelText('Joindre un fichier', { selector: 'input' }), pdf);
    expect(await screen.findByText('rapport.pdf')).toBeInTheDocument();
    await userEvent.type(textarea, 'Résume ce rapport{Enter}');
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    expect(backend.called('send_message')[0].args).toEqual({
      id: a.id,
      text: 'Résume ce rapport',
      attachments: [{ name: 'rapport.pdf', mediaType: 'application/pdf', data: btoa('%PDF-1.4') }],
      files: [],
    });
    expect(screen.queryByText('rapport.pdf')).not.toBeInTheDocument();
  });

  it('attaches the files picked even though the input empties its list once read', async () => {
    setup();
    const input = screen.getByLabelText('Joindre un fichier', { selector: 'input' }) as HTMLInputElement;
    // As a browser does: the list is the input's own, emptied when its value is cleared.
    const picked = [new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' })];
    Object.defineProperty(input, 'files', { configurable: true, get: () => picked });
    Object.defineProperty(input, 'value', { configurable: true, get: () => '', set: () => picked.splice(0) });
    await fireEvent.change(input);
    expect(await screen.findByText('rapport.pdf')).toBeInTheDocument();
  });

  it('attaches a text file Windows gives no type as text', async () => {
    const { backend } = setup();
    const md = new File(['# Notes\nà faire'], 'notes.md', { type: '' });
    await userEvent.upload(screen.getByLabelText('Joindre un fichier', { selector: 'input' }), md);
    expect(await screen.findByText('notes.md')).toBeInTheDocument();
    // A file alone is enough to send.
    await userEvent.click(screen.getByRole('button', { name: 'Envoyer' }));
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    expect(backend.called('send_message')[0].args.attachments).toEqual([
      { name: 'notes.md', mediaType: 'text/plain', data: '# Notes\nà faire' },
    ]);
  });

  it('offers every file in the dialog, with no filter on its type', () => {
    setup();
    expect(screen.getByLabelText('Joindre un fichier', { selector: 'input' })).not.toHaveAttribute('accept');
  });

  describe('while a file is being read', () => {
    const real = FileReader.prototype.readAsDataURL;
    let finish: () => void = () => {};
    beforeEach(() => {
      FileReader.prototype.readAsDataURL = function (this: FileReader, blob: Blob) {
        finish = () => real.call(this, blob);
      };
    });
    afterEach(() => {
      FileReader.prototype.readAsDataURL = real;
    });
    const pdf = () => new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' });

    it('does not send the message without it', async () => {
      const { backend, textarea } = setup();
      await userEvent.upload(screen.getByLabelText('Joindre un fichier', { selector: 'input' }), pdf());
      await userEvent.type(textarea, 'Résume{Enter}');
      expect(backend.called('send_message')).toHaveLength(0);
      expect(textarea).toHaveValue('Résume');
      finish();
      expect(await screen.findByText('rapport.pdf')).toBeInTheDocument();
      await userEvent.type(textarea, '{Enter}');
      await waitFor(() => expect(backend.called('send_message')[0]?.args.attachments).toHaveLength(1));
    });

    it('keeps it for the agent it was attached to', async () => {
      const a = agent({ id: 'r1' });
      const b = agent({ id: 'r2', name: 'autre' });
      resetApp({ agents: [a, b] });
      fakeBackend({ get_conversation: () => [] });
      const { rerender } = render(Composer, { agent: app.agents.r1 });
      await userEvent.upload(screen.getByLabelText('Joindre un fichier', { selector: 'input' }), pdf());
      await rerender({ agent: app.agents.r2 });
      finish();
      await new Promise((r) => setTimeout(r, 50));
      expect(screen.queryByText('rapport.pdf')).not.toBeInTheDocument();
      await rerender({ agent: app.agents.r1 });
      expect(await screen.findByText('rapport.pdf')).toBeInTheDocument();
    });
  });

  it('gives back the text and the files when the message is refused', async () => {
    const { backend, textarea } = setup();
    fakeBackend({
      send_message: () => {
        throw new Error('rapport.pdf dépasse 18 Mo');
      },
    });
    await userEvent.upload(
      screen.getByLabelText('Joindre un fichier', { selector: 'input' }),
      new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' }),
    );
    await screen.findByText('rapport.pdf');
    await userEvent.type(textarea, 'Résume{Enter}');
    await waitFor(() => expect(app.toasts.at(-1)?.text).toMatch(/dépasse 18 Mo/));
    expect(textarea).toHaveValue('Résume');
    expect(screen.getByText('rapport.pdf')).toBeInTheDocument();
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('takes a dropped file instead of letting the window open it, whatever it is', async () => {
    setup();
    fakeBackend(copying());
    const composer = screen.getByRole('group');
    const drop = (f: File) => {
      const e = createEvent.drop(composer, { dataTransfer: { files: [f], types: ['Files'] } });
      fireEvent(composer, e);
      return e.defaultPrevented;
    };
    expect(drop(new File(['a,b'], 'export.csv', { type: '' }))).toBe(true);
    expect(await screen.findByText('export.csv')).toBeInTheDocument();
    expect(drop(new File(['MZ'], 'setup.exe', { type: 'application/x-msdownload' }))).toBe(true);
    expect(await screen.findByText('setup.exe')).toBeInTheDocument();
    expect(app.toasts).toEqual([]);
  });

  describe('pasting', () => {
    const png = [137, 80, 78, 71];
    const paste = (target: HTMLElement, clipboardData: { files: File[]; types: string[] }) => {
      const e = createEvent.paste(target, { clipboardData });
      fireEvent(target, e);
      return e.defaultPrevented;
    };

    it('attaches a pasted file', async () => {
      const { textarea } = setup();
      const backend = fakeBackend({ get_conversation: () => [] });
      const capture = new File([new Uint8Array(png)], 'capture.png', { type: 'image/png' });
      expect(paste(textarea, { files: [capture], types: ['Files'] })).toBe(true);
      expect(await screen.findByAltText('capture.png')).toBeInTheDocument();
      expect(backend.called('clipboard_image')).toHaveLength(0);
    });

    it('takes the image WebKitGTK leaves out of a paste (Linux) from the system clipboard, and sends nothing without it', async () => {
      const { a, textarea } = setup();
      let give: (png: ArrayBuffer) => void = () => {};
      const backend = fakeBackend({
        get_conversation: () => [],
        clipboard_image: () => new Promise((r) => (give = r)),
      });
      await userEvent.type(textarea, 'Que vois-tu ?');
      // A browser's « Copier l'image »: the page only sees its HTML.
      expect(paste(textarea, { files: [], types: ['text/html'] })).toBe(false);
      await userEvent.type(textarea, '{Enter}');
      expect(backend.called('clipboard_image')).toHaveLength(1);
      expect(backend.called('send_message')).toHaveLength(0);
      give(new Uint8Array(png).buffer);
      expect(await screen.findByAltText('image.png')).toBeInTheDocument();
      await userEvent.type(textarea, '{Enter}');
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      expect(backend.called('send_message')[0].args).toEqual({
        id: a.id,
        text: 'Que vois-tu ?',
        attachments: [{ name: 'image.png', mediaType: 'image/png', data: btoa(String.fromCharCode(...png)) }],
        files: [],
      });
    });

    it('keeps the image read from the system clipboard for the agent it was pasted to', async () => {
      resetApp({ agents: [agent({ id: 'v1' }), agent({ id: 'v2', name: 'autre' })] });
      let give: (png: ArrayBuffer) => void = () => {};
      fakeBackend({ get_conversation: () => [], clipboard_image: () => new Promise((r) => (give = r)) });
      const { rerender } = render(Composer, { agent: app.agents.v1 });
      paste(screen.getByRole('textbox'), { files: [], types: [] });
      await rerender({ agent: app.agents.v2 });
      give(new Uint8Array(png).buffer);
      await new Promise((r) => setTimeout(r, 50));
      expect(screen.queryByAltText('image.png')).not.toBeInTheDocument();
      await rerender({ agent: app.agents.v1 });
      expect(await screen.findByAltText('image.png')).toBeInTheDocument();
    });

    it('attaches nothing when the system clipboard holds no image', async () => {
      const { textarea } = setup();
      const backend = fakeBackend({ get_conversation: () => [], clipboard_image: () => new ArrayBuffer(0) });
      paste(textarea, { files: [], types: [] });
      await waitFor(() => expect(backend.called('clipboard_image')).toHaveLength(1));
      await new Promise((r) => setTimeout(r, 20));
      expect(screen.queryByAltText('image.png')).not.toBeInTheDocument();
      expect(app.toasts).toHaveLength(0);
    });

    it('says why when the system clipboard cannot be read', async () => {
      const { textarea } = setup();
      fakeBackend({
        get_conversation: () => [],
        clipboard_image: () => {
          throw 'le presse-papiers ne répond pas';
        },
      });
      paste(textarea, { files: [], types: [] });
      await waitFor(() => expect(app.toasts.at(-1)?.text).toBe('le presse-papiers ne répond pas'));
    });

    it('reads the system clipboard once for pastes made while it is read', async () => {
      const { textarea } = setup();
      let give: (png: ArrayBuffer) => void = () => {};
      const backend = fakeBackend({ get_conversation: () => [], clipboard_image: () => new Promise((r) => (give = r)) });
      paste(textarea, { files: [], types: [] });
      paste(textarea, { files: [], types: [] });
      paste(textarea, { files: [], types: [] });
      await new Promise((r) => setTimeout(r, 20));
      expect(backend.called('clipboard_image')).toHaveLength(1);
      give(new Uint8Array(png).buffer);
      expect(await screen.findByAltText('image.png')).toBeInTheDocument();
      await new Promise((r) => setTimeout(r, 20));
      expect(screen.getAllByAltText('image.png')).toHaveLength(1);
      // Once answered, the next paste reads it again.
      paste(textarea, { files: [], types: [] });
      await waitFor(() => expect(backend.called('clipboard_image')).toHaveLength(2));
      give(new ArrayBuffer(0));
    });

    it('leaves pasted text to the field without reading the system clipboard', async () => {
      const { textarea } = setup();
      const backend = fakeBackend({ get_conversation: () => [] });
      expect(paste(textarea, { files: [], types: ['text/plain', 'text/html'] })).toBe(false);
      await new Promise((r) => setTimeout(r, 20));
      expect(backend.called('clipboard_image')).toHaveLength(0);
    });
  });

  it('denies a pending permission with the typed explanation', async () => {
    const p = {
      kind: 'permission',
      id: 'req-2',
      toolUseId: 't2',
      toolName: 'Bash',
      input: { command: 'rm -rf dist' },
      canAlways: false,
      defaultNo: false,
      decision: null,
      ts: 1,
    };
    const { a, backend, textarea } = setup({ status: 'waiting', pending: ['req-2'] }, [p]);
    await waitFor(() => expect(conversationOf(a.id).loaded).toBe(true));
    await userEvent.type(textarea, 'Utilise plutôt npm run clean{Enter}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
    expect(backend.called('answer_permission')[0].args).toEqual({
      id: a.id,
      requestId: 'req-2',
      decision: 'deny',
      message: 'Utilise plutôt npm run clean',
    });
  });

  describe('during a turn', () => {
    const question = {
      kind: 'question',
      id: 'req-1',
      toolUseId: 't1',
      questions: [{ question: 'Quelle base ?', options: [] }],
      answers: null,
      ts: 1,
    };
    // What a test leaves held would go once its delay is over, in the next test.
    afterEach(() => {
      for (const id of Object.keys(app.agents)) for (const h of heldOf(id)) cancelHeld(h.id);
    });

    it('holds a message typed while Claude works, and empties the field', async () => {
      const { a, backend, textarea } = setup({ status: 'running' });
      app.settings.midTurnDelaySecs = 10;
      await userEvent.type(textarea, 'Ajoute un test{Enter}');
      expect(heldOf(a.id).map((h) => [h.body, h.mode])).toEqual([['Ajoute un test', 'delay']]);
      expect(textarea.value).toBe('');
      expect(backend.called('send_message')).toEqual([]);
    });

    it('sends at once an answer to a waiting question, even with a delay', async () => {
      const { a, backend, textarea } = setup({ status: 'waiting', pending: ['req-1'] }, [question]);
      app.settings.midTurnDelaySecs = 10;
      await waitFor(() => expect(conversationOf(a.id).loaded).toBe(true));
      await userEvent.type(textarea, 'PostgreSQL{Enter}');
      await waitFor(() => expect(backend.called('answer_question')).toHaveLength(1));
      expect(heldOf(a.id)).toEqual([]);
    });

    it('sends at once while Claude works when the delay is off', async () => {
      const { a, backend, textarea } = setup({ status: 'running' });
      await userEvent.type(textarea, 'Ajoute un test{Enter}');
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      expect(heldOf(a.id)).toEqual([]);
    });

    it('brings an edited message back before what was typed since', async () => {
      const { a, textarea } = setup({ status: 'running' });
      app.settings.midTurnDelaySecs = 10;
      await userEvent.type(textarea, 'a{Enter}');
      await userEvent.type(textarea, 'b');
      editHeld(heldOf(a.id)[0].id);
      await waitFor(() => expect(textarea.value).toBe('a\n\nb'));
      await waitFor(() => expect(document.activeElement).toBe(textarea));
    });

    it('sends a message typed while a held one is on its way only once that one is sent', async () => {
      const { a, textarea } = setup({ status: 'running' });
      let finish = () => {};
      const backend = fakeBackend({
        get_conversation: () => [],
        send_message: (args: any) => (args.text === 'Premier' ? new Promise<void>((r) => (finish = r)) : null),
      });
      app.settings.midTurnDelaySecs = 10;
      await userEvent.type(textarea, 'Premier{Enter}');
      // The turn is over: the next message is not held, but must not overtake the one going.
      app.agents[a.id].status = 'done';
      sendNow(heldOf(a.id)[0].id);
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      await userEvent.type(textarea, 'Second{Enter}');
      await new Promise((r) => setTimeout(r, 20));
      expect(backend.called('send_message').map((c) => c.args.text)).toEqual(['Premier']);
      finish();
      await waitFor(() => expect(backend.called('send_message').map((c) => c.args.text)).toEqual(['Premier', 'Second']));
    });

    it('reads its draft again when messages go back to several drafts at once', async () => {
      const { a, textarea } = setup({ status: 'done' });
      giveBack(a.id, 'Ajoute un test', []);
      giveBack('autre', 'Ailleurs', []);
      await waitFor(() => expect(textarea.value).toBe('Ajoute un test'));
    });

    it('says on « Send » that a message is held behind one still held, the turn over', async () => {
      setLang('en');
      const { a, textarea } = setup({ status: 'running' });
      app.settings.midTurnDelaySecs = 10;
      await userEvent.type(textarea, 'a{Enter}');
      app.agents[a.id].status = 'done';
      await tick();
      expect(screen.getByRole('button', { name: 'Send' })).toHaveAttribute(
        'data-tip',
        'Sent to Claude in 10 s, can be cancelled until then · Enter to send',
      );
    });

    it('says on « Send » that a message sent now can be cancelled until it goes', async () => {
      setLang('en');
      setup({ status: 'running' });
      app.settings.midTurnDelaySecs = 10;
      await waitFor(() =>
        expect(screen.getByRole('button', { name: 'Send' })).toHaveAttribute(
          'data-tip',
          'Sent to Claude in 10 s, can be cancelled until then · Enter to send',
        ),
      );
    });
  });

  it('interrupts a running agent on Escape', async () => {
    const { a, backend, textarea } = setup({ status: 'running' });
    textarea.focus();
    await fireEvent.keyDown(textarea, { key: 'Escape' });
    expect(backend.called('interrupt')).toEqual([{ cmd: 'interrupt', args: { id: a.id } }]);
  });

  it('switches the permission mode from the mode menu', async () => {
    const { a, backend } = setup({ mode: 'auto' });
    await userEvent.click(screen.getByRole('button', { name: 'Mode : Auto' }));
    expect(screen.getByRole('menuitemradio', { name: /Auto/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('menuitemradio', { name: /Plan/ }));
    expect(backend.called('set_agent_options').at(-1)?.args).toMatchObject({ id: a.id, mode: 'plan' });
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('shows the ask-every-time mode Claude falls back to after a plan', async () => {
    const { a, backend } = setup({ mode: 'default' });
    await userEvent.click(screen.getByRole('button', { name: 'Mode : Demander' }));
    expect(screen.getByRole('menuitemradio', { name: /Demander/ })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('menuitemradio', { name: /Auto/ }));
    expect(backend.called('set_agent_options').at(-1)?.args).toMatchObject({ id: a.id, mode: 'auto' });
  });

  it('disables the effort for Haiku, and keeps the auto mode of its alias before its account’s claude told', async () => {
    setup({ model: 'haiku', mode: 'acceptEdits' });
    expect(screen.getByRole('button', { name: /^Effort/ })).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    expect(screen.getByRole('menuitemradio', { name: /Opus/ })).toBeEnabled();
    // Nothing told yet: its alias runs Haiku 5.5, with the mode, from Claude Code 2.1.293 on.
    await userEvent.click(screen.getByRole('button', { name: /^Mode/ }));
    expect(screen.getByRole('menuitemradio', { name: /Auto/ })).toBeEnabled();
  });

  it('disables the auto mode where the claude of the agent’s account says its Haiku has none', async () => {
    const { a } = setup({ model: 'haiku', mode: 'acceptEdits', account: 'principal' });
    // Its account's claude is a 2.1.292: its haiku runs Haiku 4.5, without the mode (ESC-121).
    app.accountModels = {
      [a.account]: {
        version: '2.1.292',
        models: [
          { value: 'opus', resolvedModel: 'claude-opus-5-5', supportsAutoMode: true },
          { value: 'haiku', resolvedModel: 'claude-haiku-4-5-20251001' },
        ],
      },
    };
    await userEvent.click(screen.getByRole('button', { name: /^Mode/ }));
    expect(screen.getByRole('menuitemradio', { name: /Auto/ })).toBeDisabled();
    expect(screen.getByText('indisponible avec ce modèle')).toBeInTheDocument();
    expect(screen.getByRole('menuitemradio', { name: /Plan/ })).toBeEnabled();
  });

  it('offers the models the claude of the agent’s account told: one more, one less, each with its efforts (ESC-124)', async () => {
    const { a } = setup({ model: 'mythos', effort: 'high', account: 'pro' });
    const told = (id: string, label: string, efforts: string[]) => ({
      id,
      label,
      efforts,
      defaultEffort: null,
      images: true,
      default: false,
    });
    app.accountModels = {
      pro: {
        version: '2.1.302',
        models: [],
        catalog: [told('opus', 'Opus 5.5', ['low', 'medium', 'high', 'xhigh', 'max']), told('mythos', 'Mythos', ['high', 'max'])],
      },
      // Another account's: not this agent's.
      principal: { version: '2.1.302', models: [], catalog: [told('fable', 'Fable 5.1', ['low'])] },
    };
    expect(a.account).toBe('pro');
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    const names = screen.getAllByRole('menuitemradio').map((o) => o.textContent?.trim());
    expect(names.some((n) => n?.startsWith('Mythos'))).toBe(true);
    expect(names.some((n) => n?.startsWith('Opus 5.5'))).toBe(true);
    expect(names.some((n) => n?.startsWith('Fable'))).toBe(false);
    expect(names.some((n) => n?.startsWith('Sonnet'))).toBe(false);
    await userEvent.keyboard('{Escape}');
    await userEvent.click(screen.getByRole('button', { name: /^Effort/ }));
    expect(screen.getAllByRole('menuitemradio').map((o) => o.textContent?.trim())).toEqual([
      'Élevé Réflexion approfondie',
      'Max Réflexion maximale, plus de tokens',
    ]);
  });
});

describe('Composer: a message sent while the one before is still on its way', () => {
  it('sends it once the one before is taken, rather than drop it in silence', async () => {
    const a = agent({ id: `q${Math.random()}`, status: 'idle' });
    resetApp({ agents: [a] });
    let release!: () => void;
    const first = new Promise<void>((r) => (release = r));
    let n = 0;
    const backend = fakeBackend({ get_conversation: () => [], send_message: () => (n++ === 0 ? first : null) });
    render(Composer, { agent: app.agents[a.id] });
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    await userEvent.type(textarea, 'un{Enter}');
    await userEvent.type(textarea, 'deux{Enter}');
    // Gone from the field at once, as the first one: it waits for its turn, not for the user.
    expect(textarea).toHaveValue('');
    expect(backend.called('send_message').map((c) => c.args.text)).toEqual(['un']);
    release();
    await waitFor(() => expect(backend.called('send_message').map((c) => c.args.text)).toEqual(['un', 'deux']));
    expect(app.toasts).toEqual([]);
  });
});

describe('Composer in a narrow column (split layout)', () => {
  const Real = globalThis.ResizeObserver;
  beforeEach(() => {
    resetApp();
    // The composer reports a 400 px width, as in half of a small window.
    globalThis.ResizeObserver = class {
      constructor(private cb: ResizeObserverCallback) {}
      observe() {
        this.cb([{ contentRect: { width: 400 } } as ResizeObserverEntry], this as unknown as ResizeObserver);
      }
      unobserve() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
  });
  afterEach(() => {
    globalThis.ResizeObserver = Real;
  });

  it('drops the menu captions to keep the send button on the same line', () => {
    setup({ model: 'sonnet', effort: 'medium', mode: 'auto' });
    const model = screen.getByRole('button', { name: 'Modèle : Sonnet' });
    expect(model).toHaveTextContent('Sonnet');
    expect(model).not.toHaveTextContent('Modèle');
    expect(screen.getByRole('button', { name: 'Mode : Auto' })).not.toHaveTextContent('Mode');
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeInTheDocument();
  });

  it('shrinks the stop button to its icon while Claude works', () => {
    setup({ status: 'running' });
    const stop = screen.getByRole('button', { name: 'Stop' });
    expect(stop.querySelector('[data-icon="stop"]')).not.toBeNull();
    expect(stop).not.toHaveTextContent('Stop');
    expect(stop).toHaveClass('icon');
    // Claude takes a message sent during a turn at its next step: it is sent, not queued.
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeInTheDocument();
    // 400 px: « Envoyer » keeps its key.
    expect(screen.getByRole('button', { name: 'Envoyer' }).querySelector('kbd')).not.toBeNull();
  });

  it('drops the key of « Envoyer » in the narrowest column (the panel beside a widened projects’ rail)', () => {
    globalThis.ResizeObserver = class {
      constructor(private cb: ResizeObserverCallback) {}
      observe() {
        this.cb([{ contentRect: { width: 260 } } as ResizeObserverEntry], this as unknown as ResizeObserver);
      }
      unobserve() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
    setup({ status: 'running' });
    const send = screen.getByRole('button', { name: 'Envoyer' });
    expect(send).toHaveTextContent('Envoyer');
    expect(send.querySelector('kbd')).toBeNull();
    expect(screen.getByRole('button', { name: 'Stop' })).toHaveClass('icon');
  });
});

describe('Composer in English', () => {
  beforeEach(() => resetApp());

  it('speaks English in the field, the bar and the menus, and keeps the shortcuts of the keyboard', async () => {
    setLang('en');
    const { textarea } = setup({ model: 'opus', effort: 'high', mode: 'plan' });
    expect(textarea).toHaveAttribute('placeholder', 'Describe the task to give to Claude…');
    expect(screen.getByRole('button', { name: 'Send' })).toHaveAttribute('data-tip', 'Enter to send · Shift+Enter for a new line');
    expect(screen.getByRole('button', { name: 'Attach a file' })).toHaveAttribute('data-tip', 'Attach a file (or paste / drop it)');
    // “Mode” alone: “Model” begins the same.
    await userEvent.click(screen.getByRole('button', { name: /^Mode\b/ }));
    expect(screen.getByText('Accept edits')).toBeInTheDocument();
    expect(screen.getByText('Claude analyzes and proposes a plan without changing anything')).toBeInTheDocument();
  });

  it('says in English that Haiku has no effort, Haiku 4.5 no Auto mode, and what to do while Claude works', async () => {
    setLang('en');
    setup({ model: 'claude-haiku-4-5', status: 'running' });
    expect(screen.getByRole('button', { name: /^Effort/ })).toHaveAttribute('data-tip', 'Haiku doesn’t support effort');
    expect(screen.getByRole('button', { name: 'Stop' })).toHaveAttribute('data-tip', 'Interrupt (Esc)');
    expect(screen.getByRole('button', { name: 'Send' })).toHaveAttribute(
      'data-tip',
      'Claude takes it into account at its next step · Enter to send',
    );
    await userEvent.click(screen.getByRole('button', { name: /^Mode\b/ }));
    expect(screen.getByText('unavailable with this model')).toBeInTheDocument();
  });

  it('refuses a file over the maximum size in English, and attaches any other', async () => {
    setLang('en');
    setup();
    fakeBackend(copying());
    app.settings.attachMaxMb = 1;
    const input = screen.getByLabelText('Attach a file', { selector: 'input' });
    await userEvent.upload(input, sized('film.mkv', 2 * MB));
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'film.mkv is over 1 MB' }));
    expect(screen.queryByText('film.mkv')).not.toBeInTheDocument();
    await userEvent.upload(input, new File([new Uint8Array([80, 75, 3, 4])], 'sources.zip', { type: 'application/zip' }));
    expect(await screen.findByRole('button', { name: 'Remove sources.zip' })).toBeInTheDocument();
    await userEvent.upload(input, new File(['%PDF-1.4'], 'a.pdf', { type: 'application/pdf' }));
    expect(await screen.findByRole('button', { name: 'Remove a.pdf' })).toBeInTheDocument();
  });
});

const MB = 1024 * 1024;
/** A file that says it has `size` bytes, without holding them. */
const sized = (name: string, size: number, type = '') => {
  const f = new File(['x'], name, { type });
  Object.defineProperty(f, 'size', { value: size });
  return f;
};
/** The file a copy makes, as the backend gives it. */
const copyOf = (name: string, over: Partial<AttachedFile> = {}): AttachedFile => ({
  name,
  size: 4,
  origin: false,
  path: `/data/attachments/ag/1f2e3d4c/${name}`,
  claudePath: `/data/attachments/ag/1f2e3d4c/${name}`,
  ...over,
});
/** The backend of a copy: each file gets its handle, named after it, and comes back as its copy. */
const copying = (over: Record<string, (a: any) => unknown> = {}) => ({
  get_conversation: () => [],
  attach_open: ({ name }: { name: string }) => `h:${name}`,
  attach_finish: ({ handle }: { handle: string }) => copyOf(handle.slice(2)),
  ...over,
});

describe('Composer: files attached by their path', () => {
  const zip = (name = 'sources.zip') => new File([new Uint8Array([80, 75, 3, 4])], name, { type: 'application/zip' });
  const input = () => screen.getByLabelText('Joindre un fichier', { selector: 'input' });
  const drop = (...files: File[]) => {
    const dataTransfer = { files, types: ['Files'] };
    const composer = screen.getByRole('group');
    const e = createEvent.drop(composer, { dataTransfer });
    fireEvent(composer, e);
    return { dataTransfer, prevented: e.defaultPrevented };
  };
  /** The copy of a big file whose second chunk waits for `release`. */
  const stalled = () => {
    let release: () => void = () => {};
    let chunks = 0;
    const backend = fakeBackend(copying({ attach_chunk: () => (++chunks === 2 ? new Promise<void>((r) => (release = r)) : undefined) }));
    // Once the second chunk is asked for, which is the one that waits.
    const stuck = () => waitFor(() => expect(backend.called('attach_chunk')).toHaveLength(2));
    return { backend, stuck, release: () => release() };
  };

  it('copies a .zip dropped on the field to the agent’s folder, shows it as a chip and sends its path with the message', async () => {
    const { a, textarea } = setup();
    const backend = fakeBackend(copying());
    expect(drop(zip()).prevented).toBe(true);
    expect(await screen.findByText('sources.zip')).toBeInTheDocument();
    expect(screen.getByText('1 Ko')).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole('progressbar')).not.toBeInTheDocument());
    expect(backend.called('attach_open')[0].args).toEqual({ owner: a.id, name: 'sources.zip', size: 4 });
    expect([...(backend.called('attach_chunk')[0].args as unknown as Uint8Array)]).toEqual([80, 75, 3, 4]);
    await userEvent.type(textarea, 'Que contient cette archive ?{Enter}');
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    expect(backend.called('send_message')[0].args).toEqual({
      id: a.id,
      text: 'Que contient cette archive ?',
      attachments: [],
      files: [copyOf('sources.zip')],
    });
    expect(screen.queryByText('sources.zip')).not.toBeInTheDocument();
  });

  it('sends a file alone, with no text', async () => {
    const { a } = setup();
    const backend = fakeBackend(copying());
    await userEvent.upload(input(), new File(['PK'], 'budget.xlsx', { type: 'application/vnd.ms-excel.sheet.macroEnabled.12' }));
    await screen.findByText('budget.xlsx');
    await waitFor(() => expect(screen.getByRole('button', { name: 'Envoyer' })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: 'Envoyer' }));
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    expect(backend.called('send_message')[0].args).toMatchObject({ id: a.id, text: '', attachments: [], files: [copyOf('budget.xlsx')] });
  });

  it('sends the files in the message and those by their path together', async () => {
    setup();
    const backend = fakeBackend(copying());
    drop(new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' }), zip(), new File(['# Notes'], 'notes.md'));
    await screen.findByText('notes.md');
    await waitFor(() => expect(screen.getByRole('button', { name: 'Envoyer' })).toBeEnabled());
    await userEvent.click(screen.getByRole('button', { name: 'Envoyer' }));
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    const { attachments, files } = backend.called('send_message')[0].args;
    expect(attachments.map((x: { name: string }) => x.name)).toEqual(['rapport.pdf', 'notes.md']);
    expect(files).toEqual([copyOf('sources.zip')]);
  });

  it('shows the progress of the copy and does not send before it is done', async () => {
    const { textarea } = setup();
    let open: (handle: string) => void = () => {};
    const backend = fakeBackend(copying({ attach_open: () => new Promise((r) => (open = r)) }));
    drop(zip());
    expect(await screen.findByRole('progressbar', { name: 'Copie de sources.zip' })).toHaveAttribute('aria-valuenow', '0');
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeDisabled();
    await userEvent.type(textarea, 'Regarde{Enter}');
    expect(backend.called('send_message')).toHaveLength(0);
    expect(textarea).toHaveValue('Regarde');
    open('h:sources.zip');
    await waitFor(() => expect(screen.queryByRole('progressbar')).not.toBeInTheDocument());
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeEnabled();
    await userEvent.click(screen.getByRole('button', { name: 'Envoyer' }));
    await waitFor(() => expect(backend.called('send_message')[0]?.args.files).toHaveLength(1));
  });

  it('says how far the copy of a big file went, a chunk at a time', async () => {
    setup();
    const { release, stuck } = stalled();
    drop(sized('film.mkv', 10 * MB, 'video/x-matroska'));
    const bar = await screen.findByRole('progressbar', { name: 'Copie de film.mkv' });
    await stuck();
    expect(bar).toHaveAttribute('aria-valuenow', '40');
    expect(screen.getByText('40 %')).toBeInTheDocument();
    release();
    await waitFor(() => expect(screen.queryByRole('progressbar')).not.toBeInTheDocument());
  });

  it('erases the copy when its chip is taken away', async () => {
    const { a } = setup();
    const backend = fakeBackend(copying());
    drop(zip());
    await screen.findByText('sources.zip');
    await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
    await waitFor(() => expect(screen.queryByRole('progressbar')).not.toBeInTheDocument());
    await userEvent.click(screen.getByRole('button', { name: 'Retirer sources.zip' }));
    expect(screen.queryByText('sources.zip')).not.toBeInTheDocument();
    expect(backend.called('attach_discard').map((c) => c.args)).toEqual([{ owner: a.id, path: copyOf('sources.zip').path }]);
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeDisabled();
  });

  it('cancels the copy when its chip is taken away while it goes on, without a word', async () => {
    setup();
    const { backend, release, stuck } = stalled();
    drop(sized('film.mkv', 10 * MB, 'video/x-matroska'));
    await stuck();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '40');
    await userEvent.click(screen.getByRole('button', { name: 'Retirer film.mkv' }));
    expect(screen.queryByText('film.mkv')).not.toBeInTheDocument();
    release();
    await waitFor(() => expect(backend.called('attach_cancel')).toHaveLength(1));
    expect(backend.called('attach_cancel')[0].args).toEqual({ handle: 'h:film.mkv' });
    expect(backend.called('attach_finish')).toHaveLength(0);
    expect(backend.called('attach_discard')).toHaveLength(0);
    expect(app.toasts).toEqual([]);
    expect(screen.queryByText('film.mkv')).not.toBeInTheDocument();
  });

  it('takes the chip of a copy that failed away and says why', async () => {
    setup();
    fakeBackend(
      copying({
        attach_open: () => {
          throw 'film.mkv dépasse 100 Mo';
        },
      }),
    );
    drop(sized('film.mkv', 2 * MB));
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'film.mkv dépasse 100 Mo' }));
    expect(screen.queryByText('film.mkv')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeDisabled();
  });

  it('gives the chip back, its copy kept, when the message is refused', async () => {
    const { textarea } = setup();
    const backend = fakeBackend(
      copying({
        send_message: () => {
          throw new Error('Claude Code est introuvable');
        },
      }),
    );
    drop(zip());
    await screen.findByText('sources.zip');
    await waitFor(() => expect(screen.getByRole('button', { name: 'Envoyer' })).toBeEnabled());
    await userEvent.type(textarea, 'Regarde{Enter}');
    await waitFor(() => expect(app.toasts.at(-1)?.text).toMatch(/introuvable/));
    expect(textarea).toHaveValue('Regarde');
    expect(screen.getByRole('button', { name: 'Retirer sources.zip' })).toBeInTheDocument();
    expect(backend.called('attach_discard')).toHaveLength(0);
    // Taken away after all: the copy goes.
    await userEvent.click(screen.getByRole('button', { name: 'Retirer sources.zip' }));
    expect(backend.called('attach_discard')).toHaveLength(1);
  });

  it('keeps the copy for the agent it was made for, and does not block the one shown meanwhile', async () => {
    resetApp({ agents: [agent({ id: 'w1' }), agent({ id: 'w2', name: 'autre' })] });
    let open: (handle: string) => void = () => {};
    const backend = fakeBackend(copying({ attach_open: () => new Promise((r) => (open = r)) }));
    const { rerender } = render(Composer, { agent: app.agents.w1 });
    drop(zip());
    await screen.findByRole('progressbar');
    await rerender({ agent: app.agents.w2 });
    expect(screen.queryByText('sources.zip')).not.toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox'), 'Pour w2');
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeEnabled();
    open('h:sources.zip');
    await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByText('sources.zip')).not.toBeInTheDocument();
    await rerender({ agent: app.agents.w1 });
    expect(await screen.findByRole('button', { name: 'Retirer sources.zip' })).toBeInTheDocument();
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    expect(backend.called('attach_open')[0].args.owner).toBe('w1');
  });

  it('copies a file pasted from the system’s clipboard, as one dropped', async () => {
    const { textarea } = setup();
    const backend = fakeBackend(copying());
    const clipboardData = { files: [zip()], types: ['Files'] };
    const e = createEvent.paste(textarea, { clipboardData });
    fireEvent(textarea, e);
    expect(e.defaultPrevented).toBe(true);
    expect(await screen.findByText('sources.zip')).toBeInTheDocument();
    // Only the files: the page has no data of the drop or the paste to hand over, a path of its own least of all.
    expect(fileOrigins).toHaveBeenCalledWith(clipboardData.files);
    await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
  });

  describe('a file the host knows', () => {
    const original = copyOf('sources.zip', { origin: true, path: '/home/jean/sources.zip', claudePath: '/home/jean/sources.zip' });
    const originMark = { name: 'Fichier d’origine, lu là où il est' };

    it('is attached where it is by its id, with nothing copied, and its chip says so', async () => {
      const { a, textarea } = setup();
      const backend = fakeBackend(copying({ attach_origin: () => original }));
      vi.mocked(fileOrigins).mockResolvedValue(['8c1f0a52-3b7e-4d49-a1c6-2f9d5e7b0a13']);
      drop(zip());
      await screen.findByText('sources.zip');
      await waitFor(() => expect(backend.called('attach_origin')).toHaveLength(1));
      expect(fileOrigins).toHaveBeenCalledWith([expect.objectContaining({ name: 'sources.zip' })]);
      // The backend is given the id the host holds, and finds the path itself.
      expect(backend.called('attach_origin')[0].args).toEqual({ owner: a.id, id: '8c1f0a52-3b7e-4d49-a1c6-2f9d5e7b0a13' });
      expect(backend.called('attach_open')).toHaveLength(0);
      expect(await screen.findByRole('img', originMark)).toBeInTheDocument();
      await waitFor(() => expect(screen.getByRole('button', { name: 'Envoyer' })).toBeEnabled());
      await userEvent.type(textarea, 'Regarde{Enter}');
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      expect(backend.called('send_message')[0].args.files).toEqual([original]);
    });

    it('is attached where it is while the others of the same drop are copied, a copy having no mark', async () => {
      const { a } = setup();
      const backend = fakeBackend(
        copying({
          attach_origin: ({ id }: { id: string }) => copyOf(id === 'id-a' ? 'a.zip' : 'b.zip', { origin: true }),
        }),
      );
      // The host knows the first file alone: the second is copied.
      vi.mocked(fileOrigins).mockResolvedValue(['id-a', null]);
      drop(zip('a.zip'), zip('b.zip'));
      await screen.findByText('b.zip');
      await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
      await waitFor(() => expect(screen.queryByRole('progressbar')).not.toBeInTheDocument());
      expect(backend.called('attach_origin').map((c) => c.args)).toEqual([{ owner: a.id, id: 'id-a' }]);
      expect(backend.called('attach_open').map((c) => c.args.name)).toEqual(['b.zip']);
      // Only the original file has its mark.
      expect(screen.getAllByRole('img', originMark)).toHaveLength(1);
      expect(screen.getByText('a.zip').closest('.att')).toContainElement(screen.getByRole('img', originMark));
    });

    it('is never erased when its chip is taken away', async () => {
      setup();
      const backend = fakeBackend(copying({ attach_origin: () => original }));
      vi.mocked(fileOrigins).mockResolvedValue(['id-sources']);
      drop(zip());
      await screen.findByText('sources.zip');
      await waitFor(() => expect(screen.getByRole('button', { name: 'Envoyer' })).toBeEnabled());
      await userEvent.click(screen.getByRole('button', { name: 'Retirer sources.zip' }));
      expect(screen.queryByText('sources.zip')).not.toBeInTheDocument();
      expect(backend.called('attach_discard')).toHaveLength(0);
    });

    it('is copied when the setting says to, and its path is not even looked for', async () => {
      setup();
      app.settings.attachOrigin = false;
      const backend = fakeBackend(copying());
      drop(zip());
      await screen.findByText('sources.zip');
      await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
      expect(fileOrigins).not.toHaveBeenCalled();
      expect(backend.called('attach_origin')).toHaveLength(0);
    });

    it('is copied when the host does not know it, and its chip has no mark', async () => {
      setup();
      const backend = fakeBackend(copying());
      vi.mocked(fileOrigins).mockResolvedValue([null]);
      drop(zip());
      await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
      expect(backend.called('attach_origin')).toHaveLength(0);
      await waitFor(() => expect(screen.queryByRole('progressbar')).not.toBeInTheDocument());
      expect(screen.getByText('sources.zip')).toBeInTheDocument();
      expect(screen.queryByRole('img', originMark)).not.toBeInTheDocument();
    });

    it('is not looked for what goes in the message anyway: a small PDF is not copied', async () => {
      setup();
      const backend = fakeBackend(copying());
      drop(new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' }));
      await screen.findByText('rapport.pdf');
      expect(backend.called('attach_open')).toHaveLength(0);
      expect(backend.called('attach_origin')).toHaveLength(0);
    });

    it('is said to be refused by the backend (an id it refuses, a file it cannot read), and its chip taken away, nothing copied', async () => {
      setup();
      const backend = fakeBackend(
        copying({
          attach_origin: () => {
            throw 'sources.zip est illisible';
          },
        }),
      );
      vi.mocked(fileOrigins).mockResolvedValue(['id-sources']);
      drop(zip());
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'sources.zip est illisible' }));
      expect(screen.queryByText('sources.zip')).not.toBeInTheDocument();
      expect(backend.called('attach_open')).toHaveLength(0);
    });
  });

  describe('what goes by the path and what does not', () => {
    it('sends by its path the file the message has no more room for, after the first that fits', async () => {
      setup();
      const backend = fakeBackend(copying());
      const pdf = (name: string) => sized(name, 10 * MB, 'application/pdf');
      await userEvent.upload(input(), pdf('a.pdf'));
      expect(await screen.findByText('a.pdf')).toBeInTheDocument();
      expect(backend.called('attach_open')).toHaveLength(0);
      await userEvent.upload(input(), pdf('b.pdf'));
      expect(await screen.findByText('b.pdf')).toBeInTheDocument();
      await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
      expect(backend.called('attach_open').map((c) => c.args.name)).toEqual(['b.pdf']);
      expect(app.toasts).toEqual([]);
      await userEvent.click(screen.getByRole('button', { name: 'Envoyer' }));
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      const { attachments, files } = backend.called('send_message')[0].args;
      expect(attachments.map((x: { name: string }) => x.name)).toEqual(['a.pdf']);
      expect(files.map((x: { name: string }) => x.name)).toEqual(['b.pdf']);
    });

    it('counts the files dropped together one after the other against the 18 Mo of the message', async () => {
      setup();
      const backend = fakeBackend(copying());
      drop(sized('a.pdf', 10 * MB, 'application/pdf'), sized('b.pdf', 10 * MB, 'application/pdf'));
      await screen.findByText('b.pdf');
      await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
      expect(backend.called('attach_open').map((c) => c.args.name)).toEqual(['b.pdf']);
    });

    it('sends a text that is none by its path, the file being binary', async () => {
      setup();
      const backend = fakeBackend(copying());
      await userEvent.upload(input(), new File([new Uint8Array([0x61, 0, 0x62])], 'binaire.log'));
      expect(await screen.findByText('binaire.log')).toBeInTheDocument();
      await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
      expect(backend.called('attach_open')[0].args.name).toBe('binaire.log');
      expect(app.toasts).toEqual([]);
    });

    it('sends a text over 256 Ko by its path, or refuses it when the setting says so', async () => {
      setup();
      const backend = fakeBackend(copying());
      await userEvent.upload(input(), sized('journal.log', 300 * 1024));
      expect(await screen.findByText('journal.log')).toBeInTheDocument();
      await waitFor(() => expect(backend.called('attach_finish')).toHaveLength(1));
      app.settings.attachBigText = 'refuse';
      await userEvent.upload(input(), sized('autre.log', 300 * 1024));
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'autre.log dépasse 256 Ko' }));
      expect(screen.queryByText('autre.log')).not.toBeInTheDocument();
    });

    it('refuses a file over the maximum size without opening a copy', async () => {
      setup();
      app.settings.attachMaxMb = 1;
      const backend = fakeBackend(copying());
      await userEvent.upload(input(), sized('film.mkv', 2 * MB));
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'film.mkv dépasse 1 Mo' }));
      expect(screen.queryByText('film.mkv')).not.toBeInTheDocument();
      expect(backend.called('attach_open')).toHaveLength(0);
    });

    it('warns of a file past the threshold, and still attaches it', async () => {
      setup();
      app.settings.attachWarnMb = 1;
      fakeBackend(copying());
      await userEvent.upload(input(), sized('film.mkv', 2 * MB));
      expect(await screen.findByRole('img', { name: 'Gros fichier : 2 Mo, copié sur le disque' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Retirer film.mkv' })).toBeInTheDocument();
      expect(app.toasts).toEqual([]);
    });
  });
});

describe('Composer of an agent on another harness', () => {
  function bare(over: Parameters<typeof agent>[0] = {}) {
    const a = agent({ id: `c${Math.random()}`, engine: 'codex', ...over });
    resetApp({ agents: [a] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    const backend = fakeBackend({ get_conversation: () => [], get_commands: () => [{ name: 'compact', description: 'Compacte' }] });
    render(Composer, { agent: app.agents[a.id] });
    return { backend, textarea: screen.getByRole('textbox') as HTMLTextAreaElement };
  }

  it('hides the effort its harness does not take, rather than greying it out', () => {
    bare({ model: 'opus' });
    expect(screen.queryByRole('button', { name: /^Effort/ })).toBeNull();
    expect(screen.getByRole('button', { name: /^Mode/ })).toBeInTheDocument();
  });

  it('keeps the five modes in their places, those its harness lacks greyed with why, the others with what they mean there (choice 17)', async () => {
    bare({ mode: 'default' });
    await userEvent.click(screen.getByRole('button', { name: /^Mode/ }));
    const modes = screen.getAllByRole('menuitemradio');
    expect(modes.map((m) => m.querySelector('.ml b, b')?.textContent ?? m.textContent)).toHaveLength(5);
    const auto = screen.getByRole('menuitemradio', { name: /Auto/ });
    expect(auto).toBeDisabled();
    expect(auto).toHaveTextContent('Harness nu n’a pas de mode Auto : rien ne décide à ta place.');
    expect(screen.getByRole('menuitemradio', { name: /Édits auto/ })).toHaveTextContent('Harness nu n’a pas de mode Édits auto.');
    expect(screen.getByRole('menuitemradio', { name: /Plan/ })).toHaveTextContent('Lecture seule, dans un bac à sable');
    expect(screen.getByRole('menuitemradio', { name: /Plan/ })).toBeEnabled();
  });

  it('suggests no / command when its harness offers none', async () => {
    const { backend, textarea } = bare();
    await userEvent.type(textarea, '/com');
    await new Promise((r) => setTimeout(r, 50));
    expect(backend.called('get_commands')).toEqual([]);
    expect(screen.queryByText('/compact')).toBeNull();
  });
});

describe('Composer: the menus come from the agent’s harness (ESC-29)', () => {
  it('lists the models its harness declares, and Claude Code’s aliases for an agent on Claude Code, as before', async () => {
    const a = agent({ id: 'cc', model: 'opus' });
    const b = agent({ id: 'cx', engine: 'codex', model: 'gpt-5.5' });
    resetApp({ agents: [a, b] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    fakeBackend({ get_conversation: () => [] });
    const { unmount } = render(Composer, { agent: app.agents.cc });
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    expect(screen.getAllByRole('menuitemradio').map((m) => m.textContent?.trim())).toEqual(['Fable', 'Opus', 'Sonnet', 'Haiku']);
    expect(screen.getAllByRole('menuitemradio')[1]).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('button', { name: /^Effort/ }));
    expect(screen.getAllByRole('menuitemradio')).toHaveLength(5);
    unmount();
    render(Composer, { agent: app.agents.cx });
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    expect(screen.getAllByRole('menuitemradio').map((m) => m.querySelector('.ml')?.textContent)).toEqual(['GPT-5.5', 'GPT-5.5 mini']);
  });
});

describe('Composer: the models of a Codex account (ESC-186)', () => {
  const told = (id: string, label: string, isDefault = false) => ({
    id,
    label,
    efforts: ['low', 'medium'],
    defaultEffort: 'medium',
    images: false,
    default: isDefault,
  });

  function codex() {
    const a = agent({ id: 'cx', engine: 'codex', model: 'gpt-5.5', account: 'travail' });
    resetApp({ agents: [a] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    const backend = fakeBackend({ get_conversation: () => [], account_models_refresh: () => null });
    render(Composer, { agent: app.agents.cx });
    return backend;
  }
  const menu = () => screen.getAllByRole('menuitemradio');

  it('says its stand-in is no more than that while the account’s list is being read, and asks for it as the menu opens', async () => {
    const backend = codex();
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    expect(menu().map((m) => m.textContent?.trim())).toEqual([
      'GPT-5.5 liste en cours de lecture',
      'GPT-5.5 mini liste en cours de lecture',
    ]);
    expect(backend.called('account_models_refresh').map((c) => c.args)).toEqual([{ id: 'travail' }]);
  });

  it('offers every model its account listed, kept by account', async () => {
    codex();
    app.accountModels = {
      travail: {
        version: '0.163.0',
        models: [],
        catalog: [told('gpt-6-luna', 'GPT-6-Luna', true), told('gpt-5.6-terra', 'GPT-5.6-Terra')],
      },
      perso: { version: '0.163.0', models: [], catalog: [told('gpt-6-sol', 'GPT-6-Sol')] },
    };
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    expect(menu().map((m) => m.textContent?.trim())).toEqual(['GPT-6-Luna', 'GPT-5.6-Terra']);
  });

  it('reads the models of a Copilot, Cursor or OpenCode account as the menu opens, its list said being read until then', async () => {
    for (const engine of ['copilot', 'cursor', 'opencode'] as const) {
      const a = agent({ id: `m-${engine}`, engine, model: '', account: `compte-${engine}` });
      resetApp({ agents: [a] });
      app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, id: engine, models: [] }];
      const backend = fakeBackend({ get_conversation: () => [], account_models_refresh: () => null });
      const { unmount } = render(Composer, { agent: app.agents[a.id] });
      await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
      // No stand-in (OpenCode's): one greyed line says the list is being read.
      const line = screen.getByRole('menuitemradio', { name: /liste en cours de lecture/ });
      expect(line).toBeDisabled();
      expect(backend.called('account_models_refresh').map((c) => c.args)).toEqual([{ id: `compte-${engine}` }]);
      unmount();
    }
  });

  it('marks the model its config.toml names, which Codex does not list, and lets it be picked', async () => {
    const backend = codex();
    backend.calls.length = 0;
    app.accountModels = {
      travail: {
        version: '0.163.0',
        models: [],
        catalog: [told('gpt-6-luna', 'GPT-6-Luna', true), { ...told('gpt-5.6-sol', 'gpt-5.6-sol'), efforts: [], fromConfig: true }],
      },
    };
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    expect(menu().map((m) => m.textContent?.trim())).toEqual(['GPT-6-Luna', 'gpt-5.6-sol de config.toml']);
    await userEvent.click(screen.getByRole('menuitemradio', { name: /gpt-5.6-sol/ }));
    expect(backend.calls.some((c) => JSON.stringify(c.args).includes('gpt-5.6-sol'))).toBe(true);
  });
});
