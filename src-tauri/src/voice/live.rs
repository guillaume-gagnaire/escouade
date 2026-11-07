//! The transcription as one speaks (ESC-136), as the app sees it: « Transcription en temps réel », for a service that
//! offers it. A stream is opened as a sentence starts, its sound goes while it is said, the text understood so far comes
//! back as it goes (`Partial`), and the sentence's end asks for its final text. What it says to its service, by a WebSocket
//! with the user's key, is its plugin's (`escouade_voice::live`, docs/PLUGINS.md §8.4.4).
//!
//! One stream by sentence (by dictation). Opening it takes the time of the first syllables, while what was heard waits in
//! the stream's queue: the sentence does not wait for it. A stream that cannot open, or breaks, says why (`OnlineError`):
//! the caller then sends the sentence whole, by a request, as before.

use super::online::{OnlineError, Provider};
use std::time::Duration;

pub use escouade_voice::live::FINAL_WAIT;

/// Told the text understood so far, the whole of it, as it changes.
pub type Partial = Box<dyn Fn(String) + Send + 'static>;

pub(super) enum Cmd {
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
    /// A stream that a plugin serves (INT-13): its commands go to `tx`, whose end answers on `result`.
    pub(super) fn bridged(
        provider: Provider,
        tx: tokio::sync::mpsc::UnboundedSender<Cmd>,
        result: std::sync::mpsc::Receiver<Result<Final, OnlineError>>,
    ) -> LiveStream {
        LiveStream {
            provider,
            tx,
            result,
        }
    }

    /// A stream no one serves: its end says `why`.
    pub(super) fn failed(provider: Provider, why: OnlineError) -> LiveStream {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let (done, result) = std::sync::mpsc::channel();
        let _ = done.send(Err(why));
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

#[cfg(test)]
#[path = "live_tests.rs"]
pub(crate) mod tests;
