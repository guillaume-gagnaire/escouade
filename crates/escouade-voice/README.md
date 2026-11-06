# escouade-voice

The voice services on line of [Escouade](https://escouade.dev): OpenAI, Groq, Mistral, ElevenLabs and Deepgram. What each
asks, what its answers mean, and the service of `voice/*` as a plugin in WASM runs it: the dictation by `net/fetch`, the
reading by `net/stream`, the live transcription by `ws/*`.

```rust
fn run() -> i32 {
    escouade_voice::serve::run("deepgram", serde_json::json!({}), None)
}

escouade_voice::protocol::module!(run);
```

Nothing here waits: a reactor turns on the events of the host, one line at a time, and `cargo test` plays it without a
module. The key of a service comes with each call and is kept nowhere.

MIT.
