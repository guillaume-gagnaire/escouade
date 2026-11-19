import { describe, expect, it } from 'vitest';
import { appTakes, utf16Column } from './keys';

describe('appTakes', () => {
  it('names the keys the app keeps in the view, as VS Code writes them', () => {
    for (const label of [
      'ctrl+p',
      'ctrl+1',
      'ctrl+9',
      'ctrl+n',
      'ctrl+shift+n',
      'ctrl+j',
      'ctrl+t',
      'ctrl+,',
      'ctrl+shift+a',
      'ctrl+shift+b',
      'ctrl+\\',
    ])
      expect(appTakes(label, 'any.command', false), label).toBe(true);
    expect(appTakes('shift+cmd+n', 'x', true)).toBe(true);
    expect(appTakes('cmd+p', 'x', true)).toBe(true);
    // The spaces' keys (ESC-539).
    expect(appTakes('alt+2', 'x', false)).toBe(true);
    expect(appTakes('ctrl+`', 'workbench.action.terminal.toggleTerminal', false)).toBe(true);
    expect(appTakes('ctrl+3', 'x', true)).toBe(true);
    expect(appTakes('alt+3', 'x', true)).toBe(false);
  });

  it('leaves VS Code the others, its chords, and its keys of another modifier', () => {
    for (const label of [
      'ctrl+s',
      'ctrl+shift+p',
      'ctrl+k',
      'ctrl+k ctrl+c',
      'ctrl+tab',
      'ctrl+alt+n',
      'ctrl+shift+l',
      'f1',
      'alt+shift+1',
      'ctrl+e',
    ])
      expect(appTakes(label, 'any.command', false), label).toBe(false);
    // On macOS, Ctrl is not the primary key.
    expect(appTakes('ctrl+p', 'x', true)).toBe(false);
    expect(appTakes(undefined, 'x', false)).toBe(false);
  });

  it('takes VS Code’s zoom whatever its key: the app zooms the whole window', () => {
    expect(appTakes('ctrl+numpad_add', 'workbench.action.zoomIn', false)).toBe(true);
    expect(appTakes(null, 'workbench.action.zoomReset', false)).toBe(true);
  });
});

describe('utf16Column', () => {
  it('counts the characters of the links as VS Code counts its columns', () => {
    expect(utf16Column('const a = 1;', 1)).toBe(0);
    expect(utf16Column('const a = 1;', 7)).toBe(6);
    // An emoji is one character, two UTF-16 units.
    expect(utf16Column('😀 = x', 3)).toBe(3);
    // Past the end: the end.
    expect(utf16Column('ab', 9)).toBe(2);
  });
});
