import { beforeEach, describe, expect, it } from 'vitest';
import { setLocks } from './locks.svelte';
import { dictationCut, overseerVoice, readable } from './overseer-voice.svelte';
import { app } from './state.svelte';
import type { ConvItem, VoiceEvent } from './types';
import { fakeBackend, resetApp } from '../test/ipc';

const ev = (e: Record<string, unknown>) => overseerVoice.take({ session: 9, ...e } as VoiceEvent);
const tick = () => new Promise((r) => setTimeout(r, 0));

const turn = (n: number): ConvItem =>
  ({
    kind: 'turn',
    id: `turn-${n}`,
    cost: 0,
    durationMs: 1,
    numTurns: 1,
    isError: false,
    interrupted: false,
  }) as unknown as ConvItem;

describe('the Overseer’s voice (ESC-468)', () => {
  beforeEach(() => {
    resetApp();
    setLocks(undefined);
  });

  async function talking(handlers: Record<string, (a: any) => unknown> = {}) {
    const b = fakeBackend({
      overseer_voice_start: () => ({ kind: 'listening', session: 9, notice: null }),
      conversation_say: () => 0,
      ...handlers,
    });
    await overseerVoice.press('ov');
    expect(overseerVoice.phase).toBe('talking');
    return b;
  }

  it('sends what was said as « dit », then reads its answer aloud, its sentences as subtitles', async () => {
    const b = await talking();
    ev({ kind: 'convPartial', text: 'pose le' });
    expect(overseerVoice.heard).toEqual({ text: 'pose le', live: true });
    // A click (short press) then a second one: sent.
    await overseerVoice.press('ov');
    expect(b.called('conversation_end')[0].args).toEqual({ session: 9, send: true });
    expect(overseerVoice.phase).toBe('sending');
    ev({ kind: 'convHeard', text: 'pose le tag', error: null });
    await tick();
    expect(b.called('conversation_send')[0].args).toEqual({ id: 'ov', text: 'pose le tag' });
    expect(overseerVoice.phase).toBe('waiting');
    // Its turn over: its passage is read.
    overseerVoice.observe([
      { kind: 'text', id: 'x', text: 'C’est **noté**.', streaming: false, spoken: 'C’est noté : je pose le tag.' },
      turn(1),
    ]);
    expect(overseerVoice.phase).toBe('speaking');
    expect(b.called('conversation_say')[0].args).toEqual({ session: 9, text: 'C’est noté : je pose le tag.' });
    await tick();
    ev({ kind: 'convSentence', reading: 0, text: 'C’est noté : je pose le tag.' });
    ev({ kind: 'convLevel', level: 0.1, played: 0.7 });
    expect(overseerVoice.played).toBe(0.7);
    // Read: the microphone hears nothing until the next press.
    ev({ kind: 'convSpoken', reading: 0, error: null, paused: false });
    expect(overseerVoice.phase).toBe('idle');
    expect(b.called('conversation_end').at(-1)?.args).toEqual({ session: 9, send: false });
  });

  it('is interrupted by the voice over it, and by a new press', async () => {
    const b = await talking({ conversation_say: () => 3 });
    overseerVoice.phase = 'speaking';
    ev({ kind: 'convBargeIn', reading: 3 });
    expect(overseerVoice.phase).toBe('talking');
    overseerVoice.phase = 'speaking';
    await overseerVoice.press('ov');
    expect(overseerVoice.phase).toBe('talking');
    expect(b.called('conversation_listen').at(-1)?.args).toEqual({ session: 9 });
  });

  it('says when nothing was heard', async () => {
    await talking();
    await overseerVoice.press('ov');
    ev({ kind: 'convHeard', text: '', error: null });
    await tick();
    expect(overseerVoice.phase).toBe('idle');
    expect(app.toasts.at(-1)?.text).toMatch(/entendu/);
  });

  it('offers Whisper’s model before it can listen', async () => {
    const b = fakeBackend({ overseer_voice_start: () => ({ kind: 'needsModel', model: 'base', size: 140_000_000 }) });
    await overseerVoice.press('ov');
    expect(overseerVoice.phase).toBe('offer');
    expect(overseerVoice.offer).toEqual({ kind: 'model', id: 'base', size: 140_000_000 });
    await overseerVoice.download();
    expect(b.called('voice_download')[0].args).toEqual({ model: 'base' });
    expect(overseerVoice.phase).toBe('off');
  });

  it('never starts while this device’s policy cuts the dictation', async () => {
    const b = fakeBackend();
    setLocks({
      locks: { 'voice.dictation': { key: 'voice.dictation', value: false, by: { kind: 'device', source: 'HKLM' }, bounds: [] } },
      rows: {},
      warnings: [],
    });
    expect(dictationCut()).toBe(true);
    await overseerVoice.press('ov');
    expect(overseerVoice.phase).toBe('off');
    expect(b.called('overseer_voice_start')).toHaveLength(0);
  });

  it('reads a text without its Markdown', () => {
    expect(readable('C’est **fait** : voir [ESC-31](https://x) et `main`.\n\n```sh\ngit push\n```')).toBe(
      'C’est fait : voir ESC-31 et main.',
    );
  });
});
