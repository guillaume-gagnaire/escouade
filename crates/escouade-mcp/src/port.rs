//! The port the server listens on: bound on 127.0.0.1, waited for when it is held (`bind`), and how a stop waits for
//! it to close (`wait_closed`).

use crate::PORTS;
use anyhow::{anyhow, Result};
use std::net::{Ipv4Addr, TcpListener};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long a stop waits for the port to close: a start right after binds it again. Longer in
/// the tests: the accept loop runs on the app's runtime, which every test of the binary shares,
/// and on a 3-core CI runner it went unscheduled for over 2 s (the port, still open, was not
/// taken again).
pub const STOP_WAIT: Duration = Duration::from_secs(if cfg!(any(test, feature = "testing")) {
    30
} else {
    2
});

/// Waits until the accept loop says it closed the port (`STOP_WAIT` at most); false when it did
/// not say so. On a worker of a multi-threaded runtime (an async command, a tool), the wait first
/// hands the worker's other tasks to another one, the accept loop among them maybe. Elsewhere (a
/// thread of its own, a single-threaded runtime) it blocks: the loop runs on the app's runtime.
pub fn wait_closed(closed: &mpsc::Receiver<()>) -> bool {
    use tokio::runtime::{Handle, RuntimeFlavor};
    let wait = || closed.recv_timeout(STOP_WAIT).is_ok();
    match Handle::try_current() {
        Ok(h) if h.runtime_flavor() == RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(wait)
        }
        _ => wait(),
    }
}

/// How long the port saved for the server is waited for, when something holds it: a server that
/// stops and starts again at once (a setting turned off and on) finds its old listener not
/// quite gone, and must not move to another port (it would be declared again in every account of
/// Claude Code).
pub const BIND_RETRY: Duration = Duration::from_secs(3);
const BIND_STEP: Duration = Duration::from_millis(25);

/// A listener on 127.0.0.1:`port`, waited for up to `BIND_RETRY` when it is held (no longer once
/// `quitting` says the app stops); when that one stays taken (or is 0), on a free port of `PORTS`
/// (tried from a random one, so that two instances, an end-to-end test's beside the user's,
/// rarely try the same ones).
pub fn bind(port: u16, quitting: &dyn Fn() -> bool) -> Result<(TcpListener, u16)> {
    bind_within(port, BIND_RETRY, quitting)
}

/// A port that is busy for now, which a moment makes free again: held by a listener on its way
/// out, or, on Windows, answered « access denied » right after it was released. Any other error
/// (an address the machine does not have...) does not pass with time.
pub fn worth_waiting_for(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    e.kind() == ErrorKind::AddrInUse || (cfg!(windows) && e.kind() == ErrorKind::PermissionDenied)
}

/// `bind`, with at most `wait` for the port asked for (tried again every `BIND_STEP`), and none
/// once `quitting` is true: the app stops, there is no server to start.
pub fn bind_within(
    port: u16,
    wait: Duration,
    quitting: &dyn Fn() -> bool,
) -> Result<(TcpListener, u16)> {
    let mut refused = None;
    if port != 0 {
        let until = Instant::now() + wait;
        loop {
            match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
                Ok(l) => return Ok((l, port)),
                Err(e) if worth_waiting_for(&e) && Instant::now() < until => {
                    if quitting() {
                        return Err(anyhow!("the app is stopping"));
                    }
                    std::thread::sleep(BIND_STEP);
                }
                Err(e) => {
                    refused = Some(e);
                    break;
                }
            }
        }
    }
    let (first, count) = (*PORTS.start(), PORTS.len() as u32);
    let from = (uuid::Uuid::new_v4().as_u128() % u128::from(count)) as u32;
    for i in 0..count {
        let p = first + ((from + i) % count) as u16;
        if p == port {
            continue;
        }
        if let Ok(l) = TcpListener::bind((Ipv4Addr::LOCALHOST, p)) {
            if let Some(e) = refused {
                log::warn!("mcp: port {port} not available ({e}), {p} is used instead");
            }
            return Ok((l, p));
        }
    }
    Err(anyhow!(tr!(
        "Aucun port libre entre {first} et {last} pour le serveur MCP",
        "No free port between {first} and {last} for the MCP server",
        last = PORTS.end()
    )))
}

/// A port of the machine that something holds, and the listener that holds it.
#[cfg(any(test, feature = "testing"))]
pub fn held_port() -> (TcpListener, u16) {
    let holder = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = holder.local_addr().unwrap().port();
    (holder, port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_port_that_is_released_a_moment_later_is_taken_again_instead_of_another_one() {
        // Stopped and started again at once: the old listener is not gone yet.
        let (holder, port) = held_port();
        let releasing = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(holder);
        });
        let (_listener, got) = bind_within(port, Duration::from_secs(3), &|| false).unwrap();
        releasing.join().unwrap();
        assert_eq!(got, port);
    }

    #[test]
    fn a_saved_port_that_stays_taken_gives_another_one_of_the_range_after_the_wait_only() {
        let (_holder, port) = held_port();
        let wait = Duration::from_millis(200);
        let started = std::time::Instant::now();
        let (_listener, got) = bind_within(port, wait, &|| false).unwrap();
        let took = started.elapsed();
        assert_ne!(got, port);
        assert!(PORTS.contains(&got), "{got}");
        assert!(took >= wait, "another port drawn after {took:?}");
        assert!(took < Duration::from_secs(1), "waited {took:?}");
    }

    #[test]
    fn only_a_port_that_is_busy_for_now_is_waited_for() {
        use std::io::{Error, ErrorKind};
        assert!(worth_waiting_for(&Error::from(ErrorKind::AddrInUse)));
        // What Windows answers for a port released a moment ago.
        assert_eq!(
            worth_waiting_for(&Error::from(ErrorKind::PermissionDenied)),
            cfg!(windows)
        );
        for kind in [ErrorKind::AddrNotAvailable, ErrorKind::InvalidInput] {
            assert!(!worth_waiting_for(&Error::from(kind)), "{kind:?}");
        }
    }

    #[test]
    fn no_saved_port_draws_one_of_the_range_without_waiting() {
        let started = std::time::Instant::now();
        let (_listener, got) = bind_within(0, Duration::from_secs(3), &|| false).unwrap();
        assert!(PORTS.contains(&got), "{got}");
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
