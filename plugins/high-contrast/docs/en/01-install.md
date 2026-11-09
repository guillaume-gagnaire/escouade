# Installing and using High contrast

## Install it

1. In Escouade, open the Plugins view (the puzzle piece icon in the projects bar).
2. **Store**, search for “High contrast” (or the **Themes** category), then **Install**. You can also start from its page on <https://store.escouade.dev/plugins/dev.escouade.high-contrast> and **Open in Escouade**.
3. The “Install High contrast?” window says “No program: nothing of it runs, its contributions are read”, and what it adds: a theme, in Settings › General › Colors. **Install**.

## Pick it

1. **Settings › General**, **Colors** row (under **Theme**).
2. Pick **High contrast**.

The theme applies at once: the window, the open terminals (without reopening them) and the editor. It is kept with your settings and set before the first paint at the next start.

**Theme**, just above, still chooses between **System**, **Light** and **Dark**: High contrast has a version for each.

To go back to Escouade’s colors, pick **Escouade** in **Colors**.

## What it changes

- The surfaces, the borders, the texts, the accent, the focus and the status colors (running, waiting, done, code additions and deletions). In dark, the background is black and the accent yellow; in light, the background is white and the accent blue.
- The palette of project colors: six colors (yellow, cyan, magenta, green, orange, purple) replace Escouade’s in the “New project” assistant, in Settings › Projects › Project and in a tile’s menu. A project keeps its color: if it is not in the palette, it is added at the end.

What it does not give keeps Escouade’s color: the filled button (`--primary`), code highlighting (`--syn-*`), the terminals’ ANSI colors (`--ansi-*`), the git history (`--lane-*`). The shape (fonts, radii, widths) always stays Escouade’s.

## What Escouade keeps, and where

- Its files in `~/.escouade/plugins/dev.escouade.high-contrast`, and its install in `~/.escouade/plugins.json`.
- Your choice of colors, with Escouade’s settings.

It has no secret, no setting and no journal (“A plugin without a program has no journal”).

## If it does not work

**“High contrast” is not in Colors.**
The plugin is turned off for the app: turn it on again in the Plugins view, **Installed**. A theme only shows when its plugin is installed and on.

**“The chosen theme is gone: its plugin (High contrast) is turned off or uninstalled. Escouade’s colors are back.”**
The plugin was turned off, uninstalled, or its files changed (its row in **Installed** then says “Its files changed since it was installed (fingerprint …): install it again.”). Your choice stays: with the plugin on again or installed again, its theme comes back.

**“One value of this theme is left out: it is not an allowed token or color.”**
Escouade refused one of its values, which keeps Escouade’s color. With the published version, this should not happen: report it on its store page.
