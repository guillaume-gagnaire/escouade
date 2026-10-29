// The themes a plugin brings (PLG-7, ESC-179; docs/PLUGINS.md §1.10): values for the published tokens of src/app.css
// (DESIGN.md « Les jetons »), dark and light, and a palette of project colours. Nothing of a theme reaches the window
// unfiltered: only the tokens of the list, only values of their kind — a colour (hex, rgb(), hsl(), oklch(),
// color-mix(), a var() of another token of the list…), a proportion (`--glow`, `--sel`), a number (`--shade`), three
// channels (`--fg-rgb`). No url(), no other function, no quote, no `;` or brace: a value can neither load anything
// nor write CSS of its own, nor hide a request of Escouade's. What a theme leaves out keeps Escouade's value.

import { textOf, type ManifestText, type PluginView } from './plugins';

/** What a token holds. */
export type TokenKind = 'color' | 'percent' | 'number' | 'channels';

/**
 * The tokens a theme may give, with their kind: DESIGN.md « Les jetons », but `--p` (the project's colour, which the
 * palette gives) and the form tokens (`--font`, `--r`, `--rail`…).
 */
export const THEME_TOKENS: Readonly<Record<string, TokenKind>> = Object.freeze({
  ...colors(['--p-ink', '--p-soft', '--brand']),
  '--sel': 'percent',
  '--glow': 'percent',
  ...colors(['--frame', '--panel', '--raise', '--raise-2', '--well', '--scrim', '--drop-base', '--vo-base', '--orb-base']),
  '--shade': 'number',
  '--fg-rgb': 'channels',
  ...colors(['--line', '--line-2', '--line-3', '--hover', '--sep', '--ring']),
  ...colors(['--text', '--soft', '--muted', '--dim', '--text-ai', '--idle']),
  ...colors([
    '--run',
    '--wait',
    '--wait-bg',
    '--wait-line',
    '--done',
    '--add',
    '--del',
    '--git-m',
    '--warn-ink',
    '--info',
    '--teal-ink',
    '--teal-ink-2',
    '--disc',
    '--brain',
  ]),
  ...colors([
    '--primary',
    '--primary-hover',
    '--on-primary',
    '--on-primary-line',
    '--on-primary-kbd',
    '--on-primary-dim',
    '--on-wait',
    '--on-hit',
    '--on-tick',
    '--on-tile',
    '--win-close',
    '--on-win-close',
  ]),
  ...colors(['--lane-main', ...Array.from({ length: 8 }, (_, i) => `--lane-${i + 1}`)]),
  ...colors([
    '--code-fg',
    '--code-ln',
    '--code-nb',
    '--code-dim',
    '--syn-k',
    '--syn-f',
    '--syn-s',
    '--syn-t',
    '--syn-n',
    '--ansi-k',
    '--ansi-w',
    '--ansi-w-2',
    '--ansi-y',
    '--ansi-cy',
    '--ansi-cy-2',
    '--term-sel-bg',
    '--term-sel-fg',
  ]),
});

function colors(names: string[]): Record<string, TokenKind> {
  return Object.fromEntries(names.map((n) => [n, 'color' as const]));
}

/** The longest value kept, and the most colours of a palette. */
export const MAX_VALUE = 160;
export const MAX_PALETTE = 24;

/** The functions a colour may be written with. */
const FUNCTIONS = new Set(['rgb', 'rgba', 'hsl', 'hsla', 'hwb', 'lab', 'lch', 'oklab', 'oklch', 'color', 'color-mix', 'var']);
/** The words a colour may hold: colour spaces and channels (color-mix(in oklch…), oklch(from … l c h / a)), a few colours. */
const WORDS = new Set(
  [
    'in from none transparent currentcolor white black',
    'srgb srgb-linear display-p3 a98-rgb prophoto-rgb rec2020 xyz xyz-d50 xyz-d65 oklch oklab lab lch hsl hwb',
    'shorter longer increasing decreasing hue',
    'l c h r g b s w a x y z alpha',
  ]
    .join(' ')
    .split(' '),
);

/** Why a value was left out. */
export type DropReason = 'token' | 'type' | 'long' | 'value';

/** A token of a theme left out, and why. */
export interface Dropped {
  mode: 'dark' | 'light' | 'projectColors';
  token: string;
  reason: DropReason;
}

/** A colour as CSS would have it, through `CSS.supports` where there is one (the window); the shape is ours anyway. */
const supported = (value: string) =>
  typeof CSS !== 'undefined' && typeof CSS.supports === 'function' ? CSS.supports('color', value) : true;

/**
 * Whether `value` is one colour, written with the functions and words above only: `#rgb` to `#rrggbbaa`, a function
 * of the list with numbers, units, words, colours and `var(--…)` of a token of the list (`vars: false`: none), in
 * balanced brackets, one term in all.
 */
export function isColor(value: string, vars = true): boolean {
  if (!/^[a-z0-9#%.,/()\s+-]*$/i.test(value)) return false;
  const tokens = value.match(
    /#[0-9a-f]+|[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?(?:%|deg|grad|rad|turn)?|--[a-z0-9-]+|[a-z][a-z0-9-]*\(?|[(),/]|\s+/gi,
  );
  if (!tokens || tokens.join('') !== value) return false;
  let depth = 0;
  let terms = 0;
  let inVar = false;
  for (let i = 0; i < tokens.length; i++) {
    const tok = tokens[i];
    const low = tok.toLowerCase();
    if (/^\s+$/.test(tok)) continue;
    const top = depth === 0;
    if (tok.startsWith('#')) {
      if (![4, 5, 7, 9].includes(tok.length)) return false;
    } else if (low.endsWith('(')) {
      const fn = low.slice(0, -1);
      if (!FUNCTIONS.has(fn) || (fn === 'var' && !vars)) return false;
      if (fn === 'var') {
        // var(--token) of the list, nothing else in it.
        const name = tokens[i + 1];
        if (!name || !(name in THEME_TOKENS || name === '--p') || tokens[i + 2] !== ')') return false;
        inVar = true;
      }
      depth++;
    } else if (tok === ')') {
      if (--depth < 0) return false;
      inVar = false;
      continue;
    } else if (tok === '(') {
      return false;
    } else if (tok.startsWith('--')) {
      if (!inVar) return false;
    } else if (/^[a-z]/i.test(tok)) {
      if (!WORDS.has(low)) return false;
    } else if (top) {
      // A number, a comma or a slash alone is no colour.
      return false;
    }
    if (top) terms++;
  }
  return depth === 0 && terms === 1 && supported(value);
}

/** `value` as the token `token` takes it, or why it is left out. */
export function filterValue(token: string, value: unknown): { value: string } | { reason: DropReason } {
  const kind = Object.prototype.hasOwnProperty.call(THEME_TOKENS, token) ? THEME_TOKENS[token] : undefined;
  if (!kind) return { reason: 'token' };
  if (typeof value !== 'string') return { reason: 'type' };
  const v = value.trim().replace(/\s+/g, ' ');
  if (!v) return { reason: 'value' };
  if (v.length > MAX_VALUE) return { reason: 'long' };
  const ok =
    kind === 'color'
      ? isColor(v)
      : kind === 'percent'
        ? /^(?:100|\d{1,2}(?:\.\d+)?)%$/.test(v)
        : kind === 'number'
          ? /^(?:0(?:\.\d+)?|1(?:\.0+)?|\.\d+)$/.test(v)
          : /^\d{1,3}, ?\d{1,3}, ?\d{1,3}$/.test(v) && v.split(',').every((c) => Number(c) <= 255);
  return ok ? { value: v } : { reason: 'value' };
}

/** A theme of a plugin, filtered: what reaches the window. */
export interface PluginTheme {
  /** `<plugin id>/<theme id>`: what « Couleurs » keeps (`Settings.pluginTheme`). */
  key: string;
  pluginId: string;
  pluginName: string;
  id: string;
  name: ManifestText;
  dark: Record<string, string>;
  light: Record<string, string>;
  projectColors: string[];
  dropped: Dropped[];
}

const isObject = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);

/** One entry of `contributes.themes`, filtered; null when it is no theme (no id). */
export function filterTheme(raw: unknown, pluginId: string, pluginName = pluginId): PluginTheme | null {
  if (!isObject(raw) || typeof raw.id !== 'string' || !/^[a-z0-9][a-z0-9._-]{0,63}$/i.test(raw.id)) return null;
  const dropped: Dropped[] = [];
  const mode = (m: 'dark' | 'light') => {
    const out: Record<string, string> = {};
    const values = raw[m];
    if (values === undefined) return out;
    if (!isObject(values)) {
      dropped.push({ mode: m, token: '', reason: 'type' });
      return out;
    }
    for (const [token, value] of Object.entries(values)) {
      const r = filterValue(token, value);
      if ('value' in r) out[token] = r.value;
      else dropped.push({ mode: m, token, reason: r.reason });
    }
    return out;
  };
  const dark = mode('dark');
  const light = mode('light');
  const projectColors: string[] = [];
  const palette = raw.projectColors;
  if (Array.isArray(palette)) {
    palette.forEach((c, i) => {
      const v = typeof c === 'string' ? c.trim().replace(/\s+/g, ' ') : null;
      const reason: DropReason | null =
        v === null
          ? 'type'
          : v.length > MAX_VALUE
            ? 'long'
            : !isColor(v, false)
              ? 'value'
              : projectColors.length >= MAX_PALETTE
                ? 'long'
                : null;
      if (reason) dropped.push({ mode: 'projectColors', token: String(i), reason });
      else if (!projectColors.includes(v!)) projectColors.push(v!);
    });
  } else if (palette !== undefined) dropped.push({ mode: 'projectColors', token: '', reason: 'type' });
  const name = (typeof raw.name === 'string' || isObject(raw.name) ? raw.name : raw.id) as ManifestText;
  return { key: `${pluginId}/${raw.id}`, pluginId, pluginName, id: raw.id, name, dark, light, projectColors, dropped };
}

/** The themes of the plugins that are on for the app and ready, in their order. */
export function themesOf(list: PluginView[]): PluginTheme[] {
  const out: PluginTheme[] = [];
  for (const p of list) {
    if (!p.choices.enabled || p.state.kind !== 'ready' || !p.manifest) continue;
    const themes = p.manifest.contributes.themes;
    if (!Array.isArray(themes)) continue;
    for (const raw of themes) {
      const theme = filterTheme(raw, p.id, p.name);
      if (theme && !out.some((t) => t.key === theme.key)) out.push(theme);
    }
  }
  return out;
}

/** A theme's name in `lang`, with its plugin's when they differ: « Solarized · Thèmes de Jean ». */
export function themeLabel(theme: PluginTheme, lang: string): string {
  const name = textOf(theme.name, lang) || theme.id;
  return name === theme.pluginName ? name : `${name} · ${theme.pluginName}`;
}

/**
 * The style sheet of a theme: its dark values over src/app.css's `:root`, its light ones over
 * `:root[data-theme='light']` (a stronger selector, whatever the order of the sheets). Every name and value went
 * through the filter: nothing else is written.
 */
export function themeCss(theme: Pick<PluginTheme, 'dark' | 'light'>): string {
  const rule = (selector: string, values: Record<string, string>) => {
    const lines = Object.entries(values)
      .filter(([token, value]) => 'value' in filterValue(token, value))
      .map(([token, value]) => `  ${token}: ${value};`);
    return lines.length ? `${selector} {\n${lines.join('\n')}\n}\n` : '';
  };
  return rule(":root:root:not([data-theme='light'])", theme.dark) + rule(":root:root[data-theme='light']", theme.light);
}
