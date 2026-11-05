//! A plugin's timers, whatever the target: tokio's in process; in a module, the host's (`timer/set`, `timer/fired`,
//! docs/PLUGINS.md §8.4.5), a task waiting for its timer as it waits for any line.

use std::fmt;
use std::future::Future;
use std::time::Duration;

/// A `timeout` that ran out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed;

impl fmt::Display for Elapsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("timed out")
    }
}

impl std::error::Error for Elapsed {}

/// Waits `delay`.
#[cfg(not(target_family = "wasm"))]
pub async fn sleep(delay: Duration) {
    tokio::time::sleep(delay).await;
}

/// `work`, or `Elapsed` past `limit`; given up then, as when it is dropped.
#[cfg(not(target_family = "wasm"))]
pub async fn timeout<F: Future>(limit: Duration, work: F) -> Result<F::Output, Elapsed> {
    tokio::time::timeout(limit, work).await.map_err(|_| Elapsed)
}

/// See the native `sleep`.
#[cfg(target_family = "wasm")]
pub async fn sleep(delay: Duration) {
    if let Some(cx) = crate::serve::reactor::cx() {
        sleep_on(&cx, delay).await;
    }
}

/// See the native `timeout`.
#[cfg(target_family = "wasm")]
pub async fn timeout<F: Future>(limit: Duration, work: F) -> Result<F::Output, Elapsed> {
    match crate::serve::reactor::cx() {
        Some(cx) => timeout_on(&cx, limit, work).await,
        None => Ok(work.await),
    }
}

#[cfg(any(test, target_family = "wasm"))]
pub(crate) use module::{sleep_on, timeout_on};

#[cfg(any(test, target_family = "wasm"))]
mod module {
    use super::*;
    use crate::serve::Cx;
    use serde_json::{json, Value};
    use std::future::poll_fn;
    use std::pin::pin;
    use std::task::Poll;

    /// A timer of the host being waited for; let go before it rang, it is cancelled.
    struct Ringing<'a> {
        cx: &'a Cx,
        timer: Value,
        rung: bool,
    }

    impl Drop for Ringing<'_> {
        fn drop(&mut self) {
            if !self.rung {
                self.cx
                    .notify("timer/cancel", json!({ "timer": self.timer }));
            }
        }
    }

    /// Waits `delay` on a timer of the host. A host that refuses one (past its 64 timers) is said in the journal and the
    /// sleep ends at once: a task that never ends would hold its call for ever.
    pub(crate) async fn sleep_on(cx: &Cx, delay: Duration) {
        let ms = delay.as_millis().min(u128::from(u64::MAX)) as u64;
        let timer = match cx.request("timer/set", json!({ "ms": ms })).await {
            Ok(v) => v.get("timer").cloned(),
            Err(e) => {
                cx.log("warn", &format!("no timer: {}", e.message));
                return;
            }
        };
        let Some(timer) = timer else { return };
        let key = timer.to_string();
        let mut ringing = Ringing {
            cx,
            timer,
            rung: false,
        };
        poll_fn(|_| {
            if cx.rang(&key) {
                ringing.rung = true;
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }

    /// `work`, or `Elapsed` past `limit`, on a timer of the host.
    pub(crate) async fn timeout_on<F: Future>(
        cx: &Cx,
        limit: Duration,
        work: F,
    ) -> Result<F::Output, Elapsed> {
        let mut work = pin!(work);
        let mut limit = pin!(sleep_on(cx, limit));
        poll_fn(|task| {
            if let Poll::Ready(done) = work.as_mut().poll(task) {
                return Poll::Ready(Ok(done));
            }
            limit.as_mut().poll(task).map(|()| Err(Elapsed))
        })
        .await
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_timeout_lets_through_what_ends_in_time_and_stops_the_rest() {
        assert_eq!(timeout(Duration::from_secs(1), async { 7 }).await, Ok(7));
        let slow = sleep(Duration::from_secs(5));
        assert_eq!(timeout(Duration::from_secs(1), slow).await, Err(Elapsed));
    }
}
