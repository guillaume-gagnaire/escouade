import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { dropConversation } from '../../lib/conversations.svelte';
import { setLocks } from '../../lib/locks.svelte';
import { isWake, overseer, scopeLabel, userWords } from '../../lib/overseer.svelte';
import { overseerVoice } from '../../lib/overseer-voice.svelte';
import { plugins } from '../../lib/plugins-view.svelte';
import { app } from '../../lib/state.svelte';
import type { ConvItem, OverseerView, VoiceEvent } from '../../lib/types';
import { agent, fakeBackend, project, resetApp, ticket } from '../../test/ipc';
import OverseerScreen from './OverseerScreen.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const NOW = Date.now();

const ITEMS: ConvItem[] = [
  {
    kind: 'user',
    id: 'u1',
    text: '[Message de l’utilisateur · m-1 · dit]\nLance les relances sur le portail\n\n<escouade-voice>Termine par un passage à lire.</escouade-voice>',
    images: 0,
    ts: NOW - 60_000,
    queued: false,
  },
  { kind: 'text', id: 't1', text: 'C’est parti.', streaming: false },
  {
    kind: 'user',
    id: 'u2',
    text: '[Escouade · événements · 1]\nCe qui suit vient d’Escouade…',
    images: 0,
    ts: NOW - 20_000,
    queued: false,
  },
  { kind: 'text', id: 't2', text: 'RAS', streaming: false },
  {
    kind: 'user',
    id: 'u3',
    text: '[Message de l’utilisateur · m-2 · tapé]\nvalide, et pour WS-12 c’est 30 jours',
    images: 0,
    ts: NOW - 5_000,
    queued: false,
  },
];

const VIEW: OverseerView = {
  enabled: true,
  running: true,
  needsMcp: false,
  agentId: 'ov',
  directives: [],
  messages: [
    { id: 'm-1', text: 'Lance les relances sur le portail', said: true, at: NOW - 60_000 },
    { id: 'm-2', text: 'valide, et pour WS-12 c’est 30 jours', said: false, at: NOW - 5_000 },
  ],
  actions: [
    {
      id: 'a-2',
      at: NOW - 59_000,
      tool: 'create_agent',
      text: 'DEM-41 lancé · portail',
      outcome: 'done',
      undo: { what: { kind: 'agent' }, until: NOW + 60_000 },
      undoable: true,
    },
    {
      id: 'a-3',
      at: NOW - 58_000,
      tool: 'add_directive',
      text: 'Consigne ajoutée : « Release 2.0.6 : tag et publication »',
      outcome: 'done',
      undo: { what: { kind: 'directive' }, until: NOW + 60_000 },
      undoable: true,
    },
    {
      id: 'a-4',
      at: NOW - 4_000,
      tool: 'add_directive',
      text: 'merge · DEM-31',
      outcome: 'refused',
      why: 'Escouade refuse : dans le message de l’utilisateur, « fusionner » est près d’une négation…',
      undoable: false,
    },
  ],
  news: [
    {
      id: 'n-1',
      at: NOW - 30_000,
      kind: 'review',
      ticketId: 'dem31',
      ticket: 'DEM-31',
      title: 'Paiement en 4 fois',
      projectId: 'p1',
      project: 'demo-api',
    },
  ],
};

function setUp(handlers: Record<string, (args: any) => unknown> = {}) {
  const b = fakeBackend({ get_conversation: () => ITEMS, speech_info: () => ({ available: true, voices: [] }), ...handlers });
  overseer.take(VIEW);
  render(OverseerScreen);
  return b;
}

const thread = () =>
  waitFor(() => {
    const el = document.querySelector('[data-overseer-thread]') as HTMLElement;
    expect(el).toHaveTextContent('C’est parti.');
    return el;
  });

describe('the Overseer’s screen (ESC-468)', () => {
  beforeEach(() => {
    resetApp({
      projects: [project()],
      tickets: [ticket({ id: 'dem31', key: 'DEM-31', title: 'Paiement en 4 fois', column: 'review', agentId: 'w31' })],
      agents: [
        agent({ id: 'w31', projectId: 'p1', ticketId: 'dem31', worktree: { path: 'C:\\w', branch: 'ticket/dem-31', baseBranch: 'main' } }),
      ],
    });
    dropConversation('ov');
    app.agents = { ...app.agents, ov: agent({ id: 'ov', projectId: 'overseer', name: 'Overseer', status: 'idle' }) };
    setLocks(undefined);
  });

  it('says it is off, with the way to its settings', async () => {
    fakeBackend();
    render(OverseerScreen);
    expect(screen.getByText('L’Overseer est éteint. Allume-le dans Réglages › Overseer.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir les réglages' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'overseer' });
  });

  it('shows the user’s messages said or typed with their time, its answers, and its wake-ups left out', async () => {
    setUp();
    const el = await thread();
    const said = within(el).getByText('Lance les relances sur le portail').closest('[data-user-message]') as HTMLElement;
    expect(said).toHaveTextContent('dit ·');
    // The voice's instruction is the backend's, not the user's words.
    expect(said).not.toHaveTextContent('escouade-voice');
    const typed = within(el).getByText('valide, et pour WS-12 c’est 30 jours').closest('[data-user-message]') as HTMLElement;
    expect(typed).toHaveTextContent('tapé ·');
    expect(el).not.toHaveTextContent('[Escouade · événements');
    expect(el).not.toHaveTextContent('RAS');
  });

  it('shows what it did under its answer, with « Annuler », « Retirer » for an instruction, and a refusal’s reason', async () => {
    const b = setUp();
    const el = await thread();
    const launched = el.querySelector('[data-action="create_agent"]') as HTMLElement;
    const text = el.textContent ?? '';
    expect(text.indexOf('C’est parti.')).toBeLessThan(text.indexOf('DEM-41 lancé'));
    await userEvent.click(within(launched).getByRole('button', { name: 'Annuler' }));
    expect(b.called('overseer_undo')[0].args).toEqual({ id: 'a-2' });
    const directive = el.querySelectorAll('[data-action="add_directive"]')[0] as HTMLElement;
    await userEvent.click(within(directive).getByRole('button', { name: 'Retirer' }));
    expect(b.called('overseer_undo')[1].args).toEqual({ id: 'a-3' });
    const refused = el.querySelector('[data-outcome="refused"]') as HTMLElement;
    expect(refused).toHaveTextContent('près d’une négation');
    expect(within(refused).queryByRole('button')).toBeNull();
  });

  it('shows the news as cards among the messages, with ready answers', async () => {
    const b = setUp({ overseer_send: () => 'm-3' });
    const el = await thread();
    const card = el.querySelector('[data-news="review"]') as HTMLElement;
    expect(card).toHaveTextContent('Terminé ·');
    expect(card).toHaveTextContent('DEM-31 · Paiement en 4 fois');
    // Between the message before it and the one after it.
    const text = el.textContent ?? '';
    expect(text.indexOf('C’est parti.')).toBeLessThan(text.indexOf('DEM-31 · Paiement'));
    expect(text.indexOf('DEM-31 · Paiement')).toBeLessThan(text.indexOf('valide, et pour WS-12'));
    await userEvent.click(within(card).getByRole('button', { name: 'Valide-le' }));
    expect(b.called('overseer_send')[0].args).toEqual({ text: 'Valide DEM-31.', said: false });
    await userEvent.click(within(card).getByRole('button', { name: 'Renvoie-le' }));
    expect(b.called('overseer_send')[1].args.text).toContain('DEM-31');
    overseer.open = true;
    await userEvent.click(within(card).getByRole('button', { name: 'Montre-moi le diff' }));
    expect(app.modal).toMatchObject({ kind: 'diff', agentId: 'w31', projectId: 'p1' });
    expect(overseer.open).toBe(false);
  });

  it('sends what is typed on Enter, and unfolds the instructions', async () => {
    const b = setUp({ overseer_send: () => 'm-3' });
    const field = screen.getByRole('textbox', { name: 'Demande à l’Overseer…' });
    await userEvent.type(field, 'Tu peux merger sur demo{Enter}');
    expect(b.called('overseer_send')[0].args).toEqual({ text: 'Tu peux merger sur demo', said: false });
    await waitFor(() => expect(field).toHaveValue(''));
    await userEvent.click(screen.getByRole('button', { name: 'Consignes · 0' }));
    expect(screen.getByText('Aucune consigne : il te demande avant chaque action qui sort.')).toBeInTheDocument();
  });

  it('speaks while Space is held in the empty field: the sentence written live, sent when let go', async () => {
    const b = setUp({ overseer_voice_start: () => ({ kind: 'listening', session: 7, notice: null }) });
    const field = screen.getByRole('textbox', { name: 'Demande à l’Overseer…' });
    field.focus();
    await fireEvent.keyDown(field, { key: ' ', code: 'Space' });
    await waitFor(() => expect(overseerVoice.phase).toBe('talking'));
    expect(b.called('overseer_voice_start')).toHaveLength(1);
    // The field is a wave now; what is said is written as it comes, under the orb and in the thread.
    expect(document.querySelector('[data-wave]')).toBeInTheDocument();
    overseerVoice.take({ kind: 'convPartial', session: 7, text: 'Quand la CI est verte' } as VoiceEvent);
    await waitFor(() => expect(document.querySelector('[data-user-live]')).toHaveTextContent('Quand la CI est verte'));
    expect(document.querySelector('[data-orb-caption]')).toHaveTextContent('Quand la CI est verte');
    expect(screen.getByText(/pour envoyer/)).toBeInTheDocument();
    // Held long enough, then let go: sent at once.
    (overseerVoice as unknown as { pressedAt: number }).pressedAt = performance.now() - 1000;
    await fireEvent.keyUp(window, { key: ' ', code: 'Space' });
    await waitFor(() => expect(b.called('conversation_end')[0]?.args).toEqual({ session: 7, send: true }));
    overseerVoice.take({ kind: 'convHeard', session: 7, text: 'Quand la CI est verte, pose le tag', error: null } as VoiceEvent);
    await waitFor(() => expect(b.called('conversation_send')[0]?.args).toEqual({ id: 'ov', text: 'Quand la CI est verte, pose le tag' }));
  });

  it('cancels with Escape while speaking, then Escape closes the screen', async () => {
    const b = setUp({ overseer_voice_start: () => ({ kind: 'listening', session: 3, notice: null }) });
    overseer.open = true;
    await userEvent.click(screen.getByRole('button', { name: 'Parler à l’Overseer' }));
    await waitFor(() => expect(overseerVoice.phase).toBe('talking'));
    await userEvent.keyboard('{Escape}');
    expect(b.called('conversation_end').at(-1)?.args).toEqual({ session: 3, send: false });
    expect(b.called('conversation_send')).toHaveLength(0);
    expect(overseer.open).toBe(true);
    await userEvent.keyboard('{Escape}');
    expect(overseer.open).toBe(false);
  });

  it('is interrupted by typing while its answer is read', async () => {
    const b = setUp({ overseer_voice_start: () => ({ kind: 'listening', session: 4, notice: null }) });
    await userEvent.click(screen.getByRole('button', { name: 'Parler à l’Overseer' }));
    await waitFor(() => expect(overseerVoice.phase).toBe('talking'));
    overseerVoice.phase = 'speaking';
    await waitFor(() => expect(screen.getByText('Parle ou tape pour l’interrompre')).toBeInTheDocument());
    await userEvent.type(screen.getByRole('textbox', { name: 'Demande à l’Overseer…' }), 'a');
    expect(overseerVoice.phase).toBe('idle');
    expect(b.called('conversation_end').at(-1)?.args).toEqual({ session: 4, send: false });
  });

  it('greys the microphone and says so when this device’s policy cuts the dictation', async () => {
    setLocks({
      locks: { 'voice.dictation': { key: 'voice.dictation', value: false, by: { kind: 'device', source: 'HKLM' }, bounds: [] } },
      rows: {},
      warnings: [],
    });
    const b = setUp();
    expect(screen.getByRole('button', { name: 'Parler à l’Overseer' })).toBeDisabled();
    expect(document.querySelector('[data-voice-cut]')).toHaveTextContent('La dictée est coupée par la stratégie de ce poste');
    const field = screen.getByRole('textbox', { name: 'Demande à l’Overseer…' });
    field.focus();
    await fireEvent.keyDown(field, { key: ' ', code: 'Space' });
    expect(b.called('overseer_voice_start')).toHaveLength(0);
  });

  it('without a voice here, the field alone and the way to Plugins', async () => {
    // No voice of its own here (an Intel Mac), and no plugin that brings a service.
    plugins.loaded = true;
    plugins.list = [];
    fakeBackend({ get_conversation: () => ITEMS, speech_info: () => ({ available: false, voices: [] }) });
    overseer.take(VIEW);
    render(OverseerScreen);
    await waitFor(() => expect(document.querySelector('[data-no-voice]')).toBeInTheDocument());
    expect(screen.queryByRole('button', { name: 'Parler à l’Overseer' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir Plugins' }));
    expect(app.ui.view).toBe('plugins');
    plugins.loaded = false;
  });

  it('opens a card’s agent from the orbit', async () => {
    setUp();
    overseer.open = true;
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir DEM-31' }));
    expect(app.ui.selectedAgent.p1).toBe('w31');
    expect(overseer.open).toBe(false);
  });
});

describe('what the window reads of its messages', () => {
  it('takes the backend’s header off the user’s words, and tells a wake-up', () => {
    expect(userWords('[Message de l’utilisateur · m-3 · dit]\nsalut\nça va')).toEqual({ id: 'm-3', text: 'salut\nça va' });
    expect(userWords('[User’s message · m-4 · typed]\nhi')).toEqual({ id: 'm-4', text: 'hi' });
    expect(userWords('Message de agent-1 : salut')).toBeNull();
    expect(isWake('[Escouade · events · 2]\n- …')).toBe(true);
    expect(isWake('salut')).toBe(false);
  });

  it('says where an instruction holds', () => {
    const base = {
      id: 'c-1',
      summary: 's',
      actions: ['merge' as const],
      source: { message: 'm-1', quote: 'q', at: 0 },
      createdAt: 0,
      ticket: null,
      message: null,
    };
    expect(scopeLabel({ ...base, scope: {}, project: null })).toBe('tous les projets');
    expect(scopeLabel({ ...base, scope: { once: true, release: 'v2.3.0' }, project: 'portail', ticket: 'DEM-3' })).toBe(
      'projet portail · ticket DEM-3 · release v2.3.0 · une fois',
    );
  });
});
