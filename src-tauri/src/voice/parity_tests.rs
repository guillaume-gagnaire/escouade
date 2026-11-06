//! The parity of the voice in modules (ESC-350, docs/PLUGINS.md, « Ce que P6 a construit »): played by the host of the app,
//! the five services answer what `escouade-plugins voice-<service>` answered, and ask their fake service what it asked: the
//! description, the catalog, a dictation, a reading, what lists the models, and a stream as far as each way it can end.
//!
//! `tests/fixtures/plugins/parity/voice-<id>.json` holds what each native plugin did at 6e0203a5. `ESCOUADE_PARITY_RECORD=1`
//! and `ESCOUADE_NATIVE_PLUGINS=<a program of then>` write them again: the module's text of a network error is the host's
//! and not reqwest's, the rest is compared word for word. No path and no port goes in them: they hold on every system.

use super::live::tests::{fake, Conn, Mode};
use super::online::tests::{nowhere, serve_with, Got, Reply, KEY};
use super::online::{Provider, Spec};
use crate::plugins::wasm::{Fetcher, Net};
use crate::plugins_wasm_parity_tests::{hello, module, root, Hosted, Native, Plugin};
use escouade_voice::wire;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// The sound of the recordings: `live_tests::speech`, as it came out where they were made. A sine is not the same to its
/// last bit on every system, and each body is compared to the bit: computed here, macOS would hear another sound.
fn sound_of(secs: f32) -> Vec<f32> {
    let table = include_str!("../../../tests/fixtures/plugins/parity/voice-sound.b64");
    let bytes = wire::unb64(table.trim()).expect("the table of the sound");
    let (samples, _) = bytes.as_chunks::<4>();
    samples
        .iter()
        .take((48_000.0 * secs) as usize)
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

/// What tells two bodies apart without keeping them.
fn digest(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h = (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}:{}", bytes.len())
}

/// The boundary of a multipart body, which is random.
fn unbound(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("escouade-") {
        out.push_str(&rest[..at]);
        out.push_str("escouade-BOUNDARY");
        let tail = &rest[at + "escouade-".len()..];
        let end = tail
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// A request as the fake service got it, with what may be the client's own left out (its agent, its accepted encodings).
fn asked(g: &Got) -> Value {
    let headers: HashMap<String, String> =
        ["authorization", "xi-api-key", "content-type", "accept"]
            .into_iter()
            .filter_map(|n| Some((n.to_string(), unbound(g.header(n)?))))
            .filter(|(n, v)| n != "accept" || v != "*/*")
            .collect();
    // The boundary is in the body: a body is compared without it.
    let body: Vec<u8> = if g.body.starts_with(b"--escouade-") {
        let mut out = Vec::new();
        let boundary = g
            .body
            .iter()
            .position(|b| *b == b'\r')
            .map_or(&b""[..], |n| &g.body[..n]);
        let mut rest = &g.body[..];
        while let Some(at) = rest.windows(boundary.len()).position(|w| w == boundary) {
            out.extend_from_slice(&rest[..at]);
            out.extend_from_slice(b"--B");
            rest = &rest[at + boundary.len()..];
        }
        out.extend_from_slice(rest);
        out
    } else {
        g.body.clone()
    };
    let shown = match std::str::from_utf8(&body) {
        Ok(text) if body.len() < 2000 => json!(text),
        _ => json!(digest(&body)),
    };
    json!({ "line": g.line, "headers": headers, "body": shown })
}

/// What a stream's fake service got.
fn conn(c: &Conn) -> Value {
    let headers: HashMap<&str, &str> = ["authorization", "xi-api-key"]
        .into_iter()
        .filter_map(|n| Some((n, c.header(n)?)))
        .collect();
    json!({
        "line": c.line, "headers": headers, "texts": c.texts, "audio": digest(&c.pcm),
        "ended": c.ended, "commits": c.commits,
    })
}

fn spec_of(p: Provider, base: &str) -> Value {
    let mut s = Spec::new(p, KEY.into());
    s.base = base.into();
    wire::spec_json(&s)
}

/// What a call gave: its answer, and what came before it.
struct Told {
    answer: Value,
    notes: Vec<Value>,
}

/// An answer, its numbers of time and its network's words apart.
fn said(mut answer: Value) -> Value {
    if answer["error"]["code"] == -32103 {
        answer["error"]["message"] = json!("(network)");
    }
    if answer["result"]["firstMs"].is_number() {
        answer["result"]["firstMs"] = json!("(a time)");
    }
    if answer["result"]["at"].is_number() {
        answer["result"]["at"] = json!("(a time)");
    }
    answer.as_object_mut().map(|o| o.remove("id"));
    answer
}

struct Session<'a> {
    plugin: &'a mut dyn Plugin,
    next: u64,
}

impl Session<'_> {
    fn tell(&mut self, method: &str, params: Value) {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        self.plugin.send(&line.to_string());
    }

    fn call(&mut self, method: &str, params: Value) -> Told {
        self.next += 1;
        let id = self.next;
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.plugin.send(&line.to_string());
        let mut notes = Vec::new();
        loop {
            let line = self
                .plugin
                .recv(Duration::from_secs(20))
                .unwrap_or_else(|| panic!("no answer to {method}"));
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"] == json!(id) {
                return Told {
                    answer: said(v),
                    notes,
                };
            }
            if v.get("method").is_some() {
                notes.push(v);
            }
        }
    }
}

/// The sound of `voice/audio`, whole: how the service cut it is the network's.
fn heard(notes: &[Value]) -> Value {
    let mut pcm = Vec::new();
    let mut rates = Vec::new();
    for n in notes.iter().filter(|n| n["method"] == wire::AUDIO) {
        if let Some(samples) = n["params"]["pcm"].as_str().and_then(wire::unb64) {
            pcm.extend_from_slice(&samples);
        }
        let rate = n["params"]["rate"].clone();
        if !rates.contains(&rate) {
            rates.push(rate);
        }
    }
    json!({ "rates": rates, "pcm": digest(&pcm) })
}

fn partials(notes: &[Value]) -> Vec<Value> {
    notes
        .iter()
        .filter(|n| n["method"] == wire::PARTIAL)
        .map(|n| n["params"]["text"].clone())
        .collect()
}

/// What each service answers a dictation.
fn dictated(p: Provider) -> &'static str {
    match p {
        Provider::Deepgram => {
            r#"{"results":{"channels":[{"alternatives":[{"transcript":" Bonjour   le monde "}]}]}}"#
        }
        _ => r#"{"text":" Bonjour   le monde "}"#,
    }
}

/// A sentence read, as each service streams it.
fn read_back(p: Provider) -> Reply {
    let pcm: Vec<u8> = (0..4800u32)
        .flat_map(|i| ((i % 2000) as i16 - 1000).to_le_bytes())
        .collect();
    let (kind, bytes): (&str, Vec<u8>) = match p {
        Provider::Mistral => {
            let floats: Vec<u8> = (0..2400u32)
                .flat_map(|i| ((i % 100) as f32 / 200.0).to_le_bytes())
                .collect();
            let event = json!({ "audio_data": wire::b64(&floats) });
            (
                "text/event-stream",
                format!("data: {event}\n\ndata: [DONE]\n\n").into_bytes(),
            )
        }
        Provider::Groq => {
            let mut wav = Vec::new();
            wav.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
            wav.extend_from_slice(&16u32.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&1u16.to_le_bytes());
            wav.extend_from_slice(&24_000u32.to_le_bytes());
            wav.extend_from_slice(&48_000u32.to_le_bytes());
            wav.extend_from_slice(&2u16.to_le_bytes());
            wav.extend_from_slice(&16u16.to_le_bytes());
            wav.extend_from_slice(b"data\0\0\0\0");
            wav.extend_from_slice(&pcm);
            ("audio/wav", wav)
        }
        _ => ("audio/pcm", pcm),
    };
    let chunks: Vec<Vec<u8>> = bytes
        .chunks(bytes.len() / 3 + 1)
        .map(<[u8]>::to_vec)
        .collect();
    Reply {
        status: 200,
        content_type: kind.into(),
        chunks,
        gap: Duration::from_millis(2),
    }
}

/// What the API of each service lists.
fn listed(p: Provider, path: &str) -> Reply {
    let text = match p {
        Provider::ElevenLabs if path.starts_with("/v2/voices") => json!({
            "voices": [{ "voice_id": "v1", "name": "Ada", "category": "cloned",
                         "labels": { "gender": "female", "accent": "french" },
                         "verified_languages": [{ "language": "fr-FR" }, { "language": "english" }] }],
            "has_more": false,
        }),
        Provider::ElevenLabs => json!([
            { "model_id": "eleven_flash_v2_5", "name": "Flash", "can_do_text_to_speech": true,
              "languages": [{ "language_id": "fr" }] },
            { "model_id": "eleven_new", "name": "New", "can_do_text_to_speech": true,
              "languages": [{ "language_id": "en" }, { "language_id": "fr" }] },
            { "model_id": "not_tts", "can_do_text_to_speech": false },
        ]),
        Provider::Mistral if path.starts_with("/v1/audio/voices") => json!({
            "items": [{ "slug": "fr_marie_neutral", "name": "Marie", "gender": "female" },
                      { "id": "own", "name": "Mine", "type": "custom", "languages": ["fr"] }],
        }),
        Provider::Deepgram => json!({
            "stt": [{ "architecture": "nova-3", "batch": true }, { "architecture": "flux", "batch": false }],
            "tts": [{ "architecture": "aura-2", "canonical_name": "aura-2-agathe-fr", "name": "agathe",
                      "languages": ["fr-FR"], "metadata": { "accent": "French", "tags": ["warm", "calm"] } }],
        }),
        _ => json!({ "data": [
            { "id": "whisper-1" }, { "id": "gpt-4o-transcribe" }, { "id": "gpt-4o-transcribe-diarize" },
            { "id": "tts-1" }, { "id": "gpt-4o-mini-tts" }, { "id": "gpt-realtime" },
            { "id": "whisper-large-v3" }, { "id": "canopylabs/orpheus-v1-english" },
            { "id": "voxtral-mini-latest" }, { "id": "voxtral-mini-tts-2603" },
        ]}),
    };
    Reply::json(200, &text.to_string())
}

/// Every step of `p`, on `plugin`.
fn transcript(plugin: &mut dyn Plugin, p: Provider, rt: &tokio::runtime::Runtime) -> Vec<Value> {
    let mut s = Session { plugin, next: 0 };
    let mut steps = Vec::new();
    let mut greeting = hello();
    greeting["lang"] = json!("fr");
    let can = s.call("initialize", greeting).answer;
    steps.push(json!({ "at": "initialize", "can": can["result"]["capabilities"]["voice"] }));
    steps.push(json!({ "at": "describe", "answer": s.call(wire::DESCRIBE, json!({})).answer }));
    let listing = json!({ "listed": null, "error": null });
    steps.push(json!({ "at": "catalog", "answer": s.call(wire::CATALOG, listing).answer }));

    // A dictation, as the service answers it, then its refusals.
    // `pcm` for the programs of then, `pcm16` for the modules: the same sound.
    let sound = wire::floats_to_b64(&sound_of(0.25));
    let sound16 = wire::pcm16_to_b64(&sound_of(0.25));
    for (at, status, body, language) in [
        ("dictation", 200, dictated(p), "fr"),
        ("any language", 200, dictated(p), "auto"),
        ("key refused", 401, "{}", "fr"),
        ("key forbidden", 403, "{}", "fr"),
        (
            "quota",
            429,
            r#"{"error":{"message":"trop de requêtes"}}"#,
            "fr",
        ),
        ("failure", 500, r#"{"detail":{"message":"en panne"}}"#, "fr"),
        (
            "the key said",
            400,
            &*format!(r#"{{"message":"mauvaise clé {KEY}"}}"#),
            "fr",
        ),
        ("no json", 200, "<html>", "fr"),
        ("no text", 200, "{}", "fr"),
    ] {
        let reply = Reply::json(status, body);
        let (url, got) = rt.block_on(serve_with(Arc::new(move |_: &Got| reply.clone())));
        let told = s.call(
            wire::TRANSCRIBE,
            json!({ "call": at, "spec": spec_of(p, &url), "language": language, "pcm": sound, "pcm16": sound16 }),
        );
        let seen: Vec<Value> = got.lock().iter().map(asked).collect();
        steps.push(json!({ "at": at, "answer": told.answer, "asked": seen }));
    }
    let off = nowhere();
    let told = s.call(
        wire::TRANSCRIBE,
        json!({ "call": "cut", "spec": spec_of(p, &off), "language": "fr", "pcm": sound, "pcm16": sound16 }),
    );
    steps.push(json!({ "at": "cut", "answer": told.answer }));

    // A sentence read, at each pace, and its refusals.
    let model = escouade_voice::catalog::default_speak_model(p);
    let voice = escouade_voice::catalog::default_voice(p, model, "en").unwrap_or("nobody");
    for (at, reply, lang, rate) in [
        ("read", read_back(p), "fr", 1.0),
        ("read faster", read_back(p), "en", 1.3),
        (
            "voice refused",
            Reply::json(422, r#"{"message":"voix inconnue"}"#),
            "fr",
            1.0,
        ),
        ("key refused", Reply::json(401, "{}"), "fr", 1.0),
        (
            "quota",
            Reply::json(429, r#"{"message":"trop"}"#),
            "fr",
            1.0,
        ),
        ("no sound", read_empty(), "fr", 1.0),
    ] {
        let (url, got) = rt.block_on(serve_with(Arc::new(move |_: &Got| reply.clone())));
        let told = s.call(
            wire::SPEAK,
            json!({ "call": at, "spec": spec_of(p, &url), "model": model, "voice": voice,
                    "lang": lang, "rate": rate, "sentence": "Bonjour, ça va ?" }),
        );
        let seen: Vec<Value> = got.lock().iter().map(asked).collect();
        steps.push(json!({ "at": format!("speak: {at}"), "answer": told.answer,
                           "heard": heard(&told.notes), "asked": seen }));
    }

    // What its API lists, and the catalog made of it.
    let (url, got) = rt.block_on(serve_with(Arc::new(move |g: &Got| {
        let path = g.line.split_whitespace().nth(1).unwrap_or("").to_string();
        listed(p, &path)
    })));
    let told = s.call(wire::LIST, json!({ "spec": spec_of(p, &url) }));
    let seen: Vec<Value> = got.lock().iter().map(asked).collect();
    let answer = told.answer.clone();
    let catalog = s.call(
        wire::CATALOG,
        json!({ "listed": answer["result"], "error": null }),
    );
    steps.push(json!({ "at": "list", "answer": answer, "asked": seen, "catalog": catalog.answer }));

    // A stream, as far as each way it ends.
    if p.offers_live() {
        for (at, mode, wait) in [
            ("speaks", Mode::Speaks, 5000),
            ("refused", Mode::Refuses(401), 5000),
            ("quota", Mode::Refuses(429), 5000),
            ("failure", Mode::Refuses(500), 5000),
            ("fails", Mode::Fails, 5000),
            ("closes", Mode::Closes, 5000),
            ("closes at its end", Mode::ClosesAtEnd, 5000),
            ("mute", Mode::Mute, 300),
        ] {
            let f = fake(p, mode);
            let mut live = Spec::new(p, KEY.into());
            live.base = f.url.clone();
            live.live = Some(p.live_model(p.model()));
            let open = json!({ "stream": at, "spec": wire::spec_json(&live), "language": "fr", "rate": 48000 });
            let opened = s.call(wire::LIVE_OPEN, open);
            for frame in sound_of(0.3).chunks(960) {
                let pcm = wire::floats_to_b64(frame);
                s.tell(wire::LIVE_AUDIO, json!({ "stream": at, "pcm": pcm }));
            }
            let told = s.call(wire::LIVE_FINISH, json!({ "stream": at, "waitMs": wait }));
            let mut notes = opened.notes;
            notes.extend(told.notes);
            // The sound's fate once the service failed is the race of the two sides: only what it got first counts.
            let streams: Vec<Value> = f.conns().iter().map(conn).collect();
            let streams = match mode {
                Mode::Speaks | Mode::Mute => json!(streams),
                _ => json!(f.conns().iter().map(|c| c.line.clone()).collect::<Vec<_>>()),
            };
            steps.push(json!({ "at": format!("live: {at}"), "answer": told.answer,
                               "partials": partials(&notes), "streams": streams }));
        }
    }
    steps
}

/// A reading whose answer holds no sound.
fn read_empty() -> Reply {
    Reply::json(200, "{}")
}

fn recording(p: Provider) -> PathBuf {
    root()
        .join("tests/fixtures/plugins/parity")
        .join(format!("voice-{}.json", p.id()))
}

/// Its entry in the programs of before.
fn entry(p: Provider) -> String {
    match p {
        Provider::Mistral => "mistral".into(),
        _ => format!("voice-{}", p.id()),
    }
}

fn parity_of(p: Provider) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let mut native = Native::start(&entry(p));
        let steps = transcript(&mut native, p, &rt);
        let text = serde_json::to_string_pretty(&steps).unwrap();
        std::fs::write(recording(p), text + "\n").unwrap();
        return;
    }
    let Some(wasm) = module(p.id()) else {
        return;
    };
    let net = Net {
        routes: HashMap::from([("127.0.0.1".to_string(), "http://127.0.0.1".to_string())]),
        ..Net::default()
    };
    let id = format!("dev.escouade.{}", p.id());
    let mut hosted = Hosted::start(wasm, Fetcher::new(&id, &[], &net));
    let steps = transcript(&mut hosted, p, &rt);
    let text = std::fs::read_to_string(recording(p)).expect("a recording of the native");
    let native: Vec<Value> = serde_json::from_str(&text).unwrap();
    for (a, b) in steps.iter().zip(&native) {
        assert_eq!(a, b);
    }
    assert_eq!(steps.len(), native.len());
}

#[test]
fn openai_answers_as_its_native_plugin() {
    parity_of(Provider::OpenAi);
}

#[test]
fn groq_answers_as_its_native_plugin() {
    parity_of(Provider::Groq);
}

#[test]
fn mistral_answers_as_its_native_plugin() {
    parity_of(Provider::Mistral);
}

#[test]
fn elevenlabs_answers_as_its_native_plugin() {
    parity_of(Provider::ElevenLabs);
}

#[test]
fn deepgram_answers_as_its_native_plugin() {
    parity_of(Provider::Deepgram);
}
