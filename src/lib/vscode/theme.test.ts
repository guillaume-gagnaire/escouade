import { describe, expect, it } from 'vitest';
import { TOKENS, vscodeTheme, type Tokens } from './theme';

const tokens = Object.fromEntries(TOKENS.map((t, i) => [t, `#${i.toString(16).padStart(6, '0')}ff`])) as Tokens;

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
