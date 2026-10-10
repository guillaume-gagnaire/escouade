import { beforeEach, describe, expect, it } from 'vitest';
import { readTreeWidth, TREE_DEFAULT, TREE_MIN, treeMax, writeTreeWidth } from './layout';

describe('the left column of the editor', () => {
  beforeEach(() => localStorage.clear());

  it('is 240 px wide until it is resized', () => {
    expect(TREE_DEFAULT).toBe(240);
    expect(readTreeWidth()).toBe(240);
  });

  it('keeps its width in the preferences, the same for all projects', () => {
    writeTreeWidth(312);
    expect(localStorage.getItem('escouade.editor.treeWidth')).toBe('312');
    expect(readTreeWidth()).toBe(312);
  });

  it('takes the usual width back for a preference that is not a width', () => {
    for (const bad of ['', 'large', '-40', '0', 'NaN', 'Infinity']) {
      localStorage.setItem('escouade.editor.treeWidth', bad);
      expect(readTreeWidth()).toBe(240);
    }
  });

  it('rounds a width that is not whole', () => {
    localStorage.setItem('escouade.editor.treeWidth', '300.6');
    expect(readTreeWidth()).toBe(301);
  });

  it('is at most half of the editor area, and at least 160 px', () => {
    expect(TREE_MIN).toBe(160);
    expect(treeMax(1000)).toBe(500);
    expect(treeMax(801)).toBe(400);
    expect(treeMax(300)).toBe(160);
    expect(treeMax(0)).toBe(160);
  });
});
