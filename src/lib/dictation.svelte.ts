// The dictation, for the whole window: one at a time, into the field it was started from (the composer, the terminal
// of the Claude Code view, a ticket's description). A short click (or press of the shortcut) starts it and the next
// stops it; a press held stops when it is let go (« appuyer pour parler »). The text is put where the field's cursor is;
// it is never sent by itself.

import { HOLD_MS } from './dictation';
import { t } from './i18n';
import { api } from './ipc';
import { app } from './state.svelte';
import type { VoiceEvent } from './types';

export type DictationKind = 'composer' | 'terminal' | 'ticket';

/** A field one dictates into. */
export interface DictationTarget {
  kind: DictationKind;
  /** Unique among the fields on screen: the dictation's panel shows beside the button of this one. */
  id: string;
  /** What holds the field: the shortcut dictates into the one that has the focus. */
  el: () => Element | null | undefined;
  /** Puts the text where the field's cursor is. */
  insert: (text: string) => void;
}

export type Phase = 'idle' | 'starting' | 'listening' | 'transcribing' | 'offer' | 'downloading';

class Dictation {
  phase = $state<Phase>('idle');
  /** How loud it is (0 to 1), and for how long it has listened (ms). */
  level = $state(0);
  ms = $state(0);
  /** What the service on line has understood so far, as one speaks (ESC-136): the final text replaces it in the field. */
  partial = $state('');
  /** The field being dictated into. */
  targetId = $state<string | null>(null);
  /** The model to download before the first dictation: what the panel asks. */
  offer = $state<{ model: string; size: number } | null>(null);
  progress = $state<{ model: string; done: number; total: number } | null>(null);
  private target: DictationTarget | null = null;
  private session: number | null = null;
  private targets: DictationTarget[] = [];
  /** The press under way that started the dictation, when it began (`performance.now()`). */
  private pressedAt: number | null = null;
  /** Let go after a long press while the listening was still starting: it stops as soon as it listens. */
  private stopOnceListening = false;

  /** A field on screen one may dictate into; the function returned takes it away (and stops its dictation). */
  register(target: DictationTarget): () => void {
    this.targets.push(target);
    return () => {
      this.targets = this.targets.filter((x) => x !== target);
      if (this.target === target) this.cancel();
    };
  }

  /** The field the shortcut dictates into: the one that has the focus, else the last one shown (the composer…). */
  focused(): DictationTarget | null {
    const active = document.activeElement;
    const live = this.targets.filter((x) => x.el()?.isConnected);
    const inside = live.filter((x) => active && x.el()?.contains(active));
    // The innermost: a ticket's form is above the composer. None has the focus: the active pane's (ESC-392), else the last shown.
    const inPane = live.filter((x) => x.el()?.closest('[data-pane-active="true"]'));
    return inside.at(-1) ?? inPane.at(-1) ?? live.at(-1) ?? null;
  }

  /** The button (or the shortcut) is pressed for `target`. */
  /** While Réglages › Voix records a new shortcut: the keys are its own. */
  recording = false;

  press(target: DictationTarget | null) {
    if (this.phase === 'listening') {
      // The second click of « un clic pour commencer, un pour finir ».
      this.pressedAt = null;
      void this.stop();
      return;
    }
    if (this.phase !== 'idle' || !target) return;
    this.pressedAt = performance.now();
    void this.start(target);
  }

  /** The button (or the shortcut) is let go: after a long press, it stops. */
  release() {
    if (this.pressedAt === null) return;
    const held = performance.now() - this.pressedAt >= HOLD_MS;
    this.pressedAt = null;
    if (!held) return;
    if (this.phase === 'listening') void this.stop();
    else if (this.phase === 'starting') this.stopOnceListening = true;
  }

  async start(target: DictationTarget) {
    this.phase = 'starting';
    this.target = target;
    this.targetId = target.id;
    this.level = 0;
    this.ms = 0;
    this.partial = '';
    this.stopOnceListening = false;
    try {
      const r = await api.voiceStart();
      if (this.target !== target || this.phase !== 'starting') {
        // Called off while it started (the field gone, Escape).
        if (r.kind === 'listening') await api.voiceCancel();
        return;
      }
      if (r.kind === 'needsModel') {
        this.pressedAt = null;
        this.offer = { model: r.model, size: r.size };
        this.phase = 'offer';
        return;
      }
      this.session = r.session;
      this.phase = 'listening';
      if (this.stopOnceListening) void this.stop();
    } catch (e) {
      this.reset();
      app.toast(String(e), 'error');
    }
  }

  /** Stops listening: the text heard goes into the field. */
  async stop() {
    if (this.phase !== 'listening' || this.session === null || !this.target) return;
    const target = this.target;
    const session = this.session;
    this.phase = 'transcribing';
    try {
      const text = await api.voiceStop(session, target.kind);
      if (this.target !== target) return;
      if (text.trim()) target.insert(text);
      else app.toast(t('voice.nothingHeard'), 'info');
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      if (this.target === target) this.reset();
    }
  }

  /** Escape, or the field gone: nothing is put anywhere. */
  cancel() {
    const was = this.phase;
    if (was === 'downloading' && this.progress) void api.voiceDownloadCancel(this.progress.model);
    if (was === 'starting' || was === 'listening' || was === 'transcribing') void api.voiceCancel().catch(() => {});
    this.reset();
  }

  /** « Télécharger » of the offer: the model comes, its progress shown; then a click dictates. */
  async download() {
    const offer = this.offer;
    if (!offer || this.phase !== 'offer') return;
    this.phase = 'downloading';
    this.progress = { model: offer.model, done: 0, total: offer.size };
    try {
      await api.voiceDownload(offer.model);
      if (this.phase !== 'downloading') return;
      app.toast(t('voice.modelReady'), 'ok');
    } catch (e) {
      if (this.phase === 'downloading') app.toast(t('voice.downloadFailed', { error: String(e) }), 'error');
    } finally {
      if (this.phase === 'downloading') this.reset();
    }
  }

  /** What the backend says while it listens and downloads. */
  take(e: VoiceEvent) {
    if (e.kind === 'download') {
      if (this.progress?.model === e.model) this.progress = { model: e.model, done: e.done, total: e.total };
      return;
    }
    if (e.kind !== 'level' && e.kind !== 'ended' && e.kind !== 'partial') return;
    if (e.session !== this.session) return;
    if (e.kind === 'partial') {
      if (this.phase === 'listening' || this.phase === 'transcribing') this.partial = e.text;
    } else if (e.kind === 'level') {
      this.level = e.level;
      this.ms = e.ms;
    } else {
      // Ended by itself: what was heard until then is still transcribed.
      if (e.error) app.toast(e.error, 'error');
      void this.stop();
    }
  }

  /** Back to rest, every field forgotten too (the tests). */
  clear() {
    this.reset();
    this.targets = [];
  }

  private reset() {
    this.phase = 'idle';
    this.target = null;
    this.targetId = null;
    this.session = null;
    this.offer = null;
    this.progress = null;
    this.level = 0;
    this.ms = 0;
    this.partial = '';
    this.pressedAt = null;
    this.stopOnceListening = false;
  }
}

export const dictation = new Dictation();
