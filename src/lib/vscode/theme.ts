// ESC-444 (study): Escouade's theme for the VS Code view. Its colour tokens (`app.css`: oklch, color-mix, rgba), read
// as the browser computes them, made `#rrggbbaa`, and laid on the keys of a VS Code colour theme and on its syntax
// scopes. Made again when the theme or the project's tint changes (`escouadeTheme`).

/** The tokens the theme is made of. */
export const TOKENS = [
  'frame',
  'panel',
  'raise',
  'well',
  'line',
  'hover',
  'text',
  'soft',
  'muted',
  'dim',
  'p',
  'p-ink',
  'p-soft',
  'add',
  'del',
  'git-m',
  'wait',
  'info',
  'code-fg',
  'code-ln',
  'code-dim',
  'syn-k',
  'syn-f',
  'syn-s',
  'syn-t',
  'syn-n',
  'primary',
  'on-primary',
] as const;
export type Tokens = Record<(typeof TOKENS)[number], string>;

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

/** What the engine gave for each token, for the study's log (WKWebView read them all transparent through a canvas). */
export const tokenSources: Record<string, string> = {};

/** The tokens of `el` (the project's tint is on its frame), as `#rrggbbaa`. */
export function readTokens(el: Element = document.documentElement): Tokens {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 1;
  const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
  const out = {} as Tokens;
  // A probe of its own for each token: WKWebView answered the first token's colour for all of them when one probe's
  // colour was changed from token to token (its text then the colour of its background).
  for (const t of TOKENS) {
    const probe = document.createElement('span');
    probe.style.color = `var(--${t})`;
    el.appendChild(probe);
    const css = getComputedStyle(probe).color;
    probe.remove();
    tokenSources[t] = css;
    out[t] = cssToHex(css) ?? canvasHex(ctx, css);
  }
  return out;
}

/** `#rrggbb` with the alpha `a` (0 to 1). */
const alpha = (c: string, a: number) =>
  c.slice(0, 7) +
  Math.round(a * 255)
    .toString(16)
    .padStart(2, '0');

/** The VS Code colour theme of the tokens `c`. */
export function vscodeTheme(c: Tokens, dark: boolean) {
  return {
    name: 'Escouade',
    type: dark ? 'dark' : 'light',
    colors: {
      foreground: c.soft,
      focusBorder: alpha(c['p-ink'], 0.6),
      'widget.border': c.line,
      'editor.background': c.panel,
      'editor.foreground': c['code-fg'],
      'editorLineNumber.foreground': c['code-ln'],
      'editorLineNumber.activeForeground': c['code-dim'],
      'editor.lineHighlightBackground': c.hover,
      'editor.selectionBackground': alpha(c.p, 0.3),
      'editor.inactiveSelectionBackground': alpha(c.p, 0.16),
      'editorCursor.foreground': c['p-ink'],
      'editorIndentGuide.background1': c.line,
      'editorWidget.background': c.raise,
      'editorWidget.border': c.line,
      'editorSuggestWidget.background': c.raise,
      'editorSuggestWidget.selectedBackground': c['p-soft'],
      'editorHoverWidget.background': c.raise,
      'editorGutter.addedBackground': c.add,
      'editorGutter.deletedBackground': c.del,
      'editorGutter.modifiedBackground': c['git-m'],
      'editorGroupHeader.tabsBackground': c.frame,
      'editorGroupHeader.tabsBorder': c.line,
      'editorGroup.border': c.line,
      'tab.activeBackground': c.panel,
      'tab.inactiveBackground': c.frame,
      'tab.activeForeground': c.text,
      'tab.inactiveForeground': c.muted,
      'tab.border': c.line,
      'tab.activeBorderTop': c['p-ink'],
      'tab.hoverBackground': c.hover,
      'sideBar.background': c.frame,
      'sideBar.foreground': c.soft,
      'sideBar.border': c.line,
      'sideBarTitle.foreground': c.muted,
      'sideBarSectionHeader.background': c.frame,
      'sideBarSectionHeader.border': c.line,
      'activityBar.background': c.frame,
      'activityBar.foreground': c.text,
      'activityBar.inactiveForeground': c.dim,
      'activityBar.activeBorder': c['p-ink'],
      'activityBar.border': c.line,
      'activityBarBadge.background': c.p,
      'activityBarBadge.foreground': c['on-primary'],
      'statusBar.background': c.frame,
      'statusBar.foreground': c.muted,
      'statusBar.border': c.line,
      'statusBar.noFolderBackground': c.frame,
      'titleBar.activeBackground': c.frame,
      'titleBar.activeForeground': c.soft,
      'titleBar.inactiveBackground': c.frame,
      'titleBar.border': c.line,
      'panel.background': c.panel,
      'panel.border': c.line,
      'list.hoverBackground': c.hover,
      'list.activeSelectionBackground': c['p-soft'],
      'list.activeSelectionForeground': c.text,
      'list.inactiveSelectionBackground': c.hover,
      'list.focusOutline': alpha(c['p-ink'], 0.5),
      'input.background': c.well,
      'input.border': c.line,
      'button.background': c.primary,
      'button.foreground': c['on-primary'],
      'badge.background': c['p-soft'],
      'badge.foreground': c.text,
      'scrollbarSlider.background': alpha(c.muted, 0.2),
      'scrollbarSlider.hoverBackground': alpha(c.muted, 0.35),
      'gitDecoration.modifiedResourceForeground': c['git-m'],
      'gitDecoration.untrackedResourceForeground': c.add,
      'gitDecoration.deletedResourceForeground': c.del,
      'menu.background': c.raise,
      'menu.border': c.line,
      'quickInput.background': c.raise,
      'notifications.background': c.raise,
      'textLink.foreground': c.info,
      'editorError.foreground': c.del,
      'editorWarning.foreground': c.wait,
    },
    tokenColors: [
      { scope: ['comment', 'punctuation.definition.comment'], settings: { foreground: c['code-dim'], fontStyle: 'italic' } },
      { scope: ['keyword', 'storage', 'storage.type', 'keyword.control'], settings: { foreground: c['syn-k'] } },
      { scope: ['entity.name.function', 'support.function', 'meta.function-call'], settings: { foreground: c['syn-f'] } },
      { scope: ['string', 'string.quoted', 'string.template'], settings: { foreground: c['syn-s'] } },
      { scope: ['entity.name.type', 'support.type', 'entity.name.class', 'storage.type.rust'], settings: { foreground: c['syn-t'] } },
      { scope: ['constant.numeric', 'constant.language', 'constant.character'], settings: { foreground: c['syn-n'] } },
      { scope: ['variable', 'meta.definition.variable'], settings: { foreground: c['code-fg'] } },
    ],
    semanticHighlighting: true,
  };
}
