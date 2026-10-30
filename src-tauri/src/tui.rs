//! « Vue Claude Code »: an agent run in Claude Code's own interface (its TUI) in a terminal, in
//! place of the stream that Escouade's conversation view drives. A session has one process at a
//! time: switching stops the one and resumes the session in the other (`--resume`).
//!
//! The window still knows what the agent does, from two sources (docs/PROTOCOL.md, « Vue Claude
//! Code »):
//! - its hooks, given to it with `--settings` as HTTP hooks to a server of Escouade's on
//!   127.0.0.1, behind a token of its own (in its environment, never on its command line): it
//!   runs, it waits for the user, it stopped;
//! - its transcript, read as it is written (`agent::transcript`): the conversation, its turns and
//!   what they cost, the plan.
//!
//! A ticket's agent never runs there: its board drives it over the stream.

use crate::accounts;
use crate::agent::{AgentHandle, Effects, TuiRun};
use crate::core::Core;
pub use crate::engine::claude_terminal::find_transcript;
#[cfg(test)]
pub use crate::engine::claude_terminal::tui_args;
#[cfg(test)]
use crate::engine::claude_terminal::EVENTS;
use crate::engine::terminal::{HookServer, TerminalLaunch};
use crate::engine::TerminalView;
#[cfg(test)]
use crate::engine::{claude_terminal, Engine, Escouade};
use crate::handover::{self, Relay};
use crate::model::*;
use crate::pty::{PtyManager, TermInfo};
use anyhow::{anyhow, bail, Context, Result};
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use parking_lot::Mutex;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::{Read, Seek, SeekFrom};
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use tauri::Runtime;

/// The path of the hooks' endpoint.
const PATH: &str = "/hook";
/// The variable that gives an interface its token.
pub(crate) const TOKEN_VAR: &str = "ESCOUADE_HOOK_TOKEN";
/// The variable that gives `escouade-hook` the hooks' server's address.
pub(crate) const URL_VAR: &str = "ESCOUADE_HOOK_URL";
/// How often the transcript of a running interface is read.
const READ_EVERY: Duration = Duration::from_millis(400);
/// The most a hook may send (its tool's input, a prompt with a pasted file).
const MAX_BODY: usize = 4 << 20;
/// How long after it started an interface's end is taken for a failure of Remote Control (the
/// option it was given), if it was started with it and no window ever showed it.
const RC_GRACE: Duration = Duration::from_secs(10);
/// How long an interface started with Remote Control, once it is up (listed by the registry of
/// Claude Code's sessions), is waited for to show a sign of its link: without one it has none (an
/// account with no claude.ai subscription, a network, a proxy or a rule of the organization), and
/// Claude Code says nothing of it in its terminal.
#[cfg(not(test))]
const LINK_WAIT: Duration = Duration::from_secs(20);
#[cfg(test)]
const LINK_WAIT: Duration = Duration::from_millis(1500);
/// How long an interface a window showed, started with Remote Control, is looked for in the
/// registry to arm that wait: past it, the registry is read no more if it never listed it.
const REGISTRY_PATIENCE: Duration = Duration::from_secs(5 * 60);
/// How long the registry says an interface idle, since a turn started, before that turn ends as
/// interrupted when its transcript still did not end it (ms). Escape before the first answer:
/// Claude Code puts the message back in its prompt, writes nothing more in its transcript and
/// calls no hook (2.1.302, ESC-12). A turn that ends normally has its end read before.
const IDLE_ENDS_TURN_MS: i64 = 3000;

/// A turn cancelled before its answer, whose message Claude Code put back in its prompt
/// (`IDLE_ENDS_TURN_MS`): the next message pasted would be added to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cancelled {
    /// Its message is the one Escouade pasted, and nothing was typed in the terminal since: the
    /// prompt holds it alone.
    pub(crate) pasted: bool,
    /// How many keys the window had typed then (`PtyManager::typed`).
    pub(crate) typed: u64,
}

/// How long the Ctrl+C that empties the prompt of a cancelled message is given to be written,
/// and the pause after it, before the next message is pasted.
const EMPTIED_WITHIN: Duration = Duration::from_secs(2);
const AFTER_EMPTYING: Duration = Duration::from_millis(300);

/// The most files of the registry looked into, and the most one of them may weigh (bytes).
const REGISTRY_FILES: usize = 512;
const REGISTRY_FILE_MAX: u64 = 64 << 10;
/// How long the first Ctrl+C that quits an interface may take to be written before the pause that
/// precedes the second begins anyway.
const QUIT_WRITTEN_WITHIN: Duration = Duration::from_secs(2);
/// How far apart the two Ctrl+C that make an interface quit are sent.
const QUIT_GAP: Duration = Duration::from_millis(700);
/// How long an interface told to quit is given to end, before it is killed.
const QUIT_WAIT: Duration = Duration::from_secs(5);

/// The server the interfaces' hooks call: started with the first interface, until the app quits.
#[derive(Default)]
pub struct Hooks {
    port: Mutex<Option<u16>>,
    /// Each running interface's token, and its agent.
    tokens: Mutex<HashMap<String, String>>,
}

impl Hooks {
    pub(crate) fn grant(&self, agent_id: &str) -> String {
        let token = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        self.tokens
            .lock()
            .insert(token.clone(), agent_id.to_string());
        token
    }

    pub(crate) fn revoke(&self, agent_id: &str) {
        self.tokens.lock().retain(|_, a| a != agent_id);
    }

    fn agent_of(&self, token: &str) -> Option<String> {
        self.tokens.lock().get(token).cloned()
    }

    /// The agent has an interface's token.
    #[cfg(test)]
    pub(crate) fn holds(&self, agent_id: &str) -> bool {
        self.tokens.lock().values().any(|a| a == agent_id)
    }
}

/// The answer to the pseudo-console's cursor position query when no window gives one.
pub(crate) const CURSOR_HOME: &[u8] = b"\x1b[1;1R";
/// The size of the terminal of an interface started with no window.
const BACKGROUND_SIZE: (u16, u16) = (120, 40);
/// The most an interface's output keeps (bytes): its last screens, for a window that shows it
/// later.
const OUTPUT_KEPT: usize = 4 << 20;
/// The size of the pieces the kept output is sent to a window in.
const REPLAY_PIECE: usize = 64 << 10;

/// What an interface printed in its terminal. The pseudo-terminal lives in the backend, which
/// keeps the last `OUTPUT_KEPT` bytes of its output, so that the interface runs with no window
/// (Remote Control keeps it up) and a window that opens it, or opens it again after a reload,
/// finds its screen as it is.
#[derive(Default)]
pub struct TuiOutput {
    inner: Mutex<Output>,
}

/// Where a window is sent what a terminal prints.
pub type Forward = Box<dyn Fn(Vec<u8>) + Send>;

#[derive(Default)]
struct Output {
    kept: std::collections::VecDeque<u8>,
    /// The window's sink, while a window shows the terminal.
    forward: Option<Forward>,
    /// A window has shown it (it still does, or did before it was closed).
    shown: bool,
    /// The private modes the program set (`CSI ? Pn h` and `l`), by number: on or off, as last
    /// seen. The start of the output, where a program sets most of them once, is dropped in
    /// time, and the window's terminal would never know.
    modes: std::collections::BTreeMap<u16, bool>,
    /// Where the sequence being read stands, from one piece of output to the next.
    parse: ModeParse,
    /// The cursor position query, whose end may come in the next piece.
    query: crate::pty::CursorQuery,
}

/// The most private modes followed.
const MAX_MODES: usize = 64;
/// The longest list of mode numbers looked into (`CSI ? 1049;25;… h`).
const MAX_MODE_PARAMS: usize = 64;
/// Where the output is cut at the start when it is too long: after the first line end found in
/// this many bytes, so that what is kept does not begin in the middle of a sequence.
const CUT_LOOKS_IN: usize = 4096;

/// What of a private mode's sequence has been read: `ESC`, `ESC [`, `ESC [ ?` and its numbers.
#[derive(Default)]
enum ModeParse {
    #[default]
    Ground,
    Esc,
    Csi,
    Private(Vec<u8>),
}

impl Output {
    /// Follows the modes `data` sets or resets.
    fn track_modes(&mut self, data: &[u8]) {
        for &b in data {
            self.parse = match (std::mem::take(&mut self.parse), b) {
                (_, ESC) => ModeParse::Esc,
                (ModeParse::Esc, b'[') => ModeParse::Csi,
                (ModeParse::Csi, b'?') => ModeParse::Private(Vec::new()),
                (ModeParse::Private(mut numbers), b'0'..=b'9' | b';')
                    if numbers.len() < MAX_MODE_PARAMS =>
                {
                    numbers.push(b);
                    ModeParse::Private(numbers)
                }
                (ModeParse::Private(numbers), b'h' | b'l') => {
                    for n in numbers.split(|c| *c == b';') {
                        let mode = std::str::from_utf8(n)
                            .ok()
                            .and_then(|n| n.parse::<u16>().ok());
                        if let Some(mode) = mode {
                            if self.modes.len() < MAX_MODES || self.modes.contains_key(&mode) {
                                self.modes.insert(mode, b == b'h');
                            }
                        }
                    }
                    ModeParse::Ground
                }
                _ => ModeParse::Ground,
            };
        }
    }

    /// The sequences that set the modes again, as last seen.
    fn modes_again(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (mode, on) in &self.modes {
            out.extend_from_slice(
                format!("\x1b[?{mode}{}", if *on { 'h' } else { 'l' }).as_bytes(),
            );
        }
        out
    }
}

impl std::fmt::Debug for TuiOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let o = self.inner.lock();
        f.debug_struct("TuiOutput")
            .field("kept", &o.kept.len())
            .field("shown", &o.shown)
            .finish()
    }
}

impl TuiOutput {
    /// An output for the window `forward` sends to, if there is one yet.
    pub(crate) fn new(forward: Option<Forward>) -> Self {
        Self {
            inner: Mutex::new(Output {
                shown: forward.is_some(),
                forward,
                ..Default::default()
            }),
        }
    }

    /// What the terminal just printed: kept, and sent to the window that shows it. Under one lock
    /// with `attach`, so that a window taking over finds every byte once, in order. True when it
    /// ends the cursor position query of a terminal no window has shown: the caller answers it,
    /// where a window would have (the same lock tells whether one does).
    pub(crate) fn push(&self, data: Vec<u8>) -> bool {
        let mut o = self.inner.lock();
        o.track_modes(&data);
        let asked = o.query.feed(&data);
        o.kept.extend(data.iter());
        let excess = o.kept.len().saturating_sub(OUTPUT_KEPT);
        if excess > 0 {
            o.kept.drain(..excess);
            // Not in the middle of a sequence, or of a line: from the next line.
            let line = o.kept.iter().take(CUT_LOOKS_IN).position(|b| *b == b'\n');
            if let Some(line) = line {
                o.kept.drain(..=line);
            }
        }
        if let Some(forward) = &o.forward {
            forward(data);
        }
        asked && !o.shown
    }

    /// A window is taking the terminal over: it is shown from now on, so that nothing stops it
    /// as one nobody looks at. Called under the lock of the agent, with the terminal found.
    pub(crate) fn mark_shown(&self) {
        self.inner.lock().shown = true;
    }

    /// A window takes the terminal over: it is sent all that was kept, then what comes, and the
    /// window that had it before (a reloaded one) is let go. The queries the terminal made
    /// (`without_terminal_queries`) are left out of what is replayed: they were answered, and
    /// the window's terminal would answer them again, the answers typed into the interface.
    pub(crate) fn attach(&self, forward: Forward) {
        let mut o = self.inner.lock();
        let kept: Vec<u8> = o.kept.iter().copied().collect();
        // The modes first: the start of the output, which set them, may be gone.
        let modes = o.modes_again();
        if !modes.is_empty() {
            forward(modes);
        }
        for piece in without_terminal_queries(&kept).chunks(REPLAY_PIECE) {
            forward(piece.to_vec());
        }
        o.forward = Some(forward);
        o.shown = true;
    }

    /// A window has shown the terminal.
    pub(crate) fn shown(&self) -> bool {
        self.inner.lock().shown
    }

    /// The private mode `mode` as the program last set it; None: never set.
    pub(crate) fn mode(&self, mode: u16) -> Option<bool> {
        self.inner.lock().modes.get(&mode).copied()
    }

    /// The bytes kept (tests).
    #[cfg(test)]
    pub(crate) fn kept(&self) -> Vec<u8> {
        self.inner.lock().kept.iter().copied().collect()
    }
}

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;
/// The longest OSC or DCS string looked into for a query (the longer ones are kept as they are).
const MAX_STRING: usize = 4096;

/// `bytes` less the requests a terminal answers (the answers go to the program as input): device
/// attributes (`CSI c`), device and cursor status reports (`CSI n`, `CSI ? n`), `CSI > q`, mode
/// requests (`CSI $ p`), the keyboard protocol's (`CSI ? u`), window reports (`CSI 14 t`, 16, 18,
/// 19), the OSC that ask a color or the clipboard (`?` for a value), and the DCS `$ q` and `+ q`.
/// All else stays byte for byte. A sequence cut short at the end (or started before what the
/// buffer begins with) is left as it is.
fn without_terminal_queries(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let Some(at) = bytes[i..].iter().position(|b| *b == ESC) else {
            out.extend_from_slice(&bytes[i..]);
            break;
        };
        out.extend_from_slice(&bytes[i..i + at]);
        i += at;
        match terminal_query_len(&bytes[i..]) {
            Some(n) => i += n,
            None => {
                out.push(ESC);
                i += 1;
            }
        }
    }
    out
}

/// The length of the query `s`, which begins with ESC, opens; None when it opens something else
/// or is not whole.
fn terminal_query_len(s: &[u8]) -> Option<usize> {
    match *s.get(1)? {
        b'[' => csi_query_len(s),
        b']' => {
            let (end, len) = string_end(s, true)?;
            let mut parts = s[2..end].split(|b| *b == b';');
            let colors_or_clipboard = matches!(parts.next()?, b"4" | b"10" | b"11" | b"12" | b"52");
            (colors_or_clipboard && parts.any(|p| p == b"?")).then_some(len)
        }
        b'P' => {
            let (end, len) = string_end(s, false)?;
            let payload = &s[2..end];
            (payload.starts_with(b"$q") || payload.starts_with(b"+q")).then_some(len)
        }
        _ => None,
    }
}

/// Where the OSC or DCS string `s` (ESC, then its opener) ends: the end of its payload and the
/// length of the whole, up to its terminator (BEL too, for an OSC; else ESC \). None when it has
/// none within `MAX_STRING`, or something else comes first.
fn string_end(s: &[u8], bel: bool) -> Option<(usize, usize)> {
    let limit = s.len().min(2 + MAX_STRING);
    for i in 2..limit {
        match s[i] {
            BEL if bel => return Some((i, i + 1)),
            ESC => return (s.get(i + 1) == Some(&b'\\')).then_some((i, i + 2)),
            _ => {}
        }
    }
    None
}

/// The length of the query the CSI `s` (ESC [ …) is, if it is one.
fn csi_query_len(s: &[u8]) -> Option<usize> {
    let mut i = 2;
    while s.get(i).is_some_and(|b| (0x30..=0x3f).contains(b)) {
        i += 1;
    }
    let params = &s[2..i];
    let from = i;
    while s.get(i).is_some_and(|b| (0x20..=0x2f).contains(b)) {
        i += 1;
    }
    let intermediates = &s[from..i];
    let last = *s.get(i)?;
    // `params` less the prefix (one of `prefixes`, or none), made of digits only.
    let digits_after = |prefixes: &[u8]| {
        let rest = match params.split_first() {
            Some((p, rest)) if prefixes.contains(p) => rest,
            _ => params,
        };
        rest.iter().all(u8::is_ascii_digit)
    };
    let query = match (intermediates, last) {
        // Device attributes: `CSI c`, `CSI 0 c`, `CSI > c`, `CSI > 0 c`, `CSI = c`.
        ([], b'c') => digits_after(b">="),
        // Status reports: `CSI 5 n`, `CSI 6 n`, `CSI ? 6 n`, `CSI ? 996 n`.
        ([], b'n') => !params.is_empty() && params != b"?" && digits_after(b"?"),
        // The terminal's name and version: `CSI > q`, `CSI > 0 q`.
        ([], b'q') => params.first() == Some(&b'>') && digits_after(b">"),
        // A mode's state: `CSI Ps $ p`, `CSI ? Ps $ p`.
        ([b'$'], b'p') => digits_after(b"?"),
        // The keyboard protocol's flags: `CSI ? u`.
        ([], b'u') => params == b"?",
        // Window reports: the size in pixels (14), of a cell (16), in characters (18), of the
        // screen (19).
        ([], b't') => {
            let first = params.split(|b| *b == b';').next().unwrap_or_default();
            params.iter().all(|b| b.is_ascii_digit() || *b == b';')
                && matches!(first, b"14" | b"16" | b"18" | b"19")
        }
        _ => false,
    };
    query.then_some(i + 1)
}

/// Ends the interface in the terminal `term` for good, as its user would: two Ctrl+C, 700 ms apart,
/// on which Claude Code closes cleanly and archives its remote session (seen on 2.1.301); killed
/// only if it is still there `QUIT_WAIT` after the second. Where the app ends an interface
/// that will not run again: back in Escouade's view, archived, deleted, no longer holding Remote
/// Control. Not when the app quits (the remote session is kept, so that the next start reattaches
/// it), nor when the user closes the terminal (`Core::kill_terminal`).
pub(crate) async fn quit_interface(pty: &PtyManager, term: &str, key: &'static [u8]) {
    // The first one is waited for until it is really written (the terminal's input thread may be
    // behind), off the runtime's threads: the pause starts then, not when it was queued, else the
    // two could be written together, one key press and not two.
    let first = {
        let (pty, term) = (pty.clone(), term.to_string());
        tokio::task::spawn_blocking(move || pty.write_and_wait(&term, key, QUIT_WRITTEN_WITHIN))
            .await
    };
    match first {
        Ok(Ok(_)) => {
            tokio::time::sleep(QUIT_GAP).await;
            match pty.write(term, key) {
                Ok(_) => {
                    let deadline = Instant::now() + QUIT_WAIT;
                    while pty.exists(term) && Instant::now() < deadline {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
                // Gone meanwhile, or it reads nothing: killed below all the same.
                Err(e) => log::debug!("terminal {term}: second Ctrl+C not written: {e:#}"),
            }
        }
        // Gone already, or it takes no input (killed below).
        Ok(Err(e)) => log::debug!("terminal {term}: Ctrl+C not written: {e:#}"),
        Err(e) => log::warn!("terminal {term}: Ctrl+C not written: {e}"),
    }
    // Closing a pseudo-console may wait: off the runtime's threads too.
    let (pty, term) = (pty.clone(), term.to_string());
    let _ = tokio::task::spawn_blocking(move || pty.kill(&term)).await;
}

/// Claude Code's settings for an interface whose hooks call `port` (`claude_terminal::settings`),
/// Escouade's server kept out of it when its tools are refused (`access`).
#[cfg(test)]
pub fn tui_settings(port: u16, engine: &dyn Engine, access: &Escouade) -> Value {
    debug_assert_eq!(engine.id(), EngineId::CLAUDE);
    claude_terminal::settings(&hook_url(port), TOKEN_VAR, access)
}

/// The key of a hook's payload that holds the event its command named (`escouade-hook <event>`,
/// posted as `?event=<event>`): Copilot's payloads do not say it.
pub(crate) const EVENT_KEY: &str = "escouade_hook_event";

/// The event of a hook's query (`event=<name>`), a name of letters and digits only.
fn event_of_query(query: &str) -> Option<String> {
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("event="))
        .filter(|e| !e.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(str::to_string)
}

/// The address of the hooks' server on `port`.
fn hook_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}{PATH}")
}

/// An interface that stops sooner than this after it started, other than by its user's wish
/// (`/exit`, code 0), stopped « as soon as it started ».
pub(crate) const QUICK_EXIT: Duration = Duration::from_secs(20);
/// An automatic start is refused once this many interfaces in a row stopped as soon as they
/// started.
const QUICK_EXITS_MAX: u32 = 2;
/// How much of an interface's first output is kept, to read what it said when it stopped at once.
const EXIT_OUTPUT_KEPT: usize = 8 << 10;

/// What the check before a start found of the session the interface is to resume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCheck {
    /// None to resume: the interface starts a session of its own.
    Fresh,
    /// Its transcript is in the account's folder: resumed.
    Found,
    /// Not in the account's folder, but in the folder of another account (its name, its folder).
    InOtherAccount { account: String, dir: PathBuf },
    /// Nowhere: Claude Code would refuse it.
    Missing,
}

/// The session whose transcript Claude Code must find to start the interface: the agent's own,
/// else the original's of a copy that has not started its own yet (`tui_args`).
fn session_to_resume(m: &AgentMeta) -> Option<&str> {
    m.session_id.as_deref().or(m.fork_of.as_deref())
}

/// `check_session_of` for Claude Code.
#[cfg(test)]
pub fn check_session(
    m: &AgentMeta,
    config_dir: &Path,
    others: impl FnOnce() -> Vec<(String, PathBuf)>,
) -> SessionCheck {
    check_session_of(&claude_terminal::CLAUDE_TERMINAL, m, config_dir, others)
}

/// Whether the session the interface `view` is to resume (`m`) is where it will look for it, in
/// the account's folder `config_dir`, or else in the folder of one of the `others` (looked for
/// only then: their name and folder). Claude Code quits at once on a session it does not find.
pub fn check_session_of(
    view: &dyn TerminalView,
    m: &AgentMeta,
    config_dir: &Path,
    others: impl FnOnce() -> Vec<(String, PathBuf)>,
) -> SessionCheck {
    let Some(session) = session_to_resume(m) else {
        return SessionCheck::Fresh;
    };
    if view.find_log(config_dir, session).is_some() {
        return SessionCheck::Found;
    }
    others()
        .into_iter()
        .find(|(_, dir)| view.find_log(dir, session).is_some())
        .map_or(SessionCheck::Missing, |(account, dir)| {
            SessionCheck::InOtherAccount { account, dir }
        })
}

/// The accounts a session may be found in, other than `own`: the name and the folder of each,
/// those of the same side as `own` only (`in_wsl`: Claude Code finds a session by the folder it
/// runs in, which WSL names in its own way, so a session does not go from Windows to WSL, nor
/// back), and none that shares `own`'s folder.
pub fn other_accounts(
    settings: &Settings,
    own: &Account,
    in_wsl: impl Fn(&Account) -> bool,
) -> Vec<(String, PathBuf)> {
    let (own_dir, side) = (accounts::config_dir(own, settings), in_wsl(own));
    settings
        .accounts
        .iter()
        .filter(|a| a.id != own.id && in_wsl(a) == side)
        .map(|a| {
            (
                accounts::name_in(crate::i18n::ui(), a),
                accounts::config_dir(a, settings),
            )
        })
        .filter(|(_, dir)| *dir != own_dir)
        .collect()
}

/// `bytes` added to the output kept of an interface, up to `EXIT_OUTPUT_KEPT`: its first lines, which
/// say why it stopped at once.
fn keep_output(kept: &mut Vec<u8>, bytes: &[u8]) {
    let room = EXIT_OUTPUT_KEPT.saturating_sub(kept.len());
    kept.extend_from_slice(&bytes[..bytes.len().min(room)]);
}

/// `classify_exit_of` for Claude Code.
#[cfg(test)]
pub fn classify_exit(elapsed: Duration, code: Option<u32>, output: &[u8]) -> TuiExit {
    classify_exit_of(&claude_terminal::CLAUDE_TERMINAL, elapsed, code, output)
}

/// How the interface `view` that ran for `elapsed` ended with `code` (None: killed), having
/// written `output`. Quick: soon after it started, and not by its user's wish (`/exit` ends with
/// code 0). The session is lost when it was quick and the output says the interface did not find
/// it.
pub fn classify_exit_of(
    view: &dyn TerminalView,
    elapsed: Duration,
    code: Option<u32>,
    output: &[u8],
) -> TuiExit {
    let quick = elapsed < QUICK_EXIT && code != Some(0);
    TuiExit {
        code,
        quick,
        session_lost: quick && view.says_session_lost(output),
    }
}

/// Whether a start is refused: it is automatic (`manual`: the user asked for it), and the
/// interface stopped as soon as it started `quick_exits` times in a row.
fn starts_too_fast(manual: bool, quick_exits: u32) -> bool {
    !manual && quick_exits >= QUICK_EXITS_MAX
}

/// What an interface wrote, once its last bytes are in: the terminal's reader may still be behind
/// when the exit is told. Waits a moment for more while it has not said the session was not
/// found, and its output still grows.
fn settled_output(view: &dyn TerminalView, kept: &Mutex<Vec<u8>>) -> Vec<u8> {
    let mut seen = None;
    for _ in 0..12 {
        let len = {
            let output = kept.lock();
            if view.says_session_lost(&output) {
                break;
            }
            output.len()
        };
        if seen == Some(len) {
            break;
        }
        seen = Some(len);
        std::thread::sleep(Duration::from_millis(40));
    }
    kept.lock().clone()
}

/// Why the interface starts a new session: Claude Code could not find the one it was to resume.
pub(crate) fn session_lost_notice() -> String {
    tr!(
        "Session Claude introuvable : l’interface repart en nouvelle session.",
        "Claude session not found: the interface starts a new session."
    )
}

/// The same, for a copy: it is its original's session that is gone.
pub(crate) fn original_lost_notice() -> String {
    tr!(
        "La session de l’original est introuvable : la copie repart en nouvelle session.",
        "The original’s session was not found: the copy starts a new session."
    )
}

/// The complete lines of `path` from `offset` on (a line still being written waits for the next
/// read), and the offset after them.
pub fn read_lines(path: &Path, offset: u64) -> std::io::Result<(Vec<Value>, u64)> {
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    // Written again from the start (another file under the same name): read from there.
    let offset = if len < offset { 0 } else { offset };
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::new();
    f.take(len - offset).read_to_end(&mut buf)?;
    let Some(end) = buf.iter().rposition(|b| *b == b'\n') else {
        return Ok((Vec::new(), offset));
    };
    let lines = buf[..end]
        .split(|b| *b == b'\n')
        .filter_map(|l| serde_json::from_slice::<Value>(l).ok())
        .collect();
    Ok((lines, offset + end as u64 + 1))
}

/// An interface as the registry of Claude Code's sessions lists it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Registered {
    /// The file that lists it (`<pid>.json`).
    pub file: PathBuf,
    /// Its session is linked to claude.ai: the file has a `bridgeSessionId`.
    pub linked: bool,
    /// Since when (ms) it says the interface idle (`status`, `statusUpdatedAt`), if it does.
    pub idle_since: Option<i64>,
}

/// What `file` of the registry says, if it is a whole JSON object: one that is being written, is
/// empty, is too big or is no file gives nothing (it is read again soon).
fn read_registry_file(file: &Path) -> Option<Value> {
    let meta = std::fs::metadata(file).ok()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > REGISTRY_FILE_MAX {
        return None;
    }
    let value: Value = serde_json::from_slice(&std::fs::read(file).ok()?).ok()?;
    value.is_object().then_some(value)
}

/// The interface of `session` in the registry of Claude Code's sessions, `dir` (`<the account's
/// config folder>/sessions`): Claude Code writes a `<pid>.json` there (`sessionId`, `kind`,
/// `entrypoint`, `status`, and a `bridgeSessionId` when the session's Remote Control is active)
/// for each of its processes, once it is up: not on its welcome screen nor on the box about the
/// folder (only a `<pid>.<hash>.key` is there then). The interface's entry has the `cli`
/// entrypoint, the stream's process of the same session the `sdk-cli` one. Once the interface's
/// file is known (`known`) only that one is read. Never blocks for long nor fails: what cannot be
/// read is not there yet.
pub(crate) fn find_registered(
    dir: &Path,
    session: &str,
    known: Option<&Path>,
) -> Option<Registered> {
    let registered = |file: &Path, entry: &Value| Registered {
        file: file.to_path_buf(),
        linked: entry["bridgeSessionId"]
            .as_str()
            .is_some_and(|id| !id.is_empty()),
        idle_since: (entry["status"] == "idle")
            .then(|| entry["statusUpdatedAt"].as_i64())
            .flatten(),
    };
    if let Some(file) = known {
        let entry = read_registry_file(file)?;
        return (entry["entrypoint"] == "cli").then(|| registered(file, &entry));
    }
    for item in std::fs::read_dir(dir).ok()?.flatten().take(REGISTRY_FILES) {
        let file = item.path();
        // `<pid>.json`: the `.key` files and the like are other things.
        let named_by_pid = file.extension().is_some_and(|e| e == "json")
            && file
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
        if !named_by_pid {
            continue;
        }
        let Some(entry) = read_registry_file(&file) else {
            continue;
        };
        if entry["sessionId"] == session && entry["entrypoint"] == "cli" {
            return Some(registered(&file, &entry));
        }
    }
    None
}

/// An entry written before `since` (ms): one of the conversation's history (a session forked or
/// resumed with no read kept), which its view already has.
fn before(e: &Value, since: i64) -> bool {
    e["timestamp"]
        .as_str()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .is_some_and(|t| t.timestamp_millis() < since)
}

/// The relay written for the interface of the agent of `meta` taken away from its folder, then the
/// folders Escouade made for it, innermost first, those that are empty (`remove_dir` leaves one
/// with something in it).
pub(crate) fn take_relay_away(view: &dyn TerminalView, meta: &mut AgentMeta) {
    if let Some(path) = view.relay_path(Path::new(&meta.cwd)) {
        let _ = std::fs::remove_file(path);
    }
    for dir in std::mem::take(&mut meta.relay_made).iter().rev() {
        let _ = std::fs::remove_dir(dir);
    }
}

/// `NO_PROXY` with the loopback in it: the hooks' calls never go through a proxy.
pub(crate) fn no_proxy(env: &mut Vec<(String, String)>) {
    const LOOPBACK: &str = "127.0.0.1,localhost,::1";
    for key in ["NO_PROXY", "no_proxy"] {
        match env.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) if !v.trim().is_empty() => *v = format!("{v},{LOOPBACK}"),
            Some((_, v)) => *v = LOOPBACK.into(),
            None => env.push((key.into(), LOOPBACK.into())),
        }
    }
}

/// How long `send_to_tui` waits for a message's paste to be written to the terminal before it
/// goes on (a program that reads nothing: the paste stays queued, and the Enter follows it).
const PASTE_WRITTEN_WITHIN: Duration = Duration::from_secs(5);

/// Bracketed paste (`CSI ? 2004 h`).
const BRACKETED_PASTE: u16 = 2004;
/// How long `send_to_tui` waits for an interface that turned bracketed paste off to turn it on
/// again, before it pastes all the same. Claude Code under Windows turns it off as it starts,
/// listed in its registry already, and on again up to eight seconds after (2.1.301, ESC-12): pasted
/// in between, a line break would send the message's first line alone.
const PASTE_MODE_WITHIN: Duration = Duration::from_secs(15);

/// How long an image pasted in an interface is kept. Claude Code reads the file as it is pasted
/// and keeps the image in its transcript (base64), which a resume sends again without the file
/// (seen on 2.1.300); but it tells the model the file's path (« [Image: source: …] »), which the
/// model may open in the turns that follow.
const PASTES_KEPT: Duration = Duration::from_secs(24 * 3600);

/// The most a pasted image may weigh, as all the files of a message in the composer (`MAX_TOTAL`).
const MAX_PASTE: usize = 18 << 20;

/// The extension Claude Code knows an image by, from its first bytes; None for anything but the
/// images it reads.
fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// Removes the pasted images older than `PASTES_KEPT` from `dir`.
pub(crate) fn clear_pastes(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let aged = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > PASTES_KEPT);
        if aged {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// The interface in a terminal of the harness of the agent `m` (« Vue <harness> »), and its name;
/// refused for a harness that has none.
pub(crate) fn view_of(m: &AgentMeta) -> Result<(&'static dyn TerminalView, &'static str)> {
    let harness = crate::engine::of(m);
    match harness.terminal() {
        Some(view) => Ok((view, harness.name())),
        None => bail!(tr!(
            "{harness} n’a pas de vue terminal : l’agent reste dans la vue d’Escouade.",
            "{harness} has no terminal view: the agent stays in Escouade’s view.",
            harness = harness.name()
        )),
    }
}

/// The key that ends the interface of the agent `m` for good, sent twice (`quit_interface`):
/// its harness's, Ctrl+C for one with no interface.
pub(crate) fn quit_key(m: &AgentMeta) -> &'static [u8] {
    view_of(m).map_or(b"\x03", |(view, _)| view.quit_key())
}

/// Refused: a ticket's agent is driven by its board, over the stream.
pub(crate) fn ticket_refusal() -> anyhow::Error {
    anyhow!(tr!(
        "Un agent de ticket reste dans la vue d’Escouade : son tableau le pilote.",
        "A ticket’s agent stays in Escouade’s view: its board drives it."
    ))
}

impl<R: Runtime> Core<R> {
    /// The port of the hooks' server, once started.
    #[cfg(test)]
    pub(crate) fn hooks_port(&self) -> Option<u16> {
        *self.hooks.port.lock()
    }

    /// The port of the hooks' server, started the first time.
    pub(crate) fn hook_port(self: &Arc<Self>) -> Result<u16> {
        let mut port = self.hooks.port.lock();
        if let Some(p) = *port {
            return Ok(p);
        }
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let p = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let me = Arc::downgrade(self);
        tauri::async_runtime::spawn(serve(listener, me, p));
        *port = Some(p);
        Ok(p)
    }

    /// « Vue Claude Code » on or off for the agent. On, its stream's process stops (the window
    /// then opens its interface, `open_agent_terminal`); off, its interface stops and the stream
    /// takes its session up again.
    pub async fn set_agent_view(self: &Arc<Self>, id: &str, terminal: bool) -> Result<()> {
        let h = self.agent(id)?;
        // OpenCode: its interface joins the process, which goes on (`attach`).
        if crate::engine::of(&h.lock().meta).attaches() {
            return self.set_attached(&h, id, terminal).await;
        }
        let lock = self.spawn_lock(id);
        let guard = lock.lock().await;
        {
            let rt = h.lock();
            if rt.meta.terminal == terminal {
                return Ok(());
            }
            if terminal && rt.meta.ticket_id.is_some() {
                return Err(ticket_refusal());
            }
            if terminal {
                view_of(&rt.meta)?;
            }
            if rt.meta.status.is_active() && rt.tui.is_none() {
                bail!(tr!(
                    "L’agent travaille encore : change de vue quand son tour est fini.",
                    "The agent is still working: switch views once its turn is over."
                ));
            }
        }
        if terminal {
            // Its stream's session on claude.ai is closed for good first, the interface will hold
            // its own (Remote Control stays in force): a process that just stopped would leave
            // one behind, orphan, at every switch.
            let (name, link) = {
                let mut rt = h.lock();
                rt.meta.terminal = true;
                // Cursor: what is said from now on is its interface's, told to its session on the
                // way back (`relay_back`).
                let relays = view_of(&rt.meta)
                    .is_ok_and(|(v, _)| v.relay_path(Path::new(&rt.meta.cwd)).is_some());
                if relays {
                    let last = rt
                        .conv
                        .last_where(|_| true)
                        .and_then(|i| i["id"].as_str().map(str::to_string));
                    rt.meta.relay_from = Some(last.unwrap_or_default());
                }
                // The user chose this view: the interface it starts is a fresh try.
                rt.tui_quick_exits = 0;
                rt.tui_exit = None;
                (self.remote_name(&rt.meta), rt.end_link())
            };
            if let Some(link) = link {
                self.request_save();
                // Closed even if Remote Control is in force: the interface holds its own session.
                self.close_link(&h, link, &name, Duration::from_secs(15), false)
                    .await;
            }
            let stopped = h.lock().detach();
            if let Some(p) = stopped {
                p.close_input();
                stop_and_wait(&p).await;
            }
        } else {
            self.read_transcript(id);
            self.relay_back(id).await;
            let term = {
                let mut fx = Effects::default();
                let mut rt = h.lock();
                rt.meta.terminal = false;
                rt.tui_exit = None;
                // The stream shows what it does itself: an interface started later reads the
                // log from its end then, not what the stream added to it again.
                rt.meta.transcript_read = None;
                rt.gen += 1;
                let term = rt
                    .tui
                    .as_ref()
                    .map(|t| (t.term_id.clone(), quit_key(&rt.meta)));
                rt.stop_tui(&mut fx);
                term
            };
            self.hooks.revoke(id);
            // Gone before the stream resumes the session: never two processes on one. The launch
            // lock is not held while it quits (five seconds at worst): what starts a process or
            // an interface of the agent waits for the quit (`after_quit`).
            if let Some((t, key)) = term {
                let quit = self.quit_soon(id, &t, key);
                drop(guard);
                let _ = quit.await;
            }
        }
        self.emit_agent(&h);
        self.request_save();
        // The stream's process; or, in the Claude Code view, the interface of an agent that holds
        // Remote Control (it stays up with no window to show it).
        self.warm(id);
        Ok(())
    }

    /// « Vue Cursor » (`TerminalView::relay_path`): the conversation of the agent until it went to
    /// its interface, written where the interface reads it (a rule in its folder, kept out of git);
    /// the file taken away when nothing was said yet.
    async fn relay_to_interface(&self, view: &dyn TerminalView, h: &AgentHandle) {
        let lang = self.lang().claude;
        let (cwd, text) = {
            let mut rt = h.lock();
            let items = rt.conv.items();
            let upto = match rt.meta.relay_from.as_deref() {
                Some("") => 0,
                Some(mark) => items
                    .iter()
                    .position(|i| i["id"] == mark)
                    .map_or(items.len(), |at| at + 1),
                None => items.len(),
            };
            let text = handover::relay(
                lang,
                Relay::ToInterface,
                &items[..upto],
                rt.meta.plan.as_ref(),
                handover::RELAY_BUDGET,
            );
            (rt.meta.cwd.clone(), text)
        };
        let Some(path) = view.relay_path(Path::new(&cwd)) else {
            return;
        };
        let Some(text) = text else {
            let _ = std::fs::remove_file(&path);
            return;
        };
        // The folders made for it (not there before), kept to be taken away with it.
        let made: Vec<String> = path
            .ancestors()
            .skip(1)
            .take_while(|d| d.starts_with(&cwd) && *d != Path::new(&cwd))
            .filter(|d| !d.exists())
            .map(|d| d.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if !made.is_empty() {
            let mut rt = h.lock();
            for d in made {
                if !rt.meta.relay_made.contains(&d) {
                    rt.meta.relay_made.push(d);
                }
            }
        }
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, view.relay_file(&text)));
        if let Err(e) = written {
            log::warn!("{}: relay not written: {e}", path.display());
            return;
        }
        let rel = path.strip_prefix(&cwd).unwrap_or(&path);
        let pattern = format!("**/{}", rel.to_string_lossy().replace('\\', "/"));
        if let Err(e) = crate::git::ensure_excluded(&cwd, &pattern).await {
            log::debug!("{cwd}: relay not kept out of git: {e:#}");
        }
    }

    /// Back in Escouade's view from Cursor's: what was said in its interface since it went there,
    /// kept to be told to its session with the next message (`AgentMeta::relay`), and the rule
    /// written for the interface taken away.
    async fn relay_back(&self, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let lang = self.lang().claude;
        let mut rt = h.lock();
        let Ok((view, _)) = view_of(&rt.meta) else {
            return;
        };
        if view.relay_path(Path::new(&rt.meta.cwd)).is_none() {
            return;
        }
        take_relay_away(view, &mut rt.meta);
        // No mark: born in the view, all it said is its interface's.
        let mark = rt.meta.relay_from.take().unwrap_or_default();
        let items = rt.conv.items();
        let from = items
            .iter()
            .position(|i| i["id"] == mark.as_str())
            .map_or(0, |at| at + 1);
        let said = handover::relay(
            lang,
            Relay::ToSession,
            &items[from..],
            None,
            handover::RELAY_BUDGET,
        );
        if let Some(said) = said {
            // Added to what was not told yet (the view opened and left twice without a message).
            rt.meta.relay = Some(match rt.meta.relay.take() {
                Some(before) => format!("{before}\n\n{said}"),
                None => said,
            });
        }
        drop(rt);
        self.request_save();
    }

    /// Before the agent's process starts, out of its harness's view: a relay left in its folder
    /// (the app stopped while the view was open) is taken away, and the folders made for it.
    pub(crate) fn clear_stale_relay(&self, h: &AgentHandle) {
        let mut rt = h.lock();
        if rt.meta.terminal {
            return;
        }
        let Ok((view, _)) = view_of(&rt.meta) else {
            return;
        };
        let stale = view
            .relay_path(Path::new(&rt.meta.cwd))
            .is_some_and(|p| p.exists());
        if stale || !rt.meta.relay_made.is_empty() {
            log::info!("agent {}: relay of its interface taken away", rt.meta.id);
            take_relay_away(view, &mut rt.meta);
            drop(rt);
            self.request_save();
        }
    }

    /// The user closes the terminal `term` (the window's « × » or « Relancer »). When it is an
    /// agent's interface, its end is no failure to start with Remote Control, however soon it
    /// comes.
    pub fn kill_terminal(&self, term: &str) {
        for h in self.agents.read().values() {
            if let Some(run) = h.lock().tui.as_mut().filter(|t| t.term_id == term) {
                run.killed = true;
            }
        }
        self.pty.kill(term);
    }

    /// Claude Code's interface for the agent, in a terminal the window shows (`on_data`): its
    /// session resumed, or started. One that runs already (started with Remote Control, or shown
    /// before the window was reloaded) is neither stopped nor started again: the window takes it
    /// over, and is sent its screen first.
    ///
    /// The session to resume is looked for first, where Claude Code will look (`settle_session`):
    /// found in another account's folder, it is copied into the agent's; found nowhere, it is given
    /// up and a new one starts, instead of an interface that quits at once.
    ///
    /// `manual`: the user asked for it (« Relancer »). The window's own start (it opened the view)
    /// is refused once the interface stopped as soon as it started twice in a row: it would only
    /// loop. A manual start always goes, and begins the count again.
    pub async fn open_agent_terminal(
        self: &Arc<Self>,
        id: &str,
        size: (u16, u16),
        manual: bool,
        on_data: impl Fn(Vec<u8>) + Send + 'static,
    ) -> Result<TermInfo> {
        let h = self.agent(id)?;
        if crate::engine::of(&h.lock().meta).attaches() {
            return self.open_attached(&h, id, size, Box::new(on_data)).await;
        }
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        if self.quitting.load(std::sync::atomic::Ordering::Acquire) {
            bail!(crate::core::quitting_refusal());
        }
        {
            let mut rt = h.lock();
            let (_, harness) = view_of(&rt.meta)?;
            if !rt.meta.terminal {
                bail!(tr!(
                    "L’agent n’est pas en vue {harness}.",
                    "The agent is not in the {harness} view.",
                    harness = harness
                ));
            }
            if rt.meta.ticket_id.is_some() {
                return Err(ticket_refusal());
            }
            if rt.meta.archived {
                bail!(tr!("L’agent est archivé.", "The agent is archived."));
            }
            if starts_too_fast(manual, rt.tui_quick_exits) {
                bail!(tr!(
                    "{harness} s’arrête dès son lancement ({n} fois de suite) : regarde sa sortie, puis clique sur « Relancer ».",
                    "{harness} stops as soon as it starts ({n} times in a row): read its output, then click “Restart”.",
                    harness = harness,
                    n = rt.tui_quick_exits
                ));
            }
            if manual {
                rt.tui_quick_exits = 0;
            }
        }
        if let Some((info, output)) = self.running_interface(&h, true) {
            output.attach(Box::new(on_data));
            // Claude Code draws its screen again at the window's size.
            let _ = self.pty.resize(&info.id, size.0, size.1);
            return Ok(info);
        }
        self.launch_interface(&h, id, size, Some(Box::new(on_data)))
            .await
    }

    /// The agent's interface, when it runs: its terminal and what that printed. With `show`, a
    /// window is taking it over: it is shown from now on, under the lock that found it, so that
    /// `stop_unseen_interface` (which looks under that lock) does not stop it before the window
    /// is attached.
    pub(crate) fn running_interface(
        &self,
        h: &AgentHandle,
        show: bool,
    ) -> Option<(TermInfo, Arc<TuiOutput>)> {
        let rt = h.lock();
        let run = rt.tui.as_ref()?;
        if !self.pty.exists(&run.term_id) {
            return None;
        }
        if show {
            run.output.mark_shown();
        }
        Some((run.info.clone(), run.output.clone()))
    }

    /// Starts the interface of an agent in the Claude Code view that holds Remote Control, with no
    /// window (`TuiOutput`): the session must stay reachable from claude.ai. One that runs already
    /// is left as it is, as is an agent that does not hold it, has no such view, is a ticket's or
    /// is archived.
    pub(crate) async fn start_interface(self: &Arc<Self>, id: &str) -> Result<()> {
        let h = self.agent(id)?;
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        if self.quitting.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        {
            let rt = h.lock();
            let m = &rt.meta;
            if !(m.remote_control && m.terminal) || m.archived || m.ticket_id.is_some() {
                return Ok(());
            }
            // It stopped as soon as it started too often in a row: not started again by itself,
            // as the window's own start is not (`open_agent_terminal`); « Relancer » is the user's.
            if starts_too_fast(false, rt.tui_quick_exits) {
                log::info!(
                    "agent {id}: interface not started in the background, it stopped as soon as it started {} times in a row",
                    rt.tui_quick_exits
                );
                return Ok(());
            }
        }
        if self.running_interface(&h, false).is_some() {
            return Ok(());
        }
        self.launch_interface(&h, id, BACKGROUND_SIZE, None)
            .await
            .map(|_| ())
    }

    /// Stops the interface of the agent that holds Remote Control no more, when no window has
    /// shown it: nobody types in it. Left alone when a window showed it (the choice holds from
    /// its next start). While it runs a turn (stopped, the work would be lost) it is marked to stop
    /// when the turn is over (`stop_interface_when_idle`).
    pub(crate) fn stop_unseen_interface(&self, h: &AgentHandle) {
        let (id, term) = {
            let mut rt = h.lock();
            let (held, active) = (rt.meta.remote_control, rt.meta.status.is_active());
            let Some(run) = rt.tui.as_mut() else { return };
            if held {
                // It holds it again: nothing is to stop.
                run.stop_when_idle = false;
                return;
            }
            if run.output.shown() {
                return;
            }
            if active {
                run.stop_when_idle = true;
                return;
            }
            let term = (run.term_id.clone(), quit_key(&rt.meta));
            rt.gen += 1;
            rt.stop_tui(&mut Effects::default());
            (rt.meta.id.clone(), term)
        };
        self.hooks.revoke(&id);
        // Quits on a task of its own, as its user would; its end is ignored (the agent's run of it
        // is over), so it is no failure to start with Remote Control.
        self.quit_soon(&id, &term.0, term.1);
        self.emit_agent(h);
        self.request_save();
    }

    /// The turn of the agent's interface may be over: the interface that was to stop with it
    /// (`stop_unseen_interface`) stops, if no window showed it and it holds no Remote Control.
    pub(crate) fn stop_interface_when_idle(&self, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let marked = h.lock().tui.as_ref().is_some_and(|t| t.stop_when_idle);
        if marked {
            self.stop_unseen_interface(&h);
        }
    }

    /// The lock held while an interface of the agent `id` quits (`quit_soon`), and that what
    /// starts a process or an interface of the agent waits for (`after_quit`).
    fn quit_lock(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.quit_locks
            .lock()
            .entry(id.to_string())
            .or_default()
            .clone()
    }

    /// Quits the interface in the terminal `term` of the agent `id` as its user would
    /// (`quit_interface`), on a task of its own, which the caller may wait for. Called under the
    /// agent's launch lock, the caller may let it go next: the quit is already marked, and nothing
    /// starts a process or an interface of the agent before it is over (`after_quit`). Neither
    /// holds the launch lock for the five seconds an interface that holds out takes to be killed.
    pub(crate) fn quit_soon(
        &self,
        id: &str,
        term: &str,
        key: &'static [u8],
    ) -> tauri::async_runtime::JoinHandle<()> {
        let lock = self.quit_lock(id);
        // Marked now when no other quit of the agent is under way (it is, in the rare case of two
        // in a row: the task then waits its turn).
        let held = lock.clone().try_lock_owned().ok();
        let (pty, term) = (self.pty.clone(), term.to_string());
        tauri::async_runtime::spawn(async move {
            let _held = match held {
                Some(held) => held,
                None => lock.lock_owned().await,
            };
            quit_interface(&pty, &term, key).await;
        })
    }

    /// Waits for the interface of the agent `id` that quits (`quit_soon`) to be gone: never two
    /// processes on one session. Called with the launch lock held.
    pub(crate) async fn after_quit(&self, id: &str) {
        let lock = self.quit_locks.lock().get(id).cloned();
        if let Some(lock) = lock {
            drop(lock.lock().await);
        }
    }

    /// Waits at most `within` for the interface of the agent to be up (`interface_ready`), or
    /// gone: what keeps a place in the starts that a setting makes a few at a time.
    pub(crate) async fn wait_interface_ready(&self, h: &AgentHandle, within: Duration) {
        let deadline = Instant::now() + within;
        loop {
            {
                let rt = h.lock();
                if rt.tui.is_none() || rt.interface_ready() {
                    return;
                }
            }
            if Instant::now() >= deadline {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Starts the agent's interface (its spawn lock held): `forward` is the window's sink, if a
    /// window asks for it.
    async fn launch_interface(
        self: &Arc<Self>,
        h: &AgentHandle,
        id: &str,
        size: (u16, u16),
        forward: Option<Forward>,
    ) -> Result<TermInfo> {
        // Never in the project's folder: its worktree, which waited for its first message, is made
        // under the name it has (ESC-156), and set up.
        if h.lock().meta.worktree_pending {
            self.make_pending_worktree(id, None).await;
            self.wait_setup(id).await;
        }
        // Never two processes on one session.
        self.after_quit(id).await;
        let stopped = h.lock().detach();
        if let Some(p) = stopped {
            p.close_input();
            stop_and_wait(&p).await;
        }
        let old = h.lock().tui.take();
        if let Some(old) = old {
            let (pty, term) = (self.pty.clone(), old.term_id);
            let _ = tokio::task::spawn_blocking(move || pty.kill(&term)).await;
        }
        self.hooks.revoke(id);
        let settings = self.settings.read().clone();
        // An account removed meanwhile: another of its harness, else refused (ESC-128).
        let account = self.agent_account(h)?;
        let harness = crate::engine::of(&h.lock().meta);
        let (view, harness_name) = view_of(&h.lock().meta)?;
        let program = accounts::program(&account, &settings).ok_or_else(|| {
            anyhow!(crate::engine::missing_line(
                account.engine,
                crate::i18n::ui()
            ))
        })?;
        // A version older than the tested ones is refused, with its reason.
        crate::engine::registry::check(harness, &program).await?;
        let port = self.hook_port()?;
        let dir = self.data.root().join("tui");
        std::fs::create_dir_all(&dir)?;
        let settings_file = dir.join(format!("{id}.json"));
        let config_dir = self.view_dir(id, harness, &account, &settings);
        if !view.resumes_any() {
            self.settle_session(view, id, &settings, &account, &program, &config_dir)
                .await;
        }
        self.copy_first(view, h, id, &program, &settings, &account)
            .await;
        // A model its Claude account does not list: the default one, said (ESC-124).
        if harness.models_by_alias() {
            self.fit_claude_model(h);
        }
        let started = now_ms();
        let (meta, new_session, remote, cwd, name, gen, transcript, since) = {
            let mut rt = h.lock();
            // A session of its own from the start, when the harness takes its name: its log is
            // known before any hook.
            let new_session = (rt.meta.session_id.is_none() && view.names_its_session())
                .then(|| uuid::Uuid::new_v4().to_string());
            // Without the option once an interface closed right after starting with it.
            let remote =
                (rt.meta.remote_control && !rt.remote_fallback).then(|| self.remote_name(&rt.meta));
            // As it was before the new session is its: what the interface is started from.
            let meta = rt.meta.clone();
            if let Some(s) = new_session.clone() {
                rt.unwritten_session = Some(s.clone());
                rt.meta.session_id = Some(s);
                rt.meta.fork_of = None;
                rt.meta.fork_at = None;
            }
            let file = rt
                .meta
                .session_id
                .clone()
                .and_then(|sid| view.find_log(&config_dir, &sid));
            // Where the last read stopped, when it was this file; else from now on.
            let kept = rt
                .meta
                .transcript_read
                .clone()
                .filter(|(p, _)| file.as_deref() == Some(Path::new(p)));
            let (transcript, offset, since) = match (kept, file) {
                (Some((p, off)), _) => (Some(PathBuf::from(p)), off, None),
                (None, Some(f)) => {
                    let len = std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
                    (Some(f), len, None)
                }
                (None, None) => (None, 0, Some(started - 2_000)),
            };
            rt.gen += 1;
            rt.start_tui();
            (
                meta,
                new_session,
                remote,
                rt.meta.cwd.clone(),
                rt.meta.name.clone(),
                rt.gen,
                (transcript, offset),
                since,
            )
        };
        let with_remote = remote.is_some();
        if !Path::new(&cwd).is_dir() {
            bail!(tr!(
                "Le dossier {dir} n'existe plus",
                "The folder {dir} no longer exists",
                dir = cwd
            ));
        }
        self.ready_for_wsl(&program, h);
        let access = self.agent_access(id, harness);
        // « Projet privé »: the paths its tools are refused.
        let fence = self.agent_fence(id);
        let (url, command) = (hook_url(port), crate::engine::terminal::hook_command());
        let home = harness.agent_home_var().map(|_| self.data.agent_home(id));
        let started_from = view.launch(&TerminalLaunch {
            meta: &meta,
            session: new_session.as_deref(),
            remote: remote.as_deref(),
            private_file: &settings_file,
            home: home.as_deref(),
            hooks: HookServer {
                url: &url,
                token_var: TOKEN_VAR,
                url_var: URL_VAR,
                command: command.as_deref(),
            },
            access: &access,
            fence: &fence,
            program: &program,
        });
        let mut args = started_from.args;
        // The folder of the files joined to its messages, read without a question: the window
        // copies a file dropped on its terminal there (`attachfiles`) and types its path.
        args.extend(
            self.agent_read_dir(id)
                .into_iter()
                .flat_map(|d| view.read_dir_args(&d)),
        );
        let launch = view.escouade(harness, &access, &config_dir);
        args.extend(launch.args);
        // Its hooks, and Escouade's server kept out of it when its tools are refused.
        let written = match &started_from.file {
            Some(bytes) => std::fs::write(&settings_file, bytes).map_err(anyhow::Error::from),
            None => Ok(()),
        }
        .and_then(|()| {
            for (path, bytes) in &started_from.files {
                if let Some(dir) = path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(path, bytes)?;
            }
            Ok(())
        });
        if let Err(e) = written {
            if let Some(t) = access.token() {
                self.mcp.release_agent(id, t);
            }
            return Err(e);
        }
        // Cursor: the conversation so far, where its interface reads it.
        self.relay_to_interface(view, h).await;
        let token = self.hooks.grant(id);
        let mut env = [settings.agent_env(), accounts::launch_env(&account)].concat();
        // A configuration folder of the agent's own, the one of its process (Cursor: what its
        // « Toujours » keeps, and its model).
        if let Some(var) = harness.agent_home_var() {
            let home = self.data.agent_home(id);
            if let Err(e) = std::fs::create_dir_all(&home) {
                log::warn!("agent {id}: {} not made: {e}", home.display());
            }
            env.retain(|(k, _)| k != var);
            env.push((var.to_string(), home.to_string_lossy().into_owned()));
        }
        env.push((TOKEN_VAR.into(), token));
        env.extend(started_from.env);
        env.extend(launch.env);
        let masked = view.masked_env(&env);
        env.extend(masked);
        no_proxy(&mut env);
        // What its harness has to be asked before it starts (Codex: the trust of its hooks).
        match view.prepare(&program, &cwd, &args, &env).await {
            Ok(more) => args.extend(more),
            Err(e) => {
                log::warn!("agent {id}: {harness_name}'s interface not prepared: {e:#}");
                let text = tr!(
                    "Escouade ne suivra pas l’état de cet agent dans son terminal : {harness} n’a pas dit comment approuver ses hooks ({error}).",
                    "Escouade won’t follow this agent’s state in its terminal: {harness} did not tell how to trust its hooks ({error}).",
                    harness = harness_name,
                    error = format!("{e:#}")
                );
                let _ = self.with_agent(id, |rt, fx| {
                    rt.notice("warn", text, fx);
                    Ok(())
                });
            }
        }
        // What its account would refuse (Copilot CLI: a model a Free account does not list).
        let args = view.for_account(&program, &cwd, args, &env).await;
        let info = TermInfo {
            id: new_id(),
            // No project's: the window lists it with no project's terminals.
            project_id: String::new(),
            name,
            shell: view.shell().into(),
        };
        log::info!(
            "agent {id}: starting {harness_name}'s interface {} {} in {cwd} (account {})",
            program.describe(),
            args.join(" "),
            account.id
        );
        let (me, agent, term, granted) = (
            Arc::downgrade(self),
            id.to_string(),
            info.id.clone(),
            access.token().map(str::to_string),
        );
        let output = Arc::new(TuiOutput::new(forward));
        // Its first output is kept apart too: what it says when it stops at once (a session it
        // does not find) is read from it, by its exit.
        let first = Arc::new(Mutex::new(Vec::new()));
        let on_data = {
            let (pty, term, output, kept) = (
                self.pty.clone(),
                info.id.clone(),
                output.clone(),
                first.clone(),
            );
            move |data: Vec<u8>| {
                {
                    let mut kept = kept.lock();
                    if kept.len() < EXIT_OUTPUT_KEPT {
                        keep_output(&mut kept, &data);
                    }
                }
                // With no window to answer it, the pseudo-console's cursor position query is
                // answered here (ConPTY waits for the answer before running anything). Queued, as
                // all writes are: this is the terminal's reader thread, which never waits on it.
                if output.push(data) {
                    if let Err(e) = pty.write(&term, CURSOR_HOME) {
                        log::debug!("terminal {term}: cursor position not answered: {e:#}");
                    }
                }
            }
        };
        let spawned = self.pty.spawn_program(
            info.clone(),
            &view.program(&program),
            &args,
            &cwd,
            size,
            env,
            // Everything it started goes with it.
            false,
            on_data,
            move |code| {
                if let Some(c) = me.upgrade() {
                    if let Some(t) = &granted {
                        c.mcp.release_agent(&agent, t);
                    }
                    c.on_tui_exit(&agent, gen, &term, code, &first);
                }
            },
        );
        if let Err(e) = spawned {
            self.hooks.revoke(id);
            if let Some(t) = access.token() {
                self.mcp.release_agent(id, t);
            }
            return Err(e);
        }
        {
            let mut rt = h.lock();
            rt.tui = Some(TuiRun {
                term_id: info.id.clone(),
                info: info.clone(),
                output,
                gen,
                started: now_ms(),
                transcript: transcript.0,
                offset: transcript.1,
                program,
                remote: with_remote,
                launched_at: Instant::now(),
                spoke: false,
                registered: None,
                registry_file: None,
                sessions: view.running_dir(&config_dir),
                linked: false,
                typing: Default::default(),
                // Gone already (it exited at once): its exit follows.
                typed: self.pty.typed(&info.id).unwrap_or_default(),
                empty_at: Some(0),
                pasted: None,
                cancelled: None,
                killed: false,
                stop_when_idle: false,
            });
            rt.tui_since = since;
            // Under its lock, as a process's start does it (`start_process_of`).
            let given = access.token().is_some_and(|t| self.mcp.caller(t).is_some());
            rt.started_with_escouade(given, self.escouade_granted());
        }
        self.emit_agent(h);
        self.request_save();
        let (c, agent) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move { c.read_loop(&agent, gen).await });
        Ok(info)
    }

    /// An image pasted in the agent's interface (base64 `data`), which reads the clipboard's only
    /// on a key of its own: written to a file, whose path the window types there. Claude Code takes
    /// a pasted path to an image for the image, as a file dropped on a terminal. The path is the
    /// one its `claude` reads (in WSL, the Linux one).
    pub fn paste_image(&self, id: &str, media_type: &str, data: &str) -> Result<String> {
        use base64::Engine;
        let (program, view, harness) = {
            let h = self.agent(id)?;
            let rt = h.lock();
            let (view, harness) = view_of(&rt.meta)?;
            match &rt.tui {
                Some(t) => (t.program.clone(), view, harness),
                None if rt.meta.terminal => bail!(tr!(
                    "{harness} s’est arrêté : relance-le pour y coller une image.",
                    "{harness} stopped: restart it to paste an image into it.",
                    harness = harness
                )),
                None => bail!(tr!(
                    "L’agent n’est pas en vue {harness}.",
                    "The agent is not in the {harness} view.",
                    harness = harness
                )),
            }
        };
        if !view.image_types().contains(&media_type) {
            bail!(tr!(
                "{harness} ne lit pas les images {media_type}",
                "{harness} does not read {media_type} images",
                harness = harness,
                media_type = media_type
            ));
        }
        let too_big = || {
            anyhow!(tr!(
                "L’image dépasse 18 Mo : {harness} ne la recevra pas.",
                "The image is over 18 MB: {harness} will not get it.",
                harness = harness
            ))
        };
        // Measured before it is decoded.
        if data.len() > MAX_PASTE.div_ceil(3) * 4 {
            return Err(too_big());
        }
        let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;
        if bytes.len() > MAX_PASTE {
            return Err(too_big());
        }
        // What it holds, whatever its type says (a file copied under a wrong name).
        let Some(extension) = image_extension(&bytes) else {
            bail!(tr!(
                "Ce n’est pas une image PNG, JPEG, GIF ou WebP : {harness} ne la lit pas.",
                "This is no PNG, JPEG, GIF or WebP image: {harness} does not read it.",
                harness = harness
            ));
        };
        let dir = self.pastes_dir();
        std::fs::create_dir_all(&dir)?;
        clear_pastes(&dir);
        // Named after its agent, whose deletion takes it away.
        let file = dir.join(format!(
            "{id}-{}.{extension}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::write(&file, bytes).with_context(|| file.display().to_string())?;
        Ok(program.file_path(&file))
    }

    /// Where the images pasted in the interfaces are kept.
    pub(crate) fn pastes_dir(&self) -> PathBuf {
        self.data.root().join("tui").join("pastes")
    }

    /// What the interface of the deleted agent `m` left behind, its harness says
    /// (`TerminalView::forget`: Copilot CLI's plugin, and its folder in the account's).
    pub(crate) fn forget_interface(&self, m: &AgentMeta) {
        let Ok((view, _)) = view_of(m) else { return };
        let settings = self.settings.read().clone();
        let account = accounts::get(&settings, &m.account);
        let file = self.data.root().join("tui").join(format!("{}.json", m.id));
        view.forget(&file, &accounts::config_dir(&account, &settings));
    }

    /// Removes the images pasted in the interface of the agent `id`, which is deleted.
    pub(crate) fn forget_pastes(&self, id: &str) {
        let Ok(entries) = std::fs::read_dir(self.pastes_dir()) else {
            return;
        };
        let prefix = format!("{id}-");
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    /// Reads the transcript of the agent's interface `gen` as it is written, and the registry of
    /// Claude Code's sessions while it tells something new, until the interface stops.
    async fn read_loop(self: Arc<Self>, id: &str, gen: u64) {
        loop {
            tokio::time::sleep(READ_EVERY).await;
            let Ok(h) = self.agent(id) else { return };
            if h.lock().tui.as_ref().is_none_or(|t| t.gen != gen) {
                return;
            }
            self.read_transcript(id);
            self.read_registry(&h, id, gen).await;
        }
    }

    /// Looks the agent's interface `gen` up in the registry of Claude Code's sessions
    /// (`find_registered`) while that has something to tell: that it is up, until it is known to
    /// be (a window showed it, or its transcript spoke, do too); and, started with Remote Control,
    /// whether its session has a link, until a sign of one is seen. No link `LINK_WAIT` after it
    /// is up (a sign in its transcript comes first, in the same pass) is the verdict that it has
    /// none (Claude Code says nothing of it in its terminal); one that comes later still connects it.
    async fn read_registry(self: &Arc<Self>, h: &AgentHandle, id: &str, gen: u64) {
        let look = {
            let rt = h.lock();
            let Some(run) = rt.tui.as_ref().filter(|t| t.gen == gen) else {
                return;
            };
            let waits_for_a_link = run.remote
                && !run.linked
                && (run.registered.is_some() || run.launched_at.elapsed() < REGISTRY_PATIENCE);
            // During a turn too, for whether it says the interface idle (its file, once found, is
            // the only one read).
            let in_a_turn = rt.meta.status == AgentStatus::Running;
            (!rt.interface_ready() || waits_for_a_link || in_a_turn)
                .then(|| {
                    Some((
                        view_of(&rt.meta).ok()?.0,
                        run.sessions.clone()?,
                        rt.meta.session_id.clone()?,
                        run.registry_file.clone(),
                    ))
                })
                .flatten()
        };
        // Off the runtime's threads: files are read, as many as the folder holds.
        let found = match look {
            Some((view, dir, session, known)) => tokio::task::spawn_blocking(move || {
                view.find_running(&dir, &session, known.as_deref())
            })
            .await
            .ok()
            .flatten(),
            None => None,
        };
        let mut fx = Effects::default();
        let (pid, name, view) = {
            let mut rt = h.lock();
            if rt.tui.as_ref().is_none_or(|t| t.gen != gen) {
                return;
            }
            if let Some(found) = found {
                if let Some(run) = rt.tui.as_mut() {
                    run.registry_file = Some(found.file);
                    run.registered.get_or_insert_with(Instant::now);
                }
                if found.linked {
                    rt.interface_linked(&mut fx);
                }
                // Idle for a while since the turn started, and the transcript did not end it: the
                // interface cancelled it (`IDLE_ENDS_TURN_MS`).
                let cancelled = found.idle_since.is_some_and(|at| {
                    rt.meta.status == AgentStatus::Running
                        && rt.active_since.is_some_and(|since| at > since)
                        && now_ms() - at >= IDLE_ENDS_TURN_MS
                });
                if cancelled {
                    if let Some(run) = rt.tui.as_mut() {
                        let typed = run.typed.load(Ordering::Acquire);
                        run.cancelled = Some(Cancelled {
                            pasted: run.pasted == Some(typed),
                            typed,
                        });
                    }
                    rt.interrupted = true;
                    rt.end_tui_turn(None, &mut fx);
                }
            }
            // Up for `LINK_WAIT` and no sign of a link, though it was started with Remote Control
            // and the agent holds it still: it has none.
            let late = rt.tui.as_ref().is_some_and(|t| {
                t.remote && !t.linked && t.registered.is_some_and(|at| at.elapsed() >= LINK_WAIT)
            });
            if late && rt.meta.remote_control {
                rt.interface_unlinked(
                    tr!(
                        "Remote control indisponible : l’interface de Claude Code ne l’a pas ouvert (compte sans abonnement claude.ai, réseau, proxy ou règle de l’organisation).",
                        "Remote control unavailable: Claude Code’s interface did not open it (an account with no claude.ai subscription, a network, a proxy or a rule of the organization)."
                    ),
                    &mut fx,
                );
            }
            let view = fx.agent_changed.then(|| rt.view());
            (rt.meta.project_id.clone(), rt.meta.name.clone(), view)
        };
        // Told only when something changed: this runs at every pass of the read loop.
        if view.is_some() || !fx.ops.is_empty() {
            self.apply(id, &pid, &name, fx, view);
        }
    }

    /// Where the interface of the agent `id` keeps its sessions: the agent's own configuration
    /// folder when its harness gives it one (`Engine::agent_home_var`: Mistral Vibe keeps them
    /// there), else its account's.
    fn view_dir(
        &self,
        id: &str,
        harness: &dyn crate::engine::Engine,
        account: &Account,
        settings: &Settings,
    ) -> PathBuf {
        match harness.agent_home_var() {
            Some(_) => self.data.agent_home(id),
            None => accounts::config_dir(account, settings),
        }
    }

    /// What the agent's interface wrote in its transcript since the last read.
    pub(crate) fn read_transcript(self: &Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let (view, gen, known, offset, account, sid, harness) = {
            let rt = h.lock();
            let Some(t) = rt.tui.as_ref() else { return };
            let Ok((view, _)) = view_of(&rt.meta) else {
                return;
            };
            (
                view,
                t.gen,
                t.transcript.clone(),
                t.offset,
                rt.meta.account.clone(),
                rt.meta.session_id.clone(),
                crate::engine::of(&rt.meta),
            )
        };
        let find = || {
            let settings = self.settings.read().clone();
            let account = accounts::get(&settings, &account);
            let dir = self.view_dir(id, harness, &account, &settings);
            sid.as_deref().and_then(|s| view.find_log(&dir, s))
        };
        // Not written yet when the interface started: looked for until it is.
        let path = match known.clone() {
            Some(p) => p,
            None => {
                let Some(found) = find() else {
                    return;
                };
                found
            }
        };
        let read = read_lines(&path, offset);
        // Read to its end (or gone), a log that moves goes on in its next file, read from its start.
        let moved = match &read {
            Ok((lines, _)) if !lines.is_empty() => None,
            _ if view.log_moves() => find().filter(|p| *p != path),
            _ => None,
        };
        let (path, read) = match moved {
            Some(next) => {
                log::info!("agent {id}: its log goes on in {}", next.display());
                let read = read_lines(&next, 0);
                (next, read)
            }
            None => (path, read),
        };
        let (lines, next) = match read {
            Ok(read) => read,
            Err(e) => {
                log::debug!("agent {id}: transcript not read: {e}");
                return;
            }
        };
        let mut fx = Effects::default();
        let (pid, name, view, naming) = {
            let mut rt = h.lock();
            // Another read (the read loop's and a hook's run at once) applied these lines
            // meanwhile: they are not applied twice, and what follows them is read next time.
            if rt
                .tui
                .as_ref()
                .is_none_or(|t| t.gen != gen || t.offset != offset || t.transcript != known)
            {
                return;
            }
            let mut naming = None;
            for e in &lines {
                if rt.tui_since.is_some_and(|since| before(e, since)) {
                    continue;
                }
                let prompts = rt.meta.prompts;
                view.on_log_entry(&mut rt, e, &mut fx);
                // A message of the user's names the agent, as one sent from the window does: the
                // first that gives a text to name it from (not a command with nothing to say, nor
                // what the app typed by itself).
                if rt.meta.prompts > prompts {
                    let prompt = rt.tui_prompt.clone().unwrap_or_default();
                    if !rt.took_unprompted(&prompt) && naming.is_none() {
                        naming = rt.name_from(&prompt);
                    }
                }
            }
            if let Some(t) = rt.tui.as_mut() {
                t.transcript = Some(path.clone());
                t.offset = next;
            }
            let read = Some((path.to_string_lossy().into_owned(), next));
            if rt.meta.transcript_read != read {
                rt.meta.transcript_read = read;
            }
            let view = fx.agent_changed.then(|| rt.view());
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                view,
                naming,
            )
        };
        if lines.is_empty() && fx.ops.is_empty() {
            return;
        }
        self.apply(id, &pid, &name, fx, view);
        // The turn that held up the stop of an interface may be over.
        self.stop_interface_when_idle(id);
        if let Some(text) = naming {
            let (c, id) = (self.clone(), id.to_string());
            tauri::async_runtime::spawn(async move { c.auto_name(&id, &text).await });
        }
    }

    /// A hook of the agent's interface: what it tells, after what its transcript said before.
    pub(crate) fn on_tui_hook(self: &Arc<Self>, id: &str, hook: Value) {
        self.read_transcript(id);
        let Ok(h) = self.agent(id) else { return };
        // A `claude` of WSL tells its transcript by its Linux path.
        let wsl = {
            let settings = self.settings.read().clone();
            let account = accounts::get(&settings, &h.lock().meta.account);
            accounts::program(&account, &settings).and_then(|p| p.wsl().cloned())
        };
        let mut fx = Effects::default();
        let (out, pid, name, view, gen, turn) = {
            let mut rt = h.lock();
            let Some(gen) = rt.tui.as_ref().map(|t| t.gen) else {
                return;
            };
            let Ok((view, _)) = view_of(&rt.meta) else {
                return;
            };
            let out = view.on_hook(&mut rt, &hook, &mut fx);
            if let Some(path) = &out.transcript {
                let path = match &wsl {
                    Some(wsl) if path.starts_with('/') => wsl.windows_path(path),
                    _ => PathBuf::from(path),
                };
                if let Some(t) = rt.tui.as_mut() {
                    t.transcript = Some(path);
                    t.offset = 0;
                }
                rt.tui_since = None;
            }
            let view = fx.agent_changed.then(|| rt.view());
            (
                out,
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                view,
                gen,
                rt.tui_turn_mark(),
            )
        };
        self.apply(id, &pid, &name, fx, view);
        if out.stopped {
            // A background subagent the user stopped in the interface says so only in its files.
            self.end_stopped_subagents(id);
            // The transcript tells the end of the turn right after the hook; if it still did not
            // after a moment (a version that does not write it), the turn ends here: that turn, not
            // the next message's, which may have started meanwhile.
            let (c, id) = (self.clone(), id.to_string());
            tauri::async_runtime::spawn(async move {
                for wait in [300, 1500] {
                    tokio::time::sleep(Duration::from_millis(wait)).await;
                    c.read_transcript(&id);
                }
                let _ = c.with_agent(&id, |rt, fx| {
                    if rt.tui.as_ref().is_some_and(|t| t.gen == gen) {
                        rt.end_stopped_tui_turn(&turn, fx);
                    }
                    Ok(())
                });
                c.stop_interface_when_idle(&id);
            });
        }
    }

    /// The agent's interface `gen` ended (`/exit`, closed, killed), having written `output` (its
    /// first bytes). How it ended is told to the window (`AgentView::tui_exit`): one that quit at
    /// once because Claude Code did not find the session to resume gives that session up.
    fn on_tui_exit(
        self: &Arc<Self>,
        id: &str,
        gen: u64,
        term: &str,
        code: Option<u32>,
        output: &Mutex<Vec<u8>>,
    ) {
        self.hub.emit(UiEvent::TerminalExit {
            id: term.to_string(),
            code,
        });
        if self.quitting.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }
        let started = self.agent(id).ok().and_then(|h| {
            let rt = h.lock();
            let view = view_of(&rt.meta).ok()?;
            rt.tui
                .as_ref()
                .filter(|t| t.gen == gen)
                .map(|t| (t.started, view))
        });
        let Some((started, (view, harness))) = started else {
            log::info!("agent {id}: an interface that is no longer its own exited (code {code:?})");
            return;
        };
        log::info!("agent {id}: {harness}'s interface exited (code {code:?})");
        self.read_transcript(id);
        let elapsed = Duration::from_millis((now_ms() - started).max(0) as u64);
        // Only what a quick exit may have said is read, and waited for.
        let said = if elapsed < QUICK_EXIT && code != Some(0) {
            settled_output(view, output)
        } else {
            Vec::new()
        };
        let exit = classify_exit_of(view, elapsed, code, &said);
        if exit.session_lost {
            log::info!("agent {id}: {harness} did not find the session to resume");
        }
        // A session that is gone made it stop, not Remote Control.
        let failed = !exit.session_lost && self.remote_failed_to_start(id, gen);
        self.tui_ended_with(id, gen, Some(exit));
        // No window ever showed it: it is started again in the background, without the option,
        // unless it quit at once too often already (`start_interface` counts the quick exits as
        // the window does): an interface that quits quickly is never started again twice.
        if failed {
            let (c, id) = (self.clone(), id.to_string());
            tauri::async_runtime::spawn(async move {
                if let Err(e) = c.start_interface(&id).await {
                    log::warn!("agent {id}: interface not started again: {e:#}");
                }
            });
        }
    }

    /// The agent's interface `gen`, started with Remote Control, closed right after it started
    /// (`RC_GRACE`), though no window ever showed it and the user did not close its terminal: the
    /// option is taken for the cause. The agent is told, and its next interfaces start without the
    /// option until the app starts again or the user turns Remote Control on by hand
    /// (`AgentRt::remote_fallback`). Whether that is the case. One a window showed is no such
    /// failure, whatever its code: its user saw its screen, and may have ended it (« No, exit » on
    /// the box about the folder, Escape there, `/exit`, two Ctrl+C); « Relancer » is theirs.
    fn remote_failed_to_start(self: &Arc<Self>, id: &str, gen: u64) -> bool {
        self.with_agent(id, |rt, fx| {
            let Some(run) = rt.tui.as_ref().filter(|t| t.gen == gen) else {
                return Ok(false);
            };
            if !run.remote || run.killed || run.launched_at.elapsed() >= RC_GRACE || run.output.shown()
            {
                return Ok(false);
            }
            // Turned off meanwhile, or archived: not what closed it.
            if !rt.meta.remote_control || rt.meta.archived {
                return Ok(false);
            }
            rt.remote_fallback = true;
            rt.remote_unavailable_notice(
                tr!(
                    "Remote control indisponible : l’interface de Claude Code s’est fermée juste après son lancement avec le remote control ; elle est relancée sans lui.",
                    "Remote control unavailable: Claude Code’s interface closed right after it started with remote control; it is started again without it."
                ),
                fx,
            );
            Ok(true)
        })
        .unwrap_or(false)
    }

    /// The interface `gen` of the agent is over: its hooks' token is refused and the agent's run of
    /// it ends, only if, under the agent's lock, it is still the one running. A newer one, which
    /// takes its place under that lock before it is given a token, keeps its own (« Relancer » kills
    /// a live interface just before the next one starts, and the old one's exit may come after).
    #[cfg(test)]
    pub(crate) fn tui_ended(self: &Arc<Self>, id: &str, gen: u64) {
        self.tui_ended_with(id, gen, None);
    }

    /// `tui_ended`, with how the interface ended when it is known (`exit`): the agent keeps it for
    /// its view, counts the quick exits in a row, and gives its session up when Claude Code did not
    /// find it (its next start is a new session).
    fn tui_ended_with(self: &Arc<Self>, id: &str, gen: u64, exit: Option<TuiExit>) {
        let _ = self.with_agent(id, |rt, fx| {
            if rt.tui.as_ref().is_some_and(|t| t.gen == gen) {
                self.hooks.revoke(id);
                rt.stop_tui(fx);
                if let Some(exit) = exit {
                    if exit.session_lost {
                        rt.forget_session();
                        rt.notice("warn", session_lost_notice(), fx);
                    }
                    rt.tui_quick_exits = if exit.quick {
                        rt.tui_quick_exits + 1
                    } else {
                        0
                    };
                    rt.tui_exit = Some(exit);
                }
                fx.save = true;
            }
            Ok(())
        });
    }

    /// Before the agent's interface starts: the session it is to resume is looked for where Claude
    /// Code will look, in the account's folder (`check_session`).
    /// - In another account's folder only (the agent changed account, its session was left
    ///   behind): copied into this one's, resumed, and the conversation says so.
    /// - Nowhere (the agent never sent a message, so there is no transcript; the transcript is
    ///   gone; a copy whose original is): given up, and the interface starts a new session, which
    ///   the conversation says when the agent had a conversation to lose.
    ///
    /// Nobody writes the session meanwhile: the agent's processes are stopped, and the interface is
    /// not started yet.
    async fn settle_session(
        self: &Arc<Self>,
        view: &'static dyn TerminalView,
        id: &str,
        settings: &Settings,
        account: &Account,
        program: &crate::engine::Program,
        config_dir: &Path,
    ) {
        let Ok(h) = self.agent(id) else { return };
        let (meta, unwritten) = {
            let rt = h.lock();
            (rt.meta.clone(), rt.unwritten_session.clone())
        };
        let Some(session) = session_to_resume(&meta).map(str::to_string) else {
            return;
        };
        // A `claude` of WSL: its folder is read through the distribution, which may not answer
        // (not running): then nothing is known of the session, and Claude Code is left to tell.
        if program.wsl().is_some() && !config_dir.is_dir() {
            return;
        }
        let (all, own, dir, m) = (
            settings.clone(),
            account.clone(),
            config_dir.to_path_buf(),
            meta.clone(),
        );
        let checked = tokio::task::spawn_blocking(move || {
            check_session_of(view, &m, &dir, || {
                let in_wsl =
                    |a: &Account| accounts::program(a, &all).is_some_and(|p| p.wsl().is_some());
                other_accounts(&all, &own, in_wsl)
            })
        })
        .await;
        let Ok(check) = checked else { return };
        let is_copy = meta.session_id.is_none();
        // A conversation to lose: messages sent, or entries of it read; never in the session an
        // interface named and did not write (no message sent in it).
        let had_conversation = (meta.prompts > 0 || meta.last_entry.is_some())
            && unwritten.as_deref() != Some(session.as_str());
        let to = accounts::name_in(crate::i18n::ui(), account);
        let lost = |notice: Option<String>| {
            let _ = self.with_agent(id, |rt, fx| {
                rt.forget_session();
                fx.save = true;
                if let Some(text) = notice {
                    rt.notice("warn", text, fx);
                }
                Ok(())
            });
        };
        match check {
            SessionCheck::Fresh | SessionCheck::Found => {}
            SessionCheck::Missing => {
                log::info!("agent {id}: session {session} not found in any account's folder");
                let notice = if is_copy {
                    Some(original_lost_notice())
                } else {
                    had_conversation.then(session_lost_notice)
                };
                lost(notice);
            }
            SessionCheck::InOtherAccount { account: from, dir } => {
                log::info!("agent {id}: session {session} found in the folder of account {from}");
                let (target, sid) = (config_dir.to_path_buf(), session.clone());
                let copied =
                    tokio::task::spawn_blocking(move || view.copy_session(&dir, &target, &sid))
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|r| r);
                match copied {
                    Ok(()) => {
                        let _ = self.with_agent(id, |rt, fx| {
                            let text = tr!(
                                "La session était dans le dossier du compte « {from} » : elle a été copiée dans celui de « {to} » pour être reprise.",
                                "The session was in the folder of the “{from}” account: it was copied into the one of “{to}” to be resumed.",
                                from = from,
                                to = to
                            );
                            rt.notice("info", text, fx);
                            Ok(())
                        });
                    }
                    Err(e) => {
                        log::warn!("agent {id}: session {session} not copied from {from}: {e:#}");
                        let text = tr!(
                            "La session est dans le dossier du compte « {from} », mais elle n’a pas pu être copiée dans celui de « {to} » ({error}) : l’interface repart en nouvelle session.",
                            "The session is in the folder of the “{from}” account, but it could not be copied into the one of “{to}” ({error}): the interface starts a new session.",
                            from = from,
                            to = to,
                            error = format!("{e:#}")
                        );
                        lost(Some(text));
                    }
                }
            }
        }
    }

    /// The copy of a conversation whose interface cannot make it itself (Copilot's): made before it
    /// starts (`TerminalView::copy_first`), and resumed then as the agent's own session. Failed, the
    /// interface starts a new session, which the conversation says.
    async fn copy_first(
        self: &Arc<Self>,
        view: &'static dyn TerminalView,
        h: &AgentHandle,
        id: &str,
        program: &crate::engine::Program,
        settings: &Settings,
        account: &Account,
    ) {
        let (of, at, cwd) = {
            let rt = h.lock();
            match (&rt.meta.session_id, &rt.meta.fork_of) {
                (None, Some(of)) => (of.clone(), rt.meta.fork_at.clone(), rt.meta.cwd.clone()),
                _ => return,
            }
        };
        let mut env = [settings.agent_env(), accounts::launch_env(account)].concat();
        let masked = view.masked_env(&env);
        env.extend(masked);
        no_proxy(&mut env);
        let Some(copy) = view.copy_first(program, &cwd, &env, &of, at.as_deref()) else {
            return;
        };
        let copied = copy.await;
        let _ = self.with_agent(id, |rt, fx| {
            match copied {
                Ok(session) => {
                    log::info!("agent {id}: session {of} copied into {session} for its interface");
                    rt.meta.session_id = Some(session);
                    rt.meta.fork_of = None;
                    rt.meta.fork_at = None;
                }
                Err(e) => {
                    log::warn!("agent {id}: session {of} not copied for its interface: {e:#}");
                    rt.forget_session();
                    let text = tr!(
                        "La conversation n’a pas pu être copiée ({error}) : l’interface part d’une nouvelle session.",
                        "The conversation could not be copied ({error}): the interface starts a new session.",
                        error = format!("{e:#}")
                    );
                    rt.notice("warn", text, fx);
                }
            }
            fx.save = true;
            fx.agent_changed = true;
            Ok(())
        });
    }

    /// Before the stream resumes the agent's session (its launch lock held), once its interface is
    /// gone: a session the interface named and never wrote (no message sent in it, so Claude Code
    /// has no transcript of it) is given up without a word, and the stream starts a new one.
    /// Resumed, it would fail, and tell a loss once more after the interface told its own
    /// (ESC-113). Left to the stream when the account's folder cannot be read (a `claude` of WSL
    /// whose distribution is not running).
    pub(crate) fn drop_unwritten_session(&self, h: &AgentHandle) {
        let (session, meta) = {
            let rt = h.lock();
            let Some(s) = rt.unwritten_session.clone() else {
                return;
            };
            if rt.meta.session_id.as_deref() != Some(s.as_str()) {
                return;
            }
            (s, rt.meta.clone())
        };
        let Ok((view, _)) = view_of(&meta) else {
            return;
        };
        let settings = self.settings.read().clone();
        let dir = accounts::config_dir(&accounts::get(&settings, &meta.account), &settings);
        if !dir.is_dir() || view.find_log(&dir, &session).is_some() {
            return;
        }
        let mut rt = h.lock();
        if rt.meta.session_id.as_deref() == Some(session.as_str()) {
            log::info!(
                "agent {}: session {session} never written by its interface, the stream starts a new one",
                rt.meta.id
            );
            rt.forget_session();
        }
    }

    /// Whether the agent's interface, up in its terminal, asks the question about the trust of its
    /// folder (Codex, Copilot CLI): the band of the harness's view says so, above the terminal.
    pub(crate) fn tui_asks_trust(&self, id: &str) -> Result<bool> {
        let h = self.agent(id)?;
        let (account, cwd, spoke, view) = {
            let rt = h.lock();
            let Some(tui) = rt.tui.as_ref() else {
                return Ok(false);
            };
            let (view, _) = view_of(&rt.meta)?;
            (
                rt.meta.account.clone(),
                rt.meta.cwd.clone(),
                tui.spoke,
                view,
            )
        };
        let config_dir = {
            let settings = self.settings.read();
            accounts::config_dir(&accounts::get(&settings, &account), &settings)
        };
        Ok(view.paste_refusal(&config_dir, &cwd, spoke).is_some())
    }

    /// A message for an agent in the Claude Code view: typed in its interface, as pasted text
    /// then Enter. Refused while it waits for an answer there (the text would go to its dialog).
    pub(crate) fn send_to_tui(self: &Arc<Self>, id: &str, text: &str) -> Result<()> {
        let h = self.agent(id)?;
        let (account, cwd) = {
            let rt = h.lock();
            (rt.meta.account.clone(), rt.meta.cwd.clone())
        };
        let config_dir = {
            let settings = self.settings.read();
            accounts::config_dir(&accounts::get(&settings, &account), &settings)
        };
        let (term, typing, output, view, harness) = {
            let rt = h.lock();
            let (view, harness) = view_of(&rt.meta)?;
            if rt.tui.is_some() {
                let spoke = rt.tui.as_ref().is_some_and(|t| t.spoke);
                if let Some(why) = view.paste_refusal(&config_dir, &cwd, spoke) {
                    bail!(why);
                }
            }
            if rt.meta.status == AgentStatus::Waiting {
                bail!(tr!(
                    "L’agent attend une réponse dans son terminal : réponds-y d’abord.",
                    "The agent is waiting for an answer in its terminal: answer there first."
                ));
            }
            // An interface that is not up may be on its welcome screen, or on the box about the
            // folder, where a text typed chooses an option.
            if rt.tui.is_some() && !rt.interface_ready() {
                bail!(tr!(
                    "L’interface de {harness} de cet agent n’est pas encore prête (premier lancement ou dossier à approuver) : ouvre l’agent pour la voir.",
                    "The {harness} interface of this agent is not ready yet (first launch or a folder to approve): open the agent to see it.",
                    harness = harness
                ));
            }
            (
                rt.tui.as_ref().map(|t| t.term_id.clone()),
                rt.tui.as_ref().map(|t| t.typing.clone()),
                rt.tui.as_ref().map(|t| t.output.clone()),
                view,
                harness,
            )
        };
        let (Some(term), Some(typing)) = (term, typing) else {
            bail!(tr!(
                "L’agent est en vue {harness} : ouvre-le pour démarrer son terminal.",
                "The agent is in the {harness} view: open it to start its terminal.",
                harness = harness
            ));
        };
        // Line breaks pasted stay in the message (Enter alone would send it).
        let pasted = view.paste(text);
        // One message at a time, paste and Enter: another one's paste between them would join the
        // two messages into one. (This runs off the runtime's threads: `Core::send`.)
        let _typing = typing.lock();
        let start = Instant::now();
        while output
            .as_ref()
            .is_some_and(|o| o.mode(BRACKETED_PASTE) == Some(false))
            && start.elapsed() < PASTE_MODE_WITHIN
        {
            std::thread::sleep(Duration::from_millis(50));
        }
        // A message cancelled before its answer, that Claude Code put back in its prompt
        // (`Cancelled`): the one Escouade pasted alone, it is emptied first by a single Ctrl+C
        // (two within a second quit the interface); anything else is left there, and said.
        let (cancelled, typed) = {
            let mut rt = h.lock();
            match rt.tui.as_mut().filter(|t| t.term_id == term) {
                Some(t) => (t.cancelled.take(), t.typed.clone()),
                None => (None, Arc::default()),
            }
        };
        match cancelled {
            Some(c) if c.pasted && typed.load(Ordering::Acquire) == c.typed => {
                self.pty.write_and_wait(&term, b"\x03", EMPTIED_WITHIN)?;
                std::thread::sleep(AFTER_EMPTYING);
                log::info!("agent {id}: the prompt of the cancelled message emptied");
            }
            Some(_) => {
                let _ = self.with_agent(id, |rt, fx| {
                    rt.notice(
                        "warn",
                        tr!(
                            "{harness} a remis dans sa saisie le message annulé avant sa réponse, et du texte a été tapé dans son terminal : Escouade ne l’a pas vidée, ce message s’ajoute à ce qu’elle contient.",
                            "{harness} put the message cancelled before its answer back in its prompt, and text was typed in its terminal: Escouade did not empty it, this message is added to what it holds.",
                            harness = harness
                        ),
                        fx,
                    );
                    Ok(())
                });
            }
            None => {}
        }
        {
            // Its prompt empty as it is pasted: a cancellation of it leaves this message alone there.
            let now = typed.load(Ordering::Acquire);
            let mut rt = h.lock();
            if let Some(t) = rt.tui.as_mut().filter(|t| t.term_id == term) {
                if cancelled.is_some_and(|c| c.pasted && c.typed == now) {
                    t.empty_at = Some(now);
                }
                t.pasted = (t.empty_at == Some(now)).then_some(now);
            }
        }
        // The pause starts once the paste is really written, not queued: under load the terminal's
        // input thread may be behind, and the queue would then join the paste and the Enter into
        // one write, which is what the pause keeps apart. An error on either write is the
        // message's: it did not go, or not whole.
        self.pty
            .write_and_wait(&term, &pasted, PASTE_WRITTEN_WITHIN)?;
        std::thread::sleep(Duration::from_millis(150));
        self.pty.write(&term, view.submit()).map(|_| ())
    }

    /// Interrupts the turn of the agent's interface, as its key does there (Escape in Claude
    /// Code's).
    pub(crate) fn interrupt_tui(&self, id: &str) -> Result<bool> {
        let h = self.agent(id)?;
        let term = {
            let rt = h.lock();
            let view = view_of(&rt.meta).ok().map(|(v, _)| v);
            rt.tui.as_ref().map(|t| t.term_id.clone()).zip(view)
        };
        let Some((t, view)) = term else {
            return Ok(false);
        };
        self.pty.write(&t, view.interrupt_key())?;
        if let Some(gap) = view.interrupt_again() {
            let (pty, key) = (self.pty.clone(), view.interrupt_key());
            std::thread::spawn(move || {
                std::thread::sleep(gap);
                if let Err(e) = pty.write(&t, key) {
                    log::debug!("terminal {t}: interruption not confirmed: {e:#}");
                }
            });
        }
        Ok(true)
    }
}

/// Waits a moment for a stream's process asked to stop to be gone (its session is then free),
/// then kills it.
async fn stop_and_wait(p: &crate::engine::Process) {
    for _ in 0..50 {
        if !p.is_alive() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    p.kill();
}

/// Answers the hooks on `listener` until the app quits.
async fn serve<R: Runtime>(listener: TcpListener, core: Weak<Core<R>>, port: u16) {
    let listener = match tokio::net::TcpListener::from_std(listener) {
        Ok(l) => l,
        Err(e) => {
            log::error!("hooks: cannot listen: {e}");
            return;
        }
    };
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            tokio::time::sleep(Duration::from_millis(50)).await;
            continue;
        };
        if core.strong_count() == 0 {
            return;
        }
        let core = core.clone();
        tauri::async_runtime::spawn(async move {
            let service = service_fn(move |req| {
                let core = core.clone();
                async move { Ok::<_, Infallible>(answer(req, &core, port).await) }
            });
            let _ = http1::Builder::new()
                .timer(TokioTimer::new())
                .header_read_timeout(Duration::from_secs(10))
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

/// One hook call: from a program of this machine that holds an interface's token (a browser's
/// page gives itself away by its `Origin` or its `Host`). Always answered at once, with nothing
/// decided: the interface goes on as without the hook.
async fn answer<R: Runtime>(
    req: Request<Incoming>,
    core: &Weak<Core<R>>,
    port: u16,
) -> Response<Full<Bytes>> {
    let reply = |status: StatusCode, body: &str| {
        let mut r = Response::new(Full::new(Bytes::from(body.to_string())));
        *r.status_mut() = status;
        r.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        r
    };
    let headers = req.headers();
    let host_ok = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| {
            [format!("127.0.0.1:{port}"), format!("localhost:{port}")]
                .iter()
                .any(|m| m.eq_ignore_ascii_case(h))
        });
    if !host_ok || headers.contains_key(header::ORIGIN) {
        return reply(StatusCode::FORBIDDEN, "{}");
    }
    if req.method() != Method::POST || req.uri().path() != PATH {
        return reply(StatusCode::NOT_FOUND, "{}");
    }
    let event = req.uri().query().and_then(event_of_query);
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, t)| t.trim().to_string());
    let Some(core) = core.upgrade() else {
        return reply(StatusCode::SERVICE_UNAVAILABLE, "{}");
    };
    let Some(agent) = token.and_then(|t| core.hooks.agent_of(&t)) else {
        return reply(StatusCode::UNAUTHORIZED, "{}");
    };
    let body = match Limited::new(req.into_body(), MAX_BODY).collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return reply(StatusCode::PAYLOAD_TOO_LARGE, "{}"),
    };
    let Ok(mut hook) = serde_json::from_slice::<Value>(&body) else {
        return reply(StatusCode::BAD_REQUEST, "{}");
    };
    if let (Some(event), Some(map)) = (event, hook.as_object_mut()) {
        map.insert(EVENT_KEY.into(), Value::String(event));
    }
    // A search for a symbol is answered with a line that points to the code index: context only,
    // never a decision (`codeindex::hint`).
    let advice = core
        .index_hint(&agent, &hook)
        .map_or_else(|| "{}".to_string(), |v| v.to_string());
    // Off the connection: the interface does not wait for what Escouade makes of it.
    tauri::async_runtime::spawn_blocking(move || core.on_tui_hook(&agent, hook));
    reply(StatusCode::OK, &advice)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> AgentMeta {
        AgentMeta {
            model: "sonnet".into(),
            effort: "high".into(),
            mode: "default".into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_new_agent_starts_a_session_of_its_own_and_a_known_one_is_resumed() {
        let f = Path::new("/data/tui/a1.json");
        let a = tui_args(&meta(), Some("s-new"), None, f);
        assert!(a.contains(&"--session-id=s-new".to_string()));
        assert!(a
            .windows(2)
            .any(|w| w == ["--settings", "/data/tui/a1.json"]));
        assert!(a.windows(2).any(|w| w == ["--permission-mode", "default"]));
        // Nothing of the stream's.
        assert!(!a.iter().any(|x| x.contains("stream-json") || x == "-p"));
        let resumed = AgentMeta {
            session_id: Some("s1".into()),
            ..meta()
        };
        let a = tui_args(&resumed, None, None, f);
        assert!(a.contains(&"--resume=s1".to_string()));
        assert!(!a.iter().any(|x| x.starts_with("--session-id")));
        // A copy forks its original's into its own.
        let copy = AgentMeta {
            fork_of: Some("s0".into()),
            fork_at: Some("e1".into()),
            ..meta()
        };
        let a = tui_args(&copy, Some("s2"), None, f);
        for x in [
            "--resume=s0",
            "--fork-session",
            "--resume-session-at=e1",
            "--session-id=s2",
        ] {
            assert!(a.contains(&x.to_string()), "{x} in {a:?}");
        }
    }

    #[test]
    fn an_interface_with_remote_control_starts_with_its_name_in_claude_ai() {
        let f = Path::new("/data/tui/a1.json");
        let a = tui_args(&meta(), Some("s-new"), Some("demo · agent-1"), f);
        // One argument, the name glued to the flag: a name is never taken for an option of its own.
        assert!(
            a.contains(&"--remote-control=demo · agent-1".to_string()),
            "{a:?}"
        );
        assert!(!a.iter().any(|x| x == "--remote-control"), "{a:?}");
        // Not even one that starts with « - ».
        let a = tui_args(&meta(), Some("s-new"), Some("-demo · agent-1"), f);
        assert!(
            a.contains(&"--remote-control=-demo · agent-1".to_string()),
            "{a:?}"
        );
        assert!(!a.contains(&"-demo · agent-1".to_string()), "{a:?}");
        let a = tui_args(&meta(), Some("s-new"), None, f);
        assert!(
            !a.iter().any(|x| x.starts_with("--remote-control")),
            "{a:?}"
        );
    }

    fn collecting() -> (Arc<Mutex<Vec<u8>>>, Forward) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        (seen, Box::new(move |b| sink.lock().extend(b)))
    }

    #[test]
    fn an_interfaces_output_is_kept_up_to_its_limit_and_its_end_stays() {
        let out = TuiOutput::new(None);
        for _ in 0..4 {
            out.push(vec![b'a'; 1 << 20]);
        }
        assert_eq!(out.kept().len(), OUTPUT_KEPT);
        // What comes pushes the oldest out.
        out.push(b"fin".to_vec());
        let kept = out.kept();
        assert_eq!(kept.len(), OUTPUT_KEPT);
        assert!(kept.ends_with(b"fin"));
        assert!(kept.starts_with(b"aaa"));
        // A piece bigger than all of it: its end stays.
        let mut big = vec![b'x'; 10];
        big.extend(vec![b'z'; OUTPUT_KEPT]);
        out.push(big);
        let kept = out.kept();
        assert_eq!(kept.len(), OUTPUT_KEPT);
        assert!(kept.iter().all(|b| *b == b'z'));
    }

    #[test]
    fn a_window_taking_the_output_over_is_sent_what_was_kept_then_what_comes() {
        let out = TuiOutput::new(None);
        assert!(!out.shown());
        out.push(b"avant ".to_vec());
        let (first, forward) = collecting();
        out.attach(forward);
        assert!(out.shown());
        assert_eq!(*first.lock(), b"avant ");
        out.push(b"apres".to_vec());
        assert_eq!(*first.lock(), b"avant apres");
        // A reloaded window takes it over: it is sent it all, and the first is let go.
        let (second, forward) = collecting();
        out.attach(forward);
        assert_eq!(*second.lock(), b"avant apres");
        out.push(b"!".to_vec());
        assert_eq!(*second.lock(), b"avant apres!");
        assert_eq!(*first.lock(), b"avant apres");
        // A window that closes leaves it shown: it was seen.
        assert!(out.shown());
    }

    #[test]
    fn an_output_made_for_a_window_is_shown_from_the_start() {
        let (seen, forward) = collecting();
        let out = TuiOutput::new(Some(forward));
        assert!(out.shown());
        out.push(b"ok".to_vec());
        assert_eq!(*seen.lock(), b"ok");
    }

    #[test]
    fn what_is_replayed_to_a_window_is_sent_in_pieces_without_the_cursor_query() {
        let out = TuiOutput::new(None);
        // The query is answered already: a second answer would be typed into the interface.
        out.push(b"a\x1b[6".to_vec());
        out.push(b"nb".to_vec());
        out.push(vec![b'c'; REPLAY_PIECE * 2 + 1]);
        let pieces = Arc::new(Mutex::new(Vec::<usize>::new()));
        let all = Arc::new(Mutex::new(Vec::<u8>::new()));
        let (sizes, bytes) = (pieces.clone(), all.clone());
        out.attach(Box::new(move |b| {
            sizes.lock().push(b.len());
            bytes.lock().extend(b);
        }));
        let mut want = b"ab".to_vec();
        want.extend(vec![b'c'; REPLAY_PIECE * 2 + 1]);
        assert_eq!(*all.lock(), want);
        assert!(pieces.lock().iter().all(|n| *n <= REPLAY_PIECE));
        assert!(pieces.lock().len() >= 3);
        // The terminal's own copy is left as it was printed.
        assert!(out.kept().windows(4).any(|w| w == b"\x1b[6n"));
    }

    /// What a window is sent when it takes the output over, as one string.
    fn replayed(out: &TuiOutput) -> Vec<u8> {
        let (all, forward) = collecting();
        out.attach(forward);
        let all = all.lock().clone();
        all
    }

    #[test]
    fn the_modes_an_interface_set_are_posed_again_when_the_start_of_its_output_is_lost() {
        let out = TuiOutput::new(None);
        // Set once, at the start, as Claude Code does.
        out.push(b"\x1b[?2004h\x1b[?1004h\x1b[?25l\x1b[?1049h".to_vec());
        // Then more than the output keeps.
        let line = b"some line of the screen\r\n".repeat(1000);
        for _ in 0..(OUTPUT_KEPT / line.len() + 2) {
            out.push(line.clone());
        }
        assert!(!out.kept().windows(8).any(|w| w == b"\x1b[?2004h"));
        let replay = replayed(&out);
        let preamble: &[u8] = b"\x1b[?25l\x1b[?1004h\x1b[?1049h\x1b[?2004h";
        assert!(replay.starts_with(preamble), "{:?}", &replay[..40]);
        // What follows is whole lines.
        assert!(replay[preamble.len()..].starts_with(b"some line of the screen\r\n"));
    }

    #[test]
    fn a_mode_is_followed_across_pieces_with_the_last_state_seen_and_many_numbers_at_once() {
        let out = TuiOutput::new(None);
        // Cut in the middle of the sequence, of its number, of its end.
        for piece in [&b"a\x1b"[..], b"[?20", b"04", b"h b\x1b[?", b"25l"] {
            out.push(piece.to_vec());
        }
        // Several numbers in one; the last state wins.
        out.push(b"\x1b[?1049;1004h\x1b[?25h\x1b[?25l\x1b[?25h".to_vec());
        // Not a mode: a private query, an ANSI mode, a report, text that looks like one.
        out.push(b"\x1b[?2004$p\x1b[4h\x1b[?7$y \x1b[?2026 h ?7h \x1b[1;?9h".to_vec());
        let replay = replayed(&out);
        assert!(
            replay.starts_with(b"\x1b[?25h\x1b[?1004h\x1b[?1049h\x1b[?2004h"),
            "{:?}",
            String::from_utf8_lossy(&replay)
        );
    }

    #[test]
    fn nothing_is_posed_again_when_no_mode_was_set_and_garbage_is_no_mode() {
        let out = TuiOutput::new(None);
        out.push(b"plain\x1b[?h\x1b[?;h\x1b[?99999999999h\x1b[?\x1b[?2004h\x1b[?25\x07l".to_vec());
        let replay = replayed(&out);
        // Only the mode that was whole is posed; a number too big for a mode is none.
        assert!(replay.starts_with(b"\x1b[?2004h"), "{replay:?}");
        let out = TuiOutput::new(None);
        out.push(b"plain\x1b[31mred\x1b[0m".to_vec());
        assert_eq!(replayed(&out), b"plain\x1b[31mred\x1b[0m");
    }

    #[test]
    fn the_output_kept_after_the_start_was_dropped_begins_after_a_line() {
        let out = TuiOutput::new(None);
        let mut first = b"x".repeat(100);
        first.extend_from_slice(b"\x1b[38;5;2");
        out.push(first);
        // The cut falls inside a sequence: what is kept does not begin with its end.
        let mut second = b"31m text\r\nthe real start of the kept output\r\n".to_vec();
        second.extend(vec![b'y'; OUTPUT_KEPT - 40]);
        out.push(second.clone());
        let kept = out.kept();
        assert!(kept.len() < OUTPUT_KEPT);
        assert!(
            kept.starts_with(b"the real start of the kept output\r\n"),
            "{:?}",
            String::from_utf8_lossy(&kept[..40])
        );
        // No newline in the first 4 KiB: kept as it is.
        let out = TuiOutput::new(None);
        out.push(vec![b'a'; OUTPUT_KEPT + 10]);
        assert_eq!(out.kept().len(), OUTPUT_KEPT);
    }

    #[test]
    fn the_cursor_query_is_answered_by_the_one_that_sees_no_window() {
        let out = TuiOutput::new(None);
        // Cut between two pieces, as it is when it is printed.
        assert!(!out.push(b"text \x1b[6".to_vec()));
        assert!(out.push(b"n".to_vec()));
        assert!(!out.push(b"more".to_vec()));
        assert!(out.push(b"\x1b[6n".to_vec()));
        // A window shows it: its terminal answers.
        out.mark_shown();
        assert!(out.shown());
        assert!(!out.push(b"\x1b[6n".to_vec()));
        let (_, forward) = collecting();
        let shown = TuiOutput::new(Some(forward));
        assert!(!shown.push(b"\x1b[6n".to_vec()));
    }

    /// Every request a terminal answers, as a program sends it.
    const QUERIES: &[&[u8]] = &[
        // Device attributes.
        b"\x1b[c",
        b"\x1b[0c",
        b"\x1b[>c",
        b"\x1b[>0c",
        b"\x1b[=c",
        // Status reports, the cursor position's.
        b"\x1b[5n",
        b"\x1b[6n",
        b"\x1b[?6n",
        b"\x1b[?996n",
        // The terminal's name and version.
        b"\x1b[>q",
        b"\x1b[>0q",
        // Modes.
        b"\x1b[?2004$p",
        b"\x1b[20$p",
        // The keyboard protocol's.
        b"\x1b[?u",
        // Window reports.
        b"\x1b[14t",
        b"\x1b[16t",
        b"\x1b[18t",
        b"\x1b[19t",
        // Colors and clipboard, ended by BEL or ST.
        b"\x1b]10;?\x07",
        b"\x1b]11;?\x1b\\",
        b"\x1b]12;?\x07",
        b"\x1b]4;1;?\x07",
        b"\x1b]4;1;?\x1b\\",
        b"\x1b]52;c;?\x07",
        b"\x1b]52;;?\x1b\\",
        // Settings and capabilities.
        b"\x1bP$qm\x1b\\",
        b"\x1bP+q544e\x1b\\",
    ];

    /// What a terminal prints that no terminal answers.
    const ORDINARY: &[&[u8]] = &[
        b"plain text, a line\r\n",
        "é ✓ 日本語".as_bytes(),
        b"\x1b[31;1mred\x1b[0m",
        b"\x1b[2J\x1b[H\x1b[10;5H\x1b[K\x1b[2K",
        b"\x1b[?25l\x1b[?2004h\x1b[?1049h",
        b"\x1b[5 q",
        b"\x1b[!p",
        // Answers, not requests.
        b"\x1b[?1;2c",
        b"\x1b[?1u",
        // Settings, not requests.
        b"\x1b[>4;2m",
        b"\x1b[>4n",
        b"\x1b[>1u",
        b"\x1b[<u",
        b"\x1b[22;0;0t",
        b"\x1b[8;40;120t",
        b"\x1b[?2004$y",
        // Titles, links, colors set, clipboard set.
        b"\x1b]0;a title\x07",
        b"\x1b]2;a title\x1b\\",
        b"\x1b]8;;https://example.com/?q=1\x1b\\link\x1b]8;;\x1b\\",
        b"\x1b]10;#ffffff\x07",
        b"\x1b]11;rgb:00/00/00\x1b\\",
        b"\x1b]4;1;rgb:ff/00/00\x07",
        b"\x1b]52;c;aGVsbG8=\x07",
        b"\x1bPq#0;2;0;0;0\x1b\\",
        b"\x1b(B\x1b=\x1b>\x1b7\x1b8",
    ];

    #[test]
    fn every_request_a_terminal_answers_is_left_out_of_what_is_replayed() {
        for q in QUERIES {
            let shown = String::from_utf8_lossy(q).into_owned();
            assert_eq!(without_terminal_queries(q), b"", "{shown:?}");
            let mut around = b"a".to_vec();
            around.extend_from_slice(q);
            around.push(b'b');
            assert_eq!(without_terminal_queries(&around), b"ab", "{shown:?}");
        }
        // Side by side, and between ordinary sequences.
        let all: Vec<u8> = QUERIES.concat();
        assert_eq!(without_terminal_queries(&all), b"");
    }

    #[test]
    fn what_no_terminal_answers_is_replayed_byte_for_byte() {
        for s in ORDINARY {
            let shown = String::from_utf8_lossy(s).into_owned();
            assert_eq!(without_terminal_queries(s), *s, "{shown:?}");
        }
        // Mixed with requests, in order.
        let mut mixed = Vec::new();
        let mut want = Vec::new();
        for (i, s) in ORDINARY.iter().enumerate() {
            mixed.extend_from_slice(QUERIES[i % QUERIES.len()]);
            mixed.extend_from_slice(s);
            want.extend_from_slice(s);
        }
        assert_eq!(without_terminal_queries(&mixed), want);
    }

    #[test]
    fn a_request_across_two_pieces_of_the_replay_is_left_out_whole() {
        for q in QUERIES {
            let shown = String::from_utf8_lossy(q).into_owned();
            // It begins two bytes before the end of a piece (a shorter one ends in it).
            let mut buffer = vec![b'x'; REPLAY_PIECE - 2];
            buffer.extend_from_slice(q);
            buffer.extend_from_slice(b"y");
            let out = TuiOutput::new(None);
            out.push(buffer);
            let (all, forward) = collecting();
            out.attach(forward);
            let mut want = vec![b'x'; REPLAY_PIECE - 2];
            want.push(b'y');
            assert_eq!(*all.lock(), want, "{shown:?}");
        }
    }

    #[test]
    fn a_sequence_cut_short_is_left_as_it_is() {
        for q in QUERIES {
            for n in 1..q.len() {
                let cut = &q[..n];
                let shown = String::from_utf8_lossy(cut).into_owned();
                assert_eq!(without_terminal_queries(cut), cut, "{shown:?}");
                let mut after_text = b"text".to_vec();
                after_text.extend_from_slice(cut);
                assert_eq!(
                    without_terminal_queries(&after_text),
                    after_text,
                    "{shown:?}"
                );
            }
        }
        // The buffer begins in the middle of one (the beginning was dropped): nothing to find.
        for tail in [
            &b";?\x07rest"[..],
            b"6n and text",
            b"?\x1b\\ok",
            b"4;1;?\x07",
        ] {
            assert_eq!(without_terminal_queries(tail), tail);
        }
        // Another sequence begins before it ends: what came is kept, the new one is looked at.
        let aborted = b"\x1b]10;?\x1b[31mx\x1b[6n";
        assert_eq!(without_terminal_queries(aborted), b"\x1b]10;?\x1b[31mx");
        // A string with no end within reach is kept as it is.
        let mut open = b"\x1b]11;?".to_vec();
        open.extend(vec![b'z'; MAX_STRING]);
        open.push(BEL);
        assert_eq!(without_terminal_queries(&open), open);
    }

    #[test]
    fn any_bytes_are_filtered_without_a_panic() {
        // A generator of its own: bytes that open and close the sequences, much more often than
        // chance would.
        let alphabet: &[u8] = b"\x1b\x1b\x1b[[]]P\\\\\x07;?$>=<0123456789cnqputm +!xyz\x00\xff\xc3";
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..20_000 {
            let len = (next() % 48) as usize;
            let bytes: Vec<u8> = (0..len)
                .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
                .collect();
            let out = without_terminal_queries(&bytes);
            assert!(out.len() <= bytes.len());
        }
        // Openers with no end, in great number: no quadratic wait.
        let started = std::time::Instant::now();
        for opener in [&b"\x1b]"[..], b"\x1bP", b"\x1b[", b"\x1b"] {
            let bytes = opener.repeat(200_000);
            assert_eq!(without_terminal_queries(&bytes), bytes);
        }
        assert!(started.elapsed() < Duration::from_secs(20));
    }

    #[test]
    fn the_hooks_call_the_server_with_the_token_of_the_environment() {
        let granted = Escouade::Granted {
            url: "http://127.0.0.1:4321/mcp".into(),
            token: "t".into(),
            file: Some(PathBuf::from("/data/mcp/a1.json")),
        };
        let s = tui_settings(4321, &crate::engine::CLAUDE, &granted);
        let h = &s["hooks"]["PermissionRequest"][0]["hooks"][0];
        assert_eq!(h["type"], "http");
        assert_eq!(h["url"], "http://127.0.0.1:4321/hook");
        assert_eq!(h["headers"]["Authorization"], "Bearer $ESCOUADE_HOOK_TOKEN");
        assert_eq!(h["allowedEnvVars"][0], TOKEN_VAR);
        for e in EVENTS {
            assert!(s["hooks"][e].is_array(), "{e}");
        }
        // Claude Code skips its HTTP hooks for SessionStart ("HTTP hooks are not supported for
        // SessionStart"): none is asked for there, the registry of its sessions says it is up.
        assert!(!EVENTS.contains(&"SessionStart"));
        assert!(s["hooks"].get("SessionStart").is_none(), "{s}");
        assert!(s.get("deniedMcpServers").is_none(), "{s}");
    }

    #[test]
    fn an_interface_refused_escouades_tools_is_kept_from_its_server_by_the_same_settings() {
        // Claude Code reads the last `--settings` only: the refusal goes into the hooks' file.
        let s = tui_settings(
            4321,
            &crate::engine::CLAUDE,
            &Escouade::Denied { file: None },
        );
        assert!(s["hooks"]["PermissionRequest"].is_array(), "{s}");
        assert_eq!(
            s["deniedMcpServers"],
            json!([{ "serverName": "escouade" }]),
            "{s}"
        );
    }

    #[test]
    fn the_event_a_hook_names_in_its_query_is_read_whole_and_plain() {
        assert_eq!(
            event_of_query("event=agentStop").as_deref(),
            Some("agentStop")
        );
        assert_eq!(
            event_of_query("x=1&event=sessionEnd").as_deref(),
            Some("sessionEnd")
        );
        assert_eq!(event_of_query("event="), None);
        assert_eq!(event_of_query("event=a%20b"), None);
        assert_eq!(event_of_query("other=agentStop"), None);
    }

    #[test]
    fn the_transcript_is_read_by_complete_lines() {
        let dir = std::env::temp_dir().join(format!("escouade-tui-read-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.jsonl");
        std::fs::write(&path, "{\"a\":1}\n{\"b\":2}\n{\"c\":").unwrap();
        let (lines, off) = read_lines(&path, 0).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(off, 16);
        // The rest of the line comes.
        std::fs::write(&path, "{\"a\":1}\n{\"b\":2}\n{\"c\":3}\n").unwrap();
        let (lines, off) = read_lines(&path, off).unwrap();
        assert_eq!(lines, vec![json!({ "c": 3 })]);
        assert_eq!(off, 24);
        // Shorter than what was read: written again, read from the start.
        std::fs::write(&path, "{\"d\":4}\n").unwrap();
        assert_eq!(read_lines(&path, off).unwrap().0, vec![json!({ "d": 4 })]);
    }

    #[test]
    fn the_transcript_of_a_session_is_found_in_its_accounts_folder() {
        let dir = std::env::temp_dir().join(format!("escouade-tui-find-{}", new_id()));
        let folder = dir.join("projects").join("-work-demo");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("s1.jsonl"), "").unwrap();
        assert_eq!(find_transcript(&dir, "s1"), Some(folder.join("s1.jsonl")));
        assert_eq!(find_transcript(&dir, "s2"), None);
    }

    #[test]
    fn the_loopback_goes_round_the_proxy() {
        let mut env = vec![("NO_PROXY".to_string(), "corp.lan".to_string())];
        no_proxy(&mut env);
        let get = |k: &str| env.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone());
        assert_eq!(get("NO_PROXY").unwrap(), "corp.lan,127.0.0.1,localhost,::1");
        assert_eq!(get("no_proxy").unwrap(), "127.0.0.1,localhost,::1");
    }

    /// A folder for a test of the registry, with the files `(name, content)` in it.
    fn registry(files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("escouade-tui-registry-{}", new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, content) in files {
            std::fs::write(dir.join(name), content).unwrap();
        }
        dir
    }

    fn entry(session: &str, entrypoint: &str) -> String {
        json!({ "pid": 1, "sessionId": session, "kind": "interactive", "entrypoint": entrypoint,
            "status": "idle" })
        .to_string()
    }

    #[test]
    fn the_registry_lists_the_interface_of_a_session_by_its_cli_entrypoint() {
        let dir = registry(&[
            // The stream's process of the same session, and other sessions' interfaces.
            ("11.json", &entry("s1", "sdk-cli")),
            ("12.json", &entry("s2", "cli")),
            ("13.json", &entry("s1", "cli")),
        ]);
        let found = find_registered(&dir, "s1", None).unwrap();
        assert_eq!(found.file, dir.join("13.json"));
        assert!(!found.linked);
        assert_eq!(find_registered(&dir, "s3", None), None);
        // Not there yet: the folder itself may be missing.
        assert_eq!(find_registered(&dir.join("absent"), "s1", None), None);
    }

    #[test]
    fn the_registry_tells_a_link_by_a_non_empty_bridge_session_id() {
        let with = |bridge: Value| {
            let mut e: Value = serde_json::from_str(&entry("s1", "cli")).unwrap();
            e["bridgeSessionId"] = bridge;
            e.to_string()
        };
        for (bridge, linked) in [
            (json!("cse_abc"), true),
            (json!(""), false),
            (json!(null), false),
            (json!(7), false),
        ] {
            let dir = registry(&[("5.json", &with(bridge.clone()))]);
            let found = find_registered(&dir, "s1", None).unwrap();
            assert_eq!(found.linked, linked, "{bridge}");
        }
        // Absent: not linked.
        let dir = registry(&[("5.json", &entry("s1", "cli"))]);
        assert!(!find_registered(&dir, "s1", None).unwrap().linked);
    }

    #[test]
    fn the_registry_ignores_what_is_not_a_whole_file_of_a_pid_without_panicking() {
        let dir = registry(&[
            // The key of a box not answered, whatever it holds.
            ("14.0a1b2c.key", &entry("s1", "cli")),
            ("notes.json", &entry("s1", "cli")),
            ("15.txt", &entry("s1", "cli")),
            // Being written, empty, not JSON, not an object, binary.
            ("16.json", "{\"pid\":16,\"sessionId\":\"s1\",\"entry"),
            ("17.json", ""),
            ("18.json", "not json"),
            ("19.json", "[1, 2]"),
            ("20.json", "\u{0}\u{1}\u{2}"),
        ]);
        // A folder under a pid's name, and a file too big to be one of theirs.
        std::fs::create_dir_all(dir.join("21.json")).unwrap();
        let big = format!(
            "{{\"sessionId\":\"s1\",\"entrypoint\":\"cli\",\"pad\":\"{}\"}}",
            "x".repeat(70 << 10)
        );
        std::fs::write(dir.join("22.json"), big).unwrap();
        assert_eq!(find_registered(&dir, "s1", None), None);
        // The file that completes is found at the next look.
        std::fs::write(dir.join("16.json"), entry("s1", "cli")).unwrap();
        assert_eq!(
            find_registered(&dir, "s1", None).unwrap().file,
            dir.join("16.json")
        );
    }

    #[test]
    fn once_the_interfaces_file_is_known_only_it_is_read_again() {
        let dir = registry(&[("30.json", &entry("s1", "cli"))]);
        let file = dir.join("30.json");
        let found = find_registered(&dir, "s1", None).unwrap();
        assert_eq!(found.file, file);
        // It gains its link; the session id is not asked for again (a `/clear` changes it).
        let mut e: Value = serde_json::from_str(&entry("s9", "cli")).unwrap();
        e["bridgeSessionId"] = json!("cse_abc");
        std::fs::write(&file, e.to_string()).unwrap();
        assert!(find_registered(&dir, "s1", Some(&file)).unwrap().linked);
        // A moment where it cannot be read changes nothing known (None: look again later).
        std::fs::write(&file, "{\"pid\":3,").unwrap();
        assert_eq!(find_registered(&dir, "s1", Some(&file)), None);
        std::fs::remove_file(&file).unwrap();
        assert_eq!(find_registered(&dir, "s1", Some(&file)), None);
    }

    #[test]
    fn an_entry_of_the_history_is_left_out() {
        let e = json!({ "timestamp": "2026-10-12T11:15:35.144Z" });
        let at = chrono::DateTime::parse_from_rfc3339("2026-10-12T11:15:35.144Z")
            .unwrap()
            .timestamp_millis();
        assert!(before(&e, at + 1));
        assert!(!before(&e, at));
        assert!(!before(&json!({}), at));
    }

    /// An account's folder with the transcript of each session in `sessions`.
    fn folder_with(name: &str, sessions: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("escouade-tui-{name}-{}", new_id()));
        let project = dir.join("projects").join("-work-demo");
        std::fs::create_dir_all(&project).unwrap();
        for s in sessions {
            std::fs::write(project.join(format!("{s}.jsonl")), "{}\n").unwrap();
        }
        dir
    }

    fn nobody_else() -> Vec<(String, PathBuf)> {
        panic!("the other accounts are looked at only when the session is not in the agent's own")
    }

    #[test]
    fn an_agent_with_no_session_starts_a_fresh_one_without_looking_anywhere() {
        let own = folder_with("fresh", &[]);
        assert_eq!(
            check_session(&meta(), &own, nobody_else),
            SessionCheck::Fresh
        );
    }

    #[test]
    fn a_session_whose_transcript_is_in_the_accounts_folder_is_resumed() {
        let own = folder_with("found", &["s1"]);
        let m = AgentMeta {
            session_id: Some("s1".into()),
            ..meta()
        };
        assert_eq!(check_session(&m, &own, nobody_else), SessionCheck::Found);
    }

    #[test]
    fn a_session_in_another_accounts_folder_is_told_with_that_account() {
        let own = folder_with("own", &[]);
        let nothing = folder_with("nothing", &["s0"]);
        let there = folder_with("there", &["s1"]);
        let also = folder_with("also", &["s1"]);
        let m = AgentMeta {
            session_id: Some("s1".into()),
            ..meta()
        };
        let others = || {
            vec![
                ("Rien".to_string(), nothing.clone()),
                ("Là".to_string(), there.clone()),
                ("Aussi".to_string(), also.clone()),
            ]
        };
        assert_eq!(
            check_session(&m, &own, others),
            SessionCheck::InOtherAccount {
                account: "Là".into(),
                dir: there.clone()
            }
        );
    }

    #[test]
    fn a_session_found_nowhere_is_missing() {
        let own = folder_with("own-m", &["other"]);
        let elsewhere = folder_with("else-m", &["another"]);
        let m = AgentMeta {
            session_id: Some("s1".into()),
            // The session's own comes first: the original is not asked for.
            fork_of: Some("another".into()),
            ..meta()
        };
        assert_eq!(
            check_session(&m, &own, || vec![("Autre".to_string(), elsewhere.clone())]),
            SessionCheck::Missing
        );
        // A folder that is not even there.
        let gone = std::env::temp_dir().join(format!("escouade-tui-gone-{}", new_id()));
        assert_eq!(check_session(&m, &gone, Vec::new), SessionCheck::Missing);
    }

    #[test]
    fn a_copy_is_checked_by_its_originals_transcript() {
        let own = folder_with("copy-own", &["s0"]);
        let there = folder_with("copy-there", &["s9"]);
        let copy = |original: &str| AgentMeta {
            session_id: None,
            fork_of: Some(original.into()),
            fork_at: Some("e1".into()),
            ..meta()
        };
        assert_eq!(
            check_session(&copy("s0"), &own, nobody_else),
            SessionCheck::Found
        );
        assert_eq!(
            check_session(&copy("s9"), &own, || vec![(
                "Là".to_string(),
                there.clone()
            )]),
            SessionCheck::InOtherAccount {
                account: "Là".into(),
                dir: there.clone()
            }
        );
        assert_eq!(
            check_session(&copy("s7"), &own, || vec![(
                "Là".to_string(),
                there.clone()
            )]),
            SessionCheck::Missing
        );
    }

    #[test]
    fn a_session_given_up_is_replaced_by_a_new_one_in_the_arguments() {
        let f = Path::new("/data/tui/a1.json");
        for lost in [
            AgentMeta {
                session_id: Some("s1".into()),
                last_entry: Some("e9".into()),
                ..meta()
            },
            // A copy whose original's session is gone.
            AgentMeta {
                fork_of: Some("s0".into()),
                fork_at: Some("e1".into()),
                ..meta()
            },
        ] {
            let dir = std::env::temp_dir().join(format!("escouade-tui-forget-{}", new_id()));
            let mut rt = crate::agent::AgentRt::new(lost, &dir);
            rt.forget_session();
            let a = tui_args(&rt.meta, Some("s-new"), None, f);
            assert!(a.contains(&"--session-id=s-new".to_string()), "{a:?}");
            assert!(
                !a.iter()
                    .any(|x| x.starts_with("--resume") || x == "--fork-session"),
                "{a:?}"
            );
            assert_eq!(
                (
                    &rt.meta.session_id,
                    &rt.meta.last_entry,
                    &rt.meta.fork_of,
                    &rt.meta.fork_at
                ),
                (&None, &None, &None, &None)
            );
        }
    }

    #[test]
    fn an_exit_is_quick_when_it_comes_soon_and_is_no_wish_of_the_user() {
        let said = b"\x1b[31mNo conversation found with session ID: s1\x1b[0m\r\n";
        let secs = Duration::from_secs;
        // /exit (code 0) at once: the user's wish, nothing to hold against the interface.
        let ok = classify_exit(secs(3), Some(0), said);
        assert_eq!(
            ok,
            TuiExit {
                code: Some(0),
                quick: false,
                session_lost: false
            }
        );
        // Stopped at once with an error, and nothing about a session.
        let crashed = classify_exit(secs(3), Some(1), b"boom");
        assert_eq!(
            crashed,
            TuiExit {
                code: Some(1),
                quick: true,
                session_lost: false
            }
        );
        // Killed (no code) at once: quick too.
        assert!(classify_exit(secs(3), None, b"").quick);
        // The same after a long while: it lived, whatever it said.
        let late = classify_exit(secs(30), Some(1), said);
        assert_eq!((late.quick, late.session_lost), (false, false));
        // The session refused, at once.
        let lost = classify_exit(secs(3), Some(1), said);
        assert_eq!((lost.quick, lost.session_lost), (true, true));
        // The limit itself: QUICK_EXIT is the first moment that is no longer quick.
        assert!(classify_exit(QUICK_EXIT - Duration::from_millis(1), Some(1), b"").quick);
        assert!(!classify_exit(QUICK_EXIT, Some(1), b"").quick);
    }

    #[test]
    fn the_first_bytes_of_the_output_are_kept_up_to_a_cap() {
        let mut kept = Vec::new();
        keep_output(&mut kept, b"hello ");
        keep_output(&mut kept, &vec![b'x'; EXIT_OUTPUT_KEPT]);
        assert_eq!(kept.len(), EXIT_OUTPUT_KEPT);
        assert!(kept.starts_with(b"hello xxx"));
        keep_output(&mut kept, b"more");
        assert_eq!(kept.len(), EXIT_OUTPUT_KEPT);
    }

    #[test]
    fn an_automatic_start_is_refused_after_two_quick_exits_in_a_row_a_manual_one_never() {
        assert!(!starts_too_fast(false, 0));
        assert!(!starts_too_fast(false, 1));
        assert!(starts_too_fast(false, 2));
        assert!(starts_too_fast(false, 7));
        assert!(!starts_too_fast(true, 7));
    }

    #[test]
    fn a_session_is_looked_for_in_the_accounts_of_the_same_side_only() {
        let (own_dir, a_dir, b_dir) = (
            folder_with("o-own", &[]),
            folder_with("o-a", &[]),
            folder_with("o-b", &[]),
        );
        let account = |id: &str, name: &str, dir: &Path| crate::model::Account {
            id: id.into(),
            name: name.into(),
            config_dir: dir.to_string_lossy().into(),
            ..Default::default()
        };
        let own = account("own", "Propre", &own_dir);
        let settings = Settings {
            accounts: vec![
                accounts::principal(),
                own.clone(),
                account("a", "A", &a_dir),
                account("b", "B (WSL)", &b_dir),
                // The same folder as the agent's own: nothing to copy from it.
                account("twin", "Jumeau", &own_dir),
            ],
            claude_path: crate::core_tests::fake_cli(),
            ..Default::default()
        };
        let others = other_accounts(&settings, &own, |a| a.id == "b" || a.id == "x");
        let names: Vec<&str> = others.iter().map(|(n, _)| n.as_str()).collect();
        // Principal (Windows side) and A; not B (WSL while the agent's account is not), nor itself.
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.contains(&"A"), "{names:?}");
        assert!(others.iter().any(|(n, d)| n == "A" && *d == a_dir));
        // The agent's account in WSL: only the accounts there are candidates.
        let in_wsl = other_accounts(&settings, &own, |a| a.id == "own" || a.id == "b");
        let names: Vec<&str> = in_wsl.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["B (WSL)"], "{names:?}");
    }
}
