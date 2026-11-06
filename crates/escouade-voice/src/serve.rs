//! The plugin's side of a voice service (INT-13, ESC-285): it answers `voice/*` (see `wire`) on the events of the host. A
//! dictation, a reading, a list of models and a stream are each a task of the reactor (`host`): a reading lasts seconds,
//! and the sound of a stream and a cancel go through meanwhile, each as a line of a later turn.
//!
//! The answers of a service cross as the app's calls say: `voice/partial` while a stream hears, `voice/audio` while a
//! sentence is read, then the call's answer.

use crate::catalog::{self, Listed};
use crate::live::{self, Cmd, Inbox, FINAL_WAIT};
use crate::online::{self, redact, Call, OnlineError, Provider, Spec};
use crate::speech::{self, Audio, SpeakChoice};
use crate::wire;
use escouade_plugin_kit::host::Host;
use escouade_plugin_kit::protocol::{
    answer, code, greeted, initialized, method as handshake, Refusal, Request,
};
use escouade_plugin_kit::tr;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::poll_fn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

/// What a plugin serves beside its voice, in the same module (Mistral: its harness, INT-10.6 bis): the requests that are
/// not `voice/*`, in their order.
pub type Other = Box<dyn FnMut(&str, Value) -> Result<Value, Refusal>>;

/// A stream under way, as the calls of the core reach it.
struct Live {
    inbox: Arc<Inbox>,
    result: Arc<Mutex<Option<Result<String, OnlineError>>>>,
}

impl Drop for Live {
    fn drop(&mut self) {
        self.inbox.dropped.store(true, Ordering::Release);
    }
}

struct Plugin {
    provider: Provider,
    host: Arc<Host>,
    can: Value,
    streams: Mutex<HashMap<String, Live>>,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

/// The plugin of a service: one turn on the lines that came gives the lines to write. Plain code: `cargo test` plays it
/// without a module.
pub struct Reactor {
    plugin: Arc<Plugin>,
    other: Option<Other>,
}

/// How a request is answered, now or by the task that waits; a notification has nothing to answer.
#[derive(Clone)]
struct Reply {
    id: Option<Value>,
    host: Arc<Host>,
}

impl Reply {
    fn send(&self, result: Result<Value, Refusal>) {
        if let Some(id) = &self.id {
            self.host.write(answer(id, result));
        }
    }
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

impl Reactor {
    /// The service `id`, with what the module says it serves beside its voice: `other`, which `initialize` says by
    /// `capabilities`. None for a service that is not one of Escouade's.
    pub fn new(id: &str, capabilities: Value, other: Option<Other>) -> Option<Self> {
        let provider = Provider::core(id)?;
        let mut can = json!({ "voice": true });
        if let (Some(can), Some(more)) = (can.as_object_mut(), capabilities.as_object()) {
            can.extend(more.clone());
        }
        Some(Self {
            plugin: Arc::new(Plugin {
                provider,
                host: Arc::new(Host::default()),
                can,
                streams: Mutex::default(),
                cancels: Mutex::default(),
            }),
            other,
        })
    }

    /// One turn on what came: the lines to write.
    pub fn turn(&mut self, lines: impl IntoIterator<Item = String>) -> Vec<String> {
        let host = self.plugin.host.clone();
        for line in lines {
            if let Some(msg) = host.arrived(&line) {
                self.called(msg);
            }
        }
        host.run();
        host.lines()
    }

    /// A line that is a call: the handshake is answered here, the rest by the plugin, in the order it came.
    fn called(&mut self, msg: Value) {
        let host = self.plugin.host.clone();
        let Ok(request) = serde_json::from_value::<Request>(msg) else {
            let refusal = Refusal {
                code: code::PARSE,
                message: "not a JSON-RPC request".into(),
            };
            host.write(answer(&Value::Null, Err(refusal)));
            return;
        };
        let reply = Reply {
            id: request.id,
            host,
        };
        match request.method.as_str() {
            handshake::INITIALIZE => {
                greeted(&request.params);
                escouade_plugin_kit::i18n::from_greeting();
                reply.send(Ok(initialized(self.plugin.can.clone())));
            }
            handshake::SHUTDOWN => reply.send(Ok(Value::Null)),
            // Not the voice's: in its order, the frames of an agent before the next ones.
            m if !m.starts_with("voice/") && self.other.is_some() => {
                if let Some(other) = self.other.as_mut() {
                    reply.send(other(m, request.params));
                }
            }
            m => self.plugin.handle(m, request.params, reply),
        }
    }
}

impl Plugin {
    fn cancel_of(&self, call: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.cancels.lock().insert(call.to_string(), flag.clone());
        flag
    }

    fn handle(self: &Arc<Self>, method: &str, p: Value, reply: Reply) {
        let provider = self.provider;
        let answered = match method {
            wire::LIVE_AUDIO => {
                if let Some(samples) = p
                    .get("pcm")
                    .and_then(Value::as_str)
                    .and_then(wire::floats_from_b64)
                {
                    if let Some(s) = self.streams.lock().get(&text(&p, "stream")) {
                        s.inbox.cmds.lock().push_back(Cmd::Audio(samples));
                        self.host.stir();
                    }
                }
                return;
            }
            wire::LIVE_CLOSE => {
                self.streams.lock().remove(&text(&p, "stream"));
                self.host.stir();
                return;
            }
            wire::CANCEL => {
                if let Some(flag) = self.cancels.lock().get(&text(&p, "call")) {
                    flag.store(true, Ordering::Release);
                    self.host.stir();
                }
                return;
            }
            wire::DESCRIBE => Ok(json!({
                "id": provider.id(),
                "label": provider.label(),
                "defaultModel": provider.model(),
                "offersLive": provider.offers_live(),
                // A model of its own for the stream, else the one chosen.
                "liveModel": Some(provider.live_model("")).filter(|m| !m.is_empty()),
                "maxChars": speech::max_chars(provider),
                "baseUrl": provider.base_url(),
            })),
            wire::CATALOG => {
                let listed: Option<Listed> = p
                    .get("listed")
                    .filter(|v| !v.is_null())
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                let error = p.get("error").and_then(Value::as_str).map(String::from);
                serde_json::to_value(catalog::catalog(provider, listed.as_ref(), error))
                    .map_err(invalid)
            }
            wire::LIVE_OPEN => Ok(self.open(&p)),
            wire::LIVE_FINISH => return self.finish(&p, reply),
            wire::TRANSCRIBE => return self.transcribe(p, reply),
            wire::SPEAK => return self.speak(p, reply),
            wire::LIST => {
                let this = self.clone();
                let spec = wire::spec_from(provider, &p["spec"]);
                self.host.spawn(async move {
                    let listed = catalog::fetch(&this.host, &spec).await;
                    reply.send(
                        listed
                            .map_err(refuse)
                            .and_then(|l| serde_json::to_value(l).map_err(invalid)),
                    );
                });
                return;
            }
            other => Err(Refusal::not_found(other)),
        };
        reply.send(answered);
    }

    /// The stream of `p`, started: what is pushed meanwhile waits for it.
    fn open(self: &Arc<Self>, p: &Value) -> Value {
        let spec = wire::spec_from(self.provider, &p["spec"]);
        let stream = text(p, "stream");
        let language = text(p, "language");
        let rate = p.get("rate").and_then(Value::as_u64).unwrap_or(16_000) as u32;
        let inbox = Arc::new(Inbox::default());
        let result = Arc::new(Mutex::new(None));
        self.streams.lock().insert(
            stream.clone(),
            Live {
                inbox: inbox.clone(),
                result: result.clone(),
            },
        );
        let host = self.host.clone();
        self.host.spawn(async move {
            let notifier = host.clone();
            let id = stream.clone();
            let said = move |text: String| {
                notifier.notify(wire::PARTIAL, json!({ "stream": id, "text": text }));
            };
            let ended = live::run(&host, &spec, &language, rate, &inbox, said).await;
            *result.lock() = Some(ended);
            host.stir();
        });
        json!({})
    }

    /// The sentence is over: its final text, or why there is none within `waitMs`.
    fn finish(self: &Arc<Self>, p: &Value, reply: Reply) {
        let Some(stream) = self.streams.lock().remove(&text(p, "stream")) else {
            return reply.send(Err(invalid("no such stream")));
        };
        let wait = p
            .get("waitMs")
            .and_then(Value::as_u64)
            .map_or(FINAL_WAIT, Duration::from_millis);
        stream.inbox.cmds.lock().push_back(Cmd::End);
        self.host.stir();
        let (host, provider) = (self.host.clone(), self.provider);
        self.host.spawn(async move {
            let waited = poll_fn(|_| match stream.result.lock().take() {
                Some(ended) => Poll::Ready(ended),
                None => Poll::Pending,
            });
            let ended = host.timeout(wait, waited).await.unwrap_or_else(|| {
                let late = tr!("pas de texte définitif à temps", "no final text in time");
                Err(OnlineError::Network(provider, late))
            });
            reply.send(ended.map(|text| json!({ "text": text })).map_err(refuse));
            drop(stream);
            host.stir();
        });
    }

    fn transcribe(self: &Arc<Self>, p: Value, reply: Reply) {
        let spec = wire::spec_from(self.provider, &p["spec"]);
        let Some(pcm) = p.get("pcm16").and_then(Value::as_str).and_then(wire::unb64) else {
            return reply.send(Err(invalid("no sound")));
        };
        let call = text(&p, "call");
        let cancel = self.cancel_of(&call);
        let language = text(&p, "language");
        let this = self.clone();
        self.host.spawn(async move {
            let asked = dictation(&this.host, &spec, pcm, &language, &cancel).await;
            this.cancels.lock().remove(&call);
            reply.send(asked.map(|t| json!({ "text": t })).map_err(refuse));
        });
    }

    fn speak(self: &Arc<Self>, p: Value, reply: Reply) {
        let spec = wire::spec_from(self.provider, &p["spec"]);
        let choice = SpeakChoice {
            model: text(&p, "model"),
            voice: text(&p, "voice"),
            lang: text(&p, "lang"),
            rate: p.get("rate").and_then(Value::as_f64).unwrap_or(1.0) as f32,
        };
        let call = text(&p, "call");
        let cancel = self.cancel_of(&call);
        let sentence = text(&p, "sentence");
        let this = self.clone();
        self.host.spawn(async move {
            let notifier = this.host.clone();
            let id = call.clone();
            let mut on = move |a: Audio| {
                notifier.notify(
                    wire::AUDIO,
                    json!({ "call": id, "rate": a.rate, "pcm": wire::pcm16_to_b64(&a.samples) }),
                );
            };
            let read = speech::speak(&this.host, &spec, &choice, &sentence, &cancel, &mut on).await;
            this.cancels.lock().remove(&call);
            let first =
                |d: Option<Duration>| json!({ "firstMs": d.map(|d| d.as_secs_f64() * 1000.0) });
            reply.send(read.map(first).map_err(refuse));
        });
    }
}

/// An HTTP request of a service: its status and its text; a refusal of the host is a network's error, its key taken out.
pub(crate) async fn fetch_text(
    host: &Host,
    call: &Call,
    spec: &Spec,
) -> Result<(u16, String), OnlineError> {
    host.fetch(call.params())
        .await
        .map_err(|e| OnlineError::Network(spec.provider, redact(&e.message, &spec.key)))
}

/// Sends `pcm` to the service of `spec`, gives its text; gives up once `cancel` is set, with no text. Ten minutes are 19 Mo
/// here: each copy is dropped as the next is made, for the memory of a module.
async fn dictation(
    host: &Arc<Host>,
    spec: &Spec,
    pcm: Vec<u8>,
    language: &str,
    cancel: &AtomicBool,
) -> Result<String, OnlineError> {
    let audio = online::wav(&pcm);
    drop(pcm);
    let call = online::request(spec, &audio, language);
    drop(audio);
    let Some(answered) = speech::or_cancelled(cancel, fetch_text(host, &call, spec)).await else {
        return Ok(String::new());
    };
    let (status, body) = answered?;
    online::transcribed(spec, status, &body)
}

/// Serves the service `id` as this module, with what it serves beside its voice: what its first turn runs.
#[cfg(target_family = "wasm")]
pub fn run(id: &str, capabilities: Value, other: Option<Other>) -> i32 {
    use escouade_plugin_kit::protocol::turn;
    use std::cell::RefCell;
    thread_local! {
        static PLUGIN: RefCell<Option<Reactor>> = const { RefCell::new(None) };
    }
    fn one_turn() {
        PLUGIN.with(|p| {
            let mut p = p.borrow_mut();
            let Some(plugin) = p.as_mut() else { return };
            for line in plugin.turn(turn::lines()) {
                turn::write(&line);
            }
        });
        turn::flush();
    }
    let Some(reactor) = Reactor::new(id, capabilities, other) else {
        eprintln!("voice: no service {id}");
        return 2;
    };
    PLUGIN.with(|p| *p.borrow_mut() = Some(reactor));
    turn::set_turn(one_turn);
    0
}

/// A plugin of voice is a module: nothing starts it natively.
#[cfg(not(target_family = "wasm"))]
pub fn run(id: &str, _capabilities: Value, _other: Option<Other>) -> i32 {
    eprintln!("voice: {id} runs as a module");
    2
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    const KEY: &str = "sk-test-0123456789";

    fn reactor(id: &str) -> Reactor {
        Reactor::new(id, json!({}), None).unwrap()
    }

    fn turn(r: &mut Reactor, lines: &[Value]) -> Vec<Value> {
        let lines: Vec<String> = lines.iter().map(Value::to_string).collect();
        r.turn(lines)
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn call(id: u64, method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    }

    fn tell(method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "method": method, "params": params })
    }

    fn answer_of(id: u64, result: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "result": result })
    }

    fn spec() -> Value {
        json!({ "key": KEY, "base": "https://x.dev", "model": "m", "live": "m-live" })
    }

    #[test]
    fn it_says_what_it_serves_and_refuses_what_it_does_not_know() {
        let mut r = Reactor::new("mistral", json!({ "harnesses": ["vibe"] }), None).unwrap();
        let out = turn(&mut r, &[call(1, "initialize", json!({}))]);
        let can = &out[0]["result"]["capabilities"];
        assert_eq!(*can, json!({ "voice": true, "harnesses": ["vibe"] }));
        let out = turn(&mut r, &[call(2, "voice/nothing", json!({}))]);
        assert_eq!(out[0]["error"]["code"], code::METHOD_NOT_FOUND);
        assert!(Reactor::new("nobody", json!({}), None).is_none());
    }

    #[test]
    fn what_is_not_the_voices_goes_beside_it_in_its_order() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = seen.clone();
        let other: Other = Box::new(move |m, _| {
            s.lock().push(m.to_string());
            Ok(json!("beside"))
        });
        let mut r = Reactor::new("mistral", json!({}), Some(other)).unwrap();
        let out = turn(
            &mut r,
            &[
                call(1, "agent/frames", json!({})),
                call(2, "voice/describe", json!({})),
                call(3, "agent/close", json!({})),
            ],
        );
        assert_eq!(*seen.lock(), ["agent/frames", "agent/close"]);
        assert_eq!(out[0]["result"], "beside");
        assert_eq!(out[1]["result"]["id"], "mistral");
        assert_eq!(out[2]["id"], 3);
    }

    #[test]
    fn a_dictation_goes_by_a_fetch_and_nothing_waits_for_it() {
        let mut r = reactor("groq");
        let pcm = wire::pcm16_to_b64(&[0.0; 1600]);
        let params = json!({ "call": "c", "spec": spec(), "language": "fr", "pcm16": pcm });
        let out = turn(&mut r, &[call(1, wire::TRANSCRIBE, params)]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0]["method"], "net/fetch");
        assert_eq!(
            out[0]["params"]["url"],
            "https://x.dev/v1/audio/transcriptions"
        );
        let host_id = out[0]["id"].as_u64().unwrap();
        // Another call goes through meanwhile.
        let out = turn(&mut r, &[call(2, wire::DESCRIBE, json!({}))]);
        assert_eq!(out[0]["id"], 2);
        let body = json!({ "status": 200, "body": r#"{"text":"Bonjour"}"# });
        let out = turn(&mut r, &[answer_of(host_id, body)]);
        assert_eq!(out[0]["id"], 1);
        assert_eq!(out[0]["result"]["text"], "Bonjour");
    }

    #[test]
    fn a_cancelled_dictation_answers_no_text() {
        let mut r = reactor("groq");
        let pcm = wire::pcm16_to_b64(&[0.0; 160]);
        let params = json!({ "call": "c", "spec": spec(), "language": "fr", "pcm16": pcm });
        turn(&mut r, &[call(1, wire::TRANSCRIBE, params)]);
        let out = turn(&mut r, &[tell(wire::CANCEL, json!({ "call": "c" }))]);
        assert_eq!(out[0]["id"], 1);
        assert_eq!(out[0]["result"]["text"], "");
    }

    fn opened(r: &mut Reactor) -> u64 {
        let open = json!({ "stream": "s", "spec": spec(), "language": "fr", "rate": 16000 });
        let out = turn(r, &[call(1, wire::LIVE_OPEN, open)]);
        assert_eq!(out[0]["id"], 1, "answered at once: {out:?}");
        let opening = out
            .iter()
            .find(|l| l["method"] == "ws/open")
            .expect("its socket");
        assert_eq!(
            opening["params"]["headers"]["authorization"],
            format!("Token {KEY}")
        );
        let timer = out.iter().find(|l| l["method"] == "timer/set").unwrap();
        let (a, b) = (
            opening["id"].as_u64().unwrap(),
            timer["id"].as_u64().unwrap(),
        );
        turn(r, &[answer_of(b, json!({ "timer": 1 }))]);
        let out = turn(r, &[answer_of(a, json!({ "socket": 9 }))]);
        assert_eq!(out[0]["method"], "timer/cancel", "{out:?}");
        9
    }

    fn sent_answers(lines: &[Value]) -> Vec<Value> {
        lines
            .iter()
            .filter(|l| l["method"] == "ws/send")
            .map(|l| answer_of(l["id"].as_u64().unwrap(), Value::Null))
            .collect()
    }

    #[test]
    fn a_stream_sends_the_sound_as_it_comes_and_gives_its_text() {
        let mut r = reactor("deepgram");
        let socket = opened(&mut r);
        // 100 ms of sound at 16 kHz in two pieces: one frame, when the second came.
        let piece = wire::floats_to_b64(&[0.25; 800]);
        let audio = tell(wire::LIVE_AUDIO, json!({ "stream": "s", "pcm": piece }));
        assert!(turn(&mut r, std::slice::from_ref(&audio)).is_empty());
        let out = turn(&mut r, std::slice::from_ref(&audio));
        assert_eq!(out[0]["method"], "ws/send");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(out[0]["params"]["dataBase64"].as_str().unwrap())
            .unwrap();
        assert_eq!(bytes.len(), 3200);
        // What it hears is told in the same turn as its line.
        let results = json!({ "socket": socket, "text": r#"{"type":"Results","channel":{"alternatives":[{"transcript":"bonjour"}]}}"# });
        let mut lines = sent_answers(&out);
        lines.push(tell("ws/message", results));
        let out = turn(&mut r, &lines);
        assert_eq!(out[0]["method"], wire::PARTIAL);
        assert_eq!(
            out[0]["params"],
            json!({ "stream": "s", "text": "bonjour" })
        );
        // The end: Finalize and CloseStream, then its close says the text.
        let finish = json!({ "stream": "s", "waitMs": 5000 });
        let out = turn(&mut r, &[call(5, wire::LIVE_FINISH, finish)]);
        let finalize = out.iter().find(|l| l["method"] == "ws/send").unwrap();
        assert_eq!(finalize["params"]["text"], r#"{"type":"Finalize"}"#);
        let out = turn(&mut r, &sent_answers(&out));
        assert_eq!(out[0]["params"]["text"], r#"{"type":"CloseStream"}"#);
        let last = json!({ "socket": socket, "text": r#"{"type":"Results","is_final":true,"from_finalize":true,"channel":{"alternatives":[{"transcript":"bonjour le monde"}]}}"# });
        let mut lines = sent_answers(&out);
        lines.push(tell("ws/message", last));
        lines.push(tell(
            "ws/closed",
            json!({ "socket": socket, "code": 1000, "reason": "" }),
        ));
        let out = turn(&mut r, &lines);
        let done = out.iter().find(|l| l["id"] == 5).expect("its answer");
        assert_eq!(done["result"]["text"], "bonjour le monde");
    }

    #[test]
    fn a_stream_with_no_final_text_in_time_says_so_and_closes() {
        let mut r = reactor("deepgram");
        opened(&mut r);
        let finish = json!({ "stream": "s", "waitMs": 50 });
        let out = turn(&mut r, &[call(5, wire::LIVE_FINISH, finish)]);
        let timer = out.iter().find(|l| l["method"] == "timer/set").unwrap();
        let id = timer["id"].as_u64().unwrap();
        let mut lines = sent_answers(&out);
        lines.push(answer_of(id, json!({ "timer": 2 })));
        let out = turn(&mut r, &lines);
        turn(&mut r, &sent_answers(&out));
        let out = turn(&mut r, &[tell("timer/fired", json!({ "timer": 2 }))]);
        let done = out.iter().find(|l| l["id"] == 5).expect("its answer");
        assert_eq!(done["error"]["code"], -32103);
        assert!(
            out.iter().any(|l| l["method"] == "ws/close"),
            "nobody waits for it: {out:?}"
        );
    }

    #[test]
    fn a_stream_dropped_closes_and_its_failure_is_told_at_its_end() {
        let mut r = reactor("deepgram");
        let socket = opened(&mut r);
        let cut = json!({ "socket": socket, "code": 1011, "reason": "boom" });
        turn(&mut r, &[tell("ws/closed", cut)]);
        let out = turn(
            &mut r,
            &[call(5, wire::LIVE_FINISH, json!({ "stream": "s" }))],
        );
        assert_eq!(out[0]["id"], 5);
        assert_eq!(out[0]["error"]["code"], -32103);
        assert!(out[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("1011"));

        let mut r = reactor("deepgram");
        opened(&mut r);
        let out = turn(&mut r, &[tell(wire::LIVE_CLOSE, json!({ "stream": "s" }))]);
        assert_eq!(out[0]["method"], "ws/close", "{out:?}");
    }

    #[test]
    fn a_stream_that_will_not_open_says_why_at_its_end() {
        let mut r = reactor("deepgram");
        let open = json!({ "stream": "s", "spec": spec(), "language": "fr", "rate": 16000 });
        let out = turn(&mut r, &[call(1, wire::LIVE_OPEN, open)]);
        let opening = out.iter().find(|l| l["method"] == "ws/open").unwrap();
        let refused = json!({ "jsonrpc": "2.0", "id": opening["id"], "error": { "code": -32000, "message": "x.dev: HTTP 401: nope" } });
        turn(&mut r, &[refused]);
        let finish = json!({ "stream": "s" });
        let out = turn(&mut r, &[call(5, wire::LIVE_FINISH, finish)]);
        assert_eq!(out[0]["error"]["code"], -32102, "{out:?}");
    }

    #[test]
    fn a_sentence_is_read_as_it_comes_and_a_barge_in_cancels_it() {
        let mut r = reactor("deepgram");
        let say = json!({ "call": "c", "spec": spec(), "model": "aura-2", "voice": "aura-2-agathe-fr",
                          "lang": "fr", "rate": 1.0, "sentence": "Bonjour." });
        let out = turn(&mut r, &[call(1, wire::SPEAK, say)]);
        assert_eq!(out[0]["method"], "net/stream");
        let asked = out[0]["id"].as_u64().unwrap();
        let chunk = tell(
            "net/chunk",
            json!({ "stream": 3, "dataBase64": base64::engine::general_purpose::STANDARD.encode([0u8; 480]) }),
        );
        let out = turn(
            &mut r,
            &[
                answer_of(asked, json!({ "stream": 3, "status": 200 })),
                chunk,
            ],
        );
        assert_eq!(out[0]["method"], wire::AUDIO);
        assert_eq!(out[0]["params"]["rate"], 24_000);
        let out = turn(&mut r, &[tell(wire::CANCEL, json!({ "call": "c" }))]);
        let cancel = out.iter().find(|l| l["method"] == "net/cancel");
        assert!(cancel.is_some(), "{out:?}");
        let done = out.iter().find(|l| l["id"] == 1).expect("its answer");
        assert!(done["result"]["firstMs"].is_number(), "{done}");
    }

    #[test]
    fn the_refusal_of_a_reading_gives_what_the_service_said_without_the_key() {
        let mut r = reactor("openai");
        let say = json!({ "call": "c", "spec": spec(), "model": "tts-1", "voice": "nobody",
                          "lang": "en", "rate": 1.0, "sentence": "Hi." });
        let out = turn(&mut r, &[call(1, wire::SPEAK, say)]);
        let asked = out[0]["id"].as_u64().unwrap();
        let body = format!(r#"{{"error":{{"message":"no voice for {KEY}"}}}}"#);
        let out = turn(
            &mut r,
            &[
                answer_of(asked, json!({ "stream": 3, "status": 422 })),
                tell("net/chunk", json!({ "stream": 3, "data": body })),
                tell("net/end", json!({ "stream": 3 })),
            ],
        );
        let done = out.iter().find(|l| l["id"] == 1).expect("its answer");
        assert_eq!(done["error"]["code"], -32106);
        assert_eq!(done["error"]["message"], "422|no voice for ***");
    }

    #[test]
    fn the_catalog_of_a_service_lists_through_the_host() {
        let mut r = reactor("groq");
        let out = turn(&mut r, &[call(1, wire::LIST, json!({ "spec": spec() }))]);
        assert_eq!(out[0]["method"], "net/fetch");
        assert_eq!(out[0]["params"]["url"], "https://x.dev/v1/models");
        let asked = out[0]["id"].as_u64().unwrap();
        let body = json!({ "status": 200, "body": r#"{"data":[{"id":"whisper-large-v3"}]}"# });
        let out = turn(&mut r, &[answer_of(asked, body)]);
        let done = out.iter().find(|l| l["id"] == 1).expect("its answer");
        assert_eq!(done["result"]["transcribe"][0]["id"], "whisper-large-v3");
    }
}
