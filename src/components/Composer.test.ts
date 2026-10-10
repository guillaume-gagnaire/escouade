import { createEvent, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { conversationOf } from '../lib/conversations.svelte';
import { setLang } from '../lib/i18n';
import { readPref, writePref } from '../lib/prefs';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../test/ipc';
import Composer from './Composer.svelte';

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

  it('completes slash commands only at the start of the message', async () => {
    const { textarea } = setup();
    await userEvent.type(textarea, '/com');
    const options = await screen.findAllByRole('option');
    expect(options.map((o) => o.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      '/compact Compacte le contexte',
      '/commit Crée un commit',
    ]);
    await userEvent.keyboard('{ArrowDown}{Tab}');
    expect(textarea.value).toBe('/commit ');
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
    });
    expect(screen.queryByText('rapport.pdf')).not.toBeInTheDocument();
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

  it('refuses an unsupported file with a message saying what can be attached', async () => {
    setup();
    // "All files" chosen in the dialog.
    const user = userEvent.setup({ applyAccept: false });
    const zip = new File([new Uint8Array([80, 75, 3, 4])], 'sources.zip', { type: 'application/zip' });
    await user.upload(screen.getByLabelText('Joindre un fichier', { selector: 'input' }), zip);
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error' }));
    expect(app.toasts.at(-1)?.text).toMatch(/sources\.zip.*images.*PDF.*texte/);
    expect(screen.queryByText('sources.zip')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeDisabled();
  });

  it('caps the files of one message at 18 Mo in all', async () => {
    setup();
    const pdf = (name: string) => {
      const f = new File(['%PDF-1.4'], name, { type: 'application/pdf' });
      Object.defineProperty(f, 'size', { value: 10 * 1024 * 1024 });
      return f;
    };
    const input = screen.getByLabelText('Joindre un fichier', { selector: 'input' });
    await userEvent.upload(input, pdf('a.pdf'));
    expect(await screen.findByText('a.pdf')).toBeInTheDocument();
    await userEvent.upload(input, pdf('b.pdf'));
    await waitFor(() => expect(app.toasts.at(-1)?.text).toMatch(/b\.pdf.*18 Mo en tout/));
    expect(screen.queryByText('b.pdf')).not.toBeInTheDocument();
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

  it('takes a dropped file instead of letting the window open it', async () => {
    setup();
    const composer = screen.getByRole('group');
    const drop = (f: File) => {
      const e = createEvent.drop(composer, { dataTransfer: { files: [f], types: ['Files'] } });
      fireEvent(composer, e);
      return e.defaultPrevented;
    };
    expect(drop(new File(['a,b'], 'export.csv', { type: '' }))).toBe(true);
    expect(await screen.findByText('export.csv')).toBeInTheDocument();
    expect(drop(new File(['MZ'], 'setup.exe', { type: 'application/x-msdownload' }))).toBe(true);
    await waitFor(() => expect(app.toasts.at(-1)?.text).toMatch(/setup\.exe/));
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

  it('disables the effort and the auto mode for Haiku', async () => {
    setup({ model: 'haiku', mode: 'acceptEdits' });
    expect(screen.getByRole('button', { name: /^Effort/ })).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: /^Modèle/ }));
    expect(screen.getByRole('menuitemradio', { name: /Opus/ })).toBeEnabled();
    await userEvent.click(screen.getByRole('button', { name: /^Mode/ }));
    expect(screen.getByRole('menuitemradio', { name: /Auto/ })).toBeDisabled();
    expect(screen.getByRole('menuitemradio', { name: /Plan/ })).toBeEnabled();
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
    expect(stop).toHaveTextContent('■');
    expect(stop).not.toHaveTextContent('Stop');
    // Claude takes a message sent during a turn at its next step: it is sent, not queued.
    expect(screen.getByRole('button', { name: 'Envoyer' })).toBeInTheDocument();
  });
});

describe('Composer in English', () => {
  beforeEach(() => resetApp());

  it('speaks English in the field, the bar and the menus, and keeps the shortcuts of the keyboard', async () => {
    setLang('en');
    const { textarea } = setup({ model: 'opus', effort: 'high', mode: 'plan' });
    expect(textarea).toHaveAttribute('placeholder', 'Describe the task to give to Claude…');
    expect(screen.getByRole('button', { name: 'Send' })).toHaveAttribute('title', 'Enter to send · Shift+Enter for a new line');
    expect(screen.getByRole('button', { name: 'Attach a file' })).toHaveAttribute(
      'title',
      'Attach an image, a PDF or a text file (or paste / drop it)',
    );
    // “Mode” alone: “Model” begins the same.
    await userEvent.click(screen.getByRole('button', { name: /^Mode\b/ }));
    expect(screen.getByText('Accept edits')).toBeInTheDocument();
    expect(screen.getByText('Claude analyzes and proposes a plan without changing anything')).toBeInTheDocument();
  });

  it('says in English that Haiku has no effort or Auto mode, and what to do while Claude works', async () => {
    setLang('en');
    setup({ model: 'haiku', status: 'running' });
    expect(screen.getByRole('button', { name: /^Effort/ })).toHaveAttribute('title', 'Haiku doesn’t support effort');
    expect(screen.getByRole('button', { name: 'Stop' })).toHaveAttribute('title', 'Interrupt (Esc)');
    expect(screen.getByRole('button', { name: 'Send' })).toHaveAttribute(
      'title',
      'Claude takes it into account at its next step · Enter to send',
    );
    await userEvent.click(screen.getByRole('button', { name: /^Mode\b/ }));
    expect(screen.getByText('unavailable with Haiku')).toBeInTheDocument();
  });

  it('refuses a file in English, naming the size it is over', async () => {
    setLang('en');
    setup();
    const user = userEvent.setup({ applyAccept: false });
    const input = screen.getByLabelText('Attach a file', { selector: 'input' });
    await user.upload(input, new File([new Uint8Array([80, 75, 3, 4])], 'sources.zip', { type: 'application/zip' }));
    await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ kind: 'error' }));
    expect(app.toasts.at(-1)?.text).toBe(
      '“sources.zip” can’t be attached. Supported files are images (PNG, JPEG, GIF, WebP), PDFs and text files.',
    );
    const big = new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' });
    Object.defineProperty(big, 'size', { value: 18 * 1024 * 1024 + 1 });
    await user.upload(input, big);
    await waitFor(() => expect(app.toasts.at(-1)?.text).toBe('rapport.pdf is over 18 MB'));
    const small = new File(['%PDF-1.4'], 'a.pdf', { type: 'application/pdf' });
    await user.upload(input, small);
    expect(await screen.findByRole('button', { name: 'Remove a.pdf' })).toBeInTheDocument();
  });
});
