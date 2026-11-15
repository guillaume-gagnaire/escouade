import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { dropConversation } from '../../lib/conversations.svelte';
import { isWake, overseer, scopeLabel, userWords } from '../../lib/overseer.svelte';
import { app } from '../../lib/state.svelte';
import type { ConvItem, OverseerView } from '../../lib/types';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import OverseerPanel from './OverseerPanel.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const NOW = Date.now();

const ITEMS: ConvItem[] = [
  {
    kind: 'user',
    id: 'u1',
    text: '[Message de l’utilisateur · m-1 · tapé]\nCrée un ticket « Corriger le lien »',
    images: 0,
    ts: NOW - 5000,
    queued: false,
  },
  { kind: 'text', id: 't1', text: 'Ticket DEM-1 créé.', streaming: false },
  { kind: 'user', id: 'u2', text: '[Escouade · événements · 1]\nCe qui suit vient d’Escouade…', images: 0, ts: NOW - 1000, queued: false },
  { kind: 'text', id: 't2', text: 'RAS', streaming: false },
];

const VIEW: OverseerView = {
  enabled: true,
  running: true,
  needsMcp: false,
  agentId: 'ov',
  directives: [],
  messages: [{ id: 'm-1', text: 'Crée un ticket « Corriger le lien »', said: false, at: NOW - 5000 }],
  actions: [
    {
      id: 'a-2',
      at: NOW - 4000,
      tool: 'create_ticket',
      text: 'project: demo · title: Corriger le lien',
      outcome: 'done',
      undo: { what: { kind: 'ticket' }, until: NOW + 60_000 },
      undoable: true,
    },
    {
      id: 'a-3',
      at: NOW - 500,
      tool: 'approve_ticket',
      text: 'ticket: DEM-2',
      outcome: 'refused',
      why: 'Escouade refuse : aucune consigne active de l’utilisateur…',
      undoable: false,
    },
  ],
};

describe('the Overseer’s minimal discussion (ESC-462)', () => {
  beforeEach(() => {
    resetApp();
    dropConversation('ov');
    app.agents = { ov: agent({ id: 'ov', projectId: 'overseer', name: 'Overseer', status: 'idle' }) };
  });

  it('says it is off, with the way to its settings', async () => {
    fakeBackend();
    render(OverseerPanel);
    expect(screen.getByText('L’Overseer est éteint. Allume-le dans Réglages › Overseer.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir les réglages' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'overseer' });
  });

  it('shows the user’s words, its answers, and under each message what it did, « Annuler » while it can', async () => {
    const b = fakeBackend({ get_conversation: () => ITEMS });
    overseer.take(VIEW);
    render(OverseerPanel);
    const thread = await waitFor(() => {
      const el = document.querySelector('[data-overseer-thread]') as HTMLElement;
      expect(el).toHaveTextContent('Ticket DEM-1 créé.');
      return el;
    });
    const user = within(thread).getByText('Crée un ticket « Corriger le lien »');
    expect(user.closest('[data-user-message]')).toHaveTextContent('tapé');
    // Its header is the backend's, not the user's words.
    expect(thread).not.toHaveTextContent('[Message de l’utilisateur');
    // The wake-up, folded to its first line.
    expect(thread).toHaveTextContent('[Escouade · événements · 1]');
    expect(thread).not.toHaveTextContent('Ce qui suit vient');
    // The ticket made, before the wake-up; the refusal after it.
    const acts = [...thread.querySelectorAll('[data-action]')].map((a) => a.getAttribute('data-action'));
    expect(acts).toEqual(['create_ticket', 'approve_ticket']);
    const made = thread.querySelector('[data-action="create_ticket"]') as HTMLElement;
    const text = thread.textContent ?? '';
    expect(text.indexOf('Ticket DEM-1 créé.')).toBeLessThan(text.indexOf('project: demo'));
    expect(text.indexOf('project: demo')).toBeLessThan(text.indexOf('[Escouade · événements'));
    const refused = thread.querySelector('[data-action="approve_ticket"]') as HTMLElement;
    expect(refused).toHaveTextContent('refusé');
    expect(refused).toHaveTextContent('aucune consigne active');
    expect(within(refused).queryByRole('button', { name: 'Annuler' })).toBeNull();
    await userEvent.click(within(made).getByRole('button', { name: 'Annuler' }));
    expect(b.called('overseer_undo')[0].args).toEqual({ id: 'a-2' });
  });

  it('sends what is typed on Enter, and unfolds the instructions', async () => {
    const b = fakeBackend({ get_conversation: () => [], overseer_send: () => 'm-2' });
    overseer.take({ ...VIEW, actions: [], messages: [] });
    render(OverseerPanel);
    expect(screen.getByText(/Demande-lui ce que tu veux/)).toBeInTheDocument();
    const field = screen.getByRole('textbox', { name: 'Demande à l’Overseer…' });
    await userEvent.type(field, 'Tu peux merger sur demo{Enter}');
    expect(b.called('overseer_send')[0].args).toEqual({ text: 'Tu peux merger sur demo', said: false });
    await waitFor(() => expect(field).toHaveValue(''));
    await userEvent.click(screen.getByRole('button', { name: 'Consignes · 0' }));
    expect(screen.getByText('Aucune consigne : il te demande avant chaque action qui sort.')).toBeInTheDocument();
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
