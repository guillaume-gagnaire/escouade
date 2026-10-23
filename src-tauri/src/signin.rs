//! « OAuth session expired and could not be refreshed » (ESC-162): a turn of Claude Code stopped
//! by a sign-in of its account that its process could not refresh, as Claude Code 2.1.309 tells it.
//!
//! What its binary does (read in it, docs/PROTOCOL.md, « Connexion qui ne se rafraîchit pas »):
//! its processes of one account refresh the token under a lock of its folder
//! (`.oauth_refresh.lock`) and read the file again under it, so that one whose token another just
//! refreshed takes that one. The error comes when the refresh token was refused (`invalid_grant`):
//! the process keeps it for dead in memory, and empties the file if it still holds it. Until it
//! reads the file again (at most every 30 s), each of its turns fails at once, even though another
//! process may have written a good sign-in meanwhile: started again, it reads it. Every process
//! started on an out-of-date token refreshes it before it answers anything, `initialize` included.
//!
//! So Escouade, once such a turn ended, reads the account's sign-in: good, or another than the one
//! the process started with, it starts the process again on its session and sends it "continue"
//! (twice at most, waiting a little longer each time); otherwise the card asks for a new sign-in,
//! and the agent goes on by itself once it comes. And the processes of an account whose token is
//! out of date start one after the other (`RefreshGate`): the first refreshes it, the others wait
//! for the new one rather than all queue for the lock, whose wait Claude Code gives up after 8 to
//! 17 s (« another Claude Code process is refreshing it »).

use crate::accounts;
use crate::agent::{AgentAlert, AgentHandle, NotifyKind};
use crate::core::Core;
use crate::i18n::{self, Lang};
use crate::model::*;
use crate::usage::{self, Credentials, Stored};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tauri::Runtime;

/// What Claude Code says of a sign-in its process could not refresh: refused (`OAuthRefreshDeadError`),
/// or held by another process past its wait (`OAuthRefreshLockTimeoutError`).
const LOST: [&str; 2] = [
    "OAuth session expired and could not be refreshed",
    "Failed to refresh OAuth token: another Claude Code process is refreshing it",
];

/// Whether `text`, the error of a turn, is a sign-in Claude Code could not refresh.
pub(crate) fn lost(text: &str) -> bool {
    LOST.iter().any(|l| text.contains(l))
}

/// The restarts of the process for one stopped turn.
pub(crate) const RETRIES: u8 = 2;

/// How long before the restart after `done` of them: 3 s, then 10 s. The variable
/// `ESCOUADE_SIGNIN_RETRY_MS` sets the first (the second three times as long), for the e2e; the
/// unit tests wait 50 ms.
pub(crate) fn retry_delay(done: u8) -> Duration {
    let base = std::env::var("ESCOUADE_SIGNIN_RETRY_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .or(cfg!(test).then_some(50));
    delay_of(base, done)
}

/// `retry_delay` with the first wait `base` (ms) set, else Escouade's own.
fn delay_of(base: Option<u64>, done: u8) -> Duration {
    match (base, done) {
        (Some(ms), 0) => Duration::from_millis(ms),
        (Some(ms), _) => Duration::from_millis(ms * 3),
        (None, 0) => Duration::from_secs(3),
        (None, _) => Duration::from_secs(10),
    }
}

/// Whether the process can go on with the sign-in read now (`now`), having started with `launch`:
/// a token in date, or another sign-in than its own (another process refreshed it, or the user
/// signed in again). Unreadable, gone (Claude Code empties it when its refresh token is refused),
/// or the same out of date, it cannot.
pub(crate) fn can_go_on(now: Option<&Stored>, launch: Option<&str>) -> bool {
    match now.map(|s| (&s.credentials, s.stamp.as_deref())) {
        Some((Credentials::Token(_), _)) => true,
        Some((Credentials::Expired, stamp)) => stamp != launch,
        _ => false,
    }
}

/// Whether the sign-in read now is the new one an agent waits for (`AgentMeta::sign_in`).
pub(crate) fn signed_in_again(now: Option<&Stored>, wait: &SignInWait) -> bool {
    now.is_some_and(|s| s.credentials != Credentials::Missing && s.stamp != wait.stamp)
}

/// What the conversation says when the process starts again with a good sign-in.
pub(crate) fn refreshed_line(lang: Lang) -> String {
    tr_in!(
        lang,
        "Session du compte rafraîchie : l’agent reprend.",
        "Account session refreshed: the agent goes on."
    )
}

/// What the conversation says when the account signed in again and the agent goes on.
pub(crate) fn signed_in_line(lang: Lang, account: &str) -> String {
    tr_in!(
        lang,
        "Compte {account} reconnecté : l’agent reprend.",
        "{account} account signed in again: the agent goes on."
    )
}

/// What the notification says of an agent waiting for its account to sign in again.
pub(crate) fn wait_body(lang: Lang, account: &str) -> String {
    tr_in!(
        lang,
        "Le compte {account} doit se reconnecter : l’agent reprendra seul après.",
        "The {account} account must sign in again: the agent will go on by itself after."
    )
}

/// The launches of Claude Code of an account whose token is out of date, one after the other: the
/// first goes and refreshes it, the next ones wait for another sign-in (`bound` at most, then go
/// all the same).
pub(crate) struct RefreshGate {
    /// The account (its folder) whose first launch went, and when.
    going: Mutex<HashMap<String, Instant>>,
}

/// Every launch of Escouade's goes through this one.
pub(crate) static GATE: LazyLock<RefreshGate> = LazyLock::new(RefreshGate::new);

/// How long a launch waits at most for the first one's refresh.
pub(crate) const GATE_BOUND: Duration = Duration::from_secs(20);

impl RefreshGate {
    pub(crate) fn new() -> Self {
        Self {
            going: Mutex::new(HashMap::new()),
        }
    }

    /// Lets a launch of the account `key` go, its sign-in read by `read`, once it may: at once
    /// when its token is in date (or not read), or when it is the first since the token went out
    /// of date; else once the sign-in changed, `bound` at most, looked at every `poll`. Returns
    /// the sign-in read last, the one the process starts with.
    pub(crate) async fn pass<F, Fut>(
        &self,
        key: &str,
        bound: Duration,
        poll: Duration,
        read: F,
    ) -> Option<Stored>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Option<Stored>>,
    {
        let first = read().await;
        let Some(stamp) = first
            .as_ref()
            .filter(|s| s.credentials == Credentials::Expired)
            .map(|s| s.stamp.clone())
        else {
            self.going.lock().remove(key);
            return first;
        };
        {
            let mut going = self.going.lock();
            match going.get(key) {
                Some(since) if since.elapsed() < bound => {}
                _ => {
                    going.insert(key.to_string(), Instant::now());
                    return first;
                }
            }
        }
        let until = Instant::now() + bound;
        let mut last = first;
        while Instant::now() < until {
            tokio::time::sleep(poll).await;
            last = read().await;
            let changed = last
                .as_ref()
                .is_none_or(|s| s.credentials != Credentials::Expired || s.stamp != stamp);
            if changed {
                self.going.lock().remove(key);
                return last;
            }
        }
        log::info!("sign-in of {key}: still out of date after {bound:?}, the launch goes");
        last
    }
}

/// The sign-in of `account` (Claude Code's), None when it could not be read.
async fn read(account: &Account, settings: &Settings) -> Option<Stored> {
    let at = usage::sign_in(account, settings);
    match usage::read_stored(&at.dir, &at.service).await {
        Ok(stored) => Some(stored),
        Err(e) => {
            log::debug!("sign-in of account {}: {e:#}", account.id);
            None
        }
    }
}

impl<R: Runtime> Core<R> {
    /// Before a process of Claude Code starts on `account`: its turn, when its token is out of
    /// date (`GATE`). Returns its sign-in then (`Stored::stamp`), None for another harness.
    pub(crate) async fn before_claude(&self, account: &Account) -> Option<String> {
        if account.engine != EngineId::Claude {
            return None;
        }
        let settings = self.settings.read().clone();
        let key = usage::sign_in(account, &settings)
            .dir
            .to_string_lossy()
            .into_owned();
        let poll = Duration::from_millis(500);
        GATE.pass(&key, GATE_BOUND, poll, || read(account, &settings))
            .await
            .and_then(|s| s.stamp)
    }

    /// Stops the agent's process, which keeps the sign-in it could not refresh in its memory
    /// (Claude Code reads the file again 30 s later at best): the next message starts a new one,
    /// on its session. False when the agent was taken over meanwhile (archived, or working).
    async fn stop_for_sign_in(&self, h: &AgentHandle) -> bool {
        let stopped = {
            let mut rt = h.lock();
            if rt.meta.archived || rt.meta.status.is_active() {
                return false;
            }
            rt.detach()
        };
        if let Some(p) = stopped {
            p.kill();
            p.wait_gone(Duration::from_secs(5)).await;
            self.emit_agent(h);
        }
        true
    }

    /// `sign_in_lost`, in the background: a turn of the agent just stopped on a sign-in that
    /// could not be refreshed.
    pub(crate) fn sign_in_lost_soon(self: &Arc<Self>, id: &str) {
        let (c, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            c.sign_in_lost(&id).await;
        });
    }

    /// The agent's turn stopped on a sign-in its process could not refresh: with a good one on
    /// disk (or another than the one the process started with), its process starts again on its
    /// session, which goes on ("continue"), `RETRIES` times at most; else it waits for the account
    /// to sign in again (`sign_in_due`), and the card says so.
    pub(crate) async fn sign_in_lost(self: &Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let Ok(account) = self.agent_account(&h) else {
            return;
        };
        let settings = self.settings.read().clone();
        let (launch, done) = {
            let rt = h.lock();
            (rt.launch_stamp.clone(), rt.sign_in_retries)
        };
        let now = read(&account, &settings).await;
        let lang = i18n::ui();
        let name = accounts::name_in(lang, &account);
        if done < RETRIES && can_go_on(now.as_ref(), launch.as_deref()) {
            h.lock().sign_in_retries = done + 1;
            tokio::time::sleep(retry_delay(done)).await;
            // Taken over meanwhile (a message, an archive, a stop), or gone: nothing to do.
            if !self.stop_for_sign_in(&h).await {
                return;
            }
            log::info!(
                "agent {id}: sign-in of the account {} not refreshed by its process, started again ({})",
                account.id,
                done + 1
            );
            let _ = self.with_agent(id, |rt, fx| {
                rt.notice("info", refreshed_line(lang), fx);
                Ok(())
            });
            if let Err(e) = self.send_continue(id).await {
                log::warn!("agent {id}: no resume after the sign-in: {e:#}");
            }
            return;
        }
        log::info!(
            "agent {id}: waits for the account {} to sign in again",
            account.id
        );
        let wait = SignInWait {
            account: account.id.clone(),
            stamp: now.and_then(|s| s.stamp),
        };
        let Ok((project, agent)) = self.with_agent(id, |rt, fx| {
            rt.meta.sign_in = Some(wait);
            fx.save = true;
            Ok((rt.meta.project_id.clone(), rt.meta.name.clone()))
        }) else {
            return;
        };
        let alert = AgentAlert {
            kind: NotifyKind::Error,
            body: wait_body(lang, &name),
        };
        self.notify_agent(alert, &project, id, &agent);
    }

    /// The agents waiting for their account to sign in again (`sign_in_lost`) whose sign-in
    /// changed go on by themselves: "continue", as after a usage limit.
    pub async fn sign_in_due(self: &Arc<Self>) {
        let waiting: Vec<(String, SignInWait)> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                let wait = rt.meta.sign_in.clone()?;
                (!rt.meta.archived).then(|| (rt.meta.id.clone(), wait))
            })
            .collect();
        if waiting.is_empty() {
            return;
        }
        let settings = self.settings.read().clone();
        let lang = i18n::ui();
        for (id, wait) in waiting {
            let account = accounts::get(&settings, &wait.account);
            let now = read(&account, &settings).await;
            if !signed_in_again(now.as_ref(), &wait) {
                continue;
            }
            // Taken under the agent's lock: a message just before took it over.
            let Ok(still) = self.with_agent(&id, |rt, fx| {
                if rt.meta.sign_in.as_ref() != Some(&wait) || rt.meta.status.is_active() {
                    return Ok(false);
                }
                rt.meta.sign_in = None;
                rt.sign_in_retries = 0;
                fx.save = true;
                let name = accounts::name_in(lang, &account);
                rt.notice("info", signed_in_line(lang, &name), fx);
                Ok(true)
            }) else {
                continue;
            };
            if !still {
                continue;
            }
            // Its process, if it lives, holds the dead sign-in: a new one reads the new.
            let Ok(h) = self.agent(&id) else { continue };
            if !self.stop_for_sign_in(&h).await {
                continue;
            }
            log::info!("agent {id}: the account {} signed in again", account.id);
            if let Err(e) = self.send_continue(&id).await {
                log::warn!("agent {id}: no resume after the sign-in: {e:#}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn stored(credentials: Credentials, stamp: Option<&str>) -> Stored {
        Stored {
            credentials,
            stamp: stamp.map(str::to_string),
        }
    }

    #[test]
    fn the_two_errors_of_a_sign_in_not_refreshed_are_told_from_others() {
        assert!(lost(
            "Failed to authenticate: OAuth session expired and could not be refreshed"
        ));
        assert!(lost("Failed to refresh OAuth token: another Claude Code process is refreshing it or exited mid-refresh. This is usually transient; retry in a minute"));
        assert!(!lost("Invalid API key · Please run /login"));
        assert!(!lost("You've hit your limit · resets 3pm"));
        assert!(!lost("API Error: 500"));
    }

    #[test]
    fn the_process_goes_on_with_a_good_sign_in_or_another_one() {
        let token = stored(Credentials::Token("t".into()), Some("a"));
        assert!(can_go_on(Some(&token), Some("a")));
        // Another process refreshed it: out of date already, but another.
        let other = stored(Credentials::Expired, Some("b"));
        assert!(can_go_on(Some(&other), Some("a")));
        // The same, out of date: nobody refreshed it.
        assert!(!can_go_on(
            Some(&stored(Credentials::Expired, Some("a"))),
            Some("a")
        ));
        // Emptied by Claude Code (its refresh token refused), or never there, or unreadable.
        assert!(!can_go_on(Some(&stored(Credentials::Missing, None)), None));
        assert!(!can_go_on(
            Some(&stored(Credentials::Missing, None)),
            Some("a")
        ));
        assert!(!can_go_on(None, Some("a")));
    }

    #[test]
    fn a_wait_ends_with_another_sign_in_only() {
        let wait = SignInWait {
            account: "pro".into(),
            stamp: None,
        };
        assert!(!signed_in_again(
            Some(&stored(Credentials::Missing, None)),
            &wait
        ));
        assert!(signed_in_again(
            Some(&stored(Credentials::Token("t".into()), Some("n"))),
            &wait
        ));
        let wait = SignInWait {
            account: "pro".into(),
            stamp: Some("a".into()),
        };
        assert!(!signed_in_again(
            Some(&stored(Credentials::Expired, Some("a"))),
            &wait
        ));
        assert!(signed_in_again(
            Some(&stored(Credentials::Expired, Some("b"))),
            &wait
        ));
        assert!(!signed_in_again(None, &wait));
    }

    #[test]
    fn the_lines_are_said_in_both_languages() {
        assert_eq!(
            refreshed_line(Lang::Fr),
            "Session du compte rafraîchie : l’agent reprend."
        );
        assert_eq!(
            refreshed_line(Lang::En),
            "Account session refreshed: the agent goes on."
        );
        assert_eq!(
            signed_in_line(Lang::Fr, "Pro"),
            "Compte Pro reconnecté : l’agent reprend."
        );
        assert_eq!(
            wait_body(Lang::En, "Pro"),
            "The Pro account must sign in again: the agent will go on by itself after."
        );
    }

    #[test]
    fn the_restarts_wait_longer_each_time() {
        assert_eq!(delay_of(None, 0), Duration::from_secs(3));
        assert_eq!(delay_of(None, 1), Duration::from_secs(10));
        assert_eq!(delay_of(Some(200), 1), Duration::from_millis(600));
        assert!(retry_delay(1) > retry_delay(0));
    }

    /// A sign-in read from a value the test changes, counting the readings.
    struct Disk {
        now: Mutex<Option<Stored>>,
        reads: AtomicUsize,
    }

    impl Disk {
        fn new(s: Option<Stored>) -> Arc<Self> {
            Arc::new(Self {
                now: Mutex::new(s),
                reads: AtomicUsize::new(0),
            })
        }
        fn read(self: &Arc<Self>) -> impl Fn() -> std::future::Ready<Option<Stored>> {
            let d = self.clone();
            move || {
                d.reads.fetch_add(1, Ordering::SeqCst);
                std::future::ready(d.now.lock().clone())
            }
        }
    }

    const POLL: Duration = Duration::from_millis(10);

    #[tokio::test]
    async fn a_token_in_date_lets_every_launch_go_at_once() {
        let gate = RefreshGate::new();
        let disk = Disk::new(Some(stored(Credentials::Token("t".into()), Some("a"))));
        for _ in 0..3 {
            let t = Instant::now();
            let s = gate
                .pass("pro", Duration::from_secs(5), POLL, disk.read())
                .await;
            assert_eq!(s.and_then(|s| s.stamp).as_deref(), Some("a"));
            assert!(t.elapsed() < Duration::from_millis(200));
        }
        assert_eq!(disk.reads.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn out_of_date_the_first_launch_goes_and_the_next_wait_for_its_refresh() {
        let gate = Arc::new(RefreshGate::new());
        let disk = Disk::new(Some(stored(Credentials::Expired, Some("old"))));
        let t = Instant::now();
        // The first goes at once: it is the one that refreshes.
        let first = gate
            .pass("pro", Duration::from_secs(5), POLL, disk.read())
            .await;
        assert_eq!(first.and_then(|s| s.stamp).as_deref(), Some("old"));
        assert!(t.elapsed() < Duration::from_millis(200));
        // The next two wait…
        let (g1, g2) = (gate.clone(), gate.clone());
        let (r1, r2) = (disk.read(), disk.read());
        let next1 =
            tokio::spawn(async move { g1.pass("pro", Duration::from_secs(5), POLL, r1).await });
        let next2 =
            tokio::spawn(async move { g2.pass("pro", Duration::from_secs(5), POLL, r2).await });
        // …another account's launch does not.
        let other = Disk::new(Some(stored(Credentials::Expired, Some("x"))));
        let t = Instant::now();
        gate.pass("team", Duration::from_secs(5), POLL, other.read())
            .await;
        assert!(t.elapsed() < Duration::from_millis(200));
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(!next1.is_finished() && !next2.is_finished());
        // …until the first one wrote its new sign-in.
        *disk.now.lock() = Some(stored(Credentials::Token("new".into()), Some("new")));
        let (a, b) = (next1.await.unwrap(), next2.await.unwrap());
        assert_eq!(a.and_then(|s| s.stamp).as_deref(), Some("new"));
        assert_eq!(b.and_then(|s| s.stamp).as_deref(), Some("new"));
        assert!(t.elapsed() < Duration::from_secs(2));
        // The token in date again: the next launch goes at once, and a later one out of date is a
        // first again.
        *disk.now.lock() = Some(stored(Credentials::Expired, Some("new")));
        let t = Instant::now();
        gate.pass("pro", Duration::from_secs(5), POLL, disk.read())
            .await;
        assert!(t.elapsed() < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn a_refresh_that_never_comes_holds_a_launch_for_the_bound_only() {
        let gate = RefreshGate::new();
        let disk = Disk::new(Some(stored(Credentials::Expired, Some("old"))));
        let bound = Duration::from_millis(300);
        gate.pass("pro", bound, POLL, disk.read()).await;
        let t = Instant::now();
        let s = gate.pass("pro", bound, POLL, disk.read()).await;
        let waited = t.elapsed();
        assert!(waited >= bound && waited < bound * 4, "{waited:?}");
        assert_eq!(s.map(|s| s.credentials), Some(Credentials::Expired));
        // Its bound over, the first's turn is over too: the next one goes as a first.
        tokio::time::sleep(bound).await;
        let t = Instant::now();
        gate.pass("pro", bound, POLL, disk.read()).await;
        assert!(t.elapsed() < Duration::from_millis(200));
    }

    #[tokio::test]
    async fn a_refresh_refused_lets_the_waiting_launches_go() {
        let gate = Arc::new(RefreshGate::new());
        let disk = Disk::new(Some(stored(Credentials::Expired, Some("old"))));
        gate.pass("pro", Duration::from_secs(5), POLL, disk.read())
            .await;
        let (g, r) = (gate.clone(), disk.read());
        let next =
            tokio::spawn(async move { g.pass("pro", Duration::from_secs(5), POLL, r).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        // Claude Code empties the file when its refresh token is refused.
        *disk.now.lock() = Some(stored(Credentials::Missing, None));
        let s = next.await.unwrap();
        assert_eq!(s.map(|s| s.credentials), Some(Credentials::Missing));
    }
}
