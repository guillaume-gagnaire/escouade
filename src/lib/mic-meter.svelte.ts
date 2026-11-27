// « Sensibilité du micro » and its tester (ESC-665), in Réglages › Voix. The sensitivity is a step from -5 (less
// sensitive) to 5 (more), 0 being the detector as it always was; it moves the threshold over the room's noise, and the
// floor under which nothing is speech, as `vad::Levels` does on the backend (crates/escouade-voice-local/src/vad.rs).
// The line of the tester is drawn from here, from the noise the backend learnt and the slider's value: it moves the
// moment the slider does, before the backend's next level. « Voix détectée » is the backend's (its detector, at the
// sensitivity it was told). Nothing of the sound reaches the window: levels only.

import { api } from './ipc';
import type { VoiceEvent } from './types';

export const SENSITIVITY_MAX = 5;
const OVER_NOISE = 3.2;
const FLOOR = 0.005;
const STEP_DB = 1.5;
const MIN_OVER_NOISE = 2;

/** What a sensitivity makes of the detector (`vad::Levels::of`). */
export function levelsOf(sensitivity: number): { overNoise: number; floor: number } {
  const s = Math.max(-SENSITIVITY_MAX, Math.min(SENSITIVITY_MAX, Math.round(sensitivity)));
  if (s === 0) return { overNoise: OVER_NOISE, floor: FLOOR };
  const factor = 10 ** ((-s * STEP_DB) / 20);
  return { overNoise: Math.max(OVER_NOISE * factor, MIN_OVER_NOISE), floor: FLOOR * factor };
}

/** The level (RMS) over which a frame is speech, in a room of `noise`, at `sensitivity`. */
export function thresholdOf(noise: number, sensitivity: number): number {
  const l = levelsOf(sensitivity);
  return Math.max(noise * l.overNoise, l.floor);
}

/** A level in dBFS (an RMS of 1 is 0 dBFS). */
export const dbfs = (rms: number) => 20 * Math.log10(Math.max(rms, 1e-6));

/** The meter's scale: from -70 dBFS (a quiet room) to -10 (a voice close to the microphone). */
const LOW_DB = -70;
const HIGH_DB = -10;
/** Where a level sits on the meter, from 0 to 1. */
export const meterPos = (rms: number) => Math.max(0, Math.min(1, (dbfs(rms) - LOW_DB) / (HIGH_DB - LOW_DB)));

/** The value beside the slider: « Par défaut », « +2 », « −3 ». */
export const sensitivityText = (s: number, byDefault: string) => (s === 0 ? byDefault : s > 0 ? `+${s}` : `−${-s}`);

/** The tester: one for the app, as the microphone is. */
class MicMeter {
  /** The listening under way. */
  session = $state<number | null>(null);
  starting = $state(false);
  /** The loudest frame of the last moment, the room's noise learnt, and whether a sentence is heard. */
  level = $state(0);
  noise = $state(0);
  speech = $state(false);
  /** Whether a level came yet. */
  heard = $state(false);
  /** The slider's value, which draws the line. */
  sensitivity = $state(0);
  error = $state('');
  /** Stopped while it opened: the listening is let go as soon as it is open. */
  private called = false;

  readonly threshold = $derived(thresholdOf(this.noise, this.sensitivity));
  readonly running = $derived(this.session !== null || this.starting);

  /** « Tester »: listens to `device` (empty: the system's) as the conversation's detector would. */
  async start(device: string, sensitivity: number, silenceMs: number) {
    if (this.running) return;
    this.reset();
    this.sensitivity = sensitivity;
    this.starting = true;
    this.called = false;
    try {
      const id = await api.voiceMeterStart(device, sensitivity, silenceMs);
      if (this.called) {
        void api.voiceMeterStop(id).catch(() => {});
        return;
      }
      this.session = id;
      // Moved while it opened.
      if (this.sensitivity !== sensitivity) void api.voiceMeterSet(id, this.sensitivity).catch(() => {});
    } catch (e) {
      if (!this.called) this.error = String(e);
    } finally {
      this.starting = false;
    }
  }

  /** The slider moved: the line at once, the detector from its next frames. */
  set(sensitivity: number) {
    if (sensitivity === this.sensitivity) return;
    this.sensitivity = sensitivity;
    if (this.session !== null) void api.voiceMeterSet(this.session, sensitivity).catch(() => {});
  }

  /** « Arrêter », or the page left. */
  stop() {
    if (this.starting) this.called = true;
    const id = this.session;
    this.session = null;
    this.speech = false;
    if (id !== null) void api.voiceMeterStop(id).catch(() => {});
  }

  /** What the backend says of the tester. */
  take(e: VoiceEvent) {
    if (e.kind !== 'meter' && e.kind !== 'meterEnded') return;
    if (e.session !== this.session) return;
    if (e.kind === 'meterEnded') {
      this.session = null;
      this.speech = false;
      if (e.error) this.error = e.error;
      return;
    }
    this.level = e.level;
    this.noise = e.noise;
    this.speech = e.speech;
    this.heard = true;
  }

  /** Back to rest (the tests). */
  clear() {
    this.stop();
    this.starting = false;
    this.called = false;
    this.reset();
  }

  private reset() {
    this.level = 0;
    this.noise = 0;
    this.speech = false;
    this.heard = false;
    this.error = '';
  }
}

export const micMeter = new MicMeter();
