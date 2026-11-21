import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { conversation } from '../lib/conversation.svelte';
import { setLang } from '../lib/i18n';
import { draftIfAny } from '../lib/question-answers.svelte';
import { app } from '../lib/state.svelte';
import type { QuestionItem, VoiceEvent } from '../lib/types';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import ConversationOverlay from './ConversationOverlay.svelte';

const SESSION = 5;
const ev = (e: Record<string, unknown>) => conversation.take({ session: SESSION, ...e } as VoiceEvent);
const overlay = () => document.querySelector('[data-conversation-overlay]') as HTMLElement;
const tick = () => new Promise((r) => setTimeout(r, 0));

describe('the voice conversation over the whole window', () => {
  let backend: ReturnType<typeof fakeBackend>;
  let reading = 0;

  beforeEach(async () => {
    setLang('fr');
    reading = 0;
    resetApp({ projects: [project({ id: 'p1', name: 'web-shop' })], agents: [agent({ id: 'a1', name: 'Alice', projectId: 'p1' })] });
    conversation.clear();
    backend = fakeBackend({
      conversation_start: () => ({ kind: 'listening', session: SESSION }),
      conversation_say: () => ++reading,
      get_conversation: () => [],
    });
    await conversation.start('a1');
  });

  it('names the agent, says the state, and shows what the user said, then each sentence read as it is heard', async () => {
    render(ConversationOverlay);
    expect(screen.getByRole('dialog', { name: 'Conversation vocale avec Alice' })).toBeTruthy();
    expect(overlay().textContent).toContain('web-shop');
    expect(overlay().dataset.conversationPhase).toBe('listening');
    expect(screen.getByRole('status').textContent).toContain('À l’écoute : parle quand tu veux');
    // Whisper's text, once transcribed: the user's subtitle.
    ev({ kind: 'convHeard', text: 'Ajoute un test pour la page des réglages.', error: null });
    await tick();
    const sub = () => document.querySelector('[data-subtitle]') as HTMLElement;
    await waitFor(() => expect(sub().dataset.subtitle).toBe('user'));
    expect(sub().textContent).toContain('Toi');
    expect(sub().textContent).toContain('Ajoute un test pour la page des réglages.');
    expect(screen.getByRole('status').textContent).toContain('Alice travaille…');
    // The agent's passage, sentence by sentence as it is heard.
    conversation.observe(agent({ id: 'a1', name: 'Alice' }), [
      { kind: 'text', id: 'm', text: '…', spoken: 'C’est fait. Les tests passent.', streaming: false },
      { kind: 'turn', id: 't', ts: 2, durationMs: 1, cost: 0, tokens: 0, isError: false, interrupted: false, error: null },
    ]);
    await tick();
    ev({ kind: 'convSentence', reading: 1, index: 0, text: 'C’est fait.' });
    await waitFor(() => expect(sub().dataset.subtitle).toBe('agent'));
    expect(sub().textContent).toContain('Alice');
    expect(sub().textContent).toContain('C’est fait.');
    ev({ kind: 'convSentence', reading: 1, index: 1, text: 'Les tests passent.' });
    await waitFor(() => expect(sub().textContent).toContain('Les tests passent.'));
    expect(sub().textContent).not.toContain('C’est fait.');
    expect(screen.getByRole('status').textContent).toContain('Alice te répond');
    // The sentences of a reading cut off (barge-in) are not shown.
    ev({ kind: 'convBargeIn', reading: 1 });
    ev({ kind: 'convSentence', reading: 1, index: 2, text: 'Trop tard.' });
    await tick();
    expect(sub().textContent).not.toContain('Trop tard.');
  });

  it('says who transcribes the sentence: the machine, or the service of « Transcription : En ligne » (ESC-134)', async () => {
    render(ConversationOverlay);
    ev({ kind: 'convTranscribing' });
    await waitFor(() => expect(screen.getByRole('status').textContent).toContain('Transcription sur ta machine…'));
    app.settings.voice = { ...app.settings.voice, transcription: 'online', onlineService: 'elevenlabs' };
    await waitFor(() => expect(screen.getByRole('status').textContent).toContain('Transcription par ElevenLabs…'));
  });

  it('writes what the user says as they say it, its end kept, then the final text in its place (ESC-136)', async () => {
    render(ConversationOverlay);
    const sub = () => document.querySelector('[data-subtitle]') as HTMLElement;
    // Not while it waits for a sentence: a late text of the one before is not shown.
    ev({ kind: 'convPartial', text: 'en retard' });
    await tick();
    expect(sub()).toBeNull();
    ev({ kind: 'convSpeech' });
    ev({ kind: 'convPartial', text: 'Ajoute un' });
    await waitFor(() => expect(sub().textContent).toContain('Ajoute un'));
    expect(sub().dataset.subtitle).toBe('user');
    expect(sub().hasAttribute('data-live')).toBe(true);
    const long =
      'Ajoute un test pour la page des réglages, puis lance la suite, puis commite, puis pousse la branche, puis ouvre la demande de fusion';
    ev({ kind: 'convPartial', text: long });
    await waitFor(() => expect(sub().textContent).toContain('ouvre la demande de fusion'));
    expect(sub().textContent).toContain('…');
    expect(sub().textContent).not.toContain('Ajoute un test');
    ev({ kind: 'convTranscribing' });
    ev({ kind: 'convHeard', text: 'Ajoute un test pour la page des réglages.', error: null });
    await waitFor(() => expect(sub().hasAttribute('data-live')).toBe(false));
    expect(sub().textContent).toContain('Ajoute un test pour la page des réglages.');
  });

  it('follows the level of the voice played and of the microphone', () => {
    ev({ kind: 'convLevel', level: 0.4, played: 0.8 });
    expect(conversation.level).toBe(0.4);
    expect(conversation.played).toBe(0.8);
  });

  it('Escape ends the conversation and the panel goes', async () => {
    const { unmount } = render(ConversationOverlay);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(conversation.phase).toBe('idle');
    expect(backend.called('conversation_stop')).toHaveLength(1);
    expect(conversation.overlay).toBe(false);
    unmount();
  });

  it('« Arrêter » ends it too, and « Reprendre » shows on pause', async () => {
    render(ConversationOverlay);
    conversation.phase = 'paused';
    await tick();
    expect(screen.getByRole('button', { name: 'Reprendre' })).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: /Arrêter/ }));
    expect(backend.called('conversation_stop')).toHaveLength(1);
  });

  it('draws only while it is shown: its animation frame is cancelled when it goes', () => {
    const raf = vi.spyOn(window, 'requestAnimationFrame');
    const caf = vi.spyOn(window, 'cancelAnimationFrame');
    const getContext = vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
      setTransform() {},
      clearRect() {},
      createRadialGradient: () => ({ addColorStop() {} }),
      beginPath() {},
      arc() {},
      fill() {},
      moveTo() {},
      lineTo() {},
      stroke() {},
    } as unknown as CanvasRenderingContext2D);
    const { unmount } = render(ConversationOverlay);
    expect(raf).toHaveBeenCalled();
    unmount();
    expect(caf).toHaveBeenCalled();
    getContext.mockRestore();
    raf.mockRestore();
    caf.mockRestore();
  });

  describe('a request of questions asked aloud (ESC-131)', () => {
    const request: QuestionItem = {
      kind: 'question',
      id: 'rq',
      toolUseId: 'tu',
      ts: 3,
      answers: null,
      questions: [
        {
          question: 'Quelle base ?',
          header: 'Base',
          options: [
            { label: 'PostgreSQL', description: 'Relationnelle' },
            { label: 'SQLite', description: 'Embarquée' },
          ],
        },
        {
          question: 'Quels tests ?',
          header: 'Tests',
          multiSelect: true,
          options: [{ label: 'Unitaires' }, { label: 'E2E' }, { label: 'Lint' }],
        },
        { question: 'Quel nom ?', header: 'Nom', options: [{ label: 'web-shop' }, { label: 'admin' }] },
      ],
    };
    const later = (ms = 5) => new Promise((r) => setTimeout(r, ms));
    const choice = (name: string) => screen.getByRole('button', { name: new RegExp(name) });
    const asking = () => document.querySelector('[data-conversation-ask]') as HTMLElement | null;

    async function ask() {
      app.agents.a1 = { ...app.agents.a1, pending: ['rq'] };
      ev({ kind: 'convHeard', text: 'Configure le projet', error: null });
      await tick();
      conversation.observe(app.agents.a1, [request]);
      await tick();
    }

    async function heard(text: string) {
      ev({ kind: 'convSpoken', reading, error: null });
      ev({ kind: 'convHeard', text, error: null });
      await tick();
    }

    it('shows the question under the orb with its choices, their ranks and descriptions, where it is, and « Autre »', async () => {
      render(ConversationOverlay);
      await ask();
      expect(overlay().classList.contains('asking')).toBe(true);
      const section = screen.getByRole('region', { name: 'Choix de la question' });
      expect(section.querySelector('[data-step]')?.textContent).toBe('Question 1 sur 3');
      expect(section.textContent).toContain('Quelle base ?');
      const items = [...section.querySelectorAll('li')].map((li) => li.textContent?.replace(/\s+/g, ' ').trim());
      expect(items).toEqual(['1 PostgreSQL Relationnelle', '2 SQLite Embarquée']);
      expect(choice('PostgreSQL').getAttribute('aria-keyshortcuts')).toBe('1');
      expect(section.textContent).toContain('Autre : dis ta réponse');
      // The voice says the question, not its choices.
      expect(backend.called('conversation_say').at(-1)?.args.text).toBe('Question 1 sur 3. Quelle base ?');
    });

    it('lights the choice understood a moment, then shows the next question', async () => {
      conversation.flashMs = 40;
      render(ConversationOverlay);
      await ask();
      // The clock in the test's hand: a loaded machine outlasts the 40 ms of the light between two lines of the test.
      vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
      try {
        ev({ kind: 'convSpoken', reading, error: null });
        ev({ kind: 'convHeard', text: 'La deuxième', error: null });
        await vi.advanceTimersByTimeAsync(0);
        expect(choice('SQLite').classList.contains('lit')).toBe(true);
        expect(choice('PostgreSQL').classList.contains('lit')).toBe(false);
        await vi.advanceTimersByTimeAsync(39);
        expect(choice('SQLite').classList.contains('lit')).toBe(true);
        await vi.advanceTimersByTimeAsync(1);
        expect(asking()?.dataset.conversationAsk).toBe('1');
        expect(screen.getByRole('region').textContent).toContain('Plusieurs réponses possibles');
      } finally {
        vi.useRealTimers();
      }
    });

    it('says on screen too an answer not understood, and shows a free answer understood', async () => {
      conversation.flashMs = 0;
      render(ConversationOverlay);
      await ask();
      await heard('SQLite ou PostgreSQL');
      await tick();
      expect(screen.getByRole('alert').textContent).toBe('Je n’ai pas compris : redis ta réponse, ou choisis ci-dessous.');
      // Each answer once the question it answers is asked, not after a delay that a loaded machine outlasts.
      await heard('SQLite');
      await waitFor(() => expect(asking()?.dataset.conversationAsk).toBe('1'));
      await heard('Toutes');
      await waitFor(() => expect(asking()?.dataset.conversationAsk).toBe('2'));
      // The last one lit until it is read: a flash shorter than the wait would put it away first and send the request.
      conversation.flashMs = 60_000;
      await heard('Escouade-front');
      await tick();
      expect(document.querySelector('[data-understood]')?.textContent).toBe('Ta réponse : Escouade-front');
    });

    it('a click or the key of its rank chooses; several choices are ticked, then « Valider » or Enter sends them', async () => {
      conversation.flashMs = 0;
      render(ConversationOverlay);
      await ask();
      await userEvent.click(choice('SQLite'));
      await later();
      expect(draftIfAny('rq')?.picks['Quelle base ?']).toEqual(['SQLite']);
      expect(asking()?.dataset.conversationAsk).toBe('1');
      await userEvent.click(choice('Unitaires'));
      await fireEvent.keyDown(window, { key: '3' });
      await tick();
      expect(choice('Unitaires').getAttribute('aria-pressed')).toBe('true');
      expect(choice('Lint').getAttribute('aria-pressed')).toBe('true');
      expect(choice('E2E').getAttribute('aria-pressed')).toBe('false');
      expect(asking()?.dataset.conversationAsk).toBe('1');
      await userEvent.click(screen.getByRole('button', { name: /Valider/ }));
      await later();
      expect(asking()?.dataset.conversationAsk).toBe('2');
      await fireEvent.keyDown(window, { key: '2' });
      await later();
      expect(backend.called('answer_question')[0].args.answers).toEqual({
        'Quelle base ?': 'SQLite',
        'Quels tests ?': 'Unitaires, Lint',
        'Quel nom ?': 'admin',
      });
      expect(asking()).toBeNull();
    });

    it('Enter confirms the choices ticked of a question of several', async () => {
      conversation.flashMs = 0;
      render(ConversationOverlay);
      await ask();
      await fireEvent.keyDown(window, { key: '1' });
      await later();
      await fireEvent.keyDown(window, { key: '2' });
      await fireEvent.keyDown(window, { key: 'Enter' });
      await later();
      expect(draftIfAny('rq')?.picks['Quels tests ?']).toEqual(['E2E']);
      expect(asking()?.dataset.conversationAsk).toBe('2');
    });
  });
});
