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
    // The changes first, then the new files.
    assert!(
        proposal.contains("Diff de : src/app.ts, .env.example."),
        "{proposal}"
    );
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
    assert!(!dir.join("never.txt").exists());
    assert_eq!(h.view(&id).setup, None);
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
    let started = std::time::Instant::now();
    assert_eq!(h.core.delete_agent(&a.meta.id, true).await.unwrap(), None);
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
    // Its test launch is isola's: `isola up` in the worktree.
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
    // Its setup was stopped with the project: it never ends its work.
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(!r.join("late.txt").exists());
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

// ---------- suggested by Claude ----------

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
    // The step whose folder leaves the project is dropped.
    assert_eq!(setup, [("npm ci", ""), ("npm run gen", "src")]);
    assert_eq!(s.teardown.len(), 1);
    assert_eq!(s.teardown[0].command, "docker compose down");
    let first = crate::pty::detect_shells(&h.core.settings.read())
        .first()
        .map(|s| s.id.clone())
        .unwrap();
    assert!(s.setup.iter().chain(&s.teardown).all(|x| x.shell == first));
    // Run in the project, with nothing but tools that read.
    let argv = h.launches(&r).pop().expect("claude run in the project");
    let tools = argv.iter().position(|a| a == "--tools").unwrap();
    assert_eq!(argv[tools + 1], "Read,Glob,Grep");
    assert!(argv.contains(&"--strict-mcp-config".to_string()));
    assert!(!argv
        .iter()
        .any(|a| a.contains("Bash") || a.contains("Edit")));
    let sent = serde_json::to_value(&s).unwrap();
    assert_eq!(sent["setup"][1]["cwd"], "src");
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
