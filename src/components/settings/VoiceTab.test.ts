import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { dictation } from '../../lib/dictation.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { onlineVoice } from '../../lib/online-voice.svelte';
import type { ServiceCatalog, VoiceInfo } from '../../lib/types';
import { fakeBackend, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const INFO: VoiceInfo = {
  devices: [
    { id: 'wasapi:{a}', name: 'Micro du casque', default: true },
    { id: 'wasapi:{b}', name: 'Micro de la webcam', default: false },
  ],
  models: [
    { id: 'tiny', size: 77_691_713, speed: 5, quality: 1, downloaded: false },
    { id: 'base', size: 147_951_465, speed: 4, quality: 2, downloaded: true },
    { id: 'small', size: 487_601_967, speed: 3, quality: 3, downloaded: false },
  ],
};

const save = () => userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }));

function open(over: Record<string, (a: any) => unknown> = {}) {
  const backend = fakeBackend({ voice_info: () => INFO, save_settings: () => [], ...over });
  render(SettingsModal, { tab: 'voice' });
  return backend;
}

/** What a service's catalog looks like (Groq's, listed when `listed`). */
function catalog(service: string, listed = false): ServiceCatalog {
  return {
    service,
    transcribe: [
      { id: 'whisper-large-v3-turbo', label: 'Whisper Large v3 Turbo', note: 'rapide', langs: [] },
      { id: 'whisper-large-v3', label: 'Whisper Large v3', note: 'plus précis', langs: [] },
    ],
    speak: [{ id: 'canopylabs/orpheus-v1-english', label: 'Orpheus English', note: 'anglais seulement', langs: ['en'] }],
    defaultTranscribe: 'whisper-large-v3-turbo',
    defaultSpeak: 'canopylabs/orpheus-v1-english',
    voices: null,
    defaultVoices: { 'canopylabs/orpheus-v1-english|en': 'hannah' },
    voicesByModel: false,
    voiceExample: 'hannah, troy, austin, diana…',
    voicesUrl: 'https://console.groq.com/docs/text-to-speech/orpheus',
    speed: null,
    listed,
    listedAt: listed ? 1_792_400_000 : 0,
    error: null,
  };
}

describe('Réglages › Voix', () => {
  beforeEach(() => {
    resetApp();
    onlineVoice.catalogs = {};
    onlineVoice.keys = [];
    setLang('fr');
    app.modal = { kind: 'settings' };
  });

  it('lists the microphones, the system’s first, and saves the one chosen', async () => {
    const backend = open();
    const select = (await screen.findByRole('combobox', { name: 'Micro utilisé' })) as HTMLSelectElement;
    await waitFor(() => expect(select.options).toHaveLength(3));
    expect([...select.options].map((o) => o.text)).toEqual(['Celui du système (Micro du casque)', 'Micro du casque', 'Micro de la webcam']);
    expect(select.value).toBe('');
    await userEvent.selectOptions(select, 'wasapi:{b}');
    await save();
    expect(backend.called('save_settings')[0].args.settings.voice).toEqual({
      device: 'wasapi:{b}',
      model: 'base',
      language: 'ui',
      shortcut: 'Mod+Shift+Space',
      speech: {
        voices: {},
        speakers: {},
        rate: 1,
        volume: 1,
        silenceMs: 1200,
        engine: 'local',
        onlineService: 'openai',
        onlineModels: {},
        onlineVoices: {},
      },
      transcription: 'local',
      onlineService: 'openai',
      onlineModels: {},
    });
  });

  it('transcribes locally by default, and says nothing leaves the machine', async () => {
    const backend = open();
    const group = await screen.findByRole('group', { name: 'Transcrite par' });
    expect(within(group).getByRole('button', { name: 'Local (Whisper)' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText(/rien de ce que tu dis n’est envoyé/)).toBeInTheDocument();
    expect(screen.queryByRole('combobox', { name: 'Service' })).toBeNull();
    expect(await screen.findByRole('radio', { name: /^base/ })).toBeInTheDocument();
    await waitFor(() => expect(backend.called('voice_online_keys')).toHaveLength(1));
  });

  it('online: names the service, says what leaves and where, and saves the choice', async () => {
    const backend = open({ voice_online_keys: () => [] });
    const group = await screen.findByRole('group', { name: 'Transcrite par' });
    await userEvent.click(within(group).getByRole('button', { name: 'En ligne' }));
    // The models of Whisper are not what transcribes any more.
    expect(screen.queryByRole('radio', { name: /^base/ })).toBeNull();
    const select = screen.getByRole('combobox', { name: 'Service' }) as HTMLSelectElement;
    expect([...select.options].map((o) => o.text)).toEqual(['OpenAI', 'Groq', 'Mistral', 'ElevenLabs', 'Deepgram']);
    // As one speaks by default (ESC-136); Groq, which does not, takes each dictation whole.
    expect(screen.getByText(/le son de ce que tu dis, au fil de ta parole, vers OpenAI \(api\.openai\.com\)/)).toBeInTheDocument();
    expect(screen.getByText(/Aucune clé enregistrée/)).toBeInTheDocument();
    await userEvent.selectOptions(select, 'groq');
    expect(
      screen.getByText(/le son de chaque dictée \(un fichier WAV\), une fois que tu la finis, vers Groq \(api\.groq\.com\)/),
    ).toBeInTheDocument();
    await userEvent.selectOptions(select, 'deepgram');
    expect(screen.getByText(/vers Deepgram \(api\.deepgram\.com\)/)).toBeInTheDocument();
    expect(screen.getByText('Payé à l’usage sur ton compte Deepgram.')).toBeInTheDocument();
    await save();
    const voice = backend.called('save_settings')[0].args.settings.voice;
    expect(voice.transcription).toBe('online');
    expect(voice.onlineService).toBe('deepgram');
    // The settings never hold a key.
    expect(JSON.stringify(voice)).not.toMatch(/key/i);
  });

  it('keeps the key in the keychain at once, never in the settings, and shows it saved, not the key', async () => {
    app.settings.voice = { ...app.settings.voice, transcription: 'online', onlineService: 'groq' };
    let keys: string[] = [];
    const backend = open({
      voice_online_keys: () => keys,
      voice_online_key_set: ({ service }) => {
        keys = [service];
      },
      voice_online_key_delete: () => {
        keys = [];
      },
    });
    const field = await screen.findByLabelText('Clé de Groq');
    expect(field).toHaveAttribute('type', 'password');
    expect(screen.getByRole('button', { name: 'Enregistrer la clé' })).toBeDisabled();
    await userEvent.type(field, 'gsk_secret');
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer la clé' }));
    expect(backend.called('voice_online_key_set')[0].args).toEqual({ service: 'groq', key: 'gsk_secret' });
    expect(await screen.findByText('Enregistrée dans le trousseau')).toBeInTheDocument();
    expect(screen.queryByLabelText('Clé de Groq')).toBeNull();
    expect(document.body.innerHTML).not.toContain('gsk_secret');
    expect(backend.called('save_settings')).toHaveLength(0);
    await userEvent.click(screen.getByRole('button', { name: 'Effacer la clé' }));
    expect(backend.called('voice_online_key_delete')[0].args).toEqual({ service: 'groq' });
    expect(await screen.findByLabelText('Clé de Groq')).toBeInTheDocument();
  });

  it('says the Vibe account Mistral goes by, whose key it may replace but not erase (ESC-327)', async () => {
    app.settings.voice = { ...app.settings.voice, transcription: 'online', onlineService: 'mistral' };
    open({
      voice_online_keys: () => ['mistral'],
      voice_mistral_key: () => ({ own: null, borrowed: { account: 'Équipe', hint: 'clé …wxyz' } }),
    });
    expect(await screen.findByText('Celle du compte Vibe « Équipe » · clé …wxyz')).toBeInTheDocument();
    expect(screen.getByText(/Le compte Mistral, partagé par la voix et Mistral Vibe/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Effacer la clé' })).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Donner une autre clé' }));
    expect(await screen.findByLabelText('Clé de Mistral')).toHaveAttribute('type', 'password');
  });

  it('keeps a key of its own for Mistral as for any service', async () => {
    app.settings.voice = { ...app.settings.voice, transcription: 'online', onlineService: 'mistral' };
    open({
      voice_online_keys: () => ['mistral'],
      voice_mistral_key: () => ({ own: 'clé …a1b2', borrowed: null }),
    });
    expect(await screen.findByText('Enregistrée dans le trousseau')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Effacer la clé' })).toBeInTheDocument();
  });

  it('offers the models of the service, today’s by default, and keeps the one chosen by service (ESC-132)', async () => {
    app.settings.voice = { ...app.settings.voice, transcription: 'online', onlineService: 'groq' };
    const backend = open({
      voice_online_keys: () => ['groq'],
      voice_online_catalog: ({ service, refresh }) => catalog(service, refresh),
    });
    const select = (await screen.findByRole('combobox', { name: 'Modèle' })) as HTMLSelectElement;
    await waitFor(() => expect(select.options).toHaveLength(2));
    expect([...select.options].map((o) => o.text)).toEqual([
      'Whisper Large v3 Turbo · rapide (par défaut)',
      'Whisper Large v3 · plus précis',
    ]);
    expect(select.value).toBe('whisper-large-v3-turbo');
    expect(screen.getByText('Liste de la documentation de Groq : enregistre ta clé pour lire la sienne.')).toBeInTheDocument();
    // The key is the one of the dictation and of the voice: said on its line.
    expect(screen.getByText(/la même pour la dictée et pour la voix de la conversation : enregistrée une fois/)).toBeInTheDocument();
    await userEvent.selectOptions(select, 'whisper-large-v3');
    // « Actualiser la liste »: read again from the service, with the key.
    await userEvent.click(screen.getByRole('button', { name: 'Actualiser la liste' }));
    await waitFor(() => expect(backend.called('voice_online_catalog').some((c) => c.args.refresh)).toBe(true));
    expect(await screen.findByText(/^Liste lue chez Groq le /)).toBeInTheDocument();
    await save();
    expect(backend.called('save_settings')[0].args.settings.voice.onlineModels).toEqual({ groq: 'whisper-large-v3' });
  });

  it('« Transcription en temps réel », ticked by default, kept by service, shown only for a service that offers it (ESC-136)', async () => {
    app.settings.voice = { ...app.settings.voice, transcription: 'online', onlineService: 'elevenlabs' };
    const backend = open({ voice_online_keys: () => [], voice_online_catalog: ({ service }) => catalog(service) });
    const box = (await screen.findByRole('switch', { name: 'Transcription en temps réel' })) as HTMLInputElement;
    expect(box.checked).toBe(true);
    expect(screen.getByText(/Le texte s’écrit pendant que tu parles/)).toHaveTextContent('« scribe_v2_realtime »');
    expect(screen.getByText(/En temps réel, ElevenLabs facture la minute plus cher/)).toBeInTheDocument();
    // What leaves, said as it goes: only while the user speaks.
    expect(screen.getByText(/au fil de ta parole, vers ElevenLabs \(api\.elevenlabs\.io\)/)).toBeInTheDocument();
    expect(screen.getByText(/ni le silence, ni la voix de l’agent/)).toBeInTheDocument();
    await userEvent.click(box);
    expect(box.checked).toBe(false);
    expect(screen.getByText(/le son de chaque dictée \(un fichier WAV\), une fois que tu la finis/)).toBeInTheDocument();
    // Deepgram streams with the model chosen; Groq offers none: no box.
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Service' }), 'deepgram');
    const dg = (await screen.findByRole('switch', { name: 'Transcription en temps réel' })) as HTMLInputElement;
    expect(dg.checked).toBe(true);
    expect(screen.getByText(/Le texte s’écrit pendant que tu parles/)).toHaveTextContent('« nova-3 »');
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Service' }), 'groq');
    await waitFor(() => expect(screen.queryByRole('switch', { name: 'Transcription en temps réel' })).toBeNull());
    await save();
    expect(backend.called('save_settings')[0].args.settings.voice.onlineLive).toEqual({ elevenlabs: false });
  });

  it('a model the service no longer offers falls back on its default, and says so', async () => {
    app.settings.voice = {
      ...app.settings.voice,
      transcription: 'online',
      onlineService: 'groq',
      onlineModels: { groq: 'whisper-gone' },
    };
    open({ voice_online_keys: () => ['groq'], voice_online_catalog: ({ service }) => catalog(service) });
    expect(
      await screen.findByText('Groq ne propose plus « whisper-gone » : son modèle par défaut, « whisper-large-v3-turbo », le remplace.'),
    ).toBeInTheDocument();
    expect((screen.getByRole('combobox', { name: 'Modèle' }) as HTMLSelectElement).value).toBe('whisper-large-v3-turbo');
  });

  it('says when the keychain refuses the key', async () => {
    app.settings.voice = { ...app.settings.voice, transcription: 'online', onlineService: 'mistral' };
    const toast = vi.spyOn(app, 'toast');
    open({
      voice_online_keys: () => [],
      voice_online_key_set: () => {
        throw 'no system keychain here';
      },
    });
    await userEvent.type(await screen.findByLabelText('Clé de Mistral'), 'k-123{Enter}');
    await waitFor(() => expect(toast).toHaveBeenCalledWith('La clé n’a pas été enregistrée : no system keychain here', 'error'));
  });

  it('shows each model with its size, speed and quality: base by default, downloaded; another chosen is saved', async () => {
    const backend = open();
    const base = await screen.findByRole('radio', { name: /^base/ });
    expect(base).toBeChecked();
    expect(screen.getByText(/^base/).closest('.model')).toHaveTextContent('Téléchargé');
    expect(screen.getByRole('radio', { name: /^small/ }).closest('label')).toHaveTextContent(/4\d\d Mo · Vitesse 3\/5 · qualité 3\/5/);
    await userEvent.click(screen.getByRole('radio', { name: /^small/ }));
    await save();
    expect(backend.called('save_settings')[0].args.settings.voice.model).toBe('small');
  });

  it('deletes a model at once, and downloads one with its progress', async () => {
    let info = INFO;
    let finish!: () => void;
    const backend = open({
      voice_info: () => info,
      voice_delete_model: () => {
        info = { ...INFO, models: INFO.models.map((m) => ({ ...m, downloaded: false })) };
      },
      voice_download: () => new Promise<void>((r) => (finish = r)),
    });
    await userEvent.click(await screen.findByRole('button', { name: 'Effacer' }));
    expect(backend.called('voice_delete_model')[0].args).toEqual({ model: 'base' });
    await waitFor(() => expect(screen.queryByText('Téléchargé')).toBeNull());
    await userEvent.click(screen.getAllByRole('button', { name: 'Télécharger' })[0]);
    expect(backend.called('voice_download')[0].args).toEqual({ model: 'tiny' });
    dictation.take({ kind: 'download', model: 'tiny', done: 38_845_856, total: 77_691_713 });
    await waitFor(() => expect(screen.getByText('50 %')).toBeInTheDocument());
    info = { ...INFO, models: INFO.models.map((m) => ({ ...m, downloaded: m.id === 'tiny' })) };
    finish();
    await waitFor(() => expect(screen.getByText(/^tiny/).closest('.model')).toHaveTextContent('Téléchargé'));
  });

  it('sets the language spoken: the interface’s by default, automatic detection, or one named in itself', async () => {
    const backend = open();
    const menu = (await screen.findByRole('combobox', { name: 'Langue parlée' })) as HTMLSelectElement;
    expect(menu.selectedOptions[0].text).toBe('Celle de l’interface');
    expect(within(menu).getByRole('option', { name: 'Español' })).toHaveAttribute('lang', 'es');
    await userEvent.selectOptions(menu, 'auto');
    await save();
    expect(backend.called('save_settings')[0].args.settings.voice.language).toBe('auto');
  });

  it('records a new shortcut, keeps the old one on Escape, and goes back to the default one', async () => {
    const backend = open();
    expect(await screen.findByText('Ctrl+Maj+Espace')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Changer' }));
    const rec = screen.getByRole('button', { name: 'Appuie sur le raccourci…' });
    expect(dictation.recording).toBe(true);
    await fireEvent.keyDown(rec, { key: 'Escape', code: 'Escape' });
    expect(screen.getByText('Ctrl+Maj+Espace')).toBeInTheDocument();
    expect(dictation.recording).toBe(false);
    await userEvent.click(screen.getByRole('button', { name: 'Changer' }));
    // A letter without Ctrl or Alt would be typed in every field: not taken.
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Appuie sur le raccourci…' }), { key: 'd', code: 'KeyD' });
    await fireEvent.keyDown(screen.getByRole('button', { name: 'Appuie sur le raccourci…' }), {
      key: 'd',
      code: 'KeyD',
      ctrlKey: true,
      altKey: true,
    });
    expect(screen.getByText('Ctrl+Alt+D')).toBeInTheDocument();
    await save();
    expect(backend.called('save_settings')[0].args.settings.voice.shortcut).toBe('Mod+Alt+KeyD');
  });

  it('in English', async () => {
    open();
    setLang('en');
    expect(await screen.findByRole('combobox', { name: 'Microphone used' })).toBeInTheDocument();
    expect(screen.getByText('Ctrl+Shift+Space')).toBeInTheDocument();
    expect(within(screen.getByRole('group', { name: 'Transcribed by' })).getByRole('button', { name: 'Online' })).toBeInTheDocument();
    setLang('fr');
  });
});
