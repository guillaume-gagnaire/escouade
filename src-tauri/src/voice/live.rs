//! The transcription as one speaks (ESC-136): « Transcription en temps réel », for a service that offers it. A stream
//! (a WebSocket, with the user's key) is opened as a sentence starts, its sound goes to the service while it is said,
//! the text it has understood so far comes back as it goes (`Partial`), and the sentence's end asks for its final text,
//! there a few hundred milliseconds after the last word.
//!
//! One stream by sentence (by dictation): nothing goes between two, neither the silence nor the agent's voice, and a
//! service that bills by the time the stream is open bills only the words. Opening it takes the time of the first
//! syllables, while what was heard waits in the stream's queue: the sentence does not wait for it.
//!
//! The settings' network is followed: the proxy (an HTTP CONNECT tunnel, opened by hand, as reqwest opens one; who goes
//! around it, `no_proxy`), and their « certificats non vérifiés ». A stream that cannot open, or breaks, says why
//! (`OnlineError`): the caller then sends the sentence whole, by a request, as before.
//!
//! - ElevenLabs, Scribe v2 Realtime: `wss://…/v1/speech-to-text/realtime`, `input_audio_chunk` (16 kHz PCM in base64),
//!   the last one with `commit`; `partial_transcript`, then `committed_transcript`.
//! - Mistral, Voxtral Realtime: `wss://…/v1/audio/transcriptions/realtime`, `session.update` (its format),
//!   `input_audio.append`, then `input_audio.flush` and `input_audio.end`; `transcription.text.delta`, then
//!   `transcription.done`.
//! - Deepgram: `wss://…/v1/listen` (the raw sound in binary frames, `interim_results`), then `Finalize` and
//!   `CloseStream`; `Results` (`is_final`, `from_finalize`).
//! - OpenAI: `wss://…/v1/realtime?intent=transcription`, `session.update` (a transcription session, 24 kHz PCM, no turn
//!   detection), `input_audio_buffer.append`, then `input_audio_buffer.commit`;
//!   `conversation.item.input_audio_transcription.delta`, then `.completed`.

use super::online::{redact, OnlineError, Provider, Spec};
use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest, Message};

/// The stream opens within this (the proxy, TLS and the WebSocket's handshake), or the sentence goes by request.
pub const CONNECT_WAIT: Duration = Duration::from_secs(5);
/// Once the sentence ended, its final text comes within this, or it goes by request.
pub const FINAL_WAIT: Duration = Duration::from_secs(5);
/// What is sent at a time: 100 ms of sound.
const CHUNK_MS: usize = 100;

/// Told the text understood so far, the whole of it, as it changes.
pub type Partial = Box<dyn Fn(String) + Send + 'static>;

enum Cmd {
    Audio(Vec<f32>),
    /// The sentence is over: all that was sent is committed, its text asked for.
    End,
}

/// The final text of a sentence, and when it was there.
#[derive(Debug, Clone, PartialEq)]
pub struct Final {
    pub text: String,
    pub ready: std::time::Instant,
}

/// A stream under way. Dropped before `finish`, it closes, and nothing more goes.
pub struct LiveStream {
    provider: Provider,
    tx: tokio::sync::mpsc::UnboundedSender<Cmd>,
    result: std::sync::mpsc::Receiver<Result<Final, OnlineError>>,
}

impl LiveStream {
    /// Opens the stream of `spec` (its `live` model) for sound at `rate`, in `language` (Whisper's code, or "auto"),
    /// on a thread of its own; what is pushed meanwhile waits for it.
    pub fn open(spec: &Spec, language: &str, rate: u32, partial: Partial) -> LiveStream {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (done, result) = std::sync::mpsc::channel();
        let spec = spec.clone();
        let language = language.to_string();
        let provider = spec.provider;
        let spawned = std::thread::Builder::new()
            .name("voice-live".into())
            .spawn(move || {
                let r = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| OnlineError::Network(spec.provider, e.to_string()))
                    .and_then(|rt| rt.block_on(run(&spec, &language, rate, rx, partial)));
                let _ = done.send(r);
            });
        if let Err(e) = spawned {
            log::warn!("voice: the live stream's thread did not start: {e}");
        }
        LiveStream {
            provider,
            tx,
            result,
        }
    }

    /// Sends `samples` (mono, at the rate it was opened with).
    pub fn push(&self, samples: &[f32]) {
        let _ = self.tx.send(Cmd::Audio(samples.to_vec()));
    }

    /// The sentence is over: its final text, or why there is none within `wait` (then it goes by request).
    pub fn finish(self, wait: Duration) -> Result<Final, OnlineError> {
        let _ = self.tx.send(Cmd::End);
        match self.result.recv_timeout(wait) {
            Ok(r) => r,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(OnlineError::Network(
                self.provider,
                tr!("pas de texte définitif à temps", "no final text in time"),
            )),
            Err(_) => Err(OnlineError::Network(
                self.provider,
                "the stream's thread stopped".into(),
            )),
        }
    }
}

/// The rate it is sent at: OpenAI takes 24 kHz only.
fn target_rate(p: Provider) -> u32 {
    match p {
        Provider::OpenAi => 24_000,
        _ => 16_000,
    }
}

/// The stream's address, for `language` and the model of `spec`.
pub(crate) fn url_of(spec: &Spec, language: &str) -> String {
    let base = spec.base.trim_end_matches('/');
    let base = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        base.to_string()
    };
    let lang = (language != "auto" && !language.is_empty()).then_some(language);
    let model = spec.live.clone().unwrap_or_else(|| spec.model.clone());
    let mut q: Vec<(&str, String)> = Vec::new();
    let path = match spec.provider {
        Provider::ElevenLabs => {
            q.push(("model_id", model));
            q.push(("audio_format", "pcm_16000".into()));
            q.push(("commit_strategy", "manual".into()));
            if let Some(l) = lang {
                q.push(("language_code", l.into()));
            }
            "/v1/speech-to-text/realtime"
        }
        Provider::Mistral => {
            q.push(("model", model));
            "/v1/audio/transcriptions/realtime"
        }
        Provider::Deepgram => {
            let multi = model.starts_with("nova-3");
            q.push(("model", model));
            q.push(("encoding", "linear16".into()));
            q.push(("sample_rate", "16000".into()));
            q.push(("channels", "1".into()));
            q.push(("interim_results", "true".into()));
            q.push(("smart_format", "true".into()));
            match lang {
                Some(l) => q.push(("language", l.into())),
                // Found as it goes: Nova-3's « multi ».
                None if multi => q.push(("language", "multi".into())),
                None => {}
            }
            "/v1/listen"
        }
        _ => {
            q.push(("intent", "transcription".into()));
            "/v1/realtime"
        }
    };
    let query: Vec<String> = q
        .iter()
        .map(|(k, v)| format!("{k}={}", encode(v)))
        .collect();
    format!("{base}{path}?{}", query.join("&"))
}

fn encode(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// What is said as the stream opens.
fn hello(spec: &Spec, language: &str) -> Vec<String> {
    let lang = (language != "auto" && !language.is_empty()).then_some(language);
    match spec.provider {
        Provider::Mistral => vec![serde_json::json!({
            "type": "session.update",
            "session": {"audio_format": {"encoding": "pcm_s16le", "sample_rate": 16000}}
        })
        .to_string()],
        Provider::OpenAi => {
            let mut transcription = serde_json::json!({
                "model": spec.live.clone().unwrap_or_else(|| spec.model.clone())
            });
            if let Some(l) = lang {
                transcription["language"] = l.into();
            }
            vec![serde_json::json!({
                "type": "session.update",
                "session": {
                    "type": "transcription",
                    "audio": {"input": {
                        "format": {"type": "audio/pcm", "rate": 24000},
                        "transcription": transcription,
                        "turn_detection": null
                    }}
                }
            })
            .to_string()]
        }
        _ => Vec::new(),
    }
}

fn pcm16(samples: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// What carries `samples` (at the target rate), then, `commit`, what asks for the text of what was sent since the last
/// one (the user went quiet), and, `end`, what closes the sentence.
fn audio_messages(p: Provider, samples: &[f32], commit: bool, end: bool) -> Vec<Message> {
    let mut out = Vec::new();
    let pcm = |s: &[f32]| b64(&pcm16(s));
    match p {
        Provider::ElevenLabs => {
            // Its commit rides on a chunk of sound: a hundredth of a second of zeros when none is left.
            let zeros;
            let samples = if samples.is_empty() && commit {
                zeros = vec![0f32; 160];
                &zeros[..]
            } else {
                samples
            };
            if !samples.is_empty() {
                out.push(Message::text(
                    serde_json::json!({
                        "message_type": "input_audio_chunk",
                        "audio_base_64": pcm(samples),
                        "commit": commit,
                        "sample_rate": 16000
                    })
                    .to_string(),
                ));
            }
        }
        Provider::Mistral => {
            if !samples.is_empty() {
                out.push(Message::text(
                    serde_json::json!({"type": "input_audio.append", "audio": pcm(samples)})
                        .to_string(),
                ));
            }
            if commit || end {
                out.push(Message::text(r#"{"type":"input_audio.flush"}"#));
            }
            if end {
                out.push(Message::text(r#"{"type":"input_audio.end"}"#));
            }
        }
        Provider::Deepgram => {
            if !samples.is_empty() {
                out.push(Message::binary(pcm16(samples)));
            }
            if commit || end {
                out.push(Message::text(r#"{"type":"Finalize"}"#));
            }
            if end {
                out.push(Message::text(r#"{"type":"CloseStream"}"#));
            }
        }
        _ => {
            if !samples.is_empty() {
                out.push(Message::text(
                    serde_json::json!({"type": "input_audio_buffer.append", "audio": pcm(samples)})
                        .to_string(),
                ));
            }
            if commit {
                out.push(Message::text(r#"{"type":"input_audio_buffer.commit"}"#));
            }
        }
    }
    out
}

/// What a message of the service says.
#[derive(Debug, PartialEq)]
enum Said {
    Partial(String),
    Final(String),
    Nothing,
}

/// What the service has said so far, and how to read what it says next.
#[derive(Default)]
struct Heard {
    /// The final pieces (ElevenLabs' commits, Deepgram's finals, OpenAI's items in their order).
    finals: Vec<String>,
    /// What the deltas wrote (Mistral; OpenAI's item under way).
    deltas: String,
    /// The commits sent, and those answered (ElevenLabs, OpenAI): all answered once ended, the text is final.
    commits: usize,
    answered: usize,
    /// OpenAI: its items in the order they were committed, their text when completed.
    items: Vec<(String, Option<String>)>,
    /// Deepgram answered its `Finalize` (a result `from_finalize`): its close then ends the sentence; before, it is a cut.
    finalized: bool,
}

fn join(parts: &[String], last: &str) -> String {
    parts
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(last))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

fn str_at<'a>(v: &'a serde_json::Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(|x| x.as_str()).unwrap_or("")
}

impl Heard {
    /// The text once ended, when it is all there: every commit answered (ElevenLabs, OpenAI).
    fn complete(&self, p: Provider, ended: bool) -> Option<String> {
        let counted = matches!(p, Provider::ElevenLabs | Provider::OpenAi | Provider::Groq);
        (ended && counted && self.answered >= self.commits).then(|| self.text(p, ""))
    }

    fn text(&self, p: Provider, last: &str) -> String {
        match p {
            Provider::OpenAi | Provider::Groq => {
                let done: Vec<String> = self.items.iter().map_while(|(_, t)| t.clone()).collect();
                let rest = self
                    .items
                    .iter()
                    .skip(done.len())
                    .filter_map(|(_, t)| t.clone());
                let mut all = done;
                all.extend(rest);
                join(&all, &format!("{} {last}", self.deltas))
            }
            Provider::Mistral => join(&[], &self.deltas),
            _ => join(&self.finals, last),
        }
    }

    /// Reads a text message of `p`; `ended`: the sentence's end was sent.
    fn read(
        &mut self,
        p: Provider,
        text: &str,
        ended: bool,
        key: &str,
    ) -> Result<Said, OnlineError> {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
            return Ok(Said::Nothing);
        };
        let service = |m: &str| OnlineError::Service(p, 0, redact(m, key));
        let after = |h: &Heard| match h.complete(p, ended) {
            Some(t) => Said::Final(t),
            None => Said::Partial(h.text(p, "")),
        };
        match p {
            Provider::ElevenLabs => {
                let kind = str_at(&v, "/message_type");
                match kind {
                    "partial_transcript" => Ok(Said::Partial(self.text(p, str_at(&v, "/text")))),
                    "committed_transcript" => {
                        let t = str_at(&v, "/text").to_string();
                        if !t.trim().is_empty() {
                            self.finals.push(t);
                        }
                        self.answered += 1;
                        Ok(after(self))
                    }
                    "session_started" | "committed_transcript_with_timestamps" | "warning" | "" => {
                        Ok(Said::Nothing)
                    }
                    _ => {
                        let msg = format!("{kind}: {}", str_at(&v, "/error"));
                        Err(match kind {
                            "auth_error" => OnlineError::KeyRefused(p),
                            "quota_exceeded" | "rate_limited" | "commit_throttled" => {
                                OnlineError::Quota(p, redact(&msg, key))
                            }
                            _ => service(&msg),
                        })
                    }
                }
            }
            Provider::Mistral => match str_at(&v, "/type") {
                "transcription.text.delta" => {
                    self.deltas.push_str(str_at(&v, "/text"));
                    Ok(Said::Partial(self.text(p, "")))
                }
                "transcription.done" => {
                    let t = str_at(&v, "/text");
                    Ok(Said::Final(join(
                        &[],
                        if t.trim().is_empty() { &self.deltas } else { t },
                    )))
                }
                "error" => {
                    let m = v
                        .pointer("/error/message")
                        .map(|m| match m.as_str() {
                            Some(s) => s.to_string(),
                            None => m.to_string(),
                        })
                        .unwrap_or_else(|| text.chars().take(200).collect());
                    let code = v
                        .pointer("/error/code")
                        .and_then(|c| c.as_u64())
                        .unwrap_or(0);
                    Err(match code {
                        401 | 403 => OnlineError::KeyRefused(p),
                        429 => OnlineError::Quota(p, redact(&m, key)),
                        c => OnlineError::Service(p, c as u16, redact(&m, key)),
                    })
                }
                _ => Ok(Said::Nothing),
            },
            Provider::Deepgram => match str_at(&v, "/type") {
                "Results" => {
                    if v.get("from_finalize").and_then(|x| x.as_bool()) == Some(true) {
                        self.finalized = true;
                    }
                    let t = str_at(&v, "/channel/alternatives/0/transcript").to_string();
                    if v.get("is_final").and_then(|x| x.as_bool()) == Some(true) {
                        if !t.trim().is_empty() {
                            self.finals.push(t);
                        }
                        return Ok(Said::Partial(self.text(p, "")));
                    }
                    Ok(Said::Partial(self.text(p, &t)))
                }
                "Error" => Err(service(&format!(
                    "{} {}",
                    str_at(&v, "/description"),
                    str_at(&v, "/message")
                ))),
                _ => Ok(Said::Nothing),
            },
            _ => {
                let item = str_at(&v, "/item_id").to_string();
                let slot = |h: &mut Heard| -> usize {
                    match h.items.iter().position(|(i, _)| *i == item) {
                        Some(i) => i,
                        None => {
                            h.items.push((item.clone(), None));
                            h.items.len() - 1
                        }
                    }
                };
                match str_at(&v, "/type") {
                    "input_audio_buffer.committed" => {
                        slot(self);
                        Ok(Said::Nothing)
                    }
                    "conversation.item.input_audio_transcription.delta" => {
                        self.deltas.push_str(str_at(&v, "/delta"));
                        Ok(Said::Partial(self.text(p, "")))
                    }
                    "conversation.item.input_audio_transcription.completed" => {
                        let i = slot(self);
                        self.items[i].1 = Some(str_at(&v, "/transcript").to_string());
                        self.deltas.clear();
                        self.answered += 1;
                        Ok(after(self))
                    }
                    "conversation.item.input_audio_transcription.failed" | "error" => {
                        let m = [str_at(&v, "/error/message"), str_at(&v, "/error/code")]
                            .into_iter()
                            .find(|s| !s.is_empty())
                            .unwrap_or("error")
                            .to_string();
                        Err(if str_at(&v, "/error/code") == "invalid_api_key" {
                            OnlineError::KeyRefused(p)
                        } else {
                            service(&m)
                        })
                    }
                    _ => Ok(Said::Nothing),
                }
            }
        }
    }

    /// The stream closed after the sentence's end: what it said, for a service that says its last words by closing
    /// (Deepgram).
    fn closed(&self, p: Provider) -> Option<String> {
        (p == Provider::Deepgram && self.finalized).then(|| join(&self.finals, ""))
    }
}

/// OpenAI refuses to commit less than 100 ms of sound.
fn min_commit(p: Provider, rate: u32) -> usize {
    match p {
        Provider::OpenAi | Provider::Groq => rate as usize / 10,
        _ => 1,
    }
}

/// The stream, from its opening to the sentence's final text.
async fn run(
    spec: &Spec,
    language: &str,
    rate: u32,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Cmd>,
    partial: Partial,
) -> Result<Final, OnlineError> {
    let p = spec.provider;
    let target = target_rate(p);
    let now = std::time::Instant::now;
    let url = url_of(spec, language);
    let ws = tokio::time::timeout(CONNECT_WAIT, connect(spec, &url))
        .await
        .map_err(|_| OnlineError::Network(p, tr!("délai dépassé", "timed out")))??;
    let (mut sink, mut stream) = ws.split();
    let net = |e: tungstenite::Error| OnlineError::Network(p, redact(&e.to_string(), &spec.key));
    for m in hello(spec, language) {
        sink.send(Message::text(m)).await.map_err(net)?;
    }
    let chunk = target as usize * CHUNK_MS / 1000;
    let mut queued: Vec<f32> = Vec::new();
    // The sound sent since the last commit.
    let mut uncommitted = 0usize;
    let mut heard = Heard::default();
    let mut ended = false;
    let mut last_partial = String::new();
    loop {
        tokio::select! {
            cmd = rx.recv(), if !ended => {
                let (commit, end) = match cmd {
                    Some(Cmd::Audio(s)) => {
                        queued.extend(super::audio::resample(&s, rate, target));
                        if queued.len() < chunk {
                            continue;
                        }
                        (false, false)
                    }
                    Some(Cmd::End) => (true, true),
                    // Nobody waits for it any more: it closes.
                    None => {
                        let _ = sink.send(Message::Close(None)).await;
                        return Err(OnlineError::Network(p, "called off".into()));
                    }
                };
                let out = std::mem::take(&mut queued);
                uncommitted += out.len();
                // Too little to commit (OpenAI wants 100 ms): no commit, its answer would not come.
                let commit = commit && uncommitted >= min_commit(p, target);
                for m in audio_messages(p, &out, commit, end) {
                    sink.send(m).await.map_err(net)?;
                }
                if commit {
                    uncommitted = 0;
                    heard.commits += 1;
                }
                if end {
                    ended = true;
                    // Every commit already answered: the text is there.
                    if let Some(text) = heard.complete(p, true) {
                        let _ = sink.send(Message::Close(None)).await;
                        return Ok(Final { text, ready: now() });
                    }
                }
            },
            msg = stream.next() => {
                // Closed once the end was sent: Deepgram's way of saying its last words.
                if ended && matches!(msg, None | Some(Ok(Message::Close(_)))) {
                    if let Some(text) = heard.closed(p) {
                        return Ok(Final { text, ready: now() });
                    }
                }
                match msg {
                    Some(Ok(Message::Text(t))) => match heard.read(p, t.as_str(), ended, &spec.key)? {
                        Said::Partial(text) => {
                            if text != last_partial {
                                last_partial = text.clone();
                                partial(text);
                            }
                        }
                        Said::Final(text) => {
                            let _ = sink.send(Message::Close(None)).await;
                            return Ok(Final { text, ready: now() });
                        }
                        Said::Nothing => {}
                    },
                    Some(Ok(Message::Close(frame))) => {
                        let why = frame
                            .map(|f| format!("{} {}", u16::from(f.code), f.reason))
                            .unwrap_or_default();
                        return Err(OnlineError::Network(
                            p,
                            redact(&tr!("flux fermé ({why})", "stream closed ({why})"), &spec.key),
                        ));
                    }
                    None => {
                        return Err(OnlineError::Network(p, tr!("flux fermé", "stream closed")));
                    }
                    Some(Err(e)) => return Err(net(e)),
                    Some(Ok(_)) => {}
                }
            }
        }
    }
}

/// What a WebSocket runs over: a TCP connection, TLS or not, through the proxy or not.
trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

type Ws = tokio_tungstenite::WebSocketStream<Box<dyn Io>>;

/// Opens `url`, with the key of `spec`, through the settings' proxy and TLS.
async fn connect(spec: &Spec, url: &str) -> Result<Ws, OnlineError> {
    let p = spec.provider;
    let fail = |e: String| OnlineError::Network(p, redact(&e, &spec.key));
    let parsed = reqwest::Url::parse(url).map_err(|e| fail(e.to_string()))?;
    let tls = parsed.scheme() == "wss";
    let host = parsed.host_str().unwrap_or_default().to_string();
    let port = parsed
        .port_or_known_default()
        .unwrap_or(if tls { 443 } else { 80 });
    let tcp = match proxy_for(&spec.net, &host) {
        Some(proxy) => tunnel(&proxy, &host, port).await.map_err(fail)?,
        None => tokio::net::TcpStream::connect((host.as_str(), port))
            .await
            .map_err(|e| fail(e.to_string()))?,
    };
    let _ = tcp.set_nodelay(true);
    let io: Box<dyn Io> = if tls {
        let name = rustls::pki_types::ServerName::try_from(host.clone())
            .map_err(|e| fail(e.to_string()))?;
        let connector = tokio_rustls::TlsConnector::from(tls_config(spec.net.insecure));
        Box::new(
            connector
                .connect(name, tcp)
                .await
                .map_err(|e| fail(format!("TLS: {e}")))?,
        )
    } else {
        Box::new(tcp)
    };
    let mut req = url.into_client_request().map_err(|e| fail(e.to_string()))?;
    let h = req.headers_mut();
    let value = |v: String| {
        tungstenite::http::HeaderValue::from_str(&v).map_err(|_| OnlineError::KeyRefused(p))
    };
    match p {
        Provider::ElevenLabs => {
            h.insert("xi-api-key", value(spec.key.clone())?);
        }
        Provider::Deepgram => {
            h.insert("authorization", value(format!("Token {}", spec.key))?);
        }
        _ => {
            h.insert("authorization", value(format!("Bearer {}", spec.key))?);
        }
    }
    match tokio_tungstenite::client_async(req, io).await {
        Ok((ws, _)) => Ok(ws),
        Err(tungstenite::Error::Http(r)) => {
            let status = r.status().as_u16();
            let body = r
                .body()
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            let said = super::online::said(&body, &spec.key);
            Err(match status {
                401 | 403 => OnlineError::KeyRefused(p),
                429 => OnlineError::Quota(p, said),
                s => OnlineError::Service(p, s, said),
            })
        }
        Err(e) => Err(fail(e.to_string())),
    }
}

/// The proxy `host` goes through: the settings', unless `no_proxy` names it (a host, a domain and what is under it,
/// `*` for all).
pub(crate) fn proxy_for(net: &super::online::Net, host: &str) -> Option<reqwest::Url> {
    if net.proxy.is_empty() {
        return None;
    }
    let host = host.trim_matches(['[', ']']).to_lowercase();
    let around = net
        .no_proxy
        .split([',', ' ', ';'])
        .map(|e| e.trim().trim_start_matches('.').to_lowercase())
        .filter(|e| !e.is_empty())
        .any(|e| e == "*" || host == e || host.ends_with(&format!(".{e}")));
    if around {
        return None;
    }
    let raw = if net.proxy.contains("://") {
        net.proxy.clone()
    } else {
        format!("http://{}", net.proxy)
    };
    reqwest::Url::parse(&raw).ok()
}

/// A tunnel to `host:port` through the HTTP proxy `proxy` (CONNECT), its user and password sent when it has them.
async fn tunnel(
    proxy: &reqwest::Url,
    host: &str,
    port: u16,
) -> Result<tokio::net::TcpStream, String> {
    let at = (
        proxy.host_str().unwrap_or_default().to_string(),
        proxy.port_or_known_default().unwrap_or(80),
    );
    let mut s = tokio::net::TcpStream::connect((at.0.as_str(), at.1))
        .await
        .map_err(|e| format!("proxy {}:{}: {e}", at.0, at.1))?;
    let mut head = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n");
    if !proxy.username().is_empty() {
        let user = percent_decode(proxy.username());
        let pass = percent_decode(proxy.password().unwrap_or(""));
        head.push_str(&format!(
            "Proxy-Authorization: Basic {}\r\n",
            b64(format!("{user}:{pass}").as_bytes())
        ));
    }
    head.push_str("\r\n");
    s.write_all(head.as_bytes())
        .await
        .map_err(|e| format!("proxy: {e}"))?;
    let mut answer = Vec::new();
    let mut byte = [0u8; 1];
    while !answer.ends_with(b"\r\n\r\n") {
        let n = s.read(&mut byte).await.map_err(|e| format!("proxy: {e}"))?;
        if n == 0 || answer.len() > 16 * 1024 {
            return Err(tr!(
                "le proxy a fermé la connexion",
                "the proxy closed the connection"
            ));
        }
        answer.push(byte[0]);
    }
    let line = String::from_utf8_lossy(&answer);
    let line = line.lines().next().unwrap_or_default().to_string();
    let ok = line
        .split_whitespace()
        .nth(1)
        .is_some_and(|c| c.starts_with('2'));
    if !ok {
        return Err(tr!(
            "le proxy refuse le flux ({line})",
            "the proxy refuses the stream ({line})"
        ));
    }
    Ok(s)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// rustls with ring (as reqwest), the system's roots and Mozilla's, HTTP/1.1 only (a WebSocket's upgrade);
/// `insecure`: no certificate checked, as the settings may ask (a proxy that inspects TLS without its root installed).
fn tls_config(insecure: bool) -> Arc<rustls::ClientConfig> {
    static CHECKED: std::sync::OnceLock<Arc<rustls::ClientConfig>> = std::sync::OnceLock::new();
    static UNCHECKED: std::sync::OnceLock<Arc<rustls::ClientConfig>> = std::sync::OnceLock::new();
    let provider = || Arc::new(rustls::crypto::ring::default_provider());
    if insecure {
        return UNCHECKED
            .get_or_init(|| {
                let p = provider();
                let mut c = rustls::ClientConfig::builder_with_provider(p.clone())
                    .with_safe_default_protocol_versions()
                    .expect("ring's protocol versions")
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoCheck(p)))
                    .with_no_client_auth();
                c.alpn_protocols = vec![b"http/1.1".to_vec()];
                Arc::new(c)
            })
            .clone();
    }
    CHECKED
        .get_or_init(|| {
            let mut roots = rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let native = rustls_native_certs::load_native_certs();
            for e in &native.errors {
                log::warn!("voice: a root certificate of the system not read: {e}");
            }
            let (added, _) = roots.add_parsable_certificates(native.certs);
            log::info!("voice: {added} root certificates of the system for the live streams");
            let mut c = rustls::ClientConfig::builder_with_provider(provider())
                .with_safe_default_protocol_versions()
                .expect("ring's protocol versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
            c.alpn_protocols = vec![b"http/1.1".to_vec()];
            Arc::new(c)
        })
        .clone()
}

/// No certificate checked (« certificats non vérifiés » of the settings); the signatures still are.
#[derive(Debug)]
struct NoCheck(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for NoCheck {
    fn verify_server_cert(
        &self,
        _: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
#[path = "live_tests.rs"]
pub(crate) mod tests;
