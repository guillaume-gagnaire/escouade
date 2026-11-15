import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { paint, parse, ratio, substitute, tokensOf, type Rgba } from '../../test/color';
import { BRAND, PROJECT_COLORS } from '../theme';
import {
  COLORS,
  SYNTAX,
  THEME_IDS,
  TOKENS,
  cssToHex,
  isDark,
  syntaxRules,
  themeSettings,
  vscodeColors,
  vscodeTheme,
  type Tokens,
} from './theme';

const fake = Object.fromEntries(TOKENS.map((t, i) => [t, `#${(i + 1).toString(16).padStart(6, '0')}ff`])) as Tokens;

// The app's own tokens (src/app.css), in both themes, for every project colour: what the view is given.
const CSS = readFileSync('src/app.css', 'utf8');
const DARK = tokensOf(CSS, ':root');
const LIGHT = { ...DARK, ...tokensOf(CSS, ":root[data-theme='light']") };
const hex = (c: Rgba) =>
  '#' +
  [c.r * 255, c.g * 255, c.b * 255, c.a * 255]
    .map((x) =>
      Math.round(Math.min(255, Math.max(0, x)))
        .toString(16)
        .padStart(2, '0'),
    )
    .join('');
function appTokens(theme: Record<string, string>, p: string): Tokens {
  const t = { ...theme, '--p': p };
  return Object.fromEntries(TOKENS.map((k) => [k, hex(parse(substitute(`var(--${k})`, t)))]));
}
const THEMES = { dark: DARK, light: LIGHT };
const TINTS = [...PROJECT_COLORS, BRAND];

/** A `#rrggbbaa` as a layer. */
const layer = (h: string): Rgba => ({
  r: parseInt(h.slice(1, 3), 16) / 255,
  g: parseInt(h.slice(3, 5), 16) / 255,
  b: parseInt(h.slice(5, 7), 16) / 255,
  a: h.length >= 9 ? parseInt(h.slice(7, 9), 16) / 255 : 1,
});
/** The contrast of `ink` over the stack `under` (bottom first). */
const contrast = (ink: string, ...under: string[]) => ratio(paint([...under, ink].map(layer)), paint(under.map(layer)));

describe('the colours as each engine computes them', () => {
  it('reads rgb(), rgba(), #hex and color(srgb …)', () => {
    expect(cssToHex('rgb(255, 0, 128)')).toBe('#ff0080ff');
    expect(cssToHex('rgba(255, 255, 255, 0.075)')).toBe('#ffffff13');
    expect(cssToHex('#08090a')).toBe('#08090aff');
    expect(cssToHex('#fff8')).toBe('#ffffff88');
    expect(cssToHex('color(srgb 1 0.5 0 / 0.5)')).toBe('#ff800080');
  });

  it('reads oklch() and oklab() (WebKit keeps the space of a color-mix)', () => {
    expect(cssToHex('oklch(1 0 0)')).toBe('#ffffffff');
    expect(cssToHex('oklch(0 0 0)')).toBe('#000000ff');
    // oklch(0.628 0.2577 29.23) is sRGB red.
    expect(cssToHex('oklch(0.628 0.2577 29.23)')).toBe('#ff0000ff');
    expect(cssToHex('oklch(62.8% 0.2577 29.23 / 0.5)')).toBe('#ff000080');
    expect(cssToHex('oklab(0.628 0.2249 0.1258)')).toBe('#ff0000ff');
  });

  it('leaves another notation to the canvas', () => {
    expect(cssToHex('lab(50 20 30)')).toBeNull();
    expect(cssToHex('color(display-p3 1 0 0)')).toBeNull();
  });
});

describe("Escouade's theme for the VS Code view", () => {
  it('gives each key of VS Code the colour of its token, or its token at its alpha', () => {
    const colors = vscodeColors(fake);
    for (const [key, v] of Object.entries(COLORS)) {
      if (typeof v === 'string') expect(colors[key as keyof typeof COLORS], key).toBe(fake[v]);
      else
        expect(colors[key as keyof typeof COLORS], key).toBe(
          fake[v[0]].slice(0, 7) +
            Math.round(v[1] * 255)
              .toString(16)
              .padStart(2, '0'),
        );
    }
    expect(colors['editor.background']).toBe(fake.panel);
    expect(colors['sideBar.background']).toBe(fake.frame);
    expect(colors['tab.activeBorderTop']).toBe(fake['p-ink']);
    expect(colors['editor.selectionBackground']).toBe(fake.p.slice(0, 7) + '2e');
  });

  it('keeps the alpha of a token that has one (a hairline, a hover)', () => {
    expect(vscodeColors({ ...fake, muted: '#ffffff80' })['scrollbarSlider.background']).toBe('#ffffff1a');
  });

  it('reads every token it lays, and every one of them is a token of the app in both themes', () => {
    for (const t of [...Object.values(COLORS).map((v) => (typeof v === 'string' ? v : v[0])), ...SYNTAX.map((s) => s.token)])
      expect(TOKENS).toContain(t);
    for (const t of TOKENS) {
      expect(DARK, `--${t}`).toHaveProperty(`--${t}`);
      expect(LIGHT, `--${t}`).toHaveProperty(`--${t}`);
    }
  });

  it('colours the syntax with the editor’s own scopes', () => {
    const rules = syntaxRules(fake);
    const of = (scope: string) => rules.find((r) => r.scope.includes(scope))?.settings.foreground;
    expect(of('keyword')).toBe(fake['syn-k']);
    expect(of('string')).toBe(fake['syn-s']);
    expect(of('entity.name.function')).toBe(fake['syn-f']);
    expect(of('entity.name.type')).toBe(fake['syn-t']);
    expect(of('constant.numeric')).toBe(fake['syn-n']);
    expect(of('comment')).toBe(fake['code-dim']);
  });

  it('tells a dark theme by its panel, and picks the theme of its kind', () => {
    expect(isDark({ ...fake, panel: '#08090aff' })).toBe(true);
    expect(isDark({ ...fake, panel: '#fafafaff' })).toBe(false);
    expect(themeSettings({ ...fake, panel: '#08090aff' })['workbench.colorTheme']).toBe(THEME_IDS.dark);
    expect(themeSettings({ ...fake, panel: '#fafafaff' })['workbench.colorTheme']).toBe(THEME_IDS.light);
    expect(vscodeTheme(fake, true).type).toBe('dark');
    expect(vscodeTheme(fake, false).type).toBe('light');
  });

  it('lays the colours and the syntax as customizations, which VS Code applies at once', () => {
    const s = themeSettings(fake);
    expect(s['workbench.colorCustomizations']).toEqual(vscodeColors(fake));
    expect(s['editor.tokenColorCustomizations'].textMateRules).toEqual(syntaxRules(fake));
  });

  for (const [name, theme] of Object.entries(THEMES))
    it(`gives the app's own colours in the ${name} theme, the tint of the project in them`, () => {
      const a = appTokens(theme, PROJECT_COLORS[0]);
      const b = appTokens(theme, PROJECT_COLORS[1]);
      expect(isDark(a)).toBe(name === 'dark');
      // Each project's own tint on the frame and the selection.
      expect(vscodeColors(a)['sideBar.background']).not.toBe(vscodeColors(b)['sideBar.background']);
      expect(vscodeColors(a)['editor.selectionBackground']).not.toBe(vscodeColors(b)['editor.selectionBackground']);
    });

  for (const [name, theme] of Object.entries(THEMES))
    it(`stays readable in the ${name} theme, for every project colour`, () => {
      const bad: string[] = [];
      for (const p of TINTS) {
        const c = vscodeColors(appTokens(theme, p));
        const check = (label: string, r: number, min: number) => {
          if (r < min) bad.push(`${label} : ${r.toFixed(2)} (${p})`);
        };
        const bg = c['editor.background'];
        const tokens = appTokens(theme, p);
        const code = ['editor.foreground', ...syntaxRules(tokens).map((r) => r.settings.foreground)];
        // The text and its syntax, on the editor, its current line, its selection and its matches: AA. A comment, the
        // code's secondary text, keeps 3:1 under a selection or a match (AA on the editor and its current line).
        for (const ink of code) {
          const at = ink.startsWith('#') ? ink : c[ink as keyof typeof c];
          const lit = at === tokens['code-dim'] ? 3 : 4.5;
          check(`code ${at} sur l'éditeur`, contrast(at, bg), 4.5);
          check(`code ${at} sur la ligne courante`, contrast(at, bg, c['editor.lineHighlightBackground']), 4.5);
          check(`code ${at} sur la sélection`, contrast(at, bg, c['editor.selectionBackground']), lit);
          check(`code ${at} sur une occurrence`, contrast(at, bg, c['editor.findMatchHighlightBackground']), lit);
          check(`code ${at} sur l'occurrence courante`, contrast(at, bg, c['editor.findMatchBackground']), lit);
        }
        // The labels: the tabs, the tree, the bars, the menus, the palette, the tooltips.
        const pairs: [string, string[]][] = [
          ['tab.activeForeground', ['tab.activeBackground']],
          ['tab.inactiveForeground', ['tab.inactiveBackground']],
          ['sideBar.foreground', ['sideBar.background']],
          ['sideBarTitle.foreground', ['sideBar.background']],
          ['list.activeSelectionForeground', ['sideBar.background', 'list.activeSelectionBackground']],
          ['list.highlightForeground', ['sideBar.background']],
          ['statusBar.foreground', ['statusBar.background']],
          ['menu.foreground', ['menu.background']],
          ['menu.selectionForeground', ['menu.background', 'menu.selectionBackground']],
          ['quickInput.foreground', ['quickInput.background']],
          ['quickInputList.focusForeground', ['quickInput.background', 'quickInputList.focusBackground']],
          ['editorHoverWidget.foreground', ['editorHoverWidget.background']],
          ['editorWidget.foreground', ['editorWidget.background']],
          ['input.foreground', ['input.background']],
          ['input.placeholderForeground', ['input.background']],
          ['button.foreground', ['button.background']],
          ['editorLineNumber.activeForeground', ['editor.background']],
          ['gitDecoration.modifiedResourceForeground', ['sideBar.background']],
          ['gitDecoration.untrackedResourceForeground', ['sideBar.background']],
          ['gitDecoration.deletedResourceForeground', ['sideBar.background']],
          ['textLink.foreground', ['editor.background']],
        ];
        for (const [ink, under] of pairs)
          check(`${ink} sur ${under.join(' + ')}`, contrast(c[ink as keyof typeof c], ...under.map((u) => c[u as keyof typeof c])), 4.5);
        // The marks that are no text: the errors and warnings under the code, the cursor, the border of a match: 3:1.
        for (const ink of [
          'editorError.foreground',
          'editorWarning.foreground',
          'editorInfo.foreground',
          'editorCursor.foreground',
          'editor.findMatchBorder',
        ])
          check(`${ink} sur l'éditeur`, contrast(c[ink as keyof typeof c], bg), 3);
      }
      expect(bad).toEqual([]);
    });
});
