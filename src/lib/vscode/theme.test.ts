import { describe, expect, it } from 'vitest';
import { TOKENS, cssToHex, vscodeTheme, type Tokens } from './theme';

const tokens = Object.fromEntries(TOKENS.map((t, i) => [t, `#${i.toString(16).padStart(6, '0')}ff`])) as Tokens;

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
  it('lays the tokens on the editor, the tabs and the tree', () => {
    const th = vscodeTheme(tokens, true);
    expect(th.type).toBe('dark');
    expect(th.colors['editor.background']).toBe(tokens.panel);
    expect(th.colors['sideBar.background']).toBe(tokens.frame);
    expect(th.colors['tab.activeBorderTop']).toBe(tokens['p-ink']);
    expect(vscodeTheme(tokens, false).type).toBe('light');
  });

  it('gives the selection the tint at a low alpha', () => {
    expect(vscodeTheme(tokens, true).colors['editor.selectionBackground']).toBe(tokens.p.slice(0, 7) + '4d');
  });

  it('colours the syntax with the editor’s own scopes', () => {
    const kw = vscodeTheme(tokens, true).tokenColors.find((x) => x.scope.includes('keyword'));
    expect(kw?.settings.foreground).toBe(tokens['syn-k']);
  });
});
