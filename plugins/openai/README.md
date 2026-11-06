# OpenAI

`dev.escouade.openai`, a plugin [Escouade](https://escouade.dev) gives: shipped with the app, signed with it, updated with it
(its version is Escouade's). It is the voice service OpenAI of Réglages › Voix, taken out of the core (INT-13,
`docs/PLUGINS.md` §1.4 and §4.13): GPT-4o.

Le service de voix OpenAI de Réglages › Voix, sorti du cœur : dictée d'un fichier, transcription en direct, voix de l'agent, catalogue des modèles et des voix.

## Use it

Nothing to do: it is in the Plugins view under « Livrés avec Escouade », on. Add your key in Réglages › Voix, as before. Turn the
plugin off and the service goes with it (the dictation falls back to local). A plugin of the store that serves the same
service id replaces it.

## What it may do

- reach `api.openai.com` (permission `network`), nothing else;
- hear the sound the user dictates, and the text the agent reads, for this service alone;
- receive the key of the service in each call (it stays in the app's keychain, entry `dictation-openai`; the plugin keeps nothing and writes nothing).

Default model: `gpt-4o-mini-transcribe`; live model: `gpt-live-transcribe`.

## Accounts

One key per service, in the system's keychain (`dictation-openai`). 

## Run it alone

A module: `sh scripts/plugins-wasm.sh openai` builds it against this repository's crates, then, with the app's own `escouade-plugin`
(`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin test plugins/openai --first-party --escouade 2.2.0 --wasm
```

Its service is the crate `crates/escouade-voice`, and its tests are there (`cargo test -p escouade-voice`, the reactor played
without a module) and in `src-tauri/src/voice/*_tests.rs` (the module, against fake servers). `voice/parity_tests.rs` compares it
to what the native plugin did (`tests/fixtures/plugins/parity/voice-openai.json`).

| File | What it is |
| ---- | ---------- |
| `escouade-plugin.json` | The manifest |
| `Cargo.toml` | The crate `escouade-plugin-openai` |
| `src/lib.rs` | One line: the service of `escouade-voice` |
| `README.md` | This page |
