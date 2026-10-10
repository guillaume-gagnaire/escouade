import { bracketMatching } from '@codemirror/language';
import { unifiedMergeView } from '@codemirror/merge';
import { EditorState, type Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';
import { setLang } from '../i18n';
import { editorTheme, phrases } from './theme';

// jsdom has no cascade, so compare the rules CodeMirror injects: our rule must be at least as specific as
// the base theme's one (a class counts for as much as another) and come after it, or the base wins.
function rules(extensions: Extension = bracketMatching()): { selectors: string[]; body: string; at: number }[] {
  const view = new EditorView({
    state: EditorState.create({ doc: 'a', extensions: [editorTheme, extensions] }),
    parent: document.body,
  });
  const css = [...document.querySelectorAll('style')].map((s) => s.textContent ?? '').join('\n');
  view.destroy();
  return [...css.matchAll(/([^{}]+)\{([^}]*)\}/g)].map((m) => ({
    selectors: m[1].split(',').map((s) => s.trim()),
    body: m[2],
    at: m.index,
  }));
}
const specificity = (selector: string) => (selector.match(/\.[^\s.>:,+~]+/g) ?? []).length;
const strongest = (r: { selectors: string[] }, within: RegExp) => Math.max(...r.selectors.filter((s) => within.test(s)).map(specificity));

describe('editorTheme', () => {
  it('tints the focused selection with the accent, over the base theme', () => {
    const all = rules();
    const ours = all.find((r) => r.body.includes('var(--accent) 28%'))!;
    const base = all.filter(
      (r) =>
        /cm-focused > \.cm-scroller > \.cm-selectionLayer \.cm-selectionBackground/.test(r.selectors.join()) && !r.body.includes('var('),
    );
    const focused = /cm-focused > \.cm-scroller > \.cm-selectionLayer \.cm-selectionBackground/;
    expect(base.length).toBeGreaterThan(0);
    for (const b of base) {
      expect(strongest(ours, focused)).toBeGreaterThanOrEqual(strongest(b, focused));
      expect(ours.at).toBeGreaterThan(b.at);
    }
  });

  it('styles the matching bracket while focused, over the base theme', () => {
    const all = rules();
    const ours = all.find((r) => r.body.includes('var(--elev2)'))!;
    const base = all.find((r) => r.selectors.some((s) => /\.cm-focused \.cm-matchingBracket/.test(s)) && !r.body.includes('var('))!;
    const focused = /\.cm-focused \.cm-matchingBracket/;
    expect(strongest(ours, focused)).toBeGreaterThanOrEqual(strongest(base, focused));
    expect(ours.at).toBeGreaterThan(base.at);
  });

  it('colors the changes shown in the text as the gutter marks them, over the merge view’s own theme', () => {
    const all = rules(unifiedMergeView({ original: 'b' }));
    // A value only our rule has, the elements both rules style, and a property the merge view's rules set there.
    const cases: [string, RegExp, string][] = [
      ['var(--add) 8%', /\.cm-changedLine$/, 'background'],
      ['var(--add) 24%', /\.cm-changedText$/, 'background'],
      ['var(--del) 8%', /\.cm-deletedChunk$/, 'background'],
      ['var(--del) 28%', /\.cm-deletedText$/, 'background'],
      ['font-size: 11px', /\.cm-deletedChunk (button|\.cm-blockAction)$/, 'color'],
    ];
    for (const [mark, within, property] of cases) {
      const ours = all.find((r) => r.body.includes(mark) && r.selectors.some((s) => within.test(s)));
      expect(ours, mark).toBeDefined();
      const base = all.filter((r) => r.selectors.some((s) => within.test(s)) && !r.body.includes('var(') && r.body.includes(property));
      expect(base.length, mark).toBeGreaterThan(0);
      for (const b of base) {
        expect(strongest(ours!, within), `${mark} over ${b.selectors.join()}`).toBeGreaterThanOrEqual(strongest(b, within));
        expect(ours!.at, `${mark} after ${b.selectors.join()}`).toBeGreaterThan(b.at);
      }
    }
  });
});

describe('phrases', () => {
  it('has all the texts the merge view asks for in French', () => {
    const merge = readFileSync(createRequire(import.meta.url).resolve('@codemirror/merge'), 'utf8');
    const asked = new Set([...merge.matchAll(/\.phrase\("([^"]+)"/g)].map((m) => m[1]));
    expect(asked.size).toBeGreaterThan(0);
    expect([...asked].filter((p) => !(p in phrases()))).toEqual([]);
    const state = EditorState.create({ extensions: EditorState.phrases.of(phrases()) });
    expect(state.phrase('$ unchanged lines', 12)).toBe('12 lignes inchangées');
  });

  it('says them in the language of the interface, whenever it is asked', () => {
    setLang('en');
    const state = EditorState.create({ extensions: EditorState.phrases.of(phrases()) });
    expect(state.phrase('$ unchanged lines', 12)).toBe('12 unchanged lines');
    expect(state.phrase('Find')).toBe('Find');
    expect(state.phrase('Revert this chunk')).toBe('Revert this block');
  });
});
