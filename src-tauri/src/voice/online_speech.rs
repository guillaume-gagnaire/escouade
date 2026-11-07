//! The conversation's voice made by a service on line (ESC-132), as the app sees it: which service and which voice the
//! settings choose, and a passage cut in sentences. What goes to the service is its plugin's (`escouade_voice::speech`).

use super::online::{OnlineError, Spec};
use super::speech::Audio;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub use escouade_voice::speech::{refuses_the_voice, voice_key, SpeakChoice};

/// `text` cut as Piper cuts it (its sentences), a sentence longer than `max` cut again at its commas, else its spaces.
pub fn sentences(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    for s in super::speech::piper::sentences(text) {
        let mut cur = String::new();
        for (clause, mark) in s {
            let piece = format!("{clause}{mark}");
            if !cur.is_empty() && cur.chars().count() + 1 + piece.chars().count() > max {
                out.push(std::mem::take(&mut cur));
            }
            if piece.chars().count() > max {
                for word in piece.split_whitespace() {
                    if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > max {
                        out.push(std::mem::take(&mut cur));
                    }
                    if !cur.is_empty() {
                        cur.push(' ');
                    }
                    cur.extend(word.chars().take(max));
                }
                continue;
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(&piece);
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out
}

/// A reading on line that blocks: on a runtime of its own (the conversation's threads are not async).
pub struct Blocking {
    rt: tokio::runtime::Runtime,
}

impl Blocking {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        })
    }

    pub fn speak(
        &self,
        spec: &Spec,
        choice: &SpeakChoice,
        sentence: &str,
        cancel: &AtomicBool,
        on: &mut dyn FnMut(Audio),
    ) -> Result<Option<Duration>, OnlineError> {
        self.rt
            .block_on(super::service::of(spec.provider).speak(spec, choice, sentence, cancel, on))
    }
}

/// What « Voix générée par » makes of the settings for a conversation in `lang`.
#[derive(Debug, Default)]
pub struct Resolved {
    /// The service reads.
    pub online: Option<super::conversation::OnlineVoice>,
    /// It was chosen but cannot be asked (no key).
    pub error: Option<OnlineError>,
    /// What to say as the conversation starts.
    pub notice: Option<String>,
}

pub fn resolve(
    settings: &crate::model::Settings,
    secrets: &dyn crate::integrations::secrets::SecretStore,
    catalog: &super::online_catalog::CatalogStore,
    lang: &str,
) -> Resolved {
    use super::online_catalog::Purpose;
    let sp = &settings.voice.speech;
    if sp.engine != "online" {
        return Resolved::default();
    }
    let Some(p) = super::online::find(&sp.online_service) else {
        return Resolved::default();
    };
    let chosen = sp
        .online_models
        .get(p.id())
        .map(String::as_str)
        .unwrap_or("");
    let (model, mut notice) = catalog.checked_model(p, Purpose::Speak, chosen);
    let listed = catalog.get(p);
    let known = super::online_catalog::catalog(p, listed.as_ref(), None);
    let Some(default_voice) = known
        .default_voices
        .get(&format!("{model}|{lang}"))
        .cloned()
    else {
        let label = p.label();
        let language = if lang == "fr" {
            tr!("le français", "French")
        } else {
            tr!("l’anglais", "English")
        };
        log::info!("speech: {label} ({model}) does not speak {lang}: Piper reads");
        return Resolved {
            notice: Some(tr!(
                "{label} ({model}) ne lit pas {language} : la voix locale (Piper) lit la conversation.",
                "{label} ({model}) does not read {language}: the local voice (Piper) reads the conversation."
            )),
            ..Default::default()
        };
    };
    let voice = sp
        .online_voices
        .get(&voice_key(p, &model, lang))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default_voice.clone());
    let rate = if sp.rate.is_finite() {
        sp.rate.clamp(0.5, 2.0)
    } else {
        1.0
    };
    match super::online::keyed(p, settings, secrets) {
        Ok(spec) => Resolved {
            online: Some(super::conversation::OnlineVoice {
                spec: spec.with_model(&model),
                choice: SpeakChoice {
                    model,
                    voice,
                    lang: lang.into(),
                    rate,
                },
                default_voice,
            }),
            error: None,
            notice: notice.take(),
        },
        Err(e) => Resolved {
            online: None,
            error: Some(match e {
                super::VoiceError::Online(e) => e,
                other => {
                    OnlineError::Network(p, other.message(crate::i18n::ui(), std::env::consts::OS))
                }
            }),
            notice,
        },
    }
}

#[cfg(test)]
#[path = "online_speech_tests.rs"]
mod tests;
