//! The dictation transcribed by a service on line, with the user's key (ESC-110), off by default:
//! the sound of the dictation (a WAV file, mono, 16 kHz) goes to the service chosen once the user
//! stops, and its text comes back. Nothing else goes: neither the project, nor the conversation.
//!
//! OpenAI, Groq and Mistral take the same request (`/v1/audio/transcriptions`, multipart, the key
//! as a bearer); ElevenLabs (`/v1/speech-to-text`) and Deepgram (`/v1/listen`, the raw sound) each
//! have their own. The key is kept in the system's keychain (`secrets`), never in a file nor in
//! the log: what a service answers is written down with the key taken out.

use super::transcribe::Transcriber;
use crate::integrations::secrets::SecretStore;
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A service the dictation can go to, by its id (INT-13): the five of the core are the constants below, a plugin's
/// is `Provider::of(id)`. The id is interned (a plugin's service lives as long as the app), so that a `Provider` is
/// `Copy` and a pattern like `Provider::OpenAi` reads as before.
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

    /// The core's own service of this id.
    pub fn core(id: &str) -> Option<Provider> {
        PROVIDERS.into_iter().find(|p| p.0 == id)
    }

    /// The service of this id, the core's or a plugin's registered one.
    pub fn find(id: &str) -> Option<Provider> {
        Self::core(id).or_else(|| super::service::find(id).map(|_| Provider::of(id)))
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

/// The network settings a WebSocket follows (ESC-136), as reqwest's client does: the proxy, who goes around it, and
/// whether a certificate is checked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Net {
    pub proxy: String,
    pub no_proxy: String,
    pub insecure: bool,
}

impl Net {
    pub fn of(settings: &crate::model::Settings) -> Self {
        Self {
            proxy: settings.proxy_url.trim().to_string(),
            no_proxy: settings.no_proxy.clone(),
            insecure: settings.insecure_tls,
        }
    }
}

/// What a transcription on line is given: the service, the key, the client of the settings (their
/// proxy), and where the service is. Never `Debug` with the key.
#[derive(Clone)]
pub struct Spec {
    pub provider: Provider,
    /// The model it is asked for (`Provider::model` by default; the one chosen in the settings).
    pub model: String,
    pub key: String,
    pub client: reqwest::Client,
    pub base: String,
    /// « Transcription en temps réel » (ESC-136): the model it streams with; none: each sentence goes whole.
    pub live: Option<String>,
    /// What the WebSocket of the transcription as one speaks follows: the client's settings.
    pub net: Net,
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

/// The same service, asked the same way (its client aside).
impl PartialEq for Spec {
    fn eq(&self, o: &Self) -> bool {
        self.provider == o.provider
            && self.model == o.model
            && self.key == o.key
            && self.base == o.base
            && self.live == o.live
            && self.net == o.net
    }
}

impl Spec {
    pub fn new(provider: Provider, key: String, client: reqwest::Client) -> Self {
        Self {
            provider,
            model: super::service::of(provider).default_model(),
            key,
            client,
            base: provider.base_url().into(),
            live: None,
            net: Net::default(),
        }
    }

    /// Streamed with `model` (ESC-136), when the service offers it.
    pub fn with_live(mut self, model: Option<String>) -> Self {
        let offers = super::service::of(self.provider).offers_live();
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
    pub fn message(&self, lang: crate::i18n::Lang) -> String {
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
        f.write_str(&self.message(crate::i18n::ui()))
    }
}

impl std::error::Error for OnlineError {}

/// The sound (mono, 16 kHz, in [-1, 1]) as a WAV file, 16 bits.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: super::audio::WHISPER_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut out = std::io::Cursor::new(Vec::with_capacity(44 + samples.len() * 2));
    {
        let mut w = hound::WavWriter::new(&mut out, spec).expect("a WAV header in memory");
        for s in samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            w.write_sample(v).expect("a sample in memory");
        }
        w.finalize().expect("a WAV in memory");
    }
    out.into_inner()
}

/// A multipart body of `fields` and the WAV file `audio`, and its content type.
fn multipart(fields: &[(&str, &str)], audio: &[u8]) -> (Vec<u8>, String) {
    let boundary = format!("escouade-{}", uuid::Uuid::new_v4().simple());
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
fn request(spec: &Spec, audio: &[u8], language: &str) -> reqwest::RequestBuilder {
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
            spec.client
                .post(format!("{base}/v1/audio/transcriptions"))
                .bearer_auth(&spec.key)
                .header(reqwest::header::CONTENT_TYPE, ct)
                .body(body)
        }
        Provider::ElevenLabs => {
            let mut fields = vec![("model_id", model), ("tag_audio_events", "false")];
            if let Some(l) = lang {
                fields.push(("language_code", l));
            }
            let (body, ct) = multipart(&fields, audio);
            spec.client
                .post(format!("{base}/v1/speech-to-text"))
                .header("xi-api-key", &spec.key)
                .header(reqwest::header::CONTENT_TYPE, ct)
                .body(body)
        }
        _ => {
            let mut query = vec![("model", model), ("smart_format", "true")];
            match lang {
                Some(l) => query.push(("language", l)),
                None => query.push(("detect_language", "true")),
            }
            spec.client
                .post(format!("{base}/v1/listen"))
                .query(&query)
                .header(
                    reqwest::header::AUTHORIZATION,
                    format!("Token {}", spec.key),
                )
                .header(reqwest::header::CONTENT_TYPE, "audio/wav")
                .body(audio.to_vec())
        }
    }
}

/// The text of a service's answer.
fn text_of(provider: Provider, body: &serde_json::Value) -> Option<String> {
    let text = match provider {
        Provider::Deepgram => body
            .pointer("/results/channels/0/alternatives/0/transcript")?
            .as_str()?,
        _ => body.get("text")?.as_str()?,
    };
    Some(text.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// What a service said of an error: its message when it gives one, else the start of its answer;
/// the key taken out, should it be there.
pub(super) fn said(body: &str, key: &str) -> String {
    let v: Option<serde_json::Value> = serde_json::from_str(body).ok();
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
        .unwrap_or_else(|| body.chars().take(200).collect());
    redact(msg.trim(), key)
}

/// `text` without `key`.
pub fn redact(text: &str, key: &str) -> String {
    if key.len() < 4 {
        return text.to_string();
    }
    text.replace(key, "***")
}

/// Sends `samples` to the service of `spec`, gives its text; gives up once `cancel` is set.
pub async fn transcribe(
    spec: &Spec,
    samples: &[f32],
    language: &str,
    cancel: &AtomicBool,
) -> std::result::Result<String, OnlineError> {
    let p = spec.provider;
    let audio = wav(samples);
    let started = std::time::Instant::now();
    let send = request(spec, &audio, language).send();
    let watch = async {
        while !cancel.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    };
    let response = tokio::select! {
        r = send => r,
        _ = watch => return Ok(String::new()),
    };
    let response = response.map_err(|e| {
        let e = redact(&network_reason(&e), &spec.key);
        log::warn!("voice: {} not reached: {e}", p.label());
        OnlineError::Network(p, e)
    })?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| OnlineError::Network(p, redact(&network_reason(&e), &spec.key)))?;
    log::info!(
        "voice: {} answered {} in {} ms",
        p.label(),
        status.as_u16(),
        started.elapsed().as_millis()
    );
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        log::warn!(
            "voice: {} refuses the key: {}",
            p.label(),
            said(&body, &spec.key)
        );
        return Err(OnlineError::KeyRefused(p));
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let e = said(&body, &spec.key);
        log::warn!("voice: {} answered 429: {e}", p.label());
        return Err(OnlineError::Quota(p, e));
    }
    if !status.is_success() {
        let e = said(&body, &spec.key);
        log::warn!("voice: {} answered {}: {e}", p.label(), status.as_u16());
        return Err(OnlineError::Service(p, status.as_u16(), e));
    }
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| OnlineError::Service(p, status.as_u16(), said(&body, &spec.key)))?;
    text_of(p, &json).ok_or_else(|| {
        OnlineError::Service(
            p,
            status.as_u16(),
            tr!("réponse sans texte", "an answer without text"),
        )
    })
}

/// What reqwest says of a request that did not get through, with its causes (the proxy's, the
/// connection's).
pub(super) fn network_reason(e: &reqwest::Error) -> String {
    let mut out = if e.is_timeout() {
        tr!("délai dépassé", "timed out")
    } else {
        e.to_string()
    };
    let mut source = std::error::Error::source(e);
    while let Some(s) = source {
        let text = s.to_string();
        if !out.contains(&text) {
            out.push_str(": ");
            out.push_str(&text);
        }
        source = s.source();
    }
    out
}

/// The service of the settings, with its key from the keychain, the model chosen for it (its default when the
/// service no longer offers it) and the client of their proxy; none when the dictation is local. No key kept: `NoKey`,
/// told before anything is listened to.
pub fn spec_of(
    settings: &crate::model::Settings,
    secrets: &dyn SecretStore,
    catalog: &super::online_catalog::CatalogStore,
) -> std::result::Result<Option<Spec>, super::VoiceError> {
    let Some(p) = super::online_provider(&settings.voice) else {
        return Ok(None);
    };
    let chosen = settings
        .voice
        .online_models
        .get(p.id())
        .map(String::as_str)
        .unwrap_or("");
    let (model, _) = catalog.checked_model(p, super::online_catalog::Purpose::Transcribe, chosen);
    // « Transcription en temps réel », on unless unticked (ESC-136).
    let live = settings
        .voice
        .online_live
        .get(p.id())
        .copied()
        .unwrap_or(true);
    Ok(Some(
        keyed(p, settings, secrets)?
            .with_model(&model)
            .with_live(live.then(|| super::service::of(p).live_model(&model))),
    ))
}

/// The entry of the voice of the plugin Mistral, the former name of the Mistral account (ESC-327).
const MISTRAL_ENTRY: &str = "dictation-mistral";

/// The key under the entry `entry` of a service, for the plugin Mistral under the former name read first and copied
/// under the account's (the migration of INT-6).
fn read_key(secrets: &dyn SecretStore, entry: &str) -> anyhow::Result<Option<String>> {
    if entry == MISTRAL_ENTRY {
        let name = crate::plugins::accounts::mistral_vault_name();
        crate::integrations::secrets::read_entry(secrets, Some(entry), &name, entry)
    } else {
        secrets.get(entry)
    }
}

/// The key of the Mistral account, if the keychain gives one.
pub fn mistral_key(secrets: &dyn SecretStore) -> Option<String> {
    read_key(secrets, MISTRAL_ENTRY)
        .unwrap_or_else(|e| {
            log::warn!("voice: the system keychain did not give the key of Mistral: {e:#}");
            None
        })
        .filter(|k| !k.trim().is_empty())
}

/// What `p`'s key is when it has none of its own: for Mistral, the first Vibe account's (ESC-327).
fn borrowed_key(p: Provider, settings: &crate::model::Settings) -> Option<String> {
    (p == Provider::Mistral)
        .then(|| crate::accounts::vibe_key_for_voice(settings))
        .flatten()
        .map(|(_, key)| key)
}

/// `p` with its key from the keychain (the one of its dictation, ESC-132: one entry by service) and the client of the
/// settings' proxy.
pub fn keyed(
    p: Provider,
    settings: &crate::model::Settings,
    secrets: &dyn SecretStore,
) -> std::result::Result<Spec, super::VoiceError> {
    let key = read_key(secrets, &p.entry())
        .unwrap_or_else(|e| {
            log::warn!(
                "voice: the system keychain did not give the key of {}: {e:#}",
                p.label()
            );
            None
        })
        .filter(|k| !k.trim().is_empty())
        .or_else(|| borrowed_key(p, settings))
        .ok_or(super::VoiceError::Online(OnlineError::NoKey(p)))?;
    let client =
        super::models::client(settings).map_err(|e| super::VoiceError::Other(e.to_string()))?;
    Ok(Spec {
        net: Net::of(settings),
        ..Spec::new(p, key, client)
    })
}

/// The services whose key is in the keychain (never the keys).
pub fn keys(secrets: &dyn SecretStore) -> Vec<String> {
    super::service::services()
        .into_iter()
        .filter(|s| {
            read_key(secrets, &s.account_entry())
                .ok()
                .flatten()
                .is_some_and(|k| !k.trim().is_empty())
        })
        .map(|s| s.id())
        .collect()
}

/// `keys`, with the service whose key is another account's: Mistral on a Vibe account's (ESC-327).
pub fn usable_keys(settings: &crate::model::Settings, secrets: &dyn SecretStore) -> Vec<String> {
    let mut list = keys(secrets);
    let id = Provider::Mistral.id().to_string();
    if !list.contains(&id)
        && super::service::find(&id).is_some()
        && borrowed_key(Provider::Mistral, settings).is_some()
    {
        list.push(id);
    }
    list
}

/// Réglages › Voix and the accounts of Mistral Vibe: the key of the Mistral account, by its end, and the Vibe account
/// whose key the voice goes by when it has none.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MistralKey {
    pub own: Option<String>,
    pub borrowed: Option<Borrowed>,
}

/// A Vibe account the voice borrows its key from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Borrowed {
    pub account: String,
    pub hint: String,
}

/// What `MistralKey` says now.
pub fn mistral_state(settings: &crate::model::Settings) -> MistralKey {
    let own = crate::accounts::mistral_key_hint();
    let borrowed = crate::accounts::vibe_key_for_voice(settings)
        .filter(|_| own.is_none())
        .map(|(account, key)| Borrowed {
            account,
            hint: crate::accounts::key_hint(&key),
        });
    MistralKey { own, borrowed }
}

/// Keeps the key of `service` in the keychain, read back to check; never in a file: a keychain
/// that refuses it (or none, on Linux) leaves it unsaved, and says so, the key taken out.
pub fn save_key(
    secrets: &dyn SecretStore,
    service: &str,
    key: &str,
) -> std::result::Result<(), String> {
    let p = super::service::find(service).ok_or_else(|| format!("unknown service {service}"))?;
    let key = key.trim();
    if key.is_empty() {
        return Err(tr!("La clé est vide.", "The key is empty."));
    }
    keep_key(secrets, &p.account_entry(), key).map_err(|e| {
        let e = redact(&format!("{e:#}"), key);
        log::warn!(
            "voice: the system keychain refused the key of {}: {e}",
            p.label()
        );
        tr!(
            "Le trousseau du système refuse la clé ({e}) : elle n’est pas gardée, et jamais dans un fichier.",
            "The system keychain refuses the key ({e}): it is not kept, and never in a file."
        )
    })?;
    log::info!("voice: key of {} kept in the system keychain", p.label());
    Ok(())
}

/// The key kept under `entry` and, for the Mistral account, under its name of the plugin too.
fn keep_key(secrets: &dyn SecretStore, entry: &str, key: &str) -> anyhow::Result<()> {
    use crate::integrations::secrets::keep_secret;
    keep_secret(secrets, entry, key)?;
    if entry == MISTRAL_ENTRY {
        keep_secret(
            secrets,
            &crate::plugins::accounts::mistral_vault_name(),
            key,
        )?;
        crate::accounts::remember_mistral_key(Some(key));
    }
    Ok(())
}

/// Forgets the key of `service`, under each of its names.
pub fn delete_key(secrets: &dyn SecretStore, service: &str) -> std::result::Result<(), String> {
    let p = super::service::find(service).ok_or_else(|| format!("unknown service {service}"))?;
    let entry = p.account_entry();
    secrets.delete(&entry).map_err(|e| format!("{e:#}"))?;
    if entry == MISTRAL_ENTRY {
        secrets
            .delete(&crate::plugins::accounts::mistral_vault_name())
            .map_err(|e| format!("{e:#}"))?;
        crate::accounts::remember_mistral_key(None);
    }
    Ok(())
}

/// The transcriber of a dictation on line: what `Voice` calls, as it calls Whisper. It asks the service of the
/// contract (`service::VoiceService`), which runs the request on a runtime of its own, on a thread of its own:
/// `Voice::stop` blocks.
pub struct Online(pub Spec);

impl Transcriber for Online {
    fn transcribe(
        &self,
        _model: &Path,
        samples: &[f32],
        language: &str,
        cancel: &Arc<AtomicBool>,
    ) -> Result<String> {
        super::service::of(self.0.provider)
            .transcribe(&self.0, samples, language, cancel)
            .map_err(anyhow::Error::new)
    }
}

#[cfg(test)]
#[path = "online_tests.rs"]
pub(super) mod tests;
