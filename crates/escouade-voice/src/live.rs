//! The transcription as one speaks (ESC-136): « Transcription en temps réel », for a service that offers it. A stream
//! (a WebSocket, with the user's key) is opened as a sentence starts, its sound goes to the service while it is said,
//! the text it has understood so far comes back as it goes (`Partial`), and the sentence's end asks for its final text,
//! there a few hundred milliseconds after the last word.
//!
//! One stream by sentence (by dictation): nothing goes between two, neither the silence nor the agent's voice, and a
//! service that bills by the time the stream is open bills only the words. Opening it takes the time of the first
//! syllables, while what was heard waits in the stream's queue: the sentence does not wait for it.
//!
//! The settings' network is followed by the host of the plugin (`ws/open`, docs/PLUGINS.md §8.4.4). A stream that cannot
//! open, or breaks, says why (`OnlineError`): the caller then sends the sentence whole, by a request, as before.
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

use crate::online::{key_header, redact, said, OnlineError, Provider, Spec};
use base64::Engine as _;
use escouade_plugin_kit::host::{Host, SocketEvent};
use escouade_plugin_kit::tr;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::future::poll_fn;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

pub use escouade_plugin_kit::host::Frame;

/// The stream opens within this (the proxy, TLS and the WebSocket's handshake), or the sentence goes by request.
pub const CONNECT_WAIT: Duration = Duration::from_secs(5);
/// Once the sentence ended, its final text comes within this, or it goes by request.
pub const FINAL_WAIT: Duration = Duration::from_secs(5);
/// What is sent at a time: 100 ms of sound.
const CHUNK_MS: usize = 100;

/// What the sentence's side tells its stream.
pub(crate) enum Cmd {
    Audio(Vec<f32>),
    /// The sentence is over: all that was sent is committed, its text asked for.
    End,
}

/// The way between the calls of the core and the task of a stream.
#[derive(Default)]
pub(crate) struct Inbox {
    pub(crate) cmds: Mutex<VecDeque<Cmd>>,
    /// Nobody waits for it any more: it closes.
    pub(crate) dropped: std::sync::atomic::AtomicBool,
}

/// The rate it is sent at: OpenAI takes 24 kHz only.
pub fn target_rate(p: Provider) -> u32 {
    match p {
        Provider::OpenAi => 24_000,
        _ => 16_000,
    }
}

/// The stream's address, for `language` and the model of `spec`.
pub fn url_of(spec: &Spec, language: &str) -> String {
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
pub fn hello(spec: &Spec, language: &str) -> Vec<String> {
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
pub fn audio_messages(p: Provider, samples: &[f32], commit: bool, end: bool) -> Vec<Frame> {
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
                out.push(Frame::text(
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
                out.push(Frame::text(
                    serde_json::json!({"type": "input_audio.append", "audio": pcm(samples)})
                        .to_string(),
                ));
            }
            if commit || end {
                out.push(Frame::text(r#"{"type":"input_audio.flush"}"#));
            }
            if end {
                out.push(Frame::text(r#"{"type":"input_audio.end"}"#));
            }
        }
        Provider::Deepgram => {
            if !samples.is_empty() {
                out.push(Frame::Binary(pcm16(samples)));
            }
            if commit || end {
                out.push(Frame::text(r#"{"type":"Finalize"}"#));
            }
            if end {
                out.push(Frame::text(r#"{"type":"CloseStream"}"#));
            }
        }
        _ => {
            if !samples.is_empty() {
                out.push(Frame::text(
                    serde_json::json!({"type": "input_audio_buffer.append", "audio": pcm(samples)})
                        .to_string(),
                ));
            }
            if commit {
                out.push(Frame::text(r#"{"type":"input_audio_buffer.commit"}"#));
            }
        }
    }
    out
}

/// What a message of the service says.
#[derive(Debug, PartialEq)]
pub enum Said {
    Partial(String),
    Final(String),
    Nothing,
}

/// What the service has said so far, and how to read what it says next.
#[derive(Default)]
pub struct Heard {
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
    pub fn complete(&self, p: Provider, ended: bool) -> Option<String> {
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
    pub fn read(
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
    pub fn closed(&self, p: Provider) -> Option<String> {
        (p == Provider::Deepgram && self.finalized).then(|| join(&self.finals, ""))
    }
}

/// OpenAI refuses to commit less than 100 ms of sound.
pub fn min_commit(p: Provider, rate: u32) -> usize {
    match p {
        Provider::OpenAi | Provider::Groq => rate as usize / 10,
        _ => 1,
    }
}

/// The sum of what a socket said when it would not open (`ws/open`: `<host>: HTTP <status>: <what it answered>`).
fn refused_at_opening(spec: &Spec, message: &str) -> OnlineError {
    let p = spec.provider;
    let Some((status, body)) = message.split_once("HTTP ").and_then(|(_, rest)| {
        let (status, body) = rest.split_once(": ").unwrap_or((rest, ""));
        Some((status.parse::<u16>().ok()?, body))
    }) else {
        return OnlineError::Network(p, redact(message, &spec.key));
    };
    let told = said(body, &spec.key);
    match status {
        401 | 403 => OnlineError::KeyRefused(p),
        429 => OnlineError::Quota(p, told),
        s => OnlineError::Service(p, s, told),
    }
}

/// The stream, from its opening to the sentence's final text.
pub(crate) async fn run(
    host: &Arc<Host>,
    spec: &Spec,
    language: &str,
    rate: u32,
    inbox: &Inbox,
    partial: impl Fn(String),
) -> Result<String, OnlineError> {
    use std::sync::atomic::Ordering;
    let p = spec.provider;
    let target = target_rate(p);
    let (name, value) = key_header(spec);
    let url = url_of(spec, language);
    let opening = host.open_socket(&url, name, &value);
    let mut socket = match host.timeout(CONNECT_WAIT, opening).await {
        None => return Err(OnlineError::Network(p, tr!("délai dépassé", "timed out"))),
        Some(Err(e)) => return Err(refused_at_opening(spec, &e.message)),
        Some(Ok(socket)) => socket,
    };
    let net = |e: String| OnlineError::Network(p, redact(&e, &spec.key));
    for m in hello(spec, language) {
        socket.send(Frame::Text(m)).await.map_err(net)?;
    }
    let chunk = target as usize * CHUNK_MS / 1000;
    let mut queued: Vec<f32> = Vec::new();
    // The sound sent since the last commit.
    let mut uncommitted = 0usize;
    let mut heard = Heard::default();
    let mut ended = false;
    let mut last_partial = String::new();
    enum Next {
        Cmd(Cmd),
        Dropped,
        Socket(SocketEvent),
    }
    loop {
        let next = poll_fn(|_| {
            if !ended {
                if let Some(cmd) = inbox.cmds.lock().pop_front() {
                    return Poll::Ready(Next::Cmd(cmd));
                }
            }
            // Past its end too: a socket held for a text nobody waits for is one of the four of the plugin.
            if inbox.dropped.load(Ordering::Acquire) {
                return Poll::Ready(Next::Dropped);
            }
            socket.poll_event().map(Next::Socket)
        })
        .await;
        match next {
            Next::Cmd(cmd) => {
                let (commit, end) = match cmd {
                    Cmd::Audio(s) => {
                        queued.extend(crate::resample(&s, rate, target));
                        if queued.len() < chunk {
                            continue;
                        }
                        (false, false)
                    }
                    Cmd::End => (true, true),
                };
                let out = std::mem::take(&mut queued);
                uncommitted += out.len();
                // Too little to commit (OpenAI wants 100 ms): no commit, its answer would not come.
                let commit = commit && uncommitted >= min_commit(p, target);
                for m in audio_messages(p, &out, commit, end) {
                    socket.send(m).await.map_err(net)?;
                }
                if commit {
                    uncommitted = 0;
                    heard.commits += 1;
                }
                if end {
                    ended = true;
                    // Every commit already answered: the text is there.
                    if let Some(text) = heard.complete(p, true) {
                        socket.close();
                        return Ok(text);
                    }
                }
            }
            Next::Dropped => {
                socket.close();
                return Err(OnlineError::Network(p, "called off".into()));
            }
            Next::Socket(SocketEvent::Text(t)) => match heard.read(p, &t, ended, &spec.key)? {
                Said::Partial(text) => {
                    if text != last_partial {
                        last_partial = text.clone();
                        partial(text);
                    }
                }
                Said::Final(text) => {
                    socket.close();
                    return Ok(text);
                }
                Said::Nothing => {}
            },
            Next::Socket(SocketEvent::Binary) => {}
            Next::Socket(SocketEvent::Closed { code, reason }) => {
                // Closed once the end was sent: Deepgram's way of saying its last words.
                if ended {
                    if let Some(text) = heard.closed(p) {
                        return Ok(text);
                    }
                }
                return Err(match (code, reason.as_str()) {
                    // No close frame: the connection went without a word.
                    (1005, _) => net(tr!("flux fermé ()", "stream closed ()")),
                    (1006, "the connection closed") => net(tr!("flux fermé", "stream closed")),
                    (1006, why) => net(why.to_string()),
                    (code, reason) => {
                        let why = format!("{code} {reason}");
                        net(tr!("flux fermé ({why})", "stream closed ({why})"))
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(p: Provider) -> Spec {
        let mut s = Spec::new(p, "sk-0123456789".into());
        s.base = "https://x.dev".into();
        s
    }

    #[test]
    fn each_stream_says_its_address() {
        let mut s = spec(Provider::ElevenLabs);
        s.live = Some("scribe_v2_realtime".into());
        assert_eq!(
            url_of(&s, "fr"),
            "wss://x.dev/v1/speech-to-text/realtime?model_id=scribe_v2_realtime&audio_format=pcm_16000&commit_strategy=manual&language_code=fr"
        );
        let d = url_of(&spec(Provider::Deepgram), "auto");
        assert!(d.ends_with("&smart_format=true&language=multi"), "{d}");
        assert!(
            url_of(&spec(Provider::OpenAi), "fr").ends_with("/v1/realtime?intent=transcription")
        );
        let mut local = spec(Provider::Mistral);
        local.base = "http://127.0.0.1:9".into();
        assert!(url_of(&local, "fr").starts_with("ws://127.0.0.1:9/"));
    }

    #[test]
    fn a_refusal_at_the_opening_says_what_the_service_said_without_the_key() {
        let s = spec(Provider::Deepgram);
        let p = s.provider;
        assert_eq!(
            refused_at_opening(&s, "x.dev: HTTP 401: {}"),
            OnlineError::KeyRefused(p)
        );
        assert_eq!(
            refused_at_opening(&s, r#"x.dev: HTTP 429: {"message":"slow sk-0123456789"}"#),
            OnlineError::Quota(p, "slow ***".into())
        );
        assert_eq!(
            refused_at_opening(&s, "x.dev: HTTP 500: boom"),
            OnlineError::Service(p, 500, "boom".into())
        );
        assert_eq!(
            refused_at_opening(&s, "x.dev: no route"),
            OnlineError::Network(p, "x.dev: no route".into())
        );
    }

    #[test]
    fn a_commit_needs_what_openai_wants() {
        assert_eq!(min_commit(Provider::OpenAi, 24_000), 2_400);
        assert_eq!(min_commit(Provider::Deepgram, 16_000), 1);
    }

    #[test]
    fn deepgram_says_its_last_words_only_once_finalized() {
        let p = Provider::Deepgram;
        let mut h = Heard::default();
        let said = r#"{"type":"Results","is_final":true,"channel":{"alternatives":[{"transcript":"bonjour"}]}}"#;
        assert_eq!(
            h.read(p, said, false, "").unwrap(),
            Said::Partial("bonjour".into())
        );
        assert_eq!(h.closed(p), None, "a cut, not the end");
        let last = r#"{"type":"Results","is_final":true,"from_finalize":true,"channel":{"alternatives":[{"transcript":"le monde"}]}}"#;
        h.read(p, last, true, "").unwrap();
        assert_eq!(h.closed(p).as_deref(), Some("bonjour le monde"));
    }
}
