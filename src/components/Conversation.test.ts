import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cleanup, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, onTestFinished, vi } from 'vitest';

/** The interfaces of Claude Code started (« Vue Claude Code »). */
const tui = vi.hoisted(() => ({ opened: 0 }));

// "Tester" starts test launches, which have logs (xterm.js): none in jsdom.
vi.mock('../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
  // « Vue Claude Code »: its interface, not drawn in jsdom.
  openAgentTerminal: async () => {
    tui.opened += 1;
    return { id: 't1', projectId: '', name: 'x', shell: 'claude' };
  },
  getXTerm: () => undefined,
  roomOf: () => null,
  mountTerminal() {},
  disposeTerminal() {},
  // The terminal of the header, in the agent’s worktree: only what is asked of the backend is looked at.
  openTerminal: vi.fn((projectId: string, shell: string, name: string) => Promise.resolve({ id: `t-${name}`, projectId, name, shell })),
}));

import { applyConvOps, conversationOf, releaseIdle } from '../lib/conversations.svelte';
import { setLang } from '../lib/i18n';
import { cancelHeld, heldOf } from '../lib/outbox.svelte';
import { app } from '../lib/state.svelte';
import type { Agent } from '../lib/types';
import { answerClock, settle } from '../test/conversation';
import {
  agent,
  BARE_ENGINE,
  CLAUDE_ENGINE,
  CURSOR_ENGINE,
  fakeBackend,
  gitInfo,
  project,
  pullRequest,
  resetApp,
  ticket,
} from '../test/ipc';
import { workflows } from '../lib/workflows.svelte';
import { GSD_ID, gsd, passage, scanOf, track } from '../test/workflows';
import Conversation from './Conversation.svelte';
import ContextMenu from './ContextMenu.svelte';

// jsdom has no layout: give scrollable elements a fixed geometry.
const geometry = { scrollHeight: 2000, clientHeight: 500 };
const saved: PropertyDescriptor[] = [];

beforeEach(() => {
  for (const [k, v] of Object.entries(geometry)) {
    saved.push(Object.getOwnPropertyDescriptor(HTMLElement.prototype, k)!);
    Object.defineProperty(HTMLElement.prototype, k, { configurable: true, get: () => v });
  }
});

afterEach(() => {
  for (const k of Object.keys(geometry)) {
    const d = saved.shift();
    if (d) Object.defineProperty(HTMLElement.prototype, k, d);
  }
});

function setup(over: Partial<Agent> = {}, items: unknown[] = []) {
  const a = agent({ id: `v${Math.random()}`, status: 'running', ...over });
  resetApp({ projects: [project()], agents: [a] });
  fakeBackend({ get_conversation: () => items });
  const r = render(Conversation, { agent: a, project: project() });
  return { a, ...r, scroller: r.container.querySelector('.scroll') as HTMLElement };
}

const frame = () => new Promise((r) => requestAnimationFrame(() => r(null)));

const user = (i: number) => ({ kind: 'user' as const, id: `u${i}`, text: `Message ${i}`, images: 0, ts: i, queued: false });
/** A conversation of `n` messages, `u0` to `u${n - 1}`. */
const many = (n: number) => Array.from({ length: n }, (_, i) => user(i));
/** The items the view draws a block for, in order. */
const drawnIds = (c: HTMLElement) => [...c.querySelectorAll<HTMLElement>('[data-item]')].map((e) => e.dataset.item);

const WT = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\w1', branch: 'escouade/w1', baseBranch: 'main' };

describe('Conversation', () => {
  it('opens scrolled to the latest message', async () => {
    const { scroller } = setup({}, [{ kind: 'user', id: 'u1', text: 'Salut', images: 0, ts: 1, queued: false }]);
    await frame();
    expect(scroller.scrollTop).toBe(2000);
  });

  it('says the worktree is being set up, and that messages wait for it', () => {
    setup({ status: 'idle', setup: '1/2 · npm ci' });
    const line = screen.getByRole('status');
    expect(line).toHaveTextContent('Préparation du worktree · 1/2 · npm ci — tes messages partiront une fois terminée.');
    expect(screen.queryByText('Claude travaille…')).toBeNull();
  });

  it('says before the first message that the worktree will be made then, under the agent’s name (ESC-156)', () => {
    setup({ status: 'idle', worktree: null, worktreePending: true });
    expect(screen.getByText('Le worktree sera créé au premier message, au nom de l’agent.')).toBeInTheDocument();
    // Not on the project's branch: on none yet.
    expect(document.querySelector('.crumbs')).toHaveTextContent('demo-api/—');
  });

  it('says the branch of the worktree the agent went into by itself, not the project’s folder’s (ESC-161)', () => {
    const a = agent({
      id: 'found1',
      status: 'idle',
      worktree: null,
      foundWorktree: { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a', branch: 'feat/a' },
      prWord: 'PR',
    });
    resetApp({ projects: [project()], agents: [a] });
    app.git[project().id] = gitInfo({ branch: 'dev' });
    const b = fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    expect(document.querySelector('.crumbs .branch')).toHaveTextContent('feat/a');
    expect(screen.getByRole('button', { name: 'Ouvrir une PR' })).toBeInTheDocument();
    expect(b.called('agent_prs_refresh').map((c) => c.args)).toEqual([{ id: 'found1', now: false }]);
  });

  it('shows the steps of the first message: the name, the worktree, then its setup (ESC-156)', async () => {
    const { a, rerender } = setup({ status: 'idle', worktree: null, worktreePending: true, setup: 'Nom de l’agent' });
    expect(screen.getByRole('status')).toHaveTextContent(
      'Préparation du worktree · Nom de l’agent — tes messages partiront une fois terminée.',
    );
    // Said once: the line of the steps says it now.
    expect(screen.queryByText('Le worktree sera créé au premier message, au nom de l’agent.')).toBeNull();
    await rerender({ agent: { ...a, setup: 'Création du worktree ajoute-un-bouton' }, project: project() });
    expect(screen.getByRole('status')).toHaveTextContent('Préparation du worktree · Création du worktree ajoute-un-bouton');
    await rerender({ agent: { ...a, worktreePending: false, worktree: WT, setup: '1/1 · npm ci' }, project: project() });
    expect(screen.getByRole('status')).toHaveTextContent('Préparation du worktree · 1/1 · npm ci');
  });

  describe('the output of the worktree’s setup', () => {
    it('unfolds on the live output of the step running, named with its rank', async () => {
      const { a, rerender } = setup({ status: 'idle', setup: '2/3 · npm ci' });
      app.setupOutput = { [a.id]: { step: 1, total: 2, lines: ['added 1 package', 'audited 2 packages'] } };
      const see = screen.getByRole('button', { name: 'Voir la sortie' });
      expect(see).toHaveAttribute('aria-expanded', 'false');
      expect(screen.queryByRole('log')).toBeNull();
      await userEvent.click(see);
      expect(see).toHaveAttribute('aria-expanded', 'true');
      const out = screen.getByRole('log', { name: 'Sortie de 2/3 · npm ci' });
      expect(see).toHaveAttribute('aria-controls', out.id);
      // Line by line, in a monospaced block.
      expect(out.tagName).toBe('PRE');
      expect(out.textContent).toBe('added 1 package\naudited 2 packages');
      // As it comes, and the next step's in place of it.
      app.setupOutput = { [a.id]: { step: 1, total: 3, lines: ['added 1 package', 'audited 2 packages', 'found 0 vulnerabilities'] } };
      await tick();
      expect(out.textContent).toBe('added 1 package\naudited 2 packages\nfound 0 vulnerabilities');
      app.setupOutput = { [a.id]: { step: 2, total: 0, lines: [] } };
      await rerender({ agent: { ...a, setup: '3/3 · npm run gen (web)' }, project: project() });
      const next = screen.getByRole('log', { name: 'Sortie de 3/3 · npm run gen (web)' });
      expect(next).toHaveTextContent('Pas encore de sortie.');
      // Folded again.
      await userEvent.click(see);
      expect(see).toHaveAttribute('aria-expanded', 'false');
      expect(screen.queryByRole('log')).toBeNull();
      // The line saying the setup is under way is the same.
      expect(screen.getByRole('status')).toHaveTextContent(
        'Préparation du worktree · 3/3 · npm run gen (web) — tes messages partiront une fois terminée.',
      );
    });

    it('follows what the step writes, unless the reader scrolled up in it', async () => {
      const { a } = setup({ status: 'idle', setup: '1/1 · npm ci' });
      app.setupOutput = { [a.id]: { step: 0, total: 1, lines: ['un'] } };
      await userEvent.click(screen.getByRole('button', { name: 'Voir la sortie' }));
      const out = screen.getByRole('log');
      await waitFor(() => expect(out.scrollTop).toBe(2000));
      // Scrolled up: left there.
      out.scrollTop = 100;
      out.dispatchEvent(new Event('scroll'));
      app.setupOutput = { [a.id]: { step: 0, total: 2, lines: ['un', 'deux'] } };
      await tick();
      expect(out.scrollTop).toBe(100);
      // Back at the bottom: followed again.
      out.scrollTop = 1500;
      out.dispatchEvent(new Event('scroll'));
      app.setupOutput = { [a.id]: { step: 0, total: 3, lines: ['un', 'deux', 'trois'] } };
      await waitFor(() => expect(out.scrollTop).toBe(2000));
    });
  });

  describe('a message typed during a turn', () => {
    function typeDuringTurn(over: Partial<Agent> = {}) {
      const a = agent({ id: `h${Math.random()}`, status: 'running', ...over });
      resetApp({ projects: [project()], agents: [a] });
      app.engines = [CLAUDE_ENGINE, CURSOR_ENGINE];
      app.settings.midTurnDelaySecs = 10;
      const backend = fakeBackend({ get_conversation: () => [] });
      render(Conversation, { agent: a, project: project() });
      return { a, backend, textarea: screen.getByRole('textbox') as HTMLTextAreaElement };
    }
    // What a test leaves held would go once its delay is over, in the next test.
    afterEach(() => {
      for (const id of Object.keys(app.agents)) for (const h of heldOf(id)) cancelHeld(h.id);
    });

    it('shows it held with its countdown, and gives it back to the field on « Modifier »', async () => {
      const { backend, textarea } = typeDuringTurn();
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      const bubble = (await screen.findByText('Ajoute des tests')).closest('.msg') as HTMLElement;
      expect(within(bubble).getByText(/^Envoi dans (10|9) s$/)).toBeInTheDocument();
      await userEvent.click(within(bubble).getByRole('button', { name: 'Modifier' }));
      await waitFor(() => expect(textarea).toHaveValue('Ajoute des tests'));
      expect(screen.queryByText(/^Envoi dans/)).not.toBeInTheDocument();
      expect(backend.called('send_message')).toEqual([]);
    });

    it('sends it at once on « Envoyer maintenant », and nothing on « Annuler »', async () => {
      const { a, backend, textarea } = typeDuringTurn();
      await userEvent.type(textarea, 'Premier{Enter}');
      await userEvent.click(await screen.findByRole('button', { name: 'Envoyer maintenant' }));
      await waitFor(() => expect(backend.called('send_message').map((c) => c.args.text)).toEqual(['Premier']));
      await userEvent.type(textarea, 'Second{Enter}');
      await userEvent.click(await screen.findByRole('button', { name: 'Annuler' }));
      expect(screen.queryByText('Second')).not.toBeInTheDocument();
      expect(heldOf(a.id)).toEqual([]);
      expect(backend.called('send_message')).toHaveLength(1);
    });

    it('keeps one to Cursor until the end of the turn, with no « Envoyer maintenant »', async () => {
      const { textarea } = typeDuringTurn({ engine: 'cursor' });
      await userEvent.type(textarea, 'Pour Cursor{Enter}');
      const bubble = (await screen.findByText('Pour Cursor')).closest('.msg') as HTMLElement;
      expect(within(bubble).getByText('Attend la fin du tour')).toBeInTheDocument();
      expect(within(bubble).queryByRole('button', { name: 'Envoyer maintenant' })).not.toBeInTheDocument();
    });
  });

  describe('a message sent while the worktree is being set up', () => {
    // The backend answers once the setup is over, and records the message then.
    function sendDuringSetup(setupLabel: string | null = '1/2 · npm ci') {
      const a = agent({ id: `s${Math.random()}`, status: 'idle', setup: setupLabel });
      resetApp({ projects: [project()], agents: [a] });
      let finish: (reply: unknown) => void = () => {};
      const reply = new Promise((resolve, reject) => {
        finish = (r) => (r instanceof Error ? reject(r) : resolve(r));
      });
      const backend = fakeBackend({ get_conversation: () => [], send_message: () => reply });
      const { rerender } = render(Conversation, { agent: a, project: project() });
      return { a, backend, finish, rerender, textarea: screen.getByRole('textbox') as HTMLTextAreaElement };
    }

    it('shows its bubble at once, saying it waits for the preparation', async () => {
      const { backend, textarea } = sendDuringSetup();
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      const bubble = (await screen.findByText('Ajoute des tests')).closest('.msg') as HTMLElement;
      expect(within(bubble).getByText('En attente de la préparation…')).toBeInTheDocument();
      expect(textarea).toHaveValue('');
    });

    it('waits the same way for the worktree made at the first message (ESC-156)', async () => {
      const a = agent({ id: `s${Math.random()}`, status: 'idle', setup: null, worktree: null, worktreePending: true });
      resetApp({ projects: [project()], agents: [a] });
      const backend = fakeBackend({ get_conversation: () => [], send_message: () => new Promise(() => {}) });
      render(Conversation, { agent: a, project: project() });
      await userEvent.type(screen.getByRole('textbox'), 'Ajoute un bouton{Enter}');
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      const bubble = (await screen.findByText('Ajoute un bouton')).closest('.msg') as HTMLElement;
      expect(within(bubble).getByText('En attente de la préparation…')).toBeInTheDocument();
    });

    it('does not say the agent is ready for a task while the message waits', async () => {
      const { a, textarea } = sendDuringSetup();
      await waitFor(() => expect(conversationOf(a.id).loaded).toBe(true));
      expect(screen.getByText('Agent prêt')).toBeInTheDocument();
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      await screen.findByText('En attente de la préparation…');
      expect(screen.queryByText('Agent prêt')).not.toBeInTheDocument();
    });

    it('gives its place to the recorded message once the preparation is over', async () => {
      const { a, finish, textarea } = sendDuringSetup();
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      await screen.findByText('En attente de la préparation…');
      // The backend records the message, then answers.
      applyConvOps(a.id, [{ op: 'append', item: { kind: 'user', id: 'u1', text: 'Ajoute des tests', images: 0, ts: 2, queued: false } }]);
      finish(null);
      await waitFor(() => expect(screen.queryByText('En attente de la préparation…')).not.toBeInTheDocument());
      expect(screen.getAllByText('Ajoute des tests')).toHaveLength(1);
    });

    it('is taken off, and given back to the field, when it is refused', async () => {
      const { finish, textarea } = sendDuringSetup();
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      await screen.findByText('En attente de la préparation…');
      finish(new Error('Claude Code est introuvable'));
      await waitFor(() => expect(screen.queryByText('En attente de la préparation…')).not.toBeInTheDocument());
      expect(textarea).toHaveValue('Ajoute des tests');
    });

    it('says why another message sent meanwhile does not go, and leaves it in the field', async () => {
      const { backend, textarea } = sendDuringSetup();
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      await screen.findByText('En attente de la préparation…');
      await userEvent.type(textarea, 'Et la doc{Enter}');
      expect(app.toasts.at(-1)).toMatchObject({ text: 'Un message attend déjà la fin de la préparation.', kind: 'info' });
      expect(backend.called('send_message')).toHaveLength(1);
      expect(textarea).toHaveValue('Et la doc');
      // Nothing typed: nothing to say.
      await userEvent.clear(textarea);
      app.toasts = [];
      await userEvent.type(textarea, '{Enter}');
      expect(app.toasts).toEqual([]);
    });

    it('is given back, refused, above the message typed since, their files together', async () => {
      const { finish, textarea } = sendDuringSetup();
      const attach = (f: File) => userEvent.upload(screen.getByLabelText('Joindre un fichier', { selector: 'input' }), f);
      await attach(new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' }));
      await screen.findByRole('button', { name: 'Retirer rapport.pdf' });
      await userEvent.type(textarea, 'Résume{Enter}');
      await screen.findByText('En attente de la préparation…');
      await attach(new File(['à relire'], 'notes.txt', { type: 'text/plain' }));
      await userEvent.type(textarea, 'Et la doc');
      finish(new Error('Claude Code est introuvable'));
      await waitFor(() => expect(textarea).toHaveValue('Résume\n\nEt la doc'));
      expect(screen.getByRole('button', { name: 'Retirer rapport.pdf' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Retirer notes.txt' })).toBeInTheDocument();
    });

    it('is given back, refused, to its own agent’s field, not to the agent shown meanwhile', async () => {
      const { a, finish, rerender, textarea } = sendDuringSetup();
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      await screen.findByText('En attente de la préparation…');
      const b = agent({ id: `s${Math.random()}`, name: 'landing', status: 'idle' });
      app.agents[b.id] = b;
      await rerender({ agent: b, project: project() });
      await userEvent.type(textarea, 'Pour landing');
      finish(new Error('Claude Code est introuvable'));
      await waitFor(() => expect(app.toasts.at(-1)).toMatchObject({ text: 'Error: Claude Code est introuvable', kind: 'error' }));
      expect(textarea).toHaveValue('Pour landing');
      await rerender({ agent: app.agents[a.id], project: project() });
      expect(textarea).toHaveValue('Ajoute des tests');
    });

    it('shows the files it carries', async () => {
      const { textarea } = sendDuringSetup();
      await userEvent.upload(
        screen.getByLabelText('Joindre un fichier', { selector: 'input' }),
        new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' }),
      );
      await screen.findByText('rapport.pdf');
      await userEvent.type(textarea, 'Résume{Enter}');
      const bubble = (await screen.findByText('En attente de la préparation…')).closest('.msg') as HTMLElement;
      expect(within(bubble).getByText(/rapport\.pdf/)).toBeInTheDocument();
    });

    it('shows no waiting bubble once the worktree is ready: the message goes out at once', async () => {
      const { backend, textarea } = sendDuringSetup(null);
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      expect(screen.queryByText('En attente de la préparation…')).not.toBeInTheDocument();
    });
  });

  it('does not yank the reader back to the bottom when the agent is updated', async () => {
    const { a, scroller, rerender } = setup();
    await frame();
    scroller.scrollTop = 100;
    scroller.dispatchEvent(new Event('scroll'));
    await rerender({ agent: { ...a, tokens: 999, contextTokens: 12 }, project: project() });
    await frame();
    expect(scroller.scrollTop).toBe(100);
  });

  describe('near the bottom', () => {
    // Resizes of the content (messages rendered as they scroll into view, streamed text).
    let resized: ((entries: unknown[]) => void)[] = [];
    const Real = globalThis.ResizeObserver;
    beforeEach(() => {
      resized = [];
      globalThis.ResizeObserver = class {
        constructor(cb: (entries: unknown[]) => void) {
          resized.push(cb);
        }
        observe() {}
        unobserve() {}
        disconnect() {}
      } as unknown as typeof ResizeObserver;
    });
    afterEach(() => {
      globalThis.ResizeObserver = Real;
    });
    // The reader scrolls with the wheel (or keys, or by dragging); the layout moves the view alone.
    const readerScrollsTo = (el: HTMLElement, top: number) => {
      el.dispatchEvent(new WheelEvent('wheel', { deltaY: top - el.scrollTop }));
      layoutMovesTo(el, top);
    };
    const layoutMovesTo = (el: HTMLElement, top: number) => {
      el.scrollTop = top;
      el.dispatchEvent(new Event('scroll'));
    };
    const grows = (scroller: HTMLElement) => {
      let height = 2000;
      Object.defineProperty(scroller, 'scrollHeight', { configurable: true, get: () => height });
      return (h: number) => (height = h);
    };
    /** Every block of the conversation is `height()` tall, one under the other; the view is 500 px high, at the top of the screen. */
    const layOut = (height: () => number) =>
      vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
        const rect = (top: number, h: number) => ({
          top,
          bottom: top + h,
          height: h,
          left: 0,
          right: 0,
          width: 0,
          x: 0,
          y: top,
          toJSON: () => ({}),
        });
        if (this.classList.contains('scroll')) return rect(0, 500);
        // What a block holds is where its block is.
        const block = this.closest('.msgs > *');
        const msgs = block?.parentElement;
        if (block && msgs) {
          const scroller = msgs.parentElement as HTMLElement;
          return rect([...msgs.children].indexOf(block) * height() - scroller.scrollTop, height());
        }
        return rect(0, 0);
      });
    const messages = Array.from({ length: 8 }, (_, i) => ({
      kind: 'user',
      id: `u${i}`,
      text: `Message ${i}`,
      images: 0,
      ts: i,
      queued: false,
    }));

    it('lets the reader scroll up a little without pulling them back down', async () => {
      const { scroller } = setup();
      await frame();
      // At the bottom (2000), the reader scrolls up by 10 px, still within the old 80 px magnet.
      readerScrollsTo(scroller, 1490);
      resized.forEach((cb) => cb([]));
      expect(scroller.scrollTop).toBe(1490);
    });

    it('leaves the bottom when the reader drags the scrollbar up', async () => {
      const { scroller } = setup();
      await frame();
      scroller.dispatchEvent(new Event('pointerdown', { bubbles: true }));
      layoutMovesTo(scroller, 1300);
      window.dispatchEvent(new Event('pointerup'));
      resized.forEach((cb) => cb([]));
      expect(scroller.scrollTop).toBe(1300);
    });

    const press = (el: HTMLElement, key: string, mods: KeyboardEventInit = {}) =>
      el.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...mods }));

    it.each([
      ['ArrowUp', {}],
      ['PageUp', {}],
      ['Home', {}],
      ['Home', { ctrlKey: true }],
      [' ', { shiftKey: true }],
      // A focus move up: the browser brings what gets the focus into view.
      ['Tab', { shiftKey: true }],
    ])('leaves the bottom when the reader presses « %s » %o', async (key, mods) => {
      const { scroller } = setup();
      await frame();
      press(scroller, key, mods);
      layoutMovesTo(scroller, 1300);
      resized.forEach((cb) => cb([]));
      expect(scroller.scrollTop).toBe(1300);
    });

    // The first keystroke of Ctrl+Entrée or Alt+1…9 answering a card (which then folds to its summary line), with the
    // focus in the conversation: a modifier pressed alone scrolls nothing.
    it.each([
      ['a tool row', (c: HTMLElement) => c.querySelector('[data-item="t1"] .row') as HTMLElement],
      ['the conversation', (c: HTMLElement) => c.querySelector('.scroll') as HTMLElement],
    ])('keeps following when the content shrinks just after modifiers pressed alone, the focus on %s', async (_, target) => {
      const bash = { kind: 'tool', id: 't1', name: 'Bash', input: { command: 'ls' }, status: 'ok', ts: 1 };
      const { container, scroller } = setup({}, [bash]);
      await screen.findByText('ls');
      await frame();
      const el = target(container);
      el.focus();
      const height = grows(scroller);
      for (const key of ['Control', 'Alt', 'Shift', 'Meta', 'AltGraph']) press(el, key);
      height(1800);
      layoutMovesTo(scroller, 1300);
      height(2400);
      resized.forEach((cb) => cb([]));
      expect(scroller.scrollTop).toBe(2400);
    });

    it.each(['ArrowDown', 'PageDown', 'End', ' '])(
      'keeps following when the content shrinks just after the reader pressed « %s », which scrolls down',
      async (key) => {
        const { scroller } = setup();
        await frame();
        const height = grows(scroller);
        press(scroller, key);
        height(1800);
        layoutMovesTo(scroller, 1300);
        height(2400);
        resized.forEach((cb) => cb([]));
        expect(scroller.scrollTop).toBe(2400);
      },
    );

    it('keeps following when the content shrinks under a view at the bottom', async () => {
      const { scroller } = setup();
      await frame();
      const height = grows(scroller);
      // The end of a turn replaces taller content (the running indicator): the browser pulls the
      // view up with it, to the new bottom. The reader did not scroll.
      height(1800);
      layoutMovesTo(scroller, 1300);
      height(2400);
      resized.forEach((cb) => cb([]));
      expect(scroller.scrollTop).toBe(2400);
    });

    it('keeps following when the message field shrinks back as the message is sent', async () => {
      const { scroller } = setup();
      await frame();
      const height = grows(scroller);
      // The taller view pulls it up, and the message is in before the scroll event is.
      height(2300);
      layoutMovesTo(scroller, 1200);
      resized.forEach((cb) => cb([]));
      expect(scroller.scrollTop).toBe(2300);
    });

    it('follows the conversation again once the reader is back at the bottom', async () => {
      const { scroller } = setup();
      await frame();
      readerScrollsTo(scroller, 1400);
      readerScrollsTo(scroller, 1495);
      resized.forEach((cb) => cb([]));
      expect(scroller.scrollTop).toBe(2000);
    });

    describe('when the agent is opened again', () => {
      // App re-creates the conversation for each agent ({#key}): the view is a new element each time.
      const reopen = async (a: Agent) => {
        resized = [];
        const r = render(Conversation, { agent: a, project: project() });
        await frame();
        return r.container.querySelector('.scroll') as HTMLElement;
      };

      it('puts the conversation back where the reader left it', async () => {
        const { a, scroller, unmount } = setup();
        await frame();
        readerScrollsTo(scroller, 700);
        unmount();
        expect((await reopen(a)).scrollTop).toBe(700);
      });

      it('keeps each agent’s own place', async () => {
        const first = setup();
        await frame();
        readerScrollsTo(first.scroller, 700);
        first.unmount();
        const b = agent({ id: `v${Math.random()}`, status: 'running' });
        app.agents[b.id] = b;
        const other = render(Conversation, { agent: b, project: project() });
        await frame();
        expect((other.container.querySelector('.scroll') as HTMLElement).scrollTop).toBe(2000);
        other.unmount();
        expect((await reopen(first.a)).scrollTop).toBe(700);
      });

      it('stays glued to the bottom when it was left there, and follows the new messages', async () => {
        const { a, scroller, unmount } = setup();
        await frame();
        expect(scroller.scrollTop).toBe(2000);
        unmount();
        const view = await reopen(a);
        expect(view.scrollTop).toBe(2000);
        const height = grows(view);
        height(2600);
        resized.forEach((cb) => cb([]));
        expect(view.scrollTop).toBe(2600);
      });

      it('does not take the reader back down when new messages come in', async () => {
        const { a, scroller, unmount } = setup();
        await frame();
        readerScrollsTo(scroller, 700);
        unmount();
        const view = await reopen(a);
        const height = grows(view);
        height(2600);
        resized.forEach((cb) => cb([]));
        expect(view.scrollTop).toBe(700);
      });

      it('does not announce new messages just for being reopened, only for those that come in after', async () => {
        const { a, scroller, unmount } = setup();
        await frame();
        readerScrollsTo(scroller, 700);
        unmount();
        const view = await reopen(a);
        // The observer's first report is the size the view starts from.
        resized.forEach((cb) => cb([]));
        expect(screen.queryByRole('button', { name: /Nouveaux messages/ })).not.toBeInTheDocument();
        // Once the view has settled back in place, what grows is new messages.
        const now = performance.now();
        vi.spyOn(performance, 'now').mockReturnValue(now + 5000);
        grows(view)(2600);
        resized.forEach((cb) => cb([]));
        expect(await screen.findByRole('button', { name: /Nouveaux messages/ })).toBeInTheDocument();
      });

      describe('with the messages laid out', () => {
        const items = messages;
        // What a message weighs once drawn, and what the browser takes it for until then (content-visibility).
        const REAL = 150;
        const ESTIMATE = 60;
        let height = REAL;
        let spy: ReturnType<typeof vi.spyOn>;
        beforeEach(() => {
          height = REAL;
          spy = layOut(() => height);
        });
        afterEach(() => spy.mockRestore());

        it('puts back the message the reader was on, not the same distance from the top', async () => {
          const { a, scroller, unmount } = setup({}, items);
          await frame();
          // Message 2 (300-450) is the first in view, 50 px above the top of the view.
          readerScrollsTo(scroller, 350);
          unmount();
          // Messages off screen are not drawn, so they weigh less: 350 px from the top is then another message.
          height = ESTIMATE;
          expect((await reopen(a)).scrollTop).toBe(170);
        });

        it('keeps it there while the messages around it are drawn at their real height', async () => {
          const { a, scroller, unmount } = setup({}, items);
          await frame();
          readerScrollsTo(scroller, 350);
          unmount();
          height = ESTIMATE;
          const view = await reopen(a);
          height = REAL;
          resized.forEach((cb) => cb([]));
          expect(view.scrollTop).toBe(350);
        });

        it('reads the conversation again once it left the memory, then puts the reader back on their message', async () => {
          const { a, scroller, unmount } = setup({}, items);
          await frame();
          readerScrollsTo(scroller, 350);
          unmount();
          // 10 minutes out of sight, the agent idle.
          releaseIdle(Date.now() + 10 * 60_000, (id) => id !== a.id);
          expect(conversationOf(a.id).items).toEqual([]);
          let read: (items: unknown[]) => void = () => {};
          const backend = fakeBackend({ get_conversation: () => new Promise((r) => (read = r)) });
          height = ESTIMATE;
          const view = await reopen(a);
          expect(backend.called('get_conversation')).toHaveLength(1);
          // Nothing to put back while it is read: the place stays the reader's, even if the empty view scrolls.
          await frame();
          view.dispatchEvent(new Event('scroll'));
          expect(conversationOf(a.id).place?.anchor?.item).toBe('u2');
          read(items);
          await frame();
          await frame();
          expect(view.scrollTop).toBe(170);
        });

        it('leaves the reader alone once they scroll, however the messages are drawn', async () => {
          const { a, scroller, unmount } = setup({}, items);
          await frame();
          readerScrollsTo(scroller, 350);
          unmount();
          height = ESTIMATE;
          const view = await reopen(a);
          readerScrollsTo(view, 200);
          height = REAL;
          resized.forEach((cb) => cb([]));
          expect(view.scrollTop).toBe(200);
        });
      });
    });

    describe('going to a message (a search result)', () => {
      let height = 150;
      let spy: ReturnType<typeof vi.spyOn>;
      beforeEach(() => {
        height = 150;
        spy = layOut(() => height);
      });
      afterEach(() => {
        spy.mockRestore();
        vi.useRealTimers();
      });
      /** Where the view is with the message `i` in its middle: 150 px tall, 175 px below the top of the 500 px view. */
      const middle = (i: number) => i * 150 - 175;
      const highlighted = (c: HTMLElement) => [...c.querySelectorAll<HTMLElement>('.found')].map((e) => e.dataset.item);
      const settled = async () => {
        await frame();
        await frame();
      };

      /** The agent opened from a result: the message is asked for before its view is made. */
      function openAt(id: string, items: unknown[], load: () => unknown = () => items) {
        const a = agent({ id: `j${Math.random()}`, status: 'done' });
        resetApp({ projects: [project()], agents: [a] });
        fakeBackend({ get_conversation: load });
        conversationOf(a.id).reveal(id);
        const r = render(Conversation, { agent: a, project: project() });
        return { a, ...r, scroller: r.container.querySelector('.scroll') as HTMLElement };
      }

      it('brings the message into the middle of the view and highlights it for 2 s', async () => {
        vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
        const { container, scroller } = openAt('u5', messages);
        await settled();
        expect(scroller.scrollTop).toBe(middle(5));
        expect(highlighted(container)).toEqual(['u5']);
        vi.advanceTimersByTime(1999);
        await tick();
        expect(highlighted(container)).toEqual(['u5']);
        vi.advanceTimersByTime(1);
        await tick();
        expect(highlighted(container)).toEqual([]);
      });

      it('wins over the place the reader had left, and is the place kept after', async () => {
        const { a, scroller, unmount } = setup({ status: 'done' }, messages);
        await frame();
        readerScrollsTo(scroller, 700);
        unmount();
        conversationOf(a.id).reveal('u3');
        const again = render(Conversation, { agent: a, project: project() });
        await settled();
        expect((again.container.querySelector('.scroll') as HTMLElement).scrollTop).toBe(middle(3));
        again.unmount();
        const later = render(Conversation, { agent: a, project: project() });
        await settled();
        expect((later.container.querySelector('.scroll') as HTMLElement).scrollTop).toBe(middle(3));
      });

      it('goes there once the conversation is loaded', async () => {
        let release: (items: unknown[]) => void = () => {};
        const { container, scroller } = openAt('u6', messages, () => new Promise((r) => (release = r)));
        await settled();
        expect(highlighted(container)).toEqual([]);
        release(messages);
        await settled();
        expect(scroller.scrollTop).toBe(middle(6));
        expect(highlighted(container)).toEqual(['u6']);
      });

      it('goes to a message of the agent already on screen', async () => {
        const { a, container, scroller } = setup({ status: 'done' }, messages);
        await frame();
        expect(scroller.scrollTop).toBe(2000);
        conversationOf(a.id).reveal('u4');
        await settled();
        expect(scroller.scrollTop).toBe(middle(4));
        expect(highlighted(container)).toEqual(['u4']);
        // Not taken back down by the messages drawn around it.
        resized.forEach((cb) => cb([]));
        expect(scroller.scrollTop).toBe(middle(4));
      });

      it('keeps the message in view while the messages around it are drawn at their real height', async () => {
        // Off screen, messages weigh a guess (content-visibility) until drawn.
        height = 60;
        const { scroller } = openAt('u5', messages);
        await settled();
        // Message 5 (300-360) in the middle: 220 px below the top of the view.
        expect(scroller.scrollTop).toBe(80);
        height = 150;
        resized.forEach((cb) => cb([]));
        expect(scroller.scrollTop).toBe(5 * 150 - 220);
      });

      it('shows a subagent’s message in the call that ran it', async () => {
        const items = [
          messages[0],
          { kind: 'tool', id: 't1', name: 'Agent', input: { description: 'Enquête' }, status: 'ok', ts: 1 },
          { kind: 'text', id: 's1:0', parent: 't1', text: 'Trouvé dans db.ts', streaming: false },
          messages[1],
        ];
        const { container } = openAt('s1:0', items);
        await settled();
        expect(highlighted(container)).toEqual(['t1']);
      });

      it('leaves the view where it would have been when the message is not there', async () => {
        const { container, scroller } = openAt('nope', messages);
        await settled();
        expect(scroller.scrollTop).toBe(2000);
        expect(highlighted(container)).toEqual([]);
      });

      it('goes to a message older than the window, drawing what leads to it', async () => {
        const { container, scroller } = openAt('u20', many(200));
        await settled();
        // From a few messages before it to the end: the button stays above for the older ones.
        expect(drawnIds(container)[0]).toBe('u10');
        expect(drawnIds(container)).toHaveLength(190);
        expect(screen.getByRole('button', { name: 'Afficher les 10 précédents' })).toBeInTheDocument();
        // The button, then u10…u19 above it.
        expect(scroller.scrollTop).toBe(middle(11));
        expect(highlighted(container)).toEqual(['u20']);
      });
    });

    describe('a long conversation', () => {
      /** The button once the view is in place (the frame after the load): as the reader first sees it. */
      const olderButton = async (name: string) => {
        const button = await screen.findByRole('button', { name });
        await frame();
        return button;
      };

      it('draws only its last 80 items, with a button for the 80 before', async () => {
        const { container } = setup({ status: 'done' }, many(2000));
        await screen.findByText('Message 1999');
        expect(drawnIds(container)).toHaveLength(80);
        expect(drawnIds(container)[0]).toBe('u1920');
        expect(screen.queryByText('Message 1919')).not.toBeInTheDocument();
        expect(screen.getByRole('button', { name: 'Afficher les 80 précédents' })).toBeInTheDocument();
      });

      it('draws no button when every item is drawn', async () => {
        const { container } = setup({ status: 'done' }, many(80));
        await screen.findByText('Message 79');
        expect(drawnIds(container)).toHaveLength(80);
        expect(screen.queryByRole('button', { name: /Afficher/ })).not.toBeInTheDocument();
      });

      it('draws the items before by slices of 80, down to the first', async () => {
        const { container } = setup({ status: 'done' }, many(201));
        await userEvent.click(await olderButton('Afficher les 80 précédents'));
        expect(drawnIds(container)).toHaveLength(160);
        expect(drawnIds(container)[0]).toBe('u41');
        await userEvent.click(screen.getByRole('button', { name: 'Afficher les 41 précédents' }));
        expect(drawnIds(container)).toHaveLength(201);
        expect(drawnIds(container)[0]).toBe('u0');
        expect(screen.queryByRole('button', { name: /Afficher/ })).not.toBeInTheDocument();
      });

      it('says « Afficher le précédent » for a single one', async () => {
        setup({ status: 'done' }, many(81));
        expect(await screen.findByRole('button', { name: 'Afficher le précédent' })).toBeInTheDocument();
      });

      it('moves the focus on to the conversation once the last slice is drawn, not back to the page', async () => {
        const { scroller } = setup({ status: 'done' }, many(120));
        const button = await olderButton('Afficher les 40 précédents');
        button.focus();
        await userEvent.keyboard('{Enter}');
        await waitFor(() => expect(scroller).toHaveFocus());
      });

      describe('told to a screen reader', () => {
        /** The live region's text, once it says something. */
        const said = (text: string) => {
          const line = screen.getByText(text);
          expect(line.closest('[aria-live="polite"]')).not.toBeNull();
          return line;
        };

        it('says how many messages a slice drew, and keeps the focus on the button for the next one', async () => {
          const { scroller } = setup({ status: 'done' }, many(200));
          const button = await olderButton('Afficher les 80 précédents');
          button.focus();
          await userEvent.keyboard('{Enter}');
          await waitFor(() => said('80 messages précédents affichés'));
          expect(screen.getByRole('button', { name: 'Afficher les 40 précédents' })).toHaveFocus();
          await userEvent.keyboard('{Enter}');
          await waitFor(() => said('40 messages précédents affichés'));
          expect(scroller).toHaveFocus();
        });

        it('says it of a single message too', async () => {
          setup({ status: 'done' }, many(81));
          await userEvent.click(await olderButton('Afficher le précédent'));
          await waitFor(() => said('1 message précédent affiché'));
        });

        it('says it again for a slice of the same size', async () => {
          setup({ status: 'done' }, many(400));
          await userEvent.click(await olderButton('Afficher les 80 précédents'));
          const first = await waitFor(() => said('80 messages précédents affichés'));
          await userEvent.click(screen.getByRole('button', { name: 'Afficher les 80 précédents' }));
          // A new node in the live region: the same text is read again.
          await waitFor(() => expect(said('80 messages précédents affichés')).not.toBe(first));
        });
      });

      describe('laid out', () => {
        let height = 150;
        let spy: ReturnType<typeof vi.spyOn>;
        beforeEach(() => {
          height = 150;
          spy = layOut(() => height);
        });
        afterEach(() => spy.mockRestore());

        it('keeps the reader on the message they were reading when a slice is drawn above it', async () => {
          const { container, scroller } = setup({ status: 'done' }, many(160));
          await frame();
          grows(scroller)(24_000);
          // The button (0-150), u80 (150-300), u81 (300-450): u81 is the first in view, 100 px above its top.
          readerScrollsTo(scroller, 400);
          await userEvent.click(screen.getByRole('button', { name: 'Afficher les 80 précédents' }));
          expect(drawnIds(container)[0]).toBe('u0');
          // u81 is now the 82nd block: still 100 px above the top of the view.
          expect(scroller.scrollTop).toBe(81 * 150 + 100);
          // Kept there while the blocks drawn above it are measured.
          height = 100;
          resized.forEach((cb) => cb([]));
          expect(scroller.scrollTop).toBe(81 * 100 + 100);
        });

        it('puts the reader back among the older items they had drawn', async () => {
          const { a, container, scroller, unmount } = setup({ status: 'done' }, many(200));
          await frame();
          grows(scroller)(30_000);
          await userEvent.click(screen.getByRole('button', { name: 'Afficher les 80 précédents' }));
          expect(drawnIds(container)[0]).toBe('u40');
          // The button, then u40 (150-300)… u59 (3000-3150): u59 at the top of the view.
          readerScrollsTo(scroller, 3000);
          unmount();
          resized = [];
          const again = render(Conversation, { agent: a, project: project() });
          await frame();
          expect(drawnIds(again.container)[0]).toBe('u40');
          expect((again.container.querySelector('.scroll') as HTMLElement).scrollTop).toBe(3000);
        });

        describe('the focus moving while the view follows the bottom', () => {
          const bash = { kind: 'tool', id: 't1', name: 'Bash', input: { command: 'ls' }, status: 'ok', ts: 1 };
          // 81 blocks of 150 px (the button, then 80 items) in the 500 px view, at its bottom.
          const BOTTOM = 81 * 150 - 500;
          async function atTheBottom(items: unknown[]) {
            const r = setup({ status: 'running' }, items);
            await frame();
            await frame();
            const height = grows(r.scroller);
            height(81 * 150);
            layoutMovesTo(r.scroller, BOTTOM);
            return { ...r, height };
          }

          it('leaves the bottom for an element above it, the focus coming from the message field', async () => {
            const { scroller } = await atTheBottom(many(200));
            // Shift+Tab in the message field, whose keys are not the conversation's, lands above.
            const field = screen.getByRole('textbox');
            field.focus();
            press(field, 'Tab', { shiftKey: true });
            screen.getByRole('button', { name: 'Afficher les 80 précédents' }).focus();
            // The browser brings it into view.
            layoutMovesTo(scroller, 0);
            resized.forEach((cb) => cb([]));
            expect(scroller.scrollTop).toBe(0);
          });

          it('keeps following when the focus goes to an element at the bottom', async () => {
            const { container, scroller, height } = await atTheBottom([...many(199), bash]);
            (container.querySelector('[data-item="t1"] .row') as HTMLElement).focus();
            // The content shrinks under the view, which goes on following.
            height(81 * 150 - 100);
            layoutMovesTo(scroller, BOTTOM - 100);
            resized.forEach((cb) => cb([]));
            expect(scroller.scrollTop).toBe(81 * 150 - 100);
          });
        });
      });

      it('follows the new messages at the bottom, drawing the latest 80', async () => {
        const { a, container, scroller } = setup({ status: 'running' }, many(200));
        await screen.findByText('Message 199');
        await frame();
        expect(scroller.scrollTop).toBe(2000);
        applyConvOps(a.id, [{ op: 'append', item: user(200) }]);
        await tick();
        grows(scroller)(2600);
        resized.forEach((cb) => cb([]));
        expect(scroller.scrollTop).toBe(2600);
        expect(drawnIds(container)).toHaveLength(80);
        expect(drawnIds(container)[0]).toBe('u121');
      });

      describe('while the latest 80 slide under a view at the bottom', () => {
        // The first block goes as a new one comes: the browser lowers the view (scroll anchoring, or clamping).
        async function slides(before: (scroller: HTMLElement) => void) {
          const { a, container, scroller } = setup({ status: 'running' }, many(200));
          await screen.findByText('Message 199');
          await frame();
          // The observer's first report is the size the view starts from.
          resized.forEach((cb) => cb([]));
          before(scroller);
          applyConvOps(a.id, [{ op: 'append', item: user(200) }]);
          await tick();
          layoutMovesTo(scroller, 1940);
          resized.forEach((cb) => cb([]));
          return { container, scroller };
        }
        const following = async ({ container, scroller }: { container: HTMLElement; scroller: HTMLElement }) => {
          expect(scroller.scrollTop).toBe(2000);
          expect(drawnIds(container)).toHaveLength(80);
          expect(drawnIds(container)[0]).toBe('u121');
          await tick();
          expect(screen.queryByRole('button', { name: /Nouveaux messages/ })).not.toBeInTheDocument();
        };

        it('keeps following just after the reader wheeled down', async () => {
          await following(await slides((s) => s.dispatchEvent(new WheelEvent('wheel', { deltaY: 120 }))));
        });

        it('keeps following just after the reader pressed a key that scrolls down', async () => {
          await following(await slides((s) => s.dispatchEvent(new KeyboardEvent('keydown', { key: 'PageDown', bubbles: true }))));
        });

        it('keeps following while the reader holds the pointer down (selecting text)', async () => {
          await following(await slides((s) => s.dispatchEvent(new Event('pointerdown', { bubbles: true }))));
        });
      });

      it('keeps following when the content shrinks under a view at the bottom, the reader wheeling down', async () => {
        const { scroller } = setup({ status: 'running' }, many(20));
        await screen.findByText('Message 19');
        await frame();
        const height = grows(scroller);
        scroller.dispatchEvent(new WheelEvent('wheel', { deltaY: 120 }));
        height(1800);
        layoutMovesTo(scroller, 1300);
        height(2400);
        resized.forEach((cb) => cb([]));
        expect(scroller.scrollTop).toBe(2400);
      });

      it('keeps what the reader scrolled up to when new messages come in', async () => {
        const { a, container, scroller } = setup({ status: 'running' }, many(200));
        await screen.findByText('Message 199');
        await frame();
        readerScrollsTo(scroller, 1000);
        applyConvOps(a.id, [{ op: 'append', item: user(200) }]);
        await tick();
        resized.forEach((cb) => cb([]));
        expect(scroller.scrollTop).toBe(1000);
        expect(drawnIds(container)[0]).toBe('u120');
        expect(drawnIds(container)).toHaveLength(81);
      });

      it('draws the latest 80 again once the reader goes down to the new messages', async () => {
        const { a, container } = setup({ status: 'running' }, many(200));
        await screen.findByText('Message 199');
        await frame();
        // The observer's first report is the size the view starts from.
        resized.forEach((cb) => cb([]));
        await userEvent.click(screen.getByRole('button', { name: 'Afficher les 80 précédents' }));
        expect(drawnIds(container)).toHaveLength(160);
        applyConvOps(a.id, [{ op: 'append', item: user(200) }]);
        // Out of the settling of the slice: what grows is new messages.
        const now = performance.now();
        vi.spyOn(performance, 'now').mockReturnValue(now + 5000);
        resized.forEach((cb) => cb([]));
        await userEvent.click(await screen.findByRole('button', { name: /Nouveaux messages/ }));
        expect(drawnIds(container)).toHaveLength(80);
        expect(drawnIds(container)[0]).toBe('u121');
      });
    });
  });

  it('names the agent’s model with the version Claude Code runs for it, in the composer’s chip (no more in the header)', () => {
    const { container } = setup({ model: 'sonnet' });
    app.models = [{ value: 'sonnet', resolvedModel: 'claude-sonnet-5-5' }];
    expect(container.querySelector('.head .model')).toBeNull();
    return waitFor(() => expect(screen.getByRole('button', { name: /^Modèle/ })).toHaveTextContent('Sonnet 5.5'));
  });

  it('says so when the conversation cannot be loaded', async () => {
    const a = agent({ id: `w${Math.random()}` });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({
      get_conversation: () => {
        throw new Error('journal illisible');
      },
    });
    render(Conversation, { agent: a, project: project() });
    await waitFor(() => expect(conversationOf(a.id).loaded).toBe(true));
    expect(await screen.findByText(/journal illisible/)).toBeInTheDocument();
    expect(screen.queryByText('Agent prêt')).not.toBeInTheDocument();
    expect(app.agents[a.id]).toBeDefined();
  });

  it('shows what Claude Code passed on by itself as such, older conversations included', async () => {
    const report = '<agent-message from="a42">\n[Subagent hand-back] … The report follows:\n  **Cause trouvée**\n</agent-message>';
    const { container } = setup({ status: 'done' }, [
      { kind: 'user', id: 'u1', text: 'Enquête sur le bug', images: 0, ts: 1, queued: false },
      {
        kind: 'tool',
        id: 't1',
        name: 'Agent',
        input: { description: 'Investigate PDF upload bug' },
        status: 'ok',
        ts: 1,
        result: { isError: false, text: 'Async agent launched successfully.\nagentId: a42' },
      },
      // Saved before events had their kind: a message "from claude.ai".
      {
        kind: 'user',
        id: 'u2',
        origin: 'remote',
        text: '<task-notification>\n<status>completed</status>\n<summary>Agent "Investigate PDF upload bug" finished</summary>\n</task-notification>',
        images: 0,
        ts: 2,
        queued: false,
      },
      { kind: 'event', id: 'e1', source: 'agent', from: 'a42', text: report, ts: 3 },
    ]);
    expect(await screen.findByText('Rapport du sous-agent « Investigate PDF upload bug »')).toBeInTheDocument();
    expect(screen.getByText('Tâche de fond terminée')).toBeInTheDocument();
    // Only the user's own message is in a bubble.
    expect([...container.querySelectorAll('.bubble')].map((b) => b.textContent?.trim())).toEqual(['Enquête sur le bug']);
  });

  it('shows the local commands a conversation saved before kept with their tags as typed, with their outputs (ESC-154)', async () => {
    const remote = (id: string, text: string) => ({ ...user(0), id, text, origin: 'remote' });
    const { container } = setup({ status: 'done' }, [
      remote('c0', '<local-command-caveat>Caveat: x</local-command-caveat>'),
      remote('c1', '<command-name>/context</command-name>\n<command-message>context</command-message>\n<command-args></command-args>'),
      remote('c2', '<local-command-stdout> \u001b[1mContext Usage\u001b[22m\n ⛁ ⛀   12k/200k</local-command-stdout>'),
      remote('c3', '<local-command-stderr>Unknown model</local-command-stderr>'),
      remote('c4', '<command-message>loop</command-message>\n<command-name>/loop</command-name>\n<command-args>check ESC-1</command-args>'),
      { kind: 'notice', id: 'n1', ts: 1, level: 'output', text: 'Total cost: $0.01\nTotal duration: 3s' },
      { kind: 'event', id: 'e1', ts: 2, source: 'wake', text: '/loop check ESC-2' },
    ]);
    await screen.findByText('/context');
    expect(container.textContent).not.toMatch(/<\/?(local-command|command-|bash-)/);
    expect(container.textContent).not.toContain('\u001b');
    expect([...container.querySelectorAll('.bubble .text')].map((b) => b.textContent)).toEqual(['/context']);
    const outputs = [...container.querySelectorAll('.notice.output')].map((n) => n.textContent);
    expect(outputs).toEqual([' Context Usage\n ⛁ ⛀   12k/200k', 'Total cost: $0.01\nTotal duration: 3s']);
    expect(container.querySelector('.notice.error')?.textContent).toBe('Unknown model');
    expect([...container.querySelectorAll('.event .detail')].map((d) => d.textContent)).toEqual(['/loop check ESC-1', '/loop check ESC-2']);
    expect(screen.getAllByText('Réveil')).toHaveLength(2);
    // The caveat shows nothing, not even an empty block.
    expect(container.querySelector('[data-item="c0"]')).toBeNull();
  });

  it('keeps telling when a stopped agent resumes, even with a notice after its turn', async () => {
    const end = { kind: 'turn', id: 'r1', ts: 1, durationMs: 1, cost: 0, tokens: 0, isError: true, interrupted: false, error: 'limit' };
    const notice = { kind: 'notice', id: 'n1', ts: 2, level: 'info', text: 'Contexte compacté' };
    setup({ status: 'error', resumeAt: Date.now() + 3_600_000 }, [end, notice]);
    expect(await screen.findByText(/^Reprise automatique/)).toBeInTheDocument();
  });

  it('ends a finished task with the files its last turn edited', async () => {
    const edit = (id: string, file: string, add: number) => ({
      kind: 'tool',
      id,
      name: 'Edit',
      input: { file_path: `C:\\code\\demo-api\\${file}` },
      status: 'ok',
      result: { isError: false, add, del: 1 },
      ts: 1,
    });
    const end = (id: string) => ({
      kind: 'turn',
      id,
      ts: 1,
      durationMs: 1,
      cost: 0,
      tokens: 0,
      isError: false,
      interrupted: false,
      error: null,
    });
    setup({ status: 'done' }, [edit('e1', 'old.ts', 1), end('r1'), edit('e2', 'src\\auth.ts', 5), end('r2')]);
    const recap = await screen.findByRole('list', { name: 'Fichiers modifiés' });
    expect(recap).toHaveTextContent('src/auth.ts+5−1');
    expect(recap).not.toHaveTextContent('old.ts');
  });

  it('shows an agent’s report of its criteria as a card, with its launch recipe', async () => {
    const text =
      'Fait.\n\n```escouade\n{"criteres": [{"n": 1, "ok": true, "note": "vérifié"}, {"n": 2, "ok": false, "note": "reste"}], "lancement": {"processus": [{"nom": "web", "commande": "npm run dev", "url": "http://localhost:4101"}]}}\n```';
    const { a } = setup({ status: 'done' }, [{ kind: 'text', id: 'm1', text, streaming: false }]);
    app.tickets.t1 = ticket({ agentId: a.id, column: 'doing' });
    const list = await screen.findByRole('list', { name: 'Bilan des critères' });
    const items = within(list).getAllByRole('listitem');
    expect(items[0]).toHaveTextContent('Le fichier existevérifié');
    expect(items[0].querySelector('.mark')).toHaveAccessibleName('Atteint');
    expect(items[1]).toHaveTextContent('Tests vertsreste');
    expect(items[1].querySelector('[data-icon="circle"]')).not.toBeNull();
    expect(screen.getByText('Lancement de test')).toBeInTheDocument();
    expect(screen.getByText('http://localhost:4101')).toBeInTheDocument();
    expect(screen.getByText('Fait.')).toBeInTheDocument();
    expect(screen.queryByText(/"criteres"/)).not.toBeInTheDocument();
    // No progress given: nothing about it.
    expect(screen.queryByRole('list', { name: 'Avancement' })).not.toBeInTheDocument();
    expect(screen.queryByText('Ce qui a été fait')).not.toBeInTheDocument();
  });

  it('lists what the agent has done after its criteria, and names a criterion by its number without a ticket', async () => {
    const text =
      '```escouade\n{"criteres": [{"n": 1, "ok": true}, {"n": 3, "ok": false, "note": "reste"}], "avancement": ["Tokens signés", "Middleware réécrit"]}\n```';
    setup({ status: 'done' }, [{ kind: 'text', id: 'm1', text, streaming: false }]);
    const criteria = await screen.findByRole('list', { name: 'Bilan des critères' });
    expect(
      within(criteria)
        .getAllByRole('listitem')
        .map((li) => li.textContent),
    ).toEqual(['Critère 1', 'Critère 3reste']);
    expect(screen.getByText('1/2')).toBeInTheDocument();
    expect(screen.getByText('Ce qui a été fait')).toBeInTheDocument();
    const done = screen.getByRole('list', { name: 'Avancement' });
    expect(
      within(done)
        .getAllByRole('listitem')
        .map((li) => li.textContent),
    ).toEqual(['Tokens signés', 'Middleware réécrit']);
    // After the criteria, in the card.
    expect(criteria.compareDocumentPosition(done) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('shows a report that is only progress, or only a recipe, without a list of criteria', async () => {
    const progress = '```escouade\n{"avancement": ["Adaptateur écrit"]}\n```';
    const recipe = '```escouade\n{"lancement": {"processus": [{"commande": "npm start"}]}}\n```';
    setup({ status: 'done' }, [
      { kind: 'text', id: 'm1', text: progress, streaming: false },
      { kind: 'text', id: 'm2', text: recipe, streaming: false },
    ]);
    expect(await screen.findByText('Adaptateur écrit')).toBeInTheDocument();
    expect(screen.getByText('npm start')).toBeInTheDocument();
    expect(screen.queryByRole('list', { name: 'Bilan des critères' })).not.toBeInTheDocument();
    expect(screen.queryByText('Bilan des critères')).not.toBeInTheDocument();
  });

  it('tests the agent from the launch recipe of its report', async () => {
    const text =
      '```escouade\n{"lancement": {"processus": [{"nom": "web", "commande": "npm run dev", "url": "http://localhost:4101"}]}}\n```';
    const { a } = setup({ status: 'done', worktree: WT, recipe: { prepare: [], processes: [], open: '' } }, [
      { kind: 'text', id: 'm1', text, streaming: false },
    ]);
    const list = await screen.findByRole('list', { name: 'Lancement de test' });
    await userEvent.click(within(list.closest('.report') as HTMLElement).getByRole('button', { name: 'Tester' }));
    expect(app.modal).toEqual({ kind: 'testLaunch', agentId: a.id });
  });

  it('offers no test from a report while the ticket is being validated, nor for an archived agent', async () => {
    const text = '```escouade\n{"lancement": {"processus": [{"nom": "web", "commande": "npm run dev"}]}}\n```';
    const recipe = { prepare: [], processes: [], open: '' };
    const { a, unmount } = setup({ status: 'done', worktree: WT, recipe }, [{ kind: 'text', id: 'm1', text, streaming: false }]);
    app.tickets.t1 = ticket({ agentId: a.id, column: 'review', step: 'Merge…' });
    const list = await screen.findByRole('list', { name: 'Lancement de test' });
    expect(within(list.closest('.report') as HTMLElement).queryByRole('button', { name: 'Tester' })).not.toBeInTheDocument();
    unmount();
    setup({ status: 'done', worktree: WT, recipe, archived: true }, [{ kind: 'text', id: 'm1', text, streaming: false }]);
    await screen.findByRole('list', { name: 'Lancement de test' });
    expect(screen.queryByRole('button', { name: 'Tester' })).not.toBeInTheDocument();
  });

  it('lists the preparation of a recipe that only prepares', async () => {
    const text =
      '```escouade\n{"lancement": {"preparation": [{"commande": "npm install", "dossier": "web"}, {"commande": "npm run build"}]}}\n```';
    setup({ status: 'done' }, [{ kind: 'text', id: 'm1', text, streaming: false }]);
    const list = await screen.findByRole('list', { name: 'Lancement de test' });
    expect(
      within(list)
        .getAllByRole('listitem')
        .map((li) => li.textContent),
    ).toEqual(['Préparationnpm install', 'Préparationnpm run build']);
  });

  it('leaves a block it cannot read as code, in its message', async () => {
    const text = 'Voici :\n\n```escouade\n{"criteres": oups}\n```';
    setup({ status: 'done' }, [{ kind: 'text', id: 'm1', text, streaming: false }]);
    expect(await screen.findByText(/oups/)).toBeInTheDocument();
    expect(screen.queryByRole('list', { name: 'Bilan des critères' })).not.toBeInTheDocument();
  });
});

describe('Conversation of an agent whose harness has no account (choice 19)', () => {
  const NO_CODEX = 'no-account:codex|Aucun compte Codex : ajoute-en un dans Réglages › Comptes.';
  const CODEX = { ...BARE_ENGINE, id: 'codex' as const, name: 'Codex', speaker: 'Codex' };

  it('says in its header that a Codex agent runs in a sandbox, and what it means in its mode (choice 8)', async () => {
    const SANDBOXED = { ...CODEX, caps: { ...CODEX.caps, sandbox: true } };
    const a = agent({ id: `sb${Math.random()}`, status: 'idle', engine: 'codex', mode: 'plan' });
    resetApp({ projects: [project()], agents: [a] });
    app.engines = [CLAUDE_ENGINE, SANDBOXED];
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    expect(screen.getByRole('img', { name: 'Bac à sable. Lecture seule, dans un bac à sable' })).toBeInTheDocument();
    cleanup();
    // Claude Code has none.
    const b = agent({ id: `sc${Math.random()}`, status: 'idle' });
    resetApp({ projects: [project()], agents: [b] });
    app.engines = [CLAUDE_ENGINE, SANDBOXED];
    render(Conversation, { agent: b, project: project() });
    expect(screen.queryByRole('img', { name: /^Bac à sable/ })).toBeNull();
  });

  it('says so in an amber card, keeps the message in the field, and adds an account from it', async () => {
    const a = agent({ id: `na${Math.random()}`, status: 'idle', engine: 'codex' });
    resetApp({ projects: [project()], agents: [a] });
    app.engines = [CLAUDE_ENGINE, CODEX];
    fakeBackend({
      get_conversation: () => [],
      send_message: () => {
        throw NO_CODEX;
      },
    });
    render(Conversation, { agent: a, project: project() });
    await userEvent.type(screen.getByRole('textbox'), 'Ajoute un bouton{Enter}');
    const card = await screen.findByRole('group', { name: 'Aucun compte Codex : ajoute-en un dans Réglages › Comptes.' });
    expect(card).toHaveClass('limit');
    expect(screen.getByRole('textbox')).toHaveValue('Ajoute un bouton');
    // Said once, in the card: no toast besides.
    expect(app.toasts).toHaveLength(0);
    await userEvent.click(within(card).getByRole('button', { name: 'Ajouter un compte Codex…' }));
    expect(app.modal).toEqual({ kind: 'account', engine: 'codex' });
  });

  it('goes once an account of its harness is there', async () => {
    const a = agent({ id: `nb${Math.random()}`, status: 'idle', engine: 'codex' });
    resetApp({ projects: [project()], agents: [a] });
    app.engines = [CLAUDE_ENGINE, CODEX];
    fakeBackend({
      get_conversation: () => [],
      send_message: () => {
        throw NO_CODEX;
      },
    });
    render(Conversation, { agent: a, project: project() });
    await userEvent.type(screen.getByRole('textbox'), 'Ajoute un bouton{Enter}');
    await screen.findByRole('group', { name: /Aucun compte Codex/ });
    app.settings.accounts = [...(app.settings.accounts ?? []), { ...app.settings.accounts![0], id: 'cx', engine: 'codex', active: true }];
    await waitFor(() => expect(screen.queryByRole('group', { name: /Aucun compte Codex/ })).not.toBeInTheDocument());
  });
});

describe('Conversation header', () => {
  it('switches between the classic and the split layout', async () => {
    setup();
    const split = screen.getByRole('button', { name: 'Conversation et fichiers côte à côte' });
    const classic = screen.getByRole('button', { name: 'Disposition classique' });
    expect(classic).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(split);
    expect(app.split).toBe(true);
    expect(split).toHaveAttribute('aria-pressed', 'true');
    await userEvent.click(classic);
    expect(app.split).toBe(false);
  });

  it('keeps the files counter but no panel toggle in the split layout, where the files are always shown', async () => {
    setup();
    await userEvent.click(screen.getByRole('button', { name: /Fichiers/ }));
    expect(app.filesOpen).toBe(true);
    app.toggleLayout();
    await frame();
    expect(screen.queryByRole('button', { name: /Fichiers/ })).not.toBeInTheDocument();
    expect(screen.getByText('Fichiers')).toBeInTheDocument();
  });
});

describe('Conversation header test launch', () => {
  const recipe = { prepare: [], processes: [], open: '' };

  it('names its buttons without their text, which a narrow header hides', async () => {
    const { a, rerender } = setup({ worktree: WT });
    const prepare = screen.getByRole('button', { name: 'Préparer le lancement' });
    expect(prepare.querySelector('.lbl')).toHaveTextContent('Préparer le lancement');
    await rerender({ agent: { ...a, recipe }, project: project() });
    expect(screen.getByRole('button', { name: 'Tester' }).querySelector('.lbl')).toHaveTextContent('Tester');
    expect(screen.queryByRole('button', { name: 'Préparer le lancement' })).not.toBeInTheDocument();
  });

  it('does not ask a ticket agent under way for its recipe, which its ticket already asks for', async () => {
    const { a } = setup({ worktree: WT });
    expect(screen.getByRole('button', { name: 'Préparer le lancement' })).toBeInTheDocument();
    app.tickets.t1 = ticket({ agentId: a.id, column: 'doing' });
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Préparer le lancement' })).not.toBeInTheDocument());
    app.tickets.t1 = ticket({ agentId: a.id, column: 'review' });
    expect(await screen.findByRole('button', { name: 'Préparer le lancement' })).toBeInTheDocument();
  });

  it('offers no test while the agent’s ticket is being validated', async () => {
    const { a } = setup({ worktree: WT, recipe });
    expect(screen.getByRole('button', { name: 'Tester' })).toBeInTheDocument();
    app.tickets.t1 = ticket({ agentId: a.id, column: 'review', step: 'Tests…' });
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Tester' })).not.toBeInTheDocument());
  });

  it('tests a worktree whose services isola runs without a recipe, and asks for none', () => {
    setup({ worktree: WT, isola: true });
    expect(screen.getByRole('button', { name: 'Tester' })).toHaveAttribute(
      'data-tip',
      'Lance les services isola de ce worktree et ouvre la fonctionnalité dans le navigateur',
    );
    expect(screen.queryByRole('button', { name: 'Préparer le lancement' })).not.toBeInTheDocument();
  });

  it('offers neither to an agent without a worktree, nor to an archived one', () => {
    const { unmount } = setup({ recipe });
    expect(screen.queryByRole('button', { name: 'Tester' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Préparer le lancement' })).not.toBeInTheDocument();
    unmount();
    setup({ worktree: WT, archived: true });
    expect(screen.queryByRole('button', { name: 'Préparer le lancement' })).not.toBeInTheDocument();
  });
});

describe('Conversation header usage', () => {
  it('shows how full the context is, out of the model’s window', () => {
    setup({ contextTokens: 45_200, contextWindow: 200_000 });
    const ctx = screen.getByText('Contexte').closest('.fig')!;
    expect(ctx.querySelector('dd')).toHaveTextContent('45,2 k / 200 k');
    expect(ctx).toHaveAttribute('data-tip', expect.stringContaining('23 %'));
    expect(ctx.querySelector('dd')).not.toHaveClass('full');
    // A thin gauge under the figure, filled to the share used.
    expect(ctx.querySelector<HTMLElement>('.ctx-meter i')!.style.width).toBe('23%');
  });

  it('warns when the context is nearly full', () => {
    setup({ contextTokens: 900_000, contextWindow: 1_000_000 });
    const dd = screen.getByText('Contexte').closest('.fig')!.querySelector('dd');
    expect(dd).toHaveTextContent('900,0 k / 1 M');
    expect(dd).toHaveClass('full');
  });

  it('shows the context alone while the window is unknown', () => {
    setup({ contextTokens: 12_300, contextWindow: 0 });
    const ctx = screen.getByText('Contexte').closest('.fig')!;
    expect(ctx.querySelector('dd')).toHaveTextContent(/^12,3 k$/);
    expect(ctx.querySelector('.ctx-meter')).toBeNull();
  });

  it('counts the running turn in the tokens and the estimated cost', () => {
    setup({ tokens: 2000, cost: 0.4, liveTokens: 1000, liveCost: 0.2 });
    expect(screen.getByText('3,0 k')).toBeInTheDocument();
    expect(screen.getByText('≈ 0,60 $')).toBeInTheDocument();
  });
});

describe('Conversation editor entry', () => {
  it('has no « Éditeur » of its own: the title bar has it (2.0 design, choice 5)', () => {
    setup({ worktree: WT });
    expect(screen.queryByRole('button', { name: /Éditeur/ })).not.toBeInTheDocument();
  });

  it('asks a worktree agent for its launch recipe from its header, then tests it', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\w1', branch: 'escouade/w1', baseBranch: 'main' };
    const a = agent({ id: 'w1', worktree: wt });
    resetApp({ projects: [project()], agents: [a] });
    const backend = fakeBackend({ get_conversation: () => [] });
    const { rerender } = render(Conversation, { agent: a, project: project() });
    await userEvent.click(screen.getByRole('button', { name: 'Préparer le lancement' }));
    expect(backend.called('agent_prepare_launch')).toEqual([{ cmd: 'agent_prepare_launch', args: { id: 'w1' } }]);
    // A recipe without processes: the modal opens, nothing starts.
    await rerender({ agent: { ...a, recipe: { prepare: [], processes: [], open: '' } }, project: project() });
    await userEvent.click(screen.getByRole('button', { name: 'Tester' }));
    expect(app.modal).toEqual({ kind: 'testLaunch', agentId: 'w1' });
  });

  it('opens a file edited in the conversation at its first changed line', async () => {
    // Its own id: conversations are kept by agent.
    resetApp({ agents: [agent({ id: 'link1' })] });
    const edit = {
      kind: 'tool',
      id: 'e1',
      name: 'Edit',
      input: { file_path: 'C:\\code\\demo-api\\src\\auth.ts' },
      status: 'ok',
      result: { isError: false, add: 1, del: 0, patch: [{ oldStart: 4, newStart: 7, lines: ['+x'] }] },
      ts: 1,
    };
    fakeBackend({
      get_conversation: () => [edit],
      fs_tree: () => ({ root: 'C:/code/demo-api', files: [], truncated: false }),
    });
    render(Conversation, { agent: app.agents.link1, project: project() });
    await userEvent.click(await screen.findByRole('button', { name: 'Ouvrir src/auth.ts dans l’éditeur' }));
    await waitFor(() => expect(app.editor.p1?.places.project?.active).toBe('src/auth.ts'));
    expect(app.editor.p1).toMatchObject({ on: true, source: 'project', reveal: { path: 'src/auth.ts', line: 7 } });
  });
});

describe('Conversation header « … » (2.0 design, choice 6)', () => {
  it('asks the sidebar for the menu of the agent, under the button, the one of its card', async () => {
    const { a } = setup({ worktree: WT });
    const more = screen.getByRole('button', { name: 'Actions de l’agent' });
    expect(more).toHaveAttribute('aria-haspopup', 'menu');
    expect(more.querySelector('[data-icon="more"]')).not.toBeNull();
    await userEvent.click(more);
    expect(app.agentMenuAsk).toMatchObject({ id: a.id, seq: 1 });
    await userEvent.click(more);
    expect(app.agentMenuAsk).toMatchObject({ id: a.id, seq: 2 });
  });

  it('tells the menu what the header, too narrow, no longer shows: « Fichiers » and the layouts (ESC-166)', async () => {
    setup({ worktree: WT });
    const more = screen.getByRole('button', { name: 'Actions de l’agent' });
    await userEvent.click(more);
    expect(app.agentMenuAsk?.folded).toEqual({ files: false, layout: false });
    // Folded away by its container queries, which jsdom neither applies nor tells (no checkVisibility).
    Element.prototype.checkVisibility = function (this: Element) {
      return !this.matches('.fig.files, .layout-seg');
    };
    onTestFinished(() => {
      delete (Element.prototype as Partial<Element>).checkVisibility;
    });
    await userEvent.click(more);
    expect(app.agentMenuAsk?.folded).toEqual({ files: true, layout: true });
  });

  it('has no « Terminal » of its own: the menu opens one in the agent’s worktree', () => {
    setup({ worktree: WT });
    expect(screen.queryByRole('button', { name: 'Terminal' })).not.toBeInTheDocument();
  });

  it('is said in English', () => {
    setLang('en');
    setup();
    expect(screen.getByRole('button', { name: 'Agent actions' })).toBeInTheDocument();
  });
});

describe('Conversation answers from the keyboard', () => {
  const perm = (id: string) => ({
    kind: 'permission',
    id,
    toolUseId: `t-${id}`,
    toolName: 'Bash',
    input: { command: `rm -rf ${id}` },
    canAlways: true,
    defaultNo: false,
    decision: null,
    ts: 1,
  });
  const ask = (id: string) => ({
    kind: 'question',
    id,
    toolUseId: `t-${id}`,
    ts: 1,
    answers: null,
    questions: [
      { question: 'Base ?', multiSelect: false, options: [{ label: 'PG' }, { label: 'SQLite' }] },
      { question: 'Cible ?', multiSelect: false, options: [{ label: 'Web' }, { label: 'Desktop' }] },
    ],
  });
  // An agent waiting on the requests of `items`, its conversation on screen.
  function waiting(items: unknown[], over: Partial<Agent> = {}) {
    const a = agent({ id: `k${Math.random()}`, status: 'waiting', pending: items.map((i) => (i as { id: string }).id), ...over });
    resetApp({ projects: [project()], agents: [a] });
    const backend = fakeBackend({ get_conversation: () => items });
    const { rerender } = render(Conversation, { agent: a, project: project() });
    return { a, backend, rerender };
  }
  const field = () => screen.getByRole('textbox') as HTMLTextAreaElement;
  answerClock();
  /** The cards of the requests, drawn a moment ago: the keys answer them. */
  async function shown(testId: string) {
    const cards = await screen.findAllByTestId(testId);
    settle();
    return cards;
  }

  it('answers nothing, and sends nothing, when a Ctrl+Enter meant to send the message comes just as a request does', async () => {
    const { backend } = waiting([perm('r1')]);
    await screen.findByTestId('permission-pending');
    await userEvent.type(field(), 'Lance les tests');
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    await tick();
    expect(backend.called('answer_permission')).toHaveLength(0);
    expect(backend.called('send_message')).toHaveLength(0);
    expect(field()).toHaveValue('Lance les tests');
    settle();
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
    expect(backend.called('answer_permission')[0].args).toMatchObject({ requestId: 'r1', decision: 'allow' });
  });

  it('waits again on the next request once Ctrl+Enter answered the first: a second press does not allow it unread', async () => {
    const { a, backend, rerender } = waiting([perm('r1'), perm('r2')]);
    await shown('permission-pending');
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
    // The backend's answer: the second request is the first to wait now.
    await rerender({ agent: { ...a, pending: ['r2'] }, project: project() });
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    await tick();
    expect(backend.called('answer_permission')).toHaveLength(1);
    settle();
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(2));
    expect(backend.called('answer_permission')[1].args).toMatchObject({ requestId: 'r2', decision: 'allow' });
  });

  it('allows a request with Ctrl+Enter typed in the message field, and keeps what was typed there', async () => {
    const { a, backend } = waiting([perm('r1')]);
    await shown('permission-pending');
    await userEvent.type(field(), 'Plutôt npm run clean');
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
    expect(backend.called('answer_permission')[0].args).toEqual({ id: a.id, requestId: 'r1', decision: 'allow', message: null });
    expect(backend.called('send_message')).toHaveLength(0);
    expect(field()).toHaveValue('Plutôt npm run clean');
  });

  it('neither refuses nor answers with the text typed when Enter comes just as a request does: the text stays in the field', async () => {
    const { backend } = waiting([perm('r1')]);
    await screen.findByTestId('permission-pending');
    await userEvent.type(field(), 'Lance les tests{Enter}');
    await tick();
    expect(backend.called('answer_permission')).toHaveLength(0);
    expect(backend.called('send_message')).toHaveLength(0);
    expect(field()).toHaveValue('Lance les tests');
    // A new line still goes in.
    await userEvent.keyboard('{Shift>}{Enter}{/Shift}');
    expect(field()).toHaveValue('Lance les tests\n');
    settle();
    await userEvent.keyboard('{Backspace}{Enter}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
    expect(backend.called('answer_permission')[0].args).toMatchObject({ requestId: 'r1', decision: 'deny', message: 'Lance les tests' });
  });

  it('still refuses with the typed explanation on Enter', async () => {
    const { a, backend } = waiting([perm('r1')]);
    await shown('permission-pending');
    await userEvent.type(field(), 'Plutôt npm run clean{Enter}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
    expect(backend.called('answer_permission')[0].args).toEqual({
      id: a.id,
      requestId: 'r1',
      decision: 'deny',
      message: 'Plutôt npm run clean',
    });
  });

  it('answers only the first of several requests, and shows the keys on it alone', async () => {
    const { backend } = waiting([perm('r1'), perm('r2')]);
    const cards = await shown('permission-pending');
    expect(cards).toHaveLength(2);
    expect(within(cards[0]).getByRole('button', { name: 'Autoriser' })).toHaveAttribute('aria-keyshortcuts', 'Control+Enter');
    expect(within(cards[1]).getByRole('button', { name: 'Autoriser' })).not.toHaveAttribute('aria-keyshortcuts');
    await userEvent.type(field(), '{Control>}{Enter}{/Control}');
    await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
    expect(backend.called('answer_permission')[0].args.requestId).toBe('r1');
  });

  it('picks the options of a question with Alt+digit from the message field, then validates with Ctrl+Enter', async () => {
    const { a, backend } = waiting([ask('q1')]);
    await shown('question-pending');
    await userEvent.type(field(), 'un mot');
    await userEvent.keyboard('{Alt>}2{/Alt}'); // Base: SQLite
    await userEvent.keyboard('{Alt>}1{/Alt}'); // Cible: Web
    expect(field()).toHaveValue('un mot');
    await userEvent.keyboard('{Control>}{Enter}{/Control}');
    await waitFor(() => expect(backend.called('answer_question')).toHaveLength(1));
    expect(backend.called('answer_question')[0].args).toEqual({
      id: a.id,
      requestId: 'q1',
      answers: { 'Base ?': 'SQLite', 'Cible ?': 'Web' },
    });
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('lets Ctrl+Enter send the typed answer to a question the options have not answered', async () => {
    const { backend } = waiting([ask('q1')]);
    await shown('question-pending');
    await userEvent.type(field(), 'Les deux{Control>}{Enter}{/Control}');
    await waitFor(() => expect(backend.called('answer_question')).toHaveLength(1));
    expect(backend.called('answer_question')[0].args.answers).toEqual({ 'Base ?': 'Les deux', 'Cible ?': 'Les deux' });
  });

  it('keeps Ctrl+Enter for sending when nothing waits', async () => {
    const { a, backend } = waiting([], { status: 'idle' });
    await userEvent.type(field(), 'Lance les tests{Control>}{Enter}{/Control}');
    await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
    expect(backend.called('send_message')[0].args).toMatchObject({ id: a.id, text: 'Lance les tests' });
  });

  // Ctrl+J gives the focus to its card in the list (ESC-75); Enter there opens it, as a click does.
  describe('an agent waiting, opened from its card', () => {
    it('puts the focus on the message field of an agent with a request waiting, where typing then Enter refuses', async () => {
      const { a, backend } = waiting([perm('r1')]);
      await shown('permission-pending');
      field().blur();
      app.selectAgent(app.agent!.id);
      await waitFor(() => expect(field()).toHaveFocus());
      await userEvent.keyboard('Plutôt npm run clean{Enter}');
      await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
      expect(backend.called('answer_permission')[0].args).toEqual({
        id: a.id,
        requestId: 'r1',
        decision: 'deny',
        message: 'Plutôt npm run clean',
      });
    });

    it('lets the keys that answer work from there', async () => {
      const { backend } = waiting([perm('r1')]);
      await shown('permission-pending');
      field().blur();
      app.selectAgent(app.agent!.id);
      await waitFor(() => expect(field()).toHaveFocus());
      await userEvent.keyboard('{Control>}{Enter}{/Control}');
      await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
      expect(backend.called('answer_permission')[0].args).toMatchObject({ requestId: 'r1', decision: 'allow' });
    });

    it('puts the focus on the message field when no request waits', async () => {
      waiting([]);
      field().blur();
      app.selectAgent(app.agent!.id);
      await waitFor(() => expect(field()).toHaveFocus());
    });
  });
});

describe('Conversation plan banner', () => {
  const plan = (over: Record<string, unknown> = {}) => ({
    source: 'tools' as const,
    tasks: [
      { id: '1', title: 'Auditer le hero', status: 'done' as const },
      { id: '2', title: 'Écrire les tests', status: 'inProgress' as const },
      { id: '3', title: 'Relire', status: 'pending' as const },
    ],
    agents: [],
    launched: 0,
    workflows: [],
    bgTasks: [],
    ...over,
  });

  it('sits between the header and the messages when the agent has a plan', () => {
    const { container, scroller } = setup({ plan: plan({ title: 'Refonte du hero' }) });
    const banner = container.querySelector('.plan')!;
    expect(banner).not.toBeNull();
    const main = container.querySelector('main')!;
    const parts = [...main.children];
    expect(parts.indexOf(container.querySelector('header')!)).toBeLessThan(parts.indexOf(banner));
    expect(parts.indexOf(banner)).toBeLessThan(parts.indexOf(scroller));
    // The banner is the header's next sibling: nothing comes between.
    expect(container.querySelector('header')!.nextElementSibling).toBe(banner);
    expect(banner.nextElementSibling).toBe(scroller);
    expect(screen.getByRole('button', { name: 'Plan : Refonte du hero' })).toBeInTheDocument();
    expect(within(screen.getByRole('button', { name: 'Plan : Refonte du hero' })).getByText('1/3 tâches')).toBeInTheDocument();
  });

  it('is not there without a plan, nor for a plan with no task and no subagent running', () => {
    const none = setup({});
    expect(none.container.querySelector('.plan')).toBeNull();
    none.unmount();
    const empty = setup({ plan: plan({ tasks: [], source: 'plan', planFile: 'docs/p.md' }) });
    expect(empty.container.querySelector('.plan')).toBeNull();
  });

  it('appears and goes with the plan of the agent', async () => {
    const { a, rerender, container } = setup({});
    expect(container.querySelector('.plan')).toBeNull();
    await rerender({ agent: { ...a, plan: plan() }, project: project() });
    expect(container.querySelector('.plan')).not.toBeNull();
    await rerender({ agent: { ...a, plan: undefined }, project: project() });
    expect(container.querySelector('.plan')).toBeNull();
  });

  it('shows the agent’s ticket as the title of a plan that has none', () => {
    const a = agent({ id: 'v-ticket', status: 'running', plan: plan() });
    resetApp({
      projects: [project()],
      agents: [a],
      tickets: [ticket({ id: 't1', agentId: a.id, column: 'doing', title: 'Ajouter le fichier' })],
    });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    expect(screen.getByRole('button', { name: 'Plan : Ajouter le fichier' })).toBeInTheDocument();
  });

  it('leaves the messages 160 px at least, whatever the plan takes', () => {
    const source = readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), 'Conversation.svelte'), 'utf8');
    expect(source).toMatch(/\.scroll\s*\{[^}]*flex:\s*1 0 160px;[^}]*min-height:\s*160px/);
  });

  it('keeps its place above the messages while the conversation scrolls', async () => {
    const { container, scroller } = setup({ plan: plan() }, many(30));
    await frame();
    expect(scroller.scrollTop).toBe(2000);
    expect(container.querySelector('.plan')!.nextElementSibling).toBe(scroller);
    expect(scroller.contains(container.querySelector('.plan'))).toBe(false);
  });

  it('says in English what it holds', () => {
    setLang('en');
    setup({ plan: plan({ title: 'Hero redesign' }) });
    const head = screen.getByRole('button', { name: 'Plan: Hero redesign' });
    expect(within(head).getByText('1/3 tasks')).toBeInTheDocument();
  });
});

describe('Conversation in English', () => {
  it('words the header, with the figures written as in English', async () => {
    setLang('en');
    setup({ status: 'running', contextTokens: 120_000, contextWindow: 200_000, activeMs: 151_000 });
    expect(screen.getByText('Running')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Agent actions' })).toHaveAttribute('aria-haspopup', 'menu');
    expect(screen.getByText('Context').nextElementSibling).toHaveTextContent('120.0k / 200k');
    expect(screen.getByText('Cost')).toBeInTheDocument();
    expect(screen.getByText('Duration').nextElementSibling).toHaveTextContent('2m 31s');
    expect(screen.getByRole('button', { name: 'Files 0' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: /^Layout \(Ctrl\+Shift\+L\)$/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Classic layout' })).toBeInTheDocument();
    expect(screen.getByText('Claude is working…')).toBeInTheDocument();
  });

  it('says in English that an agent is ready, archived or stopped', async () => {
    setLang('en');
    const ready = setup({ status: 'idle' });
    expect(await screen.findByText('Agent ready')).toBeInTheDocument();
    expect(screen.getByText(/^Describe the task to give to Claude\. The agent works in/)).toBeInTheDocument();
    ready.unmount();
    const stopped = setup({ status: 'error' });
    expect(screen.getByText('The agent stopped. Send a message to restart Claude on the same session.')).toBeInTheDocument();
    stopped.unmount();
    setup({ archived: true });
    expect(screen.getByText('Agent archived.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Restore' })).toBeInTheDocument();
  });

  it('says in English that the worktree is being set up', async () => {
    setLang('en');
    setup({ status: 'idle', setup: '1/2 · npm ci' });
    expect(screen.getByRole('status')).toHaveTextContent(
      'Setting up the worktree · 1/2 · npm ci — your messages will be sent once it’s done.',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Show output' }));
    expect(screen.getByRole('log', { name: 'Output of 1/2 · npm ci' })).toHaveTextContent('No output yet.');
  });

  it('offers the older messages with the plural of English', async () => {
    setLang('en');
    const first = setup({ status: 'idle' }, many(100));
    expect(await screen.findByRole('button', { name: 'Show the previous 20 messages' })).toBeInTheDocument();
    first.unmount();
    setup({ status: 'idle' }, many(81));
    expect(await screen.findByRole('button', { name: 'Show the previous message' })).toBeInTheDocument();
  });

  it('names the agent as its harness does: Claude for Claude Code, another one its own', async () => {
    setLang('en');
    const first = setup({ status: 'idle', engine: 'claude' });
    expect(await screen.findByText(/^Describe the task to give to Claude. The agent works in/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Describe the task to give to Claude…')).toBeInTheDocument();
    first.unmount();
    const second = setup({ status: 'idle', engine: 'codex' });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    expect(await screen.findByText(/^Describe the task to give to Nu. The agent works in/)).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Describe the task to give to Nu…')).toBeInTheDocument();
    second.unmount();
    setup({ status: 'running', engine: 'codex' });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    await tick();
    expect(screen.getByText('Nu is working…')).toBeInTheDocument();
    expect(screen.queryByText('Claude is working…')).not.toBeInTheDocument();
  });

  it('follows a change of language while it is open', async () => {
    setup({ status: 'running' });
    expect(screen.getByText('Claude travaille…')).toBeInTheDocument();
    setLang('en');
    await tick();
    expect(screen.getByText('Claude is working…')).toBeInTheDocument();
    expect(screen.queryByText('Claude travaille…')).not.toBeInTheDocument();
  });
});

describe('« Vue Claude Code »', () => {
  it('switches the agent to Claude Code’s interface from its header', async () => {
    setLang('fr');
    const a = agent({ id: 'vt1', status: 'done' });
    resetApp({ projects: [project()], agents: [a] });
    const b = fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    await userEvent.click(screen.getByRole('button', { name: 'Vue Claude Code' }));
    expect(b.called('set_agent_view')[0].args).toEqual({ id: 'vt1', terminal: true });
  });

  it('shows the interface in place of the messages and the composer, and offers to go back', async () => {
    setLang('fr');
    const a = agent({ id: 'vt2', status: 'done', terminal: true });
    resetApp({ projects: [project()], agents: [a] });
    const b = fakeBackend({ get_conversation: () => [] });
    const { container } = render(Conversation, { agent: a, project: project() });
    expect(container.querySelector('.scroll')).toBeNull();
    expect(container.querySelector('textarea')).toBeNull();
    expect(container.querySelector('.tui')).not.toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Vue Escouade' }));
    expect(b.called('set_agent_view')[0].args).toEqual({ id: 'vt2', terminal: false });
  });

  it('is never offered to a ticket’s agent', () => {
    setLang('fr');
    const a = agent({ id: 'vt3', status: 'done', ticketId: 'k1' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    expect(screen.queryByRole('button', { name: 'Vue Claude Code' })).toBeNull();
  });

  it('is never offered to an archived agent, in either view', () => {
    setLang('fr');
    for (const terminal of [false, true]) {
      const a = agent({ id: 'vt4', status: 'done', archived: true, terminal });
      resetApp({ projects: [project()], agents: [a] });
      fakeBackend({ get_conversation: () => [] });
      const { unmount } = render(Conversation, { agent: a, project: project() });
      expect(screen.queryByRole('button', { name: /^Vue (Claude Code|Escouade)$/ })).toBeNull();
      unmount();
    }
  });
});

describe('« Vue <harness> » (ESC-34)', () => {
  /** A harness of its own interface in a terminal, under its name. */
  const WITH_VIEW = { ...BARE_ENGINE, name: 'Codex', caps: { ...BARE_ENGINE.caps, terminalView: true } };

  it('is named after the agent’s harness, in the button and in its terminal', async () => {
    setLang('fr');
    const a = agent({ id: 'vh1', status: 'done', engine: 'codex' });
    resetApp({ projects: [project()], agents: [a] });
    app.engines = [CLAUDE_ENGINE, WITH_VIEW];
    const b = fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    await userEvent.click(screen.getByRole('button', { name: 'Vue Codex' }));
    expect(b.called('set_agent_view')[0].args).toEqual({ id: 'vh1', terminal: true });
    expect(screen.queryByRole('button', { name: 'Vue Claude Code' })).toBeNull();
  });

  it('says so in English', () => {
    setLang('en');
    const a = agent({ id: 'vh2', status: 'done', engine: 'codex' });
    resetApp({ projects: [project()], agents: [a] });
    app.engines = [CLAUDE_ENGINE, WITH_VIEW];
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    expect(screen.getByRole('button', { name: 'Codex view' })).toBeInTheDocument();
    setLang('fr');
  });

  it('is hidden for a harness with no terminal view', () => {
    setLang('fr');
    const a = agent({ id: 'vh3', status: 'done', engine: 'codex' });
    resetApp({ projects: [project()], agents: [a] });
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    expect(screen.queryByRole('button', { name: /^Vue / })).toBeNull();
  });
});

describe('« Outils d’Escouade au prochain démarrage » (ESC-91)', () => {
  const banner = () => screen.queryByRole('region', { name: 'Outils d’Escouade au prochain démarrage' });

  it('says nothing while the agent’s process has what a start would give it', () => {
    setLang('fr');
    const a = agent({ id: 'en0', status: 'done', escouadeNext: false });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    expect(banner()).toBeNull();
  });

  it('says an agent at rest gets Escouade’s tools at its next start, and restarts its process on its session', async () => {
    setLang('fr');
    const a = agent({ id: 'en1', status: 'done', escouadeNext: true });
    resetApp({ projects: [project()], agents: [a] });
    const b = fakeBackend({ get_conversation: () => [], restart_agent_process: () => null });
    render(Conversation, { agent: a, project: project() });
    const region = banner()!;
    expect(region).not.toBeNull();
    expect(region).toHaveTextContent(
      'Son process a démarré sans les outils d’Escouade, que « Les agents peuvent piloter Escouade » lui donne maintenant : relance-le pour qu’il les ait. Sa session est reprise et sa conversation reste.',
    );
    // At rest with nothing in the background, nothing stops it: neither phrase of the wait is there.
    expect(region).not.toHaveTextContent('Il pourra être relancé');
    expect(region).not.toHaveTextContent('tâches de fond');
    await userEvent.click(within(region).getByRole('button', { name: 'Relancer' }));
    expect(b.called('restart_agent_process').map((c) => c.args)).toEqual([{ id: 'en1' }]);
  });

  const BUSY = 'Il pourra être relancé quand il aura fini ce qu’il fait.';
  const STOP_FIRST =
    'Il a encore des tâches de fond (commande, surveillance, sous-agent, workflow) : le relancer les arrêterait. Une commande ou un serveur de dev ne finit pas seul : arrête-la d’abord, depuis sa vue (« Arrêter »), ou attends qu’elle ait fini.';
  const bgTask = (over: Record<string, unknown> = {}) =>
    ({ id: 'bg1', kind: 'command', title: 'Serveur de dev', status: 'running', startedAt: 1, ...over }) as never;
  const planWith = (over: Record<string, unknown>) =>
    ({ source: null, tasks: [], launched: 0, agents: [], workflows: [], bgTasks: [], ...over }) as never;
  const sub = (over: Record<string, unknown> = {}) =>
    ({ id: 'c1', title: 'Relire', status: 'running', background: true, tools: 0, startedAt: 1, ...over }) as never;
  const flow = (over: Record<string, unknown> = {}) =>
    ({ id: 'w1', name: 'review', title: 'Revue', status: 'running', tools: 0, tokens: 0, startedAt: 1, ...over }) as never;

  /** Draws the conversation of `a` and gives its button, and the backend that answers a restart. */
  function drawn(a: ReturnType<typeof agent>) {
    resetApp({ projects: [project()], agents: [a] });
    const backend = fakeBackend({ get_conversation: () => [], restart_agent_process: () => null });
    const view = render(Conversation, { agent: a, project: project() });
    return { ...view, backend, button: () => within(banner()!).getByRole('button', { name: 'Relancer' }) };
  }

  it('waits for the end of its turn before it can be restarted, or of its worktree’s setup', () => {
    setLang('fr');
    for (const over of [
      { status: 'running' as const },
      { status: 'waiting' as const },
      { status: 'idle' as const, setup: '1/2 · npm ci' },
    ]) {
      const { button, unmount } = drawn(agent({ id: 'en2', escouadeNext: true, ...over }));
      expect(button()).toBeDisabled();
      expect(banner()).toHaveTextContent(BUSY);
      expect(banner()).not.toHaveTextContent('tâches de fond');
      unmount();
    }
  });

  it('keeps the wait for its turn when a task runs in the background as well: the turn ends first', () => {
    setLang('fr');
    const { button } = drawn(agent({ id: 'en6', status: 'running', escouadeNext: true, plan: planWith({ bgTasks: [bgTask()] }) }));
    expect(button()).toBeDisabled();
    expect(banner()).toHaveTextContent(BUSY);
    expect(banner()).not.toHaveTextContent(STOP_FIRST);
  });

  it('refuses a restart while a task runs in the background, and says to stop it first (a dev server never ends by itself)', async () => {
    setLang('fr');
    const a = agent({ id: 'en7', status: 'done', escouadeNext: true, plan: planWith({ bgTasks: [bgTask()] }) });
    const { button, backend } = drawn(a);
    expect(button()).toBeDisabled();
    expect(banner()).toHaveTextContent(STOP_FIRST);
    expect(banner()).not.toHaveTextContent(BUSY);
    await userEvent.click(button());
    expect(backend.called('restart_agent_process')).toHaveLength(0);
  });

  it('counts a monitor or a remote agent among the tasks, as the backend does', () => {
    setLang('fr');
    for (const kind of ['monitor', 'remote', 'teammate']) {
      const { button, unmount } = drawn(
        agent({ id: 'en8', status: 'done', escouadeNext: true, plan: planWith({ bgTasks: [bgTask({ kind })] }) }),
      );
      expect(button()).toBeDisabled();
      expect(banner()).toHaveTextContent(STOP_FIRST);
      unmount();
    }
  });

  it('can be restarted again once that task is over (finished, failed or stopped)', async () => {
    setLang('fr');
    const a = agent({
      id: 'en9',
      status: 'done',
      escouadeNext: true,
      plan: planWith({ bgTasks: [bgTask()] }),
    });
    const { button, rerender } = drawn(a);
    expect(button()).toBeDisabled();
    for (const status of ['done', 'failed', 'stopped', 'interrupted'] as const) {
      await rerender({ agent: { ...a, plan: planWith({ bgTasks: [bgTask({ status, endedAt: 5 })] }) }, project: project() });
      expect(button()).toBeEnabled();
      expect(banner()).not.toHaveTextContent(STOP_FIRST);
      expect(banner()).not.toHaveTextContent(BUSY);
    }
  });

  it('refuses a restart while a workflow or a subagent runs alone in the background, with the same words', () => {
    setLang('fr');
    for (const plan of [planWith({ workflows: [flow()] }), planWith({ agents: [sub()], launched: 1 })]) {
      const { button, unmount } = drawn(agent({ id: 'en5', status: 'done', escouadeNext: true, plan }));
      expect(button()).toBeDisabled();
      expect(banner()).toHaveTextContent(STOP_FIRST);
      expect(banner()).not.toHaveTextContent(BUSY);
      unmount();
    }
  });

  it('is not held back by a subagent or a workflow that is over', () => {
    setLang('fr');
    const plan = planWith({
      agents: [sub({ status: 'done' })],
      workflows: [flow({ status: 'failed' })],
      launched: 1,
    });
    const { button } = drawn(agent({ id: 'en10', status: 'done', escouadeNext: true, plan }));
    expect(button()).toBeEnabled();
  });

  it('is not held back by a plan from an older backend, which has no list of tasks', () => {
    setLang('fr');
    const { button } = drawn(
      agent({
        id: 'en11',
        status: 'done',
        escouadeNext: true,
        plan: { source: null, tasks: [], launched: 0, agents: [], workflows: [] } as never,
      }),
    );
    expect(button()).toBeEnabled();
  });

  it('in the Claude Code view, starts its interface again (its session resumed)', async () => {
    setLang('fr');
    const a = agent({ id: 'en3', status: 'done', terminal: true, escouadeNext: true });
    resetApp({ projects: [project()], agents: [a] });
    const b = fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    await userEvent.click(within(banner()!).getByRole('button', { name: 'Relancer' }));
    expect(b.called('restart_agent_process')).toHaveLength(0);
    expect(app.relaunched[a.id]).toBe(1);
  });

  // ESC-106: the app replaces the agent by a new object with each update of it, the end of its interface among them.
  it('in the Claude Code view, starts no other interface when the agent is replaced after its interface stopped', async () => {
    setLang('fr');
    const a = agent({ id: 'tui1', status: 'done', terminal: true, terminalId: null });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ get_conversation: () => [] });
    tui.opened = 0;
    const r = render(Conversation, { agent: a, project: project() });
    await waitFor(() => expect(tui.opened).toBe(1));
    // The terminal stopped (as it does, the session not found), then the agent that says so, replaced by a new object.
    app.exitedTerms['t1'] = 1;
    for (let i = 0; i < 5; i++) {
      await r.rerender({
        agent: { ...a, terminalId: null, tokens: i, tuiExit: { code: 1, quick: true, sessionLost: false } },
        project: project(),
      });
      await new Promise((done) => setTimeout(done, 10));
    }
    expect(tui.opened).toBe(1);
    expect(await screen.findByText(/Claude Code s’est arrêté dès son lancement \(code 1\)/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Relancer' })).toBeInTheDocument();
  });

  it('is written in English when the interface is', () => {
    setLang('en');
    const a = agent({ id: 'en4', status: 'done', escouadeNext: true });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    const region = screen.getByRole('region', { name: 'Escouade’s tools at the next start' });
    expect(region).toHaveTextContent(
      'Its process started without Escouade’s tools, which “Agents can drive Escouade” now gives it: restart it to give it the tools. Its session is resumed and its conversation stays.',
    );
    expect(within(region).getByRole('button', { name: 'Restart' })).toBeEnabled();
    setLang('fr');
  });

  it('is written in English when a task runs in the background, and when a turn does', () => {
    setLang('en');
    try {
      const { unmount } = drawn(agent({ id: 'en12', status: 'done', escouadeNext: true, plan: planWith({ bgTasks: [bgTask()] }) }));
      const region = screen.getByRole('region', { name: 'Escouade’s tools at the next start' });
      expect(within(region).getByRole('button', { name: 'Restart' })).toBeDisabled();
      expect(region).toHaveTextContent(
        'It still has background tasks (command, monitor, subagent, workflow): restarting it would stop them. A command or a dev server doesn’t end by itself: stop it first, from its view (“Stop”), or wait until it has finished.',
      );
      unmount();
      drawn(agent({ id: 'en13', status: 'running', escouadeNext: true }));
      expect(screen.getByRole('region', { name: 'Escouade’s tools at the next start' })).toHaveTextContent(
        'It can be restarted once it is done with what it is doing.',
      );
    } finally {
      setLang('fr');
    }
  });
});

describe('Conversation and the subagents of its agent', () => {
  const launch = {
    kind: 'tool',
    id: 'c1',
    name: 'Agent',
    input: { description: 'Chercher la cause', prompt: 'Cherche.' },
    status: 'ok',
    result: { text: 'Async agent launched successfully.\nagentId: t1', isError: false },
    ts: 1,
  };
  const asked = {
    kind: 'permission',
    id: 'r1',
    toolUseId: 'k1',
    toolName: 'Bash',
    input: { command: 'curl localhost:3000' },
    canAlways: false,
    defaultNo: false,
    decision: null,
    ts: 2,
    sub: 'c1',
  };

  it('opens a subagent’s conversation from the line of its call', async () => {
    const { a } = setup({}, [launch]);
    await userEvent.click(await screen.findByRole('button', { name: 'Ouvrir la conversation du sous-agent Chercher la cause' }));
    expect(app.subView[a.id]).toBe('c1');
  });

  it('says which subagent made a request, which is answered here too, and opens it', async () => {
    const { a } = setup({ status: 'waiting', pending: ['r1'] }, [launch, asked]);
    expect(await screen.findByText('Demande du sous-agent « Chercher la cause »')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Autoriser/ })).toBeInTheDocument();
    const opens = screen.getAllByRole('button', { name: 'Ouvrir sa conversation' });
    await userEvent.click(opens.at(-1)!);
    expect(app.subView[a.id]).toBe('c1');
  });

  it('says nothing of a request whose subagent nothing knows, and offers no way to it', async () => {
    setup({ status: 'waiting', pending: ['r1'] }, [{ ...asked, sub: 'gone' }]);
    // Still answered here.
    expect(await screen.findByRole('button', { name: /Autoriser/ })).toBeInTheDocument();
    expect(screen.queryByText(/Demande du sous-agent/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Ouvrir sa conversation' })).not.toBeInTheDocument();
  });

  describe('back from a subagent’s conversation', () => {
    /** The agent, its conversation (the call of a subagent) and an element that has the focus, as a subagent's view is opened then left. */
    function leave(opener: HTMLElement | null) {
      const a = agent({ id: `fb${Math.random()}`, status: 'done' });
      resetApp({ projects: [project()], agents: [a] });
      fakeBackend({ get_conversation: () => [launch] });
      if (opener) {
        document.body.append(opener);
        opener.focus();
      }
      app.openSubagent(a.id, 'c1');
      return a;
    }
    afterEach(() => document.querySelectorAll('.opener').forEach((e) => e.remove()));
    const button = () => Object.assign(document.createElement('button'), { className: 'opener' });

    it('puts the focus back where the view was opened from', async () => {
      const opener = button();
      const a = leave(opener);
      expect(app.subFocus.from?.el).toBe(opener);
      app.closeSubagent(a.id);
      render(Conversation, { agent: a, project: project() });
      await waitFor(() => expect(opener).toHaveFocus());
      // Once: it is not taken again by the next conversation drawn.
      expect(app.takeFocusBack(a.id)).toBeNull();
    });

    it('remembers the first opener when a subagent opens another', async () => {
      const opener = button();
      const a = leave(opener);
      const inside = button();
      document.body.append(inside);
      inside.focus();
      app.openSubagent(a.id, 'c2');
      expect(app.subView[a.id]).toBe('c2');
      app.closeSubagent(a.id);
      render(Conversation, { agent: a, project: project() });
      await waitFor(() => expect(opener).toHaveFocus());
    });

    it('puts it on the button of the call when the element is gone from the document', async () => {
      const opener = button();
      const a = leave(opener);
      opener.remove();
      app.closeSubagent(a.id);
      render(Conversation, { agent: a, project: project() });
      const open = await screen.findByRole('button', { name: 'Ouvrir la conversation du sous-agent Chercher la cause' });
      await waitFor(() => expect(open).toHaveFocus());
    });

    describe('to the stack of dots of the plan', () => {
      const running = (id: string, title: string) => ({ id, title, status: 'running' as const, background: false, tools: 0, startedAt: 1 });
      const planOf = (...subs: ReturnType<typeof running>[]) => ({
        source: 'tools' as const,
        tasks: [{ id: '1', title: 'Chercher', status: 'inProgress' as const }],
        agents: subs,
        launched: subs.length,
        workflows: [],
        bgTasks: [],
      });
      const ANY = 'Ouvrir la conversation d’un sous-agent';
      const withPlan = (id: string, ...subs: ReturnType<typeof running>[]) => {
        const a = agent({ id, status: 'running', plan: planOf(...subs) });
        resetApp({ projects: [project()], agents: [a] });
        fakeBackend({ get_conversation: () => [launch] });
        return a;
      };

      it('puts the focus back on the stack when the subagent was chosen from its menu', async () => {
        const a = withPlan('fbmenu', running('c1', 'Chercher la cause'), running('c2', 'Relire'));
        render(ContextMenu);
        const first = render(Conversation, { agent: a, project: project() });
        await userEvent.click(await screen.findByRole('button', { name: ANY }));
        await userEvent.click(await screen.findByRole('menuitem', { name: /^Relire/ }));
        await waitFor(() => expect(app.subView[a.id]).toBe('c2'));
        expect(app.subFocus.from?.opener).toBe('dots');
        // The view of the subagent takes the place of the conversation, which is drawn anew when it is left.
        first.unmount();
        app.closeSubagent(a.id);
        render(Conversation, { agent: a, project: project() });
        const stack = await screen.findByRole('button', { name: ANY });
        await waitFor(() => expect(stack).toHaveFocus());
      });

      // The line of the call has the same name as the stack of one subagent: the banner is where to look.
      const inPlan = (name: string) => within(document.querySelector<HTMLElement>('.plan')!).findByRole('button', { name });
      const ONE = 'Ouvrir la conversation du sous-agent Chercher la cause';

      it('puts it back on the stack of one subagent, which opened at once', async () => {
        const a = withPlan('fbone', running('c1', 'Chercher la cause'));
        const first = render(Conversation, { agent: a, project: project() });
        await userEvent.click(await inPlan(ONE));
        expect(app.subView[a.id]).toBe('c1');
        first.unmount();
        app.closeSubagent(a.id);
        render(Conversation, { agent: a, project: project() });
        const stack = await inPlan(ONE);
        await waitFor(() => expect(stack).toHaveFocus());
      });

      it('finds the stack again whichever subagents it stands for by then', async () => {
        const a = withPlan('fbwho', running('c1', 'Chercher la cause'));
        const first = render(Conversation, { agent: a, project: project() });
        await userEvent.click(await inPlan(ONE));
        first.unmount();
        app.closeSubagent(a.id);
        // That subagent ended while its view was open: two others run, and the stack stands for them.
        render(Conversation, { agent: { ...a, plan: planOf(running('c2', 'Relire'), running('c3', 'Tester')) }, project: project() });
        const stack = await screen.findByRole('button', { name: ANY });
        await waitFor(() => expect(stack).toHaveFocus());
      });

      it('falls back to the button of the call when no stack is left', async () => {
        const a = withPlan('fbnone', running('c1', 'Chercher la cause'));
        const first = render(Conversation, { agent: a, project: project() });
        await userEvent.click(await inPlan(ONE));
        first.unmount();
        app.closeSubagent(a.id);
        render(Conversation, { agent: { ...a, plan: undefined }, project: project() });
        // The only way left to that subagent is the line of its call.
        const open = await screen.findByRole('button', { name: ONE });
        await waitFor(() => expect(open).toHaveFocus());
      });
    });

    it('moves nothing when the view was left by choosing the agent (the composer has the focus then)', async () => {
      const opener = button();
      const a = leave(opener);
      app.selectAgent(a.id);
      expect(app.subView[a.id]).toBeNull();
      render(Conversation, { agent: a, project: project() });
      await screen.findByRole('button', { name: 'Ouvrir la conversation du sous-agent Chercher la cause' });
      await tick();
      expect(opener).not.toHaveFocus();
    });
  });
});

describe('« Vue OpenCode » (ESC-46)', () => {
  /** OpenCode: its interface joins the agent's process (`attached`), not in its place (`terminal`). */
  const OPENCODE = { ...BARE_ENGINE, id: 'opencode' as const, name: 'OpenCode', caps: { ...BARE_ENGINE.caps, terminalView: true } };

  it('shows the interface joined to the process in place of the messages, and offers to go back', async () => {
    setLang('fr');
    const a = agent({ id: 'oa1', status: 'running', engine: 'opencode', attached: true });
    resetApp({ projects: [project()], agents: [a] });
    app.engines = [CLAUDE_ENGINE, OPENCODE];
    const b = fakeBackend({ get_conversation: () => [] });
    const { container } = render(Conversation, { agent: a, project: project() });
    expect(container.querySelector('.scroll')).toBeNull();
    expect(container.querySelector('.tui')).not.toBeNull();
    // The two views in segments: the one shown pressed.
    expect(screen.getByRole('button', { name: 'Vue OpenCode' })).toHaveAttribute('aria-pressed', 'true');
    const back = screen.getByRole('button', { name: 'Vue Escouade' });
    expect(back).toHaveAttribute('aria-pressed', 'false');
    await userEvent.click(back);
    expect(b.called('set_agent_view')[0].args).toEqual({ id: 'oa1', terminal: false });
  });

  it('is offered from the conversation, during a turn too', async () => {
    setLang('fr');
    const a = agent({ id: 'oa2', status: 'running', engine: 'opencode' });
    resetApp({ projects: [project()], agents: [a] });
    app.engines = [CLAUDE_ENGINE, OPENCODE];
    const b = fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    await userEvent.click(screen.getByRole('button', { name: 'Vue OpenCode' }));
    expect(b.called('set_agent_view')[0].args).toEqual({ id: 'oa2', terminal: true });
  });
});

describe('Conversation header — the agent’s pull requests (ESC-64)', () => {
  const WT = { path: 'C:\wt\login', branch: 'escouade/login', baseBranch: 'main', existing: false };
  afterEach(() => setLang('fr'));

  it('reads them when the agent is shown, and offers « Ouvrir une PR » while none is open', async () => {
    const a = agent({ id: 'pr1', worktree: WT, prWord: 'PR' });
    resetApp({ projects: [project()], agents: [a] });
    const b = fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    await waitFor(() => expect(b.called('agent_prs_refresh').map((c) => c.args)).toEqual([{ id: 'pr1', now: false }]));
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir une PR' }));
    expect(app.modal).toEqual({ kind: 'pullRequest', agentId: 'pr1' });
  });

  it('reads them but offers no « Ouvrir une PR » while the backend has not said its host opens them (no remote, a host without a client yet, no account)', async () => {
    const a = agent({ id: 'pr5', worktree: WT });
    resetApp({ projects: [project()], agents: [a] });
    const b = fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    await waitFor(() => expect(b.called('agent_prs_refresh')).toHaveLength(1));
    expect(screen.queryByRole('button', { name: 'Ouvrir une PR' })).not.toBeInTheDocument();
  });

  it('shows their numbers, each opening in the browser, and no « Ouvrir une PR » once one is open', () => {
    const prs = [pullRequest({ number: 12, checks: 'failure' }), pullRequest({ number: 9, state: 'merged' })];
    const a = agent({ id: 'pr2', worktree: WT, prs, prWord: 'MR' });
    resetApp({ projects: [project()], agents: [a] });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: a, project: project() });
    const twelve = screen.getByRole('button', { name: 'Ouvrir la MR #12 dans le navigateur (ouverte)' });
    expect(twelve).toHaveTextContent(/^#12$/);
    // Beside the branch, the icon of its state coloured by what to look at first.
    expect(twelve.closest('.crumbs')).not.toBeNull();
    expect(twelve).toHaveClass('fail');
    expect(twelve.querySelector('[data-icon="close"]')).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Ouvrir la MR #9 dans le navigateur (mergée)' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Ouvrir une MR' })).not.toBeInTheDocument();
  });

  it('offers nothing to open to a ticket’s agent, whose pull request is only read, and is said in English', () => {
    const a = agent({ id: 'pr3', worktree: WT, ticketId: 't1' });
    resetApp({ projects: [project()], agents: [a] });
    const b = fakeBackend({ get_conversation: () => [] });
    const r = render(Conversation, { agent: a, project: project() });
    expect(screen.queryByRole('button', { name: 'Ouvrir une PR' })).not.toBeInTheDocument();
    // Its board opened it: read for the status bar, never offered to open.
    expect(b.called('agent_prs_refresh').map((c) => c.args)).toEqual([{ id: 'pr3', now: false }]);
    r.unmount();
    setLang('en');
    const c = agent({ id: 'pr4', worktree: WT, prWord: 'PR' });
    resetApp({ projects: [project()], agents: [c] });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: c, project: project() });
    expect(screen.getByRole('button', { name: 'Open a PR' })).toBeInTheDocument();
  });
});

describe('Conversation: files an agent sends (ESC-171)', () => {
  const PNG = 'data:image/png;base64,iVBORw0KGgo=';
  const call = {
    kind: 'tool',
    id: 'toolu_s1',
    name: 'SendUserFile',
    input: {
      files: ['C:/w/.impeccable/tmp/a.png', 'C:/w/b.png', 'C:/w/contrat.pdf'],
      caption: 'Deux vues et le contrat',
      status: 'normal',
      display: 'render',
    },
    status: 'ok',
    ts: 1,
  };

  it('shows the caption, the images as thumbnails and the other files with their actions', async () => {
    setLang('fr');
    const a = agent({ id: 'sender', status: 'idle' });
    resetApp({ projects: [project()], agents: [a] });
    const asked: unknown[] = [];
    fakeBackend({
      get_conversation: () => [call],
      sent_files: (args: unknown) => {
        asked.push(args);
        return [
          { name: 'a.png', kind: 'image', state: 'ok', size: 2048, url: PNG },
          { name: 'b.png', kind: 'image', state: 'ok', size: 4096, url: PNG },
          { name: 'contrat.pdf', kind: 'file', state: 'ok', size: 1468006 },
        ];
      },
    });
    render(Conversation, { agent: a, project: project() });
    expect(await screen.findByText('Deux vues et le contrat')).toBeInTheDocument();
    await waitFor(() => expect(screen.getAllByRole('img').filter((i) => i.getAttribute('src') === PNG)).toHaveLength(2));
    expect(asked[0]).toEqual({ agentId: 'sender', itemId: 'toolu_s1' });
    expect(screen.getByRole('button', { name: 'Voir a.png en grand' })).toBeInTheDocument();
    expect(screen.getByText('contrat.pdf')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Ouvrir contrat.pdf' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Afficher contrat.pdf dans le dossier' })).toBeInTheDocument();
    // Not a tool call among the others.
    expect(screen.queryByText('SendUserFile')).not.toBeInTheDocument();
  });
});

describe('the entry in a step of a workflow (ESC-197, choice 145)', () => {
  it('draws a divider before the message of the door, which leads to the Workflow tab on that workflow', async () => {
    const at = new Date(2026, 9, 26, 10, 47).getTime();
    const items = [
      { kind: 'user', id: 'u0', text: 'Bonjour', images: 0, ts: at - 600_000, queued: false },
      { kind: 'user', id: 'u1', text: '/gsd:verify-work 3', images: 0, ts: at, queued: false },
    ];
    const { a } = setup({}, items);
    const named = { ...gsd(), steps: gsd().steps.map((s) => (s.name === 'gsd:verify-work' ? { ...s, name: 'Vérifier', named: true } : s)) };
    workflows.scans.p1 = scanOf(named);
    workflows.take(
      track(a.id, {
        step: 'Vérifier',
        passages: [passage(1, 'Vérifier', 47, 1, { door: '/gsd:verify-work 3', outcome: 'running', at: at + 30 })],
      }),
    );
    const div = await screen.findByTestId('step-divider');
    expect(div).toHaveTextContent('GSD · Vérifier/gsd:verify-work 310:47');
    // Before the message of its door.
    expect(div.nextElementSibling).toHaveAttribute('data-item', 'u1');
    await userEvent.click(within(div).getByRole('button', { name: 'GSD · Vérifier' }));
    expect(app.panelTab).toBe('workflow');
    expect(app.filesOpen).toBe(true);
    expect(workflows.chosen.p1).toBe(GSD_ID);
    // A workflow dismissed is in no list: no divider.
    workflows.scans.p1 = scanOf();
    await waitFor(() => expect(screen.queryByTestId('step-divider')).toBeNull());
  });
});
