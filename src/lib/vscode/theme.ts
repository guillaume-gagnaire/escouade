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
function hex(ctx: CanvasRenderingContext2D, css: string): string {
  ctx.clearRect(0, 0, 1, 1);
  ctx.fillStyle = '#000';
  ctx.fillStyle = css;
  ctx.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data;
  return '#' + [r, g, b, a].map((x) => x.toString(16).padStart(2, '0')).join('');
}

/** The tokens of `el` (the project's tint is on its frame), as `#rrggbbaa`. */
export function readTokens(el: Element = document.documentElement): Tokens {
  const probe = document.createElement('span');
  el.appendChild(probe);
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 1;
  const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
  const out = {} as Tokens;
  for (const t of TOKENS) {
    probe.style.color = `var(--${t})`;
    out[t] = hex(ctx, getComputedStyle(probe).color);
  }
  probe.remove();
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
