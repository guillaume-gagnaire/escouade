//! The plugin's side of a voice service (INT-13, ESC-285): what `escouade-plugins voice-<service>` runs. It answers
//! `voice/*` (see `wire`) with the code of the service as it was in the core: `online::transcribe`, `LiveStream`,
//! `online_speech::speak`, `online_catalog`. Requests run on threads of their own (a reading lasts seconds, the
//! stream's audio and a cancel must get through meanwhile); the answers and the notifications share one output.

use super::live::{LiveStream, FINAL_WAIT};
use super::online::{self, OnlineError, Provider};
use super::online_catalog::{self, Listed};
use super::online_speech::{self, SpeakChoice};
use super::speech::Audio;
use super::wire;
use escouade_plugin_protocol::{answer, code, initialized, Refusal, Request};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Out = Arc<Mutex<Box<dyn Write + Send>>>;

struct State {
    provider: Provider,
    out: Out,
    streams: Mutex<HashMap<String, LiveStream>>,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl State {
    fn send(&self, line: &str) {
        send(&self.out, line);
    }

    fn notify(&self, method: &str, params: Value) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string());
    }

    fn cancel_of(&self, call: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.cancels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(call.to_string(), flag.clone());
        flag
    }

    fn forget(&self, call: &str) {
        self.cancels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(call);
    }
}

fn send(out: &Out, line: &str) {
    let mut o = out.lock().unwrap_or_else(|e| e.into_inner());
    let _ = writeln!(o, "{line}");
    let _ = o.flush();
}

fn refuse(e: OnlineError) -> Refusal {
    let (code, message) = wire::refusal_of(&e);
    Refusal { code, message }
}

fn invalid(why: impl std::fmt::Display) -> Refusal {
    Refusal {
        code: -32602,
        message: why.to_string(),
    }
}

fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

/// The service `id`'s plugin, until its input ends or `shutdown`. Returns the exit code.
pub fn run(id: &str) -> i32 {
    let Some(provider) = Provider::core(id) else {
        eprintln!("escouade-plugins voice-{id}: no such service");
        return 2;
    };
    let out: Out = Arc::new(Mutex::new(Box::new(std::io::stdout())));
    match serve(provider, std::io::stdin().lock(), out) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("escouade-plugins voice-{}: {e}", provider.id());
            1
        }
    }
}

/// The loop, on any input and output (the tests').
pub fn serve<R: BufRead>(provider: Provider, input: R, out: Out) -> std::io::Result<()> {
    let state = Arc::new(State {
        provider,
        out,
        streams: Mutex::new(HashMap::new()),
        cancels: Mutex::new(HashMap::new()),
    });
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Request>(&line) else {
            let refusal = Refusal {
                code: code::PARSE,
                message: "not a JSON-RPC request".into(),
            };
            state.send(&answer(&Value::Null, Err(refusal)));
            continue;
        };
        let Request { id, method, params } = request;
        match method.as_str() {
            "initialize" => {
                if let Some(id) = &id {
                    state.send(&answer(id, Ok(initialized(json!({ "voice": true })))));
                }
            }
            "shutdown" => {
                if let Some(id) = &id {
                    state.send(&answer(id, Ok(Value::Null)));
                }
                return Ok(());
            }
            // In order, on this thread: the sound must reach the stream as it left, the stream must exist
            // before its first piece, and a cancel must not wait behind a reading.
            wire::LIVE_AUDIO => {
                if let Some(samples) = params
                    .get("pcm")
                    .and_then(Value::as_str)
                    .and_then(wire::floats_from_b64)
                {
                    let streams = state.streams.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(s) = streams.get(&text(&params, "stream")) {
                        s.push(&samples);
                    }
                }
            }
            wire::LIVE_CLOSE => {
                state
                    .streams
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&text(&params, "stream"));
            }
            wire::CANCEL => {
                if let Some(flag) = state
                    .cancels
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(&text(&params, "call"))
                {
                    flag.store(true, Ordering::SeqCst);
                }
            }
            wire::LIVE_OPEN => {
                let result = live_open(&state, &params);
                reply(&state, &id, result);
            }
            _ => {
                let state = state.clone();
                std::thread::spawn(move || {
                    let result = handle(&state, &method, params);
                    reply(&state, &id, result);
                });
            }
        }
    }
    Ok(())
}

fn reply(state: &State, id: &Option<Value>, result: Result<Value, Refusal>) {
    // A notification has no id: nothing is answered.
    if let Some(id) = id {
        state.send(&answer(id, result));
    }
}

/// A runtime of its own on this thread.
fn block<T>(f: impl std::future::Future<Output = T>) -> Result<T, Refusal> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map(|rt| rt.block_on(f))
        .map_err(invalid)
}

fn live_open(state: &Arc<State>, p: &Value) -> Result<Value, Refusal> {
    let spec = wire::spec_from(state.provider, &p["spec"]).map_err(invalid)?;
    let stream = text(p, "stream");
    let language = text(p, "language");
    let rate = p.get("rate").and_then(Value::as_u64).unwrap_or(16_000) as u32;
    let notifier = state.clone();
    let id = stream.clone();
    let opened = LiveStream::open(
        &spec,
        &language,
        rate,
        Box::new(move |t| notifier.notify(wire::PARTIAL, json!({ "stream": id, "text": t }))),
    );
    state
        .streams
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(stream, opened);
    Ok(json!({}))
}

fn handle(state: &Arc<State>, method: &str, p: Value) -> Result<Value, Refusal> {
    let provider = state.provider;
    match method {
        wire::DESCRIBE => Ok(json!({
            "id": provider.id(),
            "label": provider.label(),
            "defaultModel": provider.model(),
            "offersLive": provider.offers_live(),
            // A model of its own for the stream, else the one chosen.
            "liveModel": Some(provider.live_model("")).filter(|m| !m.is_empty()),
            "maxChars": online_speech::max_chars(provider),
            "baseUrl": provider.base_url(),
        })),
        wire::TRANSCRIBE => {
            let spec = wire::spec_from(provider, &p["spec"]).map_err(invalid)?;
            let samples = p
                .get("pcm")
                .and_then(Value::as_str)
                .and_then(wire::floats_from_b64)
                .ok_or_else(|| invalid("no sound"))?;
            let call = text(&p, "call");
            let cancel = state.cancel_of(&call);
            let language = text(&p, "language");
            let r = block(online::transcribe(&spec, &samples, &language, &cancel));
            state.forget(&call);
            r?.map(|t| json!({ "text": t })).map_err(refuse)
        }
        wire::LIVE_FINISH => {
            let stream = state
                .streams
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&text(&p, "stream"))
                .ok_or_else(|| invalid("no such stream"))?;
            let wait = p
                .get("waitMs")
                .and_then(Value::as_u64)
                .map(Duration::from_millis)
                .unwrap_or(FINAL_WAIT);
            stream
                .finish(wait)
                .map(|f| json!({ "text": f.text }))
                .map_err(refuse)
        }
        wire::SPEAK => {
            let spec = wire::spec_from(provider, &p["spec"]).map_err(invalid)?;
            let choice = SpeakChoice {
                model: text(&p, "model"),
                voice: text(&p, "voice"),
                lang: text(&p, "lang"),
                rate: p.get("rate").and_then(Value::as_f64).unwrap_or(1.0) as f32,
            };
            let call = text(&p, "call");
            let cancel = state.cancel_of(&call);
            let sentence = text(&p, "sentence");
            let notifier = state.clone();
            let id = call.clone();
            let mut on = move |a: Audio| {
                notifier.notify(
                    wire::AUDIO,
                    json!({ "call": id, "rate": a.rate, "pcm": wire::pcm16_to_b64(&a.samples) }),
                )
            };
            let r = block(online_speech::speak(
                &spec, &choice, &sentence, &cancel, &mut on,
            ));
            state.forget(&call);
            r?.map(|first| json!({ "firstMs": first.map(|d| d.as_secs_f64() * 1000.0) }))
                .map_err(refuse)
        }
        wire::CATALOG => {
            let listed: Option<Listed> = p
                .get("listed")
                .filter(|v| !v.is_null())
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let error = p.get("error").and_then(Value::as_str).map(String::from);
            serde_json::to_value(online_catalog::catalog(provider, listed.as_ref(), error))
                .map_err(invalid)
        }
        wire::LIST => {
            let spec = wire::spec_from(provider, &p["spec"]).map_err(invalid)?;
            let listed = block(online_catalog::fetch(&spec))?.map_err(refuse)?;
            serde_json::to_value(listed).map_err(invalid)
        }
        other => Err(Refusal::not_found(other)),
    }
}
