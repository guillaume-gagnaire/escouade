# Developing High contrast

`dev.escouade.high-contrast` is a plugin of Escouade’s own hand, in `plugins/high-contrast/` of its repository. It is declarative: everything is in its manifest.

## Its folder

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | the manifest: the theme (`contributes.themes`), dark and light, and its project palette |
| `test/theme.test.mjs` | the contrast ratios, checked on the manifest |
| `README.fr.md`, `README.en.md` | its page, on the store and in the app |
| `docs/fr/`, `docs/en/` | this documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | its versions, its license (MIT) |

Its tests check, dark and light, on `--frame`, `--raise`, `--raise-2` and `--well`: the text (`--text`, `--soft`, `--muted`, `--dim`) at 7:1 at least, the accent and the statuses (`--p-ink`, `--ring`, `--run`, `--wait`, `--done`, `--add`, `--del`) at 4.5:1, the borders (`--line`, `--line-2`, `--line-3`) at 3:1; and that both modes set the same tokens, with six project colors.

## Checking it

In its folder:

```sh
node --test                                 # the contrast ratios, no dependency
escouade-plugin validate . --first-party    # the manifest
escouade-plugin test . --first-party        # Escouade's conformance bench
```

To see it, load its folder in development (Plugins view › **In development** › **Load a folder…**), pick it in Settings › General › Colors, and switch from Dark to Light. Escouade reads it again at each change of a file.

`node plugins/ci.mjs`, from the repository’s root (with `ESCOUADE_PLUGIN_BIN` naming the app’s `escouade-plugin`), does all of this for each plugin of the folder, with its LICENSE and CHANGELOG.

`--first-party`: its id `dev.escouade.*` is kept for Escouade’s own plugins. The store takes it from this folder of Escouade’s repository alone; its tags are `plugins/high-contrast/v1.2.3`.
