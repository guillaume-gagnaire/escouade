// « Couleurs » (Réglages › Application › Affichage, PLG-7): the window takes the theme of a plugin, or Escouade's.
// Applied at once and as it changes: a style sheet of its filtered values (lib/plugin-themes.ts) over src/app.css,
// `data-plugin-theme` on the root for what draws its colours itself (the terminals, lib/terminals.ts), and its palette
// in place of the project colours. The plugin switched off, uninstalled or gone invalid, Escouade's colours come back
// (the choice kept, for when it comes back). The theme shown is kept in the page's storage too, filtered again when read
// before the first render: a themed window never flashes Escouade's colours at its start.

import { isColor, themeCss, themesOf, type PluginTheme } from './plugin-themes';
import { plugins } from './plugins-view.svelte';
import { PROJECT_COLORS } from './theme';

/** Where the page keeps the theme shown, for its next start. */
export const PLUGIN_THEME_KEY = 'escouade-plugin-theme';
const STYLE_ID = 'escouade-plugin-theme';

/** The palette of the theme shown (none: Escouade's). */
const shown = $state({ key: '', colors: [] as string[] });

/** The themes of the plugins on for the app, filtered. */
export function pluginThemes(): PluginTheme[] {
  return themesOf(plugins.list);
}

/** The theme `key` names among them (Settings.pluginTheme), or null: Escouade's. */
export function chosenTheme(key: string | undefined): PluginTheme | null {
  return key ? (pluginThemes().find((t) => t.key === key) ?? null) : null;
}

/** The key of the theme the window shows now ('' : Escouade's). */
export const shownThemeKey = () => shown.key;

/** The project colours offered (the theme's palette, else Escouade's), with `current` at the end when it is none of them. */
export function projectPalette(current?: string | null): string[] {
  const base = shown.colors.length ? shown.colors : PROJECT_COLORS;
  return current && !base.includes(current) ? [...base, current] : base;
}

function paint(theme: Pick<PluginTheme, 'key' | 'dark' | 'light' | 'projectColors'> | null) {
  const root = document.documentElement;
  const css = theme ? themeCss(theme) : '';
  let sheet = document.getElementById(STYLE_ID);
  if (css) {
    if (!sheet) {
      sheet = document.createElement('style');
      sheet.id = STYLE_ID;
      document.head.appendChild(sheet);
    }
    if (sheet.textContent !== css) sheet.textContent = css;
  } else sheet?.remove();
  // After the sheet: who follows the attribute reads the new values.
  if (theme) {
    if (root.dataset.pluginTheme !== theme.key) root.dataset.pluginTheme = theme.key;
  } else if (root.dataset.pluginTheme !== undefined) delete root.dataset.pluginTheme;
  shown.key = theme?.key ?? '';
  shown.colors = theme?.projectColors ?? [];
}

/** Shows `theme` (null: Escouade's colours) and keeps it for the next start. */
export function applyPluginTheme(theme: PluginTheme | null) {
  paint(theme);
  try {
    if (theme)
      localStorage.setItem(
        PLUGIN_THEME_KEY,
        JSON.stringify({ key: theme.key, dark: theme.dark, light: theme.light, projectColors: theme.projectColors }),
      );
    else localStorage.removeItem(PLUGIN_THEME_KEY);
  } catch {
    // No storage: shown all the same, from the settings at the next start.
  }
}

/** Before the first render: the theme the page kept, filtered again, until the plugins are read. */
export function earlyPluginTheme() {
  try {
    const kept = JSON.parse(localStorage.getItem(PLUGIN_THEME_KEY) ?? 'null');
    if (!kept || typeof kept.key !== 'string') return;
    const values = (v: unknown) => (v && typeof v === 'object' ? (v as Record<string, string>) : {});
    const colors = Array.isArray(kept.projectColors)
      ? kept.projectColors.filter((c: unknown): c is string => typeof c === 'string' && isColor(c, false))
      : [];
    paint({ key: kept.key, dark: values(kept.dark), light: values(kept.light), projectColors: colors });
  } catch {
    // Nothing kept, or not readable: Escouade's colours until the plugins are read.
  }
}
