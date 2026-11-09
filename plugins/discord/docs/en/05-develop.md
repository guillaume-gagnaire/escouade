# Developing Discord

`dev.escouade.discord` is a plugin of Escouade’s own hand, in `plugins/discord/` of its repository. It is declarative: everything is in its manifest.

## Its folder

| File | What it is |
| --- | --- |
| `escouade-plugin.json` | the manifest: its two webhooks (`contributes.webhooks`), its secret and its permissions |
| `test/webhooks.test.mjs` | which facts post what, and what it may not do |
| `README.fr.md`, `README.en.md` | its page, on the store and in the app |
| `docs/fr/`, `docs/en/` | this documentation (`store.docs`) |
| `CHANGELOG.md`, `LICENSE` | its versions, its license (MIT) |

A webhook has an address (`url`, here `{{secrets.webhook}}`), the facts that set it off (`events`), a filter on their values (`when`, here `data.to` at `review` or `data.status` at `waiting`) and a body (`body`), whose `{{…}}` are filled from the fact. The “Events” page of Escouade’s documentation says all a webhook can declare.

## Checking it

In its folder:

```sh
node --test                                 # the webhooks, on the manifest, no dependency
escouade-plugin validate . --first-party    # the manifest
escouade-plugin test . --first-party        # Escouade's conformance bench
```

Its tests fill the templates as Escouade does and check that a ticket reaching `review` and a `waiting` agent post the right text, that nothing else posts, that mentions are off, and that it asks only for what it uses.

`node plugins/ci.mjs`, from the repository’s root (with `ESCOUADE_PLUGIN_BIN` naming the app’s `escouade-plugin`), does all of this for each plugin of the folder, with its LICENSE and CHANGELOG.

`--first-party`: its id `dev.escouade.*` is kept for Escouade’s own plugins. The store takes it from this folder of Escouade’s repository alone; its tags are `plugins/discord/v1.2.3`.
