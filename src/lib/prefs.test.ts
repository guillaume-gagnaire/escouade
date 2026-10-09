import { beforeEach, describe, expect, it } from 'vitest';
import { readPref, removePref, writePref } from './prefs';

describe('preferences', () => {
  beforeEach(() => localStorage.clear());

  it('are kept under the escouade prefix', () => {
    writePref('diffSplit', '1');
    expect(localStorage.getItem('escouade.diffSplit')).toBe('1');
    expect(readPref('diffSplit')).toBe('1');
  });

  it('saved under the former ccm prefix are taken over, once', () => {
    localStorage.setItem('ccm.statsRange', 'month');
    expect(readPref('statsRange')).toBe('month');
    expect(localStorage.getItem('escouade.statsRange')).toBe('month');
    expect(localStorage.getItem('ccm.statsRange')).toBeNull();
  });

  it('under the new prefix win over a leftover old one', () => {
    localStorage.setItem('ccm.statsRange', 'month');
    localStorage.setItem('escouade.statsRange', 'week');
    expect(readPref('statsRange')).toBe('week');
  });

  it('removed leave neither the current key nor a former one to come back from', () => {
    writePref('draft.a1', 'texte');
    localStorage.setItem('ccm.draft.a1', 'ancien');
    removePref('draft.a1');
    expect(localStorage.getItem('escouade.draft.a1')).toBeNull();
    expect(localStorage.getItem('ccm.draft.a1')).toBeNull();
    expect(readPref('draft.a1')).toBeNull();
  });

  it('never set are empty', () => {
    expect(readPref('nothing')).toBeNull();
  });
});
