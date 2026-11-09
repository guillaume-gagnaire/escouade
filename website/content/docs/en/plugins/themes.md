# Themes

A **declarative** plugin can give Escouade other colors: values for the interface’s tokens, dark and light, and a palette of project colors. It is picked in **Settings › General › Colors** and applies at once, terminals and editor included.

## A theme in the manifest

The “High contrast” plugin, whole: one theme, dark and light, and its project palette.

`plugins/high-contrast/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.high-contrast",
  "name": { "fr": "Contraste élevé", "en": "High contrast" },
  "version": "1.1.2",
  "description": {
    "fr": "Un thème à fort contraste, clair et sombre : texte pur, bordures nettes, couleurs d'état franches.",
    "en": "A high-contrast theme, light and dark: pure text, sharp borders, plain status colors."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/high-contrast",
  "escouade": { "api": "1", "min": "2.1.0" },
  "contributes": {
    "themes": [
      {
        "id": "high-contrast",
        "name": { "fr": "Contraste élevé", "en": "High contrast" },
        "dark": {
          "--frame": "#000000",
          "--panel": "color-mix(in oklch, var(--p) 4%, #050505)",
          "--raise": "#141414",
          "--raise-2": "#1f1f1f",
          "--well": "#000000",
          "--line": "#8a8a8a",
          "--line-2": "#b0b0b0",
          "--line-3": "#d8d8d8",
          "--text": "#ffffff",
          "--soft": "#f0f0f0",
          "--muted": "#d0d0d0",
          "--dim": "#b8b8b8",
          "--p-ink": "#ffd400",
          "--ring": "#ffd400",
          "--run": "#4dff88",
          "--wait": "#ffb000",
          "--done": "#7ab8ff",
          "--add": "#4dff88",
          "--del": "#ff6b6b"
        },
        "light": {
          "--frame": "#ffffff",
          "--panel": "#ffffff",
          "--raise": "#f2f2f2",
          "--raise-2": "#e6e6e6",
          "--well": "#ffffff",
          "--line": "#5c5c5c",
          "--line-2": "#3d3d3d",
          "--line-3": "#1f1f1f",
          "--text": "#000000",
          "--soft": "#0a0a0a",
          "--muted": "#262626",
          "--dim": "#3d3d3d",
          "--p-ink": "#0040c0",
          "--ring": "#0040c0",
          "--run": "#006b2e",
          "--wait": "#8a4b00",
          "--done": "#0040c0",
          "--add": "#006b2e",
          "--del": "#b00020"
        },
        "projectColors": ["#ffd400", "#00e0ff", "#ff5cf0", "#7cff4f", "#ff8a00", "#b48cff"]
      }
    ]
  },
  "store": { "docs": "docs" }
}
```

Each entry of `contributes.themes` is a theme:

- `id`: its id within the plugin (letters, digits, `.`, `_`, `-`);
- `name`: its name, in one language or per language; without it, its `id`;
- `dark` and `light`: the values of the tokens, either or both;
- `projectColors`: the palette offered for projects (24 colors at most), in place of Escouade’s.

A plugin may bring several themes. In the “Colors” menu, each is called by its name, followed by the plugin’s when they differ.

## What a theme leaves out

**A missing token keeps Escouade’s value.** A theme that only gives `--frame` changes the window’s frame and nothing else; a theme with only `dark` leaves light mode to Escouade. System, Light or Dark stays the user’s choice, in the “Theme” row above.

`--p`, the color of the open project, is not a theme token: the theme gives the palette the user picks it from. Escouade’s surfaces are tinted with it (`color-mix(in oklch, var(--p) 8%, …)`); a theme can keep that tint by writing its surfaces the same way.

## The tokens

They are those of `src/app.css`, listed in `DESIGN.md` (“Les jetons”):

| Family | Tokens |
| --- | --- |
| Project | `--p-ink`, `--p-soft`, `--sel`, `--glow`, `--brand` |
| Surfaces | `--frame`, `--panel`, `--raise`, `--raise-2`, `--well`, `--scrim`, `--shade`, `--drop-base`, `--vo-base`, `--orb-base` |
| Lines and hover | `--fg-rgb`, `--line`, `--line-2`, `--line-3`, `--hover`, `--sep`, `--ring` |
| Text | `--text`, `--soft`, `--muted`, `--dim`, `--text-ai`, `--idle` |
| States | `--run`, `--wait`, `--wait-bg`, `--wait-line`, `--done`, `--add`, `--del`, `--git-m`, `--warn-ink`, `--info`, `--teal-ink`, `--teal-ink-2`, `--disc`, `--brain` |
| Filled button and inks | `--primary`, `--primary-hover`, `--on-primary`, `--on-primary-line`, `--on-primary-kbd`, `--on-primary-dim`, `--on-wait`, `--on-hit`, `--on-tick`, `--on-tile`, `--win-close`, `--on-win-close` |
| Git history | `--lane-main`, `--lane-1` to `--lane-8` |
| Code and terminal | `--code-fg`, `--code-ln`, `--code-nb`, `--code-dim`, `--syn-k`, `--syn-f`, `--syn-s`, `--syn-t`, `--syn-n`, `--ansi-k`, `--ansi-w`, `--ansi-w-2`, `--ansi-y`, `--ansi-cy`, `--ansi-cy-2`, `--term-sel-bg`, `--term-sel-fg` |

Shape is not a theme’s business: fonts, radii, widths of the projects’ rail and of the sidebar (`--font`, `--mono`, `--r`, `--rail`, `--side`…) stay Escouade’s.

Terminals take their colors from `--well`, `--code-fg`, `--text`, the ANSI colors (`--ansi-*`, `--del`, `--add`, `--syn-f`, `--syn-k`, `--code-dim`) and `--p` for the selection; the editor, from `--code-*` and `--syn-*`.

## Allowed values

Escouade filters each value before putting it on the window. What does not pass is left out, and the token keeps Escouade’s value; Settings › General says how many values of the chosen theme were left out.

- **Colors** (almost every token): `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, the functions `rgb()`, `rgba()`, `hsl()`, `hsla()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color()` and `color-mix()`, relative colors (`oklch(from var(--wait) l c h / 0.1)`), `transparent`, `white`, `black`, and `var()` of another token of the list (or of `--p`). One color per token, 160 characters at most.
- **The proportions** `--sel` and `--glow`: a percentage from 0 to 100% (`26%`).
- **The number** `--shade`, which softens shadows: from 0 to 1 (`0.3`).
- **The three channels** `--fg-rgb`, the ink of lines: `15, 17, 22`.
- **The palette** `projectColors`: colors, without `var()`.

Never `url()`, another function (`image-set()`, `attr()`, `env()`…), quotes, backslashes, `;`, braces or `!important`: a theme loads nothing, writes no CSS of its own, and cannot hide one of Escouade’s requests (a permission, a question). A token that is not in the list is ignored.

## What the user sees

- **The choice.** The “Colors” row, under “Theme”: “Escouade”, then the themes of the plugins installed and turned on for the app. With no theme installed, a “Browse themes” button opens the Plugins view on its “Themes” category.
- **At once.** The window, the open terminals (without reopening them) and the editor take the theme as soon as it is chosen; the palette replaces Escouade’s in the “New project” wizard, in Settings › Projects › Project and in a tile’s menu. A project keeps its color: if it is not in the palette, it is added at the end.
- **At startup.** The chosen theme is kept with the settings, and put on before the first paint.
- **The plugin removed.** Turned off, uninstalled or gone invalid, its theme leaves the window and Escouade’s colors come back; Settings › General says so. The choice stays: the plugin turned on again, its theme comes back.

## Testing a theme

`escouade-plugin validate` checks the manifest. To see the theme, install the plugin’s folder in development (Plugins view › “In development”), pick it in Settings › General and switch from Dark to Light. The “High contrast” example plugin, the manifest above (`plugins/high-contrast` in Escouade’s repository), is a starting point: see [Example plugins](plugins/examples#high-contrast).
