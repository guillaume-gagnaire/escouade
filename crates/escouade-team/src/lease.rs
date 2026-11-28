//! The lease of the team (OE-2, `docs/TEAM-OVERSEER.md` §7.1): one machine of the team holds it and runs the team's
//! Overseer; another waits, says so, and takes it when it runs out.
//!
//! - **Taken** at the start ([`LeaseKeeper::start`]), **renewed** in the background every 3 minutes (the lease is of 10
//!   on the server), **given back** at the stop ([`LeaseKeeper::stop`]).
//! - **A token** comes with each take; every write in the name of the Overseer carries it ([`LeaseKeeper::write`]),
//!   and the server refuses one under a token another machine took over since (`stale_token`).
//! - **Lost cleanly**: once its renewal is refused, or its lease ran out on this machine's clock without a renewal the
//!   server heard, the keeper holds nothing: [`LeaseKeeper::fence`] gives no token, and nothing more is written under
//!   the old one.
//! - **Waiting**: [`LeaseState::Waiting`] names the machine that holds it (its device, since when, until when); the
//!   text is [`waiting_text`] (« En attente : l'Overseer d'équipe tourne sur la machine build-2 »), that OE-3 and OE-4
//!   show. It tries again at the end of the other's lease.
//!
//! The server sees of it a name, a device and hours; the machine's name stays in the encrypted projects
//! ([`crate::Team::machine_name`]).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{watch, Notify};
use tokio::task::JoinHandle;

use crate::client::{ClientError, ServerClient};
use crate::wire::{Lease, LeaseAnswer, LeaseFenceWire};

/// The name of the team's Overseer's lease (one per team, O6-A; one per project later, O6-B, by the same routes).
pub const OVERSEER: &str = "overseer";

/// How often the holder renews its lease of 10 minutes.
pub const RENEW_EVERY: Duration = Duration::from_secs(3 * 60);

/// What this machine knows of the lease.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaseState {
    /// Not asked yet, or the server not reached: this machine holds nothing.
    Unknown,
    /// This machine holds it, under `token`, since `since` (seconds of the server's clock).
    Held { token: u64, since: i64 },
    /// Another machine holds it (its device), since `since`, until `until` (seconds of the server's clock): this one
    /// waits, and takes it when it runs out.
    Waiting {
        holder: [u8; 16],
        since: i64,
        until: i64,
    },
    /// Given back: the keeper stopped.
    Stopped,
}

/// A write in the name of the lease's holder refused, or not sent.
#[derive(Debug, thiserror::Error)]
pub enum LeaseError {
    /// This machine does not hold the lease (any more): nothing is written in the name of the Overseer.
    #[error("this machine does not hold the lease of the team")]
    NotHeld,
    #[error(transparent)]
    Client(#[from] ClientError),
}

/// What a machine waiting says, with the name of the machine that holds the lease when it is known.
pub fn waiting_text(machine: Option<&str>) -> String {
    match machine {
        Some(m) if !m.trim().is_empty() => {
            format!(
                "En attente : l'Overseer d'équipe tourne sur la machine {}",
                m.trim()
            )
        }
        _ => "En attente : l'Overseer d'équipe tourne sur une autre machine de l'équipe".into(),
    }
}

struct Kept {
    state: LeaseState,
    /// When this machine's hold ends on its own clock, short of a renewal the server heard.
    deadline: Option<Instant>,
}

struct Inner {
    client: Arc<ServerClient>,
    name: String,
    every: Duration,
    me: [u8; 16],
    kept: Mutex<Kept>,
    changed: watch::Sender<LeaseState>,
    wake: Notify,
}

/// Keeps a lease of the team for this machine: takes it, renews it, waits for it, gives it back.
pub struct LeaseKeeper {
    inner: Arc<Inner>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl LeaseKeeper {
    /// Starts keeping the lease `name`, renewed every `every` ([`RENEW_EVERY`] in the app; shorter in the tests).
    pub fn start(client: Arc<ServerClient>, name: &str, every: Duration) -> Arc<LeaseKeeper> {
        let (changed, _) = watch::channel(LeaseState::Unknown);
        let inner = Arc::new(Inner {
            me: client.device(),
            client,
            name: name.to_string(),
            every,
            kept: Mutex::new(Kept {
                state: LeaseState::Unknown,
                deadline: None,
            }),
            changed,
            wake: Notify::new(),
        });
        let run = inner.clone();
        let task = tokio::spawn(async move {
            loop {
                let wait = run.step().await;
                tokio::select! {
                    _ = tokio::time::sleep(wait) => {}
                    _ = run.wake.notified() => {}
                }
            }
        });
        Arc::new(LeaseKeeper {
            inner,
            task: Mutex::new(Some(task)),
        })
    }

    /// The state now: a hold whose lease ran out on this machine's clock is none.
    pub fn state(&self) -> LeaseState {
        self.inner.state()
    }

    /// Follows the state (taken, lost, waiting, stopped).
    pub fn watch(&self) -> watch::Receiver<LeaseState> {
        self.inner.changed.subscribe()
    }

    /// Whether this machine holds the lease now.
    pub fn held(&self) -> bool {
        matches!(self.state(), LeaseState::Held { .. })
    }

    /// The token a write in the name of the holder carries; none when this machine does not hold the lease.
    pub fn fence(&self) -> Option<LeaseFenceWire> {
        match self.state() {
            LeaseState::Held { token, .. } => Some(LeaseFenceWire {
                name: self.inner.name.clone(),
                token,
            }),
            _ => None,
        }
    }

    /// The device that holds the lease while this machine waits.
    pub fn holder(&self) -> Option<[u8; 16]> {
        match self.state() {
            LeaseState::Waiting { holder, .. } => Some(holder),
            _ => None,
        }
    }

    /// Writes a message in the team's stream (`project` None) or a project's, in the name of the lease's holder:
    /// [`LeaseError::NotHeld`] without the lease; refused by the server (`stale_token`) once another machine took it
    /// over, after which this machine holds nothing.
    pub async fn write(
        &self,
        project: Option<&[u8; 16]>,
        payload: &[u8],
    ) -> Result<u64, LeaseError> {
        let fence = self.fence().ok_or(LeaseError::NotHeld)?;
        match self
            .inner
            .client
            .append_leased(project, payload, fence)
            .await
        {
            Ok(seq) => Ok(seq),
            Err(ClientError::Lease { code, lease }) => {
                self.inner.lost(&lease);
                self.inner.wake.notify_one();
                Err(ClientError::Lease { code, lease }.into())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// One pass now (take, renew or look again), instead of waiting for the next.
    pub fn poke(&self) {
        self.inner.wake.notify_one();
    }

    /// Stops: no more renewal, and the lease given back when this machine holds it (the other takes it at once).
    pub async fn stop(&self) {
        let task = self.task.lock().expect("the keeper's task").take();
        if let Some(t) = task {
            t.abort();
            let _ = t.await;
        }
        let token = match self.inner.kept.lock().expect("the lease kept").state {
            LeaseState::Held { token, .. } => Some(token),
            _ => None,
        };
        if let Some(token) = token {
            let _ = self
                .inner
                .client
                .release_lease(&self.inner.name, token)
                .await;
        }
        self.inner.set(LeaseState::Stopped, None);
    }
}

impl Drop for LeaseKeeper {
    fn drop(&mut self) {
        if let Some(t) = self.task.lock().ok().and_then(|mut t| t.take()) {
            t.abort();
        }
    }
}

impl Inner {
    fn state(&self) -> LeaseState {
        let mut kept = self.kept.lock().expect("the lease kept");
        if matches!(kept.state, LeaseState::Held { .. })
            && kept.deadline.is_some_and(|d| Instant::now() >= d)
        {
            // Run out without a renewal the server heard: held no more.
            kept.state = LeaseState::Unknown;
            kept.deadline = None;
            self.changed.send_replace(LeaseState::Unknown);
        }
        kept.state.clone()
    }

    fn set(&self, state: LeaseState, deadline: Option<Instant>) {
        let mut kept = self.kept.lock().expect("the lease kept");
        if kept.state == LeaseState::Stopped {
            return;
        }
        kept.deadline = deadline;
        if kept.state != state {
            kept.state = state.clone();
            self.changed.send_replace(state);
        }
    }

    /// Held from `asked` (when the request left, on this machine's clock: the server's lease ends after).
    fn hold(&self, a: &LeaseAnswer, asked: Instant) {
        let length = Duration::from_secs(a.length.max(1) as u64);
        self.set(
            LeaseState::Held {
                token: a.lease.token,
                since: a.lease.claimed_at,
            },
            Some(asked + length),
        );
    }

    /// The lease as the server said it in a refusal or a reading: another machine's, or nobody's.
    fn lost(&self, lease: &Lease) {
        match lease
            .holder
            .as_deref()
            .and_then(|h| <[u8; 16]>::try_from(h).ok())
        {
            Some(h) if h != self.me => self.set(
                LeaseState::Waiting {
                    holder: h,
                    since: lease.claimed_at,
                    until: lease.until,
                },
                None,
            ),
            _ => self.set(LeaseState::Unknown, None),
        }
    }

    /// One pass; answers how long to wait before the next.
    async fn step(&self) -> Duration {
        let retry = (self.every / 3).max(Duration::from_millis(50));
        if let LeaseState::Held { token, .. } = self.state() {
            let asked = Instant::now();
            match self.client.renew_lease(&self.name, token).await {
                Ok(a) => {
                    self.hold(&a, asked);
                    return self.every;
                }
                Err(ClientError::Lease { lease, .. }) => self.lost(&lease),
                // Not reached: the hold lasts until its deadline, the renewal is tried again sooner.
                Err(_) => return retry,
            }
        }
        // Not held: who holds it, and until when; taken when it is free or run out.
        let seen = match self.client.lease(&self.name).await {
            Ok(a) => a,
            Err(_) => {
                self.set(LeaseState::Unknown, None);
                return retry;
            }
        };
        let other = seen
            .lease
            .holder
            .as_deref()
            .is_some_and(|h| h != self.me.as_slice());
        if other && seen.lease.until > seen.now {
            self.lost(&seen.lease);
            return self.until_end(&seen);
        }
        let asked = Instant::now();
        match self.client.take_lease(&self.name).await {
            Ok(a) => {
                self.hold(&a, asked);
                self.every
            }
            Err(ClientError::Lease { lease, .. }) => {
                self.lost(&lease);
                retry
            }
            Err(_) => {
                self.set(LeaseState::Unknown, None);
                retry
            }
        }
    }

    /// Until the other's lease runs out (and a little after), the renewal's period at most.
    fn until_end(&self, seen: &LeaseAnswer) -> Duration {
        let left = (seen.lease.until - seen.now).max(0) as u64;
        (Duration::from_secs(left) + Duration::from_millis(200)).min(self.every)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_waiting_machine_says_who_runs_the_overseer() {
        assert_eq!(
            waiting_text(Some("build-2")),
            "En attente : l'Overseer d'équipe tourne sur la machine build-2"
        );
        assert_eq!(
            waiting_text(None),
            "En attente : l'Overseer d'équipe tourne sur une autre machine de l'équipe"
        );
    }
}
