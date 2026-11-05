//! A reactor on serde: it answers `initialize` and `shutdown`, refuses the rest, and `probe/panic` panics with its place
//! in the sources, which is what `rustc-canon` keeps the same from a system to the other.

mod wire;

use std::io::{Read, Write};
use wire::{Answer, Call};

/// The host's turn: the lines that came, their answers, and the end of the turn.
#[export_name = "escouade-turn"]
pub extern "C" fn turn() {
    let mut text = String::new();
    let _ = std::io::stdin().lock().read_to_string(&mut text);
    let mut out = String::new();
    for line in text.lines() {
        let Ok(call) = serde_json::from_str::<Call>(line) else {
            continue;
        };
        if let Some(answer) = Answer::to(&call) {
            out.push_str(&serde_json::to_string(&answer).expect("an answer"));
            out.push('\n');
        }
    }
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}
