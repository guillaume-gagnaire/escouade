// The Overseer's voice (ESC-468, design/overseer/voix-tu-parles.png and voix-il-repond.png): the voice conversation of
// ESC-97 (`voice::conversation`: the microphone, Whisper or the service on line, the voice that reads, the barge-in),
// led by its screen as « appuyer pour parler ». Space held (the field empty) or the microphone button: it listens, the
// sentence written as it is said; let go, it is sent (« dit »); Escape drops it. Its answer is read aloud, its sentences
// as subtitles; speaking (Space, or the voice over it) or typing interrupts it. Between two, the microphone hears
// nothing. Cut, with the dictation, by this device's policy (TS-12).

import { turnOver, turnsIn } from './conversation';
import { conversationOf } from './conversations.svelte';
import { HOLD_MS } from './dictation';
import { t } from './i18n';
import { api } from './ipc';
import { locks } from './locks.svelte';
import { app } from './state.svelte';
import type { ConvItem, VoiceEvent } from './types';

export type VoicePhase =
  /** No session: the microphone closed. */
  | 'off'
  | 'starting'
  /** Whisper's model or the voice to download first. */
  | 'offer'
  | 'downloading'
  /** A session, the microphone hearing nothing. */
  | 'idle'
  /** The user speaks: Space held, or the button. */
  | 'talking'
  /** Let go: what was said is being transcribed. */
  | 'sending'
  /** Sent: the Overseer works on it. */
  | 'waiting'
  /** Its answer is read aloud. */
  | 'speaking';

/** The policy's key of the dictation (`escouade_policy::list::DICTATION`). */
export const DICTATION_KEY = 'voice.dictation';

/** Whether this device's policy turns the dictation off, and with it the Overseer's voice. */
export function dictationCut(): boolean {
  return locks.current.locks[DICTATION_KEY]?.value === false;
}

/** A text as said aloud: its Markdown taken off (marks, links, code), on one line. */
export function readable(text: string): string {
  return text
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/`([^`]*)`/g, '$1')
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[*_#>]+/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

class OverseerVoice {
  phase = $state<VoicePhase>('off');
  /** How loud the microphone is, and the voice played (0 to 1): what the orb follows. */
  level = $state(0);
  played = $state(0);
  /** What the user says, as it is said (`live`), or said; null once sent. */
  heard = $state<{ text: string; live: boolean } | null>(null);
  /** The sentence of its answer being read: the subtitle under the orb. */
  subtitle = $state<string | null>(null);
  /** What is to be downloaded before it can talk. */
  offer = $state<{ kind: 'model' | 'voice'; id: string; size: number } | null>(null);
  private session: number | null = null;
  private agentId: string | null = null;
  /** The reading under way (its number, -1 while not known yet). */
  private reading: number | null = null;
  /** Its conversation's turns when the sentence was sent: one more is its answer. */
  private turns: number | null = null;
  /** Let go while it was still starting: it sends as soon as it listens. */
  private letGo = false;
  /** When the press that started it began: a short click starts, the next one sends; a press held sends when let go. */
  private pressedAt: number | null = null;

  /** Space held, or the button pressed, for the Overseer `agentId`. */
  async press(agentId: string) {
    if (dictationCut()) {
      app.toast(t('overseer.voice.cut'), 'error');
      return;
    }
    if (this.phase === 'talking') {
      // The second click of « un clic pour commencer, un pour finir ».
      this.pressedAt = null;
      return this.send();
    }
    this.pressedAt = performance.now();
    this.agentId = agentId;
    this.heard = { text: '', live: true };
    if (this.session !== null && (this.phase === 'idle' || this.phase === 'speaking' || this.phase === 'waiting')) {
      // Speaking over its answer interrupts it; the microphone listens.
      if (this.phase === 'speaking') this.hush();
      this.phase = 'talking';
      this.turns = null;
      await api.conversationListen(this.session);
      return;
    }
    if (this.phase !== 'off') return;
    this.phase = 'starting';
    this.letGo = false;
    try {
      const r = await api.overseerVoiceStart();
      if (this.phase !== 'starting') {
        if (r.kind === 'listening') void api.conversationStop();
        return;
      }
      if (r.kind === 'needsModel' || r.kind === 'needsVoice') {
        this.offer = r.kind === 'needsModel' ? { kind: 'model', id: r.model, size: r.size } : { kind: 'voice', id: r.voice, size: r.size };
        this.phase = 'offer';
        this.heard = null;
        return;
      }
      if (r.kind === 'unavailable') {
        app.toast(t('voice.conversation.unavailable'), 'error');
        return this.reset();
      }
      this.session = r.session;
      if (r.notice) app.toast(r.notice);
      this.phase = 'talking';
      if (this.letGo) void this.send();
    } catch (e) {
      this.reset();
      app.toast(String(e), 'error');
    }
  }

  /** Space let go, or the button: after a press held, what was said goes; a short click keeps listening. */
  release() {
    if (this.pressedAt === null) return;
    const held = performance.now() - this.pressedAt >= HOLD_MS;
    this.pressedAt = null;
    if (!held) return;
    if (this.phase === 'starting') this.letGo = true;
    else if (this.phase === 'talking') void this.send();
  }

  /** What was said goes: transcribed at once, without waiting for its silence. */
  private async send() {
    if (this.phase !== 'talking' || this.session === null) return;
    this.phase = 'sending';
    await api.conversationEnd(this.session, true);
  }

  /** Escape while it listens: nothing goes. While it reads: it stops. */
  cancel() {
    if (this.session === null) return this.reset();
    if (this.phase === 'talking' || this.phase === 'sending' || this.phase === 'speaking') {
      this.hush();
      this.heard = null;
      this.phase = 'idle';
    }
  }

  /** Typing interrupts its answer read aloud. */
  interrupt() {
    if (this.phase !== 'speaking') return;
    this.hush();
    this.phase = 'idle';
  }

  /** The reading under way stops, what is being said is dropped, the microphone hears nothing. */
  private hush() {
    this.reading = null;
    this.subtitle = null;
    this.played = 0;
    if (this.session !== null) void api.conversationEnd(this.session, false);
  }

  /** « Télécharger » of the offer: the model or the voice comes; the next press talks. */
  async download() {
    const offer = this.offer;
    if (!offer || this.phase !== 'offer') return;
    this.phase = 'downloading';
    try {
      if (offer.kind === 'model') await api.voiceDownload(offer.id);
      else await api.speechDownload(offer.id);
      if (this.phase === 'downloading') this.reset();
    } catch (e) {
      if (this.phase === 'downloading') {
        app.toast(t('voice.downloadFailed', { error: String(e) }), 'error');
        this.reset();
      }
    }
  }

  /** The screen closed: the microphone too. */
  stop() {
    if (this.phase === 'off') return;
    if (this.session !== null || this.phase === 'starting') void api.conversationStop().catch(() => {});
    this.reset();
  }

  /** What the backend says of its session. */
  take(e: VoiceEvent) {
    if (!('session' in e) || this.session === null || e.session !== this.session) return;
    switch (e.kind) {
      case 'convLevel':
        this.level = e.level;
        this.played = e.played;
        break;
      case 'convPartial':
        if ((this.phase === 'talking' || this.phase === 'sending') && e.text.trim()) this.heard = { text: e.text, live: true };
        break;
      case 'convSpeech':
        // The voice over its answer (a barge-in heard by itself): it listens, the sentence ends at its silence.
        if (this.phase === 'idle' || this.phase === 'waiting') this.phase = 'talking';
        break;
      case 'convBargeIn':
        this.reading = null;
        this.subtitle = null;
        this.heard = { text: '', live: true };
        this.phase = 'talking';
        break;
      case 'convHeard':
        if (e.error) app.toast(e.error, 'error');
        void this.heardSaid(e.text);
        break;
      case 'convSentence':
        if (this.reading !== null && (this.reading === -1 || this.reading === e.reading)) this.subtitle = e.text;
        break;
      case 'convSpoken':
        if (this.reading === null || (this.reading !== -1 && this.reading !== e.reading)) break;
        if (e.error) app.toast(e.error, 'error');
        // Read: the microphone hears nothing until the next press (it would listen again by itself).
        this.hush();
        this.phase = 'idle';
        break;
      case 'convEnded':
        if (e.error) app.toast(e.error, 'error');
        this.reset();
        break;
    }
  }

  /** A sentence heard: sent to the Overseer as said; nothing heard, said so. */
  private async heardSaid(text: string) {
    const agentId = this.agentId;
    if (this.session === null || !agentId) return;
    if (this.phase !== 'sending' && this.phase !== 'talking') return;
    if (!text.trim()) {
      this.heard = null;
      this.phase = 'idle';
      app.toast(t('voice.nothingHeard'), 'info');
      return;
    }
    this.heard = { text, live: false };
    this.turns = turnsIn(conversationOf(agentId).items);
    this.phase = 'waiting';
    try {
      await api.conversationSend(agentId, text);
      this.heard = null;
    } catch (e) {
      this.turns = null;
      this.heard = null;
      this.phase = 'idle';
      app.toast(String(e), 'error');
    }
  }

  /** Its conversation as it changes (the screen's effect): the end of its turn has its answer read aloud. */
  observe(items: ConvItem[]) {
    if (this.phase !== 'waiting' || this.turns === null || this.session === null) return;
    const over = turnOver(items, this.turns);
    if (!over) return;
    this.turns = null;
    const last = [...items].reverse().find((i) => i.kind === 'text' && !i.parent && i.text.trim());
    const text = readable(over.spoken ?? (last?.kind === 'text' ? last.text : ''));
    if (!text) {
      this.phase = 'idle';
      return;
    }
    this.phase = 'speaking';
    this.reading = -1;
    const session = this.session;
    api
      .conversationSay(session, text)
      .then((n) => {
        if (this.reading === -1 && this.session === session) this.reading = n;
      })
      .catch((e) => {
        app.toast(String(e), 'error');
        if (this.session === session) this.phase = 'idle';
      });
  }

  private reset() {
    this.phase = 'off';
    this.session = null;
    this.agentId = null;
    this.reading = null;
    this.turns = null;
    this.letGo = false;
    this.pressedAt = null;
    this.offer = null;
    this.level = 0;
    this.played = 0;
    this.heard = null;
    this.subtitle = null;
  }

  /** Back to rest (the tests). */
  clear() {
    this.reset();
  }
}

export const overseerVoice = new OverseerVoice();
