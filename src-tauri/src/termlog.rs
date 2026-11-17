//! The output of the terminals kept by the core, numbered (ESC-495, `docs/REMOTE.md`, « La fondation 2b »).
//!
//! - **One ring per terminal and per launch** ([`Ring`]): its last bytes, numbered by their place in the whole output
//!   (the byte `n` is the `n`-th the program printed). A plain terminal, a launch command, a step of a test launch and a
//!   sign-in keep theirs in a [`TermLog`] ([`TERM_KEPT`]); an agent's interface keeps its own in `tui::TuiOutput`
//!   ([`crate::tui`], 4 MB), which is the same ring.
//! - **Where it goes** ([`Sink`]): the window takes the bytes alone (Tauri's IPC, as before); a client of the remote API
//!   takes them numbered, so that a client that lost its connection says the end of the last bytes it received and
//!   gets the rest ([`Replay::resume`]), or the ring whole with the count of what it lost when it is too far behind.
//! - Nothing here is written in the log: a terminal's output is the user's.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

/// The most a plain terminal, a launch, a step of a test launch or a sign-in keep of their output (bytes).
pub const TERM_KEPT: usize = 1 << 20;
/// The size of the pieces a replay is sent in.
pub const REPLAY_PIECE: usize = 64 << 10;

/// The last bytes of an output, numbered: `start` is the number of the first one kept, `end()` the number after the
/// last one (how many the program printed).
#[derive(Debug)]
pub struct Ring {
    bytes: VecDeque<u8>,
    start: u64,
    cap: usize,
}

/// What a client that received the output up to `after` is to be sent: the bytes from `from` (`data`), and how many
/// were lost between (`lost`: they left the ring).
#[derive(Debug, PartialEq, Eq)]
pub struct Since {
    pub lost: u64,
    pub from: u64,
    pub data: Vec<u8>,
}

impl Ring {
    pub fn new(cap: usize) -> Self {
        Self {
            bytes: VecDeque::new(),
            start: 0,
            cap,
        }
    }

    /// The number of the first byte kept.
    pub fn start(&self) -> u64 {
        self.start
    }

    /// The number after the last byte: how many were printed.
    pub fn end(&self) -> u64 {
        self.start + self.bytes.len() as u64
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// `data` printed: kept, the oldest bytes let go past the ring's size. The end of `data`.
    pub fn push(&mut self, data: &[u8]) -> u64 {
        self.bytes.extend(data.iter());
        let excess = self.bytes.len().saturating_sub(self.cap);
        self.drop_front(excess);
        self.end()
    }

    /// The first `n` bytes kept let go.
    pub fn drop_front(&mut self, n: usize) {
        let n = n.min(self.bytes.len());
        self.bytes.drain(..n);
        self.start += n as u64;
    }

    /// The first byte kept that is `b`, among the first `within`: its place from the start of what is kept.
    pub fn position(&self, b: u8, within: usize) -> Option<usize> {
        self.bytes.iter().take(within).position(|x| *x == b)
    }

    /// All that is kept.
    pub fn kept(&self) -> Vec<u8> {
        self.bytes.iter().copied().collect()
    }

    /// What follows `after` (the end of what a client received): the rest, or all that is kept and the count of what
    /// was lost. An `after` past the end (no client received more than was printed) is the end: nothing to send.
    pub fn since(&self, after: u64) -> Since {
        let after = after.min(self.end());
        if after < self.start {
            return Since {
                lost: self.start - after,
                from: self.start,
                data: self.kept(),
            };
        }
        let skip = (after - self.start) as usize;
        Since {
            lost: 0,
            from: after,
            data: self.bytes.iter().skip(skip).copied().collect(),
        }
    }
}

/// Where a terminal's output goes: the window, or a client of the remote API.
pub trait Sink: Send + Sync {
    /// Bytes of the output, the last of them numbered `end - 1`.
    fn send(&self, end: u64, data: &[u8]);
    /// Bytes outside the numbering, which a window shows as they come: a replay of the screen (its modes set again,
    /// what is kept less the terminal's queries), followed by [`Sink::mark`].
    fn replay(&self, data: &[u8]);
    /// The numbering goes on at `end` after a replay.
    fn mark(&self, _end: u64) {}
    /// `bytes` of the output left the ring before the client could have them: what follows is all that is kept.
    fn lost(&self, _bytes: u64) {}
    /// A resume's replay is over: what comes is the output as it is printed.
    fn resumed(&self, _ended: bool) {}
    /// The output this sink is given from now on, for a client that will resume it (the remote API's sessions).
    fn bound(&self, _replay: Arc<dyn Replay>) {}
}

/// The window's sink, or a test's: the bytes alone, numbered or not (Tauri's IPC never loses any).
impl<F: Fn(Vec<u8>) + Send + Sync> Sink for F {
    fn send(&self, _end: u64, data: &[u8]) {
        self(data.to_vec())
    }
    fn replay(&self, data: &[u8]) {
        self(data.to_vec())
    }
}

impl Sink for Box<dyn Sink> {
    fn send(&self, end: u64, data: &[u8]) {
        (**self).send(end, data)
    }
    fn replay(&self, data: &[u8]) {
        (**self).replay(data)
    }
    fn mark(&self, end: u64) {
        (**self).mark(end)
    }
    fn lost(&self, bytes: u64) {
        (**self).lost(bytes)
    }
    fn resumed(&self, ended: bool) {
        (**self).resumed(ended)
    }
    fn bound(&self, replay: Arc<dyn Replay>) {
        (**self).bound(replay)
    }
}

/// What a terminal's reader is given (`PtyManager::spawn…`): the output kept in a [`TermLog`] of its own and sent to
/// `sink`; the log is over when the reader lets it go (the program's output closed).
pub fn feed(sink: Box<dyn Sink>) -> impl Fn(Vec<u8>) + Send + Sync + 'static {
    struct Over(Arc<TermLog>);
    impl Drop for Over {
        fn drop(&mut self) {
            self.0.end();
        }
    }
    let over = Over(TermLog::with_size(TERM_KEPT, sink));
    move |bytes| over.0.push(&bytes)
}

/// An output a client may take over again, from where it stopped.
pub trait Replay: Send + Sync {
    /// The client of `sink` received the output up to `after` (`None`: nothing yet): it is sent the rest, then what
    /// comes, in place of the sink before.
    fn resume(&self, after: Option<u64>, sink: Box<dyn Sink>);
    /// The program ended: nothing more will come.
    fn over(&self) -> bool;
}

/// Sends `since` to `sink`, numbered, in pieces.
pub fn send_since(sink: &dyn Sink, since: &Since) {
    if since.lost > 0 {
        sink.lost(since.lost);
    }
    let mut end = since.from;
    for piece in since.data.chunks(REPLAY_PIECE) {
        end += piece.len() as u64;
        sink.send(end, piece);
    }
}

/// The output of a plain terminal, a launch, a step of a test launch or a sign-in: its last [`TERM_KEPT`] bytes, and
/// the client that shows it.
pub struct TermLog {
    inner: Mutex<LogInner>,
}

struct LogInner {
    ring: Ring,
    sink: Option<Box<dyn Sink>>,
    over: bool,
}

impl TermLog {
    /// A log of `cap` bytes whose output goes to `sink` from its first byte.
    pub fn with_size(cap: usize, sink: Box<dyn Sink>) -> Arc<Self> {
        let log = Arc::new(Self {
            inner: Mutex::new(LogInner {
                ring: Ring::new(cap),
                sink: None,
                over: false,
            }),
        });
        sink.bound(log.clone());
        log.inner.lock().sink = Some(sink);
        log
    }

    /// What the program printed: kept, and sent to the client that shows it. Under one lock with `resume`, so that a
    /// client taking over finds every byte once, in order.
    pub fn push(&self, data: &[u8]) {
        let mut o = self.inner.lock();
        let end = o.ring.push(data);
        if let Some(sink) = &o.sink {
            sink.send(end, data);
        }
    }

    /// The program ended.
    pub fn end(&self) {
        self.inner.lock().over = true;
    }

    /// The number after the last byte printed.
    #[cfg(test)]
    pub fn printed(&self) -> u64 {
        self.inner.lock().ring.end()
    }
}

impl Replay for TermLog {
    fn resume(&self, after: Option<u64>, sink: Box<dyn Sink>) {
        let mut o = self.inner.lock();
        send_since(&*sink, &o.ring.since(after.unwrap_or(0)));
        sink.resumed(o.over);
        o.sink = Some(sink);
    }

    fn over(&self) -> bool {
        self.inner.lock().over
    }
}

impl std::fmt::Debug for TermLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let o = self.inner.lock();
        f.debug_struct("TermLog")
            .field("kept", &o.ring.len())
            .field("end", &o.ring.end())
            .finish()
    }
}
