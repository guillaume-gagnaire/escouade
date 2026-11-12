//! The voice of Escouade on the machine, cut out of the app's crate for it to compile faster (docs/CRATES.md): the
//! microphone (`source`, cpal), the sound brought to what Whisper takes (`audio`), the dictation (`transcribe`,
//! whisper.cpp), the end of a sentence heard (`vad`), the reading (`speech`: Piper's voices, eSpeak NG, ONNX Runtime)
//! and the download of their models (`models`). The app's `voice` module drives them and re-exports them under its
//! own name.

#[macro_use]
extern crate escouade_plugin_cli;

pub mod audio;
pub mod models;
pub mod source;
pub mod speech;
pub mod transcribe;
pub mod vad;

use escouade_plugin_cli::i18n::Lang;

/// ONNX Runtime, as this crate links it: the app's vectors (`escouade-vectors`) run their model with it.
pub use ort;

/// whisper.cpp's logs, handed to the `log` crate (the app filters them).
pub fn install_logging_hooks() {
    whisper_rs::install_logging_hooks();
}

/// The kit's language, for the messages of a service on line.
fn kit(lang: Lang) -> escouade_plugin_kit::i18n::Lang {
    match lang {
        Lang::Fr => escouade_plugin_kit::i18n::Lang::Fr,
        Lang::En => escouade_plugin_kit::i18n::Lang::En,
    }
}

/// What went wrong, as the window tells it: each with what to do about it.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceError {
    /// No microphone at all (or the one chosen and the system's both gone).
    NoDevice,
    /// The system keeps the microphone from the app.
    Denied,
    /// Another app holds it.
    Busy,
    /// No sound system answers (Linux: neither PulseAudio, PipeWire nor ALSA).
    NoAudioSystem,
    /// It listened, but not one sample was not zero: what macOS gives an app it keeps the
    /// microphone from (it refuses nothing), and a muted one.
    Silent,
    /// Already listening.
    AlreadyListening,
    /// The service on line did not transcribe it (ESC-110).
    Online(escouade_voice::online::OnlineError),
    Other(String),
}

impl VoiceError {
    /// What the window says, in `lang`, on `os` ("windows", "macos", "linux").
    pub fn message(&self, lang: Lang, os: &str) -> String {
        match self {
            VoiceError::NoDevice => tr_in!(
                lang,
                "Aucun micro trouvé. Branche un micro, ou choisis-en un dans Réglages › Voix.",
                "No microphone found. Plug one in, or pick one in Settings › Voice."
            ),
            VoiceError::Denied | VoiceError::Silent => {
                let steps = match (os, lang) {
                    ("macos", Lang::Fr) => "ouvre Réglages Système › Confidentialité et sécurité › Microphone, coche Escouade, puis relance l’app.",
                    ("macos", Lang::En) => "open System Settings › Privacy & Security › Microphone, tick Escouade, then restart the app.",
                    ("windows", Lang::Fr) => "ouvre Paramètres › Confidentialité et sécurité › Microphone, et allume « Accès au microphone » puis « Autoriser les applications de bureau à accéder à votre microphone ».",
                    ("windows", Lang::En) => "open Settings › Privacy & security › Microphone, and turn on “Microphone access”, then “Let desktop apps access your microphone”.",
                    (_, Lang::Fr) => "vérifie que PulseAudio ou PipeWire tourne, que le micro n’est pas coupé (pavucontrol, ou les réglages du son), et que ton compte peut s’en servir (groupe audio).",
                    (_, Lang::En) => "check that PulseAudio or PipeWire is running, that the microphone is not muted (pavucontrol, or the sound settings), and that your account may use it (audio group).",
                };
                if *self == VoiceError::Silent {
                    tr_in!(lang, "Aucun son n’est arrivé du micro : il est coupé, ou le système le refuse à Escouade. Pour l’autoriser, {steps}", "No sound came from the microphone: it is muted, or the system keeps it from Escouade. To allow it, {steps}")
                } else {
                    tr_in!(lang, "Le système refuse le micro à Escouade. Pour l’autoriser, {steps}", "The system keeps the microphone from Escouade. To allow it, {steps}")
                }
            }
            VoiceError::Busy => tr_in!(
                lang,
                "Le micro est pris par une autre application. Ferme-la, ou choisis un autre micro dans Réglages › Voix.",
                "The microphone is taken by another app. Close it, or pick another microphone in Settings › Voice."
            ),
            VoiceError::NoAudioSystem => tr_in!(
                lang,
                "Aucun système de son ne répond. Démarre PulseAudio ou PipeWire (ou vérifie ALSA), puis réessaie.",
                "No sound system answers. Start PulseAudio or PipeWire (or check ALSA), then try again."
            ),
            VoiceError::AlreadyListening => tr_in!(lang, "Une dictée est déjà en cours.", "A dictation is already under way."),
            VoiceError::Online(e) => e.message(kit(lang)),
            VoiceError::Other(e) => tr_in!(lang, "La dictée a échoué : {e}", "Dictation failed: {e}"),
        }
    }
}

impl std::fmt::Display for VoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message(Lang::En, std::env::consts::OS))
    }
}

#[cfg(test)]
pub(crate) use escouade_base::testing::test_dir;

/// The sounds of the tests, kept beside the app's tests of the voice that play them too.
#[cfg(test)]
pub(crate) fn testdata(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src-tauri/src/voice/testdata")
        .join(name)
}
