import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { DraftAttachment } from '../../lib/attachments';
import { HOLD_MS } from '../../lib/answer-keys.svelte';
import { conversationOf, dropConversation } from '../../lib/conversations.svelte';
import { forgetDraft, getDraft, setDraft } from '../../lib/drafts';
import { setLang } from '../../lib/i18n';
import { menu } from '../../lib/menu.svelte';
import { cardSpend } from '../../lib/sidebar';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import { permissionItem, planWith, questionItem, req, sub } from '../../test/workspace';
import AgentCard from './AgentCard.svelte';

const image = (name: string): DraftAttachment => ({ kind: 'image', name, mediaType: 'image/png', data: 'AA==', size: 1 });

/** Renders the card of a1, types `text` in its field and presses Enter. */
async function typeAndEnter(text: string) {
  render(AgentCard, { agent: app.agents.a1 });
  const field = screen.getByRole('textbox', { name: 'Écrire à un' }) as HTMLInputElement;
  await fireEvent.input(field, { target: { value: text } });
  await fireEvent.keyDown(field, { key: 'Enter' });
  return field;
}

describe('the card of an agent on the workspace', () => {
  let backend: ReturnType<typeof fakeBackend>;
  beforeEach(() => {
    setLang('fr');
    backend = fakeBackend({ get_conversation: () => [] });
    resetApp({ agents: [agent({ id: 'a1', name: 'un' })] });
  });
  afterEach(() => {
    menu.close();
    forgetDraft('a1');
    dropConversation('a1');
  });

  it("sends by Enter through the conversation's path", async () => {
    const field = await typeAndEnter('Bonjour');
    await vi.waitFor(() => expect(backend.called('send_message')[0]?.args).toMatchObject({ id: 'a1', text: 'Bonjour' }));
    expect(field.value).toBe('');
  });

  it('shares its draft with the conversation', () => {
    setDraft('a1', { text: 'déjà là', files: [image('a.png')] });
    render(AgentCard, { agent: app.agents.a1 });
    expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('déjà là');
    expect(screen.getByText('1 fichier')).toBeTruthy();
  });

  it('opens its conversation for an interactive command, the text kept', async () => {
    app.openWorkspace('p1');
    // `/model` est interactif pour Claude Code (interactiveOf)
    await typeAndEnter('/model');
    expect(app.workspaceOn).toBe(false);
    expect(getDraft('a1').text).toBe('/model');
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('offers model, effort, mode and « Joindre un fichier »', async () => {
    render(AgentCard, { agent: app.agents.a1 });
    await fireEvent.click(screen.getByRole('button', { name: 'Paramètres' }));
    expect(menu.open!.items.filter((i) => !i.separator).map((i) => i.label)).toEqual(['Modèle', 'Effort', 'Mode', 'Joindre un fichier']);
    expect(menu.open!.items[0].submenu!.some((i) => i.checked)).toBe(true);
  });

  it('opens its conversation on a double click', async () => {
    app.openWorkspace('p1');
    render(AgentCard, { agent: app.agents.a1 });
    await fireEvent.dblClick(screen.getByRole('article'));
    expect(app.workspaceOn).toBe(false);
    expect(app.ui.selectedAgent.p1).toBe('a1');
  });

  it('answers the question waiting, its conversation not opened before', async () => {
    dropConversation('a1');
    let loaded!: (items: unknown[]) => void;
    const question = { kind: 'question', id: 'q1', toolUseId: 't', questions: [{ question: 'Q ?', options: [] }], answers: null, ts: 1 };
    backend = fakeBackend({ get_conversation: () => new Promise((r) => (loaded = r)) });
    resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting', pending: ['q1'] })] });
    // Not loaded yet: the text stays, nothing leaves as a new message.
    const field = await typeAndEnter('Oui');
    expect(backend.called('send_message')).toHaveLength(0);
    expect(field.value).toBe('Oui');
    loaded([question]);
    // The request shown on the card (not only asked for): Enter waits HOLD_MS from then, as in the conversation.
    await vi.waitFor(() => expect(screen.getByText('Q ?')).toBeTruthy());
    await new Promise((r) => setTimeout(r, HOLD_MS + 100));
    await fireEvent.keyDown(field, { key: 'Enter' });
    await vi.waitFor(() =>
      expect(backend.called('answer_question')[0]?.args).toMatchObject({ id: 'a1', requestId: 'q1', answers: { 'Q ?': 'Oui' } }),
    );
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('says why a message is refused for want of an account', async () => {
    backend = fakeBackend({
      get_conversation: () => [],
      send_message: () => {
        throw 'no-account:codex|Aucun compte Codex : ajoute-en un dans Réglages › Comptes et harness.';
      },
    });
    const field = await typeAndEnter('Bonjour');
    await vi.waitFor(() =>
      expect(app.toasts.map((x) => x.text)).toContain('Aucun compte Codex : ajoute-en un dans Réglages › Comptes et harness.'),
    );
    await vi.waitFor(() => expect(field.value).toBe('Bonjour'));
  });

  it("leaves the app's shortcuts to the app while one types", async () => {
    const heard = vi.fn();
    window.addEventListener('keydown', heard);
    const field = await typeAndEnter('');
    await fireEvent.keyDown(field, { key: 'k', metaKey: true });
    window.removeEventListener('keydown', heard);
    expect(heard).toHaveBeenCalledWith(expect.objectContaining({ key: 'k', metaKey: true }));
  });

  it('sends nothing while a file of the draft is being copied', async () => {
    setDraft('a1', { text: '', files: [{ ...image('a.png'), kind: 'file', copying: true }] });
    const field = await typeAndEnter('Bonjour');
    await new Promise((r) => setTimeout(r, 0));
    expect(backend.called('send_message')).toHaveLength(0);
    expect(field.value).toBe('Bonjour');
  });

  it('sends a draft of files alone', async () => {
    setDraft('a1', { text: '', files: [image('a.png')] });
    await typeAndEnter('');
    await vi.waitFor(() => expect(backend.called('send_message')[0]?.args).toMatchObject({ id: 'a1', text: '' }));
  });

  it("says its state as the sidebar's card does, with its spend", () => {
    resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting' })] });
    const { container } = render(AgentCard, { agent: app.agents.a1 });
    expect(screen.getByText('Question')).toBeTruthy();
    expect(screen.getByRole('article', { name: 'un, Question' })).toBeTruthy();
    // Its spaces are the narrow ones of the French figures, which the text queries normalise away.
    expect(container.querySelector('.spend')?.textContent?.trim()).toBe(cardSpend(app.agents.a1));
  });

  it('sends by its « Envoyer » button too', async () => {
    render(AgentCard, { agent: app.agents.a1 });
    await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'Bonjour' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Envoyer' }));
    await vi.waitFor(() => expect(backend.called('send_message')[0]?.args).toMatchObject({ id: 'a1', text: 'Bonjour' }));
  });

  it('grows with its question, between its lines and its field', async () => {
    backend = fakeBackend({ get_conversation: () => [questionItem('q1')] });
    resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] });
    const { container } = render(AgentCard, { agent: app.agents.a1 });
    await screen.findByText('Question q1 ?');
    const order = [...container.querySelectorAll('.a-act, .ask, .bar input')].map((e) => (e.tagName === 'INPUT' ? 'INPUT' : e.className));
    expect(order[0]).toContain('a-act');
    expect(order[1]).toContain('ask');
    expect(order[2]).toBe('INPUT');
  });

  it('says the question of its subagent, and leaves it to its card', async () => {
    backend = fakeBackend({ get_conversation: () => [questionItem('q1', { sub: 's1' })] });
    const plan = planWith([], { agents: [{ ...sub('s1'), kind: 'Explore' }] });
    resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['q1'], requests: [req('q1')], plan })] });
    const { container } = render(AgentCard, { agent: app.agents.a1, shown: ['s1'] });
    expect(await screen.findByText('question de Explore')).toBeTruthy();
    expect(screen.queryByText('Question q1 ?')).toBeNull();
    expect(container.querySelector('.agent')?.classList).toContain('sub-asks');
  });

  it('lets its conversation go once answered', async () => {
    const gone = vi.fn();
    vi.spyOn(conversationOf('a1'), 'show').mockReturnValue(gone);
    resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] });
    const { rerender } = render(AgentCard, { agent: app.agents.a1 });
    expect(gone).not.toHaveBeenCalled();
    await rerender({ agent: { ...app.agents.a1, status: 'running', pending: [], requests: [] } });
    expect(gone).toHaveBeenCalled();
  });

  it('reports its height', async () => {
    const onsize = vi.fn();
    render(AgentCard, { agent: app.agents.a1, onsize });
    await vi.waitFor(() => expect(onsize).toHaveBeenCalled());
  });

  it('shows the request of a subagent that has no card, saying whose it is', async () => {
    backend = fakeBackend({ get_conversation: () => [questionItem('q1', { sub: 's1' })] });
    const plan = planWith([], { agents: [{ ...sub('s1', 'done'), kind: 'Explore' }] });
    resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['q1'], requests: [req('q1')], plan })] });
    render(AgentCard, { agent: app.agents.a1, shown: [] });
    expect(await screen.findByText('Question q1 ?')).toBeTruthy();
    expect(screen.getByText('question de Explore')).toBeTruthy();
  });

  it("sends a message, not an answer, while only a subagent's request on its own card waits", async () => {
    backend = fakeBackend({ get_conversation: () => [permissionItem('p1', 'rm -rf dist', { sub: 's1' })] });
    const plan = planWith([], { agents: [sub('s1')] });
    resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting', pending: ['p1'], requests: [req('p1', 'permission')], plan })] });
    render(AgentCard, { agent: app.agents.a1, shown: ['s1'] });
    await vi.waitFor(() => expect(backend.called('get_conversation').length).toBeGreaterThan(0));
    await new Promise((r) => setTimeout(r, HOLD_MS + 20));
    const field = screen.getByRole('textbox', { name: 'Écrire à un' });
    await fireEvent.input(field, { target: { value: 'Bonjour' } });
    await fireEvent.keyDown(field, { key: 'Enter' });
    await vi.waitFor(() => expect(backend.called('send_message')[0]?.args).toMatchObject({ id: 'a1', text: 'Bonjour' }));
    expect(backend.called('answer_permission')).toHaveLength(0);
  });

  it('does not answer unread a request that comes while one types', async () => {
    backend = fakeBackend({ get_conversation: () => [questionItem('q1')] });
    const { rerender } = render(AgentCard, { agent: app.agents.a1 });
    const field = screen.getByRole('textbox', { name: 'Écrire à un' });
    await fireEvent.input(field, { target: { value: 'Oui' } });
    app.agents.a1 = { ...app.agents.a1, status: 'waiting', pending: ['q1'], requests: [req('q1')] };
    await rerender({ agent: app.agents.a1 });
    await screen.findByText('Question q1 ?');
    await fireEvent.keyDown(field, { key: 'Enter' });
    expect(backend.called('answer_question')).toHaveLength(0);
    expect(backend.called('send_message')).toHaveLength(0);
    expect((field as HTMLInputElement).value).toBe('Oui');
    await new Promise((r) => setTimeout(r, HOLD_MS + 20));
    await fireEvent.keyDown(field, { key: 'Enter' });
    await vi.waitFor(() => expect(backend.called('answer_question')[0]?.args).toMatchObject({ id: 'a1', requestId: 'q1' }));
  });
});
