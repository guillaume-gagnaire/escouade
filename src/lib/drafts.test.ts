import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { agent, fakeBackend, project, SETTINGS } from '../test/ipc';
import type { DraftAttachment } from './attachments';
import { SAVE_DELAY, forgetDraft, getDraft, setDraft } from './drafts';
import { readPref } from './prefs';
import { app } from './state.svelte';
import type { InitialState, UiEvent } from './types';

const file: DraftAttachment = { name: 'notes.txt', mediaType: 'text/plain', data: 'AAAA', kind: 'text', size: 3 };
const kept = (id: string) => readPref(`draft.${id}`);

describe('drafts', () => {
  beforeEach(() => {
    localStorage.clear();
    forgetDraft('a1');
    forgetDraft('a2');
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());

  it('keep the text 300 ms after the last keystroke, not before', () => {
    expect(SAVE_DELAY).toBe(300);
    setDraft('a1', { text: 'Ajoute', files: [] });
    vi.advanceTimersByTime(200);
    setDraft('a1', { text: 'Ajoute un test', files: [] });
    vi.advanceTimersByTime(299);
    expect(kept('a1')).toBeNull();
    vi.advanceTimersByTime(1);
    expect(kept('a1')).toBe('Ajoute un test');
  });

  it('come back after a restart, text only', async () => {
    setDraft('a1', { text: 'Pense aux tests', files: [file] });
    vi.advanceTimersByTime(SAVE_DELAY);
    // A new process: the memory is empty, the preferences are what is left.
    vi.resetModules();
    const fresh = await import('./drafts');
    expect(fresh.getDraft('a1')).toEqual({ text: 'Pense aux tests', files: [] });
  });

  it('are the memory’s while they are typed, attachments included', () => {
    setDraft('a1', { text: 'Regarde ça', files: [file] });
    expect(getDraft('a1')).toEqual({ text: 'Regarde ça', files: [file] });
    // Not yet kept: the memory is the draft.
    expect(kept('a1')).toBeNull();
  });

  it('are empty for an agent that never had one', () => {
    expect(getDraft('a2')).toEqual({ text: '', files: [] });
  });

  it('keep nothing of attachments alone', () => {
    setDraft('a1', { text: '', files: [file] });
    vi.advanceTimersByTime(SAVE_DELAY);
    expect(kept('a1')).toBeNull();
    expect(getDraft('a1').files).toEqual([file]);
  });

  it('are cleared at once when the text is emptied, as a message is sent', () => {
    setDraft('a1', { text: 'Un message', files: [] });
    vi.advanceTimersByTime(SAVE_DELAY);
    expect(kept('a1')).toBe('Un message');
    setDraft('a1', { text: '', files: [] });
    expect(kept('a1')).toBeNull();
    expect(getDraft('a1')).toEqual({ text: '', files: [] });
  });

  it('are not written after they were emptied, whatever was waiting to be saved', () => {
    setDraft('a1', { text: 'Un message', files: [] });
    setDraft('a1', { text: '', files: [] });
    vi.advanceTimersByTime(SAVE_DELAY * 2);
    expect(kept('a1')).toBeNull();
  });

  it('are kept apart for each agent', () => {
    setDraft('a1', { text: 'pour A', files: [] });
    setDraft('a2', { text: 'pour B', files: [] });
    vi.advanceTimersByTime(SAVE_DELAY);
    expect(kept('a1')).toBe('pour A');
    expect(kept('a2')).toBe('pour B');
  });

  it('go with the agent, even before they were written', () => {
    setDraft('a1', { text: 'perdu', files: [file] });
    forgetDraft('a1');
    vi.advanceTimersByTime(SAVE_DELAY);
    expect(kept('a1')).toBeNull();
    expect(getDraft('a1')).toEqual({ text: '', files: [] });
    setDraft('a2', { text: 'gardé puis retiré', files: [] });
    vi.advanceTimersByTime(SAVE_DELAY);
    forgetDraft('a2');
    expect(kept('a2')).toBeNull();
  });

  it('are written at once when the window is hidden or closed', () => {
    setDraft('a1', { text: 'Presque perdu', files: [] });
    window.dispatchEvent(new Event('pagehide'));
    expect(kept('a1')).toBe('Presque perdu');
  });

  describe('of an agent the backend removes', () => {
    async function start() {
      let channel: { onmessage: (e: UiEvent) => void } | null = null;
      const initial: InitialState = {
        projects: [project()],
        agents: [agent({ id: 'a1' }), agent({ id: 'a2', name: 'tests-e2e', createdAt: 2 })],
        ui: { activeProject: 'p1', view: 'project', selectedAgent: {} },
        settings: SETTINGS,
        usage: { fiveHour: null, sevenDay: null, todayCost: 0, updatedAt: 0 },
        git: {},
        shells: [],
        terminals: [],
        tickets: [],
        claudeFound: true,
        version: '0.1.0',
        models: [],
      };
      fakeBackend({
        subscribe: (args: any) => {
          channel = args.channel;
          return initial;
        },
        get_conversation: () => [],
      });
      await app.init();
      return (e: UiEvent) => channel!.onmessage(e);
    }

    it('are deleted with it, and only its own', async () => {
      const emit = await start();
      setDraft('a1', { text: 'reste', files: [] });
      setDraft('a2', { text: 'part', files: [file] });
      vi.advanceTimersByTime(SAVE_DELAY);
      emit({ type: 'agentRemoved', id: 'a2', projectId: 'p1' });
      expect(kept('a2')).toBeNull();
      expect(getDraft('a2')).toEqual({ text: '', files: [] });
      expect(kept('a1')).toBe('reste');
    });
  });
});
