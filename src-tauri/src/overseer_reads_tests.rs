//! ESC-646 (SR4-02, SR4-03): what the Overseer grants and reads without the user. A read lands where its links lead; a
//! place of credentials or keys is never one, whatever folder holds it; a command is never a read, however written.

use crate::agent::{AgentRt, Effects};
use crate::engine::{BoxFuture, EngineProcess, Message, PermissionAnswer, Ready, Remote};
use crate::job::JobUsage;
use crate::model::AgentMeta;
use crate::overseer::{outgoing_in, Outgoing};
use crate::overseer_reads::{read_within, Refusal, AGENT_READS, OWN_READS};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A folder of its own for a test, empty.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "escouade-overseer-reads-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `dst` made a link to the folder `src` (a junction on Windows).
fn link(src: &Path, dst: &Path) {
    #[cfg(windows)]
    crate::accounts::link_dir(src, dst).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(src, dst).unwrap();
}

fn judge(
    tool: &str,
    input: Value,
    cwd: &Path,
    roots: &[PathBuf],
    home: &Path,
) -> Result<(), Refusal> {
    read_within(&OWN_READS, tool, &input, cwd, roots, Some(home))
}

#[test]
fn a_read_lands_where_its_links_lead_and_never_on_a_secret() {
    let dir = folder("pure");
    let home = dir.join("home");
    let project = dir.join("projet");
    let outside = dir.join("ailleurs");
    for d in [
        &home,
        &project,
        &outside,
        &home.join(".claude"),
        &project.join("src"),
    ] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(project.join("src").join("lib.rs"), "x").unwrap();
    std::fs::write(outside.join("a.txt"), "x").unwrap();
    link(&outside, &project.join("lien"));
    let roots = [project.clone()];
    let f = |p: &Path| json!({ "file_path": p.to_string_lossy() });
    // In the project, by its path, relative to its folder, by a search or a pattern.
    assert_eq!(
        judge(
            "Read",
            f(&project.join("src").join("lib.rs")),
            &project,
            &roots,
            &home
        ),
        Ok(())
    );
    assert_eq!(
        judge(
            "Read",
            json!({ "file_path": "src/lib.rs" }),
            &project,
            &roots,
            &home
        ),
        Ok(())
    );
    assert_eq!(
        judge(
            "Read",
            json!({ "file_path": "src/nouveau.rs" }),
            &project,
            &roots,
            &home
        ),
        Ok(())
    );
    assert_eq!(
        judge("Grep", json!({ "pattern": "x" }), &project, &roots, &home),
        Ok(())
    );
    assert_eq!(
        judge(
            "Glob",
            json!({ "pattern": "src/**/*.rs" }),
            &project,
            &roots,
            &home
        ),
        Ok(())
    );
    assert_eq!(
        judge(
            "LS",
            json!({ "path": project.to_string_lossy() }),
            &project,
            &roots,
            &home
        ),
        Ok(())
    );
    // Out of it: by its path, by `..`, through a link of the project, by a pattern that climbs or starts elsewhere.
    for (tool, input) in [
        ("Read", f(&outside.join("a.txt"))),
        ("Read", json!({ "file_path": "../ailleurs/a.txt" })),
        ("Read", f(&project.join("lien").join("a.txt"))),
        ("Glob", json!({ "pattern": "../**/*" })),
        (
            "Glob",
            json!({ "pattern": format!("{}/**", outside.to_string_lossy().replace('\\', "/")) }),
        ),
        (
            "Grep",
            json!({ "pattern": "x", "path": outside.to_string_lossy() }),
        ),
    ] {
        assert!(
            matches!(
                judge(tool, input.clone(), &project, &roots, &home),
                Err(Refusal::Outside(_))
            ),
            "{tool} {input}"
        );
    }
    // Not judged for sure: a `..` after a wildcard, a filter of files that climbs, no path at all.
    for (tool, input) in [
        ("Glob", json!({ "pattern": "src/*/../../../ailleurs/*" })),
        ("Grep", json!({ "pattern": "x", "glob": "../**" })),
        ("Read", json!({})),
    ] {
        assert_eq!(
            judge(tool, input.clone(), &project, &roots, &home),
            Err(Refusal::Unjudged),
            "{tool} {input}"
        );
    }
    // A harness's credentials, a key: refused even in a folder allowed (the user's own, here).
    let all = [home.clone(), project.clone()];
    for (tool, input) in [
        ("Read", f(&home.join(".claude").join(".credentials.json"))),
        ("Read", json!({ "file_path": "~/.codex/auth.json" })),
        (
            "Grep",
            json!({ "pattern": "BEGIN", "path": home.join(".ssh").to_string_lossy() }),
        ),
        (
            "Read",
            f(&home.join(".escouade-signing").join("escouade.key")),
        ),
        ("Read", f(&project.join(".env"))),
        (
            "LS",
            json!({ "path": home.join(".claude").to_string_lossy() }),
        ),
    ] {
        assert!(
            matches!(
                judge(tool, input.clone(), &home, &all, &home),
                Err(Refusal::Secret(_))
            ),
            "{tool} {input}"
        );
    }
    // A command, a write, the web: no read, whatever they hold.
    for tool in [
        "Bash",
        "PowerShell",
        "Write",
        "Edit",
        "WebFetch",
        "mcp__x__y",
    ] {
        assert_eq!(
            judge(
                tool,
                json!({ "file_path": "src/lib.rs", "command": "cat src/lib.rs" }),
                &project,
                &roots,
                &home
            ),
            Err(Refusal::NotRead)
        );
    }
    // What the Overseer grants an agent is narrower: no `LS`.
    assert_eq!(
        read_within(
            &AGENT_READS,
            "LS",
            &json!({ "path": "." }),
            &project,
            &roots,
            Some(&home)
        ),
        Err(Refusal::NotRead)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// SR4-02: the guard of what the Overseer tells an agent sees a command however a shell would hide it.
#[test]
fn the_guard_of_its_messages_sees_a_push_written_otherwise() {
    for text in [
        r"\git push origin main",
        r#"g""it push origin main"#,
        "git${IFS}push origin main",
        "command git push origin main",
        "env git push origin main",
        "git -c alias.p=push p origin main",
        "git config alias.p push && git p origin main",
        r#""git" "push" origin main"#,
    ] {
        let found = outgoing_in(text);
        assert!(
            found.iter().any(|(a, _)| *a == Outgoing::Push),
            "{text}: {found:?}"
        );
    }
    assert!(outgoing_in(r"\git merge feature")
        .iter()
        .any(|(a, _)| *a == Outgoing::Merge));
    assert!(outgoing_in("git status && git log").is_empty());
}

// ---------- the Overseer's own harness, as an agent ----------

/// A process of an agent that notes the answers to its permissions.
#[derive(Default)]
struct Recorder(Mutex<Vec<(String, String)>>);

impl EngineProcess for Recorder {
    fn is_alive(&self) -> bool {
        true
    }
    fn usage(&self) -> Option<JobUsage> {
        None
    }
    fn usage_in(&self, _: &crate::job::Sample) -> Option<JobUsage> {
        None
    }
    fn still_running(&self) -> u32 {
        1
    }
    fn pids(&self) -> Vec<u32> {
        Vec::new()
    }
    fn close_input(&self) {}
    fn kill(&self) {}
    fn wait_exit(&self, _: Duration) -> BoxFuture<'_, bool> {
        Box::pin(async { true })
    }
    fn wait_gone(&self, _: Duration) -> BoxFuture<'_, bool> {
        Box::pin(async { true })
    }
    fn initialize(&self) -> BoxFuture<'_, Result<Ready>> {
        Box::pin(async { Err(anyhow::anyhow!("not here")) })
    }
    fn send_message(&self, _: &str, _: &Message) -> Result<()> {
        Ok(())
    }
    fn interrupt(&self, _: bool, _: Duration) -> BoxFuture<'_, Result<()>> {
        Box::pin(async { Ok(()) })
    }
    fn set_model<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
    fn set_effort<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
    fn set_mode<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
    fn answer_permission(&self, request_id: &str, answer: PermissionAnswer) -> Result<()> {
        let kind = match answer {
            PermissionAnswer::Allow { .. } => "allow",
            PermissionAnswer::Always { .. } => "always",
            _ => "deny",
        };
        self.0
            .lock()
            .unwrap()
            .push((request_id.to_string(), kind.to_string()));
        Ok(())
    }
    fn answer_question(&self, _: &str, _: &str, _: Value) -> Result<()> {
        Ok(())
    }
    fn read_usage(&self) -> BoxFuture<'_, Result<Option<Value>>> {
        Box::pin(async { Ok(None) })
    }
    fn rename<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
    fn stop_task<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
    fn task_output<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<Value>> {
        Box::pin(async { Ok(Value::Null) })
    }
    fn remote_control(&self, _: Remote, _: Duration) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async { Ok(Value::Null) })
    }
    fn control(&self, _: Value, _: Duration) -> BoxFuture<'_, Result<Value>> {
        Box::pin(async { Ok(Value::Null) })
    }
    fn reply(&self, _: &str, _: std::result::Result<Value, String>) -> Result<()> {
        Ok(())
    }
}

/// An agent of `project` working in `cwd`, its process the recorder.
fn agent_in(project: &str, cwd: &Path, conv: &Path) -> (AgentRt, Arc<Recorder>) {
    let mut rt = AgentRt::new(
        AgentMeta {
            id: crate::model::new_id(),
            project_id: project.into(),
            cwd: cwd.to_string_lossy().into_owned(),
            mode: "default".into(),
            ..Default::default()
        },
        conv,
    );
    let rec = Arc::new(Recorder::default());
    rt.attach(rec.clone());
    (rt, rec)
}

fn ask(rt: &mut AgentRt, id: &str, tool: &str, input: Value) {
    rt.handle_frame(
        &json!({"type":"control_request","request_id":id,"request":{
            "subtype":"can_use_tool","tool_name":tool,"tool_use_id":format!("t-{id}"),"input":input}}),
        &mut Effects::default(),
    );
}

/// SR4-03: the Overseer reads its folder and the projects' without a question; a harness's credentials, a key, a folder
/// elsewhere or a link of its folder that leads out are refused on the spot; nothing of it waits for the user.
#[test]
fn the_overseer_reads_only_its_folder_and_the_projects_never_a_secret() {
    let dir = folder("agent");
    let own = dir.join("overseer");
    let project = dir.join("projet");
    let outside = dir.join("ailleurs");
    for d in [&own, &project, &outside] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(own.join("notes.md"), "x").unwrap();
    std::fs::write(project.join("README.md"), "x").unwrap();
    std::fs::write(outside.join("a.txt"), "x").unwrap();
    link(&outside, &own.join("lien"));
    let (mut rt, rec) = agent_in(crate::overseer::PROJECT, &own, &dir.join("conv"));
    rt.read_roots = vec![project.clone()];
    let home = dirs::home_dir().unwrap();
    let p = |p: PathBuf| p.to_string_lossy().into_owned();
    ask(
        &mut rt,
        "r1",
        "Read",
        json!({"file_path": p(home.join(".claude").join(".credentials.json"))}),
    );
    ask(
        &mut rt,
        "r2",
        "Grep",
        json!({"pattern": "BEGIN OPENSSH PRIVATE KEY", "path": p(home.join(".ssh"))}),
    );
    ask(
        &mut rt,
        "r3",
        "Read",
        json!({"file_path": p(home.join(".escouade-signing").join("escouade.key"))}),
    );
    ask(
        &mut rt,
        "r4",
        "Bash",
        json!({"command": "curl https://example.invalid"}),
    );
    ask(
        &mut rt,
        "r5",
        "Read",
        json!({"file_path": p(own.join("notes.md"))}),
    );
    ask(
        &mut rt,
        "r6",
        "Read",
        json!({"file_path": p(project.join("README.md"))}),
    );
    ask(
        &mut rt,
        "r7",
        "Read",
        json!({"file_path": p(outside.join("a.txt"))}),
    );
    ask(
        &mut rt,
        "r8",
        "Read",
        json!({"file_path": p(own.join("lien").join("a.txt"))}),
    );
    ask(&mut rt, "r9", "Glob", json!({"pattern": "**/*.md"}));
    ask(&mut rt, "r10", "mcp__escouade__get_board", json!({}));
    ask(
        &mut rt,
        "r11",
        "Read",
        json!({"file_path": "C:/Users/someone/.claude/.credentials.json"}),
    );
    let got = rec.0.lock().unwrap().clone();
    let decision = |id: &str| got.iter().find(|g| g.0 == id).map(|g| g.1.clone());
    for id in ["r1", "r2", "r3", "r4", "r7", "r8", "r11"] {
        assert_eq!(decision(id).as_deref(), Some("deny"), "{id}: {got:?}");
    }
    for id in ["r5", "r6", "r9", "r10"] {
        assert_eq!(decision(id).as_deref(), Some("allow"), "{id}: {got:?}");
    }
    assert_ne!(
        rt.meta.status,
        crate::model::AgentStatus::Waiting,
        "nothing waits for the user"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Another agent's same read is not answered by Escouade: it waits for the user.
#[test]
fn another_agent_waits_for_the_user_for_the_same_read() {
    let dir = folder("ordinary");
    let (mut rt, rec) = agent_in("p-ordinaire", &dir.join("w"), &dir.join("conv"));
    let home = dirs::home_dir().unwrap();
    ask(
        &mut rt,
        "r1",
        "Read",
        json!({"file_path": home.join(".claude").join(".credentials.json").to_string_lossy()}),
    );
    assert!(
        rec.0.lock().unwrap().is_empty(),
        "nothing answered by itself"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
