//! The process groups of the agents Escouade starts, noted in a file of its own (ESC-417), so that
//! the next launch stops those an app killed left behind.
//!
//! On macOS and Linux an agent's process leads a group of its own (`job::isolate`), which the app
//! kills when it closes (`Core::shutdown`) or stops the agent. Killed itself (Jetsam under memory
//! pressure, a crash, a forced quit), it ends nothing: its agents run on with the ppid 1, unseen by
//! the next launch, which started as many again (issue #26: 1,642 starts of `claude` in a day). So
//! each group is noted, by its leader: its pid, when it started and its program. At the next
//! launch, a group noted whose leader still runs with the same start and the same program is
//! stopped (SIGTERM, then SIGKILL a few seconds later), with a line in the log; a pid that another
//! program took since is never touched. The interface of an agent in a pseudo-terminal is not
//! noted: the system hangs its session up when the app's end closes the terminal.
//!
//! On Windows nothing is noted: each agent's Job is created with KILL_ON_JOB_CLOSE (`job`), and the
//! system ends the whole tree with the app, however it ends.

// Off Unix, only its tests use it.
#![cfg_attr(not(unix), allow(dead_code))]

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// A group noted: its leader as the system told it when it started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Noted {
    pub pid: i32,
    #[serde(flatten)]
    pub identity: Identity,
}

/// What a process is, as the system tells it: when it started, and its program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub start: String,
    pub program: String,
}

/// What the system is asked of the processes (a test gives its own).
pub trait Probe {
    /// The process `pid`, if it runs.
    fn identity(&self, pid: i32) -> Option<Identity>;
    /// Sends the group `pgid` SIGTERM, or SIGKILL with `kill`.
    fn signal(&self, pgid: i32, kill: bool);
    /// Whether a process of the group `pgid` still runs.
    fn group_alive(&self, pgid: i32) -> bool;
}

/// The groups of this launch, and the file that holds them.
pub struct Ledger {
    file: PathBuf,
    noted: Mutex<Vec<Noted>>,
}

impl Ledger {
    pub fn new(file: PathBuf) -> Self {
        Ledger {
            file,
            noted: Mutex::new(Vec::new()),
        }
    }

    /// What the file holds: the groups the previous launch noted and did not end.
    pub fn left(&self) -> Vec<Noted> {
        std::fs::read(&self.file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Notes the group `pid` leads, as `probe` tells its leader; nothing when it is gone already.
    pub fn note(&self, pid: i32, probe: &dyn Probe) {
        let Some(identity) = probe.identity(pid) else {
            return;
        };
        let mut noted = self.noted.lock();
        noted.retain(|n| n.pid != pid);
        noted.push(Noted { pid, identity });
        self.write(&noted);
    }

    /// The group `pid` led is over.
    pub fn forget(&self, pid: i32) {
        let mut noted = self.noted.lock();
        let before = noted.len();
        noted.retain(|n| n.pid != pid);
        if noted.len() != before {
            self.write(&noted);
        }
    }

    /// What this launch has noted.
    #[cfg(test)]
    pub fn noted(&self) -> Vec<Noted> {
        self.noted.lock().clone()
    }

    fn write(&self, noted: &[Noted]) {
        let written = serde_json::to_vec(noted)
            .map_err(anyhow::Error::from)
            .and_then(|b| Ok(crate::paths::write_atomic(&self.file, &b)?));
        if let Err(e) = written {
            log::warn!("{}: not written: {e:#}", self.file.display());
        }
    }
}

/// Of the groups `left` noted, those whose leader still runs as it was noted: the same start, the
/// same program. A pid gone, or taken by another program since, is not one.
pub fn still_there(left: &[Noted], probe: &dyn Probe) -> Vec<Noted> {
    left.iter()
        .filter(|n| probe.identity(n.pid).as_ref() == Some(&n.identity))
        .cloned()
        .collect()
}

/// Stops the groups a previous launch left (`still_there`): SIGTERM, then SIGKILL to those still
/// there after `grace`, unless their leader's pid went to another program meanwhile. Returns those
/// it stopped.
pub fn stop_left(left: &[Noted], probe: &dyn Probe, grace: Duration) -> Vec<Noted> {
    let found = still_there(left, probe);
    for n in &found {
        log::warn!(
            "agent process group {} ({}, started {}) left by a previous launch: stopped",
            n.pid,
            n.identity.program,
            n.identity.start
        );
        probe.signal(n.pid, false);
    }
    let until = Instant::now() + grace;
    while found.iter().any(|n| probe.group_alive(n.pid)) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(100));
    }
    for n in &found {
        if !probe.group_alive(n.pid) {
            continue;
        }
        // Its leader's pid went to another program: its group is no longer the one noted.
        if probe.identity(n.pid).is_some_and(|id| id != n.identity) {
            continue;
        }
        log::warn!("agent process group {} still there: killed", n.pid);
        probe.signal(n.pid, true);
    }
    found
}

/// How long the groups left get to end after SIGTERM.
const GRACE: Duration = Duration::from_secs(3);

/// This launch's ledger, once `start` set it (the app's; tests make their own).
static LEDGER: OnceLock<Ledger> = OnceLock::new();

/// At the app's launch: the groups the previous one left are stopped, and this one's are noted in
/// `file` from now on. Nothing on Windows (see the module).
pub fn start(file: PathBuf) {
    #[cfg(unix)]
    {
        let ledger = Ledger::new(file);
        let left = ledger.left();
        if !left.is_empty() {
            stop_left(&left, &System, GRACE);
        }
        ledger.write(&[]);
        let _ = LEDGER.set(ledger);
    }
    #[cfg(not(unix))]
    let _ = file;
}

/// The group an agent's process `pid` leads, noted (once `start` set the ledger).
pub fn note(pid: i32) {
    #[cfg(unix)]
    if let Some(l) = LEDGER.get() {
        l.note(pid, &System);
    }
    #[cfg(not(unix))]
    let _ = pid;
}

/// The group `pid` led is over: forgotten.
pub fn forget(pid: i32) {
    if let Some(l) = LEDGER.get() {
        l.forget(pid);
    }
}

/// The system's answers, by `ps` and the signals of groups.
#[cfg(unix)]
pub struct System;

#[cfg(unix)]
impl System {
    /// One field of `ps` for `pid` (`lstart`, `comm`), in the C locale: None when it is gone.
    fn ps(pid: i32, field: &str) -> Option<String> {
        let out = std::process::Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", &format!("{field}=")])
            .env("LC_ALL", "C")
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (out.status.success() && !text.is_empty()).then_some(text)
    }
}

#[cfg(unix)]
impl Probe for System {
    fn identity(&self, pid: i32) -> Option<Identity> {
        Some(Identity {
            start: System::ps(pid, "lstart")?,
            program: System::ps(pid, "comm")?,
        })
    }

    fn signal(&self, pgid: i32, kill: bool) {
        let sig = if kill { libc::SIGKILL } else { libc::SIGTERM };
        // SAFETY: plain syscall; a group that is gone only yields ESRCH.
        unsafe {
            libc::killpg(pgid, sig);
        }
    }

    fn group_alive(&self, pgid: i32) -> bool {
        // SAFETY: signal 0 checks only that the group exists.
        unsafe { libc::killpg(pgid, 0) == 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A system of fake processes: the leaders it runs, the groups still alive, the signals sent.
    #[derive(Default)]
    struct Fake {
        running: Mutex<HashMap<i32, Identity>>,
        alive: Mutex<Vec<i32>>,
        /// Groups that end on SIGTERM.
        polite: Vec<i32>,
        sent: Mutex<Vec<(i32, bool)>>,
    }

    impl Probe for Fake {
        fn identity(&self, pid: i32) -> Option<Identity> {
            self.running.lock().get(&pid).cloned()
        }
        fn signal(&self, pgid: i32, kill: bool) {
            self.sent.lock().push((pgid, kill));
            if kill || self.polite.contains(&pgid) {
                self.alive.lock().retain(|p| *p != pgid);
                self.running.lock().remove(&pgid);
            }
        }
        fn group_alive(&self, pgid: i32) -> bool {
            self.alive.lock().contains(&pgid)
        }
    }

    fn id(start: &str, program: &str) -> Identity {
        Identity {
            start: start.into(),
            program: program.into(),
        }
    }

    fn ledger(name: &str) -> Ledger {
        let dir = escouade_base::testing::test_dir(&format!("agentgroups-{name}"));
        Ledger::new(dir.join("agent-groups.json"))
    }

    /// Criterion 4: a group left by a previous launch whose leader still runs as noted is stopped,
    /// SIGTERM first, SIGKILL when it stays; a pid another program took since is never touched,
    /// nor one gone.
    #[test]
    fn the_groups_a_previous_launch_left_are_stopped_never_a_pid_reused() {
        let before = ledger("left");
        let fake = Fake {
            polite: vec![10],
            ..Default::default()
        };
        for (pid, who) in [
            (10, id("Thu Nov 12 09:00:57 2026", "claude")),
            (11, id("Thu Nov 12 09:01:02 2026", "codex")),
            (12, id("Thu Nov 12 09:01:03 2026", "claude")),
            (13, id("Thu Nov 12 09:01:04 2026", "claude")),
            (14, id("Thu Nov 12 09:01:05 2026", "claude")),
        ] {
            fake.running.lock().insert(pid, who);
            before.note(pid, &fake);
        }
        assert_eq!(before.noted().len(), 5);
        // The previous launch was killed. Since: 12 ended and its pid went to another program, 13
        // to the same program started again, 14 is gone.
        fake.running
            .lock()
            .insert(12, id("Thu Nov 12 10:00:00 2026", "bash"));
        fake.running
            .lock()
            .insert(13, id("Thu Nov 12 10:00:01 2026", "claude"));
        fake.running.lock().remove(&14);
        *fake.alive.lock() = vec![10, 11, 12, 13];

        let next = Ledger::new(before.file.clone());
        let left = next.left();
        assert_eq!(left, before.noted());
        let stopped = stop_left(&left, &fake, Duration::from_millis(300));
        let pids: Vec<i32> = stopped.iter().map(|n| n.pid).collect();
        assert_eq!(pids, [10, 11]);
        // 10 ended on SIGTERM; 11 stayed, and was killed. 12 and 13 got nothing.
        assert_eq!(*fake.sent.lock(), [(10, false), (11, false), (11, true)]);
        assert!(fake.group_alive(12) && fake.group_alive(13));
    }

    #[test]
    fn a_group_over_is_forgotten_and_a_leader_gone_before_its_note_is_not_noted() {
        let l = ledger("forget");
        let fake = Fake::default();
        fake.running.lock().insert(20, id("a", "claude"));
        l.note(20, &fake);
        l.note(21, &fake);
        assert_eq!(l.noted().len(), 1);
        l.forget(20);
        assert!(l.noted().is_empty());
        assert!(Ledger::new(l.file.clone()).left().is_empty());
    }

    /// The same, with real processes: a group noted is stopped by the next launch, and an entry
    /// whose pid runs with another start (a pid reused) is not.
    #[cfg(unix)]
    #[test]
    fn a_real_group_left_is_stopped_and_a_reused_pid_is_not() {
        use std::os::unix::process::CommandExt;
        use std::process::Command;
        let spawn = || {
            let mut c = Command::new("sleep");
            c.arg("60").process_group(0);
            c.spawn().unwrap()
        };
        let (mut left, mut other) = (spawn(), spawn());
        let before = ledger("real");
        before.note(left.id() as i32, &System);
        let noted = before.noted();
        assert_eq!(noted[0].identity.program, "sleep");
        // The other one stands for a pid reused: noted with a start it does not have.
        let reused = Noted {
            pid: other.id() as i32,
            identity: Identity {
                start: "Thu Jan  1 00:00:00 1970".into(),
                program: "sleep".into(),
            },
        };
        let stopped = stop_left(&[noted[0].clone(), reused], &System, GRACE);
        assert_eq!(stopped.len(), 1);
        assert!(left.wait().unwrap().code().is_none(), "ended by a signal");
        assert!(other.try_wait().unwrap().is_none(), "never touched");
        other.kill().unwrap();
        let _ = other.wait();
    }
}
