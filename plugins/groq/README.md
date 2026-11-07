# Groq

`dev.escouade.groq`, a plugin of [Escouade](https://escouade.dev), published on its store like any other: it has its own version,
and can be uninstalled. It is the voice service Groq of Réglages › Voix, taken out of the core (INT-13,
`docs/PLUGINS.md` §1.4 and §4.13): Whisper et Orpheus.

Le service de voix Groq de Réglages › Voix, sorti du cœur : dictée d'un fichier, voix de l'agent, catalogue des modèles et des voix.

## Use it

It is in the Plugins view, on. Add your key in Réglages › Voix, as before. Turn the plugin off and the service goes with it (the
dictation falls back to local). A plugin of another author cannot serve this service: the key of a service only goes to Escouade's own
plugin.

## What it may do

- reach `api.groq.com` (permission `network`), nothing else;
- hear the sound the user dictates, and the text the agent reads, for this service alone;
- receive the key of the service in each call (it stays in the app's keychain, entry `dictation-groq`; the plugin keeps nothing and writes nothing).

Default model: `whisper-large-v3-turbo`.

## Accounts

One key per service, in the system's keychain (`dictation-groq`). 

## Run it alone

A module: `sh scripts/plugins-wasm.sh groq` builds it against this repository's crates, then, with the app's own `escouade-plugin`
(`cargo build --bin escouade-plugin` in `src-tauri/`):

```sh
escouade-plugin test plugins/groq --first-party --escouade 2.2.0 --wasm
```

Its service is the crate `crates/escouade-voice`, and its tests are there (`cargo test -p escouade-voice`, the reactor played
without a module) and in `src-tauri/src/voice/*_tests.rs` (the module, against fake servers). `voice/parity_tests.rs` compares it
to what the native plugin did (`tests/fixtures/plugins/parity/voice-groq.json`).

| File | What it is |
| ---- | ---------- |
| `escouade-plugin.json` | The manifest |
| `Cargo.toml` | The crate `escouade-plugin-groq` |
| `src/lib.rs` | One line: the service of `escouade-voice` |
| `README.md` | This page |
