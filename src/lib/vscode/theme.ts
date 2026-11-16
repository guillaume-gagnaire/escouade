// Escouade's theme for the VS Code view (ESC-444, taken in ESC-447, followed live since ESC-472). Its colour tokens
// (`app.css`: oklch, color-mix, rgba), read as the browser computes them, made `#rrggbbaa`, and laid on the keys of VS
// Code's colours and on its syntax scopes. The view registers two themes, dark and light, that VS Code's defaults of
// their kind fill in; our colours go over them as the user's customizations (`themeSettings`), which VS Code applies at
// once when they change: the window's theme, the project's tint, a plugin's theme (`workbench.ts::followTheme`).

/**
 * The colours of VS Code, each from a token of `app.css` (its name without `--`), or a token at an alpha. The only table
 * to write: the tokens read (`TOKENS`) are those it names, so that a key added here with a token new to the app needs
 * nothing else, and a token whose value changes (the theme, the tint, a plugin's theme) is followed at once.
 */
export const COLORS = {
  foreground: 'soft',
  descriptionForeground: 'muted',
  errorForeground: 'del',
  'icon.foreground': 'muted',
  focusBorder: ['p-ink', 0.6],
  'widget.border': 'line',
  'widget.shadow': ['frame', 0.5],
  'sash.hoverBorder': ['p-ink', 0.6],
  'selection.background': ['p', 0.18],
  'textLink.foreground': 'info',
  'textLink.activeForeground': 'info',
  'progressBar.background': 'p-ink',
  'editor.background': 'panel',
  'editor.foreground': 'code-fg',
  'editorLineNumber.foreground': 'code-ln',
  'editorLineNumber.activeForeground': 'code-dim',
  'editor.lineHighlightBackground': 'hover',
  'editor.lineHighlightBorder': ['line', 0],
  'editor.selectionBackground': ['p', 0.18],
  'editor.inactiveSelectionBackground': ['p', 0.1],
  'editor.selectionHighlightBackground': ['p', 0.14],
  'editor.wordHighlightBackground': ['p', 0.14],
  'editor.findMatchBackground': ['p', 0.18],
  'editor.findMatchBorder': 'p-ink',
  'editor.findMatchHighlightBackground': ['p', 0.1],
  'editor.findRangeHighlightBackground': 'hover',
  'editor.findMatchHighlightBorder': ['p-ink', 0.45],
  'editor.rangeHighlightBackground': 'hover',
  'editor.hoverHighlightBackground': ['p', 0.1],
  'editorCursor.foreground': 'p-ink',
  'editorWhitespace.foreground': 'line-2',
  'editorIndentGuide.background1': 'line',
  'editorIndentGuide.activeBackground1': 'line-3',
  'editorBracketMatch.background': ['p', 0.16],
  'editorBracketMatch.border': ['p-ink', 0.5],
  'editorRuler.foreground': 'line',
  'editorLink.activeForeground': 'info',
  'editorWidget.background': 'raise',
  'editorWidget.foreground': 'soft',
  'editorWidget.border': 'line-2',
  'editorSuggestWidget.background': 'raise',
  'editorSuggestWidget.border': 'line-2',
  'editorSuggestWidget.foreground': 'soft',
  'editorSuggestWidget.selectedBackground': 'p-soft',
  'editorSuggestWidget.highlightForeground': 'p-ink',
  'editorHoverWidget.background': 'raise',
  'editorHoverWidget.foreground': 'soft',
  'editorHoverWidget.border': 'line-2',
  'editorStickyScroll.background': 'panel',
  'editorError.foreground': 'del',
  'editorWarning.foreground': 'wait',
  'editorInfo.foreground': 'info',
  'editorGutter.background': 'panel',
  'editorGutter.addedBackground': 'add',
  'editorGutter.deletedBackground': 'del',
  'editorGutter.modifiedBackground': 'git-m',
  'editorOverviewRuler.border': 'line',
  'editorOverviewRuler.findMatchForeground': ['p-ink', 0.6],
  'editorOverviewRuler.rangeHighlightForeground': ['p-ink', 0.3],
  'editorOverviewRuler.selectionHighlightForeground': ['p-ink', 0.3],
  'editorOverviewRuler.wordHighlightForeground': ['p-ink', 0.3],
  'editorOverviewRuler.errorForeground': 'del',
  'editorOverviewRuler.warningForeground': 'wait',
  'editorGroupHeader.tabsBackground': 'frame',
  'editorGroupHeader.tabsBorder': 'line',
  'editorGroupHeader.noTabsBackground': 'frame',
  'editorGroup.border': 'line',
  'editorGroup.dropBackground': 'p-soft',
  'breadcrumb.foreground': 'muted',
  'breadcrumb.focusForeground': 'text',
  'breadcrumb.activeSelectionForeground': 'text',
  'breadcrumb.background': 'panel',
  'tab.activeBackground': 'panel',
  'tab.inactiveBackground': 'frame',
  'tab.activeForeground': 'text',
  'tab.inactiveForeground': 'muted',
  'tab.unfocusedActiveForeground': 'soft',
  'tab.unfocusedInactiveForeground': 'muted',
  'tab.border': 'line',
  'tab.activeBorderTop': 'p-ink',
  'tab.unfocusedActiveBorderTop': ['p-ink', 0.5],
  'tab.hoverBackground': 'hover',
  'tab.dragAndDropBorder': 'p-ink',
  'sideBar.background': 'frame',
  'sideBar.foreground': 'soft',
  'sideBar.border': 'line',
  'sideBarTitle.foreground': 'muted',
  'sideBarSectionHeader.background': 'frame',
  'sideBarSectionHeader.foreground': 'soft',
  'sideBarSectionHeader.border': 'line',
  'activityBar.background': 'frame',
  'activityBar.foreground': 'text',
  'activityBar.inactiveForeground': 'dim',
  'activityBar.activeBorder': 'p-ink',
  'activityBar.border': 'line',
  'activityBarBadge.background': 'p',
  'activityBarBadge.foreground': 'on-tile',
  'statusBar.background': 'frame',
  'statusBar.foreground': 'muted',
  'statusBar.border': 'line',
  'statusBar.noFolderBackground': 'frame',
  'statusBarItem.hoverBackground': 'hover',
  'titleBar.activeBackground': 'frame',
  'titleBar.activeForeground': 'soft',
  'titleBar.inactiveBackground': 'frame',
  'titleBar.inactiveForeground': 'muted',
  'titleBar.border': 'line',
  'panel.background': 'panel',
  'panel.border': 'line',
  'panelTitle.activeForeground': 'text',
  'panelTitle.inactiveForeground': 'muted',
  'panelTitle.activeBorder': 'p-ink',
  'list.hoverBackground': 'hover',
  'list.hoverForeground': 'text',
  'list.activeSelectionBackground': 'p-soft',
  'list.activeSelectionForeground': 'text',
  'list.inactiveSelectionBackground': 'hover',
  'list.inactiveSelectionForeground': 'text',
  'list.focusBackground': 'p-soft',
  'list.focusForeground': 'text',
  'list.focusOutline': ['p-ink', 0.5],
  'list.highlightForeground': 'p-ink',
  'list.focusHighlightForeground': 'p-ink',
  'list.errorForeground': 'del',
  'list.warningForeground': 'wait',
  'list.dropBackground': 'p-soft',
  'tree.indentGuidesStroke': 'line-2',
  'input.background': 'well',
  'input.foreground': 'text',
  'input.border': 'line-2',
  'input.placeholderForeground': 'dim',
  'inputOption.activeBackground': 'p-soft',
  'inputOption.activeBorder': ['p-ink', 0.6],
  'inputOption.activeForeground': 'text',
  'inputValidation.errorBackground': 'raise',
  'inputValidation.errorBorder': 'del',
  'inputValidation.warningBackground': 'raise',
  'inputValidation.warningBorder': 'wait',
  'inputValidation.infoBackground': 'raise',
  'inputValidation.infoBorder': 'info',
  'dropdown.background': 'raise',
  'dropdown.foreground': 'soft',
  'dropdown.border': 'line-2',
  'dropdown.listBackground': 'raise',
  'checkbox.background': 'well',
  'checkbox.border': 'line-2',
  'checkbox.foreground': 'text',
  'button.background': 'primary',
  'button.foreground': 'on-primary',
  'button.hoverBackground': 'primary-hover',
  'button.secondaryBackground': 'raise-2',
  'button.secondaryForeground': 'soft',
  'button.secondaryHoverBackground': 'hover',
  'badge.background': 'p-soft',
  'badge.foreground': 'text',
  'scrollbar.shadow': ['frame', 0],
  'scrollbarSlider.background': ['muted', 0.2],
  'scrollbarSlider.hoverBackground': ['muted', 0.35],
  'scrollbarSlider.activeBackground': ['muted', 0.45],
  'gitDecoration.modifiedResourceForeground': 'git-m',
  'gitDecoration.addedResourceForeground': 'add',
  'gitDecoration.untrackedResourceForeground': 'add',
  'gitDecoration.deletedResourceForeground': 'del',
  'menu.background': 'raise',
  'menu.foreground': 'soft',
  'menu.border': 'line-2',
  'menu.selectionBackground': 'p-soft',
  'menu.selectionForeground': 'text',
  'menu.separatorBackground': 'line-2',
  'quickInput.background': 'raise',
  'quickInput.foreground': 'soft',
  'quickInputTitle.background': 'raise-2',
  'quickInputList.focusBackground': 'p-soft',
  'quickInputList.focusForeground': 'text',
  'quickInputList.focusIconForeground': 'text',
  'pickerGroup.foreground': 'p-ink',
  'pickerGroup.border': 'line',
  'keybindingLabel.background': 'well',
  'keybindingLabel.foreground': 'soft',
  'keybindingLabel.border': 'line-2',
  'keybindingLabel.bottomBorder': 'line-2',
  'notifications.background': 'raise',
  'notifications.foreground': 'soft',
  'notifications.border': 'line',
  'notificationCenterHeader.background': 'raise-2',
  'notificationsErrorIcon.foreground': 'del',
  'notificationsWarningIcon.foreground': 'wait',
  'notificationsInfoIcon.foreground': 'info',
  'problemsErrorIcon.foreground': 'del',
  'problemsWarningIcon.foreground': 'wait',
  'problemsInfoIcon.foreground': 'info',
  'peekView.border': 'p-ink',
  'peekViewEditor.background': 'well',
  'peekViewResult.background': 'raise',
  'peekViewTitle.background': 'raise-2',
  'searchEditor.findMatchBackground': ['p', 0.18],
  'toolbar.hoverBackground': 'hover',
  'toolbar.activeBackground': 'p-soft',
  'diffEditor.insertedTextBackground': ['add', 0.18],
  'diffEditor.removedTextBackground': ['del', 0.18],
  // The changes from the base (ESC-476): the comparison's lines, its gutter and the overview ruler, as the marks.
  'diffEditor.insertedLineBackground': ['add', 0.1],
  'diffEditor.removedLineBackground': ['del', 0.1],
  'diffEditor.border': 'line',
  'diffEditor.diagonalFill': ['line-2', 0.6],
  'diffEditorGutter.insertedLineBackground': ['add', 0.14],
  'diffEditorGutter.removedLineBackground': ['del', 0.14],
  'diffEditorOverview.insertedForeground': ['add', 0.6],
  'diffEditorOverview.removedForeground': ['del', 0.6],
  'editorOverviewRuler.addedForeground': ['add', 0.6],
  'editorOverviewRuler.modifiedForeground': ['git-m', 0.6],
  'editorOverviewRuler.deletedForeground': ['del', 0.6],
  'minimapGutter.addedBackground': 'add',
  'minimapGutter.modifiedBackground': 'git-m',
  'minimapGutter.deletedBackground': 'del',
} as const satisfies Record<string, string | readonly [string, number]>;

/** The syntax: TextMate scopes, each from a token. */
export const SYNTAX = [
  { scope: ['comment', 'punctuation.definition.comment'], token: 'code-dim', fontStyle: 'italic' },
  { scope: ['keyword', 'storage', 'storage.type', 'storage.modifier', 'keyword.control'], token: 'syn-k' },
  { scope: ['keyword.operator', 'punctuation.separator', 'punctuation.terminator', 'meta.brace'], token: 'code-fg' },
  { scope: ['entity.name.function', 'support.function', 'meta.function-call'], token: 'syn-f' },
  { scope: ['string', 'string.quoted', 'string.template', 'markup.inline.raw'], token: 'syn-s' },
  {
    scope: ['entity.name.type', 'support.type', 'entity.name.class', 'entity.other.inherited-class', 'storage.type.rust'],
    token: 'syn-t',
  },
  { scope: ['constant.numeric', 'constant.language', 'constant.character', 'support.constant'], token: 'syn-n' },
  { scope: ['entity.name.tag', 'markup.heading'], token: 'syn-k' },
  { scope: ['entity.other.attribute-name', 'support.type.property-name'], token: 'syn-f' },
  { scope: ['variable', 'meta.definition.variable', 'variable.parameter'], token: 'code-fg' },
  { scope: ['invalid', 'invalid.illegal'], token: 'del' },
] as const;

/** The tokens the theme is made of: those the tables name, once each (`panel` tells dark from light). */
export const TOKENS: string[] = [
  ...new Set<string>([...Object.values(COLORS).map((v) => (typeof v === 'string' ? v : v[0])), ...SYNTAX.map((s) => s.token), 'panel']),
];
export type Tokens = Record<string, string>;

/** `#rrggbbaa` of a colour of the browser's, by a canvas of one pixel (any notation the engine knows). */
function canvasHex(ctx: CanvasRenderingContext2D, css: string): string {
  ctx.clearRect(0, 0, 1, 1);
  ctx.fillStyle = '#000';
  ctx.fillStyle = css;
  ctx.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data;
  return toHex(r, g, b, a / 255);
}

const toHex = (r: number, g: number, b: number, a: number) =>
  '#' +
  [r, g, b, a * 255]
    .map((x) =>
      Math.round(Math.min(255, Math.max(0, x)))
        .toString(16)
        .padStart(2, '0'),
    )
    .join('');

/** sRGB's transfer curve, from a linear value (0 to 1) to 0 to 255. */
const encode = (x: number) => 255 * (x <= 0.0031308 ? 12.92 * x : 1.055 * Math.pow(x, 1 / 2.4) - 0.055);

/** Oklab (L 0 to 1) to sRGB 0 to 255 (Björn Ottosson's matrices). */
function oklabToRgb(L: number, a: number, b: number): [number, number, number] {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    encode(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
    encode(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
    encode(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s),
  ];
}

/**
 * `#rrggbbaa` of a colour as `getComputedStyle` gives it, by each engine its own way: `rgb()`/`rgba()` (WebView2),
 * `color(srgb …)`, `oklch()`, `oklab()` (WebKit keeps a color-mix's space), `#hex`. Null for another notation.
 */
export function cssToHex(css: string): string | null {
  const s = css.trim().toLowerCase();
  const nums = (body: string) =>
    body
      .replace(/[,/]/g, ' ')
      .split(/\s+/)
      .filter(Boolean)
      .map((x) => (x.endsWith('%') ? parseFloat(x) / 100 : x === 'none' ? 0 : parseFloat(x)));
  const inner = /^[a-z-]+\((.*)\)$/.exec(s)?.[1];
  if (s.startsWith('#')) {
    const h = s.slice(1);
    const full = h.length <= 4 ? [...h].map((c) => c + c).join('') : h;
    const [r, g, b, a = 255] = (full.match(/../g) ?? []).map((x) => parseInt(x, 16));
    return toHex(r, g, b, a / 255);
  }
  if (!inner) return null;
  if (s.startsWith('rgb')) {
    const [r, g, b, a = 1] = nums(inner);
    return toHex(r, g, b, a);
  }
  if (s.startsWith('color(')) {
    const [space, ...rest] = inner.split(/\s+/);
    if (space !== 'srgb') return null;
    const [r, g, b, a = 1] = nums(rest.join(' '));
    return toHex(r * 255, g * 255, b * 255, a);
  }
  if (s.startsWith('oklch(')) {
    const [L, C, H, a = 1] = nums(inner);
    const h = (H * Math.PI) / 180;
    const [r, g, b] = oklabToRgb(L > 1 ? L / 100 : L, C * Math.cos(h), C * Math.sin(h));
    return toHex(r, g, b, a);
  }
  if (s.startsWith('oklab(')) {
    const [L, A, B, a = 1] = nums(inner);
    const [r, g, b] = oklabToRgb(L > 1 ? L / 100 : L, A, B);
    return toHex(r, g, b, a);
  }
  return null;
}

/** The tokens of `el` (the project's tint is on the root), as `#rrggbbaa`. */
export function readTokens(el: Element = document.documentElement): Tokens {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 1;
  const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
  const out: Tokens = {};
  // A probe of its own for each token: WKWebView answered the first token's colour for all of them when one probe's
  // colour was changed from token to token (its text then the colour of its background).
  for (const t of TOKENS) {
    const probe = document.createElement('span');
    probe.style.color = `var(--${t})`;
    el.appendChild(probe);
    const css = getComputedStyle(probe).color;
    probe.remove();
    out[t] = cssToHex(css) ?? canvasHex(ctx, css);
  }
  return out;
}

/** Whether the tokens are those of a dark theme: the panel's luminance. */
export function isDark(c: Tokens): boolean {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(c.panel.slice(i, i + 2), 16));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b < 128;
}

/** `#rrggbbaa` with its alpha multiplied by `a` (0 to 1): a token at an alpha of its own keeps it. */
const alpha = (c: string, a: number) => {
  const own = c.length >= 9 ? parseInt(c.slice(7, 9), 16) / 255 : 1;
  return (
    c.slice(0, 7) +
    Math.round(a * own * 255)
      .toString(16)
      .padStart(2, '0')
  );
};

/** The colours of VS Code of the tokens `c`, `#rrggbbaa` each. */
export function vscodeColors(c: Tokens): Record<keyof typeof COLORS, string> {
  return Object.fromEntries(Object.entries(COLORS).map(([key, v]) => [key, typeof v === 'string' ? c[v] : alpha(c[v[0]], v[1])])) as Record<
    keyof typeof COLORS,
    string
  >;
}

/** The TextMate rules of the syntax of the tokens `c`. */
export function syntaxRules(c: Tokens) {
  return SYNTAX.map((s) => ({
    scope: [...s.scope] as string[],
    settings: 'fontStyle' in s ? { foreground: c[s.token], fontStyle: s.fontStyle as string } : { foreground: c[s.token] },
  }));
}

/** The two themes the view switches between, by the window's: VS Code's defaults of their kind under our colours. */
export const THEME_IDS = { dark: 'Escouade Dark', light: 'Escouade Light' } as const;

/** The VS Code colour theme of the tokens `c`: the one registered, with the colours the view starts with. */
export function vscodeTheme(c: Tokens, dark: boolean) {
  return {
    name: dark ? THEME_IDS.dark : THEME_IDS.light,
    type: dark ? 'dark' : 'light',
    colors: vscodeColors(c),
    tokenColors: syntaxRules(c),
    semanticHighlighting: true,
  };
}

/**
 * The user settings that lay the tokens `c` on the view: the theme of their kind, and over it our colours and syntax
 * as customizations, which VS Code applies at once when they change (a theme's own file it reads once).
 */
export function themeSettings(c: Tokens) {
  return {
    'workbench.colorTheme': isDark(c) ? THEME_IDS.dark : THEME_IDS.light,
    'workbench.colorCustomizations': vscodeColors(c),
    'editor.tokenColorCustomizations': { textMateRules: syntaxRules(c) },
  };
}
