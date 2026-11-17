//! Tests of the numbered output of the terminals (`termlog`, ESC-495): the ring and its numbers, the resume of a
//! plain terminal's output and of an agent's interface (`tui::TuiOutput`), with a client that applies what it is sent
//! as the window's transport does (`src/lib/transport.ts`): nothing lost, nothing twice.

use crate::termlog::{Replay, Ring, Since, Sink, TermLog};
use crate::tui::TuiOutput;
use parking_lot::Mutex;
use std::sync::Arc;

/// What a sink is told, in order.
#[derive(Debug, Clone, PartialEq)]
enum Told {
    Send(u64, Vec<u8>),
    Replay(Vec<u8>),
    Mark(u64),
    Lost(u64),
    Resumed(bool),
}

#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<Told>>>);

impl Recorder {
    fn take(&self) -> Vec<Told> {
        std::mem::take(&mut self.0.lock())
    }
}

impl Sink for Recorder {
    fn send(&self, end: u64, data: &[u8]) {
        self.0.lock().push(Told::Send(end, data.to_vec()));
    }
    fn replay(&self, data: &[u8]) {
        self.0.lock().push(Told::Replay(data.to_vec()));
    }
    fn mark(&self, end: u64) {
        self.0.lock().push(Told::Mark(end));
    }
    fn lost(&self, bytes: u64) {
        self.0.lock().push(Told::Lost(bytes));
    }
    fn resumed(&self, ended: bool) {
        self.0.lock().push(Told::Resumed(ended));
    }
}

/// A client as the window's transport is (`src/lib/transport.ts`, `TermStream`): the end of the last numbered byte
/// it received, and what it shows.
#[derive(Default)]
struct Shown {
    last: Option<u64>,
    shown: Vec<u8>,
    lost: u64,
    gaps: u32,
}

impl Shown {
    fn apply(&mut self, told: &[Told]) {
        for t in told {
            match t {
                Told::Send(end, data) => {
                    let start = end - data.len() as u64;
                    let last = *self.last.get_or_insert(start);
                    if *end <= last {
                        continue; // twice
                    }
                    if start > last {
                        self.gaps += 1; // a hole: the client resumes, nothing shown
                        continue;
                    }
                    self.shown
                        .extend_from_slice(&data[(last - start) as usize..]);
                    self.last = Some(*end);
                }
                Told::Replay(data) => self.shown.extend_from_slice(data),
                Told::Mark(end) => self.last = Some(*end),
                Told::Lost(n) => {
                    // What follows is all that is kept: the numbering goes on past what was lost.
                    self.lost += n;
                    self.last = self.last.map(|l| l + n);
                }
                Told::Resumed(_) => {}
            }
        }
    }
}

#[test]
fn a_ring_numbers_its_bytes_and_keeps_the_last_ones() {
    let mut r = Ring::new(8);
    assert!(r.is_empty());
    assert_eq!(r.push(b"abcde"), 5);
    assert_eq!(r.push(b"fghij"), 10);
    // Ten printed, the last eight kept: from byte 2.
    assert_eq!(
        (r.start(), r.end(), r.kept()),
        (2, 10, b"cdefghij".to_vec())
    );
    // What follows a byte still kept.
    assert_eq!(
        r.since(7),
        Since {
            lost: 0,
            from: 7,
            data: b"hij".to_vec()
        }
    );
    assert_eq!(r.since(10).data, b"");
    // A client further than the end (none can be): nothing to send, nothing lost.
    assert_eq!(
        r.since(99),
        Since {
            lost: 0,
            from: 10,
            data: Vec::new()
        }
    );
    // One that is too far behind: all that is kept, and how much it lost.
    assert_eq!(
        r.since(0),
        Since {
            lost: 2,
            from: 2,
            data: b"cdefghij".to_vec()
        }
    );
}

#[test]
fn a_terminal_resumed_from_where_its_client_stopped_shows_every_byte_once() {
    let first = Recorder::default();
    let log = TermLog::with_size(1 << 10, Box::new(first.clone()));
    let mut client = Shown::default();
    log.push(b"one ");
    log.push(b"two ");
    client.apply(&first.take());
    // The cut: what is printed meanwhile goes to a client that is gone.
    log.push(b"three ");
    log.push(b"four ");
    first.take();
    // The client comes back, says where it stopped, and gets the rest, then what comes.
    let second = Recorder::default();
    log.resume(client.last, Box::new(second.clone()));
    log.push(b"five");
    let told = second.take();
    assert_eq!(told.last(), Some(&Told::Send(23, b"five".to_vec())));
    assert!(told.contains(&Told::Resumed(false)));
    client.apply(&told);
    assert_eq!(client.shown, b"one two three four five");
    assert_eq!((client.lost, client.gaps), (0, 0));
    assert_eq!(log.printed(), 23);
}

#[test]
fn a_resume_that_overlaps_what_came_already_shows_no_byte_twice() {
    let sink = Recorder::default();
    let log = TermLog::with_size(1 << 10, Box::new(sink.clone()));
    let mut client = Shown::default();
    log.push(b"abc");
    log.push(b"def");
    let live = sink.take();
    client.apply(&live);
    // The client asks from an older place (it resumed while the live bytes were on their way): the bytes it has
    // already are left out.
    let again = Recorder::default();
    log.resume(Some(3), Box::new(again.clone()));
    client.apply(&again.take());
    client.apply(&live);
    log.push(b"ghi");
    client.apply(&again.take());
    assert_eq!(client.shown, b"abcdefghi");
}

#[test]
fn a_client_too_far_behind_gets_the_ring_whole_and_the_count_of_what_it_lost() {
    let sink = Recorder::default();
    let log = TermLog::with_size(10, Box::new(sink.clone()));
    let mut client = Shown::default();
    log.push(b"0123");
    client.apply(&sink.take());
    for _ in 0..4 {
        log.push(b"abcde");
    }
    sink.take();
    let back = Recorder::default();
    log.resume(client.last, Box::new(back.clone()));
    let told = back.take();
    // 24 printed, 10 kept (14 to 24): 10 lost after the 4 it had.
    assert_eq!(told[0], Told::Lost(10));
    client.apply(&told);
    assert_eq!(client.lost, 10);
    assert_eq!(client.shown, b"0123abcdeabcde");
}

#[test]
fn a_resume_after_the_program_ended_says_it() {
    let log = TermLog::with_size(64, Box::new(Recorder::default()));
    log.push(b"done");
    log.end();
    assert!(log.over());
    let back = Recorder::default();
    log.resume(Some(0), Box::new(back.clone()));
    assert_eq!(
        back.take(),
        vec![Told::Send(4, b"done".to_vec()), Told::Resumed(true)]
    );
}

#[test]
fn a_long_replay_goes_in_numbered_pieces() {
    let log = TermLog::with_size(1 << 20, Box::new(Recorder::default()));
    let big: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    log.push(&big);
    let back = Recorder::default();
    log.resume(Some(0), Box::new(back.clone()));
    let told = back.take();
    let sends: Vec<_> = told
        .iter()
        .filter_map(|t| match t {
            Told::Send(end, data) => Some((*end, data.len())),
            _ => None,
        })
        .collect();
    assert!(sends.len() >= 4, "{}", sends.len());
    assert!(sends
        .iter()
        .all(|(_, n)| *n <= crate::termlog::REPLAY_PIECE));
    let mut client = Shown {
        last: Some(0),
        ..Default::default()
    };
    client.apply(&told);
    assert_eq!(client.shown, big);
}

#[test]
fn the_reader_of_a_terminal_keeps_its_output_and_ends_it_when_let_go() {
    let sink = Recorder::default();
    let feed = crate::termlog::feed(Box::new(sink.clone()));
    feed(b"abc".to_vec());
    assert_eq!(sink.take(), vec![Told::Send(3, b"abc".to_vec())]);
    drop(feed);
}

// ---------- an agent's interface: the same ring, in `TuiOutput` ----------

#[test]
fn an_interface_resumed_from_where_its_client_stopped_gets_the_rest_numbered() {
    let out = Arc::new(TuiOutput::new(None));
    let first = Recorder::default();
    out.attach(Box::new(first.clone()));
    let mut client = Shown::default();
    out.push(b"\x1b[?1049hscreen ".to_vec());
    client.apply(&first.take());
    out.push(b"while cut ".to_vec());
    first.take();
    let second = Recorder::default();
    out.resume(client.last, Box::new(second.clone()));
    out.push(b"after".to_vec());
    let told = second.take();
    // No replay of the screen: the bytes it missed, numbered, as they were printed.
    assert!(
        !told.iter().any(|t| matches!(t, Told::Replay(_))),
        "{told:?}"
    );
    client.apply(&told);
    assert_eq!(client.shown, b"\x1b[?1049hscreen while cut after");
}

#[test]
fn an_interface_whose_client_is_too_far_behind_draws_its_screen_again() {
    let out = Arc::new(TuiOutput::new(None));
    out.push(b"\x1b[?1049h".to_vec());
    // More than it keeps (4 MB), in lines.
    let mut line = vec![b'x'; 1023];
    line.push(b'\n');
    for _ in 0..(5 << 10) {
        out.push(line.clone());
    }
    let back = Recorder::default();
    out.resume(Some(0), Box::new(back.clone()));
    let told = back.take();
    assert!(
        matches!(told[0], Told::Lost(n) if n > 0),
        "{:?}",
        &told[..1]
    );
    // The modes it set first, then what is kept, then the numbering from the end.
    assert_eq!(told[1], Told::Replay(b"\x1b[?1049h".to_vec()));
    let end = (5u64 << 10) * 1024 + 8;
    assert!(told.contains(&Told::Mark(end)), "mark");
    assert_eq!(told.last(), Some(&Told::Resumed(false)));
}

#[test]
fn an_interface_opened_by_a_window_is_numbered_from_the_end_of_its_replay() {
    let out = Arc::new(TuiOutput::new(None));
    out.push(b"before".to_vec());
    let window = Recorder::default();
    out.attach(Box::new(window.clone()));
    out.push(b" after".to_vec());
    let mut client = Shown::default();
    client.apply(&window.take());
    assert_eq!(client.shown, b"before after");
    assert_eq!(client.last, Some(12));
}
