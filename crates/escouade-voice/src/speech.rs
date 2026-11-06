//! The conversation's voice made by a service on line (ESC-132), with the key of its dictation: the text of each
//! passage read goes to the service, sentence by sentence, and its sound comes back as it is made (streamed, when the
//! service streams it). Never the sound of the microphone.
//!
//! OpenAI (`/v1/audio/speech`, raw PCM), Groq (the same, a WAV file), Mistral (`/v1/audio/speech`, server-sent events
//! of float PCM in base64), ElevenLabs (`/v1/text-to-speech/<voice>/stream`, raw PCM) and Deepgram (`/v1/speak`, raw
//! PCM). The key is never written down, nor the text read.

use crate::catalog::speed_range;
use crate::online::{query, redact, said, Call, OnlineError, Provider, Spec};
use base64::Engine as _;
use escouade_plugin_kit::host::Host;
use std::future::{poll_fn, Future};
use std::pin::pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, Instant};

/// How a passage is read: the model, the voice, the conversation's language (`fr`, `en`) and the pace (1: the
/// voice's own).
#[derive(Debug, Clone, PartialEq)]
pub struct SpeakChoice {
    pub model: String,
    pub voice: String,
    pub lang: String,
    pub rate: f32,
}

/// The longest text it takes at once (Groq's Orpheus: 200 characters).
pub fn max_chars(p: Provider) -> usize {
    match p {
        Provider::Groq => 200,
        Provider::Deepgram => 2000,
        Provider::OpenAi | Provider::Mistral => 4000,
        _ => 5000,
    }
}

/// Raw samples, as they come: 16-bit integers or 32-bit floats, little-endian, of `channels` interleaved.
#[derive(Debug, Clone)]
pub struct Pcm {
    rate: u32,
    channels: u16,
    float: bool,
    carry: Vec<u8>,
}

impl Pcm {
    fn new(rate: u32, channels: u16, float: bool) -> Self {
        Self {
            rate,
            channels: channels.max(1),
            float,
            carry: Vec::new(),
        }
    }

    /// The mono samples of `bytes` (what does not make a whole frame waits for the next ones).
    fn feed(&mut self, bytes: &[u8]) -> Vec<f32> {
        self.carry.extend_from_slice(bytes);
        let width = if self.float { 4 } else { 2 };
        let frame = width * self.channels as usize;
        let whole = self.carry.len() / frame * frame;
        let data: Vec<u8> = self.carry.drain(..whole).collect();
        data.chunks(frame)
            .map(|f| {
                let sum: f32 = f
                    .chunks(width)
                    .map(|b| {
                        if self.float {
                            f32::from_le_bytes([b[0], b[1], b[2], b[3]])
                        } else {
                            i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0
                        }
                    })
                    .sum();
                (sum / self.channels as f32).clamp(-1.0, 1.0)
            })
            .collect()
    }
}

/// What turns a service's answer into samples, as its bytes come.
pub enum Decoder {
    Raw(Pcm),
    /// A WAV file: its header first, then its samples (its sizes not trusted: a streamed one does not know them).
    Wav {
        head: Vec<u8>,
        pcm: Option<Pcm>,
    },
    /// Server-sent events whose `audio_data` is base64 (Mistral), or one JSON of it.
    Events {
        text: String,
        pcm: Pcm,
    },
}

impl Decoder {
    pub fn rate(&self) -> Option<u32> {
        match self {
            Decoder::Raw(p) | Decoder::Events { pcm: p, .. } => Some(p.rate),
            Decoder::Wav { pcm, .. } => pcm.as_ref().map(|p| p.rate),
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<f32>, String> {
        match self {
            Decoder::Raw(p) => Ok(p.feed(bytes)),
            Decoder::Wav { head, pcm } => {
                if let Some(p) = pcm {
                    return Ok(p.feed(bytes));
                }
                head.extend_from_slice(bytes);
                let Some((p, at)) = wav_header(head)? else {
                    return Ok(Vec::new());
                };
                let rest = head.split_off(at);
                let mut p = p;
                let out = p.feed(&rest);
                *pcm = Some(p);
                Ok(out)
            }
            Decoder::Events { text, pcm } => {
                text.push_str(&String::from_utf8_lossy(bytes));
                let mut out = Vec::new();
                while let Some(end) = text.find('\n') {
                    let line: String = text.drain(..=end).collect();
                    if let Some(data) = line.trim().strip_prefix("data:") {
                        out.extend(event_samples(data.trim(), pcm)?);
                    }
                }
                Ok(out)
            }
        }
    }

    /// What is left once the answer is over (a JSON answer, not events).
    pub fn finish(&mut self) -> Result<Vec<f32>, String> {
        match self {
            Decoder::Events { text, pcm } => {
                let rest = std::mem::take(text);
                let rest = rest.trim();
                if rest.is_empty() {
                    return Ok(Vec::new());
                }
                let data = rest
                    .strip_prefix("data:")
                    .unwrap_or(rest)
                    .trim()
                    .to_string();
                event_samples(&data, pcm)
            }
            Decoder::Wav { pcm: None, head } if !head.is_empty() => Err("not a WAV file".into()),
            _ => Ok(Vec::new()),
        }
    }
}

fn event_samples(data: &str, pcm: &mut Pcm) -> Result<Vec<f32>, String> {
    if data.is_empty() || data == "[DONE]" {
        return Ok(Vec::new());
    }
    let v: serde_json::Value = serde_json::from_str(data).map_err(|e| e.to_string())?;
    if let Some(e) = v.get("error") {
        return Err(e
            .get("message")
            .and_then(|m| m.as_str())
            .map(String::from)
            .unwrap_or_else(|| e.to_string()));
    }
    let Some(b64) = v.get("audio_data").and_then(|a| a.as_str()) else {
        return Ok(Vec::new());
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| e.to_string())?;
    Ok(pcm.feed(&bytes))
}

/// The format of a WAV header, and where its samples start; none while the header is not whole.
fn wav_header(b: &[u8]) -> Result<Option<(Pcm, usize)>, String> {
    if b.len() < 12 {
        return Ok(None);
    }
    if &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let mut at = 12;
    let mut fmt: Option<Pcm> = None;
    while at + 8 <= b.len() {
        let id = &b[at..at + 4];
        let size = u32::from_le_bytes([b[at + 4], b[at + 5], b[at + 6], b[at + 7]]) as usize;
        if id == b"data" {
            let Some(p) = fmt else {
                return Err("a WAV file without its format".into());
            };
            return Ok(Some((p, at + 8)));
        }
        if at + 8 + size > b.len() {
            return Ok(None);
        }
        if id == b"fmt " && size >= 16 {
            let c = &b[at + 8..];
            let tag = u16::from_le_bytes([c[0], c[1]]);
            let channels = u16::from_le_bytes([c[2], c[3]]);
            let rate = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
            let bits = u16::from_le_bytes([c[14], c[15]]);
            let float = tag == 3 || (tag == 0xFFFE && bits == 32 && size >= 40 && c[24] == 3);
            if !(bits == 16 || (bits == 32 && float)) {
                return Err(format!("a WAV file of {bits} bits"));
            }
            fmt = Some(Pcm::new(rate, channels, float));
        }
        at += 8 + size + (size & 1);
    }
    Ok(None)
}

/// The sample rate asked of those whose rate is chosen.
pub const RATE: u32 = 24_000;

/// Whether `model` of ElevenLabs is told the language (its multilingual v2 and English ones are not).
fn eleven_knows_language(model: &str) -> bool {
    !(model.contains("multilingual_v2") || model.contains("monolingual") || model.ends_with("_v2"))
}

/// The request for `sentence`, and how its answer is read.
pub fn request(spec: &Spec, c: &SpeakChoice, sentence: &str) -> (Call, Decoder) {
    let base = spec.base.trim_end_matches('/');
    let p = spec.provider;
    let range = speed_range(p);
    let speed = range
        .map(|[lo, hi]| c.rate.clamp(lo, hi))
        .filter(|s| (s - 1.0).abs() > 0.01);
    match p {
        Provider::OpenAi | Provider::Groq => {
            let mut body = serde_json::json!({
                "model": c.model,
                "input": sentence,
                "voice": c.voice,
                "response_format": if p == Provider::OpenAi { "pcm" } else { "wav" },
            });
            if p == Provider::OpenAi {
                if let Some(s) = speed {
                    body["speed"] = s.into();
                }
                // Its gpt-4o voices follow instructions: the conversation's language.
                if c.model.starts_with("gpt-") {
                    body["instructions"] = if c.lang == "fr" {
                        "Lis en français, d’une voix naturelle.".into()
                    } else {
                        "Read in English, in a natural voice.".into()
                    };
                }
            }
            let decoder = if p == Provider::OpenAi {
                Decoder::Raw(Pcm::new(RATE, 1, false))
            } else {
                Decoder::Wav {
                    head: Vec::new(),
                    pcm: None,
                }
            };
            (
                Call::post_json(format!("{base}/v1/audio/speech"), &body).keyed(spec),
                decoder,
            )
        }
        Provider::Mistral => (
            Call::post_json(
                format!("{base}/v1/audio/speech"),
                &serde_json::json!({
                    "model": c.model,
                    "input": sentence,
                    "voice_id": c.voice,
                    "response_format": "pcm",
                    "stream": true,
                }),
            )
            .keyed(spec)
            .header("accept", "text/event-stream".into()),
            Decoder::Events {
                text: String::new(),
                pcm: Pcm::new(RATE, 1, true),
            },
        ),
        Provider::ElevenLabs => {
            let mut body = serde_json::json!({ "text": sentence, "model_id": c.model });
            if eleven_knows_language(&c.model) {
                body["language_code"] = c.lang.clone().into();
            }
            if let Some(s) = speed {
                body["voice_settings"] = serde_json::json!({ "speed": s });
            }
            let voice: String = url_segment(&c.voice);
            let format = format!("pcm_{RATE}");
            let pairs = query(&[("output_format", format.as_str())]);
            (
                Call::post_json(
                    format!("{base}/v1/text-to-speech/{voice}/stream?{pairs}"),
                    &body,
                )
                .keyed(spec),
                Decoder::Raw(Pcm::new(RATE, 1, false)),
            )
        }
        _ => {
            let rate = RATE.to_string();
            let mut pairs = vec![
                ("model", c.voice.as_str()),
                ("encoding", "linear16"),
                ("sample_rate", rate.as_str()),
                ("container", "none"),
            ];
            // Its pace is not taken in every language: English only.
            let pace = speed.filter(|_| c.lang == "en").map(|s| format!("{s:.2}"));
            if let Some(s) = &pace {
                pairs.push(("speed", s));
            }
            (
                Call::post_json(
                    format!("{base}/v1/speak?{}", query(&pairs)),
                    &serde_json::json!({ "text": sentence }),
                )
                .keyed(spec),
                Decoder::Raw(Pcm::new(RATE, 1, false)),
            )
        }
    }
}

/// A voice's id as a piece of a path.
fn url_segment(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect()
}

/// Sound as it comes: mono samples, and their rate.
#[derive(Debug, Clone, PartialEq)]
pub struct Audio {
    pub samples: Vec<f32>,
    pub rate: u32,
}

/// `work`, or None once `cancel` is set; dropped then.
pub(crate) async fn or_cancelled<F: Future>(cancel: &AtomicBool, work: F) -> Option<F::Output> {
    let mut work = pin!(work);
    poll_fn(|task| {
        if cancel.load(Ordering::Acquire) {
            return Poll::Ready(None);
        }
        work.as_mut().poll(task).map(Some)
    })
    .await
}

/// Reads `sentence` with the service of `spec`: its sound given to `on` as it comes (a piece at a time). Gives up, without
/// an error, once `cancel` is set. Returns how long its first sound took.
pub async fn speak(
    host: &Arc<Host>,
    spec: &Spec,
    choice: &SpeakChoice,
    sentence: &str,
    cancel: &AtomicBool,
    on: &mut (dyn FnMut(Audio) + Send),
) -> Result<Option<Duration>, OnlineError> {
    let p = spec.provider;
    let started = Instant::now();
    let (call, mut decoder) = request(spec, choice, sentence);
    let network = |e: String| OnlineError::Network(p, redact(&e, &spec.key));
    let Some(response) = or_cancelled(cancel, host.stream(call.params())).await else {
        return Ok(None);
    };
    let mut response = response.map_err(|e| network(e.message))?;
    let status = response.status();
    if !(200..300).contains(&status) {
        let mut body = Vec::new();
        while let Some(Ok(bytes)) = response.next().await {
            body.extend_from_slice(&bytes);
        }
        let e = said(&String::from_utf8_lossy(&body), &spec.key);
        eprintln!("speech: {} answered {status}: {e}", p.label());
        return Err(match status {
            401 | 403 => OnlineError::KeyRefused(p),
            429 => OnlineError::Quota(p, e),
            s => OnlineError::Speech(p, s, e),
        });
    }
    let mut first = None;
    let mut give = |samples: Vec<f32>, rate: u32, first: &mut Option<Duration>| {
        if samples.is_empty() {
            return;
        }
        if first.is_none() {
            *first = Some(started.elapsed());
        }
        on(Audio { samples, rate });
    };
    loop {
        let Some(piece) = or_cancelled(cancel, response.next()).await else {
            return Ok(first);
        };
        let Some(piece) = piece else { break };
        let bytes = piece.map_err(network)?;
        // Mistral answers one JSON when it does not stream.
        let samples = decoder
            .feed(&bytes)
            .map_err(|e| OnlineError::Speech(p, status, redact(&e, &spec.key)))?;
        if let Some(rate) = decoder.rate() {
            give(samples, rate, &mut first);
        }
    }
    let rest = decoder
        .finish()
        .map_err(|e| OnlineError::Speech(p, status, redact(&e, &spec.key)))?;
    if let Some(rate) = decoder.rate() {
        give(rest, rate, &mut first);
    }
    if first.is_none() {
        return Err(OnlineError::Speech(
            p,
            status,
            escouade_plugin_kit::tr!("réponse sans son", "an answer without sound"),
        ));
    }
    Ok(first)
}

/// Whether `e` says the voice (or the model) is not the service's: then its default voice is tried.
pub fn refuses_the_voice(e: &OnlineError) -> bool {
    matches!(e, OnlineError::Speech(_, 400 | 404 | 422, _))
}

/// Where the voice chosen for `model` of `p` in `lang` is kept in the settings.
pub fn voice_key(p: Provider, model: &str, lang: &str) -> String {
    if p == Provider::Deepgram {
        format!("{}|{model}|{lang}", p.id())
    } else {
        format!("{}|{lang}", p.id())
    }
}
