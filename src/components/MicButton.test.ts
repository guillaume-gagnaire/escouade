import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HOLD_MS } from '../lib/dictation';
import { dictation } from '../lib/dictation.svelte';
import { fileOrigins } from '../lib/file-origin';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../test/ipc';
import TicketForm from './board/TicketForm.svelte';
import Composer from './Composer.svelte';

vi.mock('../lib/file-origin', () => ({ fileOrigins: vi.fn() }));

const mic = () => screen.getByRole('button', { name: /^(Dicter|Finir la dictée|Dictate|End dictation)$/ });
const panel = () => document.querySelector('[data-voice-phase]');

let now = 1000;
beforeEach(() => {
  resetApp();
  setLang('fr');
  now = 1000;
  vi.spyOn(performance, 'now').mockImplementation(() => now);
  vi.mocked(fileOrigins).mockImplementation(async (files) => files.map(() => null));
});
afterEach(() => vi.restoreAllMocks());

function composer(text = 'Ajoute un test') {
  const a = agent({ id: `m${Math.random()}` });
  resetApp({ agents: [a] });
  const backend = fakeBackend({
    get_conversation: () => [],
    voice_start: () => ({ kind: 'listening', session: 4 }),
    voice_stop: () => text,
  });
  render(Composer, { agent: app.agents[a.id] });
  return { backend, a, textarea: screen.getByRole('textbox') as HTMLTextAreaElement };
}

/** A click on the button: pressed, then let go at once. */
async function click(el: HTMLElement) {
  await fireEvent.pointerDown(el, { button: 0, pointerId: 1 });
  now += 80;
  await fireEvent.pointerUp(el, { button: 0, pointerId: 1 });
}

describe('MicButton in the composer', () => {
  it('says what it does and its shortcut', () => {
    composer();
    expect(mic()).toHaveAttribute('aria-pressed', 'false');
    expect(mic()).toHaveAttribute('aria-keyshortcuts', 'Control+Shift+Space');
  });

  it('shows the dictation in the field itself, above the text, not in a panel over the microphone (2.0 design)', async () => {
    composer();
    await click(mic());
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    const band = panel()!;
    expect(band).toHaveClass('dict');
    expect(band.closest('.composer')).not.toBeNull();
    expect(band.compareDocumentPosition(screen.getByRole('textbox')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(mic().closest('.mic-wrap')!.querySelector('.panel')).toBeNull();
    // Listening: the microphone in the project's colour.
    expect(mic()).toHaveClass('on');
  });

  it('says while it listens that a second click on the microphone writes what was said, in French and in English (ESC-664)', async () => {
    composer();
    await click(mic());
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    expect(panel()).toHaveTextContent('Reclique sur le micro ou Ctrl+Maj+Espace pour écrire ce que tu as dit · Échap pour annuler');
    dictation.cancel();
    cleanup();
    setLang('en');
    composer();
    await click(mic());
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    expect(panel()).toHaveTextContent('Click the mic again or Ctrl+Shift+Space to type what you said · Esc to cancel');
  });

  it('a click listens, shows the level and the time, a second puts the text at the cursor, unsent', async () => {
    const { backend, textarea } = composer('les tests');
    await userEvent.type(textarea, 'Lance  puis commite');
    textarea.setSelectionRange(6, 6);
    await click(mic());
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    expect(mic()).toHaveAccessibleName('Finir la dictée');
    dictation.take({ kind: 'level', session: 4, level: 0.42, ms: 3200 });
    await waitFor(() => expect(screen.getByRole('meter', { name: 'Niveau du micro' })).toHaveAttribute('aria-valuenow', '42'));
    expect(panel()).toHaveTextContent('À l’écoute');
    expect(panel()).toHaveTextContent('0:03');
    expect(panel()).toHaveTextContent('Reclique sur le micro ou Ctrl+Maj+Espace pour écrire ce que tu as dit');
    await click(mic());
    await waitFor(() => expect(textarea.value).toBe('Lance les tests puis commite'));
    expect(textarea.selectionStart).toBe(15);
    expect(panel()).toBeNull();
    expect(backend.called('voice_stop')[0].args).toEqual({ session: 4, target: 'composer' });
    expect(backend.called('send_message')).toHaveLength(0);
    // Changed by hand, then sent by hand.
    await userEvent.type(textarea, ' vite', { skipClick: true });
    await userEvent.keyboard('{Enter}');
    expect(backend.called('send_message')[0].args.text).toBe('Lance les tests vite puis commite');
  });

  it('on line as one speaks, writes the text in its panel, then puts the final one in the field (ESC-136)', async () => {
    let give!: (t: string) => void;
    const a = agent({ id: `m${Math.random()}` });
    resetApp({ agents: [a] });
    fakeBackend({
      get_conversation: () => [],
      voice_start: () => ({ kind: 'listening', session: 6 }),
      voice_stop: () => new Promise((r) => (give = r)),
    });
    render(Composer, { agent: app.agents[a.id] });
    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
    await click(mic());
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    const partial = () => document.querySelector('[data-voice-partial]');
    expect(partial()).toBeNull();
    // Another dictation's text is not this one's.
    dictation.take({ kind: 'partial', session: 5, text: 'pas celle-ci' });
    dictation.take({ kind: 'partial', session: 6, text: 'Ajoute un test' });
    await waitFor(() => expect(partial()).toHaveTextContent('Ajoute un test'));
    dictation.take({ kind: 'partial', session: 6, text: 'Ajoute un test pour la page' });
    await waitFor(() => expect(partial()).toHaveTextContent('Ajoute un test pour la page'));
    expect(textarea.value).toBe('');
    await click(mic());
    // Still shown while the final text comes, then that one in the field, the panel gone.
    await waitFor(() => expect(panel()).toHaveAttribute('data-voice-phase', 'transcribing'));
    expect(partial()).toHaveTextContent('Ajoute un test pour la page');
    give('Ajoute un test pour la page des réglages.');
    await waitFor(() => expect(textarea.value).toBe('Ajoute un test pour la page des réglages.'));
    expect(panel()).toBeNull();
    expect(dictation.partial).toBe('');
  });

  it('held down, it listens until it is let go', async () => {
    const { textarea } = composer('Bonjour');
    await fireEvent.pointerDown(mic(), { button: 0, pointerId: 1 });
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    now += HOLD_MS + 500;
    await fireEvent.pointerUp(mic(), { button: 0, pointerId: 1 });
    await waitFor(() => expect(textarea.value).toBe('Bonjour'));
  });

  it('while it transcribes, says so', async () => {
    let give!: (t: string) => void;
    const a = agent({ id: `m${Math.random()}` });
    resetApp({ agents: [a] });
    fakeBackend({
      get_conversation: () => [],
      voice_start: () => ({ kind: 'listening', session: 1 }),
      voice_stop: () => new Promise((r) => (give = r)),
    });
    render(Composer, { agent: app.agents[a.id] });
    await click(mic());
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    await click(mic());
    await waitFor(() => expect(panel()).toHaveTextContent('Transcription sur ta machine'));
    give('fin');
    await waitFor(() => expect(panel()).toBeNull());
  });

  it('on line, says which service transcribes', async () => {
    let give!: (t: string) => void;
    const a = agent({ id: `m${Math.random()}` });
    resetApp({ agents: [a] });
    app.settings.voice = { ...app.settings.voice, transcription: 'online', onlineService: 'elevenlabs' };
    fakeBackend({
      get_conversation: () => [],
      voice_start: () => ({ kind: 'listening', session: 1 }),
      voice_stop: () => new Promise((r) => (give = r)),
    });
    render(Composer, { agent: app.agents[a.id] });
    await click(mic());
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    await click(mic());
    await waitFor(() => expect(panel()).toHaveTextContent('Transcription par ElevenLabs…'));
    give('fin');
    await waitFor(() => expect(panel()).toBeNull());
  });

  it('the first time, offers the model with its size, and shows its download', async () => {
    const a = agent({ id: `m${Math.random()}` });
    resetApp({ agents: [a] });
    const backend = fakeBackend({
      get_conversation: () => [],
      voice_start: () => ({ kind: 'needsModel', model: 'base', size: 147_951_465 }),
      voice_download: () => new Promise(() => {}),
    });
    render(Composer, { agent: app.agents[a.id] });
    await click(mic());
    await waitFor(() => expect(panel()).toHaveTextContent('Télécharger le modèle de transcription ?'));
    expect(panel()).toHaveTextContent(/« base » \(1\d\d Mo\)/);
    await userEvent.click(screen.getByRole('button', { name: 'Télécharger' }));
    dictation.take({ kind: 'download', model: 'base', done: 73_975_732, total: 147_951_465 });
    await waitFor(() => expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '50'));
    expect(backend.called('voice_download')[0].args).toEqual({ model: 'base' });
    await userEvent.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(backend.called('voice_download_cancel')).toHaveLength(1);
    expect(panel()).toBeNull();
  });

  it('in English too', async () => {
    composer();
    setLang('en');
    await waitFor(() => expect(mic()).toHaveAccessibleName('Dictate'));
    setLang('fr');
  });
});

describe('MicButton in the ticket form', () => {
  it('dictates into the description, at its cursor', async () => {
    const backend = fakeBackend({
      voice_start: () => ({ kind: 'listening', session: 2 }),
      voice_stop: () => 'la page des réglages',
    });
    app.modal = { kind: 'ticket', projectId: 'p1', onSubmit: vi.fn() as never };
    render(TicketForm, { projectId: 'p1', onsubmit: vi.fn() });
    const description = screen.getByRole('textbox', { name: 'Description' }) as HTMLTextAreaElement;
    await userEvent.type(description, 'Corriger');
    await click(mic());
    await waitFor(() => expect(mic()).toHaveAttribute('aria-pressed', 'true'));
    await click(mic());
    await waitFor(() => expect(description.value).toBe('Corriger la page des réglages'));
    expect(backend.called('voice_stop')[0].args).toEqual({ session: 2, target: 'ticket' });
    expect(backend.called('ticket_create')).toHaveLength(0);
  });
});
