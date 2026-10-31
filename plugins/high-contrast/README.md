# High contrast · Contraste élevé

`dev.escouade.high-contrast`, a plugin of [Escouade](https://escouade.dev)'s own hand.

A high-contrast theme, dark and light: pure text, sharp borders, plain status colors, and six project colors that stand
apart. The text is at least 7:1 on every surface (WCAG AAA), the accent and the status colors 4.5:1, the borders 3:1.

Un thème à fort contraste, sombre et clair : le texte au moins à 7:1 sur chaque surface, l'accent et les couleurs d'état
à 4,5:1, les bordures à 3:1.

## Use it

Install it from the Plugins view of Escouade (or from <https://store.escouade.dev/plugins/dev.escouade.high-contrast>),
then pick « High contrast » in Settings › Application › Theme. It is declarative: no program, no network, no secret.

## What is in this folder

| File                   | What it is                                                     |
| ---------------------- | -------------------------------------------------------------- |
| `escouade-plugin.json` | The manifest: the theme (`contributes.themes`), dark and light |
| `test/theme.test.mjs`  | The contrast ratios, checked on the manifest                   |
| `CHANGELOG.md`         | Its versions                                                   |

## Check it

```sh
node --test                                 # the contrast ratios
escouade-plugin validate . --first-party    # the manifest
escouade-plugin test . --first-party        # Escouade's conformance bench
```

`--first-party`: its id `dev.escouade.*` is kept for Escouade's own plugins. The store takes it from this folder of
Escouade's repository alone; its tags are `plugins/high-contrast/v1.2.3`.

MIT license (`LICENSE`).
