import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { tokensOf } from '../test/color';
import { filterTheme, filterValue, isColor, MAX_PALETTE, THEME_TOKENS, themeCss, themeLabel, themesOf } from './plugin-themes';
import type { PluginView } from './plugins';

const APP_CSS = fs.readFileSync(path.resolve(import.meta.dirname, '..', 'app.css'), 'utf8');
const DARK = tokensOf(APP_CSS, ':root');
const LIGHT = tokensOf(APP_CSS, ":root[data-theme='light']");
/** The tokens no theme gives: the project's colour (the palette's) and the form. */
const NOT_THEMED = ['--p', '--font', '--mono', '--ease', '--r', '--r-sm', '--rail', '--side', '--mac-corner'];

describe('the tokens a theme may give', () => {
  it('are those of src/app.css, but the project colour and the form', () => {
    expect(Object.keys(THEME_TOKENS).sort()).toEqual(
      Object.keys(DARK)
        .filter((t) => !NOT_THEMED.includes(t))
        .sort(),
    );
    for (const t of Object.keys(LIGHT)) expect(t in THEME_TOKENS, t).toBe(true);
  });

  it('take every value Escouade gives them, in both themes', () => {
    for (const values of [DARK, LIGHT])
      for (const [token, value] of Object.entries(values)) {
        if (NOT_THEMED.includes(token)) continue;
        expect(filterValue(token, value), `${token}: ${value}`).toEqual({ value });
      }
  });
});

describe('the filter of the values', () => {
  it('keeps colours: hex, functions, color-mix and relative colours on tokens of the list', () => {
    for (const v of [
      '#fff',
      '#ffff',
      '#0040c0',
      '#0040c0cc',
      'rgb(0 64 192)',
      'rgba(255, 255, 255, 0.1)',
      'hsl(210deg 40% 50% / .5)',
      'oklch(0.72 0.12 48)',
      'oklch(from var(--wait) l c h / 0.2)',
      'color-mix(in oklch, var(--p) 8%, #000)',
      'color-mix(in srgb, white 20%, transparent)',
      'color(display-p3 1 0 0)',
      'var(--text)',
      'transparent',
    ])
      expect(filterValue('--frame', v), v).toEqual({ value: v });
  });

  it('refuses url(), other functions, free CSS and what is no colour', () => {
    for (const v of [
      'url(https://evil.example/x.png)',
      'URL(x)',
      "url('x')",
      'image-set("x.png" 1x)',
      'expression(alert(1))',
      'attr(data-x)',
      'env(safe-area-inset-top)',
      'red; background: url(x)',
      '#000 } body { display: none',
      '#000 !important',
      '"#000"',
      '\\23 000',
      'var(--evil)',
      'var(--text, url(x))',
      'var(--font)',
      'red',
      '#12',
      '#1234567',
      '12px',
      '#000 #fff',
      'oklch(0.5 0.1 20',
      'oklch 0.5 0.1 20)',
      '10%',
      '/* x */ #000',
    ])
      expect(filterValue('--frame', v), v).toEqual({ reason: 'value' });
  });

  it('refuses a token that is not of the list, a value that is no text, one too long', () => {
    expect(filterValue('--font', 'Comic Sans')).toEqual({ reason: 'token' });
    expect(filterValue('--p', '#f00')).toEqual({ reason: 'token' });
    expect(filterValue('color', '#f00')).toEqual({ reason: 'token' });
    expect(filterValue('__proto__', '#f00')).toEqual({ reason: 'token' });
    expect(filterValue('--text', 12)).toEqual({ reason: 'type' });
    expect(filterValue('--text', `rgb(${'0 '.repeat(100)})`)).toEqual({ reason: 'long' });
  });

  it('holds the proportions, the number and the channels to their kind', () => {
    expect(filterValue('--glow', '20%')).toEqual({ value: '20%' });
    expect(filterValue('--sel', '100%')).toEqual({ value: '100%' });
    expect(filterValue('--glow', '120%')).toEqual({ reason: 'value' });
    expect(filterValue('--glow', '#fff')).toEqual({ reason: 'value' });
    expect(filterValue('--shade', '0.5')).toEqual({ value: '0.5' });
    expect(filterValue('--shade', '2')).toEqual({ reason: 'value' });
    expect(filterValue('--fg-rgb', '0, 0, 0')).toEqual({ value: '0, 0, 0' });
    expect(filterValue('--fg-rgb', '0, 0, 300')).toEqual({ reason: 'value' });
    expect(filterValue('--fg-rgb', 'url(x)')).toEqual({ reason: 'value' });
  });

  it('takes no var() in a project colour', () => {
    expect(isColor('var(--text)', false)).toBe(false);
    expect(isColor('oklch(0.7 0.1 20)', false)).toBe(true);
  });
});

describe('a theme of a plugin', () => {
  const raw = {
    id: 'solarized',
    name: { fr: 'Solarisé', en: 'Solarized' },
    dark: { '--frame': '#002b36', '--text': '#839496', '--font': 'serif', '--panel': 'url(x)' },
    light: { '--frame': '#fdf6e3' },
    projectColors: ['#b58900', '#cb4b16', '#b58900', 'var(--text)', 7],
  };

  it('keeps what passes the filter and says what it left out', () => {
    const t = filterTheme(raw, 'dev.exemple.sol', 'Sol')!;
    expect(t.key).toBe('dev.exemple.sol/solarized');
    expect(t.dark).toEqual({ '--frame': '#002b36', '--text': '#839496' });
    expect(t.light).toEqual({ '--frame': '#fdf6e3' });
    expect(t.projectColors).toEqual(['#b58900', '#cb4b16']);
    expect(t.dropped).toEqual([
      { mode: 'dark', token: '--font', reason: 'token' },
      { mode: 'dark', token: '--panel', reason: 'value' },
      { mode: 'projectColors', token: '3', reason: 'value' },
      { mode: 'projectColors', token: '4', reason: 'type' },
    ]);
    expect(themeLabel(t, 'fr')).toBe('Solarisé · Sol');
    expect(themeLabel(t, 'en')).toBe('Solarized · Sol');
  });

  it('is none without an id, and holds its palette to its size', () => {
    expect(filterTheme({ dark: {} }, 'p')).toBeNull();
    expect(filterTheme({ id: '../x' }, 'p')).toBeNull();
    expect(filterTheme('x', 'p')).toBeNull();
    const many = filterTheme({ id: 'm', projectColors: Array.from({ length: 30 }, (_, i) => `oklch(0.7 0.1 ${i * 10})`) }, 'p')!;
    expect(many.projectColors).toHaveLength(MAX_PALETTE);
  });

  it('writes its values over the dark and the light tokens, and nothing else', () => {
    const t = filterTheme(raw, 'p')!;
    expect(themeCss(t)).toBe(
      ":root:root:not([data-theme='light']) {\n  --frame: #002b36;\n  --text: #839496;\n}\n" +
        ":root:root[data-theme='light'] {\n  --frame: #fdf6e3;\n}\n",
    );
    expect(themeCss({ dark: { '--frame': 'red;}*{x:url(y)' }, light: {} })).toBe('');
    expect(themeCss({ dark: {}, light: {} })).toBe('');
  });

  it('comes from the plugins on for the app and ready only', () => {
    const plugin = (id: string, enabled: boolean, ready = true) =>
      ({
        id,
        name: id,
        choices: { enabled, projects: {}, revoked: [], settings: {} },
        state: ready ? { kind: 'ready' } : { kind: 'disabled', reason: 'api' },
        manifest: { contributes: { themes: [raw, { id: 'solarized' }, { nope: 1 }] } },
      }) as unknown as PluginView;
    expect(themesOf([plugin('a', true), plugin('b', false), plugin('c', true, false)]).map((t) => t.key)).toEqual(['a/solarized']);
  });
});
