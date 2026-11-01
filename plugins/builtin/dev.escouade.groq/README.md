# Groq

`dev.escouade.groq`, a plugin [Escouade](https://escouade.dev) gives: shipped with the app, signed with it, updated with it
(its version is Escouade's). It is the voice service Groq of Réglages › Voix, taken out of the core (INT-13,
`docs/PLUGINS.md` §1.4 and §4.13): Whisper et Orpheus.

Le service de voix Groq de Réglages › Voix, sorti du cœur : dictée d'un fichier, transcription en direct (ce service n’en offre pas), voix de l'agent, catalogue des modèles et des voix.

## Use it

Nothing to do: it is in the Plugins view under « Livrés avec Escouade », on. Add your key in Réglages › Voix, as before. Turn the
plugin off and the service goes with it (the dictation falls back to local). A plugin of the store that serves the same
service id replaces it.

## What it may do

- reach `api.groq.com` (permission `network`), nothing else;
- hear the sound the user dictates, and the text the agent reads, for this service alone;
- receive the key of the service in each call (it stays in the app's keychain, entry `dictation-groq`; the plugin keeps nothing and writes nothing).

Default model: `whisper-large-v3-turbo`.

## Accounts

One key per service, in the system's keychain (`dictation-groq`). 

## Run it alone

Against a fake Escouade, with the app's own `escouade-plugin` and `escouade-plugins` (`cargo build --bin escouade-plugin --bin escouade-plugins` in `src-tauri/`):

```sh
escouade-plugin test plugins/builtin/dev.escouade.groq --builtin
```

Its tests are those of the service as it was in the core (`src-tauri/src/voice/*_tests.rs`, against fake servers) and
`src-tauri/src/voice/plugin_tests.rs` (the same fake servers, through the plugin's process).

| File | What it is |
| ---- | ---------- |
| `escouade-plugin.json` | The manifest |
| `README.md` | This page |

Its program is `src-tauri/src/bin/escouade-plugins/voice.rs` (`escouade-plugins voice-groq`), on `src-tauri/src/voice/plugin_guest.rs`.
