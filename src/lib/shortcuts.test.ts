import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { agent, fakeBackend, project, resetApp } from '../test/ipc';
import { buffers } from './editor/buffers.svelte';
import { answersHere, ariaEnter, enterAnswer, handleShortcut, isAppShortcut, optionAnswer } from './shortcuts';
import { app } from './state.svelte';

const key = (k: string, mods: Partial<KeyboardEventInit> = {}) => new KeyboardEvent('keydown', { key: k, ctrlKey: true, ...mods });

describe('handleShortcut', () => {
  beforeEach(() =>
    resetApp({ projects: [project(), project({ id: 'p2', name: 'studio' })], agents: [agent(), agent({ id: 'a2', createdAt: 2 })] }),
  );

  it('switches project with Ctrl+digit', () => {
    fakeBackend();
    expect(handleShortcut(key('2'))).toBe(true);
    expect(app.project?.id).toBe('p2');
    expect(handleShortcut(key('9'))).toBe(false);
  });

  it('switches project with Ctrl+digit on a French (AZERTY) keyboard too', () => {
    fakeBackend();
    // AZERTY: the digit row types & é " ' … without Shift; the physical key is still DigitN.
    expect(handleShortcut(key('é', { code: 'Digit2' }))).toBe(true);
    expect(app.project?.id).toBe('p2');
    expect(handleShortcut(key('&', { code: 'Digit1' }))).toBe(true);
    expect(app.project?.id).toBe('p1');
  });

  it('creates an agent with Ctrl+N', () => {
    const backend = fakeBackend({ create_agent: () => agent({ id: 'a3', createdAt: 3 }) });
    expect(handleShortcut(key('n'))).toBe(true);
    expect(backend.called('create_agent')).toEqual([{ cmd: 'create_agent', args: { projectId: 'p1', model: null } }]);
  });

  it('cycles agents with Ctrl+Tab and Ctrl+Shift+Tab', () => {
    fakeBackend();
    handleShortcut(key('Tab'));
    expect(app.agent?.id).toBe('a2');
    handleShortcut(key('Tab', { shiftKey: true }));
    expect(app.agent?.id).toBe('a1');
  });

  it('opens the settings with Ctrl+,', () => {
    fakeBackend();
    handleShortcut(key(','));
    expect(app.modal).toEqual({ kind: 'settings' });
  });

  it('opens the search through the conversations with Ctrl+K, Cmd+K on macOS', () => {
    fakeBackend();
    expect(handleShortcut(key('k'), false)).toBe(true);
    expect(app.modal).toEqual({ kind: 'convSearch' });
    app.modal = null;
    expect(handleShortcut(key('k'), true)).toBe(false);
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 'k', metaKey: true }), true)).toBe(true);
    expect(app.modal).toEqual({ kind: 'convSearch' });
  });

  it('switches the screen layout with Ctrl+Shift+L', () => {
    fakeBackend();
    expect(handleShortcut(key('L', { shiftKey: true }))).toBe(true);
    expect(app.split).toBe(true);
    handleShortcut(key('L', { shiftKey: true }));
    expect(app.split).toBe(false);
  });

  it('toggles the files panel with Ctrl+Shift+B, except in the split layout where it is always shown', () => {
    fakeBackend();
    expect(handleShortcut(key('B', { shiftKey: true }))).toBe(true);
    expect(app.filesOpen).toBe(true);
    app.toggleLayout();
    expect(handleShortcut(key('B', { shiftKey: true }))).toBe(false);
    expect(app.filesOpen).toBe(true);
  });

  it('does nothing behind an open dialog', () => {
    const backend = fakeBackend();
    app.modal = { kind: 'newProject' };
    expect(handleShortcut(key('n'))).toBe(false);
    expect(handleShortcut(key('2'))).toBe(false);
    expect(backend.called('create_agent')).toHaveLength(0);
    expect(app.project?.id).toBe('p1');
  });

  it('ignores keys without Ctrl, and Ctrl+Alt (AltGr on French keyboards)', () => {
    fakeBackend();
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 'n' }))).toBe(false);
    expect(handleShortcut(key('2', { altKey: true }))).toBe(false);
  });

  it('uses Cmd on macOS, and keeps Ctrl+Tab (Cmd+Tab switches applications)', () => {
    fakeBackend();
    const cmd = (k: string, mods: Partial<KeyboardEventInit> = {}) => new KeyboardEvent('keydown', { key: k, metaKey: true, ...mods });
    expect(handleShortcut(key('2'), true)).toBe(false);
    expect(handleShortcut(cmd('2'), true)).toBe(true);
    expect(app.project?.id).toBe('p2');
    handleShortcut(cmd(','), true);
    expect(app.modal?.kind).toBe('settings');
    app.modal = null;
    app.selectProject('p1');
    app.selectAgent('a1');
    expect(handleShortcut(key('Tab'), true)).toBe(true);
    expect(app.agent?.id).toBe('a2');
  });

  it('saves the file open in the editor with Ctrl+S', async () => {
    resetApp();
    const be = fakeBackend({
      fs_read: () => ({ kind: 'text', text: 'a', size: 1, hash: 'h1', eol: 'lf', bom: false }),
      fs_base: () => null,
      fs_write: () => 'h2',
      set_unsaved: () => null,
    });
    await app.openEditor({ source: 'project', path: 'a.ts' });
    const b = await buffers.open('p1', 'project', 'a.ts');
    buffers.edit(b.key, 'b');
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }), false)).toBe(true);
    await expect.poll(() => be.called('fs_write').length).toBe(1);
  });

  it('leaves Ctrl+S alone when the statistics hide the editor', async () => {
    resetApp();
    const be = fakeBackend({
      fs_read: () => ({ kind: 'text', text: 'a', size: 1, hash: 'h1', eol: 'lf', bom: false }),
      fs_base: () => null,
      fs_write: () => 'h2',
      set_unsaved: () => null,
    });
    await app.openEditor({ source: 'project', path: 'a.ts' });
    const b = await buffers.open('p1', 'project', 'a.ts');
    buffers.edit(b.key, 'b');
    app.ui.view = 'stats';
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }), false)).toBe(false);
    expect(be.called('fs_write')).toHaveLength(0);
  });

  it('leaves Ctrl+S alone without the editor', () => {
    resetApp();
    expect(handleShortcut(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }), false)).toBe(false);
  });
});

describe('isAppShortcut', () => {
  it('lets navigation shortcuts through the terminal but leaves shell keys to the shell', () => {
    expect(isAppShortcut(key('3'))).toBe(true);
    expect(isAppShortcut(key('"', { code: 'Digit3' }))).toBe(true);
    expect(isAppShortcut(key('Tab'))).toBe(true);
    expect(isAppShortcut(key('j'))).toBe(true);
    expect(isAppShortcut(key(','))).toBe(true);
    expect(isAppShortcut(key('c'))).toBe(false); // SIGINT
    expect(isAppShortcut(key('r'))).toBe(false); // reverse search
    expect(isAppShortcut(key('n'))).toBe(false); // readline: next history
    expect(isAppShortcut(key('k'), false)).toBe(false); // readline: kill to the end of the line
    expect(isAppShortcut(new KeyboardEvent('keydown', { key: '3' }))).toBe(false);
  });

  it('hands every Cmd shortcut to the app on macOS but copy and paste, and leaves Ctrl keys to the shell', () => {
    const cmd = (k: string) => new KeyboardEvent('keydown', { key: k, metaKey: true });
    for (const k of ['3', 'n', 't', 'j', ',', 'k']) expect(isAppShortcut(cmd(k), true)).toBe(true);
    expect(isAppShortcut(cmd('c'), true)).toBe(false);
    expect(isAppShortcut(cmd('v'), true)).toBe(false);
    expect(isAppShortcut(key('j'), true)).toBe(false);
    expect(isAppShortcut(key('k'), true)).toBe(false);
    expect(isAppShortcut(key('Tab'), true)).toBe(true);
  });

  it('leaves the keys of the editor’s history to the shell and the composer: only the editor takes them', () => {
    resetApp();
    const alt = (k: string) => new KeyboardEvent('keydown', { key: k, altKey: true });
    for (const e of [alt('ArrowLeft'), alt('ArrowRight')]) {
      expect(isAppShortcut(e, false)).toBe(false);
      expect(handleShortcut(e, false)).toBe(false);
    }
    // macOS: Ctrl+- and Ctrl+Shift+-.
    for (const e of [key('-'), key('_', { shiftKey: true })]) {
      expect(isAppShortcut(e, true)).toBe(false);
      expect(handleShortcut(e, true)).toBe(false);
    }
  });
});

describe('keys that answer a waiting request', () => {
  const mods = (k: string, init: KeyboardEventInit) => new KeyboardEvent('keydown', { key: k, ...init });

  it('answers with Ctrl+Enter, and "always" with Ctrl+Shift+Enter', () => {
    expect(enterAnswer(key('Enter'))).toBe('plain');
    expect(enterAnswer(key('Enter', { shiftKey: true }))).toBe('shift');
  });

  it('answers with Cmd+Enter on macOS, where Ctrl+Enter stays the shell’s', () => {
    expect(enterAnswer(mods('Enter', { metaKey: true }), true)).toBe('plain');
    expect(enterAnswer(mods('Enter', { metaKey: true, shiftKey: true }), true)).toBe('shift');
    expect(enterAnswer(key('Enter'), true)).toBeNull();
    expect(enterAnswer(mods('Enter', { metaKey: true }), false)).toBeNull();
  });

  it('leaves Enter alone, plain or with Shift or Alt, and every other key', () => {
    expect(enterAnswer(mods('Enter', {}))).toBeNull();
    expect(enterAnswer(mods('Enter', { shiftKey: true }))).toBeNull();
    expect(enterAnswer(key('Enter', { altKey: true }))).toBeNull(); // Ctrl+Alt is AltGr
    expect(enterAnswer(key('j'))).toBeNull();
  });

  it('ignores an IME confirming its composition and a key held down', () => {
    expect(enterAnswer(key('Enter', { isComposing: true }))).toBeNull();
    expect(enterAnswer(key('Enter', { repeat: true }))).toBeNull();
  });

  it('picks an option with Alt+digit, from the physical key on an AZERTY keyboard too', () => {
    expect(optionAnswer(mods('1', { altKey: true, code: 'Digit1' }))).toBe(1);
    expect(optionAnswer(mods('9', { altKey: true, code: 'Digit9' }))).toBe(9);
    expect(optionAnswer(mods('é', { altKey: true, code: 'Digit2' }))).toBe(2);
    expect(optionAnswer(mods('3', { altKey: true }))).toBe(3); // no physical key: the character
  });

  it('does not take AltGr (Ctrl+Alt), Cmd, Shift, a held key, 0 or the numpad', () => {
    expect(optionAnswer(mods('~', { altKey: true, ctrlKey: true, code: 'Digit2' }))).toBeNull();
    expect(optionAnswer(mods('1', { altKey: true, metaKey: true, code: 'Digit1' }))).toBeNull();
    expect(optionAnswer(mods('1', { altKey: true, shiftKey: true, code: 'Digit1' }))).toBeNull();
    expect(optionAnswer(mods('1', { altKey: true, repeat: true, code: 'Digit1' }))).toBeNull();
    expect(optionAnswer(mods('1', { code: 'Digit1' }))).toBeNull();
    expect(optionAnswer(mods('0', { altKey: true, code: 'Digit0' }))).toBeNull();
    expect(optionAnswer(mods('1', { altKey: true, code: 'Numpad1' }))).toBeNull(); // Alt+numpad types a character by code
  });

  it('writes them for assistive technologies', () => {
    expect(ariaEnter(false, false)).toBe('Control+Enter');
    expect(ariaEnter(true, false)).toBe('Control+Shift+Enter');
    expect(ariaEnter(true, true)).toBe('Meta+Shift+Enter');
  });
});

describe('answersHere', () => {
  beforeEach(() => resetApp());
  afterEach(() => (document.body.innerHTML = ''));

  const mount = (html: string) => {
    document.body.innerHTML = html;
    return document.body.querySelector('#t');
  };

  it('answers from the conversation and its message field', () => {
    expect(answersHere(mount('<main class="conv"><div><textarea id="t"></textarea></div></main>'))).toBe(true);
    expect(answersHere(mount('<main class="conv"><button id="t">Autoriser</button></main>'))).toBe(true);
  });

  it('answers when nothing has the focus', () => {
    expect(answersHere(document.body)).toBe(true);
  });

  it('keeps out of a terminal and of the editor', () => {
    expect(answersHere(mount('<div class="xterm"><textarea id="t" class="xterm-helper-textarea"></textarea></div>'))).toBe(false);
    expect(answersHere(mount('<div class="cm-editor"><div id="t" class="cm-content"></div></div>'))).toBe(false);
    // Even when one is shown beside the conversation.
    expect(answersHere(mount('<main class="conv"><div class="xterm"><textarea id="t"></textarea></div></main>'))).toBe(false);
  });

  it('keeps out of the rest of the window', () => {
    expect(answersHere(mount('<nav><button id="t">Nouvel agent</button></nav><main class="conv"></main>'))).toBe(false);
    expect(answersHere(null)).toBe(false);
  });

  it('keeps out of the way of a dialog', () => {
    app.modal = { kind: 'settings' };
    expect(answersHere(mount('<main class="conv"><textarea id="t"></textarea></main>'))).toBe(false);
    expect(answersHere(document.body)).toBe(false);
  });
});
