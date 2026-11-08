import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { onlineVoice } from '../../lib/online-voice.svelte';
import type { ServiceCatalog, SpeechInfo, SpeechSettings, SpeechVoice, VoiceInfo } from '../../lib/types';
import { fakeBackend, resetApp } from '../../test/ipc';
import SettingsModal from '../modals/SettingsModal.svelte';
import { afterGesture } from '../../test/settings';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const VOICE: VoiceInfo = { devices: [], models: [{ id: 'base', size: 147_951_465, speed: 4, quality: 2, downloaded: true }] };

function voice(over: Partial<SpeechVoice>): SpeechVoice {
  return {
    id: 'x',
    lang: 'fr',
    locale: 'fr_FR',
    name: 'X',
    quality: 'medium',
    speakers: 1,
    speakerNames: [],
    sampleRate: 22_050,
    repo: 'rhasspy/piper-voices',
    onnxSize: 63_201_294,
    jsonSize: 4_875,
    license: 'mit',
    datasetLicense: null,
    datasetUrl: null,
    attribution: null,
    downloaded: false,
    ...over,
  };
}

const SPEECH: SpeechInfo = {
  available: true,
  voices: [
    voice({
      id: 'fr_FR-siwis-medium',
      name: 'Siwis',
      datasetLicense: 'CC-BY 4.0',
      attribution: 'https://datashare.is.ed.ac.uk/handle/10283/2353 (CC-BY 4.0)',
      downloaded: true,
    }),
    voice({ id: 'fr_FR-mls-medium', name: 'MLS', speakers: 125, onnxSize: 76_733_750, datasetLicense: 'CC-BY 4.0' }),
    voice({ id: 'fr_FR-gilles-low', name: 'Gilles', quality: 'low', datasetLicense: 'CC0' }),
    voice({ id: 'en_US-ljspeech-medium', lang: 'en', name: 'LJSpeech', datasetLicense: 'public domain' }),
  ],
};

function open(over: Record<string, (a: any) => unknown> = {}) {
  const backend = fakeBackend({ voice_info: () => VOICE, speech_info: () => SPEECH, save_settings: () => [], ...over });
  render(SettingsModal, { page: 'voice' });
  return backend;
}

describe('Réglages › Voix, the conversation', () => {
  beforeEach(() => {
    resetApp();
    setLang('fr');
    app.modal = { kind: 'settings' };
  });

  it('shows the voice of each language with its licences and its credit; another one chosen is saved', async () => {
    const backend = open();
    const fr = await waitFor(() => {
      const el = document.querySelector('[data-speech-lang="fr"]') as HTMLElement;
      expect(el).toBeTruthy();
      return el;
    });
    const select = within(fr).getByRole('combobox', { name: 'Voix en Français' }) as HTMLSelectElement;
    await waitFor(() => expect(select.value).toBe('fr_FR-siwis-medium'));
    // Only the French ones, with their quality and size.
    expect([...select.options].map((o) => o.text)).toEqual([
      'Siwis · qualité moyenne · 60 Mo',
      'MLS (125) · qualité moyenne · 73 Mo',
      'Gilles · qualité basse · 60 Mo',
    ]);
    const licence = fr.querySelector('[data-speech-licence]')!.textContent!;
    expect(licence).toContain('Licence du modèle : mit');
    expect(licence).toContain('Jeu de données : CC-BY 4.0');
    expect(licence).toContain('Crédit : https://datashare.is.ed.ac.uk/handle/10283/2353 (CC-BY 4.0)');
    // Downloaded: it can be listened to.
    await userEvent.click(within(fr).getByRole('button', { name: 'Écouter' }));
    expect(backend.called('speech_preview')[0].args).toEqual({ id: 'fr_FR-siwis-medium', speaker: undefined });
    // Another, with several speakers: its speaker is chosen too, and both are saved.
    await userEvent.selectOptions(select, 'fr_FR-mls-medium');
    const speaker = within(fr).getByRole('combobox', { name: 'Locuteur' }) as HTMLSelectElement;
    expect(speaker.options).toHaveLength(125);
    await userEvent.selectOptions(speaker, '3');
    expect(within(fr).getByRole('button', { name: 'Télécharger (73 Mo)' })).toBeTruthy();
    await afterGesture();
    const saved = backend.called('save_settings').at(-1)!.args.settings.voice.speech;
    expect(saved.voices).toEqual({ fr: 'fr_FR-mls-medium' });
    expect(saved.speakers).toEqual({ 'fr_FR-mls-medium': 3 });
  });

  it('says honestly what the licences of a voice offered by exception are, and offers its styles', async () => {
    const fuze = voice({
      id: 'fuze-french-fr-multi-high',
      locale: null,
      name: 'Fuze - French (FR) Multi High',
      quality: 'high',
      speakers: 14,
      speakerNames: ['normal', 'joyeuse', 'excited'],
      repo: 'fox3000foxy/piper-checkpoints-fuzeiii',
      license: '',
      datasetLicense: 'CC BY 4.0',
      datasetUrl: 'https://huggingface.co/datasets/fox3000foxy/fuze-eleven-v4-piper',
      attribution: 'Fuze — dataset by fox3000foxy (CC BY 4.0), generated with ElevenLabs v4',
      exception: true,
    });
    open({ speech_info: () => ({ ...SPEECH, voices: [...SPEECH.voices, fuze] }) });
    const fr = await waitFor(() => {
      const el = document.querySelector('[data-speech-lang="fr"]') as HTMLElement;
      expect(el).toBeTruthy();
      return el;
    });
    const select = within(fr).getByRole('combobox', { name: 'Voix en Français' }) as HTMLSelectElement;
    await waitFor(() => expect(select.value).toBe('fr_FR-siwis-medium'));
    expect(fr.querySelector('[data-speech-exception]')).toBeNull();
    await userEvent.selectOptions(select, 'fuze-french-fr-multi-high');
    const licence = fr.querySelector('[data-speech-licence]')!.textContent!;
    expect(licence).toContain('Licence du modèle : aucune déclarée');
    expect(licence).toContain('Jeu de données : CC BY 4.0');
    expect(licence).toContain('Crédit : Fuze — dataset by fox3000foxy (CC BY 4.0), generated with ElevenLabs v4');
    expect(fr.querySelector('[data-speech-exception]')!.textContent).toContain('Proposée par exception à la règle des licences');
    // Its styles by their names, « normal » first and chosen by default.
    const speaker = within(fr).getByRole('combobox', { name: 'Locuteur' }) as HTMLSelectElement;
    expect([...speaker.options].slice(0, 3).map((o) => o.text)).toEqual(['normal', 'joyeuse', 'excited']);
    expect(speaker.value).toBe('0');
  });

  it('downloads a voice from here', async () => {
    const backend = open({ speech_download: () => null });
    const en = await waitFor(() => {
      const el = document.querySelector('[data-speech-lang="en"]') as HTMLElement;
      expect(el).toBeTruthy();
      return el;
    });
    await userEvent.click(await within(en).findByRole('button', { name: /^Télécharger/ }));
    expect(backend.called('speech_download')[0].args).toEqual({ id: 'en_US-ljspeech-medium' });
  });

  it('says when this machine cannot speak', async () => {
    open({ speech_info: () => ({ ...SPEECH, available: false }) });
    expect(
      await screen.findByText('La synthèse vocale n’est pas disponible : le moteur de la voix (ONNX Runtime) est introuvable dans l’app.'),
    ).toBeTruthy();
  });
});

/** ElevenLabs as its API lists it: its models, its voices (one made on the account). */
const ELEVEN: ServiceCatalog = {
  service: 'elevenlabs',
  transcribe: [{ id: 'scribe_v2', label: 'Scribe v2', note: '90 langues', langs: [] }],
  speak: [
    { id: 'eleven_flash_v2_5', label: 'Eleven Flash v2.5', note: 'le plus rapide, 32 langues', langs: [] },
    { id: 'eleven_v4', label: 'Eleven v4', note: 'le plus expressif, plus lent', langs: [] },
    { id: 'eleven_flash_v2', label: 'Eleven Flash v2', note: '', langs: ['en'] },
  ],
  defaultTranscribe: 'scribe_v2',
  defaultSpeak: 'eleven_flash_v2_5',
  voices: [
    { id: 'JBFqnCBsd6RMkjVDRZzb', name: 'George', langs: ['en', 'fr'], model: '', note: 'male, british', own: false },
    { id: 'XB0fDUnXU5powFXDhCwa', name: 'Charlotte', langs: [], model: '', note: 'female', own: false },
    { id: 'myclone123', name: 'Ma voix', langs: [], model: '', note: '', own: true },
    { id: 'en-only', name: 'Brian', langs: ['en'], model: '', note: 'male', own: false },
  ],
  defaultVoices: {
    'eleven_flash_v2_5|fr': 'JBFqnCBsd6RMkjVDRZzb',
    'eleven_flash_v2_5|en': 'JBFqnCBsd6RMkjVDRZzb',
    'eleven_v4|fr': 'JBFqnCBsd6RMkjVDRZzb',
    'eleven_v4|en': 'JBFqnCBsd6RMkjVDRZzb',
    'eleven_flash_v2|en': 'JBFqnCBsd6RMkjVDRZzb',
  },
  voicesByModel: false,
  voiceExample: 'JBFqnCBsd6RMkjVDRZzb',
  voicesUrl: 'https://elevenlabs.io/app/voice-library',
  speed: [0.7, 1.2],
  listed: true,
  listedAt: 1_792_400_000,
  error: null,
};

/** OpenAI: no voice in its API, a free field. */
const OPENAI: ServiceCatalog = {
  ...ELEVEN,
  service: 'openai',
  speak: [
    { id: 'gpt-4o-mini-tts', label: 'GPT-4o mini TTS', note: 'le plus naturel', langs: [] },
    { id: 'tts-1', label: 'TTS-1', note: 'rapide', langs: [] },
  ],
  defaultSpeak: 'gpt-4o-mini-tts',
  voices: null,
  defaultVoices: { 'gpt-4o-mini-tts|fr': 'marin', 'gpt-4o-mini-tts|en': 'marin', 'tts-1|fr': 'alloy', 'tts-1|en': 'alloy' },
  voiceExample: 'marin, cedar, coral, alloy…',
  voicesUrl: 'https://platform.openai.com/docs/guides/text-to-speech#voice-options',
  speed: [0.25, 4],
  listed: false,
  listedAt: 0,
};

/** Groq: Orpheus reads English only, at its own pace. */
const GROQ: ServiceCatalog = {
  ...OPENAI,
  service: 'groq',
  speak: [{ id: 'canopylabs/orpheus-v1-english', label: 'Orpheus English', note: 'anglais seulement', langs: ['en'] }],
  defaultSpeak: 'canopylabs/orpheus-v1-english',
  defaultVoices: { 'canopylabs/orpheus-v1-english|en': 'hannah' },
  voiceExample: 'hannah, troy, austin, diana…',
  speed: null,
};

describe('Réglages › Voix, the conversation’s voice on line (ESC-132)', () => {
  const CATALOGS: Record<string, ServiceCatalog> = { elevenlabs: ELEVEN, openai: OPENAI, groq: GROQ };

  beforeEach(() => {
    resetApp();
    setLang('fr');
    app.modal = { kind: 'settings' };
    onlineVoice.catalogs = {};
    onlineVoice.keys = [];
  });

  const online = (service: string, over: Partial<SpeechSettings> = {}) => {
    app.settings.voice = {
      ...app.settings.voice,
      speech: { ...app.settings.voice.speech, engine: 'online', onlineService: service, ...over },
    };
  };
  const openOnline = (over: Record<string, (a: any) => unknown> = {}) =>
    open({
      voice_online_keys: () => ['elevenlabs'],
      voice_online_catalog: ({ service }) => CATALOGS[service] ?? OPENAI,
      ...over,
    });

  it('says on line that the conversation pauses when the service fails and no local voice is downloaded', async () => {
    online('elevenlabs');
    openOnline({ speech_info: () => ({ ...SPEECH, voices: SPEECH.voices.map((v) => ({ ...v, downloaded: false })) }) });
    expect(
      await screen.findByText(
        'La conversation se met en pause : aucune voix locale (Piper) n’est téléchargée. Pour en avoir une en secours, repasse sur « Local (Piper) » et télécharge-la.',
      ),
    ).toBeInTheDocument();
    // Back to Piper: its voices, their downloads and licences, as before.
    await userEvent.click(within(screen.getByRole('group', { name: 'Voix générée par' })).getByRole('button', { name: 'Local (Piper)' }));
    expect(await screen.findByRole('combobox', { name: 'Voix en Français' })).toBeInTheDocument();
    expect(screen.queryByText('Si ElevenLabs ne répond pas')).toBeNull();
  });

  it('is generated locally by Piper by default; on line names the service, what leaves and where', async () => {
    const backend = open();
    const group = await screen.findByRole('group', { name: 'Voix générée par' });
    expect(within(group).getByRole('button', { name: 'Local (Piper)' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.queryByRole('combobox', { name: 'Service de la voix' })).toBeNull();
    await userEvent.click(within(group).getByRole('button', { name: 'En ligne' }));
    const service = (await screen.findByRole('combobox', { name: 'Service de la voix' })) as HTMLSelectElement;
    expect([...service.options].map((o) => o.text)).toEqual(['OpenAI', 'Groq', 'Mistral', 'ElevenLabs', 'Deepgram']);
    expect(
      screen.getByText(
        /le texte de chaque passage lu, phrase par phrase, vers OpenAI \(api\.openai\.com\).*Le son du micro n’y part pas : ce que tu dis est transcrit comme ta dictée/,
      ),
    ).toBeInTheDocument();
    // Piper's settings are those of « Local (Piper) »: on line, one line says which local voice reads if the service fails.
    expect(
      await screen.findByText(
        'La conversation continue avec la voix locale (Piper) : Siwis (Français). Pour la changer, repasse sur « Local (Piper) ».',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText('Si OpenAI ne répond pas')).toBeInTheDocument();
    expect(screen.queryByRole('combobox', { name: 'Voix en Français' })).toBeNull();
    expect(screen.queryByRole('button', { name: /^Télécharger/ })).toBeNull();
    expect(screen.queryByText(/^Licence/)).toBeNull();
    await userEvent.selectOptions(service, 'elevenlabs');
    expect(screen.getByText('Si ElevenLabs ne répond pas')).toBeInTheDocument();
    await afterGesture();
    const speech = backend.called('save_settings').at(-1)!.args.settings.voice.speech;
    expect(speech.engine).toBe('online');
    expect(speech.onlineService).toBe('elevenlabs');
    expect(JSON.stringify(speech)).not.toMatch(/key/i);
  });

  it('shares the key of the dictation: saved once, from either place', async () => {
    online('groq');
    let keys: string[] = [];
    const backend = openOnline({
      voice_online_keys: () => keys,
      voice_online_key_set: ({ service }) => {
        keys = [service];
      },
    });
    const field = await screen.findByLabelText('Clé de Groq');
    expect(screen.getByText(/la même pour la dictée et pour la voix de la conversation/)).toBeInTheDocument();
    await userEvent.type(field, 'gsk_secret{Enter}');
    expect(backend.called('voice_online_key_set')[0].args).toEqual({ service: 'groq', key: 'gsk_secret' });
    expect(await screen.findByText('Enregistrée dans le trousseau')).toBeInTheDocument();
    // Saved, its models and voices are read with it.
    await waitFor(() => expect(backend.called('voice_online_catalog').some((c) => c.args.refresh)).toBe(true));
    expect(document.body.innerHTML).not.toContain('gsk_secret');
  });

  it('offers the models of the service, its default first, kept by service', async () => {
    online('elevenlabs');
    const backend = openOnline();
    const select = (await screen.findByRole('combobox', { name: 'Modèle de la voix' })) as HTMLSelectElement;
    await waitFor(() => expect(select.options).toHaveLength(3));
    expect([...select.options].map((o) => o.text)).toEqual([
      'Eleven Flash v2.5 · le plus rapide, 32 langues (par défaut)',
      'Eleven v4 · le plus expressif, plus lent',
      'Eleven Flash v2',
    ]);
    expect(screen.getByText(/^Liste lue chez ElevenLabs le /)).toBeInTheDocument();
    await userEvent.selectOptions(select, 'eleven_v4');
    // An English-only model: French is left to Piper, and said.
    await userEvent.selectOptions(select, 'eleven_flash_v2');
    expect(
      screen.getByText('ElevenLabs (eleven_flash_v2) ne lit pas cette langue : la voix locale (Piper) lit la conversation en Français.'),
    ).toBeInTheDocument();
    await userEvent.selectOptions(select, 'eleven_v4');
    // ElevenLabs takes the speed within its bounds.
    expect(screen.getByText('ElevenLabs lit entre 0.7× et 1.2× : la vitesse est ramenée dans ces bornes.')).toBeInTheDocument();
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings.voice.speech.onlineModels).toEqual({ elevenlabs: 'eleven_v4' });
  });

  it('completes the voice as one types, the account’s own among them, picked with the keyboard, and plays it', async () => {
    online('elevenlabs');
    const backend = openOnline();
    const fr = await waitFor(() => {
      const el = document.querySelector('[data-online-voice="fr"]') as HTMLElement;
      expect(el).toBeTruthy();
      return el;
    });
    const field = within(fr).getByRole('combobox', { name: 'Voix en Français' });
    expect(field).toHaveAttribute('placeholder', 'George');
    await userEvent.click(field);
    const list = within(fr).getByRole('listbox');
    // The voices that read French (Brian is English only), the user's own first.
    expect(
      within(list)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual([expect.stringContaining('Ma voix'), expect.stringContaining('George'), expect.stringContaining('Charlotte')]);
    expect(within(list).getAllByRole('option')[0]).toHaveTextContent('ton compte');
    await userEvent.type(field, 'char');
    expect(within(list).getAllByRole('option')).toHaveLength(1);
    await userEvent.type(field, '{ArrowDown}{Enter}');
    expect(field).toHaveValue('Charlotte');
    // Filtered by description too.
    await userEvent.clear(field);
    await userEvent.type(field, 'british');
    expect(
      within(within(fr).getByRole('listbox'))
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual([expect.stringContaining('George')]);
    await userEvent.keyboard('{Escape}');
    expect(within(fr).queryByRole('listbox')).toBeNull();
    await userEvent.click(within(fr).getByRole('button', { name: 'Écouter' }));
    await waitFor(() =>
      expect(backend.called('speech_preview_online')[0].args).toEqual({
        service: 'elevenlabs',
        model: 'eleven_flash_v2_5',
        voiceId: 'XB0fDUnXU5powFXDhCwa',
        lang: 'fr',
      }),
    );
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings.voice.speech.onlineVoices).toEqual({
      'elevenlabs|fr': 'XB0fDUnXU5powFXDhCwa',
    });
  });

  it('without a list in its API: a free field, with an example and its default, and a voice refused said at once', async () => {
    online('openai');
    const toast = vi.spyOn(app, 'toast');
    const backend = openOnline({
      voice_online_keys: () => ['openai'],
      speech_preview_online: ({ voiceId }) => {
        if (voiceId === 'nope') throw 'OpenAI n’a pas lu le passage (400) : Invalid voice nope.';
      },
    });
    const fr = await waitFor(() => {
      const el = document.querySelector('[data-online-voice="fr"]') as HTMLElement;
      expect(el).toBeTruthy();
      return el;
    });
    const field = within(fr).getByRole('textbox', { name: 'Voix en Français' });
    expect(field).toHaveAttribute('placeholder', 'marin');
    expect(fr.closest('.sr')).toHaveTextContent('par exemple marin, cedar, coral, alloy…');
    expect(within(fr).getByRole('button', { name: 'Voir ses voix' })).toBeInTheDocument();
    // Empty: its default voice plays.
    await userEvent.click(within(fr).getByRole('button', { name: 'Écouter' }));
    await waitFor(() => expect(backend.called('speech_preview_online')[0]?.args.voiceId).toBe('marin'));
    await userEvent.type(field, 'nope');
    await userEvent.tab();
    await userEvent.click(within(fr).getByRole('button', { name: 'Écouter' }));
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith('La voix n’a pas pu être lue : OpenAI n’a pas lu le passage (400) : Invalid voice nope.', 'error'),
    );
    await userEvent.clear(field);
    await userEvent.type(field, 'cedar');
    await userEvent.tab();
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings.voice.speech.onlineVoices).toEqual({ 'openai|fr': 'cedar' });
  });

  it('a service that does not read French leaves it to Piper, and takes no speed', async () => {
    online('groq');
    openOnline({ voice_online_keys: () => ['groq'] });
    expect(
      await screen.findByText(
        'Groq (canopylabs/orpheus-v1-english) ne lit pas cette langue : la voix locale (Piper) lit la conversation en Français.',
      ),
    ).toBeInTheDocument();
    expect(document.querySelector('[data-online-voice="en"]')).toBeTruthy();
    expect(screen.getByText('Groq lit à son propre rythme : la vitesse ne s’applique pas.')).toBeInTheDocument();
  });
});
