//! The plugin Deepgram (`dev.escouade.deepgram`, INT-13, docs/PLUGINS.md §4.13): a voice service, served by `escouade_voice`.
//! The key stays in the keychain: each call carries it here, and the plugin keeps nothing.

pub fn run() -> i32 {
    escouade_voice::serve::run("deepgram", serde_json::json!({}), None)
}

escouade_voice::protocol::module!(run);
