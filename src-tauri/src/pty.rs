//! Integrated terminals: real pseudo-consoles (ConPTY on Windows, a pty on macOS) streamed to
//! xterm.js.

use crate::i18n::{self, Lang};
use crate::job::Job;
use crate::model::{now_ms, Settings};
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

// A shell of the machine, in `escouade-plugins` (a plugin's command runs in one).
pub use escouade_plugins::events::ShellInfo;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermInfo {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub shell: String,
}

/// What a terminal holds, shared (`Arc`) so that the map of terminals is locked for a lookup and
/// nothing else: what waits on a terminal (a resize) holds its own lock, never the map's.
struct Term {
    info: TermInfo,
    /// Locked for a resize. Taken (`close_console`) by whoever ends the terminal, `kill` or the
    /// thread waiting for its program: dropping it closes the pseudo-console, and that is theirs to
    /// do, not the business of whichever thread holds the last `Arc<Term>` (a reader answering a
    /// cursor query, the window's main thread in a write).
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    /// The terminal's input, written by its own thread (`run_input`): a program that reads
    /// nothing makes that thread wait, not the caller of `write`.
    input: mpsc::Sender<Input>,
    /// Bytes queued and not written yet: `write` refuses what would take them past `MAX_PENDING`.
    pending: Arc<AtomicUsize>,
    /// Locked to kill.
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    /// The terminal's whole process tree, including what it starts outside its console: killed
    /// when the terminal goes (all of it, or what shows no window: see `Started::spare_windows`).
    job: Option<Job>,
    /// A launch command (or a step of a test launch), not an interactive shell.
    launch: bool,
    /// See `Started::spare_windows`.
    spare_windows: bool,
    /// See `Started::detached_survive`.
    detached_survive: bool,
    /// When it last had input or output (ms).
    io_at: Arc<AtomicI64>,
    /// Its last writes and when they were queued, written and answered.
    writes: Arc<WriteLog>,
    /// When the log last got a slow echo of the window (ms), `i64::MIN` for never.
    slow_echo_at: AtomicI64,
    /// How many of the window's writes were typed by its user (`typing`), not answers of the
    /// terminal.
    typed: Arc<AtomicU64>,
}

/// A write waiting for the input thread.
struct Input {
    data: Vec<u8>,
    /// Its number in the terminal's `WriteLog`.
    seq: u64,
    /// When `PtyManager::write` was called, and when it had queued this (µs).
    called_us: i64,
    queued_us: i64,
    /// Told (once) when the input thread has written this, for `write_and_wait`. Dropped without a
    /// word when the terminal's input ends first.
    ack: Option<mpsc::Sender<()>>,
}

/// `ESCOUADE_TERM_TRACE` set: each write to a terminal, and the first output after it, go to the
/// log with their times (µs since the epoch), to measure a keystroke's way (docs/SPEC.md, *Terminaux*).
fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("ESCOUADE_TERM_TRACE").is_some())
}

fn now_us() -> i64 {
    chrono::Utc::now().timestamp_micros()
}

/// Whether what the window sends to a terminal was typed by its user, and may change what the
/// program holds as its input: not what the terminal sends on its own (its answers to the
/// program's queries, the focus that comes and goes, the mouse), nor Escape alone. A key that
/// moves or recalls (an arrow, Alt+a letter) is typing.
pub(crate) fn typing(data: &[u8]) -> bool {
    let mut s = data;
    while !s.is_empty() {
        let len = match s {
            // A mouse report of the old kind: three bytes follow, whatever they are.
            [0x1b, b'[', b'M', _, _, _, ..] => 6,
            [0x1b, b'[', rest @ ..] => {
                let Some(end) = rest.iter().position(|b| (0x40..=0x7e).contains(b)) else {
                    return true;
                };
                let (body, last) = (&rest[..end], rest[end]);
                let focus = body.is_empty() && matches!(last, b'I' | b'O');
                let mouse = body.first() == Some(&b'<') && matches!(last, b'M' | b'm');
                // A cursor position, the attributes, a status, the window, a mode.
                let answer = matches!(last, b'R' | b'c' | b'n' | b't' | b'y');
                if !(focus || mouse || answer) {
                    return true;
                }
                end + 3
            }
            // A string (a colour, a setting asked for), up to BEL or ESC \.
            [0x1b, b']' | b'P' | b'_' | b'^', rest @ ..] => {
                let Some(end) = rest.iter().enumerate().position(|(i, b)| {
                    *b == 0x07 || (*b == 0x1b && rest.get(i + 1) == Some(&b'\\'))
                }) else {
                    return true;
                };
                end + 2 + if rest[end] == 0x07 { 1 } else { 2 }
            }
            // Escape alone (or before another sequence).
            [0x1b] | [0x1b, 0x1b, ..] => 1,
            _ => return true,
        };
        s = &s[len.min(s.len())..];
    }
    false
}

/// How many writes a terminal remembers (`WriteLog`).
const RECENT_WRITES: usize = 64;

/// One write to a terminal and the times (µs since the epoch) of its way: queued by `write`,
/// started and done by the input thread, and first answered by the program.
#[derive(Debug, Clone, PartialEq)]
struct WriteRecord {
    /// Its number among the terminal's writes, to find it again once the ring has moved on.
    seq: u64,
    bytes: usize,
    queued_us: i64,
    /// The input thread begins to write it: its echo cannot come before.
    started_us: Option<i64>,
    written_us: Option<i64>,
    /// The first output read after the write began.
    first_output_us: Option<i64>,
}

/// How many reads of a terminal's output it remembers (`WriteLog`).
const RECENT_READS: usize = 512;

/// One `read` of a terminal's output. Each read becomes exactly one message to the window, of the
/// same bytes and in the same order: the window's count of the bytes it received, since the
/// terminal opened, is the `end` of the read its last message was.
#[derive(Debug, Clone, Copy, PartialEq)]
struct OutputRead {
    /// The bytes read since the terminal opened, this read's included.
    end: u64,
    /// When it was read (µs since the epoch).
    at_us: i64,
}

#[derive(Default)]
struct WriteLogState {
    next: u64,
    recent: VecDeque<WriteRecord>,
    /// The output bytes read since the terminal opened.
    read_bytes: u64,
    /// The last reads, oldest first: their `end`s only grow.
    reads: VecDeque<OutputRead>,
}

/// The last writes to a terminal and the last reads of its output, always on: a slow echo the
/// window reports is looked up here to say where the time went. Its lock is only ever held for a
/// few assignments.
#[derive(Default)]
struct WriteLog {
    state: Mutex<WriteLogState>,
}

impl WriteLogState {
    /// The record of write number `seq`, if the ring still holds it (it holds consecutive ones).
    fn get_mut(&mut self, seq: u64) -> Option<&mut WriteRecord> {
        let first = self.recent.front()?.seq;
        self.recent
            .get_mut(usize::try_from(seq.checked_sub(first)?).ok()?)
    }
}

impl WriteLog {
    /// A write of `bytes` queued at `at`; returns its number.
    fn queued(&self, bytes: usize, at: i64) -> u64 {
        let mut state = self.state.lock();
        let seq = state.next;
        state.next += 1;
        if state.recent.len() == RECENT_WRITES {
            state.recent.pop_front();
        }
        state.recent.push_back(WriteRecord {
            seq,
            bytes,
            queued_us: at,
            started_us: None,
            written_us: None,
            first_output_us: None,
        });
        seq
    }

    fn update(&self, seqs: impl IntoIterator<Item = u64>, set: impl Fn(&mut WriteRecord)) {
        let mut state = self.state.lock();
        for seq in seqs {
            if let Some(record) = state.get_mut(seq) {
                set(record);
            }
        }
    }

    /// The input thread begins to write these, at `at`.
    fn started(&self, seqs: impl IntoIterator<Item = u64>, at: i64) {
        self.update(seqs, |r| r.started_us = Some(at));
    }

    /// These are written (and flushed), at `at`.
    fn written(&self, seqs: impl IntoIterator<Item = u64>, at: i64) {
        self.update(seqs, |r| r.written_us = Some(at));
    }

    /// A read of `bytes` of output, at `at`. It is the first output of the writes that began
    /// before it and had none: not one read before (it is only reported now), and a write counts
    /// as soon as it began, not when its call returns: the echo may be read first. It also
    /// becomes the read that the window's count of bytes received will name (`read_ending_at`).
    fn output(&self, at: i64, bytes: usize) {
        let mut state = self.state.lock();
        for r in state.recent.iter_mut() {
            if r.first_output_us.is_none() && r.started_us.is_some_and(|started| started <= at) {
                r.first_output_us = Some(at);
            }
        }
        // Nothing read, nothing for the window to count (and two reads must never share an end).
        if bytes > 0 {
            state.read_bytes += bytes as u64;
            let end = state.read_bytes;
            if state.reads.len() == RECENT_READS {
                state.reads.pop_front();
            }
            state.reads.push_back(OutputRead { end, at_us: at });
        }
    }

    /// The read after which `end` bytes of output had been read since the terminal opened, if it
    /// is among the last ones: a count that ends no read (or one that left the ring) finds none.
    fn read_ending_at(&self, end: u64) -> Option<OutputRead> {
        let state = self.state.lock();
        let i = state.reads.binary_search_by_key(&end, |r| r.end).ok()?;
        state.reads.get(i).copied()
    }

    /// The write number `seq` (the one `PtyManager::write` returned), if the ring still holds it.
    fn get(&self, seq: u64) -> Option<WriteRecord> {
        self.state.lock().get_mut(seq).map(|r| r.clone())
    }

    #[cfg(test)]
    fn all(&self) -> Vec<WriteRecord> {
        self.state.lock().recent.iter().cloned().collect()
    }
}

/// What the window measured of a terminal (src/lib/terminals.ts), for the log (app.log).
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TermReport {
    /// What xterm.js draws with: "webgl" or "dom"; `detail`: the GPU's name as WebGL gives it,
    /// or why WebGL is not used (no WebGL 2, context lost).
    Renderer {
        renderer: String,
        detail: Option<String>,
    },
    /// The echo of the write number `seq` took longer than the window's threshold to show:
    /// `sent_to_received` is how long after the write left the window its echo came in,
    /// `received_to_painted` how long after that it was painted (ms, measured on the window's own
    /// clock, so durations only: never compared with the backend's). `sent_to_acked` is how long
    /// after the write left the window the backend's answer to it came back (ms, the same).
    /// `output_end` is the count of output bytes the window had received, since the terminal
    /// opened, up to and including the chunk with the echo: the backend finds the read of that
    /// chunk by its place in the stream, not by comparing clocks (`WriteLog::read_ending_at`).
    SlowEcho {
        seq: u64,
        bytes: usize,
        sent_to_received: f64,
        received_to_painted: f64,
        #[serde(default)]
        sent_to_acked: Option<f64>,
        #[serde(default)]
        output_end: Option<u64>,
    },
}

/// The log line for a slow echo of `who`, with where the time went when the backend remembers the
/// write. `echo_read_us` is when the backend read the chunk with the echo, if it found it
/// (`output_end`): the first output after a write is often not the echo (ConPTY hides the cursor
/// first, the letter comes in a later chunk), so with the echo's own read the line can tell which
/// side the time went to. The window's durations and the backend's times are two clocks: only the
/// backend's own differences, and the window's own, are ever subtracted from each other.
fn slow_echo_line(
    who: &str,
    bytes: usize,
    sent_to_received: f64,
    received_to_painted: f64,
    sent_to_acked: Option<f64>,
    write: Option<&WriteRecord>,
    echo_read_us: Option<i64>,
) -> String {
    // No negative step, whatever the rounding of the two sides.
    let ms = |ms: f64| ms.max(0.0).round() as i64;
    let between = |from: i64, to: i64| ms((to - from) as f64 / 1000.0);
    let in_window = format!(
        "in the window {} ms after the write left it",
        ms(sent_to_received)
    );
    let painted = format!("painted {} ms later", ms(received_to_painted));
    let steps = match write {
        None => format!("backend times unknown, {in_window}, {painted}"),
        Some(w) => {
            let written = match w.written_us {
                Some(done) => format!(
                    "written {} ms after the backend got it",
                    between(w.queued_us, done)
                ),
                None => "not written yet".to_string(),
            };
            match (w.written_us, w.first_output_us) {
                (Some(done), Some(output)) => {
                    // The echo's own read when it is known, else the first output stands for it.
                    let (echo, seen_at) = match echo_read_us {
                        Some(at) => (format!("the echo {} ms after", between(done, at)), at),
                        None => ("the echo not identified".to_string(), output),
                    };
                    let answer = sent_to_acked
                        .map(|a| format!(" (answer to the write {} ms)", ms(a)))
                        .unwrap_or_default();
                    format!(
                        "{written}, first output {} ms after, {echo}, to and from the window {} \
                         ms{answer}, {painted}",
                        between(done, output),
                        ms(sent_to_received - (seen_at - w.queued_us) as f64 / 1000.0),
                    )
                }
                (_, output) => {
                    let output = if output.is_some() { "seen" } else { "not seen" };
                    format!("{written}, first output {output}, {in_window}, {painted}")
                }
            }
        }
    };
    format!(
        "{who}: echo of a {bytes}-byte write shown after {} ms: {steps}",
        ms(sent_to_received + received_to_painted)
    )
}

/// What runs in the terminals, as an automatic restart for an update goes.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Activity {
    /// Launch commands (and steps of test launches) still running.
    pub launches: usize,
    /// When an interactive terminal last had input or output (ms), if one is open.
    pub shell_io_at: Option<i64>,
}

#[derive(Default, Clone)]
pub struct PtyManager {
    terms: Arc<Mutex<HashMap<String, Arc<Term>>>>,
}

/// What a terminal starts.
#[derive(Clone, Copy)]
struct Started {
    /// A launch command (or a step of a test launch), not an interactive terminal.
    launch: bool,
    /// What it starts, outside its console too, is killed with it (a `Job`).
    job: bool,
    /// What shows a window (a browser, an editor), with what it started, is not killed with it:
    /// closing the terminal (or, for `detached_survive` false, the end of its program) ends only
    /// the programs that show none, and the Job kills nothing by itself when it is dropped (the
    /// app's quit, crash or restart for an update). Otherwise it kills everything.
    spare_windows: bool,
    /// When its program ends by itself (an `exit`), what the user detached keeps running: only
    /// closing the terminal ends the tree (but for the windows, see `spare_windows`). Otherwise
    /// the end of the program ends what it started.
    detached_survive: bool,
}

/// Where shells are looked for; from the environment in the app, fake folders in tests.
#[cfg(windows)]
pub struct ShellRoots {
    pub path: Vec<PathBuf>,
    pub program_files: Option<PathBuf>,
    pub local_app_data: Option<PathBuf>,
    pub system_root: Option<PathBuf>,
}

#[cfg(windows)]
impl ShellRoots {
    fn from_env() -> Self {
        let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
        ShellRoots {
            path: std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            program_files: var("ProgramFiles").or_else(|| Some(PathBuf::from(r"C:\Program Files"))),
            local_app_data: var("LOCALAPPDATA"),
            system_root: var("SystemRoot").or_else(|| Some(PathBuf::from(r"C:\Windows"))),
        }
    }

    fn on_path(&self, exe: &str) -> Option<PathBuf> {
        self.path.iter().map(|d| d.join(exe)).find(|p| installed(p))
    }
}

/// Also true for the 0-byte "app execution alias" reparse points of Microsoft Store apps.
#[cfg(windows)]
fn installed(p: &Path) -> bool {
    p.is_file()
}

#[cfg(windows)]
fn first_installed(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|p| installed(p))
}

#[cfg(windows)]
pub fn detect_shells(s: &Settings) -> Vec<ShellInfo> {
    detect_shells_in(&ShellRoots::from_env(), s)
}

/// The user's login shell first (zsh by default on macOS), then the other usual ones.
#[cfg(not(windows))]
pub fn detect_shells(s: &Settings) -> Vec<ShellInfo> {
    let path: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let login = std::env::var_os("SHELL").map(PathBuf::from);
    detect_unix_shells(login.as_deref(), &path, s)
}

/// Unix shells: `login` (from $SHELL) first, then zsh, bash and fish, from /bin or the PATH.
#[cfg(not(windows))]
pub fn detect_unix_shells(login: Option<&Path>, path: &[PathBuf], s: &Settings) -> Vec<ShellInfo> {
    const KNOWN: [(&str, &str); 4] = [
        ("zsh", "zsh"),
        ("bash", "bash"),
        ("fish", "fish"),
        ("sh", "sh"),
    ];
    let find = |name: &str| -> Option<PathBuf> {
        if name == "bash" && !s.bash_path.is_empty() {
            return Some(PathBuf::from(&s.bash_path)).filter(|p| p.is_file());
        }
        [Path::new("/bin"), Path::new("/usr/bin")]
            .into_iter()
            .chain(path.iter().map(PathBuf::as_path))
            .map(|d| d.join(name))
            .find(|p| p.is_file())
    };
    let mut found: Vec<(&str, &str, PathBuf)> = Vec::new();
    if let Some(l) = login.filter(|l| l.is_file()) {
        let name = l.file_name().unwrap_or_default().to_string_lossy();
        if let Some((id, label)) = KNOWN.iter().find(|(id, _)| *id == name) {
            found.push((id, label, l.to_path_buf()));
        }
    }
    for (id, label) in KNOWN.iter().take(3) {
        if !found.iter().any(|f| f.0 == *id) {
            if let Some(p) = find(id) {
                found.push((id, label, p));
            }
        }
    }
    if found.is_empty() {
        if let Some(p) = find("sh") {
            found.push(("sh", "sh", p));
        }
    }
    found
        .into_iter()
        .map(|(id, label, p)| ShellInfo {
            id: id.into(),
            label: label.into(),
            path: p.to_string_lossy().into(),
        })
        .collect()
}

#[cfg(windows)]
pub fn detect_shells_in(r: &ShellRoots, s: &Settings) -> Vec<ShellInfo> {
    let mut out = Vec::new();
    let shell = |id: &str, label: &str, p: PathBuf| ShellInfo {
        id: id.into(),
        label: label.into(),
        path: p.to_string_lossy().into(),
    };
    // PowerShell 7: PATH, Program Files (also when not on the PATH), then the Store's alias.
    let pwsh = if !s.pwsh_path.is_empty() {
        Some(PathBuf::from(&s.pwsh_path)).filter(|p| installed(p))
    } else {
        r.on_path("pwsh.exe").or_else(|| {
            let pf = r.program_files.iter().flat_map(|pf| {
                ["7", "7-preview"].map(|v| pf.join("PowerShell").join(v).join("pwsh.exe"))
            });
            let store = r
                .local_app_data
                .iter()
                .map(|d| d.join(r"Microsoft\WindowsApps\pwsh.exe"));
            first_installed(pf.chain(store))
        })
    };
    match pwsh {
        Some(p) => out.push(shell("pwsh", "PowerShell", p)),
        // Windows PowerShell 5.1 ships with every Windows: the fallback when 7 is missing.
        None => {
            let builtin = r
                .system_root
                .iter()
                .map(|w| w.join(r"System32\WindowsPowerShell\v1.0\powershell.exe"));
            if let Some(p) = first_installed(builtin) {
                out.push(shell("powershell", "Windows PowerShell", p));
            }
        }
    }
    let bash = if !s.bash_path.is_empty() {
        Some(PathBuf::from(&s.bash_path)).filter(|p| installed(p))
    } else {
        let from_git = r.on_path("git.exe").and_then(|g| {
            let root = g.parent()?.parent()?.to_path_buf();
            first_installed([
                root.join("bin").join("bash.exe"),
                root.join("usr").join("bin").join("bash.exe"),
            ])
        });
        from_git.or_else(|| {
            first_installed(
                r.program_files
                    .iter()
                    .map(|pf| pf.join(r"Git\bin\bash.exe")),
            )
        })
    };
    if let Some(p) = bash {
        out.push(shell("bash", "Git Bash", p));
    }
    if let Some(wsl) = first_installed(r.system_root.iter().map(|w| w.join(r"System32\wsl.exe"))) {
        let label = if s.wsl_distro.is_empty() {
            "WSL".to_string()
        } else {
            format!("WSL ({})", s.wsl_distro)
        };
        out.push(shell("wsl", &label, wsl));
    }
    out
}

/// PowerShell script running `command` then ending with its exit code: on its own, PowerShell
/// ends with 1 for any failure, whatever the failing program's code.
pub(crate) fn with_exit_code(command: &str) -> String {
    format!("{command}\nif (-not $?) {{ if ($LASTEXITCODE) {{ exit $LASTEXITCODE }} exit 1 }}")
}

/// The arguments making zsh run `command` as the user's terminal would: a login shell, interactive
/// too so that it reads ~/.zshrc, where rbenv, nvm, asdf… set themselves up (a login shell alone
/// runs the system's Ruby or Node: macOS's /etc/zprofile puts the system folders first in the
/// PATH). Without job control (`+m`), which would give what it starts a process group of its own,
/// out of reach of the Job stopping it.
pub(crate) fn zsh_command_args(command: &str) -> [&str; 5] {
    ["-l", "-i", "+m", "-c", command]
}

/// Refused: the folder `sub` a launch command names is not in the project.
fn missing_folder(lang: Lang, sub: &str) -> String {
    tr_in!(
        lang,
        "Le dossier « {sub} » n'existe pas dans le projet",
        "The folder “{sub}” doesn’t exist in the project"
    )
}

/// Refused: the terminal written to or resized is gone (its shell ended, the tab closed).
fn closed(lang: Lang) -> anyhow::Error {
    anyhow!(tr_in!(lang, "terminal fermé", "terminal closed"))
}

fn not_reading(lang: Lang) -> anyhow::Error {
    anyhow!(tr_in!(
        lang,
        "le terminal ne lit plus son entrée",
        "the terminal no longer reads its input"
    ))
}

/// Working folder of a launch command: the project's, or one of its folders.
pub fn run_cwd(project: &str, sub: &str) -> Result<String> {
    let sub = sub.trim();
    if sub.is_empty() {
        return Ok(project.to_string());
    }
    let dir = Path::new(project).join(sub);
    if !dir.is_dir() {
        anyhow::bail!(missing_folder(i18n::ui(), sub));
    }
    Ok(dir.to_string_lossy().into_owned())
}

/// What tells Escouade's browser opener the user's own `BROWSER` (empty: none).
#[cfg(unix)]
pub const USER_BROWSER_VAR: &str = "ESCOUADE_BROWSER";

/// Escouade's browser opener (`open_browser_apart`): the user's browser, else the system's
/// opener, in a session of its own (macOS has no `setsid`: Perl's), reading nothing of Claude
/// Code's. Without the user's, `BROWSER` goes: `xdg-open` would run the opener again.
#[cfg(unix)]
const BROWSER_OPENER: &str = r#"#!/bin/sh
# Escouade: opens the page Claude Code asks for in a session of its own, so that the browser it
# starts outlives the terminal of the sign-in, whose end takes what runs in it.
browser=$ESCOUADE_BROWSER
unset ESCOUADE_BROWSER
if [ -n "$browser" ]; then
  BROWSER=$browser
  export BROWSER
  set -- "$browser" "$@"
else
  unset BROWSER
  # Under WSLg, xdg-open usually has no browser to open: Windows' does it (wslu's wslview, else
  # Windows' own handler through WSL's interop).
  if [ "$(uname)" = Darwin ]; then set -- open "$@"
  elif [ -n "$WSL_DISTRO_NAME" ] && command -v wslview >/dev/null 2>&1; then set -- wslview "$@"
  elif [ -n "$WSL_DISTRO_NAME" ] && command -v rundll32.exe >/dev/null 2>&1; then
    set -- rundll32.exe url.dll,FileProtocolHandler "$@"
  else set -- xdg-open "$@"; fi
fi
# Its input is not Claude Code's.
if command -v setsid >/dev/null 2>&1; then
  exec setsid "$@" </dev/null
fi
# A group leader cannot start a session: it forks first, as setsid does.
if command -v perl >/dev/null 2>&1; then
  exec perl -MPOSIX -e '
    if (POSIX::setsid() < 0) { fork and exit 0; POSIX::setsid(); }
    exec { $ARGV[0] } @ARGV;
    print STDERR "open-browser: $ARGV[0]: $!\n";
    exit 127;
  ' -- "$@" </dev/null
fi
exec "$@" </dev/null
"#;

/// Has a program of a terminal (Claude Code signing in) open its browser through Escouade's
/// opener, written at `opener`, out of the terminal. On Unix, when the program ends, the kernel
/// hangs up the terminal's foreground process group (SIGHUP), whose processes die of it, and that
/// group is then killed: a browser started there (by `xdg-open`, when none runs yet) would go with
/// them, and nothing tells a process with a window from one without. The opener runs the user's
/// own `BROWSER`, kept in `env` for it.
#[cfg(unix)]
pub fn open_browser_apart(opener: &Path, env: &mut Vec<(String, String)>) -> std::io::Result<()> {
    write_executable(opener, BROWSER_OPENER)?;
    // Claude Code would open no browser at all.
    if !runnable(opener) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("{} cannot be run (noexec?)", opener.display()),
        ));
    }
    let opener = opener.to_string_lossy().into_owned();
    let var = |k: &str| {
        env.iter()
            .rev()
            .find(|(name, _)| name == k)
            .map(|(_, v)| v.clone())
            .or_else(|| std::env::var(k).ok())
    };
    // Already the opener (Escouade started from a sign-in's terminal): the user's is the one it
    // was given.
    let user = match var("BROWSER") {
        Some(b) if b == opener => var(USER_BROWSER_VAR),
        b => b,
    };
    env.retain(|(k, _)| k != "BROWSER" && k != USER_BROWSER_VAR);
    env.push(("BROWSER".into(), opener));
    env.push((USER_BROWSER_VAR.into(), user.unwrap_or_default()));
    Ok(())
}

/// On Windows, nothing to do: the Job of the terminal spares what shows a window.
#[cfg(not(unix))]
pub fn open_browser_apart(_opener: &Path, _env: &mut Vec<(String, String)>) -> std::io::Result<()> {
    Ok(())
}

/// Whether this process may run the file `path`: not on a file system mounted `noexec`, whatever
/// the file's mode.
#[cfg(unix)]
fn runnable(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: plain syscall on a NUL-terminated path.
    unsafe { libc::access(path.as_ptr(), libc::X_OK) == 0 }
}

/// Writes `text` at `path`, executable by its owner alone, whole or not at all: a program may run
/// it at any time.
#[cfg(unix)]
fn write_executable(path: &Path, text: &str) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("{}-{n}.tmp", std::process::id()));
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o700)
        .open(&tmp)
        .and_then(|mut f| f.write_all(text.as_bytes()))
        .and_then(|()| std::fs::rename(&tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

/// Finds the cursor position query (`ESC [ 6 n`) in what a terminal prints, even when it is cut
/// between two reads: the end of each read is kept to join it to the next.
#[derive(Default)]
pub(crate) struct CursorQuery {
    tail: Vec<u8>,
}

impl CursorQuery {
    const QUERY: &'static [u8] = b"\x1b[6n";

    /// Whether the query ends in `read`, the next one of the terminal's output.
    pub(crate) fn feed(&mut self, read: &[u8]) -> bool {
        let mut seen = std::mem::take(&mut self.tail);
        seen.extend_from_slice(read);
        let found = seen.windows(Self::QUERY.len()).any(|w| w == Self::QUERY);
        // One byte short of the query: a whole one never waits for the next read, so it is not
        // found twice.
        let keep = seen.len().min(Self::QUERY.len() - 1);
        self.tail = seen.split_off(seen.len() - keep);
        found
    }
}

/// Ctrl+C typed in a terminal reaches its program (ESC-396): ConPTY makes it a console Ctrl+C
/// event, which a process ignores when it was started with Ctrl+C ignored, a flag every process
/// inherits from the one that started it. Escouade started in a new process group (by a launcher,
/// a test runner, a Node script with `detached`) has it, and would pass it to the shells and launch
/// commands of its terminals, which Ctrl+C would then never stop. Escouade has no console of its
/// own, so taking Ctrl+C back costs it nothing; done once, before the first terminal starts.
#[cfg(windows)]
fn let_ctrl_c_through() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // SAFETY: no handler routine; FALSE only clears the process's "Ctrl+C ignored" flag.
        if unsafe { windows_sys::Win32::System::Console::SetConsoleCtrlHandler(None, 0) } == 0 {
            log::warn!(
                "Ctrl+C ignored by the terminals' programs: {}",
                std::io::Error::last_os_error()
            );
        }
    });
}

/// Starts the thread writing to a terminal's input what `write` queues; returns the queue, and
/// the count of the bytes in it.
fn spawn_input(
    id: &str,
    writer: Box<dyn Write + Send>,
    writes: Arc<WriteLog>,
    written_at: Arc<AtomicI64>,
) -> std::io::Result<(mpsc::Sender<Input>, Arc<AtomicUsize>)> {
    let (input, queue) = mpsc::channel();
    let pending = Arc::new(AtomicUsize::new(0));
    let id = id.to_string();
    let count = pending.clone();
    std::thread::Builder::new()
        .name("pty-input".into())
        .spawn(move || run_input(&id, writer, &queue, &writes, &written_at, &count))?;
    Ok((input, pending))
}

const ESCAPE: u8 = 0x1b;

/// What a terminal's input may hold before it is written: past this, a program that reads nothing
/// (and pastes, and xterm.js's own answers piling up) would grow the app's memory without end.
const MAX_PENDING: usize = 32 * 1024 * 1024;

/// What the input thread joins in one write, at most: a bigger write alone stays whole.
const MAX_WRITE: usize = 256 * 1024;

/// Splits what the queue holds into the writes to make, in order: what is there goes in one write
/// of `MAX_WRITE` bytes at most (a bigger one alone stays whole), except a lone Escape, which goes
/// in a write of its own. Joined to what follows it, it would be read as Alt plus the next key
/// (ConPTY, then the program), when the key it is is Escape.
fn writes_to_make(items: Vec<Input>) -> Vec<Vec<Input>> {
    let mut calls: Vec<Vec<Input>> = Vec::new();
    // Whether the last call takes more: not after a lone Escape.
    let mut open = false;
    let mut size = 0;
    for item in items {
        let alone = item.data == [ESCAPE];
        let len = item.data.len();
        match calls.last_mut() {
            Some(call) if open && !alone && size + len <= MAX_WRITE => {
                size += len;
                call.push(item);
            }
            _ => {
                size = len;
                calls.push(vec![item]);
            }
        }
        open = !alone;
    }
    calls
}

/// Writes the queue to a terminal's input, in order, what is already there in one write (but for
/// a lone Escape: see `writes_to_make`). This thread is the only one to wait for the program to
/// read: a full input (a program that reads nothing, a busy console) holds back its terminal
/// alone. Ends with the queue, when the terminal is dropped, or with the pipe, when the program is
/// gone (the next `write` says the terminal is closed).
fn run_input(
    id: &str,
    mut writer: Box<dyn Write + Send>,
    queue: &mpsc::Receiver<Input>,
    writes: &WriteLog,
    written_at: &AtomicI64,
    pending: &AtomicUsize,
) {
    while let Ok(first) = queue.recv() {
        let mut batch = vec![first];
        batch.extend(queue.try_iter());
        for call in writes_to_make(batch) {
            let started = now_us();
            writes.started(call.iter().map(|w| w.seq), started);
            if tracing() {
                // Before the write: its echo may be read before the write returns.
                written_at.store(started, Ordering::Release);
            }
            let joined: Vec<u8>;
            let data: &[u8] = match call.as_slice() {
                [only] => &only.data,
                _ => {
                    joined = call.iter().flat_map(|w| w.data.iter().copied()).collect();
                    &joined
                }
            };
            let result = writer.write_all(data).and_then(|()| writer.flush());
            let done = now_us();
            writes.written(call.iter().map(|w| w.seq), done);
            if tracing() {
                for w in &call {
                    log::info!(
                        "term-trace write id={id} bytes={} called={} locked={} done={done}",
                        w.data.len(),
                        w.called_us,
                        w.queued_us
                    );
                }
            }
            if let Err(e) = result {
                // The acks go unanswered, with the writes: the callers waiting for them are told.
                // Said in the log (its kind, never what was typed): the terminal stays open, and
                // takes no more input.
                log::warn!(
                    "terminal {id}: its input can no longer be written ({:?}): it takes no more keys",
                    e.kind()
                );
                return;
            }
            pending.fetch_sub(data.len(), Ordering::AcqRel);
            for ack in call.into_iter().filter_map(|w| w.ack) {
                let _ = ack.send(());
            }
        }
    }
}

/// What the window says, as one line of the log: no control character (a line break would forge
/// a line of its own), `max` characters at most.
fn one_line(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect()
}

/// A slow echo of the window is logged once per this long, per terminal (ms).
const SLOW_ECHO_EVERY_MS: i64 = 5_000;

impl Term {
    /// What the log says of what the window `report`s at `now` (ms), if anything.
    fn report_line(&self, report: TermReport, now: i64) -> Option<(log::Level, String)> {
        let who = format!("terminal {} ({})", self.info.id, self.info.shell);
        match report {
            TermReport::Renderer { renderer, detail } => {
                let renderer = one_line(&renderer, 20);
                let detail = detail
                    .map(|d| one_line(&d, 200))
                    .filter(|d| !d.is_empty())
                    .map(|d| format!(" ({d})"))
                    .unwrap_or_default();
                Some((
                    log::Level::Info,
                    format!("{who}: renderer {renderer}{detail}"),
                ))
            }
            TermReport::SlowEcho {
                seq,
                bytes,
                sent_to_received,
                received_to_painted,
                sent_to_acked,
                output_end,
            } => {
                // The window limits them too: a guard against a loop there filling the log. One
                // atomic step, as several reports run at once: only one of them takes the slot.
                self.slow_echo_at
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |last| {
                        (now.saturating_sub(last) >= SLOW_ECHO_EVERY_MS).then_some(now)
                    })
                    .ok()?;
                let write = self.writes.get(seq);
                let echo_read = output_end.and_then(|end| self.writes.read_ending_at(end));
                let line = slow_echo_line(
                    &who,
                    bytes,
                    sent_to_received,
                    received_to_painted,
                    sent_to_acked,
                    write.as_ref(),
                    echo_read.map(|r| r.at_us),
                );
                Some((log::Level::Warn, line))
            }
        }
    }

    /// Resizes the pseudo-console; refused once it is closed.
    fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let master = self.master.lock();
        let master = master.as_ref().ok_or_else(|| closed(i18n::ui()))?;
        master
            .resize(PtySize {
                rows: rows.max(2),
                cols: cols.max(10),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow!("{e}"))
    }

    /// Closes the pseudo-console, if it still is: the end of the terminal (`kill`, or the end of
    /// its program) does it, itself and outside the map's lock. Closing it may wait for the
    /// console to be done with its output, which only the terminal's reader thread takes: that
    /// thread must never be the one to close it (by dropping the last `Arc<Term>` it holds in a
    /// `write`), nor must the window's main thread.
    fn close_console(&self) {
        let master = self.master.lock().take();
        drop(master);
    }
}

impl PtyManager {
    /// An interactive shell.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &self,
        info: TermInfo,
        shell: &ShellInfo,
        wsl_distro: &str,
        cwd: &str,
        size: (u16, u16),
        env: Vec<(String, String)>,
        on_data: impl Fn(Vec<u8>) + Send + 'static,
        on_exit: impl FnOnce(Option<u32>) + Send + 'static,
    ) -> Result<()> {
        self.spawn_inner(
            info, shell, wsl_distro, cwd, size, env, None, on_data, on_exit,
        )
    }

    /// A launch command run by `shell`: the terminal ends with it and reports its exit code.
    /// Its log is read-only, so the terminal answers the pseudo-console's cursor position query
    /// itself (ConPTY waits for it before running anything), with `cursor_row`, the log's row
    /// where the command starts: ConPTY then leaves the lines above alone, even when it repaints.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_command(
        &self,
        info: TermInfo,
        shell: &ShellInfo,
        wsl_distro: &str,
        cwd: &str,
        size: (u16, u16),
        env: Vec<(String, String)>,
        cursor_row: u16,
        command: &str,
        on_data: impl Fn(Vec<u8>) + Send + 'static,
        on_exit: impl FnOnce(Option<u32>) + Send + 'static,
    ) -> Result<()> {
        let (me, id) = (self.clone(), info.id.clone());
        let answer = format!("\x1b[{};1R", cursor_row.max(1));
        let query = Mutex::new(CursorQuery::default());
        let on_data = move |b: Vec<u8>| {
            if query.lock().feed(&b) {
                let _ = me.write(&id, answer.as_bytes());
            }
            on_data(b);
        };
        self.spawn_inner(
            info,
            shell,
            wsl_distro,
            cwd,
            size,
            env,
            Some(command),
            on_data,
            on_exit,
        )
    }

    /// An interactive terminal running `program` itself, with no shell around it (the sign-in of
    /// a Claude account, the Claude Code view): it ends with the program, and what it starts goes
    /// with it; except, with `spare_windows`, what shows a window (the browser of the sign-in). On
    /// Unix, where nothing tells it, such a program starts its browser out of the terminal
    /// (`open_browser_apart`).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_program(
        &self,
        info: TermInfo,
        program: &crate::engine::Program,
        args: &[String],
        cwd: &str,
        size: (u16, u16),
        env: Vec<(String, String)>,
        spare_windows: bool,
        on_data: impl Fn(Vec<u8>) + Send + 'static,
        on_exit: impl FnOnce(Option<u32>) + Send + 'static,
    ) -> Result<()> {
        // The terminal's variables as well: a program of WSL gets none of those set on wsl.exe.
        let terminal = [("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]
            .map(|(k, v)| (k.to_string(), v.to_string()));
        let launch = program.launch(cwd, args, &[terminal.to_vec(), env].concat())?;
        // A `.cmd` launcher (npm's claude.cmd) included: Windows runs it through cmd.exe.
        let mut cmd = CommandBuilder::new(&launch.program);
        cmd.args(&launch.args);
        let env = launch.env;
        if Path::new(cwd).is_dir() {
            cmd.cwd(cwd);
        }
        let started = Started {
            launch: false,
            job: true,
            spare_windows,
            detached_survive: false,
        };
        self.start(info, cmd, size, env, started, on_data, on_exit)
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_inner(
        &self,
        info: TermInfo,
        shell: &ShellInfo,
        wsl_distro: &str,
        cwd: &str,
        size: (u16, u16),
        env: Vec<(String, String)>,
        command: Option<&str>,
        on_data: impl Fn(Vec<u8>) + Send + 'static,
        on_exit: impl FnOnce(Option<u32>) + Send + 'static,
    ) -> Result<()> {
        let mut cmd = CommandBuilder::new(&shell.path);
        match (shell.id.as_str(), command) {
            ("pwsh" | "powershell", None) => cmd.arg("-NoLogo"),
            ("pwsh" | "powershell", Some(c)) => {
                cmd.args(["-NoLogo", "-Command", &with_exit_code(c)])
            }
            ("bash", run) => {
                match run {
                    None => cmd.args(["--login", "-i"]),
                    Some(c) => cmd.args(["--login", "-c", c]),
                }
                // Git Bash stays in the working directory instead of going home.
                cmd.env("CHERE_INVOKING", "1");
            }
            ("zsh" | "fish" | "sh", None) => cmd.arg("-l"),
            ("zsh", Some(c)) => {
                cmd.args(zsh_command_args(c));
                // Oh My Zsh would ask whether to update itself in a log that takes no input
                // (unless its update mode is set by zstyle).
                cmd.env("DISABLE_AUTO_UPDATE", "true");
            }
            ("fish" | "sh", Some(c)) => cmd.args(["-l", "-c", c]),
            ("wsl", run) => {
                if !wsl_distro.is_empty() {
                    cmd.args(["-d", wsl_distro]);
                }
                cmd.args(["--cd", cwd]);
                if let Some(c) = run {
                    cmd.args(["--", "bash", "-lc", c]);
                }
            }
            _ => {}
        }
        if Path::new(cwd).is_dir() {
            cmd.cwd(cwd);
        }
        let started = Started {
            launch: command.is_some(),
            // Not on Unix, where the job is the shell's process group, killed when dropped: an
            // `exit` would take what the user detached (nohup…) with it.
            job: command.is_some() || cfg!(windows),
            // Closing an interactive terminal leaves a browser or an editor its user started
            // open; a launch command stops with everything it started.
            spare_windows: command.is_none(),
            detached_survive: command.is_none(),
        };
        self.start(info, cmd, size, env, started, on_data, on_exit)
    }

    /// Starts `cmd` in a pseudo-console of `size`, with `env` added, streaming its output to
    /// `on_data` until it ends (`on_exit`, with its exit code).
    #[allow(clippy::too_many_arguments)]
    fn start(
        &self,
        info: TermInfo,
        mut cmd: CommandBuilder,
        size: (u16, u16),
        env: Vec<(String, String)>,
        started: Started,
        on_data: impl Fn(Vec<u8>) + Send + 'static,
        on_exit: impl FnOnce(Option<u32>) + Send + 'static,
    ) -> Result<()> {
        #[cfg(windows)]
        let_ctrl_c_through();
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: size.1.max(2),
                cols: size.0.max(10),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow!("{e}"))?;
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // Under Windows, the terminal's environment takes the account's variables from the
        // registry (portable-pty), its temporary folder too: in a test, it keeps the test's
        // (`test_tmp`), where the fake CLIs it starts keep their logs.
        #[cfg(test)]
        for k in ["TMP", "TEMP", "TMPDIR"] {
            if let Some(v) = std::env::var_os(k) {
                cmd.env(k, v);
            }
        }
        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| anyhow!("{e}"))
            .with_context(|| tr!("lancement du shell", "starting the shell"))?;
        drop(pair.slave);
        let mut killer = child.clone_killer();
        let job = started.job.then(|| Job::for_pty(child.as_ref())).flatten();
        if let (Some(job), true) = (&job, started.spare_windows) {
            job.release();
        }
        let mut reader = pair.master.try_clone_reader().map_err(|e| anyhow!("{e}"))?;
        let writer = pair.master.take_writer().map_err(|e| anyhow!("{e}"))?;
        let id = info.id.clone();
        let io_at = Arc::new(AtomicI64::new(now_ms()));
        // With `ESCOUADE_TERM_TRACE`: when the input thread began its last write (µs), until output
        // follows.
        let written_at = Arc::new(AtomicI64::new(0));
        let writes = Arc::new(WriteLog::default());
        let (input, pending) = spawn_input(&id, writer, writes.clone(), written_at.clone())
            .map_err(|e| {
                // Nobody could write to it: it would only wait for an input.
                let _ = killer.kill();
                anyhow!("{e}")
            })
            .with_context(|| tr!("lancement du shell", "starting the shell"))?;
        self.terms.lock().insert(
            id.clone(),
            Arc::new(Term {
                info,
                master: Mutex::new(Some(pair.master)),
                input,
                pending,
                killer: Mutex::new(killer),
                job,
                launch: started.launch,
                spare_windows: started.spare_windows,
                detached_survive: started.detached_survive,
                io_at: io_at.clone(),
                writes: writes.clone(),
                slow_echo_at: AtomicI64::new(i64::MIN),
                typed: Arc::default(),
            }),
        );

        let trace_id = id.clone();
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 32 * 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        io_at.store(now_ms(), Ordering::Release);
                        let read = now_us();
                        // One message of these `n` bytes for the window, then the read in the
                        // log, whose bytes the window counts too (`TermReport::SlowEcho`).
                        on_data(buf[..n].to_vec());
                        writes.output(read, n);
                        if tracing() && written_at.swap(0, Ordering::AcqRel) != 0 {
                            log::info!(
                                "term-trace read id={trace_id} bytes={n} at={read} sent={}",
                                now_us()
                            );
                        }
                    }
                }
            }
        });

        let terms = self.terms.clone();
        std::thread::spawn(move || {
            let code = child.wait().ok().map(|s| s.exit_code());
            // A term that is not there was closed by `kill`, which ended the tree.
            let term = terms.lock().remove(&id);
            if let Some(t) = &term {
                // A shell that ends by itself leaves what its user detached; a program that ends
                // takes what it started, but what shows a window (the Job, released, kills
                // nothing).
                if let (Some(job), true, false) = (&t.job, t.spare_windows, t.detached_survive) {
                    job.terminate_windowless();
                }
                // Closes the pseudo-console, which ends the reader thread (and the job, what the
                // command left running). Here, outside the map's lock, and not by whichever
                // thread drops the last `Arc<Term>`: see `Term::close_console`.
                t.close_console();
            }
            drop(term);
            on_exit(code);
        });
        Ok(())
    }

    /// Queues `data` for the terminal's input. Never waits on I/O: the terminal's input thread
    /// writes it, in the order of the calls, whenever the program reads. Returns the write's
    /// number in the terminal's log of writes, which the window sends back with the report of a
    /// slow echo (`TermReport::SlowEcho`).
    pub fn write(&self, id: &str, data: &[u8]) -> Result<u64> {
        self.queue(id, data, None)
    }

    /// Queues what the window sends (`write`), counting what its user typed (`typing`).
    pub fn write_from_window(&self, id: &str, data: &[u8]) -> Result<u64> {
        if typing(data) {
            self.term(id)?.typed.fetch_add(1, Ordering::AcqRel);
        }
        self.write(id, data)
    }

    /// How many writes of the window to terminal `id` its user typed (`write_from_window`), as
    /// they come; None for a terminal that is gone.
    pub fn typed(&self, id: &str) -> Option<Arc<AtomicU64>> {
        self.term(id).ok().map(|t| t.typed.clone())
    }

    /// Queues `data` like `write`, then waits until the input thread has written it, for at most
    /// `timeout`: for what has to be really written before the next step (a paste, then its
    /// Enter, which the queue would otherwise merge into one write). It is best effort: after the
    /// timeout (a program that does not read) the data stays queued and this returns all the
    /// same. An error is a terminal that is closed.
    pub fn write_and_wait(&self, id: &str, data: &[u8], timeout: Duration) -> Result<u64> {
        let (ack, written) = mpsc::channel();
        let seq = self.queue(id, data, Some(ack))?;
        match written.recv_timeout(timeout) {
            Ok(()) => Ok(seq),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                log::debug!("terminal {id}: write {seq} not written after {timeout:?}");
                Ok(seq)
            }
            // The input thread ended without writing it: the program is gone.
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(closed(i18n::ui())),
        }
    }

    fn queue(&self, id: &str, data: &[u8], ack: Option<mpsc::Sender<()>>) -> Result<u64> {
        let called_us = now_us();
        // The map is locked for the lookup alone.
        let term = self.term(id)?;
        // Room for it in the queue, taken in one step: writes come from several threads.
        let room = term
            .pending
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |queued| {
                (queued + data.len() <= MAX_PENDING).then_some(queued + data.len())
            });
        if room.is_err() {
            return Err(not_reading(i18n::ui()));
        }
        term.io_at.store(now_ms(), Ordering::Release);
        let queued_us = now_us();
        let seq = term.writes.queued(data.len(), queued_us);
        let sent = term.input.send(Input {
            data: data.to_vec(),
            seq,
            called_us,
            queued_us,
            ack,
        });
        if sent.is_err() {
            // Its thread ended: the program is gone.
            term.pending.fetch_sub(data.len(), Ordering::AcqRel);
            return Err(closed(i18n::ui()));
        }
        Ok(seq)
    }

    /// The terminal `id`, with the map released: what waits on a terminal waits for its own lock.
    fn term(&self, id: &str) -> Result<Arc<Term>> {
        self.terms
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| closed(i18n::ui()))
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        self.term(id)?.resize(cols, rows)
    }

    /// What the window measured of terminal `id`, in the log. Nothing for a terminal that is gone.
    pub fn report(&self, id: &str, report: TermReport) {
        let Ok(term) = self.term(id) else {
            return;
        };
        if let Some((level, line)) = term.report_line(report, now_ms()) {
            log::log!(level, "{line}");
        }
    }

    /// The terminal `id` is open (its program has not ended).
    pub fn exists(&self, id: &str) -> bool {
        self.terms.lock().contains_key(id)
    }

    pub fn kill(&self, id: &str) {
        // The map is released first: closing the pseudo-console may wait for its reader, which
        // looks the terminal up to answer a cursor position query.
        let term = self.terms.lock().remove(id);
        if let Some(t) = term {
            // The whole tree at once: the pseudo-console, closed just after, would take its time
            // to end what is attached to it, and not end the rest.
            if let Some(job) = &t.job {
                if t.spare_windows {
                    job.terminate_windowless();
                } else {
                    job.terminate();
                }
            }
            let _ = t.killer.lock().kill();
            // Closed here, not by whichever thread drops the last `Arc<Term>` (see
            // `Term::close_console`).
            t.close_console();
        }
    }

    /// The launch commands running, and the interactive terminals' last input or output.
    pub fn activity(&self) -> Activity {
        let terms = self.terms.lock();
        Activity {
            launches: terms.values().filter(|t| t.launch).count(),
            shell_io_at: terms
                .values()
                .filter(|t| !t.launch)
                .map(|t| t.io_at.load(Ordering::Acquire))
                .max(),
        }
    }

    pub fn list(&self) -> Vec<TermInfo> {
        self.terms.lock().values().map(|t| t.info.clone()).collect()
    }

    pub fn kill_project(&self, project_id: &str) {
        let ids: Vec<String> = self
            .terms
            .lock()
            .values()
            .filter(|t| t.info.project_id == project_id)
            .map(|t| t.info.id.clone())
            .collect();
        for id in ids {
            self.kill(&id);
        }
    }

    pub fn kill_all(&self) {
        let ids: Vec<String> = self.terms.lock().keys().cloned().collect();
        for id in ids {
            self.kill(&id);
        }
    }
}

/// Tests starting a real shell run one at a time: several Windows PowerShells booting at once on a
/// CI runner can outlast what the tests wait for, and the ping tests would see each other's pings.
#[cfg(test)]
fn one_shell_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    static SHELLS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    SHELLS.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{Duration, Instant};

    /// Fake system folders under `dir` (PATH entry, Program Files, LocalAppData, Windows).
    fn roots(dir: &Path) -> ShellRoots {
        ShellRoots {
            path: vec![dir.join("bin")],
            program_files: Some(dir.join("pf")),
            local_app_data: Some(dir.join("lad")),
            system_root: Some(dir.join("win")),
        }
    }

    fn touch(p: PathBuf) -> PathBuf {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "").unwrap();
        p
    }

    fn windows_powershell(dir: &Path) -> PathBuf {
        touch(dir.join(r"win\System32\WindowsPowerShell\v1.0\powershell.exe"))
    }

    fn ids(shells: &[ShellInfo]) -> Vec<&str> {
        shells.iter().map(|s| s.id.as_str()).collect()
    }

    #[test]
    fn windows_powershell_is_offered_when_powershell_7_is_not_installed() {
        let d = crate::paths::test_dir("shells-winps");
        let exe = windows_powershell(&d);
        let shells = detect_shells_in(&roots(&d), &Settings::default());
        assert_eq!(ids(&shells), ["powershell"]);
        assert_eq!(shells[0].label, "Windows PowerShell");
        assert_eq!(shells[0].path, exe.to_string_lossy());
    }

    #[test]
    fn powershell_7_is_found_off_the_path_and_preferred() {
        let d = crate::paths::test_dir("shells-pwsh-pf");
        windows_powershell(&d);
        let exe = touch(d.join(r"pf\PowerShell\7\pwsh.exe"));
        let shells = detect_shells_in(&roots(&d), &Settings::default());
        assert_eq!(ids(&shells), ["pwsh"]);
        assert_eq!(shells[0].path, exe.to_string_lossy());
    }

    #[test]
    fn powershell_7_from_the_microsoft_store_is_found() {
        let d = crate::paths::test_dir("shells-pwsh-store");
        windows_powershell(&d);
        let exe = touch(d.join(r"lad\Microsoft\WindowsApps\pwsh.exe"));
        let shells = detect_shells_in(&roots(&d), &Settings::default());
        assert_eq!(ids(&shells), ["pwsh"]);
        assert_eq!(shells[0].path, exe.to_string_lossy());
    }

    #[test]
    fn microsoft_store_app_aliases_count_as_installed() {
        // Store aliases are 0-byte reparse points: they must still count as installed programs.
        let Some(apps) = std::env::var_os("LOCALAPPDATA")
            .map(|d| PathBuf::from(d).join(r"Microsoft\WindowsApps"))
        else {
            return;
        };
        let Some(alias) = std::fs::read_dir(&apps).ok().and_then(|mut it| {
            it.find_map(|e| {
                let e = e.ok()?;
                (e.path().extension()? == "exe").then(|| e.path())
            })
        }) else {
            eprintln!("no Store alias on this machine: skipped");
            return;
        };
        assert!(installed(&alias), "{}", alias.display());
    }

    /// PIDs of running ping.exe processes.
    fn pings() -> Vec<u32> {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq PING.EXE", "/FO", "CSV", "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split("\",\"").nth(1)?.parse().ok())
            .collect()
    }

    /// Whether the process `pid` is still running: a handle that can be waited on is signaled
    /// once the process ended (a pid is not proof of life: `tasklist` lags and ids are reused).
    fn alive(pid: u32) -> bool {
        use windows_sys::Win32::Foundation::{
            CloseHandle, GetLastError, ERROR_ACCESS_DENIED, WAIT_TIMEOUT,
        };
        use windows_sys::Win32::Storage::FileSystem::SYNCHRONIZE;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        // SAFETY: the handle is checked, waited on without blocking and closed.
        unsafe {
            let h = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                // Out of reach but there; any other failure: no such process.
                return GetLastError() == ERROR_ACCESS_DENIED;
            }
            let running = WaitForSingleObject(h, 0) == WAIT_TIMEOUT;
            CloseHandle(h);
            running
        }
    }

    /// Kills the process `pid` if it is still there (a test that failed leaves its ping behind).
    fn kill_process(pid: u32) {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }

    /// Opens a PowerShell 7 terminal `id` that answers the cursor position queries the way
    /// xterm.js does in the app; returns it with what it printed, once its prompt is there.
    fn open_pwsh(id: &'static str) -> Option<(PtyManager, Arc<Mutex<String>>)> {
        let Some(shell) = detect_shells(&Settings::default())
            .into_iter()
            .find(|s| s.id == "pwsh")
        else {
            eprintln!("PowerShell 7 not installed: skipped");
            return None;
        };
        let pty = PtyManager::default();
        let out = Arc::new(Mutex::new(String::new()));
        let (sink, answer) = (out.clone(), pty.clone());
        let info = TermInfo {
            id: id.into(),
            project_id: "p".into(),
            name: "pwsh-1".into(),
            shell: "pwsh".into(),
        };
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let query = Mutex::new(CursorQuery::default());
        let on_data = move |b: Vec<u8>| {
            if query.lock().feed(&b) {
                let _ = answer.write(id, b"\x1b[1;1R");
            }
            sink.lock().push_str(&String::from_utf8_lossy(&b));
        };
        pty.spawn(info, &shell, "", &cwd, (120, 30), vec![], on_data, |_| {})
            .unwrap();
        let start = Instant::now();
        while !out.lock().contains("PS ") {
            assert!(
                start.elapsed() < Duration::from_secs(60),
                "no prompt: {}",
                out.lock()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(1500));
        Some((pty, out))
    }

    /// Types `command`, which starts a ping, in terminal `id`; returns the ping's pid.
    fn start_ping(pty: &PtyManager, out: &Mutex<String>, id: &str, command: &str) -> u32 {
        let before = pings();
        pty.write(id, format!("{command}\r").as_bytes()).unwrap();
        let start = Instant::now();
        loop {
            if let Some(p) = pings().into_iter().find(|p| !before.contains(p)) {
                return p;
            }
            assert!(
                start.elapsed() < Duration::from_secs(90),
                "ping did not start: {}",
                out.lock()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Types `command`, which starts a ping, then closes the terminal: the ping must die with it.
    fn closing_kills_the_ping_of(id: &'static str, command: &str) {
        let _one = one_shell_at_a_time();
        let Some((pty, out)) = open_pwsh(id) else {
            return;
        };
        let pid = start_ping(&pty, &out, id, command);
        assert!(alive(pid));
        pty.kill(id);
        // The pseudo-console closes on its own time (seconds on a loaded runner): the job does not
        // wait for it.
        let start = Instant::now();
        while alive(pid) {
            if start.elapsed() > Duration::from_secs(20) {
                kill_process(pid);
                panic!("ping {pid} survived its terminal");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    fn closing_a_terminal_kills_the_programs_it_started() {
        closing_kills_the_ping_of("t1", "ping -n 60 127.0.0.1");
    }

    #[test]
    fn closing_a_terminal_also_kills_programs_started_outside_its_console() {
        // Hidden, in a console of its own: closing the pseudo-console does not reach it.
        closing_kills_the_ping_of(
            "t4",
            "Start-Process ping -ArgumentList '-n','60','127.0.0.1' -WindowStyle Hidden",
        );
    }

    #[test]
    fn sixty_keys_written_back_to_back_reach_the_shell_in_order() {
        let _one = one_shell_at_a_time();
        let Some((pty, out)) = open_pwsh("t7") else {
            return;
        };
        // One write per key, as the window sends them: the input thread merges what it finds
        // queued, whatever the way it is cut.
        let digits: String = "0123456789".chars().cycle().take(53).collect();
        let typed = "echo x";
        for key in typed.chars().chain(digits.chars()) {
            pty.write("t7", key.to_string().as_bytes()).unwrap();
        }
        pty.write("t7", b"\r").unwrap();
        let start = Instant::now();
        while !out.lock().contains(&format!("x{digits}")) {
            assert!(
                start.elapsed() < Duration::from_secs(60),
                "keys out of order or lost: {}",
                out.lock()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        // The 60 writes, and their echo, are in the terminal's recent writes (after the answer to
        // the cursor position query `open_pwsh` gives).
        let term = pty.term("t7").unwrap();
        let start = Instant::now();
        loop {
            let recent = term.writes.all();
            let keys = &recent[recent.len() - 60..];
            if keys
                .iter()
                .all(|w| w.written_us.is_some() && w.first_output_us.is_some())
            {
                assert_eq!(keys.iter().filter(|w| w.bytes == 1).count(), 60);
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "writes not marked: {keys:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        pty.kill("t7");
    }

    /// Types `command`, which detaches a ping, then `exit`: the ping must run on after the shell
    /// and its terminal are gone.
    fn exit_leaves_the_ping_of(id: &'static str, command: &str) {
        let _one = one_shell_at_a_time();
        let Some((pty, out)) = open_pwsh(id) else {
            return;
        };
        let pid = start_ping(&pty, &out, id, command);
        pty.write(id, b"exit\r").unwrap();
        let start = Instant::now();
        while pty.list().iter().any(|t| t.id == id) {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "the shell did not end: {}",
                out.lock()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        // The shell is gone and its terminal dropped: what it detached must still run.
        std::thread::sleep(Duration::from_secs(2));
        let survived = alive(pid);
        kill_process(pid);
        assert!(survived, "ping {pid} died with the shell that started it");
    }

    #[test]
    fn a_shell_that_ends_by_itself_leaves_the_programs_its_user_detached() {
        // A console of its own, hidden, as `Start-Process` gives a console program. (One sharing the
        // terminal's console, `start /b`, goes with the pseudo-console when its shell ends: that
        // is Windows closing it, no job involved.)
        exit_leaves_the_ping_of(
            "t2",
            "Start-Process ping -ArgumentList '-n','30','127.0.0.1' -WindowStyle Hidden",
        );
    }

    /// Stops the launch command `command`, which starts a ping: the ping must die with it.
    fn stop_kills_its_ping(id: &str, command: &str) {
        let _one = one_shell_at_a_time();
        let pty = PtyManager::default();
        let Some(shell) = detect_shells(&Settings::default())
            .into_iter()
            .find(|s| s.id == "pwsh" || s.id == "powershell")
        else {
            eprintln!("PowerShell not installed: skipped");
            return;
        };
        let info = TermInfo {
            id: id.into(),
            project_id: "p".into(),
            name: "serveur".into(),
            shell: shell.id.clone(),
        };
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let before = pings();
        pty.spawn_command(
            info,
            &shell,
            "",
            &cwd,
            (120, 30),
            vec![],
            1,
            command,
            |_| {},
            |_| {},
        )
        .unwrap();
        let start = Instant::now();
        let pid = loop {
            if let Some(p) = pings().into_iter().find(|p| !before.contains(p)) {
                break p;
            }
            assert!(
                start.elapsed() < Duration::from_secs(90),
                "ping did not start"
            );
            std::thread::sleep(Duration::from_millis(50));
        };
        pty.kill(id);
        let start = Instant::now();
        while alive(pid) {
            if start.elapsed() > Duration::from_secs(5) {
                kill_process(pid);
                panic!("ping {pid} survived its launch command");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    fn stopping_a_launch_command_kills_the_programs_it_started() {
        stop_kills_its_ping("run1", "ping -n 60 127.0.0.1");
    }

    #[test]
    fn stopping_a_launch_command_also_kills_programs_started_outside_its_console() {
        stop_kills_its_ping(
            "run2",
            "Start-Process ping -ArgumentList '-n','60','127.0.0.1' -WindowStyle Hidden; Start-Sleep 60",
        );
    }

    /// Kills the processes it holds when it is dropped: a test that failed leaves them running.
    #[derive(Default)]
    struct Reaper(Vec<u32>);

    impl Drop for Reaper {
        fn drop(&mut self) {
            for pid in &self.0 {
                kill_process(*pid);
            }
        }
    }

    /// What a PowerShell script of the tests does before it waits: a console program only says it
    /// runs (its process id in `pid_file`), a window program shows a form first (and says "none"
    /// when Windows shows it no window: no desktop to show it on). The form is shown off the
    /// screens, without the focus nor a button in the taskbar: the user at the machine sees
    /// nothing, the app a window. (The first `ShowWindow` of a process started hidden hides.)
    fn script_text(pid_file: &Path, window: bool) -> String {
        let pid_file = pid_file.display();
        if window {
            format!(
                "Add-Type -AssemblyName System.Windows.Forms\r\n\
                 Add-Type -Name W -Namespace Esc -MemberDefinition '[DllImport(\"user32.dll\")] public static extern bool ShowWindow(IntPtr h, int c); [DllImport(\"user32.dll\")] public static extern bool IsWindowVisible(IntPtr h);'\r\n\
                 $f = New-Object Windows.Forms.Form\r\n\
                 $f.ShowInTaskbar = $false\r\n\
                 $f.StartPosition = 'Manual'\r\n\
                 $f.Location = New-Object Drawing.Point(-30000, -30000)\r\n\
                 [void][Esc.W]::ShowWindow($f.Handle, 0)\r\n\
                 [void][Esc.W]::ShowWindow($f.Handle, 4)\r\n\
                 Set-Content -Path '{pid_file}' -Value $(if ([Esc.W]::IsWindowVisible($f.Handle)) {{ $PID }} else {{ 'none' }})\r\n\
                 Start-Sleep 60\r\n"
            )
        } else {
            format!("Set-Content -Path '{pid_file}' -Value $PID\r\nStart-Sleep 60\r\n")
        }
    }

    /// Writes the script `name` in `dir`; returns it with the file its process id will be told in.
    fn write_script(dir: &Path, name: &str, window: bool) -> (PathBuf, PathBuf) {
        let (script, pid_file) = (
            dir.join(format!("{name}.ps1")),
            dir.join(format!("{name}.pid")),
        );
        std::fs::write(&script, script_text(&pid_file, window)).unwrap();
        (script, pid_file)
    }

    /// The command that runs `script` in a PowerShell of its own, outside the console of the
    /// terminal, its console hidden (a window program shows its own window).
    fn start_command(script: &Path) -> String {
        format!(
            "Start-Process powershell -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','\"{}\"'",
            script.display()
        )
    }

    /// The process id a script told in `pid_file`; None when it could not show its window.
    fn told_pid(pid_file: &Path) -> Option<u32> {
        let start = Instant::now();
        loop {
            let text = std::fs::read_to_string(pid_file).unwrap_or_default();
            if text.trim() == "none" {
                return None;
            }
            if let Ok(pid) = text.trim().parse() {
                return Some(pid);
            }
            assert!(
                start.elapsed() < Duration::from_secs(90),
                "the script told no process id"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Waits for the process `pid` to end; false if it is still there after `secs`.
    fn ends_within(pid: u32, secs: u64) -> bool {
        let start = Instant::now();
        while alive(pid) {
            if start.elapsed() > Duration::from_secs(secs) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        true
    }

    #[test]
    fn closing_a_terminal_ends_its_console_programs_but_spares_the_ones_with_a_window() {
        let _one = one_shell_at_a_time();
        let Some((pty, out)) = open_pwsh("t5") else {
            return;
        };
        let dir = crate::paths::test_dir("pty-close-spares-windows");
        let mut reaper = Reaper::default();
        // A program with no window, in a console of its own; a program showing a window.
        let (console_script, console_file) = write_script(&dir, "console", false);
        let (window_script, window_file) = write_script(&dir, "window", true);
        pty.write(
            "t5",
            format!("{}\r", start_command(&console_script)).as_bytes(),
        )
        .unwrap();
        let console = told_pid(&console_file).unwrap();
        reaper.0.push(console);
        pty.write(
            "t5",
            format!("{}\r", start_command(&window_script)).as_bytes(),
        )
        .unwrap();
        let Some(window) = told_pid(&window_file) else {
            eprintln!("no window could be shown here: skipped");
            return;
        };
        reaper.0.push(window);
        assert!(alive(console) && alive(window), "{}", out.lock());

        pty.kill("t5");

        assert!(
            ends_within(console, 20),
            "the program without a window survived its terminal"
        );
        // The terminal is closed and gone: the window still stands (given the time a wrongly
        // killed one would take to end).
        assert!(
            !ends_within(window, 3),
            "the program showing a window died with its terminal"
        );
    }

    #[test]
    fn a_terminal_dropped_without_being_closed_kills_nothing() {
        // What the app's crash, its quit or an update's restart does to a terminal: its handles
        // close, nothing asks the processes to end.
        let _one = one_shell_at_a_time();
        let Some((pty, out)) = open_pwsh("t6") else {
            return;
        };
        let dir = crate::paths::test_dir("pty-dropped-kills-nothing");
        let mut reaper = Reaper::default();
        let (script, pid_file) = write_script(&dir, "console", false);
        pty.write("t6", format!("{}\r", start_command(&script)).as_bytes())
            .unwrap();
        let program = told_pid(&pid_file).unwrap();
        reaper.0.push(program);
        assert!(alive(program), "{}", out.lock());

        let term = pty.terms.lock().remove("t6");
        drop(term);

        std::thread::sleep(Duration::from_secs(2));
        assert!(alive(program), "the dropped terminal killed its program");
    }

    /// Runs a program (a PowerShell script) in a terminal of its own: it starts a console program
    /// and a program showing a window, outside its console, and ends. Returns whether the console
    /// program and the window one ended with it; None when no window could be shown here.
    fn what_the_end_of_a_program_takes(
        name: &'static str,
        spare_windows: bool,
    ) -> Option<(bool, bool)> {
        let _one = one_shell_at_a_time();
        let Some(shell) = detect_shells(&Settings::default())
            .into_iter()
            .find(|s| s.id == "pwsh" || s.id == "powershell")
        else {
            eprintln!("PowerShell not installed: skipped");
            return None;
        };
        let dir = crate::paths::test_dir(name);
        let mut reaper = Reaper::default();
        let (console_script, console_file) = write_script(&dir, "console", false);
        let (window_script, window_file) = write_script(&dir, "window", true);
        // Both run before it ends: each says so by its file.
        let program = dir.join("program.ps1");
        std::fs::write(
            &program,
            format!(
                "{}\r\n{}\r\nwhile (-not ((Test-Path '{}') -and (Test-Path '{}'))) {{ Start-Sleep -Milliseconds 100 }}\r\nStart-Sleep 1\r\n",
                start_command(&console_script),
                start_command(&window_script),
                console_file.display(),
                window_file.display(),
            ),
        )
        .unwrap();
        let pty = PtyManager::default();
        let (answer, query) = (pty.clone(), Mutex::new(CursorQuery::default()));
        let info = TermInfo {
            id: name.into(),
            project_id: String::new(),
            name: "connexion".into(),
            shell: "claude".into(),
        };
        let (ended, done) = std::sync::mpsc::channel();
        pty.spawn_program(
            info,
            &crate::engine::Program::Native(PathBuf::from(&shell.path)),
            &[
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                program.display().to_string(),
            ],
            &dir.to_string_lossy(),
            (120, 30),
            vec![],
            spare_windows,
            move |b: Vec<u8>| {
                if query.lock().feed(&b) {
                    let _ = answer.write(name, b"\x1b[1;1R");
                }
            },
            move |_| {
                let _ = ended.send(());
            },
        )
        .unwrap();
        done.recv_timeout(Duration::from_secs(180))
            .expect("the program did not end");
        let console = told_pid(&console_file).unwrap();
        reaper.0.push(console);
        let window = told_pid(&window_file);
        reaper.0.extend(window);
        Some((ends_within(console, 20), ends_within(window?, 3)))
    }

    #[test]
    fn the_end_of_the_sign_in_ends_what_it_started_but_the_programs_with_a_window() {
        let Some((console_ended, window_ended)) =
            what_the_end_of_a_program_takes("pty-sign-in-spares-windows", true)
        else {
            return;
        };
        assert!(console_ended, "the program without a window survived");
        assert!(!window_ended, "the browser of the sign-in died with it");
    }

    #[test]
    fn the_end_of_the_claude_code_view_ends_everything_it_started() {
        let Some((console_ended, window_ended)) =
            what_the_end_of_a_program_takes("pty-view-kills-all", false)
        else {
            return;
        };
        assert!(console_ended, "the program without a window survived");
        assert!(window_ended, "the program with a window survived");
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn touch(p: PathBuf) -> PathBuf {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "").unwrap();
        p
    }

    #[test]
    fn the_login_shell_comes_first_and_fish_is_found_on_the_path() {
        let d = crate::paths::test_dir("shells-unix");
        let fish = touch(d.join("bin").join("fish"));
        let shells = detect_unix_shells(Some(&fish), &[d.join("bin")], &Settings::default());
        assert_eq!(shells[0].id, "fish");
        assert_eq!(shells[0].path, fish.to_string_lossy());
        assert_eq!(shells.iter().filter(|s| s.id == "fish").count(), 1);
        assert!(shells
            .iter()
            .any(|s| s.id == "sh" || s.id == "bash" || s.id == "zsh"));
    }

    /// Runs `command` with zsh or bash as a launch command, away from the machine's ~/.zshrc;
    /// returns its output and exit code.
    fn launch(command: &str) -> (String, Option<u32>) {
        let shell = detect_unix_shells(None, &[], &Settings::default())
            .into_iter()
            .find(|s| s.id == "bash" || s.id == "zsh")
            .expect("a shell");
        let home = crate::paths::test_dir("launch-zdotdir");
        launch_with(&shell, command, zdotdir(&home))
    }

    /// Runs `command` with `shell` as a launch command, `env` added; returns its output and exit
    /// code.
    fn launch_with(
        shell: &ShellInfo,
        command: &str,
        env: Vec<(String, String)>,
    ) -> (String, Option<u32>) {
        let _one = one_shell_at_a_time();
        let pty = PtyManager::default();
        let out = Arc::new(Mutex::new(String::new()));
        let exit = Arc::new(Mutex::new(None));
        let (sink, done) = (out.clone(), exit.clone());
        let info = TermInfo {
            id: format!("run-{command}"),
            project_id: "p".into(),
            name: "run".into(),
            shell: shell.id.clone(),
        };
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        pty.spawn_command(
            info,
            shell,
            "",
            &cwd,
            (120, 30),
            env,
            1,
            command,
            move |b| sink.lock().push_str(&String::from_utf8_lossy(&b)),
            move |code| *done.lock() = Some(code),
        )
        .unwrap();
        let start = Instant::now();
        while exit.lock().is_none() {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "did not end: {}",
                out.lock()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(100));
        let code = exit.lock().unwrap();
        let text = out.lock().clone();
        (text, code)
    }

    #[test]
    fn a_launch_command_reports_its_output_and_exit_code() {
        let (out, code) = launch("echo hello-escouade; exit 3");
        assert!(out.contains("hello-escouade"), "{out}");
        assert_eq!(code, Some(3));
        assert_eq!(launch("true").1, Some(0));
    }

    /// zsh, when the machine has it (macOS always does).
    fn zsh() -> Option<ShellInfo> {
        let zsh = detect_unix_shells(None, &[], &Settings::default())
            .into_iter()
            .find(|s| s.id == "zsh");
        if zsh.is_none() {
            eprintln!("zsh not installed: skipped");
        }
        zsh
    }

    /// The variables pointing zsh to `dir` for its startup files, with an empty .zshrc there
    /// unless it has one: without any, zsh-newuser-install (Homebrew's zsh, Linux's) would wait
    /// for an answer.
    fn zdotdir(dir: &Path) -> Vec<(String, String)> {
        let zshrc = dir.join(".zshrc");
        if !zshrc.exists() {
            std::fs::write(zshrc, "").unwrap();
        }
        vec![("ZDOTDIR".into(), dir.to_string_lossy().into_owned())]
    }

    #[test]
    fn a_zsh_launch_command_runs_the_ruby_its_zshrc_sets_up_like_a_terminal() {
        use std::os::unix::fs::PermissionsExt;
        let Some(zsh) = zsh() else { return };
        // Version managers (rbenv, nvm, asdf…) set themselves up in ~/.zshrc: without it, a
        // Rails server starts with the system's Ruby, which macOS's /etc/zprofile puts first.
        let home = crate::paths::test_dir("launch-zshrc");
        let ruby = home.join("bin").join("ruby");
        std::fs::create_dir_all(home.join("bin")).unwrap();
        std::fs::write(&ruby, "#!/bin/sh\necho ruby-du-zshrc\n").unwrap();
        std::fs::set_permissions(&ruby, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(home.join(".zshrc"), "export PATH=\"$ZDOTDIR/bin:$PATH\"\n").unwrap();
        let (out, code) = launch_with(&zsh, "ruby; echo omz=$DISABLE_AUTO_UPDATE", zdotdir(&home));
        assert!(out.contains("ruby-du-zshrc"), "{out}");
        // Oh My Zsh would ask whether to update itself in the log, which takes no input.
        assert!(out.contains("omz=true"), "{out}");
        assert_eq!(code, Some(0));
    }

    #[test]
    fn stopping_a_zsh_launch_command_kills_the_programs_it_started() {
        let Some(zsh) = zsh() else { return };
        let _one = one_shell_at_a_time();
        let home = crate::paths::test_dir("launch-zsh-stop");
        let pid_file = home.join("pid.txt");
        // Out of the shell's process group (job control), a program ignoring SIGHUP (started
        // with nohup, a server handling it) would outlive the launch command. `; true`: the shell
        // forks the program instead of becoming it.
        let command = format!(
            "sh -c 'trap \"\" HUP; echo $$ > \"{}\"; exec sleep 60'; true",
            pid_file.display()
        );
        let pty = PtyManager::default();
        let info = TermInfo {
            id: "run-zsh-stop".into(),
            project_id: "p".into(),
            name: "serveur".into(),
            shell: zsh.id.clone(),
        };
        pty.spawn_command(
            info,
            &zsh,
            "",
            &home.to_string_lossy(),
            (120, 30),
            zdotdir(&home),
            1,
            &command,
            |_| {},
            |_| {},
        )
        .unwrap();
        let start = Instant::now();
        let pid: i32 = loop {
            if let Some(p) = std::fs::read_to_string(&pid_file)
                .ok()
                .and_then(|s| s.trim().parse().ok())
            {
                break p;
            }
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "the program did not start"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        pty.kill("run-zsh-stop");
        // SAFETY: signal 0 only checks that the process exists.
        let alive = || unsafe { libc::kill(pid, 0) == 0 };
        let start = Instant::now();
        while alive() {
            if start.elapsed() > Duration::from_secs(5) {
                // SAFETY: plain syscall on the process this test started.
                unsafe { libc::kill(pid, libc::SIGKILL) };
                panic!("process {pid} survived its launch command");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn alive(pid: i32) -> bool {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    /// Kills the processes it holds when it is dropped: a test that failed leaves them running.
    #[derive(Default)]
    struct Reaper(Vec<i32>);

    impl Drop for Reaper {
        fn drop(&mut self) {
            for pid in &self.0 {
                // SAFETY: plain syscall on a process the test started.
                unsafe { libc::kill(*pid, libc::SIGKILL) };
            }
        }
    }

    /// Writes the shell script `name` in `dir`, executable.
    fn script(dir: &Path, name: &str, text: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{text}")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// The command starting, in the background, a process that tells its id in `pid_file`, then
    /// waits a minute.
    fn background(pid_file: &Path) -> String {
        format!(
            "sh -c 'echo $$ > \"$1\"; exec sleep 60' sh '{}' &",
            pid_file.display()
        )
    }

    /// The process id told in `pid_file`.
    fn told_pid(pid_file: &Path) -> i32 {
        let start = Instant::now();
        loop {
            if let Some(pid) = std::fs::read_to_string(pid_file)
                .ok()
                .and_then(|s| s.trim().parse().ok())
            {
                return pid;
            }
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "no process id in {}",
                pid_file.display()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Waits for the process `pid` to end; false if it is still there after `secs`.
    fn ends_within(pid: i32, secs: u64) -> bool {
        let start = Instant::now();
        while alive(pid) {
            if start.elapsed() > Duration::from_secs(secs) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        true
    }

    /// How a program of a terminal ends in `what_a_program_takes`.
    #[derive(PartialEq)]
    enum End {
        /// It ends by itself.
        Itself,
        /// Its terminal is closed while it runs.
        Closed,
    }

    /// Runs a program standing for Claude Code in a terminal of its own: it starts a process in
    /// the background (a program without a window) and opens a page with `$BROWSER`, as Claude
    /// Code does; the browser (a fake one) starts the process standing for itself and returns, as
    /// `xdg-open` does when no browser runs yet. Then the program ends as `end` says. With
    /// `sign_in`, it runs as the sign-in does: sparing what shows a window, with Escouade's
    /// opener. Returns whether the background process and the browser ended with it.
    fn what_a_program_takes(name: &'static str, sign_in: bool, end: End) -> (bool, bool) {
        let _one = one_shell_at_a_time();
        let dir = crate::paths::test_dir(name);
        let mut reaper = Reaper::default();
        let (console_file, browser_file) = (dir.join("console.pid"), dir.join("browser.pid"));
        let browser = script(&dir, "browser", &background(&browser_file));
        let wait = match end {
            End::Itself => "sleep 1",
            End::Closed => "sleep 60",
        };
        let program = script(
            &dir,
            "claude",
            &format!(
                "{}\n\"$BROWSER\" 'https://claude.ai/oauth/authorize?code=true'\nwhile [ ! -s '{}' ] || [ ! -s '{}' ]; do sleep 0.1; done\n{wait}\n",
                background(&console_file),
                console_file.display(),
                browser_file.display(),
            ),
        );
        let mut env = vec![("BROWSER".to_string(), browser.display().to_string())];
        if sign_in {
            open_browser_apart(&dir.join("open-browser"), &mut env).unwrap();
        }
        let pty = PtyManager::default();
        let info = TermInfo {
            id: name.into(),
            project_id: String::new(),
            name: "connexion".into(),
            shell: "claude".into(),
        };
        let (ended, done) = std::sync::mpsc::channel();
        pty.spawn_program(
            info,
            &crate::engine::Program::Native(PathBuf::from("/bin/sh")),
            &[program.display().to_string()],
            &dir.to_string_lossy(),
            (120, 30),
            env,
            sign_in,
            |_| {},
            move |_| {
                let _ = ended.send(());
            },
        )
        .unwrap();
        let console = told_pid(&console_file);
        reaper.0.push(console);
        let browser = told_pid(&browser_file);
        reaper.0.push(browser);
        if end == End::Closed {
            assert!(alive(console) && alive(browser));
            pty.kill(name);
        }
        done.recv_timeout(Duration::from_secs(60))
            .expect("the program did not end");
        // The browser is given the time a wrongly killed one would take to end.
        (ends_within(console, 10), ends_within(browser, 3))
    }

    #[test]
    fn the_end_of_the_sign_in_ends_what_it_started_but_the_browser_it_opened() {
        let (console_ended, browser_ended) =
            what_a_program_takes("pty-unix-sign-in-end", true, End::Itself);
        assert!(console_ended, "the program without a window survived");
        assert!(!browser_ended, "the browser of the sign-in died with it");
    }

    #[test]
    fn closing_the_sign_in_ends_what_it_started_but_the_browser_it_opened() {
        let (console_ended, browser_ended) =
            what_a_program_takes("pty-unix-sign-in-close", true, End::Closed);
        assert!(console_ended, "the program without a window survived");
        assert!(!browser_ended, "the browser of the sign-in died with it");
    }

    #[test]
    fn the_end_of_the_claude_code_view_ends_everything_it_started() {
        let (console_ended, browser_ended) =
            what_a_program_takes("pty-unix-view-end", false, End::Itself);
        assert!(console_ended, "the program without a window survived");
        assert!(browser_ended, "the browser survived");
    }

    #[test]
    fn the_sign_in_opens_the_users_browser_with_the_opener() {
        let dir = crate::paths::test_dir("pty-unix-opener-env");
        let opener = dir.join("open-browser");
        let mut env = vec![
            ("HTTPS_PROXY".to_string(), "http://proxy:3128".to_string()),
            ("BROWSER".to_string(), "/usr/bin/firefox".to_string()),
        ];
        open_browser_apart(&opener, &mut env).unwrap();
        let var = |k: &str| -> Vec<&str> {
            env.iter()
                .filter(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
                .collect()
        };
        assert_eq!(var("BROWSER"), vec![opener.to_str().unwrap()]);
        assert_eq!(var(USER_BROWSER_VAR), vec!["/usr/bin/firefox"]);
        assert_eq!(var("HTTPS_PROXY"), vec!["http://proxy:3128"]);
        // The opener itself is never taken for the user's browser: it would open nothing.
        let mut again = env.clone();
        open_browser_apart(&opener, &mut again).unwrap();
        assert_eq!(
            again.iter().find(|(k, _)| k == USER_BROWSER_VAR).unwrap().1,
            "/usr/bin/firefox"
        );
    }

    #[test]
    fn without_a_browser_of_the_user_the_opener_runs_the_systems() {
        let dir = crate::paths::test_dir("pty-unix-opener-system");
        let opener = dir.join("open-browser");
        open_browser_apart(&opener, &mut Vec::new()).unwrap();
        // Stand-ins for the system's openers: each tells what it was asked to open, and the
        // variables it got (a `BROWSER` still set to the opener: `xdg-open` would run it again).
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let told = dir.join("told.txt");
        for name in ["xdg-open", "open"] {
            script(
                &bin,
                name,
                &format!(
                    "echo \"{name} $1 ${{BROWSER-unset}} ${{{USER_BROWSER_VAR}-unset}}\" > '{}'\n",
                    told.display()
                ),
            );
        }
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        let status = std::process::Command::new(&opener)
            .arg("https://claude.ai/login")
            .env("PATH", path)
            .env(USER_BROWSER_VAR, "")
            .env("BROWSER", &opener)
            .status()
            .unwrap();
        assert!(status.success());
        let system = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        assert_eq!(
            std::fs::read_to_string(&told).unwrap().trim(),
            format!("{system} https://claude.ai/login unset unset")
        );
    }

    #[test]
    fn the_opener_runs_the_users_browser_with_its_own_browser_variable() {
        let dir = crate::paths::test_dir("pty-unix-opener-user");
        let opener = dir.join("open-browser");
        open_browser_apart(&opener, &mut Vec::new()).unwrap();
        let told = dir.join("told.txt");
        let browser = script(
            &dir,
            "my-browser",
            &format!(
                "{STDIN}\necho \"$1 ${{BROWSER-unset}} ${{{USER_BROWSER_VAR}-unset}} $stdin\" > '{}'\n",
                told.display()
            ),
        );
        let mut run = std::process::Command::new(&opener);
        run.arg("https://claude.ai/login")
            .env(USER_BROWSER_VAR, &browser)
            .env("BROWSER", &opener);
        assert!(run_from_claude(run).success());
        assert_eq!(
            std::fs::read_to_string(&told).unwrap().trim(),
            format!("https://claude.ai/login {} unset eof", browser.display())
        );
    }

    /// What a stand-in browser reads from its input (`$stdin`): its first line, `eof` for none.
    const STDIN: &str = "if read -r line; then stdin=$line; else stdin=eof; fi";

    /// Runs `opener` as Claude Code does, its input a pipe Claude Code writes to; returns how it
    /// ended.
    fn run_from_claude(mut opener: std::process::Command) -> std::process::ExitStatus {
        use std::io::Write;
        let mut child = opener.stdin(std::process::Stdio::piped()).spawn().unwrap();
        // Refused once nothing reads it any more.
        let _ = child.stdin.take().unwrap().write_all(b"from-claude\n");
        child.wait().unwrap()
    }

    /// A folder holding Perl alone: run with it as its PATH, the opener takes its branch for the
    /// systems without `setsid` (macOS). None when Perl is not installed.
    fn perl_alone(dir: &Path) -> Option<PathBuf> {
        let perl = std::env::split_paths(&std::env::var_os("PATH")?)
            .map(|d| d.join("perl"))
            .find(|p| p.is_file());
        let Some(perl) = perl else {
            eprintln!("Perl not installed: skipped");
            return None;
        };
        let bin = dir.join("perl-alone");
        std::fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(perl, bin.join("perl")).unwrap();
        Some(bin)
    }

    #[test]
    fn without_setsid_the_opener_leaves_its_session_even_when_it_leads_its_group() {
        use std::os::unix::process::CommandExt;
        let dir = crate::paths::test_dir("pty-unix-opener-perl-leader");
        let Some(bin) = perl_alone(&dir) else { return };
        let opener = dir.join("open-browser");
        open_browser_apart(&opener, &mut Vec::new()).unwrap();
        // The browser tells its process id, its group and its input, all at once: the opener
        // may end before it runs.
        let told = dir.join("told.txt");
        let browser = script(
            &dir,
            "my-browser",
            &format!(
                "PATH='{path}'\n{STDIN}\necho \"$$ $(ps -o pgid= -p $$ | tr -d ' ') $stdin\" > '{told}.tmp'\nmv '{told}.tmp' '{told}'\n",
                path = std::env::var("PATH").unwrap(),
                told = told.display(),
            ),
        );
        let mut run = std::process::Command::new(&opener);
        run.arg("https://claude.ai/login")
            .env("PATH", &bin)
            .env(USER_BROWSER_VAR, &browser)
            .env("BROWSER", &opener)
            // A group leader cannot start a session: it has to fork first, as setsid does.
            .process_group(0);
        let mut child = run.stdin(std::process::Stdio::piped()).spawn().unwrap();
        let leader = child.id().to_string();
        let _ = std::io::Write::write_all(&mut child.stdin.take().unwrap(), b"from-claude\n");
        assert!(child.wait().unwrap().success());
        let start = Instant::now();
        while !told.exists() {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "the browser did not run"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let text = std::fs::read_to_string(&told).unwrap();
        let (pid, group, stdin) = match text.split_whitespace().collect::<Vec<_>>()[..] {
            [pid, group, stdin] => (pid.to_string(), group.to_string(), stdin.to_string()),
            _ => panic!("{text}"),
        };
        assert_ne!(pid, leader, "the opener did not fork: it kept its session");
        assert_eq!(group, pid, "the browser is not in a session of its own");
        assert_eq!(stdin, "eof");
    }

    #[test]
    fn without_setsid_the_opener_says_why_the_browser_cannot_run() {
        let dir = crate::paths::test_dir("pty-unix-opener-perl-missing");
        let Some(bin) = perl_alone(&dir) else { return };
        let opener = dir.join("open-browser");
        open_browser_apart(&opener, &mut Vec::new()).unwrap();
        let absent = dir.join("absent-browser");
        let out = std::process::Command::new(&opener)
            .arg("https://claude.ai/login")
            .env("PATH", &bin)
            .env(USER_BROWSER_VAR, &absent)
            .env("BROWSER", &opener)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(127));
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(&*absent.to_string_lossy()), "{err}");
        assert!(err.contains("No such file or directory"), "{err}");
    }

    #[test]
    fn only_a_file_that_can_be_run_is_runnable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = crate::paths::test_dir("pty-unix-runnable");
        let file = dir.join("open-browser");
        std::fs::write(&file, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(!runnable(&file));
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(runnable(&file));
        assert!(!runnable(&dir.join("absent")));
    }
}

#[cfg(all(test, windows))]
mod windows_powershell_tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn windows_powershell() -> Option<ShellInfo> {
        let roots = ShellRoots {
            path: vec![],
            program_files: None,
            local_app_data: None,
            system_root: std::env::var_os("SystemRoot").map(PathBuf::from),
        };
        detect_shells_in(&roots, &Settings::default())
            .into_iter()
            .find(|s| s.id == "powershell")
    }

    /// Runs `command` as a launch command; returns its output and exit code.
    fn launch(command: &str) -> Option<(String, Option<u32>)> {
        let _one = one_shell_at_a_time();
        let shell = windows_powershell()?;
        let pty = PtyManager::default();
        let out = Arc::new(Mutex::new(String::new()));
        let exit = Arc::new(Mutex::new(None));
        let (sink, done) = (out.clone(), exit.clone());
        let info = TermInfo {
            id: new_test_id(),
            project_id: "p".into(),
            name: "run".into(),
            shell: shell.id.clone(),
        };
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        pty.spawn_command(
            info,
            &shell,
            "",
            &cwd,
            (120, 30),
            vec![],
            1,
            command,
            move |b| sink.lock().push_str(&String::from_utf8_lossy(&b)),
            move |code| *done.lock() = Some(code),
        )
        .unwrap();
        let start = Instant::now();
        while exit.lock().is_none() {
            assert!(
                start.elapsed() < Duration::from_secs(90),
                "did not end: {}",
                out.lock()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        // Let the reader drain what the pseudo-console still holds.
        std::thread::sleep(Duration::from_millis(300));
        let code = exit.lock().unwrap();
        let text = out.lock().clone();
        Some((text, code))
    }

    fn new_test_id() -> String {
        format!(
            "run-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    #[test]
    fn a_launch_command_runs_to_its_end_and_reports_its_exit_code() {
        let Some((out, code)) = launch("Write-Output ('lancement-' + 'ok'); exit 3") else {
            return;
        };
        assert!(out.contains("lancement-ok"), "{out}");
        assert_eq!(code, Some(3));
    }

    #[test]
    fn a_resized_launch_command_leaves_the_lines_above_its_start_alone() {
        let _one = one_shell_at_a_time();
        let Some(shell) = windows_powershell() else {
            return;
        };
        let pty = PtyManager::default();
        let out = Arc::new(Mutex::new(Vec::<u8>::new()));
        let sink = out.clone();
        let info = TermInfo {
            id: new_test_id(),
            project_id: "p".into(),
            name: "run".into(),
            shell: shell.id.clone(),
        };
        let id = info.id.clone();
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        // The log already holds four lines: the command starts on the fifth.
        pty.spawn_command(
            info,
            &shell,
            "",
            &cwd,
            (80, 20),
            vec![],
            5,
            "Write-Output pret; Start-Sleep 20",
            move |b| sink.lock().extend(b),
            |_| {},
        )
        .unwrap();
        let text = |from: usize| String::from_utf8_lossy(&out.lock()[from..]).to_string();
        let start = Instant::now();
        while !text(0).contains("pret") {
            assert!(start.elapsed() < Duration::from_secs(90), "no output");
            std::thread::sleep(Duration::from_millis(50));
        }
        let n = out.lock().len();
        pty.resize(&id, 100, 25).unwrap();
        // The pseudo-console repaints its screen from where the command started, not from the top.
        let start = Instant::now();
        while !text(n).contains("pret") && start.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(50));
        }
        pty.kill(&id);
        let repaint = text(n);
        assert!(
            screen_writes(&repaint).iter().all(|&row| row >= 5),
            "{repaint:?}"
        );
    }

    /// Starts `command` as a launch command; returns its id, its output so far and its exit code.
    #[allow(clippy::type_complexity)]
    fn start_launch(
        pty: &PtyManager,
        shell: &ShellInfo,
        command: &str,
    ) -> (String, Arc<Mutex<String>>, Arc<Mutex<Option<Option<u32>>>>) {
        let out = Arc::new(Mutex::new(String::new()));
        let exit = Arc::new(Mutex::new(None));
        let (sink, done) = (out.clone(), exit.clone());
        let info = TermInfo {
            id: new_test_id(),
            project_id: "p".into(),
            name: "run".into(),
            shell: shell.id.clone(),
        };
        let id = info.id.clone();
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        pty.spawn_command(
            info,
            shell,
            "",
            &cwd,
            (120, 30),
            vec![],
            1,
            command,
            move |b| sink.lock().push_str(&String::from_utf8_lossy(&b)),
            move |code| *done.lock() = Some(code),
        )
        .unwrap();
        (id, out, exit)
    }

    fn wait_for(what: &str, until: impl Fn() -> bool, out: &Mutex<String>) {
        let start = Instant::now();
        while !until() {
            assert!(
                start.elapsed() < Duration::from_secs(90),
                "{what}: {}",
                out.lock()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// ESC-396: what the window types in a launch command's view reaches its program (an answer to
    /// its prompt).
    #[test]
    fn a_launch_command_reads_what_the_window_types() {
        let _one = one_shell_at_a_time();
        let Some(shell) = windows_powershell() else {
            return;
        };
        let pty = PtyManager::default();
        let (id, out, exit) = start_launch(
            &pty,
            &shell,
            "$r = Read-Host 'Ton nom'; Write-Output ('bonjour-' + $r)",
        );
        wait_for("no prompt", || out.lock().contains("Ton nom"), &out);
        pty.write_from_window(&id, b"escouade\r").unwrap();
        wait_for("did not end", || exit.lock().is_some(), &out);
        std::thread::sleep(Duration::from_millis(300));
        assert!(out.lock().contains("bonjour-escouade"), "{}", out.lock());
        assert_eq!(*exit.lock(), Some(Some(0)));
    }

    /// ESC-396: Ctrl+C typed in a launch command's view stops the program running there.
    #[test]
    fn ctrl_c_typed_in_a_launch_command_stops_it() {
        let _one = one_shell_at_a_time();
        let Some(shell) = windows_powershell() else {
            return;
        };
        let pty = PtyManager::default();
        let (id, out, exit) = start_launch(
            &pty,
            &shell,
            "Write-Output tourne; while ($true) { Start-Sleep -Milliseconds 100 }",
        );
        wait_for("no output", || out.lock().contains("tourne"), &out);
        pty.write_from_window(&id, b"\x03").unwrap();
        wait_for("Ctrl+C did not stop it", || exit.lock().is_some(), &out);
    }

    /// Rows `out` moves the cursor to, or erases from (1 for an erase of the screen).
    fn screen_writes(out: &str) -> Vec<u32> {
        out.split("\x1b[")
            .skip(1)
            .filter_map(|seq| {
                let end = seq.find(|c: char| c.is_ascii_alphabetic())?;
                let first = seq[..end].split(';').next()?.parse().unwrap_or(1);
                match (&seq[end..end + 1], first) {
                    ("H" | "f", row) => Some(row),
                    ("J", 1..=3) => Some(1),
                    _ => None,
                }
            })
            .collect()
    }

    #[test]
    fn a_launch_command_reports_the_exit_code_of_the_program_that_failed() {
        let Some((out, code)) = launch("cmd /c exit 5") else {
            return;
        };
        assert_eq!(code, Some(5), "{out}");
    }

    #[test]
    fn a_launch_command_whose_last_cmdlet_fails_ends_in_error() {
        let Some((out, code)) = launch(r"Get-Item C:\ccm-introuvable-42") else {
            return;
        };
        assert_eq!(code, Some(1), "{out}");
    }

    #[test]
    fn a_launch_command_that_recovers_from_a_failure_ends_with_code_0() {
        let Some((out, code)) = launch("cmd /c exit 5; Write-Output repris") else {
            return;
        };
        assert_eq!(code, Some(0), "{out}");
    }

    #[test]
    fn a_launch_command_that_succeeds_ends_with_code_0() {
        let Some((_, code)) = launch("Write-Output fini") else {
            return;
        };
        assert_eq!(code, Some(0));
    }

    #[test]
    fn a_windows_powershell_terminal_runs_commands() {
        let _one = one_shell_at_a_time();
        let root = std::env::var_os("SystemRoot").map(PathBuf::from);
        let roots = ShellRoots {
            path: vec![],
            program_files: None,
            local_app_data: None,
            system_root: root,
        };
        let Some(shell) = detect_shells_in(&roots, &Settings::default())
            .into_iter()
            .find(|s| s.id == "powershell")
        else {
            eprintln!("Windows PowerShell not found: skipped");
            return;
        };
        let pty = PtyManager::default();
        let out = Arc::new(Mutex::new(String::new()));
        let (sink, answer) = (out.clone(), pty.clone());
        let info = TermInfo {
            id: "wps".into(),
            project_id: "p".into(),
            name: "powershell-1".into(),
            shell: shell.id.clone(),
        };
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let query = Mutex::new(CursorQuery::default());
        pty.spawn(
            info,
            &shell,
            "",
            &cwd,
            (120, 30),
            vec![],
            move |b| {
                if query.lock().feed(&b) {
                    let _ = answer.write("wps", b"\x1b[1;1R");
                }
                sink.lock().push_str(&String::from_utf8_lossy(&b));
            },
            |_| {},
        )
        .unwrap();
        let start = Instant::now();
        let mut sent = false;
        while !out.lock().contains("ccm-ok-42") {
            if !sent && out.lock().contains("PS ") {
                pty.write("wps", b"Write-Output ('ccm-ok-' + 42)\r")
                    .unwrap();
                sent = true;
            }
            assert!(
                start.elapsed() < Duration::from_secs(90),
                "no output: {}",
                out.lock()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        pty.kill("wps");
    }

    /// The window counts the bytes of the messages it gets; the log of a real terminal must have
    /// a read ending at each of those counts, for the echo the window names to be found.
    #[test]
    fn the_count_of_bytes_the_window_received_names_a_read_of_a_real_terminal() {
        let _one = one_shell_at_a_time();
        let root = std::env::var_os("SystemRoot").map(PathBuf::from);
        let roots = ShellRoots {
            path: vec![],
            program_files: None,
            local_app_data: None,
            system_root: root,
        };
        let Some(shell) = detect_shells_in(&roots, &Settings::default())
            .into_iter()
            .find(|s| s.id == "powershell")
        else {
            eprintln!("Windows PowerShell not found: skipped");
            return;
        };
        let pty = PtyManager::default();
        // What the window keeps: each message's text, and the bytes received once it was in.
        let received: Arc<Mutex<Vec<(u64, String)>>> = Arc::default();
        let (sink, answer) = (received.clone(), pty.clone());
        let info = TermInfo {
            id: "wps-count".into(),
            project_id: "p".into(),
            name: "powershell-1".into(),
            shell: shell.id.clone(),
        };
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let query = Mutex::new(CursorQuery::default());
        pty.spawn(
            info,
            &shell,
            "",
            &cwd,
            (120, 30),
            vec![],
            move |b| {
                if query.lock().feed(&b) {
                    let _ = answer.write("wps-count", b"\x1b[1;1R");
                }
                let mut got = sink.lock();
                let end = got.last().map_or(0, |m| m.0) + b.len() as u64;
                got.push((end, String::from_utf8_lossy(&b).into_owned()));
            },
            |_| {},
        )
        .unwrap();
        let seen = |text: &str| received.lock().iter().any(|m| m.1.contains(text));
        let start = Instant::now();
        let mut seq = None;
        while !seen("ccm-ok-42") {
            if seq.is_none() && seen("PS ") {
                seq = Some(
                    pty.write("wps-count", b"Write-Output ('ccm-ok-' + 42)\r")
                        .unwrap(),
                );
            }
            assert!(
                start.elapsed() < Duration::from_secs(90),
                "no output: {:?}",
                received.lock()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        let term = pty.term("wps-count").unwrap();
        let ends: Vec<u64> = received.lock().iter().map(|m| m.0).collect();
        // The reader logs a read just after sending it: wait for the last one.
        let last = *ends.last().unwrap();
        let waiting = Instant::now();
        while term.writes.read_ending_at(last).is_none() {
            assert!(
                waiting.elapsed() < Duration::from_secs(10),
                "the last read is not logged"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        // Every count of the window is the end of one read (the log keeps the last 512, as many
        // messages as it checks).
        for end in ends.iter().rev().take(RECENT_READS) {
            assert!(
                term.writes.read_ending_at(*end).is_some(),
                "no read ends at {end} bytes, among the messages {ends:?}"
            );
        }
        // The line for the window's count at the message with the command's output.
        let output_end = received
            .lock()
            .iter()
            .find(|m| m.1.contains("ccm-ok-42"))
            .map(|m| m.0)
            .unwrap();
        let report: TermReport = serde_json::from_str(&format!(
            r#"{{"kind":"slowEcho","seq":{},"bytes":30,"sentToReceived":500,"receivedToPainted":10,"outputEnd":{output_end}}}"#,
            seq.unwrap()
        ))
        .unwrap();
        let (_, line) = term.report_line(report, 10_000).unwrap();
        assert!(
            line.contains(", the echo ") && !line.contains("not identified"),
            "{line}"
        );
        pty.kill("wps-count");
    }
}

#[cfg(test)]
mod cursor_query_tests {
    use super::*;

    /// Whether the query is found in the last of `reads`, which `CursorQuery` has seen in order.
    fn answers(reads: &[&str]) -> Vec<bool> {
        let mut query = CursorQuery::default();
        reads.iter().map(|r| query.feed(r.as_bytes())).collect()
    }

    #[test]
    fn a_query_inside_one_read_is_found() {
        assert_eq!(answers(&["\x1b[6n"]), [true]);
        assert_eq!(answers(&["PS C:\\> \x1b[6n\x1b[?25h"]), [true]);
    }

    #[test]
    fn a_query_cut_between_two_reads_is_found_once_complete() {
        assert_eq!(answers(&["\x1b[", "6n"]), [false, true]);
        assert_eq!(answers(&["\x1b", "[6n"]), [false, true]);
        assert_eq!(answers(&["\x1b[6", "n"]), [false, true]);
        // Cut in three, the middle read holding a single byte.
        assert_eq!(answers(&["ab\x1b", "[", "6n cd"]), [false, false, true]);
    }

    #[test]
    fn a_query_is_found_once_not_again_in_the_reads_after_it() {
        assert_eq!(answers(&["\x1b[6n", "x", "[6n"]), [true, false, false]);
        assert_eq!(answers(&["\x1b[6n", "\x1b[6n"]), [true, true]);
    }

    #[test]
    fn other_sequences_are_not_queries() {
        assert_eq!(answers(&["\x1b[5n", "\x1b[6", "m", "[6n"]), [false; 4]);
        assert_eq!(
            answers(&["", "\x1b[", "", "6n"]),
            [false, false, false, true]
        );
    }
}

#[cfg(test)]
mod run_cwd_tests {
    use super::*;

    #[test]
    fn a_missing_folder_and_a_closed_terminal_read_in_english() {
        use crate::i18n::Lang::En;
        assert_eq!(
            missing_folder(En, "api"),
            "The folder “api” doesn’t exist in the project"
        );
        assert_eq!(closed(En).to_string(), "terminal closed");
    }

    #[test]
    fn a_launch_command_runs_in_the_project_or_one_of_its_folders() {
        let d = crate::paths::test_dir("run-cwd");
        std::fs::create_dir_all(d.join("web")).unwrap();
        let root = d.to_string_lossy().to_string();
        assert_eq!(run_cwd(&root, "").unwrap(), root);
        assert_eq!(run_cwd(&root, "  ").unwrap(), root);
        assert_eq!(
            run_cwd(&root, "web").unwrap(),
            d.join("web").to_string_lossy()
        );
        let err = run_cwd(&root, "api").unwrap_err().to_string();
        assert!(err.contains("api"), "{err}");
    }
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// A shell of this machine that runs a launch command the way the app does.
    fn shell() -> Option<ShellInfo> {
        detect_shells(&Settings::default())
            .into_iter()
            .find(|s| matches!(s.id.as_str(), "pwsh" | "powershell" | "bash" | "zsh"))
    }

    fn info(id: &str, shell: &ShellInfo) -> TermInfo {
        TermInfo {
            id: id.into(),
            project_id: "p".into(),
            name: id.into(),
            shell: shell.id.clone(),
        }
    }

    #[test]
    fn a_launch_counts_while_it_runs_and_a_terminal_by_its_last_input_or_output() {
        let _one = one_shell_at_a_time();
        let Some(sh) = shell() else {
            eprintln!("no shell found: skipped");
            return;
        };
        let pty = PtyManager::default();
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        assert_eq!(pty.activity(), Activity::default());

        let pause = match sh.id.as_str() {
            "pwsh" | "powershell" => "Start-Sleep -Milliseconds 1500",
            _ => "sleep 1.5",
        };
        let ended = Arc::new(Mutex::new(false));
        let done = ended.clone();
        pty.spawn_command(
            info("act-run", &sh),
            &sh,
            "",
            &cwd,
            (80, 24),
            vec![],
            1,
            pause,
            |_| {},
            move |_| *done.lock() = true,
        )
        .unwrap();
        // A launch is no interactive terminal: it counts while it runs, whatever it prints.
        assert_eq!(
            pty.activity(),
            Activity {
                launches: 1,
                shell_io_at: None
            }
        );
        let start = Instant::now();
        while !*ended.lock() {
            assert!(start.elapsed() < Duration::from_secs(60), "did not end");
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(pty.activity().launches, 0);

        let opened = now_ms();
        pty.spawn(
            info("act-shell", &sh),
            &sh,
            "",
            &cwd,
            (80, 24),
            vec![],
            |_| {},
            |_| {},
        )
        .unwrap();
        let first = pty.activity().shell_io_at.expect("a terminal open");
        assert!(first >= opened);
        std::thread::sleep(Duration::from_millis(30));
        let typed = now_ms();
        pty.write("act-shell", b" ").unwrap();
        assert!(pty.activity().shell_io_at.unwrap() >= typed);
        assert_eq!(pty.activity().launches, 0);
        pty.kill("act-shell");
        assert_eq!(pty.activity(), Activity::default());
    }
}

#[cfg(test)]
mod write_log_tests {
    use super::*;

    fn record(bytes: usize, queued_us: i64) -> WriteRecord {
        WriteRecord {
            seq: 0,
            bytes,
            queued_us,
            started_us: None,
            written_us: None,
            first_output_us: None,
        }
    }

    #[test]
    fn a_write_then_output_marks_the_first_output_of_the_write() {
        let log = WriteLog::default();
        let seq = log.queued(3, 1_000);
        log.started([seq], 1_100);
        log.written([seq], 1_200);
        log.output(1_500, 4);
        log.output(1_900, 4);
        assert_eq!(
            log.all(),
            [WriteRecord {
                seq,
                bytes: 3,
                queued_us: 1_000,
                started_us: Some(1_100),
                written_us: Some(1_200),
                first_output_us: Some(1_500),
            }]
        );
    }

    #[test]
    fn output_before_a_write_is_not_its_echo() {
        let log = WriteLog::default();
        log.output(900, 4);
        // Queued, but the input thread has not begun to write it.
        let queued = log.queued(1, 1_000);
        log.output(1_050, 4);
        let started = log.queued(1, 1_060);
        log.started([started], 1_100);
        // Read before the write began, but reported after: not its echo either.
        log.output(1_090, 4);
        let all = log.all();
        assert_eq!(all[0].seq, queued);
        assert_eq!(all[0].first_output_us, None);
        assert_eq!(all[1].first_output_us, None);
        log.output(1_300, 4);
        assert_eq!(log.all()[1].first_output_us, Some(1_300));
    }

    #[test]
    fn the_echo_read_before_the_write_returns_still_counts() {
        // The program echoes as soon as the bytes are in its pipe: the reader may get it before
        // the input thread is done with the write call.
        let log = WriteLog::default();
        let seq = log.queued(1, 1_000);
        log.started([seq], 1_100);
        log.output(1_150, 4);
        log.written([seq], 1_200);
        let r = &log.all()[0];
        assert_eq!(
            (r.written_us, r.first_output_us),
            (Some(1_200), Some(1_150))
        );
    }

    #[test]
    fn the_last_64_writes_are_kept_and_found_again_by_their_number() {
        let log = WriteLog::default();
        let seqs: Vec<u64> = (0..100).map(|i| log.queued(1, i * 10)).collect();
        assert_eq!(log.all().len(), RECENT_WRITES);
        assert_eq!(log.all()[0].seq, seqs[100 - RECENT_WRITES]);
        // One that left the ring is ignored, the others are marked.
        log.written(seqs.iter().copied(), 5_000);
        let all = log.all();
        assert!(all.iter().all(|r| r.written_us == Some(5_000)));
        assert_eq!(all.last().unwrap().seq, seqs[99]);
    }

    #[test]
    fn a_report_finds_its_write_by_number() {
        let log = WriteLog::default();
        let seqs: Vec<u64> = (0..3)
            .map(|i| log.queued(i + 1, 1_000 * i as i64))
            .collect();
        for (i, seq) in seqs.iter().enumerate() {
            let found = log.get(*seq).unwrap();
            assert_eq!((found.seq, found.bytes), (*seq, i + 1));
        }
        assert_eq!(log.get(seqs[2] + 1), None);
        assert_eq!(WriteLog::default().get(0), None);
    }

    #[test]
    fn a_write_that_left_the_ring_is_not_found_by_its_number() {
        let log = WriteLog::default();
        let seqs: Vec<u64> = (0..RECENT_WRITES as u64 + 10)
            .map(|_| log.queued(1, 0))
            .collect();
        assert_eq!(log.get(seqs[0]), None);
        assert_eq!(log.get(seqs[9]), None);
        assert_eq!(log.get(seqs[10]).unwrap().seq, seqs[10]);
        assert_eq!(
            log.get(*seqs.last().unwrap()).unwrap().seq,
            seqs[seqs.len() - 1]
        );
    }

    #[test]
    fn each_read_is_found_again_by_the_bytes_read_up_to_and_including_it() {
        let log = WriteLog::default();
        // Reads of 5, 3 and 10 bytes: what the window has received after each of its messages.
        log.output(100, 5);
        log.output(200, 3);
        log.output(300, 10);
        let at = |end| log.read_ending_at(end).map(|r| r.at_us);
        assert_eq!(at(5), Some(100));
        assert_eq!(at(8), Some(200));
        assert_eq!(at(18), Some(300));
        assert_eq!(
            log.read_ending_at(8),
            Some(OutputRead { end: 8, at_us: 200 })
        );
    }

    #[test]
    fn a_count_that_is_not_the_end_of_a_read_finds_none() {
        let log = WriteLog::default();
        assert_eq!(log.read_ending_at(0), None);
        assert_eq!(log.read_ending_at(5), None);
        log.output(100, 5);
        log.output(200, 3);
        // Nothing yet, in the middle of a read, between two, past the last.
        for end in [0, 1, 4, 6, 7, 9, 18, u64::MAX] {
            assert_eq!(log.read_ending_at(end), None, "{end}");
        }
    }

    #[test]
    fn the_last_512_reads_are_kept_and_the_count_goes_on_past_them() {
        let log = WriteLog::default();
        let total = RECENT_READS + 10;
        for i in 1..=total {
            log.output(i as i64 * 10, 2);
        }
        // Read number 1 to 10 left the ring; number 11 is the oldest it holds.
        assert_eq!(log.read_ending_at(2), None);
        assert_eq!(log.read_ending_at(20), None);
        assert_eq!(log.read_ending_at(22).map(|r| r.at_us), Some(110));
        // The count never restarted: the last read ends at all the bytes read since the start.
        let last = log.read_ending_at(2 * total as u64).unwrap();
        assert_eq!(last.at_us, total as i64 * 10);
        assert_eq!(log.state.lock().reads.len(), RECENT_READS);
    }

    #[test]
    fn a_read_of_no_bytes_leaves_no_read_to_find() {
        let log = WriteLog::default();
        log.output(100, 4);
        log.output(150, 0);
        log.output(200, 4);
        assert_eq!(log.read_ending_at(4).map(|r| r.at_us), Some(100));
        assert_eq!(log.read_ending_at(8).map(|r| r.at_us), Some(200));
        assert_eq!(log.state.lock().reads.len(), 2);
    }

    #[test]
    fn reads_and_writes_are_logged_apart() {
        // A write does not move the count of output, nor a read the numbers of the writes.
        let log = WriteLog::default();
        log.output(100, 7);
        let seq = log.queued(3, 150);
        log.output(200, 7);
        assert_eq!(seq, 0);
        assert_eq!(log.read_ending_at(14).map(|r| r.at_us), Some(200));
        assert_eq!(log.all().len(), 1);
    }

    fn full(queued: i64, written: i64, output: i64) -> WriteRecord {
        WriteRecord {
            started_us: Some(queued),
            written_us: Some(written),
            first_output_us: Some(output),
            ..record(1, queued)
        }
    }

    #[test]
    fn a_slow_echo_is_split_into_the_steps_of_its_way() {
        // Written 2 ms after the backend got it, the echo read 1490 ms after; it was in the window
        // 1498 ms after the write left it, so 6 ms went to and from the window; then 22 ms to
        // paint.
        let w = full(500_000, 502_000, 503_000);
        let line = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1498.0,
            22.0,
            None,
            Some(&w),
            Some(1_992_000),
        );
        assert_eq!(
            line,
            "terminal t1 (pwsh): echo of a 1-byte write shown after 1520 ms: written 2 ms after \
             the backend got it, first output 1 ms after, the echo 1490 ms after, to and from the \
             window 6 ms, painted 22 ms later"
        );
    }

    #[test]
    fn the_echo_is_dated_by_its_own_read_not_by_the_first_output_that_was_only_the_cursor() {
        // ConPTY hides the cursor 1 ms after the write; the letter is drawn 1490 ms after. The
        // window got the letter 1498 ms after the write left it: the way to and from it is 6 ms,
        // not the 1497 ms the first output would blame on it.
        let w = full(500_000, 502_000, 503_000);
        let line = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1498.0,
            22.0,
            None,
            Some(&w),
            Some(1_992_000),
        );
        assert!(
            line.contains(
                "first output 1 ms after, the echo 1490 ms after, to and from the window 6 ms,"
            ),
            "{line}"
        );
        // The same write, its echo not identified: the first output stands for it, as before.
        let line = slow_echo_line("terminal t1 (pwsh)", 1, 1498.0, 22.0, None, Some(&w), None);
        assert_eq!(
            line,
            "terminal t1 (pwsh): echo of a 1-byte write shown after 1520 ms: written 2 ms after \
             the backend got it, first output 1 ms after, the echo not identified, to and from \
             the window 1495 ms, painted 22 ms later"
        );
    }

    #[test]
    fn the_answer_to_the_write_is_told_when_the_window_gave_it() {
        let w = full(500_000, 502_000, 503_000);
        let identified = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1498.0,
            22.0,
            Some(8.4),
            Some(&w),
            Some(1_992_000),
        );
        assert_eq!(
            identified,
            "terminal t1 (pwsh): echo of a 1-byte write shown after 1520 ms: written 2 ms after \
             the backend got it, first output 1 ms after, the echo 1490 ms after, to and from the \
             window 6 ms (answer to the write 8 ms), painted 22 ms later"
        );
        // Not identified: the answer is still told.
        let unknown = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1498.0,
            22.0,
            Some(0.4),
            Some(&w),
            None,
        );
        assert!(
            unknown.contains(
                "the echo not identified, to and from the window 1495 ms (answer to the write 0 \
                 ms), painted 22 ms later"
            ),
            "{unknown}"
        );
        // A negative answer (two clocks of the window's own rounding) is never shown as such.
        let rounded = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1498.0,
            22.0,
            Some(-0.2),
            Some(&w),
            Some(1_992_000),
        );
        assert!(rounded.contains("(answer to the write 0 ms)"), "{rounded}");
    }

    #[test]
    fn an_echo_read_before_the_write_returned_is_not_a_negative_step() {
        // The program echoed as soon as its pipe had the bytes: the reader got the echo before
        // the input thread was done with the write call.
        let w = full(500_000, 502_000, 501_000);
        let line = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1000.0,
            20.0,
            None,
            Some(&w),
            Some(501_500),
        );
        assert!(!line.contains(" -"), "{line}");
        assert!(
            line.contains(
                "first output 0 ms after, the echo 0 ms after, to and from the window 999 ms,"
            ),
            "{line}"
        );
    }

    #[test]
    fn an_echo_read_changes_nothing_when_the_write_is_not_fully_known() {
        // Not written yet, or the first output never seen: the variants of before, the echo read
        // (a stray one, for such a record) is ignored.
        let mut w = full(500_000, 0, 0);
        w.written_us = None;
        let line = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1498.0,
            22.0,
            Some(5.0),
            Some(&w),
            Some(900_000),
        );
        assert!(line.contains("not written yet, first output seen, in the window 1498 ms after the write left it, painted 22 ms later"), "{line}");
        let mut w = full(500_000, 503_000, 0);
        w.first_output_us = None;
        let line = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1498.0,
            22.0,
            Some(5.0),
            Some(&w),
            Some(900_000),
        );
        assert!(line.contains("first output not seen, in the window 1498 ms after the write left it, painted 22 ms later"), "{line}");
        assert!(!line.contains("the echo"), "{line}");
    }

    #[test]
    fn a_slow_echo_of_a_write_the_backend_forgot_says_its_times_are_unknown() {
        // Whatever the window said of the echo's read and the answer.
        let line = slow_echo_line(
            "terminal t1 (pwsh)",
            12,
            1498.5,
            21.5,
            Some(3.0),
            None,
            Some(1_000_000),
        );
        assert_eq!(
            line,
            "terminal t1 (pwsh): echo of a 12-byte write shown after 1520 ms: backend times \
             unknown, in the window 1499 ms after the write left it, painted 22 ms later"
        );
    }

    #[test]
    fn a_slow_echo_whose_first_output_was_not_seen_says_what_is_known() {
        let mut w = full(500_000, 503_000, 0);
        w.first_output_us = None;
        let line = slow_echo_line("terminal t1 (pwsh)", 1, 1498.0, 22.0, None, Some(&w), None);
        assert_eq!(
            line,
            "terminal t1 (pwsh): echo of a 1-byte write shown after 1520 ms: written 3 ms after \
             the backend got it, first output not seen, in the window 1498 ms after the write \
             left it, painted 22 ms later"
        );
    }

    #[test]
    fn a_slow_echo_of_a_write_not_written_yet_says_so() {
        let mut w = full(500_000, 0, 0);
        w.written_us = None;
        w.first_output_us = None;
        let line = slow_echo_line("terminal t1 (pwsh)", 1, 1498.0, 22.0, None, Some(&w), None);
        assert_eq!(
            line,
            "terminal t1 (pwsh): echo of a 1-byte write shown after 1520 ms: not written yet, \
             first output not seen, in the window 1498 ms after the write left it, painted 22 ms \
             later"
        );
        // An echo read before the write call returned: still not a time to split.
        w.first_output_us = Some(600_000);
        let line = slow_echo_line("terminal t1 (pwsh)", 1, 1498.0, 22.0, None, Some(&w), None);
        assert!(line.contains("not written yet"), "{line}");
        assert!(!line.contains("to and from"), "{line}");
    }

    #[test]
    fn a_backend_part_longer_than_the_echo_took_in_the_window_never_shows_a_negative_step() {
        // The window's durations and the backend's times are two clocks: nothing says one side's
        // part fits in the other's whole.
        let w = full(0, 100, 5_000_000);
        let line = slow_echo_line("terminal t1 (pwsh)", 1, 1000.0, 20.0, None, Some(&w), None);
        assert!(!line.contains(" -"), "{line}");
        assert!(line.contains("to and from the window 0 ms"), "{line}");
        // The same with the echo's own read, five seconds after the write.
        let line = slow_echo_line(
            "terminal t1 (pwsh)",
            1,
            1000.0,
            20.0,
            None,
            Some(&w),
            Some(5_000_000),
        );
        assert!(!line.contains(" -"), "{line}");
        assert!(line.contains("to and from the window 0 ms"), "{line}");
    }

    #[test]
    fn the_windows_reports_are_read_from_their_json() {
        // Without what the window learned later: an older window's report.
        let slow: TermReport = serde_json::from_str(
            r#"{"kind":"slowEcho","seq":7,"bytes":4,"sentToReceived":1500.5,"receivedToPainted":20.25}"#,
        )
        .unwrap();
        assert!(matches!(
            slow,
            TermReport::SlowEcho {
                seq: 7,
                bytes: 4,
                sent_to_received,
                received_to_painted,
                sent_to_acked: None,
                output_end: None,
            } if sent_to_received == 1500.5 && received_to_painted == 20.25
        ));
        let full: TermReport = serde_json::from_str(
            r#"{"kind":"slowEcho","seq":7,"bytes":4,"sentToReceived":1500.5,"receivedToPainted":20.25,"sentToAcked":3.75,"outputEnd":12345}"#,
        )
        .unwrap();
        assert!(matches!(
            full,
            TermReport::SlowEcho {
                seq: 7,
                sent_to_received,
                received_to_painted,
                sent_to_acked: Some(acked),
                output_end: Some(12_345),
                ..
            } if sent_to_received == 1500.5 && received_to_painted == 20.25 && acked == 3.75
        ));
        // A window that knows neither says so with nulls.
        let nulls: TermReport = serde_json::from_str(
            r#"{"kind":"slowEcho","seq":7,"bytes":4,"sentToReceived":1.0,"receivedToPainted":2.0,"sentToAcked":null,"outputEnd":null}"#,
        )
        .unwrap();
        assert!(matches!(
            nulls,
            TermReport::SlowEcho {
                sent_to_acked: None,
                output_end: None,
                ..
            }
        ));
        // Only one of the two.
        let only_end: TermReport = serde_json::from_str(
            r#"{"kind":"slowEcho","seq":7,"bytes":4,"sentToReceived":1.0,"receivedToPainted":2.0,"outputEnd":9}"#,
        )
        .unwrap();
        assert!(matches!(
            only_end,
            TermReport::SlowEcho {
                sent_to_acked: None,
                output_end: Some(9),
                ..
            }
        ));
        let webgl: TermReport = serde_json::from_str(
            r#"{"kind":"renderer","renderer":"webgl","detail":"ANGLE (NVIDIA)"}"#,
        )
        .unwrap();
        assert!(matches!(
            webgl,
            TermReport::Renderer { renderer, detail: Some(d) } if renderer == "webgl" && d == "ANGLE (NVIDIA)"
        ));
        // No detail at all.
        let dom: TermReport =
            serde_json::from_str(r#"{"kind":"renderer","renderer":"dom"}"#).unwrap();
        assert!(matches!(dom, TermReport::Renderer { detail: None, .. }));
    }
}

/// What `ChildKiller` is for a terminal that has no process (tests).
#[cfg(test)]
#[derive(Debug)]
struct NoKiller;

#[cfg(test)]
impl ChildKiller for NoKiller {
    fn kill(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(NoKiller)
    }
}

#[cfg(test)]
impl PtyManager {
    /// A terminal `id` with a pseudo-console but no process, whose input goes to `writer`.
    fn insert_fake(&self, id: &str, writer: Box<dyn Write + Send>) {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let writes = Arc::new(WriteLog::default());
        let (input, pending) = spawn_input(id, writer, writes.clone(), Arc::default()).unwrap();
        self.terms.lock().insert(
            id.into(),
            Arc::new(Term {
                info: TermInfo {
                    id: id.into(),
                    project_id: "p".into(),
                    name: id.into(),
                    shell: "fake".into(),
                },
                master: Mutex::new(Some(pair.master)),
                input,
                pending,
                killer: Mutex::new(Box::new(NoKiller)),
                job: None,
                launch: false,
                spare_windows: false,
                detached_survive: false,
                io_at: Arc::new(AtomicI64::new(now_ms())),
                writes,
                slow_echo_at: AtomicI64::new(i64::MIN),
                typed: Arc::default(),
            }),
        );
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    type Seen = Arc<Mutex<Vec<u8>>>;
    /// What each call to a writer was given.
    type Calls = Arc<Mutex<Vec<Vec<u8>>>>;

    /// A writer that does not return until its gate is opened (a message, or the sender dropped
    /// when the test is over, whatever the way it ends), as a program that does not read its
    /// input; what it is given is in `seen`, once it was let through, and each call apart in
    /// `calls`.
    struct Gated {
        gate: mpsc::Receiver<()>,
        open: bool,
        seen: Seen,
        calls: Calls,
    }

    impl Write for Gated {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if !self.open {
                let _ = self.gate.recv();
                self.open = true;
            }
            // `calls` first: a test that waits for `seen` then reads `calls`.
            self.calls.lock().push(buf.to_vec());
            self.seen.lock().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn gated() -> (Gated, mpsc::Sender<()>, Seen) {
        let (writer, open, seen, _) = gated_recording();
        (writer, open, seen)
    }

    fn gated_recording() -> (Gated, mpsc::Sender<()>, Seen, Calls) {
        let (open, gate) = mpsc::channel();
        let (seen, calls) = (Seen::default(), Calls::default());
        let writer = Gated {
            gate,
            open: false,
            seen: seen.clone(),
            calls: calls.clone(),
        };
        (writer, open, seen, calls)
    }

    /// A program that reads its input.
    struct Recorder(Seen);

    impl Write for Recorder {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A program that ended: its input is a broken pipe.
    struct Broken;

    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Says when it is dropped.
    struct Dropped(mpsc::Sender<()>);

    impl Write for Dropped {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Drop for Dropped {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    /// `f`'s result, or None if it did not return within `ms` (it goes on in its thread).
    fn within<T: Send + 'static>(ms: u64, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        rx.recv_timeout(Duration::from_millis(ms)).ok()
    }

    /// Waits for `seen` to hold `len` bytes.
    fn wait_for(seen: &Seen, len: usize) {
        let start = Instant::now();
        while seen.lock().len() < len {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "the writer got {} of {len} bytes",
                seen.lock().len()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn writes_queue_at_once_while_the_program_reads_nothing_and_arrive_in_order() {
        let pty = PtyManager::default();
        let (writer, open, seen) = gated();
        pty.insert_fake("t", Box::new(writer));
        let chunks: Vec<Vec<u8>> = (0..200).map(|i| format!("<{i}>").into_bytes()).collect();
        let queuing = pty.clone();
        let sent = chunks.clone();
        let took = within(5_000, move || {
            let start = Instant::now();
            for chunk in &sent {
                queuing.write("t", chunk).unwrap();
            }
            start.elapsed()
        });
        let took = took.expect("the writes wait for the program to read its input");
        // Generous: a loaded CI runner. That the writes do not wait is proved by the closed gate.
        assert!(took < Duration::from_secs(2), "{took:?} for 200 writes");
        assert!(seen.lock().is_empty());

        open.send(()).unwrap();
        let all: Vec<u8> = chunks.concat();
        wait_for(&seen, all.len());
        assert_eq!(*seen.lock(), all);
    }

    #[test]
    fn a_program_that_ended_closes_its_terminal_for_the_next_write() {
        let pty = PtyManager::default();
        pty.insert_fake("t", Box::new(Broken));
        // The first write only queues; the input thread then finds the pipe broken and ends.
        let _ = pty.write("t", b"x");
        let start = Instant::now();
        let err = loop {
            match pty.write("t", b"y") {
                Err(e) => break e,
                Ok(_) => {
                    assert!(
                        start.elapsed() < Duration::from_secs(5),
                        "a broken input never closes the terminal"
                    );
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        };
        assert_eq!(err.to_string(), closed(i18n::ui()).to_string());
    }

    #[test]
    fn a_terminal_that_does_not_read_blocks_no_other_terminal_nor_the_manager() {
        let pty = PtyManager::default();
        let (stuck, open, stuck_seen) = gated();
        let seen = Seen::default();
        pty.insert_fake("a", Box::new(stuck));
        pty.insert_fake("b", Box::new(Recorder(seen.clone())));

        let p = pty.clone();
        assert!(
            within(5_000, move || p.write("a", b"stuck")).is_some(),
            "a write waits for the program to read its input"
        );
        let p = pty.clone();
        assert!(within(5_000, move || p.write("b", b"hello")).is_some());
        wait_for(&seen, 5);
        assert_eq!(*seen.lock(), b"hello");
        assert!(stuck_seen.lock().is_empty(), "a is meant to stay stuck");

        let p = pty.clone();
        assert!(
            within(5_000, move || p.resize("a", 100, 30)).is_some(),
            "a resize waits for the writes of its terminal"
        );
        let p = pty.clone();
        assert!(within(5_000, move || p.list().len()).is_some_and(|n| n == 2));
        let p = pty.clone();
        assert!(within(5_000, move || p.activity()).is_some());
        open.send(()).unwrap();
        wait_for(&stuck_seen, 5);
    }

    #[test]
    fn closing_a_terminal_ends_its_input_thread() {
        let pty = PtyManager::default();
        let (dropped, gone) = mpsc::channel();
        pty.insert_fake("t", Box::new(Dropped(dropped)));
        pty.write("t", b"x").unwrap();
        pty.kill("t");
        gone.recv_timeout(Duration::from_secs(5))
            .expect("the input thread outlived its terminal");
        assert!(pty.write("t", b"y").is_err());
    }

    /// A terminal `t` whose program reads everything.
    fn reading_terminal() -> (PtyManager, Arc<Term>) {
        let pty = PtyManager::default();
        pty.insert_fake("t", Box::new(Recorder(Seen::default())));
        let term = pty.term("t").unwrap();
        (pty, term)
    }

    fn renderer(renderer: &str, detail: Option<&str>) -> TermReport {
        TermReport::Renderer {
            renderer: renderer.into(),
            detail: detail.map(Into::into),
        }
    }

    #[test]
    fn the_renderer_the_window_reports_goes_to_the_log_with_its_detail() {
        let (_pty, term) = reading_terminal();
        let (level, line) = term
            .report_line(renderer("webgl", Some("ANGLE (NVIDIA)")), 0)
            .unwrap();
        assert_eq!(level, log::Level::Info);
        assert_eq!(line, "terminal t (fake): renderer webgl (ANGLE (NVIDIA))");
        let line = |detail| term.report_line(renderer("dom", detail), 0).unwrap().1;
        assert_eq!(line(None), "terminal t (fake): renderer dom");
        assert_eq!(line(Some("")), "terminal t (fake): renderer dom");
        assert_eq!(
            line(Some("context lost")),
            "terminal t (fake): renderer dom (context lost)"
        );
    }

    #[test]
    fn what_the_window_says_of_its_renderer_goes_to_the_log_on_one_line_and_cut() {
        let (_pty, term) = reading_terminal();
        let detail = format!("ANGLE\r\n(forged line)\x1b[31m{}", "x".repeat(500));
        let (_, line) = term
            .report_line(renderer("webgl\nWARN forged", Some(&detail)), 0)
            .unwrap();
        assert!(!line.chars().any(char::is_control), "{line:?}");
        assert!(line.starts_with(
            "terminal t (fake): renderer webglWARN forged (ANGLE(forged line)[31mxxx"
        ));
        let shown = &line[line.find(" (ANGLE").unwrap() + 2..line.len() - 1];
        assert_eq!(shown.chars().count(), 200);
    }

    #[test]
    fn a_terminal_that_reads_nothing_refuses_writes_beyond_32_mb_until_it_reads_again() {
        let pty = PtyManager::default();
        let (writer, open, seen) = gated();
        pty.insert_fake("t", Box::new(writer));
        let chunk = vec![b'x'; MAX_PENDING / 8];
        for _ in 0..8 {
            pty.write("t", &chunk).unwrap();
        }
        let refused = pty.write("t", b"y").unwrap_err().to_string();
        assert!(
            refused == "le terminal ne lit plus son entrée"
                || refused == "the terminal no longer reads its input",
            "{refused}"
        );
        open.send(()).unwrap();
        wait_for(&seen, MAX_PENDING);
        // Once written, the backlog is gone and the terminal takes writes again.
        let again = pty.clone();
        let taken = within(5_000, move || loop {
            if again.write("t", b"z").is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        });
        assert!(
            taken.is_some(),
            "the terminal still refuses writes once it read"
        );
        wait_for(&seen, MAX_PENDING + 1);
        assert_eq!(seen.lock().last(), Some(&b'z'));
    }

    fn slow(seq: u64) -> TermReport {
        TermReport::SlowEcho {
            seq,
            bytes: 1,
            sent_to_received: 1500.0,
            received_to_painted: 20.0,
            sent_to_acked: None,
            output_end: None,
        }
    }

    #[test]
    fn a_write_returns_its_number_in_the_terminals_log() {
        let pty = PtyManager::default();
        let (writer, open, _) = gated();
        pty.insert_fake("t", Box::new(writer));
        let term = pty.term("t").unwrap();
        let seqs: Vec<u64> = [&b"a"[..], b"bb", b"ccc"]
            .into_iter()
            .map(|data| pty.write("t", data).unwrap())
            .collect();
        assert_eq!(seqs, [0, 1, 2]);
        let bytes: Vec<usize> = seqs
            .iter()
            .map(|seq| term.writes.get(*seq).unwrap().bytes)
            .collect();
        assert_eq!(bytes, [1, 2, 3]);
        drop(open);
    }

    #[test]
    fn a_slow_echo_is_matched_with_its_write_by_number_not_by_time() {
        let (_pty, term) = reading_terminal();
        // Two writes, in the log as the input thread and the reader would leave them.
        let (a, b) = (term.writes.queued(1, 0), term.writes.queued(1, 10_000));
        term.writes.started([a], 0);
        term.writes.written([a], 2_000);
        term.writes.started([b], 10_000);
        term.writes.written([b], 17_000);
        term.writes.output(1_000_000, 4);
        let (level, line) = term.report_line(slow(b), 10_000).unwrap();
        assert_eq!(level, log::Level::Warn);
        assert_eq!(
            line,
            "terminal t (fake): echo of a 1-byte write shown after 1520 ms: written 7 ms after \
             the backend got it, first output 983 ms after, the echo not identified, to and from \
             the window 510 ms, painted 20 ms later"
        );
        // The other one, long after (the report says nothing of when).
        let (_, line) = term.report_line(slow(a), 20_000).unwrap();
        assert!(
            line.contains("written 2 ms after the backend got it, first output 998 ms after"),
            "{line}"
        );
    }

    /// The report the window sends for the echo of write `seq`, as its JSON gives it: `end` is
    /// the count of output bytes it had received up to the chunk with the echo.
    fn slow_json(seq: u64, end: u64) -> TermReport {
        serde_json::from_str(&format!(
            r#"{{"kind":"slowEcho","seq":{seq},"bytes":1,"sentToReceived":1498,"receivedToPainted":22,"sentToAcked":4,"outputEnd":{end}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn a_slow_echo_is_dated_by_the_read_the_window_names_by_the_bytes_it_had_received() {
        let (_pty, term) = reading_terminal();
        let seq = term.writes.queued(1, 500_000);
        term.writes.started([seq], 500_100);
        term.writes.written([seq], 502_000);
        // The output as the reader thread logs it: a prompt before the key (100 bytes), ConPTY
        // hiding the cursor 1 ms after the write (6), the letter 1490 ms after (12), then more.
        for (at, bytes) in [
            (400_000, 100),
            (503_000, 6),
            (1_992_000, 12),
            (2_100_000, 40),
        ] {
            term.writes.output(at, bytes);
        }
        // The window had received 100 + 6 + 12 bytes when the chunk with the letter came in.
        let (level, line) = term.report_line(slow_json(seq, 118), 10_000).unwrap();
        assert_eq!(level, log::Level::Warn);
        assert_eq!(
            line,
            "terminal t (fake): echo of a 1-byte write shown after 1520 ms: written 2 ms after \
             the backend got it, first output 1 ms after, the echo 1490 ms after, to and from \
             the window 6 ms (answer to the write 4 ms), painted 22 ms later"
        );
        // A count that is the end of no read (a window that restarted its count, say): the echo
        // is not identified, and the first output stands for it.
        let (_, line) = term.report_line(slow_json(seq, 117), 20_000).unwrap();
        assert_eq!(
            line,
            "terminal t (fake): echo of a 1-byte write shown after 1520 ms: written 2 ms after \
             the backend got it, first output 1 ms after, the echo not identified, to and from \
             the window 1495 ms (answer to the write 4 ms), painted 22 ms later"
        );
        // The cursor's chunk is a read like another: naming it dates it.
        let (_, line) = term.report_line(slow_json(seq, 106), 30_000).unwrap();
        assert!(line.contains("the echo 1 ms after"), "{line}");
        // A report of an older window has neither: as before, plus what is not known.
        let old = TermReport::SlowEcho {
            seq,
            bytes: 1,
            sent_to_received: 1498.0,
            received_to_painted: 22.0,
            sent_to_acked: None,
            output_end: None,
        };
        let (_, line) = term.report_line(old, 40_000).unwrap();
        assert!(
            line.contains("the echo not identified, to and from the window 1495 ms, painted"),
            "{line}"
        );
    }

    /// The number of ms that follows `before` in a log line.
    fn ms_after(line: &str, before: &str) -> i64 {
        let rest = line
            .split(before)
            .nth(1)
            .unwrap_or_else(|| panic!("no {before:?} in {line}"));
        rest.split_whitespace().next().unwrap().parse().unwrap()
    }

    #[test]
    fn the_echo_of_a_real_write_is_found_among_the_reads_that_followed_it() {
        let (pty, term) = reading_terminal();
        let seq = pty.write("t", b"x").unwrap();
        let start = Instant::now();
        while term.writes.get(seq).unwrap().written_us.is_none() {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "the write never was"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        // What the reader does for each read: the cursor hidden, then the letter 30 ms later and
        // the rest of the line.
        term.writes.output(now_us(), 6);
        std::thread::sleep(Duration::from_millis(30));
        term.writes.output(now_us(), 1);
        term.writes.output(now_us(), 20);
        let (_, line) = term.report_line(slow_json(seq, 7), 10_000).unwrap();
        assert!(ms_after(&line, "the echo ") >= 30, "{line}");
        assert!(ms_after(&line, "first output ") <= ms_after(&line, "the echo "));
        // The rest of the line is a later read: another count, another date.
        let (_, rest) = term.report_line(slow_json(seq, 27), 20_000).unwrap();
        assert!(
            ms_after(&rest, "the echo ") >= ms_after(&line, "the echo "),
            "{rest}"
        );
        // What the window cannot have counted yet is nobody's echo.
        let (_, none) = term.report_line(slow_json(seq, 28), 30_000).unwrap();
        assert!(none.contains("the echo not identified"), "{none}");
    }

    #[test]
    fn a_terminals_reads_are_counted_in_its_own_log() {
        // Two terminals each count their own output from their opening.
        let pty = PtyManager::default();
        pty.insert_fake("a", Box::new(Recorder(Seen::default())));
        pty.insert_fake("b", Box::new(Recorder(Seen::default())));
        let (a, b) = (pty.term("a").unwrap(), pty.term("b").unwrap());
        a.writes.output(100, 10);
        b.writes.output(200, 3);
        assert_eq!(a.writes.read_ending_at(10).map(|r| r.at_us), Some(100));
        assert_eq!(a.writes.read_ending_at(3), None);
        assert_eq!(b.writes.read_ending_at(3).map(|r| r.at_us), Some(200));
        assert_eq!(b.writes.read_ending_at(10), None);
    }

    #[test]
    fn a_slow_echo_of_a_write_the_log_forgot_says_the_backend_times_are_unknown() {
        let (pty, term) = reading_terminal();
        for _ in 0..RECENT_WRITES + 3 {
            pty.write("t", b"x").unwrap();
        }
        let (_, line) = term.report_line(slow(0), 10_000).unwrap();
        assert_eq!(
            line,
            "terminal t (fake): echo of a 1-byte write shown after 1520 ms: backend times \
             unknown, in the window 1500 ms after the write left it, painted 20 ms later"
        );
        let (_, line) = term.report_line(slow(9_999), 20_000).unwrap();
        assert!(line.contains("backend times unknown"), "{line}");
    }

    #[test]
    fn a_slow_echo_of_a_write_the_program_has_not_read_yet_says_so() {
        let pty = PtyManager::default();
        let (writer, open, _) = gated();
        pty.insert_fake("t", Box::new(writer));
        let term = pty.term("t").unwrap();
        let seq = pty.write("t", b"x").unwrap();
        let (_, line) = term.report_line(slow(seq), 10_000).unwrap();
        assert!(line.contains("not written yet"), "{line}");
        drop(open);
    }

    #[test]
    fn a_terminal_logs_one_slow_echo_in_five_seconds() {
        let (_pty, term) = reading_terminal();
        assert!(term.report_line(slow(0), 10_000).is_some());
        assert!(term.report_line(slow(0), 12_000).is_none());
        // What was dropped does not delay the next one; the renderer is not limited.
        assert!(term.report_line(renderer("webgl", None), 12_000).is_some());
        assert!(term.report_line(slow(0), 14_999).is_none());
        assert!(term.report_line(slow(0), 15_000).is_some());
    }

    #[test]
    fn reports_that_come_together_log_one_slow_echo_between_them() {
        // Two `term_report` calls run at once on the runtime's threads.
        let (_pty, term) = reading_terminal();
        for round in 1..=200 {
            let barrier = std::sync::Barrier::new(16);
            let logged = std::thread::scope(|s| {
                let reports: Vec<_> = (0..16)
                    .map(|_| {
                        s.spawn(|| {
                            barrier.wait();
                            term.report_line(slow(0), round * 10_000).is_some()
                        })
                    })
                    .collect();
                reports
                    .into_iter()
                    .map(|report| report.join().unwrap())
                    .filter(|logged| *logged)
                    .count()
            });
            assert_eq!(logged, 1, "round {round}");
        }
    }

    #[test]
    fn a_report_for_a_terminal_that_is_gone_is_ignored() {
        let pty = PtyManager::default();
        pty.report("nope", renderer("webgl", None));
        pty.report("nope", slow(0));
    }

    /// The result of `write_and_wait` on terminal `t`, once it returns, in a channel (the call
    /// runs in a thread of its own).
    fn waiting_write(
        pty: &PtyManager,
        data: &'static [u8],
        timeout: Duration,
    ) -> mpsc::Receiver<Result<u64>> {
        let (done, finished) = mpsc::channel();
        let pty = pty.clone();
        std::thread::spawn(move || {
            let _ = done.send(pty.write_and_wait("t", data, timeout));
        });
        finished
    }

    #[test]
    fn write_and_wait_returns_once_the_data_is_written_and_not_before() {
        let pty = PtyManager::default();
        let (writer, open, seen) = gated();
        pty.insert_fake("t", Box::new(writer));
        let finished = waiting_write(&pty, b"paste", Duration::from_secs(60));
        // The input thread is held by the closed gate.
        assert!(
            finished.recv_timeout(Duration::from_millis(300)).is_err(),
            "it returned before the program read anything"
        );
        open.send(()).unwrap();
        let seq = finished
            .recv_timeout(Duration::from_secs(5))
            .expect("it never returned")
            .unwrap();
        assert_eq!(seq, 0);
        assert_eq!(*seen.lock(), b"paste");
    }

    #[test]
    fn write_and_wait_waits_for_a_write_the_queue_merged_with_others() {
        let pty = PtyManager::default();
        let (writer, open, seen) = gated();
        pty.insert_fake("t", Box::new(writer));
        pty.write("t", b"first").unwrap();
        pty.write("t", b"second").unwrap();
        let finished = waiting_write(&pty, b"third", Duration::from_secs(60));
        assert!(finished.recv_timeout(Duration::from_millis(300)).is_err());
        open.send(()).unwrap();
        finished
            .recv_timeout(Duration::from_secs(5))
            .expect("it never returned")
            .unwrap();
        assert_eq!(*seen.lock(), b"firstsecondthird");
    }

    #[test]
    fn write_and_wait_gives_up_after_its_timeout_and_the_data_stays_queued() {
        let pty = PtyManager::default();
        let (writer, open, seen) = gated();
        pty.insert_fake("t", Box::new(writer));
        let finished = waiting_write(&pty, b"x", Duration::from_millis(200));
        let seq = finished
            .recv_timeout(Duration::from_secs(5))
            .expect("a timeout that never ends")
            .expect("best effort: a timeout is not an error");
        assert_eq!(seq, 0);
        assert!(seen.lock().is_empty(), "the gate is still closed");
        // It goes through once the program reads.
        open.send(()).unwrap();
        wait_for(&seen, 1);
    }

    #[test]
    fn write_and_wait_says_the_terminal_is_closed_when_the_program_ended() {
        let pty = PtyManager::default();
        pty.insert_fake("t", Box::new(Broken));
        let err = pty
            .write_and_wait("t", b"x", Duration::from_secs(5))
            .unwrap_err();
        assert_eq!(err.to_string(), closed(i18n::ui()).to_string());
        assert!(pty
            .write_and_wait("nope", b"x", Duration::from_secs(5))
            .is_err());
    }

    fn item(seq: u64, data: &[u8]) -> Input {
        Input {
            data: data.to_vec(),
            seq,
            called_us: 0,
            queued_us: 0,
            ack: None,
        }
    }

    /// The numbers of the writes of each call that `writes_to_make` makes of these.
    fn plan(items: &[&[u8]]) -> Vec<Vec<u64>> {
        let items = items
            .iter()
            .enumerate()
            .map(|(seq, data)| item(seq as u64, data))
            .collect();
        writes_to_make(items)
            .into_iter()
            .map(|call| call.into_iter().map(|w| w.seq).collect())
            .collect()
    }

    #[test]
    fn what_is_queued_is_written_in_one_call_unless_it_is_a_lone_escape() {
        assert_eq!(plan(&[b"a", b"b", b"c"]), [[0, 1, 2]]);
        assert_eq!(plan(&[b"a", b"\x1b", b"b"]), [vec![0], vec![1], vec![2]]);
        assert_eq!(plan(&[b"\x1b", b"a", b"b"]), [vec![0], vec![1, 2]]);
        assert_eq!(plan(&[b"a", b"b", b"\x1b"]), [vec![0, 1], vec![2]]);
        assert_eq!(plan(&[b"\x1b", b"\x1b"]), [vec![0], vec![1]]);
        assert_eq!(
            plan(&[b"a", b"\x1b", b"b", b"c", b"\x1b", b"d"]),
            [vec![0], vec![1], vec![2, 3], vec![4], vec![5]]
        );
        // Anything else that starts with Escape is a key sequence: whole, and merged.
        assert_eq!(
            plan(&[b"a", b"\x1b[A", b"\x1bx", b"\x1b\x1b"]),
            [[0, 1, 2, 3]]
        );
        assert!(plan(&[]).is_empty());
    }

    #[test]
    fn what_is_queued_is_written_256_kb_at_most_at_a_time_but_a_bigger_write_stays_whole() {
        let kb = |n: usize| vec![b'a'; n * 1024];
        let (a, b, c, big, small) = (kb(100), kb(100), kb(100), kb(300), vec![b'z']);
        assert_eq!(
            plan(&[&a, &b, &c, &big, &small]),
            [vec![0, 1], vec![2], vec![3], vec![4]]
        );
    }

    #[test]
    fn a_lone_escape_is_written_apart_from_what_was_queued_around_it() {
        let pty = PtyManager::default();
        let (writer, open, seen, calls) = gated_recording();
        pty.insert_fake("t", Box::new(writer));
        // Queued while the program reads nothing: without the rule, one write for the three.
        for data in [&b"a"[..], b"\x1b", b"b"] {
            pty.write("t", data).unwrap();
        }
        open.send(()).unwrap();
        wait_for(&seen, 3);
        assert_eq!(
            *calls.lock(),
            [b"a".to_vec(), b"\x1b".to_vec(), b"b".to_vec()]
        );
    }

    #[test]
    fn the_writes_of_a_queue_cut_around_an_escape_are_each_logged_when_their_call_is_done() {
        let pty = PtyManager::default();
        let (writer, open, seen, _) = gated_recording();
        pty.insert_fake("t", Box::new(writer));
        let term = pty.term("t").unwrap();
        let seqs: Vec<u64> = [&b"a"[..], b"\x1b", b"b"]
            .into_iter()
            .map(|d| pty.write("t", d).unwrap())
            .collect();
        open.send(()).unwrap();
        wait_for(&seen, 3);
        let start = Instant::now();
        while term.writes.all().iter().any(|r| r.written_us.is_none()) {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "never all written"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let all = term.writes.all();
        assert_eq!(all.iter().map(|r| r.seq).collect::<Vec<_>>(), seqs);
        assert!(all.iter().all(|r| r.started_us <= r.written_us));
        // Each call begins once the one before is done.
        assert!(all[0].written_us <= all[1].started_us);
        assert!(all[1].written_us <= all[2].started_us);
    }

    #[test]
    fn killing_a_terminal_closes_its_console_there_even_while_someone_holds_the_term() {
        let (pty, term) = reading_terminal();
        term.resize(100, 30).unwrap();
        assert!(term.master.lock().is_some());
        // A holder of the term (the main thread in a write, a reader answering a query) must not
        // be the one to close the pseudo-console: the kill does.
        pty.kill("t");
        assert!(term.master.lock().is_none());
        let err = term.resize(100, 30).unwrap_err();
        assert_eq!(err.to_string(), closed(i18n::ui()).to_string());
        // Closing again is harmless.
        term.close_console();
    }
}

#[cfg(test)]
mod typing_tests {
    use super::typing;

    #[test]
    fn what_the_terminal_sends_on_its_own_and_escape_alone_are_not_typing() {
        for data in [
            &b"\x1b"[..],
            b"\x1b\x1b",
            b"\x1b[I",
            b"\x1b[O",
            b"\x1b[12;40R",
            b"\x1b[?1;2c",
            b"\x1b[>0;276;0c",
            b"\x1b[0n",
            b"\x1b[?2004;1$y",
            b"\x1b[<0;10;5M\x1b[<0;10;5m",
            b"\x1b[M #!",
            b"\x1b]11;rgb:0000/0000/0000\x07",
            b"\x1b]10;rgb:ffff/ffff/ffff\x1b\\",
            b"\x1bP1$r0m\x1b\\",
            b"\x1b[I\x1b[12;1R\x1b",
            b"",
        ] {
            assert!(!typing(data), "{data:?}");
        }
    }

    #[test]
    fn text_keys_and_sequences_cut_short_are_typing() {
        for data in [
            &b"a"[..],
            b"\r",
            b"\x03",
            b"\x7f",
            b"\x15",
            b"\x1b[A",
            b"\x1b[3~",
            b"\x1bOP",
            b"\x1bb",
            b"\x1b\x7f",
            b"\x1b[200~texte\x1b[201~",
            b"\x1b[I a",
            b"\x1b[12;4",
            b"\x1b]11;rgb",
        ] {
            assert!(typing(data), "{data:?}");
        }
    }
}
