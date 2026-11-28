import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { dbfs, levelsOf, meterPos, micMeter, sensitivityText, thresholdOf } from '../../lib/mic-meter.svelte';
import { SETTINGS_ROWS } from '../../lib/settings-pages';
import { app } from '../../lib/state.svelte';
import type { SpeechInfo, VoiceInfo } from '../../lib/types';
import { fakeBackend, resetApp } from '../../test/ipc';
import { afterGesture } from '../../test/settings';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const VOICE: VoiceInfo = {
  devices: [{ id: 'wasapi:{a}', name: 'Micro du casque', default: true }],
  models: [{ id: 'base', size: 147_951_465, speed: 4, quality: 2, downloaded: true }],
};
const SPEECH: SpeechInfo = { available: true, voices: [] };

function open(over: Record<string, (a: any) => unknown> = {}) {
  const backend = fakeBackend({
    voice_info: () => VOICE,
    speech_info: () => SPEECH,
    save_settings: () => [],
    voice_meter_start: () => 7,
    ...over,
  });
  const view = render(SettingsModal, { page: 'voice' });
  return { backend, view };
}

const slider = () => screen.findByRole('slider', { name: 'Sensibilité du micro' }) as Promise<HTMLInputElement>;
/** What the backend says of the tester, every 50 ms. */
const level = (level: number, noise: number, speech: boolean, session = 7) =>
  micMeter.take({ kind: 'meter', session, level, noise, threshold: thresholdOf(noise, micMeter.sensitivity), speech });
const line = () => document.querySelector('[data-threshold]') as HTMLElement;

describe('« Sensibilité du micro »', () => {
  beforeEach(() => {
    resetApp();
    micMeter.clear();
    setLang('fr');
    app.modal = { kind: 'settings' };
  });
  afterEach(() => micMeter.clear());

  it('is the detector of the backend, step by step: 0 is today’s', () => {
    expect(levelsOf(0)).toEqual({ overNoise: 3.2, floor: 0.005 });
    // As `vad::Levels::of` computes it: 1.5 dB a step, the floor with it, never under twice the noise.
    expect(levelsOf(5).floor).toBeCloseTo(0.005 * 10 ** (-7.5 / 20), 6);
    expect(levelsOf(-5).floor).toBeCloseTo(0.005 * 10 ** (7.5 / 20), 6);
    expect(levelsOf(1).overNoise).toBeCloseTo(3.2 * 10 ** (-1.5 / 20), 6);
    expect(levelsOf(5).overNoise).toBe(2);
    expect(levelsOf(40)).toEqual(levelsOf(5));
    expect(thresholdOf(0.0005, 0)).toBe(0.005);
    expect(thresholdOf(0.01, 0)).toBeCloseTo(0.032, 6);
    expect(meterPos(0.1)).toBeCloseTo(50 / 60, 6);
    expect(meterPos(0)).toBe(0);
    expect(Math.round(dbfs(0.005))).toBe(-46);
    expect(sensitivityText(0, 'Par défaut')).toBe('Par défaut');
    expect(sensitivityText(2, 'x')).toBe('+2');
    expect(sensitivityText(-3, 'x')).toBe('−3');
  });

  it('is a row of Réglages › Voix, by default today’s, and is saved with the settings of the machine', async () => {
    expect(SETTINGS_ROWS.find((r) => r.id === 'micSensitivity')).toMatchObject({
      page: 'voice',
      label: 'Sensibilité du micro',
      settings: ['voice'],
    });
    expect(SETTINGS_ROWS.find((r) => r.id === 'micSensitivity')!.descIn('fr')).toMatch(/La dictée n’en dépend pas/);
    const { backend } = open();
    const s = await slider();
    expect(s.value).toBe('0');
    expect(s.min).toBe('-5');
    expect(s.max).toBe('5');
    expect(s.getAttribute('aria-valuetext')).toBe('Par défaut');
    // It says what it acts on: the conversation, cutting the agent off, the transcription on line too; not the dictation.
    expect(screen.getByText(/début et la fin de tes phrases, et ta voix quand tu coupes l’agent/)).toBeTruthy();
    expect(screen.getByText(/avec la transcription en ligne/)).toBeTruthy();
    await fireEvent.input(s, { target: { value: '3' } });
    await fireEvent.change(s);
    await afterGesture();
    expect(backend.called('save_settings').at(-1)!.args.settings.voice.speech.sensitivity).toBe(3);
    expect(document.querySelector('[data-sensitivity-value]')!.textContent).toBe('+3');
    // Never shown by the tester alone: no listening until « Tester ».
    expect(backend.called('voice_meter_start')).toHaveLength(0);
  });

  it('tests the microphone chosen: level, noise, the line moved by the slider at once, « Voix détectée » lit then out', async () => {
    app.settings.voice = { ...app.settings.voice, device: 'wasapi:{a}' };
    app.settings.voice.speech = { ...app.settings.voice.speech, silenceMs: 900 };
    const { backend } = open();
    await slider();
    await userEvent.click(screen.getByRole('button', { name: 'Tester' }));
    await waitFor(() => expect(micMeter.session).toBe(7));
    expect(backend.called('voice_meter_start')[0].args).toEqual({ device: 'wasapi:{a}', sensitivity: 0, silenceMs: 900 });
    const tester = await screen.findByRole('group', { name: 'Testeur du micro' });
    expect(within(tester).getByText(/le bruit de la pièce se mesure/)).toBeTruthy();
    // A quiet room: the line at the floor of today's detector.
    level(0.001, 0.0008, false);
    await waitFor(() => expect(Number(line().dataset.threshold)).toBeCloseTo(0.005, 6));
    const at = line().style.left;
    expect(within(tester).getByText('Pas de voix')).toBeTruthy();
    expect(within(tester).getByRole('meter', { name: 'Niveau de ta voix' }).getAttribute('aria-valuenow')).toBe(
      String(Math.round(meterPos(0.001) * 100)),
    );
    expect(within(tester).getByText('-60 dB')).toBeTruthy();
    // More sensitive: the line comes down before any new level, and the backend is told.
    await fireEvent.input(await slider(), { target: { value: '4' } });
    await waitFor(() => expect(line().style.left).not.toBe(at));
    expect(parseFloat(line().style.left)).toBeLessThan(parseFloat(at));
    expect(Number(line().dataset.threshold)).toBeCloseTo(thresholdOf(0.0008, 4), 6);
    await waitFor(() => expect(backend.called('voice_meter_set').at(-1)!.args).toEqual({ session: 7, sensitivity: 4 }));
    // Less sensitive: it goes up past where it was.
    await fireEvent.input(await slider(), { target: { value: '-4' } });
    await waitFor(() => expect(parseFloat(line().style.left)).toBeGreaterThan(parseFloat(at)));
    // A voice: the light comes on; the silence after it: it goes out.
    level(0.2, 0.0008, true);
    const witness = await within(tester).findByText('Voix détectée');
    expect(witness.closest('[data-voice-heard]')!.getAttribute('data-voice-heard')).toBe('true');
    level(0.001, 0.0008, true);
    expect(within(tester).getByText('Voix détectée')).toBeTruthy();
    level(0.001, 0.0008, false);
    await within(tester).findByText('Pas de voix');
    // A level of another listening is not this one's.
    level(0.3, 0.0008, true, 99);
    expect(within(tester).getByText('Pas de voix')).toBeTruthy();
    // « Arrêter »: the listening stops, the tester goes.
    await userEvent.click(within(document.body).getByRole('button', { name: 'Arrêter' }));
    expect(backend.called('voice_meter_stop')).toEqual([{ cmd: 'voice_meter_stop', args: { session: 7 } }]);
    await waitFor(() => expect(screen.queryByRole('group', { name: 'Testeur du micro' })).toBeNull());
  });

  it('stops when the page is left, and says why it could not listen', async () => {
    const { backend, view } = open();
    await slider();
    await userEvent.click(screen.getByRole('button', { name: 'Tester' }));
    await waitFor(() => expect(micMeter.session).toBe(7));
    view.unmount();
    expect(backend.called('voice_meter_stop').at(-1)!.args).toEqual({ session: 7 });
    expect(micMeter.session).toBeNull();
    micMeter.clear();
    resetApp();
    app.modal = { kind: 'settings' };
    open({
      voice_meter_start: () => {
        throw 'Le micro est déjà utilisé par une dictée ou une conversation.';
      },
    });
    await slider();
    await userEvent.click(screen.getByRole('button', { name: 'Tester' }));
    expect(await screen.findByText(/^Le micro n’a pas pu être ouvert : Le micro est déjà utilisé/)).toBeTruthy();
    expect(screen.queryByRole('group', { name: 'Testeur du micro' })).toBeNull();
  });

  it('stopped while it opens, lets the microphone go as soon as it is open', async () => {
    let open_: (id: number) => void = () => {};
    const { backend } = open({ voice_meter_start: () => new Promise<number>((r) => (open_ = r)) });
    await slider();
    await userEvent.click(screen.getByRole('button', { name: 'Tester' }));
    await userEvent.click(screen.getByRole('button', { name: 'Arrêter' }));
    open_(12);
    await waitFor(() => expect(backend.called('voice_meter_stop').at(-1)!.args).toEqual({ session: 12 }));
    expect(micMeter.session).toBeNull();
    expect(screen.getByRole('button', { name: 'Tester' })).toBeTruthy();
  });

  it('in English', async () => {
    setLang('en');
    open();
    const s = await screen.findByRole('slider', { name: 'Microphone sensitivity' });
    expect(s.getAttribute('aria-valuetext')).toBe('Default');
    expect(screen.getByText('Less sensitive')).toBeTruthy();
    await userEvent.click(screen.getByRole('button', { name: 'Test' }));
    expect(await screen.findByRole('group', { name: 'Microphone tester' })).toBeTruthy();
    expect(screen.getByText('No voice')).toBeTruthy();
  });
});
