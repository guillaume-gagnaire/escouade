import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { conversationOf, dropConversation } from '../../lib/conversations.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import { questionItem, req, sub } from '../../test/workspace';
import TeamCard from './TeamCard.svelte';

describe('the team of an agent on the workspace', () => {
  let backend: ReturnType<typeof fakeBackend>;
  beforeEach(() => {
    setLang('fr');
    backend = fakeBackend();
    resetApp();
  });

  it('lets one write to a subagent running in the background', async () => {
    const s = { ...sub('s1'), kind: 'Explore', taskId: 't1', background: true };
    render(TeamCard, { agent: agent({ id: 'a1' }), item: { kind: 'sub', id: 's1', sub: s }, ontoggle: () => {} });
    const field = screen.getByRole('textbox', { name: 'Écrire à Explore' });
    await fireEvent.input(field, { target: { value: 'continue' } });
    await fireEvent.keyDown(field, { key: 'Enter' });
    expect(backend.called('message_subagent')[0].args).toEqual({ id: 'a1', call: 's1', text: 'continue' });
  });

  it('has no field for a subagent in the foreground', () => {
    render(TeamCard, { agent: agent(), item: { kind: 'sub', id: 's1', sub: sub('s1') }, ontoggle: () => {} });
    expect(screen.queryByRole('textbox')).toBeNull();
  });

  it('says the pile, and unfolds it on a click', async () => {
    const ontoggle = vi.fn();
    render(TeamCard, { agent: agent(), item: { kind: 'folded', id: 'folded', done: 4, failed: 1, capped: false }, ontoggle });
    await fireEvent.click(screen.getByRole('button', { name: '+ 4 terminés · 1 échec' }));
    expect(ontoggle).toHaveBeenCalled();
  });

  it('says the pile in good French, failures only when there are', () => {
    const pile = (done: number, failed: number, capped = false) =>
      render(TeamCard, { agent: agent(), item: { kind: 'folded', id: 'folded', done, failed, capped }, ontoggle: () => {} });
    pile(1, 0);
    expect(screen.getByRole('button', { name: '+ 1 terminé' })).toBeTruthy();
    pile(29, 1, true);
    expect(screen.getByRole('button', { name: '+ 30+ terminés · 1 échec' })).toBeTruthy();
    pile(3, 2);
    expect(screen.getByRole('button', { name: '+ 3 terminés · 2 échecs' })).toBeTruthy();
  });

  it("leaves the app's shortcuts to the app while one types to a subagent", async () => {
    const heard = vi.fn();
    window.addEventListener('keydown', heard);
    const s = { ...sub('s1'), kind: 'Explore', taskId: 't1', background: true };
    render(TeamCard, { agent: agent({ id: 'a1' }), item: { kind: 'sub', id: 's1', sub: s }, ontoggle: () => {} });
    await fireEvent.keyDown(screen.getByRole('textbox'), { key: 'k', metaKey: true });
    window.removeEventListener('keydown', heard);
    expect(heard).toHaveBeenCalled();
  });

  it('shows the question of its subagent, and grows with it', async () => {
    fakeBackend({ get_conversation: () => [questionItem('q1', { sub: 's1' })] });
    resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] });
    const let_go = conversationOf('a1').show();
    const { container } = render(TeamCard, { agent: app.agents.a1, item: { kind: 'sub', id: 's1', sub: sub('s1') }, ontoggle: () => {} });
    expect(await screen.findByText('Question q1 ?')).toBeTruthy();
    expect(container.querySelector('.card')?.classList).toContain('asks');
    let_go();
    dropConversation('a1');
  });

  it('reports its height by its id, and 0 once gone', async () => {
    const onsize = vi.fn();
    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(140);
    const { unmount } = render(TeamCard, { agent: agent(), item: { kind: 'sub', id: 's1', sub: sub('s1') }, ontoggle: () => {}, onsize });
    await vi.waitFor(() => expect(onsize).toHaveBeenCalledWith('s1', 140));
    unmount();
    expect(onsize).toHaveBeenLastCalledWith('s1', 0);
    vi.restoreAllMocks();
  });
});
