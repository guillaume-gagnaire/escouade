// The contrast of the app's tokens (src/app.css), in both themes, for every project colour and Escouade's teal: the
// check of design/app-redesign/contrast.mjs (which reads the preview in a browser), on the app's own two tables. Every
// text token on every surface it sits on reaches WCAG AA, 4.5:1; marks that are not text (dots, rings), 3:1.

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { paint, parse, ratio, substitute, tokensOf } from '../test/color';
import { BRAND, PROJECT_COLORS } from './theme';

const CSS = readFileSync('src/app.css', 'utf8');
const DARK = tokensOf(CSS, ':root');
const THEMES = { sombre: DARK, clair: { ...DARK, ...tokensOf(CSS, ":root[data-theme='light']") } };
const COLORS = [...PROJECT_COLORS, BRAND];

// A surface is a stack of layers, bottom first.
const FRAME = ['var(--frame)'];
const PANEL = ['var(--panel)'];
const S: Record<string, string[]> = {
  cadre: FRAME,
  'cadre + lumière': [...FRAME, 'color-mix(in oklch, var(--p) var(--glow), transparent)'],
  'cadre + survol': [...FRAME, 'var(--hover)'],
  panneau: PANEL,
  'panneau + survol': [...PANEL, 'var(--hover)'],
  relief: ['var(--raise)'],
  'relief 2': ['var(--raise-2)'],
  puits: ['var(--well)'],
  'carte qui attend': [...FRAME, 'var(--wait-bg)'],
  'puce du projet': [...PANEL, 'var(--p-soft)'],
};
const ALL = Object.keys(S);
const PLAIN = ['cadre', 'cadre + lumière', 'cadre + survol', 'panneau', 'panneau + survol', 'relief', 'relief 2', 'puits'];
const CODE = ['puits', 'relief', 'panneau'];
const TEXT = 4.5;
const MARK = 3;
const PAIRS: [string, string[], number][] = [
  ...['--text', '--soft', '--muted', '--dim'].map((t): [string, string[], number] => [t, ALL, TEXT]),
  ['--text-ai', ['panneau', 'puits'], TEXT],
  ...[
    '--p-ink',
    '--run',
    '--del',
    '--add',
    '--git-m',
    '--warn-ink',
    '--teal-ink',
    '--teal-ink-2',
    '--done',
    '--heat-hot',
    '--heat-mid',
    '--heat-cool',
  ].map((t): [string, string[], number] => [t, PLAIN, TEXT]),
  ['--wait', [...PLAIN, 'carte qui attend'], TEXT],
  ...['--code-fg', '--code-dim', '--syn-k', '--syn-f', '--syn-s', '--syn-t', '--syn-n', '--ansi-y', '--ansi-cy', '--ansi-cy-2'].map(
    (t): [string, string[], number] => [t, CODE, TEXT],
  ),
  // xterm's white and bright white, in the terminal's well.
  ...['--ansi-w', '--ansi-w-2'].map((t): [string, string[], number] => [t, ['puits'], TEXT]),
  // The project's own colour is the product's: its dot and its bars always carry the project's name.
  ...['--idle', '--ring', '--brand'].map((t): [string, string[], number] => [t, PLAIN, MARK]),
];
// Text on a filled colour.
const ON: [string, string, string[]][] = [
  ['bouton principal', '--on-primary', ['var(--primary)']],
  ['bouton principal survolé', '--on-primary', ['var(--primary-hover)']],
  ['pastille qui attend', '--on-wait', ['var(--wait)']],
  ['sélection du terminal', '--term-sel-fg', ['var(--term-sel-bg)']],
  ['occurrence courante (éditeur)', '--on-hit', ['var(--well)', 'color-mix(in oklch, var(--p) 55%, transparent)']],
  ['texte sélectionné', '--text', ['var(--panel)', 'color-mix(in oklch, var(--p) var(--sel), transparent)']],
];

/** The worst ratio of each pair over every colour, in a theme. */
function worst(tokens: Record<string, string>) {
  const out = new Map<string, { r: number; min: number; color: string }>();
  for (const color of COLORS) {
    const t = { ...tokens, '--p': color };
    const see = (layers: string[]) => paint(layers.map((l) => parse(substitute(l, t))));
    const note = (pair: string, r: number, min: number) => {
      const w = out.get(pair);
      if (!w || r < w.r) out.set(pair, { r, min, color });
    };
    for (const [token, surfaces, min] of PAIRS)
      for (const s of surfaces) note(`${token} sur ${s}`, ratio(see([...S[s], `var(${token})`]), see(S[s])), min);
    for (const [label, token, layers] of ON) note(`${token} sur ${label}`, ratio(see([...layers, `var(${token})`]), see(layers)), TEXT);
  }
  return out;
}

describe('the contrast of the tokens', () => {
  it('resolves the colours as the browser does', () => {
    // What contrast.mjs reads in Chromium, for the preview's same tokens.
    const see = (p: string, layers: string[], tokens = DARK) => paint(layers.map((l) => parse(substitute(l, { ...tokens, '--p': p }))));
    const light = THEMES.clair;
    // A grey's hue is powerless in a mix: the frame takes the project's.
    expect(see('oklch(0.72 0.12 48)', ['var(--frame)'])).toEqual([24, 17, 14]);
    expect(see('oklch(0.72 0.12 48)', ['var(--frame)'], light)).toEqual([242, 234, 229]);
    expect(see('oklch(0.74 0.115 190)', ['var(--raise-2)'])).toEqual([28, 38, 37]);
    expect(see('oklch(0.72 0.12 250)', ['var(--well)'], light)).toEqual([242, 245, 249]);
    // Out of the gamut: clipped.
    expect(see('oklch(0.72 0.12 48)', ['var(--wait)'])).toEqual([248, 193, 83]);
    expect(see('oklch(0.72 0.12 48)', ['var(--wait)'], light)).toEqual([138, 86, 0]);
    // Translucent layers, relative colours.
    expect(see('oklch(0.72 0.12 48)', ['#08090a', 'rgba(255, 255, 255, 0.5)'])).toEqual([132, 132, 133]);
    expect(see('oklch(0.72 0.12 48)', ['var(--frame)', 'var(--wait-bg)'])).toEqual([40, 29, 19]);
    expect(see('oklch(0.72 0.12 300)', ['var(--p-ink)'], light)).toEqual([94, 80, 120]);
    // To a unit, where the canvas rounds otherwise.
    const glow = see('oklch(0.8 0.02 60)', ['var(--frame)', 'color-mix(in oklch, var(--p) var(--glow), transparent)']);
    expect(glow.map((c, i) => Math.abs(c - [46, 43, 40][i]))).toEqual([1, 0, 0]);
  });

  for (const [theme, tokens] of Object.entries(THEMES))
    it(`reaches AA for every text and 3:1 for every mark, in the ${theme} theme, for every project colour`, () => {
      const bad = [...worst(tokens)].filter(([, w]) => w.r < w.min).map(([pair, w]) => `${pair} : ${w.r.toFixed(2)} (${w.color})`);
      expect(bad).toEqual([]);
    });

  it('has the preview’s values, in both themes', () => {
    const html = readFileSync('design/app-redesign/index.html', 'utf8');
    const style = html.slice(html.indexOf('<style>'), html.indexOf('</style>'));
    for (const [selector, app] of [
      [':root', DARK],
      [":root[data-theme='light']", tokensOf(CSS, ":root[data-theme='light']")],
    ] as const) {
      const preview = tokensOf(style, selector);
      const differ = Object.keys(app).filter((k) => k in preview && preview[k] !== app[k]);
      expect(differ, selector).toEqual([]);
    }
  });
});
