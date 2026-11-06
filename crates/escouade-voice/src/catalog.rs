//! The models and the voices of each service on line (ESC-132): what transcribes a dictation, what reads the
//! conversation aloud, and with which voices. Each service's own list when its API gives one, read with the user's key,
//! kept by the app and read again on demand; else the models its documentation
//! names today, shipped with the app. A voice is never made up: a service whose API lists none gets a free field, with
//! the default voice its documentation gives.
//!
//! What goes to the service to list them: the key only.

use crate::online::{key_header, query, said, Call, OnlineError, Provider, Spec};
use escouade_plugin_kit::host::Host;
use escouade_plugin_kit::tr;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// What a model is asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Transcribe,
    Speak,
}

/// A model of a service, as the settings offer it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub id: String,
    /// Its name, readable.
    pub label: String,
    /// What sets it apart, in a few words (fast, quality, languages), when the service says it.
    #[serde(default)]
    pub note: String,
    /// The languages it speaks (`fr`, `en`…), when the service says it; empty: those of the service.
    #[serde(default)]
    pub langs: Vec<String>,
}

/// A voice of a service, as its API lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceOption {
    /// What the service is asked for.
    pub id: String,
    pub name: String,
    /// The languages it speaks (`fr`, `en`…); empty: any.
    #[serde(default)]
    pub langs: Vec<String>,
    /// The model it belongs to (Deepgram's voices are models of their own); empty: any.
    #[serde(default)]
    pub model: String,
    /// Its gender, accent or description, when the service gives them.
    #[serde(default)]
    pub note: String,
    /// Made or cloned on the user's account.
    #[serde(default)]
    pub own: bool,
}

/// What a service's API listed, kept.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Listed {
    /// When (seconds since 1970).
    pub at: u64,
    pub transcribe: Option<Vec<ModelOption>>,
    pub speak: Option<Vec<ModelOption>>,
    pub voices: Option<Vec<VoiceOption>>,
}

/// What Réglages › Voix shows of a service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceCatalog {
    pub service: String,
    pub transcribe: Vec<ModelOption>,
    pub speak: Vec<ModelOption>,
    pub default_transcribe: String,
    pub default_speak: String,
    /// Its voices, when its API lists them; none: a free field.
    pub voices: Option<Vec<VoiceOption>>,
    /// The voice it reads with when none is chosen, by `<model>|<lang>`; a language missing: the model does not
    /// speak it (Groq's Orpheus, in French).
    pub default_voices: BTreeMap<String, String>,
    /// The voices depend on the model (Deepgram): the one chosen is kept by model.
    pub voices_by_model: bool,
    /// What a voice looks like for this service, and where its voices are shown.
    pub voice_example: String,
    pub voices_url: String,
    /// The pace it takes, from the settings' (0.5 to 2): its bounds; none: it reads at its own pace.
    pub speed: Option<[f32; 2]>,
    /// Whether the models are its API's (else the documentation's, shipped), and when they were read.
    pub listed: bool,
    pub listed_at: u64,
    /// Why they could not be read, the last time.
    pub error: Option<String>,
}

impl ServiceCatalog {
    /// Nothing listed: the catalog of a service whose plugin is gone.
    pub fn empty(service: &str) -> Self {
        Self {
            service: service.into(),
            transcribe: Vec::new(),
            speak: Vec::new(),
            default_transcribe: String::new(),
            default_speak: String::new(),
            voices: None,
            default_voices: BTreeMap::new(),
            voices_by_model: false,
            voice_example: String::new(),
            voices_url: String::new(),
            speed: None,
            listed: false,
            listed_at: 0,
            error: None,
        }
    }
}

fn opt(id: &str, label: &str, note: String) -> ModelOption {
    ModelOption {
        id: id.into(),
        label: label.into(),
        note,
        langs: Vec::new(),
    }
}

/// The models its documentation names today, for `purpose`.
pub fn shipped_models(p: Provider, purpose: Purpose) -> Vec<ModelOption> {
    let fast = || tr!("rapide", "fast");
    match (p, purpose) {
        (Provider::OpenAi, Purpose::Transcribe) => vec![
            opt("gpt-4o-mini-transcribe", "GPT-4o mini Transcribe", fast()),
            opt(
                "gpt-4o-transcribe",
                "GPT-4o Transcribe",
                tr!("plus précis", "more accurate"),
            ),
            opt("whisper-1", "Whisper", String::new()),
        ],
        (Provider::Groq, Purpose::Transcribe) => vec![
            opt("whisper-large-v3-turbo", "Whisper Large v3 Turbo", fast()),
            opt(
                "whisper-large-v3",
                "Whisper Large v3",
                tr!("plus précis", "more accurate"),
            ),
        ],
        (Provider::Mistral, Purpose::Transcribe) => {
            vec![opt("voxtral-mini-latest", "Voxtral Mini", String::new())]
        }
        (Provider::ElevenLabs, Purpose::Transcribe) => vec![
            opt("scribe_v2", "Scribe v2", tr!("90 langues", "90 languages")),
            opt(
                "scribe_v2_medical",
                "Scribe v2 Medical",
                tr!("vocabulaire médical", "medical vocabulary"),
            ),
        ],
        (Provider::Deepgram, Purpose::Transcribe) => vec![
            opt("nova-3", "Nova-3", String::new()),
            opt("nova-2", "Nova-2", String::new()),
        ],
        (Provider::OpenAi, Purpose::Speak) => vec![
            opt(
                "gpt-4o-mini-tts",
                "GPT-4o mini TTS",
                tr!("le plus naturel", "the most natural"),
            ),
            opt("tts-1", "TTS-1", fast()),
            opt("tts-1-hd", "TTS-1 HD", tr!("qualité", "quality")),
        ],
        (Provider::Groq, Purpose::Speak) => vec![ModelOption {
            langs: vec!["en".into()],
            ..opt(
                "canopylabs/orpheus-v1-english",
                "Orpheus English",
                tr!("anglais seulement", "English only"),
            )
        }],
        (Provider::Mistral, Purpose::Speak) => {
            vec![opt(
                "voxtral-mini-tts-2603",
                "Voxtral Mini TTS",
                String::new(),
            )]
        }
        (Provider::ElevenLabs, Purpose::Speak) => vec![
            opt(
                "eleven_flash_v2_5",
                "Eleven Flash v2.5",
                tr!("le plus rapide, 32 langues", "the fastest, 32 languages"),
            ),
            opt(
                "eleven_v4_turbo",
                "Eleven v4 Turbo",
                tr!("expressif, en temps réel", "expressive, real time"),
            ),
            opt(
                "eleven_v4",
                "Eleven v4",
                tr!(
                    "le plus expressif, plus lent",
                    "the most expressive, slower"
                ),
            ),
            opt(
                "eleven_v3_conversational",
                "Eleven v3 Conversational",
                tr!("expressif, en temps réel", "expressive, real time"),
            ),
            opt("eleven_v3", "Eleven v3", tr!("expressif", "expressive")),
            opt(
                "eleven_multilingual_v2",
                "Eleven Multilingual v2",
                tr!("stable, plus lent", "steady, slower"),
            ),
        ],
        _ => vec![
            opt("aura-2", "Aura-2", String::new()),
            ModelOption {
                langs: vec!["en".into()],
                ..opt("aura", "Aura", tr!("anglais seulement", "English only"))
            },
        ],
    }
}

/// The model it reads with when none is chosen.
pub fn default_speak_model(p: Provider) -> &'static str {
    match p {
        Provider::OpenAi => "gpt-4o-mini-tts",
        Provider::Groq => "canopylabs/orpheus-v1-english",
        Provider::Mistral => "voxtral-mini-tts-2603",
        Provider::ElevenLabs => "eleven_flash_v2_5",
        _ => "aura-2",
    }
}

pub fn default_model(p: Provider, purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::Transcribe => p.model(),
        Purpose::Speak => default_speak_model(p),
    }
}

/// The voice its documentation gives for `model` in `lang`; none: that model does not speak it.
pub fn default_voice(p: Provider, model: &str, lang: &str) -> Option<&'static str> {
    let fr = lang == "fr";
    match p {
        // `marin` is not one of TTS-1's voices.
        Provider::OpenAi if model.starts_with("tts-1") => Some("alloy"),
        Provider::OpenAi => Some("marin"),
        Provider::Groq => (!fr).then_some("hannah"),
        Provider::Mistral => Some(if fr {
            "fr_marie_neutral"
        } else {
            "en_paul_neutral"
        }),
        Provider::ElevenLabs if model.ends_with("_v2") && !model.contains("multilingual") => {
            (!fr).then_some("JBFqnCBsd6RMkjVDRZzb")
        }
        Provider::ElevenLabs => Some("JBFqnCBsd6RMkjVDRZzb"),
        Provider::Deepgram if model == "aura" => (!fr).then_some("aura-asteria-en"),
        _ => Some(if fr {
            "aura-2-agathe-fr"
        } else {
            "aura-2-thalia-en"
        }),
    }
}

/// The bounds of the pace it takes; none: it reads at its own pace.
pub fn speed_range(p: Provider) -> Option<[f32; 2]> {
    match p {
        Provider::OpenAi => Some([0.25, 4.0]),
        Provider::ElevenLabs => Some([0.7, 1.2]),
        Provider::Deepgram => Some([0.7, 1.5]),
        _ => None,
    }
}

fn voice_help(p: Provider) -> (&'static str, &'static str) {
    match p {
        Provider::OpenAi => (
            "marin, cedar, coral, alloy…",
            "https://platform.openai.com/docs/guides/text-to-speech#voice-options",
        ),
        Provider::Groq => (
            "hannah, troy, austin, diana…",
            "https://console.groq.com/docs/text-to-speech/orpheus",
        ),
        Provider::Mistral => (
            "fr_marie_neutral",
            "https://console.mistral.ai/build/audio/text-to-speech",
        ),
        Provider::ElevenLabs => (
            "JBFqnCBsd6RMkjVDRZzb",
            "https://elevenlabs.io/app/voice-library",
        ),
        _ => (
            "aura-2-agathe-fr",
            "https://developers.deepgram.com/docs/tts-models",
        ),
    }
}

/// Whether its API lists its voices (else a free field).
pub fn lists_voices(p: Provider) -> bool {
    matches!(
        p,
        Provider::ElevenLabs | Provider::Mistral | Provider::Deepgram
    )
}

/// The catalog of `p`: what was listed, else what is shipped.
pub fn catalog(p: Provider, listed: Option<&Listed>, error: Option<String>) -> ServiceCatalog {
    let transcribe = listed
        .and_then(|l| l.transcribe.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| shipped_models(p, Purpose::Transcribe));
    let speak = listed
        .and_then(|l| l.speak.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| shipped_models(p, Purpose::Speak));
    let mut default_voices = BTreeMap::new();
    for m in &speak {
        for lang in ["fr", "en"] {
            let speaks = m.langs.is_empty() || m.langs.iter().any(|l| l == lang);
            if let Some(v) = default_voice(p, &m.id, lang).filter(|_| speaks) {
                default_voices.insert(format!("{}|{lang}", m.id), v.to_string());
            }
        }
    }
    let (example, url) = voice_help(p);
    ServiceCatalog {
        service: p.id().into(),
        default_transcribe: p.model().into(),
        default_speak: default_speak_model(p).into(),
        transcribe,
        speak,
        voices: listed
            .and_then(|l| l.voices.clone())
            .filter(|_| lists_voices(p)),
        default_voices,
        voices_by_model: p == Provider::Deepgram,
        voice_example: example.into(),
        voices_url: url.into(),
        speed: speed_range(p),
        listed: listed.is_some_and(|l| l.transcribe.is_some() || l.speak.is_some()),
        listed_at: listed.map(|l| l.at).unwrap_or(0),
        error,
    }
}

/// The ISO 639-1 code of a service's language (`fr`, `fr-FR`, `fr_FR`, `french`).
fn lang_code(l: &str) -> String {
    let l = l.trim().to_lowercase();
    match l.as_str() {
        "french" | "français" => "fr".into(),
        "english" => "en".into(),
        _ => l
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .chars()
            .take(3)
            .collect(),
    }
}

fn str_of<'a>(v: &'a serde_json::Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
}

/// The models of an OpenAI-like `/models` answer (`data[].id`) that `keep` keeps.
fn ids_of(body: &serde_json::Value, keep: impl Fn(&str) -> bool) -> Vec<String> {
    let mut ids: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .into_iter()
        .flatten()
        .filter_map(|m| m.get("id").and_then(|i| i.as_str()))
        .filter(|id| keep(id))
        .map(String::from)
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// `ids` as options: a model the app ships keeps its name and note.
fn named(p: Provider, purpose: Purpose, ids: Vec<String>) -> Vec<ModelOption> {
    let shipped = shipped_models(p, purpose);
    let mut out: Vec<ModelOption> = ids
        .into_iter()
        .map(|id| {
            shipped
                .iter()
                .find(|s| s.id == id)
                .cloned()
                .unwrap_or_else(|| opt(&id, &id, String::new()))
        })
        .collect();
    // The default first, then the shipped ones in their order, then the others.
    let rank = |m: &ModelOption| {
        shipped
            .iter()
            .position(|s| s.id == m.id)
            .unwrap_or(usize::MAX)
    };
    out.sort_by_key(rank);
    out
}

/// What `GET <path>` gives, as JSON; its refusals told as the dictation tells them.
async fn get_json(
    host: &Arc<Host>,
    spec: &Spec,
    path: &str,
    pairs: &[(&str, &str)],
) -> Result<serde_json::Value, OnlineError> {
    let p = spec.provider;
    let base = spec.base.trim_end_matches('/');
    let url = match pairs {
        [] => format!("{base}{path}"),
        _ => format!("{base}{path}?{}", query(pairs)),
    };
    let (name, value) = key_header(spec);
    let call = Call::get(url).header(name, value);
    let (status, body) = crate::serve::fetch_text(host, &call, spec).await?;
    match status {
        401 | 403 => return Err(OnlineError::KeyRefused(p)),
        429 => return Err(OnlineError::Quota(p, said(&body, &spec.key))),
        s if !(200..300).contains(&s) => {
            return Err(OnlineError::Service(p, s, said(&body, &spec.key)))
        }
        _ => {}
    }
    serde_json::from_str(&body).map_err(|_| OnlineError::Service(p, status, said(&body, &spec.key)))
}

/// The models of `p` its API lists, for both purposes; a purpose it does not list: none.
async fn list_models(
    host: &Arc<Host>,
    spec: &Spec,
) -> Result<(Option<Vec<ModelOption>>, Option<Vec<ModelOption>>), OnlineError> {
    let p = spec.provider;
    let both = |ids_t: Vec<String>, ids_s: Vec<String>| {
        (
            Some(named(p, Purpose::Transcribe, ids_t)),
            Some(named(p, Purpose::Speak, ids_s)),
        )
    };
    Ok(match p {
        Provider::OpenAi => {
            let b = get_json(host, spec, "/v1/models", &[]).await?;
            both(
                ids_of(&b, |id| {
                    (id.contains("transcribe") && !id.contains("diarize"))
                        || id.starts_with("whisper")
                }),
                ids_of(&b, |id| id.contains("tts") && !id.contains("realtime")),
            )
        }
        Provider::Groq => {
            let b = get_json(host, spec, "/v1/models", &[]).await?;
            both(
                ids_of(&b, |id| id.starts_with("whisper")),
                ids_of(&b, |id| {
                    (id.contains("orpheus") || id.contains("tts"))
                        && !id.contains("arabic")
                        && !id.contains("playai")
                }),
            )
        }
        Provider::Mistral => {
            let b = get_json(host, spec, "/v1/models", &[]).await?;
            both(
                ids_of(&b, |id| {
                    id.contains("voxtral")
                        && id.contains("mini")
                        && !id.contains("tts")
                        && !id.contains("realtime")
                }),
                ids_of(&b, |id| id.contains("voxtral") && id.contains("tts")),
            )
        }
        Provider::ElevenLabs => {
            let b = get_json(host, spec, "/v1/models", &[]).await?;
            let speak = b
                .as_array()
                .into_iter()
                .flatten()
                .filter(|m| m.get("can_do_text_to_speech").and_then(|x| x.as_bool()) == Some(true))
                .filter_map(|m| {
                    let id = str_of(m, "/model_id")?;
                    let shipped = shipped_models(p, Purpose::Speak);
                    let known = shipped.iter().find(|s| s.id == id);
                    let langs: Vec<String> = m
                        .get("languages")
                        .and_then(|l| l.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|l| str_of(l, "/language_id"))
                        .map(lang_code)
                        .collect();
                    let note = match known {
                        Some(k) => k.note.clone(),
                        None if !langs.is_empty() => {
                            let n = langs.len();
                            tr!("{n} langues", "{n} languages")
                        }
                        None => String::new(),
                    };
                    Some(ModelOption {
                        id: id.into(),
                        label: str_of(m, "/name").unwrap_or(id).into(),
                        note,
                        langs,
                    })
                })
                .collect::<Vec<_>>();
            let mut speak = speak;
            let shipped = shipped_models(p, Purpose::Speak);
            speak.sort_by_key(|m| {
                shipped
                    .iter()
                    .position(|s| s.id == m.id)
                    .unwrap_or(usize::MAX)
            });
            // Its API lists no transcription model: the documentation's.
            (None, Some(speak))
        }
        _ => {
            let b = get_json(host, spec, "/v1/models", &[]).await?;
            let arch = |list: &str, batch_only: bool| {
                let mut ids: Vec<String> = b
                    .get(list)
                    .and_then(|l| l.as_array())
                    .into_iter()
                    .flatten()
                    .filter(|m| {
                        !batch_only || m.get("batch").and_then(|x| x.as_bool()) != Some(false)
                    })
                    .filter_map(|m| str_of(m, "/architecture").map(String::from))
                    .collect();
                ids.sort();
                ids.dedup();
                ids
            };
            both(arch("stt", true), arch("tts", false))
        }
    })
}

/// The voices of `p` its API lists, the user's own among them; none for a service whose API lists none.
async fn list_voices(
    host: &Arc<Host>,
    spec: &Spec,
) -> Result<Option<Vec<VoiceOption>>, OnlineError> {
    let p = spec.provider;
    let join = |parts: Vec<Option<&str>>| {
        parts
            .into_iter()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    };
    Ok(match p {
        Provider::ElevenLabs => {
            let mut out = Vec::new();
            let mut token: Option<String> = None;
            // 100 a page; 10 pages at most.
            for _ in 0..10 {
                let mut q = vec![("page_size", "100")];
                if let Some(t) = &token {
                    q.push(("next_page_token", t.as_str()));
                }
                let b = get_json(host, spec, "/v2/voices", &q).await?;
                for v in b
                    .get("voices")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                {
                    let Some(id) = str_of(v, "/voice_id") else {
                        continue;
                    };
                    let mut langs: Vec<String> = v
                        .get("verified_languages")
                        .and_then(|l| l.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|l| str_of(l, "/language"))
                        .map(lang_code)
                        .collect();
                    langs.sort();
                    langs.dedup();
                    let category = str_of(v, "/category").unwrap_or("");
                    out.push(VoiceOption {
                        id: id.into(),
                        name: str_of(v, "/name").unwrap_or(id).into(),
                        langs,
                        model: String::new(),
                        note: join(vec![
                            str_of(v, "/labels/gender"),
                            str_of(v, "/labels/accent"),
                            str_of(v, "/labels/description")
                                .or_else(|| str_of(v, "/labels/descriptive")),
                        ]),
                        own: !matches!(category, "premade" | ""),
                    });
                }
                token = str_of(&b, "/next_page_token").map(String::from);
                let more = b.get("has_more").and_then(|x| x.as_bool()) == Some(true);
                if !more || token.is_none() {
                    break;
                }
            }
            Some(out)
        }
        Provider::Mistral => {
            let b = get_json(host, spec, "/v1/audio/voices", &[("limit", "100")]).await?;
            let list = b
                .get("items")
                .or_else(|| b.get("data"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            Some(
                list.iter()
                    .filter_map(|v| {
                        let id = str_of(v, "/slug").or_else(|| str_of(v, "/id"))?;
                        let mut langs: Vec<String> = v
                            .get("languages")
                            .and_then(|l| l.as_array())
                            .into_iter()
                            .flatten()
                            .filter_map(|l| l.as_str())
                            .map(lang_code)
                            .collect();
                        if langs.is_empty() {
                            // A preset's slug starts with its language: fr_marie_neutral.
                            if let Some((l, _)) = id.split_once('_').filter(|(l, _)| l.len() == 2) {
                                langs.push(l.into());
                            }
                        }
                        langs.dedup();
                        Some(VoiceOption {
                            id: id.into(),
                            name: str_of(v, "/name").unwrap_or(id).into(),
                            langs,
                            model: String::new(),
                            note: join(vec![str_of(v, "/gender"), str_of(v, "/description")]),
                            own: str_of(v, "/type") == Some("custom")
                                || v.get("user_id").is_some_and(|u| !u.is_null()),
                        })
                    })
                    .collect(),
            )
        }
        Provider::Deepgram => {
            let b = get_json(host, spec, "/v1/models", &[]).await?;
            Some(
                b.get("tts")
                    .and_then(|l| l.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|v| {
                        let id = str_of(v, "/canonical_name")?;
                        let mut langs: Vec<String> = v
                            .get("languages")
                            .and_then(|l| l.as_array())
                            .into_iter()
                            .flatten()
                            .filter_map(|l| l.as_str())
                            .map(lang_code)
                            .collect();
                        langs.sort();
                        langs.dedup();
                        let name = str_of(v, "/name").unwrap_or(id);
                        let mut c = name.chars();
                        let name = match c.next() {
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                            None => id.to_string(),
                        };
                        let tags =
                            v.pointer("/metadata/tags")
                                .and_then(|t| t.as_array())
                                .map(|t| {
                                    t.iter()
                                        .filter_map(|x| x.as_str())
                                        .take(3)
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                });
                        Some(VoiceOption {
                            id: id.into(),
                            name,
                            langs,
                            model: str_of(v, "/architecture").unwrap_or_default().into(),
                            note: join(vec![str_of(v, "/metadata/accent"), tags.as_deref()]),
                            own: false,
                        })
                    })
                    .collect(),
            )
        }
        _ => None,
    })
}

/// Reads again what the API of `spec`'s service lists. Its models failing fails it; its voices failing (a key that
/// may not read them) leaves them out: the field is then free.
pub async fn fetch(host: &Arc<Host>, spec: &Spec) -> Result<Listed, OnlineError> {
    let (transcribe, speak) = list_models(host, spec).await?;
    let voices = match list_voices(host, spec).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "voice: the voices of {} not listed: {}",
                spec.provider.label(),
                e.message(escouade_plugin_kit::i18n::Lang::En)
            );
            None
        }
    };
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(Listed {
        at,
        transcribe,
        speak,
        voices,
    })
}
