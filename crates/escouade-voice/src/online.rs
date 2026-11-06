//! The dictation transcribed by a service on line, with the user's key (ESC-110): the sound of the dictation (a WAV file,
//! mono, 16 kHz) goes to the service once the user stops, and its text comes back. Nothing else goes: neither the project,
//! nor the conversation.
//!
//! OpenAI, Groq and Mistral take the same request (`/v1/audio/transcriptions`, multipart, the key as a bearer); ElevenLabs
//! (`/v1/speech-to-text`) and Deepgram (`/v1/listen`, the raw sound) each have their own. The key comes with each call and
//! is kept nowhere: what a service answers is written down with the key taken out.

use base64::Engine;
use escouade_plugin_kit::i18n::{self, Lang};
use escouade_plugin_kit::tr_in;
use serde_json::{json, Value};
use std::hash::{BuildHasher, Hasher};

/// A service the dictation can go to, by its id (INT-13): the five of Escouade are the constants below, a plugin's is
/// `Provider::of(id)`. The id is interned (a plugin's service lives as long as the app), so that a `Provider` is `Copy` and
/// a pattern like `Provider::OpenAi` reads as it should.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Provider(&'static str);

#[allow(non_upper_case_globals)]
impl Provider {
    pub const OpenAi: Provider = Provider("openai");
    pub const Groq: Provider = Provider("groq");
    pub const Mistral: Provider = Provider("mistral");
    pub const ElevenLabs: Provider = Provider("elevenlabs");
    pub const Deepgram: Provider = Provider("deepgram");
}

pub const PROVIDERS: [Provider; 5] = [
    Provider::OpenAi,
    Provider::Groq,
    Provider::Mistral,
    Provider::ElevenLabs,
    Provider::Deepgram,
];

impl Provider {
    /// The service of this id, a plugin's included.
    pub fn of(id: &str) -> Provider {
        if let Some(p) = PROVIDERS.into_iter().find(|p| p.0 == id) {
            return p;
        }
        static SEEN: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(s) = seen.iter().find(|s| **s == id) {
            return Provider(s);
        }
        let leaked: &'static str = Box::leak(id.to_string().into_boxed_str());
        seen.push(leaked);
        Provider(leaked)
    }

    /// Its id in the settings, the keychain and the statistics.
    pub fn id(self) -> &'static str {
        self.0
    }

    /// Escouade's own service of this id.
    pub fn core(id: &str) -> Option<Provider> {
        PROVIDERS.into_iter().find(|p| p.0 == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::OpenAi => "OpenAI",
            Provider::Groq => "Groq",
            Provider::Mistral => "Mistral",
            Provider::ElevenLabs => "ElevenLabs",
            Provider::Deepgram => "Deepgram",
            other => other.0,
        }
    }

    /// Where its API is (a fake server's address in tests).
    pub fn base_url(self) -> &'static str {
        match self {
            Provider::OpenAi => "https://api.openai.com",
            Provider::Groq => "https://api.groq.com/openai",
            Provider::Mistral => "https://api.mistral.ai",
            Provider::ElevenLabs => "https://api.elevenlabs.io",
            Provider::Deepgram => "https://api.deepgram.com",
            _ => "",
        }
    }

    /// The model it transcribes with when the user chose none: the one of ESC-110.
    pub fn model(self) -> &'static str {
        match self {
            Provider::OpenAi => "gpt-4o-mini-transcribe",
            Provider::Groq => "whisper-large-v3-turbo",
            Provider::Mistral => "voxtral-mini-latest",
            Provider::ElevenLabs => "scribe_v2",
            Provider::Deepgram => "nova-3",
            _ => "",
        }
    }

    /// Its key's entry in the keychain.
    pub fn entry(self) -> String {
        format!("dictation-{}", self.id())
    }

    /// Whether it transcribes as one speaks (ESC-136), its text coming while the sound goes: Groq does not.
    pub fn offers_live(self) -> bool {
        self != Provider::Groq
    }

    /// The model it transcribes with as one speaks, `chosen` being the one of the settings: ElevenLabs, Mistral and
    /// OpenAI each have one of their own; Deepgram streams with the one chosen.
    pub fn live_model(self, chosen: &str) -> String {
        match self {
            Provider::ElevenLabs => "scribe_v2_realtime".into(),
            Provider::Mistral => "voxtral-mini-transcribe-realtime-2602".into(),
            Provider::OpenAi => "gpt-live-transcribe".into(),
            _ => chosen.into(),
        }
    }
}

/// What a transcription on line is given: the service, the key, and where the service is. Never `Debug` with the key.
#[derive(Clone, PartialEq)]
pub struct Spec {
    pub provider: Provider,
    /// The model it is asked for (`Provider::model` by default; the one chosen in the settings).
    pub model: String,
    pub key: String,
    pub base: String,
    /// « Transcription en temps réel » (ESC-136): the model it streams with; none: each sentence goes whole.
    pub live: Option<String>,
}

impl std::fmt::Debug for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Spec")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("base", &self.base)
            .field("live", &self.live)
            .finish_non_exhaustive()
    }
}

impl Spec {
    pub fn new(provider: Provider, key: String) -> Self {
        Self {
            provider,
            model: provider.model().into(),
            key,
            base: provider.base_url().into(),
            live: None,
        }
    }

    /// Streamed with `model` (ESC-136), when the service offers it.
    pub fn with_live(mut self, model: Option<String>) -> Self {
        let offers = self.provider.offers_live();
        self.live = model.filter(|m| offers && !m.trim().is_empty());
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        if !model.trim().is_empty() {
            self.model = model.trim().to_string();
        }
        self
    }
}

/// Why a transcription on line failed, as the window tells it.
#[derive(Debug, Clone, PartialEq)]
pub enum OnlineError {
    /// No key kept for the service.
    NoKey(Provider),
    /// The service refuses the key (401, 403).
    KeyRefused(Provider),
    /// The service could not be reached (no network, the proxy, a timeout).
    Network(Provider, String),
    /// It answered something else than a text: its status, and what it said.
    Service(Provider, u16, String),
    /// Too many requests, or the quota of the account is spent (429).
    Quota(Provider, String),
    /// It did not read the passage (ESC-132): its status, and what it said.
    Speech(Provider, u16, String),
}

impl OnlineError {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            OnlineError::NoKey(p) => {
                let p = p.label();
                tr_in!(
                    lang,
                    "Aucune clé {p} n’est enregistrée. Ajoute-la dans Réglages › Voix, ou repasse en local.",
                    "No {p} key is saved. Add it in Settings › Voice, or go back to local."
                )
            }
            OnlineError::KeyRefused(p) => {
                let p = p.label();
                tr_in!(
                    lang,
                    "{p} refuse la clé. Vérifie-la dans Réglages › Voix.",
                    "{p} refuses the key. Check it in Settings › Voice."
                )
            }
            OnlineError::Network(p, e) => {
                let p = p.label();
                tr_in!(
                    lang,
                    "{p} ne répond pas : {e}. Vérifie le réseau et le proxy (Réglages › Réseau), ou repasse en local.",
                    "{p} does not answer: {e}. Check the network and the proxy (Settings › Network), or go back to local."
                )
            }
            OnlineError::Service(p, status, e) => {
                let p = p.label();
                tr_in!(
                    lang,
                    "{p} n’a pas transcrit la dictée ({status}) : {e}",
                    "{p} did not transcribe the dictation ({status}): {e}"
                )
            }
            OnlineError::Quota(p, e) => {
                let p = p.label();
                tr_in!(
                    lang,
                    "{p} refuse pour l’instant : trop de requêtes ou quota épuisé ({e}). Attends un peu, ou vérifie le forfait de ton compte {p}.",
                    "{p} refuses for now: too many requests or the quota is spent ({e}). Wait a little, or check the plan of your {p} account."
                )
            }
            OnlineError::Speech(p, status, e) => {
                let p = p.label();
                tr_in!(
                    lang,
                    "{p} n’a pas lu le passage ({status}) : {e}",
                    "{p} did not read the passage ({status}): {e}"
                )
            }
        }
    }
}

impl std::fmt::Display for OnlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message(i18n::ui()))
    }
}

impl std::error::Error for OnlineError {}

/// An HTTP request, as `net/fetch` and `net/stream` take it.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

impl Call {
    pub fn get(url: String) -> Self {
        Self {
            method: "GET",
            url,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn post(url: String, content_type: &str, body: Vec<u8>) -> Self {
        Self {
            method: "POST",
            url,
            headers: vec![("content-type", content_type.to_string())],
            body,
        }
    }

    /// A JSON body.
    pub fn post_json(url: String, body: &Value) -> Self {
        Self::post(url, "application/json", body.to_string().into_bytes())
    }

    pub fn header(mut self, name: &'static str, value: String) -> Self {
        self.headers.push((name, value));
        self
    }

    /// The key of `spec`, as its service wants it.
    pub fn keyed(self, spec: &Spec) -> Self {
        let (name, value) = key_header(spec);
        self.header(name, value)
    }

    /// What `net/fetch` and `net/stream` are told. A body in base64, whatever it holds: the sound of a dictation is none
    /// of the text `body` carries.
    pub fn params(&self) -> Value {
        let headers: serde_json::Map<String, Value> = self
            .headers
            .iter()
            .map(|(k, v)| (k.to_string(), Value::from(v.as_str())))
            .collect();
        let mut out = json!({ "url": self.url, "method": self.method, "headers": headers });
        if !self.body.is_empty() {
            out["bodyBase64"] = base64::engine::general_purpose::STANDARD
                .encode(&self.body)
                .into();
        }
        out
    }
}

/// How `spec`'s service is told its key.
pub fn key_header(spec: &Spec) -> (&'static str, String) {
    match spec.provider {
        Provider::ElevenLabs => ("xi-api-key", spec.key.clone()),
        Provider::Deepgram => ("authorization", format!("Token {}", spec.key)),
        _ => ("authorization", format!("Bearer {}", spec.key)),
    }
}

/// `query` as a URL's, as a form encodes it.
pub fn query(pairs: &[(&str, &str)]) -> String {
    let encode = |v: &str| -> String {
        v.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                    (b as char).to_string()
                }
                b' ' => "+".to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect()
    };
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// The sound (mono, 16 kHz, 16 bits, little-endian) as a WAV file.
pub fn wav(pcm: &[u8]) -> Vec<u8> {
    const RATE: u32 = 16_000;
    let data = u32::try_from(pcm.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&data.saturating_add(36).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

/// What separates the parts of a request: random, so that no sound holds it. `RandomState` draws from the system, on both
/// targets, without one more crate.
fn boundary() -> String {
    let word = || std::collections::hash_map::RandomState::new().build_hasher();
    let (mut a, mut b) = (word(), word());
    a.write_u8(0);
    b.write_u8(1);
    format!("escouade-{:016x}{:016x}", a.finish(), b.finish())
}

/// A multipart body of `fields` and the WAV file `audio`, and its content type.
fn multipart(fields: &[(&str, &str)], audio: &[u8]) -> (Vec<u8>, String) {
    let boundary = boundary();
    let mut body = Vec::with_capacity(audio.len() + 512);
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"dictation.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(audio);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (body, format!("multipart/form-data; boundary={boundary}"))
}

/// The request of `spec` for `audio` in `language` (Whisper's code, or "auto").
pub fn request(spec: &Spec, audio: &[u8], language: &str) -> Call {
    let base = spec.base.trim_end_matches('/');
    let lang = (language != "auto" && !language.is_empty()).then_some(language);
    let model = spec.model.as_str();
    match spec.provider {
        Provider::OpenAi | Provider::Groq | Provider::Mistral => {
            let mut fields = vec![("model", model), ("response_format", "json")];
            if spec.provider == Provider::Mistral {
                // Mistral takes neither `response_format` nor anything but JSON.
                fields.pop();
            }
            if let Some(l) = lang {
                fields.push(("language", l));
            }
            let (body, ct) = multipart(&fields, audio);
            Call::post(format!("{base}/v1/audio/transcriptions"), &ct, body).keyed(spec)
        }
        Provider::ElevenLabs => {
            let mut fields = vec![("model_id", model), ("tag_audio_events", "false")];
            if let Some(l) = lang {
                fields.push(("language_code", l));
            }
            let (body, ct) = multipart(&fields, audio);
            Call::post(format!("{base}/v1/speech-to-text"), &ct, body).keyed(spec)
        }
        _ => {
            let mut pairs = vec![("model", model), ("smart_format", "true")];
            match lang {
                Some(l) => pairs.push(("language", l)),
                None => pairs.push(("detect_language", "true")),
            }
            let url = format!("{base}/v1/listen?{}", query(&pairs));
            Call::post(url, "audio/wav", audio.to_vec()).keyed(spec)
        }
    }
}

/// The text of a service's answer.
fn text_of(provider: Provider, body: &Value) -> Option<String> {
    let text = match provider {
        Provider::Deepgram => body
            .pointer("/results/channels/0/alternatives/0/transcript")?
            .as_str()?,
        _ => body.get("text")?.as_str()?,
    };
    Some(text.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// What the answer of a dictation says: its text, or why not.
pub fn transcribed(spec: &Spec, status: u16, body: &str) -> Result<String, OnlineError> {
    let p = spec.provider;
    if matches!(status, 401 | 403) {
        eprintln!(
            "voice: {} refuses the key: {}",
            p.label(),
            said(body, &spec.key)
        );
        return Err(OnlineError::KeyRefused(p));
    }
    if status == 429 {
        let e = said(body, &spec.key);
        eprintln!("voice: {} answered 429: {e}", p.label());
        return Err(OnlineError::Quota(p, e));
    }
    if !(200..300).contains(&status) {
        let e = said(body, &spec.key);
        eprintln!("voice: {} answered {status}: {e}", p.label());
        return Err(OnlineError::Service(p, status, e));
    }
    let json: Value = serde_json::from_str(body)
        .map_err(|_| OnlineError::Service(p, status, said(body, &spec.key)))?;
    text_of(p, &json).ok_or_else(|| {
        OnlineError::Service(
            p,
            status,
            escouade_plugin_kit::tr!("réponse sans texte", "an answer without text"),
        )
    })
}

/// What a service said of an error: its message when it gives one, else the start of its answer; the key taken out,
/// should it be there.
pub fn said(body: &str, key: &str) -> String {
    let v: Option<Value> = serde_json::from_str(body).ok();
    let msg = v
        .as_ref()
        .and_then(|v| {
            [
                "/error/message",
                "/detail/message",
                "/message",
                "/err_msg",
                "/detail",
                "/error",
            ]
            .iter()
            .find_map(|p| v.pointer(p).and_then(|m| m.as_str()).map(String::from))
        })
        // Before it is cut: the start of a key across the cut would stay.
        .unwrap_or_else(|| redact(body, key).chars().take(200).collect());
    redact(msg.trim(), key)
}

/// `text` without `key`.
pub fn redact(text: &str, key: &str) -> String {
    if key.len() < 4 {
        return text.to_string();
    }
    text.replace(key, "***")
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "sk-test-0123456789";

    #[test]
    fn a_key_across_the_end_of_what_is_said_of_an_answer_is_not_told() {
        let body = format!("{}{KEY} refused", "x".repeat(195));
        assert!(!said(&body, KEY).contains("sk-t"), "{}", said(&body, KEY));
    }

    fn spec(p: Provider) -> Spec {
        Spec {
            base: "https://x.dev/".into(),
            ..Spec::new(p, KEY.into())
        }
    }

    #[test]
    fn a_wav_says_what_it_holds() {
        let w = wav(&[0, 0, 0xff, 0x7f, 1, 0x80, 0xff, 0x7f]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes([w[4], w[5], w[6], w[7]]), 36 + 8);
        assert_eq!(u32::from_le_bytes([w[24], w[25], w[26], w[27]]), 16_000);
        assert_eq!(u32::from_le_bytes([w[40], w[41], w[42], w[43]]), 8);
        assert_eq!(w.len(), 44 + 8);
        assert_eq!(i16::from_le_bytes([w[50], w[51]]), i16::MAX);
    }

    #[test]
    fn each_service_is_asked_its_way_with_its_key() {
        let key_of = |c: &Call, n: &str| {
            c.headers
                .iter()
                .find(|(k, _)| *k == n)
                .map(|(_, v)| v.clone())
        };
        let o = request(&spec(Provider::OpenAi), b"wav", "fr");
        assert_eq!(o.url, "https://x.dev/v1/audio/transcriptions");
        assert_eq!(
            key_of(&o, "authorization").unwrap(),
            format!("Bearer {KEY}")
        );
        let e = request(&spec(Provider::ElevenLabs), b"wav", "auto");
        assert_eq!(e.url, "https://x.dev/v1/speech-to-text");
        assert_eq!(key_of(&e, "xi-api-key").unwrap(), KEY);
        let d = request(&spec(Provider::Deepgram), b"wav", "auto");
        assert_eq!(
            d.url,
            "https://x.dev/v1/listen?model=nova-3&smart_format=true&detect_language=true"
        );
        assert_eq!(key_of(&d, "authorization").unwrap(), format!("Token {KEY}"));
        assert_eq!(d.body, b"wav");
        assert_eq!(key_of(&d, "content-type").unwrap(), "audio/wav");
    }

    #[test]
    fn mistral_is_not_told_a_response_format() {
        let m = request(&spec(Provider::Mistral), b"wav", "fr");
        let body = String::from_utf8_lossy(&m.body);
        assert!(!body.contains("response_format"), "{body}");
        assert!(body.contains("name=\"language\"\r\n\r\nfr"), "{body}");
    }

    #[test]
    fn two_requests_have_two_boundaries_and_no_sound_is_cut() {
        let a = request(&spec(Provider::Groq), b"\x00\xffsound", "fr");
        let b = request(&spec(Provider::Groq), b"\x00\xffsound", "fr");
        let ct = |c: &Call| c.headers[0].1.clone();
        assert_ne!(ct(&a), ct(&b));
        assert!(a.body.windows(7).any(|w| w == b"\x00\xffsound"));
        let params = a.params();
        assert!(params["bodyBase64"].is_string());
        assert!(params.get("body").is_none());
    }

    #[test]
    fn a_query_is_encoded_as_a_form() {
        assert_eq!(
            query(&[("next_page_token", "a b/c:d"), ("x", "é")]),
            "next_page_token=a+b%2Fc%3Ad&x=%C3%A9"
        );
    }

    #[test]
    fn an_answer_gives_its_text_or_says_why_not_without_the_key() {
        let s = spec(Provider::Deepgram);
        let ok = r#"{"results":{"channels":[{"alternatives":[{"transcript":" Bonjour   le monde "}]}]}}"#;
        assert_eq!(transcribed(&s, 200, ok).unwrap(), "Bonjour le monde");
        let p = Provider::Deepgram;
        assert_eq!(transcribed(&s, 401, "{}"), Err(OnlineError::KeyRefused(p)),);
        let quota = format!(r#"{{"error":{{"message":"slow {KEY}"}}}}"#);
        assert_eq!(
            transcribed(&s, 429, &quota),
            Err(OnlineError::Quota(p, "slow ***".into())),
        );
        assert_eq!(
            transcribed(&s, 500, "<html>boom"),
            Err(OnlineError::Service(p, 500, "<html>boom".into())),
        );
        assert!(matches!(
            transcribed(&s, 200, "{}"),
            Err(OnlineError::Service(_, 200, _))
        ));
    }

    #[test]
    fn the_key_never_shows_in_the_spec_debug() {
        let shown = format!("{:?}", spec(Provider::OpenAi));
        assert!(!shown.contains(KEY), "{shown}");
        assert_eq!(redact("abc", "ab"), "abc", "too short to be a key");
    }
}
