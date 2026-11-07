//! What the app and a voice service's plugin say to each other (INT-13, ESC-285; `docs/PLUGINS.md` §1.4 and §4.13):
//! the methods, the shape of a `Spec` on the wire, and an `OnlineError` as a JSON-RPC refusal.
//!
//! The app calls, the plugin answers (`voice/*`); the plugin tells the app what comes while it works with notifications
//! (`voice/partial`: the text understood so far, `voice/audio`: a piece of sound). The sound crosses as base64 of
//! 32-bit floats (the stream) or of 16-bit samples (the dictation, whose ten minutes must fit a module, and the synthesis).

use crate::online::{OnlineError, Provider, Spec};
use base64::Engine;
use serde_json::{json, Value};

pub const DESCRIBE: &str = "voice/describe";
pub const TRANSCRIBE: &str = "voice/transcribe";
pub const LIVE_OPEN: &str = "voice/live/open";
pub const LIVE_AUDIO: &str = "voice/live/audio";
pub const LIVE_FINISH: &str = "voice/live/finish";
pub const LIVE_CLOSE: &str = "voice/live/close";
pub const SPEAK: &str = "voice/speak";
pub const CATALOG: &str = "voice/catalog";
pub const LIST: &str = "voice/list";
pub const CANCEL: &str = "voice/cancel";
/// Plugin to app: the text understood so far, `{ stream, text }`.
pub const PARTIAL: &str = "voice/partial";
/// Plugin to app: a piece of the sound being read, `{ call, rate, pcm }`.
pub const AUDIO: &str = "voice/audio";

/// The refusals of a service, as JSON-RPC codes (a plugin's own range).
const NO_KEY: i64 = -32101;
const KEY_REFUSED: i64 = -32102;
const NETWORK: i64 = -32103;
const SERVICE: i64 = -32104;
const QUOTA: i64 = -32105;
const SPEECH: i64 = -32106;

pub fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn unb64(text: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD.decode(text).ok()
}

/// Sound as 32-bit floats, little-endian.
pub fn floats_to_b64(samples: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(samples.len() * 4);
    for s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    b64(&bytes)
}

pub fn floats_from_b64(text: &str) -> Option<Vec<f32>> {
    let bytes = unb64(text)?;
    Some(
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect(),
    )
}

/// Sound as 16-bit samples, little-endian.
pub fn pcm16_to_b64(samples: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    b64(&bytes)
}

pub fn pcm16_from_b64(text: &str) -> Option<Vec<f32>> {
    let bytes = unb64(text)?;
    Some(
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c) as f32 / i16::MAX as f32)
            .collect(),
    )
}

/// What the plugin is told of a `Spec`: its key, where the service is, the model and the model of the stream. The key goes
/// to the plugin of this service alone. The network settings are not said: the host of the plugin follows them.
pub fn spec_json(spec: &Spec) -> Value {
    json!({
        "model": spec.model,
        "key": spec.key,
        "base": spec.base,
        "live": spec.live,
    })
}

/// The `Spec` of what the app said, for the service `provider`. `base` empty: the service's own address.
pub fn spec_from(provider: Provider, v: &Value) -> Spec {
    let text = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let mut spec = Spec::new(provider, text("key"));
    if !text("base").is_empty() {
        spec.base = text("base");
    }
    if !text("model").is_empty() {
        spec.model = text("model");
    }
    spec.live = v
        .get("live")
        .and_then(Value::as_str)
        .filter(|m| !m.is_empty())
        .map(String::from);
    spec
}

/// An error of the service as a refusal: its code and its message (the status in front, for those that carry one).
pub fn refusal_of(e: &OnlineError) -> (i64, String) {
    match e {
        OnlineError::NoKey(_) => (NO_KEY, String::new()),
        OnlineError::KeyRefused(_) => (KEY_REFUSED, String::new()),
        OnlineError::Network(_, m) => (NETWORK, m.clone()),
        OnlineError::Service(_, s, m) => (SERVICE, format!("{s}|{m}")),
        OnlineError::Quota(_, m) => (QUOTA, m.clone()),
        OnlineError::Speech(_, s, m) => (SPEECH, format!("{s}|{m}")),
    }
}

/// The error a refusal of the plugin's service says, for `provider`; none when the code is no service's.
pub fn error_of(provider: Provider, code: i64, message: &str) -> Option<OnlineError> {
    let status = |m: &str| -> (u16, String) {
        match m.split_once('|') {
            Some((s, rest)) => (s.parse().unwrap_or(0), rest.to_string()),
            None => (0, m.to_string()),
        }
    };
    Some(match code {
        NO_KEY => OnlineError::NoKey(provider),
        KEY_REFUSED => OnlineError::KeyRefused(provider),
        NETWORK => OnlineError::Network(provider, message.into()),
        SERVICE => {
            let (s, m) = status(message);
            OnlineError::Service(provider, s, m)
        }
        QUOTA => OnlineError::Quota(provider, message.into()),
        SPEECH => {
            let (s, m) = status(message);
            OnlineError::Speech(provider, s, m)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_crosses_as_it_left() {
        let s = [0.0f32, 0.5, -0.25, 1.0];
        assert_eq!(floats_from_b64(&floats_to_b64(&s)).unwrap(), s);
        let back = pcm16_from_b64(&pcm16_to_b64(&s)).unwrap();
        assert!(s.iter().zip(&back).all(|(a, b)| (a - b).abs() < 0.001));
    }

    #[test]
    fn every_error_of_a_service_crosses_as_a_refusal_and_comes_back() {
        let p = Provider::ElevenLabs;
        for e in [
            OnlineError::NoKey(p),
            OnlineError::KeyRefused(p),
            OnlineError::Network(p, "cut".into()),
            OnlineError::Service(p, 500, "boom | x".into()),
            OnlineError::Quota(p, "slow down".into()),
            OnlineError::Speech(p, 422, "no".into()),
        ] {
            let (code, message) = refusal_of(&e);
            assert_eq!(error_of(p, code, &message), Some(e));
        }
        assert_eq!(error_of(p, -32601, "x"), None);
    }
}
