//! The send queue of a connection of the remote API (ESC-495): bounded, so that a client that reads slowly, or not at
//! all, never makes the core's memory grow.
//!
//! - **Three lanes** ([`Lane`]): what is never dropped (the answers, the handshake, a resume's replay, the marks:
//!   `Control`), the events of the state (`State`), and a terminal's output (`Term`).
//! - **Past the soft bounds** ([`Bounds::messages`], [`Bounds::bytes`]): the events of the state melt into one
//!   re-read (the events queued are dropped, a [`ToClient::Resync`] takes their place: the last state wins); a
//!   terminal's output is dropped, and the client is told ([`ToClient::Gap`]) once the queue has room again, to resume
//!   it from its numbered ring (`termlog`).
//! - **Past the hard bound** ([`Bounds::hard_bytes`], answers and replays included), or **nothing sent for
//!   [`Bounds::stall`]** while something waits: the connection is to close ([`Outbox::stuck`]).

use super::proto::{self, ToClient};
use parking_lot::Mutex;
use std::collections::{BTreeSet, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio_tungstenite::tungstenite::Message;

/// What a message is, for what the queue does of it past its bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// Never dropped.
    Control,
    /// An event of the state: melts into a re-read.
    State,
    /// The output of the terminal of this channel: dropped, then resumed.
    Term(u32),
}

/// The bounds of a queue.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    /// The most messages waiting before the state melts and the terminals' output is dropped.
    pub messages: usize,
    /// The most bytes waiting before the same.
    pub bytes: usize,
    /// The most bytes waiting at all (answers and replays included): past it, the connection closes.
    pub hard_bytes: usize,
    /// The longest time nothing is sent while something waits: past it, the connection closes.
    pub stall: Duration,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            messages: 2048,
            bytes: 8 << 20,
            hard_bytes: proto::MAX_BODY + (32 << 20),
            stall: Duration::from_secs(30),
        }
    }
}

struct Item {
    message: Message,
    lane: Lane,
    size: usize,
}

struct Queue {
    items: VecDeque<Item>,
    bytes: usize,
    /// A re-read of the state is queued, not sent yet: the events of the state wait for none.
    resync: bool,
    /// The channels whose output was dropped, to say once there is room.
    gaps: BTreeSet<u32>,
    /// Past the hard bound.
    overflow: bool,
    /// When something was last sent, or began to wait in an empty queue.
    progress: Instant,
    /// The most bytes that waited at once.
    peak: usize,
    /// What was dropped, in messages.
    dropped: u64,
    done: bool,
}

/// The send queue of a connection.
pub struct Outbox {
    q: Mutex<Queue>,
    ready: Notify,
    bounds: Bounds,
}

/// The size of a message, as the queue counts it.
fn size_of(message: &Message) -> usize {
    match message {
        Message::Text(t) => t.len(),
        Message::Binary(b) => b.len(),
        _ => 16,
    }
}

impl Outbox {
    pub fn new(bounds: Bounds) -> Self {
        Self {
            q: Mutex::new(Queue {
                items: VecDeque::new(),
                bytes: 0,
                resync: false,
                gaps: BTreeSet::new(),
                overflow: false,
                progress: Instant::now(),
                peak: 0,
                dropped: 0,
                done: false,
            }),
            ready: Notify::new(),
            bounds,
        }
    }

    /// `message` queued on `lane`, or dropped (past the bounds: see the module).
    pub fn push(&self, message: Message, lane: Lane) {
        let size = size_of(&message);
        let mut q = self.q.lock();
        if q.done || q.overflow {
            return;
        }
        let full = q.items.len() >= self.bounds.messages || q.bytes + size > self.bounds.bytes;
        match lane {
            Lane::Control => {}
            Lane::State if q.resync => {
                q.dropped += 1;
                return;
            }
            Lane::State if full => {
                // The events queued and this one melt into one re-read of the state.
                let before = q.items.len();
                q.items.retain(|i| i.lane != Lane::State);
                q.dropped += (before - q.items.len()) as u64 + 1;
                q.bytes = q.items.iter().map(|i| i.size).sum();
                q.resync = true;
                let text = serde_json::to_string(&ToClient::Resync {}).unwrap_or_default();
                let size = text.len();
                Self::queue(&mut q, Message::text(text), Lane::Control, size);
                drop(q);
                self.ready.notify_one();
                return;
            }
            Lane::State => {}
            Lane::Term(channel) => {
                if full || q.gaps.contains(&channel) {
                    q.gaps.insert(channel);
                    q.dropped += 1;
                    return;
                }
            }
        }
        if q.bytes + size > self.bounds.hard_bytes {
            q.overflow = true;
            drop(q);
            self.ready.notify_one();
            return;
        }
        Self::queue(&mut q, message, lane, size);
        drop(q);
        self.ready.notify_one();
    }

    fn queue(q: &mut Queue, message: Message, lane: Lane, size: usize) {
        if q.items.is_empty() {
            q.progress = Instant::now();
        }
        q.items.push_back(Item {
            message,
            lane,
            size,
        });
        q.bytes += size;
        q.peak = q.peak.max(q.bytes);
    }

    /// The next message to send, when one waits; None once the queue is over (closed, or past its hard bound).
    pub async fn next(&self) -> Option<Message> {
        loop {
            let notified = self.ready.notified();
            {
                let mut q = self.q.lock();
                if q.overflow {
                    return None;
                }
                if let Some(item) = q.items.pop_front() {
                    q.bytes -= item.size;
                    if matches!(&item.message, Message::Text(t) if t.as_str() == r#"{"t":"resync"}"#)
                    {
                        q.resync = false;
                    }
                    // Room again: the gaps are said, for the client to resume those terminals.
                    if !q.gaps.is_empty() && q.bytes <= self.bounds.bytes / 2 {
                        let gaps = std::mem::take(&mut q.gaps);
                        for channel in gaps {
                            let text = serde_json::to_string(&ToClient::Gap { channel })
                                .unwrap_or_default();
                            let size = text.len();
                            Self::queue(&mut q, Message::text(text), Lane::Control, size);
                        }
                    }
                    return Some(item.message);
                }
                if q.done {
                    return None;
                }
            }
            notified.await;
        }
    }

    /// A message was sent: the client reads.
    pub fn sent(&self) {
        self.q.lock().progress = Instant::now();
    }

    /// No more messages but the ones queued (and `last`, a close, after them).
    pub fn finish(&self, last: Option<Message>) {
        let mut q = self.q.lock();
        if let Some(last) = last {
            let size = size_of(&last);
            Self::queue(&mut q, last, Lane::Control, size);
        }
        q.done = true;
        drop(q);
        self.ready.notify_one();
    }

    /// Why the connection is to close: too much unread, or nothing sent for too long while something waits.
    pub fn stuck(&self, now: Instant) -> Option<&'static str> {
        let q = self.q.lock();
        if q.overflow {
            return Some("too much left unread");
        }
        if !q.items.is_empty() && now.duration_since(q.progress) >= self.bounds.stall {
            return Some("nothing read for too long");
        }
        None
    }

    /// The most bytes that waited at once.
    pub fn peak(&self) -> usize {
        self.q.lock().peak
    }

    /// The bytes waiting now.
    pub fn waiting(&self) -> usize {
        self.q.lock().bytes
    }

    /// What was dropped, in messages.
    pub fn dropped(&self) -> u64 {
        self.q.lock().dropped
    }
}
