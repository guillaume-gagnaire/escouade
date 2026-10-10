import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// "▶ Tester" starts test launches, which have logs (xterm.js): none in jsdom.
vi.mock('../lib/terminals', () => ({
  launchLog: () => ({
    term: { cols: 80, rows: 24, buffer: { active: { cursorY: 0 } }, write: (_: unknown, done?: () => void) => done?.() },
    fit: { fit() {} },
  }),
  disposeLog() {},
}));

import { applyConvOps, conversationOf, releaseIdle } from '../lib/conversations.svelte';
import { app } from '../lib/state.svelte';
import type { Agent } from '../lib/types';
import { answerClock, settle } from '../test/conversation';
import { agent, fakeBackend, project, resetApp, ticket } from '../test/ipc';
import Conversation from './Conversation.svelte';

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
      render(Conversation, { agent: a, project: project() });
      return { a, backend, finish, textarea: screen.getByRole('textbox') as HTMLTextAreaElement };
    }

    it('shows its bubble at once, saying it waits for the preparation', async () => {
      const { backend, textarea } = sendDuringSetup();
      await userEvent.type(textarea, 'Ajoute des tests{Enter}');
      await waitFor(() => expect(backend.called('send_message')).toHaveLength(1));
      const bubble = (await screen.findByText('Ajoute des tests')).closest('.bubble') as HTMLElement;
      expect(within(bubble).getByText('En attente de la préparation…')).toBeInTheDocument();
      expect(textarea).toHaveValue('');
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

    it('shows the files it carries', async () => {
      const { textarea } = sendDuringSetup();
      await userEvent.upload(
        screen.getByLabelText('Joindre un fichier', { selector: 'input' }),
        new File(['%PDF-1.4'], 'rapport.pdf', { type: 'application/pdf' }),
      );
      await screen.findByText('rapport.pdf');
      await userEvent.type(textarea, 'Résume{Enter}');
      const bubble = (await screen.findByText('En attente de la préparation…')).closest('.bubble') as HTMLElement;
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

  it('names the agent’s model with the version Claude Code runs for it', () => {
    const { container } = setup({ model: 'sonnet' });
    app.models = [{ value: 'sonnet', resolvedModel: 'claude-sonnet-5-5' }];
    return waitFor(() => expect(container.querySelector('.head .model')).toHaveTextContent('Sonnet 5.5'));
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
    expect(items[0]).toHaveTextContent('✓Le fichier existevérifié');
    expect(items[1]).toHaveTextContent('○Tests vertsreste');
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
    ).toEqual(['✓Critère 1', '○Critère 3reste']);
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
    await userEvent.click(within(list.closest('.report') as HTMLElement).getByRole('button', { name: '▶ Tester' }));
    expect(app.modal).toEqual({ kind: 'testLaunch', agentId: a.id });
  });

  it('offers no test from a report while the ticket is being validated, nor for an archived agent', async () => {
    const text = '```escouade\n{"lancement": {"processus": [{"nom": "web", "commande": "npm run dev"}]}}\n```';
    const recipe = { prepare: [], processes: [], open: '' };
    const { a, unmount } = setup({ status: 'done', worktree: WT, recipe }, [{ kind: 'text', id: 'm1', text, streaming: false }]);
    app.tickets.t1 = ticket({ agentId: a.id, column: 'review', step: 'Merge…' });
    const list = await screen.findByRole('list', { name: 'Lancement de test' });
    expect(within(list.closest('.report') as HTMLElement).queryByRole('button', { name: '▶ Tester' })).not.toBeInTheDocument();
    unmount();
    setup({ status: 'done', worktree: WT, recipe, archived: true }, [{ kind: 'text', id: 'm1', text, streaming: false }]);
    await screen.findByRole('list', { name: 'Lancement de test' });
    expect(screen.queryByRole('button', { name: '▶ Tester' })).not.toBeInTheDocument();
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
    expect(screen.getByRole('button', { name: '▶ Tester' }).querySelector('.lbl')).toHaveTextContent('Tester');
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
    expect(screen.getByRole('button', { name: '▶ Tester' })).toBeInTheDocument();
    app.tickets.t1 = ticket({ agentId: a.id, column: 'review', step: 'Tests…' });
    await waitFor(() => expect(screen.queryByRole('button', { name: '▶ Tester' })).not.toBeInTheDocument());
  });

  it('tests a worktree whose services isola runs without a recipe, and asks for none', () => {
    setup({ worktree: WT, isola: true });
    expect(screen.getByRole('button', { name: '▶ Tester' })).toHaveAttribute(
      'title',
      'Lance les services isola de ce worktree et ouvre la fonctionnalité dans le navigateur',
    );
    expect(screen.queryByRole('button', { name: 'Préparer le lancement' })).not.toBeInTheDocument();
  });

  it('offers neither to an agent without a worktree, nor to an archived one', () => {
    const { unmount } = setup({ recipe });
    expect(screen.queryByRole('button', { name: '▶ Tester' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Préparer le lancement' })).not.toBeInTheDocument();
    unmount();
    setup({ worktree: WT, archived: true });
    expect(screen.queryByRole('button', { name: 'Préparer le lancement' })).not.toBeInTheDocument();
  });
});

describe('Conversation header usage', () => {
  it('shows how full the context is, out of the model’s window', () => {
    setup({ contextTokens: 45_200, contextWindow: 200_000 });
    const ctx = screen.getByText('Contexte').closest('.m')!;
    expect(ctx).toHaveTextContent('45,2 k / 200 k');
    expect(ctx).toHaveAttribute('title', expect.stringContaining('23 %'));
    expect(ctx.querySelector('.v')).not.toHaveClass('full');
  });

  it('warns when the context is nearly full', () => {
    setup({ contextTokens: 900_000, contextWindow: 1_000_000 });
    expect(screen.getByText('900,0 k / 1 M')).toHaveClass('full');
  });

  it('shows the context alone while the window is unknown', () => {
    setup({ contextTokens: 12_300, contextWindow: 0 });
    expect(screen.getByText('Contexte').closest('.m')).toHaveTextContent('12,3 k');
  });

  it('counts the running turn in the tokens and the estimated cost', () => {
    setup({ tokens: 2000, cost: 0.4, liveTokens: 1000, liveCost: 0.2 });
    expect(screen.getByText('3,0 k')).toBeInTheDocument();
    expect(screen.getByText('≈ 0,60 $')).toBeInTheDocument();
  });
});

describe('Conversation editor entry', () => {
  it('opens the editor on the agent’s worktree from its header', async () => {
    const wt = { path: 'C:\\code\\demo-api\\.claude\\worktrees\\a1', branch: 'escouade/a1', baseBranch: 'main' };
    resetApp({ agents: [agent({ worktree: wt })] });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: app.agents.a1, project: project() });
    await userEvent.click(screen.getByRole('button', { name: /Éditeur/ }));
    expect(app.editorOn).toBe(true);
    expect(app.editor.p1.source).toBe('a1');
  });

  it('names the header button without its text, which a narrow header hides', () => {
    resetApp({ agents: [agent()] });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: app.agents.a1, project: project() });
    expect(screen.getByRole('button', { name: 'Éditeur' })).toBeInTheDocument();
  });

  it('opens the editor on the project checkout from the header of an agent without a worktree', async () => {
    resetApp({ agents: [agent()] });
    fakeBackend({ get_conversation: () => [] });
    render(Conversation, { agent: app.agents.a1, project: project() });
    await userEvent.click(screen.getByRole('button', { name: /Éditeur/ }));
    expect(app.editor.p1).toMatchObject({ on: true, source: 'project' });
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
    await userEvent.click(screen.getByRole('button', { name: '▶ Tester' }));
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

  it('still refuses with the typed explanation on Enter', async () => {
    const { a, backend } = waiting([perm('r1')]);
    await screen.findByTestId('permission-pending');
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

  describe('Ctrl+J', () => {
    it('puts the focus on the message field of an agent with a request waiting, where typing then Enter refuses', async () => {
      const { a, backend } = waiting([perm('r1')]);
      await screen.findByTestId('permission-pending');
      field().blur();
      app.nextWaiting();
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
      app.nextWaiting();
      await waitFor(() => expect(field()).toHaveFocus());
      await userEvent.keyboard('{Control>}{Enter}{/Control}');
      await waitFor(() => expect(backend.called('answer_permission')).toHaveLength(1));
      expect(backend.called('answer_permission')[0].args).toMatchObject({ requestId: 'r1', decision: 'allow' });
    });

    it('puts the focus on the message field when no request waits', async () => {
      waiting([]);
      field().blur();
      app.nextWaiting();
      await waitFor(() => expect(field()).toHaveFocus());
    });
  });
});
