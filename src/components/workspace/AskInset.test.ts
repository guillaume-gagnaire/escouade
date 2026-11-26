import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { conversationOf, dropConversation } from '../../lib/conversations.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import { permissionItem, questionItem, req } from '../../test/workspace';
import AskInset from './AskInset.svelte';

describe('the questions on a card of the workspace', () => {
  let backend: ReturnType<typeof fakeBackend>;
  let let_go: () => void;
  beforeEach(() => {
    setLang('fr');
    backend = fakeBackend({
      get_conversation: () => [questionItem('q1'), questionItem('q2', { sub: 's1' }), permissionItem('p1', 'rm -rf dist')],
    });
    resetApp({
      agents: [
        agent({ id: 'a1', status: 'waiting', pending: ['q1', 'q2', 'p1'], requests: [req('q1'), req('q2'), req('p1', 'permission')] }),
      ],
    });
    // The agent's card keeps its conversation loaded while it waits (AgentCard).
    let_go = conversationOf('a1').show();
  });
  afterEach(() => {
    let_go();
    dropConversation('a1');
  });

  it("shows the agent's own requests with the conversation's cards", async () => {
    render(AskInset, { agent: app.agents.a1, mine: (s?: string) => !s, maxH: 700 });
    expect(await screen.findByText('Question q1 ?')).toBeTruthy();
    expect(screen.getByText('rm -rf dist')).toBeTruthy();
    // That of the subagent s1 is on its own card.
    expect(screen.queryByText('Question q2 ?')).toBeNull();
  });

  it('shows those of a subagent on its card', async () => {
    render(AskInset, { agent: app.agents.a1, mine: (s?: string) => s === 's1', maxH: 700 });
    expect(await screen.findByText('Question q2 ?')).toBeTruthy();
    expect(screen.queryByText('Question q1 ?')).toBeNull();
  });

  it("answers by the conversation's card", async () => {
    render(AskInset, { agent: app.agents.a1, mine: (s?: string) => !s, maxH: 700 });
    await fireEvent.click(await screen.findByRole('button', { name: 'Oui' }));
    expect(backend.called('answer_question')[0].args).toMatchObject({ id: 'a1', requestId: 'q1', answers: { 'Question q1 ?': 'Oui' } });
  });

  it('shows no answer key: several cards may wait at once, none takes the keys', async () => {
    const { container } = render(AskInset, { agent: app.agents.a1, mine: (s?: string) => !s, maxH: 700 });
    await screen.findByText('Question q1 ?');
    expect(container.querySelector('kbd')).toBeNull();
  });

  it('scrolls past its height', async () => {
    const { container } = render(AskInset, { agent: app.agents.a1, mine: (s?: string) => !s, maxH: 420 });
    await screen.findByText('Question q1 ?');
    expect((container.querySelector('.ask') as HTMLElement).style.maxHeight).toBe('420px');
  });

  it('says its question loads', () => {
    dropConversation('a1');
    backend = fakeBackend({ get_conversation: () => new Promise(() => {}) });
    let_go = conversationOf('a1').show();
    render(AskInset, { agent: app.agents.a1, mine: (s?: string) => !s, maxH: 700 });
    expect(screen.getByText('Chargement de la question…')).toBeTruthy();
  });

  it('shows nothing once answered', async () => {
    const { container, rerender } = render(AskInset, { agent: app.agents.a1, mine: (s?: string) => !s, maxH: 700 });
    await screen.findByText('Question q1 ?');
    await rerender({ agent: { ...app.agents.a1, status: 'running', pending: [], requests: [] }, mine: (s?: string) => !s, maxH: 700 });
    expect(container.querySelector('.ask')).toBeNull();
  });

  it('gives the focus back to its card once the request it held is answered', async () => {
    const back = vi.fn();
    const { rerender } = render(AskInset, { agent: app.agents.a1, mine: (s?: string) => !s, maxH: 700, onfocusback: back });
    const yes = await screen.findByRole('button', { name: 'Oui' });
    yes.focus();
    await fireEvent.click(yes);
    await rerender({ agent: { ...app.agents.a1, pending: ['q2', 'p1'] }, mine: (s?: string) => !s, maxH: 700, onfocusback: back });
    expect(screen.queryByText('Question q1 ?')).toBeNull();
    expect(back).toHaveBeenCalledOnce();
  });

  it('keeps the wheel only when it can scroll that way, else the canvas pans', async () => {
    const { container } = render(AskInset, { agent: app.agents.a1, mine: (s?: string) => !s, maxH: 700 });
    await screen.findByText('Question q1 ?');
    const ask = container.querySelector('.ask') as HTMLElement;
    const panned = vi.fn();
    container.addEventListener('wheel', panned);
    const wheel = (deltaY: number) => ask.dispatchEvent(new WheelEvent('wheel', { deltaY, bubbles: true }));
    wheel(50); // nothing to scroll
    expect(panned).toHaveBeenCalledTimes(1);
    Object.defineProperty(ask, 'scrollHeight', { value: 500 });
    Object.defineProperty(ask, 'clientHeight', { value: 100 });
    wheel(50); // down, room below
    expect(panned).toHaveBeenCalledTimes(1);
    wheel(-50); // up, at the top
    expect(panned).toHaveBeenCalledTimes(2);
    ask.scrollTop = 400;
    wheel(50); // down, at the bottom
    expect(panned).toHaveBeenCalledTimes(3);
  });
});
