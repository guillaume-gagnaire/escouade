import { describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { keyLabel, primaryKey } from './platform';

const key = (init: KeyboardEventInit) => new KeyboardEvent('keydown', init);

describe('platform', () => {
  it('uses Cmd on macOS and Ctrl elsewhere', () => {
    expect(primaryKey(key({ key: 'j', metaKey: true }), true)).toBe(true);
    expect(primaryKey(key({ key: 'j', ctrlKey: true }), true)).toBe(false);
    expect(primaryKey(key({ key: 'j', ctrlKey: true }), false)).toBe(true);
    expect(primaryKey(key({ key: 'j', metaKey: true }), false)).toBe(false);
  });

  it('writes shortcuts the macOS way', () => {
    expect(keyLabel('Ctrl+J', true)).toBe('⌘J');
    expect(keyLabel('Ctrl+Shift+F', true)).toBe('⇧⌘F');
    expect(keyLabel('Ctrl+Maj+L', true)).toBe('⇧⌘L');
    expect(keyLabel('Ctrl+Tab', true)).toBe('Ctrl+Tab');
    expect(keyLabel('Ctrl+J', false)).toBe('Ctrl+J');
  });

  it('names the keys in French, whichever way the shortcut is written', () => {
    expect(keyLabel('Ctrl+Shift+F', false)).toBe('Ctrl+Maj+F');
    expect(keyLabel('Ctrl+Enter', false)).toBe('Ctrl+Entrée');
    expect(keyLabel('Ctrl+Maj+Entrée', false)).toBe('Ctrl+Maj+Entrée');
    expect(keyLabel('Esc', false)).toBe('Échap');
    expect(keyLabel('Ctrl+Entrée', true)).toBe('⌘Entrée');
    expect(keyLabel('Ctrl+,', false)).toBe('Ctrl+,');
  });

  it('names the keys in English once the interface is', () => {
    setLang('en');
    expect(keyLabel('Ctrl+Maj+F', false)).toBe('Ctrl+Shift+F');
    expect(keyLabel('Ctrl+Entrée', false)).toBe('Ctrl+Enter');
    expect(keyLabel('Échap', false)).toBe('Esc');
    expect(keyLabel('Ctrl+Maj+Entrée', true)).toBe('⇧⌘Enter');
    expect(keyLabel('Alt+1', false)).toBe('Alt+1');
  });
});
