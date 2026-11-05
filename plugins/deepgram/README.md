# Deepgram

`dev.escouade.deepgram`, a plugin [Escouade](https://escouade.dev) gives: shipped with the app, signed with it, updated with it
(its version is Escouade's). It is the voice service Deepgram of Réglages › Voix, taken out of the core (INT-13,
`docs/PLUGINS.md` §1.4 and §4.13): Nova et Aura.

Le service de voix Deepgram de Réglages › Voix, sorti du cœur : dictée d'un fichier, transcription en direct, voix de l'agent, catalogue des modèles et des voix.

## Use it

Nothing to do: it is in the Plugins view under « Livrés avec Escouade », on. Add your key in Réglages › Voix, as before. Turn the
plugin off and the service goes with it (the dictation falls back to local). A plugin of the store that serves the same
service id replaces it.

## What it may do

- reach `api.deepgram.com` (permission `network`), nothing else;
- hear the sound the user dictates, and the text the agent reads, for this service alone;
- receive the key of the service in each call (it stays in the app's keychain, entry `dictation-deepgram`; the plugin keeps nothing and writes nothing).

Default model: `nova-3`.

## Accounts

One key per service, in the system's keychain (`dictation-deepgram`). 

## Run it alone

Against a fake Escouade, with the app's own `escouade-plugin` and `escouade-plugins` (`cargo build --bin escouade-plugin --bin escouade-plugins` in `src-tauri/`):

```sh
escouade-plugin test plugins/deepgram --builtin
```

Its tests are those of the service as it was in the core (`src-tauri/src/voice/*_tests.rs`, against fake servers) and
`src-tauri/src/voice/plugin_tests.rs` (the same fake servers, through the plugin's process).

| File | What it is |
| ---- | ---------- |
| `escouade-plugin.json` | The manifest |
| `README.md` | This page |

Its program is `src-tauri/src/bin/escouade-plugins/voice.rs` (`escouade-plugins voice-deepgram`), on `src-tauri/src/voice/plugin_guest.rs`.
