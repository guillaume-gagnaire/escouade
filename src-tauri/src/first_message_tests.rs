//! ESC-156: the worktree of a new agent made at its first message, under the name the agent is
//! given (branch and folder), « Créer le worktree au premier message, au nom de l’agent ».

use crate::core::AgentOptions;
use crate::core_tests::{git, harness, wt_step, Harness};
use crate::model::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// A setup step that writes, in the worktree, the folder it runs in and its branch.
const PREPARED: &str = r#"node -e "require('fs').writeFileSync('prepared.txt', require('path').basename(process.cwd()) + ' ' + process.env.ESCOUADE_BRANCH)""#;

fn branches(r: &Path) -> Vec<String> {
    git(r, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(str::to_string)
        .collect()
}

/// The folders of the project's worktrees folder.
fn folders(p: &Project) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(p.worktrees_root())
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The steps the window was told of the agent's setup, in order, each once.
fn steps_told(h: &Harness, id: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for e in h.events.lock().iter() {
        if e["type"] != "agent" || e["agent"]["id"] != id {
            continue;
        }
        if let Some(s) = e["agent"]["setup"].as_str() {
            if out.last().map(String::as_str) != Some(s) {
                out.push(s.to_string());
            }
        }
    }
    out
}

/// A project as a new one is (« Créer le worktree au premier message » on), a worktree per agent,
/// its setup writing `prepared.txt`.
async fn lazy_project(h: &Harness) -> (Project, PathBuf) {
    let (p, r) = h.project_as_new(true).await;
    assert!(p.worktree_at_first_message, "on for a new project");
    h.set_worktree_steps(&p.id, vec![wt_step(PREPARED, "")], vec![]);
    (h.core.project(&p.id).unwrap(), r)
}

#[tokio::test]
async fn a_new_agent_gets_its_branch_and_its_folder_at_its_first_message_under_its_name() {
    let h = harness("esc156-first");
    let (p, r) = lazy_project(&h).await;
    let before = branches(&r);
    let a = h.core.create_agent(&p.id, None).await.unwrap();
    let id = a.meta.id.clone();
    // Nothing yet: no worktree, no branch, no process, its folder the project's.
    assert!(a.meta.worktree.is_none());
    assert!(a.meta.worktree_pending);
    assert_eq!(a.meta.name, "agent-1");
    assert_eq!(a.meta.cwd, p.path);
    assert_eq!(branches(&r), before);
    assert!(folders(&p).is_empty(), "{:?}", folders(&p));
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!h.alive(&id), "no process before the first message");
    h.turn(&id, "Ajoute un bouton").await;
    // Named by the fake Haiku first, then its branch and its folder under that name.
    let m = h.agent(&id);
    assert_eq!((m.name.as_str(), m.named), ("ajoute-un-fake", true));
    assert!(!m.worktree_pending);
    let wt = m.worktree.clone().unwrap();
    let dir = p.worktrees_root().join("ajoute-un-fake");
    assert_eq!(PathBuf::from(&wt.path), dir);
    assert_eq!(wt.branch, "escouade/ajoute-un-fake");
    assert_eq!(wt.base_branch, "main");
    assert_eq!(current_branch(&dir), "escouade/ajoute-un-fake");
    assert_eq!(m.cwd, wt.path);
    // Its setup ran in that folder, on that branch, before its turn.
    assert_eq!(
        std::fs::read_to_string(dir.join("prepared.txt")).unwrap(),
        "ajoute-un-fake escouade/ajoute-un-fake"
    );
    // Its harness ran there.
    assert!(!h.launches(&dir).is_empty());
    // Its first message went once it was ready, after the setup said it was over.
    let items = h.items(&id);
    let ready = items.iter().position(|i| {
        i["kind"] == "notice"
            && i["text"]
                .as_str()
                .unwrap()
                .starts_with("Worktree préparé (1 commande")
    });
    let user = items.iter().position(|i| i["kind"] == "user");
    assert!(ready.is_some() && ready < user, "{items:?}");
    // The conversation showed the steps: the name, the worktree, the setup.
    let steps = steps_told(&h, &id);
    assert_eq!(
        steps,
        [
            "Nom de l’agent".to_string(),
            "Création du worktree ajoute-un-fake".to_string(),
            format!("1/1 · {}", crate::worktrees::label(&wt_step(PREPARED, ""))),
        ],
    );
    // Its session took its name.
    h.wait("its session renamed", |h| {
        renames(h, &dir)
            .iter()
            .any(|r| r["title"] == "ajoute-un-fake")
    })
    .await;
    h.core.stop_for_good();
}

/// The titles the fake `claude` started in `cwd` was given (`rename_session`).
fn renames(_h: &Harness, cwd: &Path) -> Vec<serde_json::Value> {
    let key: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let file = std::env::temp_dir().join(format!("fake-claude-{key}.rename.jsonl"));
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn current_branch(dir: &Path) -> String {
    git(dir, &["branch", "--show-current"])
}

#[tokio::test]
async fn a_name_whose_branch_or_folder_is_taken_goes_on_with_a_number() {
    let h = harness("esc156-taken");
    let (p, r) = lazy_project(&h).await;
    // Its branch is somebody else's, and another repository's worktree has its folder.
    git(&r, &["branch", "escouade/ajoute-un-fake"]);
    std::fs::create_dir_all(p.worktrees_root().join("ajoute-un-fake-2")).unwrap();
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Ajoute un bouton").await;
    let wt = h.agent(&id).worktree.unwrap();
    assert_eq!(
        PathBuf::from(&wt.path),
        p.worktrees_root().join("ajoute-un-fake-3")
    );
    assert_eq!(wt.branch, "escouade/ajoute-un-fake-3");
    // Two agents named alike: the second gets the next number, its name too.
    let other = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&other, "Ajoute un bouton").await;
    let m = h.agent(&other);
    assert_eq!(m.name, "ajoute-un-fake-2");
    assert_eq!(
        PathBuf::from(&m.worktree.unwrap().path),
        p.worktrees_root().join("ajoute-un-fake-2-2")
    );
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_name_too_late_leaves_agent_n_without_holding_up_the_turn_and_renames_its_branch_after() {
    let h = harness("esc156-late");
    let (p, _) = lazy_project(&h).await;
    h.core.naming_ms.store(500, Ordering::Release);
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // The fake Haiku answers 3 s late: the worktree did not wait for it.
    h.turn(&id, "Ajoute un bouton [lent]").await;
    let wt = h.agent(&id).worktree.unwrap();
    assert_eq!(PathBuf::from(&wt.path), p.worktrees_root().join("agent-1"));
    // The name, come later, renames the branch as it does today; the folder stays.
    h.wait("named", |h| h.agent(&id).named).await;
    let m = h.agent(&id);
    assert_eq!(m.name, "ajoute-un-fake");
    let wt = m.worktree.unwrap();
    assert_eq!(wt.branch, "escouade/ajoute-un-fake");
    assert_eq!(PathBuf::from(&wt.path), p.worktrees_root().join("agent-1"));
    assert_eq!(
        current_branch(Path::new(&wt.path)),
        "escouade/ajoute-un-fake"
    );
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_name_that_fails_leaves_agent_n() {
    let h = harness("esc156-fails");
    let (p, _) = lazy_project(&h).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "Ajoute un bouton [muet]").await;
    let m = h.agent(&id);
    assert_eq!((m.name.as_str(), m.named), ("agent-1", false));
    let wt = m.worktree.unwrap();
    assert_eq!(PathBuf::from(&wt.path), p.worktrees_root().join("agent-1"));
    assert_eq!(wt.branch, "escouade/agent-1");
    assert!(Path::new(&wt.path).join("prepared.txt").exists());
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_message_with_nothing_to_name_by_makes_the_worktree_as_agent_n() {
    let h = harness("esc156-command");
    let (p, _) = lazy_project(&h).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&id, "/clear").await;
    let m = h.agent(&id);
    assert_eq!(m.name, "agent-1");
    assert_eq!(
        PathBuf::from(&m.worktree.unwrap().path),
        p.worktrees_root().join("agent-1")
    );
    h.core.stop_for_good();
}

#[tokio::test]
async fn an_agent_renamed_before_its_first_message_gets_its_worktree_under_that_name() {
    let h = harness("esc156-renamed");
    let (p, _) = lazy_project(&h).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.rename_agent(&id, "Mon agent").await.unwrap();
    assert!(h.agent(&id).worktree_pending);
    h.turn(&id, "Ajoute un bouton").await;
    let m = h.agent(&id);
    assert_eq!(m.name, "Mon agent");
    let wt = m.worktree.unwrap();
    assert_eq!(
        PathBuf::from(&wt.path),
        p.worktrees_root().join("mon-agent")
    );
    assert_eq!(wt.branch, "escouade/mon-agent");
    h.core.stop_for_good();
}

#[tokio::test]
async fn a_start_before_the_first_message_makes_the_worktree_under_the_name_it_has() {
    let h = harness("esc156-start");
    let (p, _) = lazy_project(&h).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Started by something else than a message (Remote Control, a view): never in the
    // project's folder.
    h.core.ensure_process(&id).await.unwrap();
    let m = h.agent(&id);
    assert!(!m.worktree_pending);
    let wt = m.worktree.unwrap();
    assert_eq!(PathBuf::from(&wt.path), p.worktrees_root().join("agent-1"));
    assert_eq!(m.cwd, wt.path);
    assert!(Path::new(&wt.path).join("prepared.txt").exists());
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_agents_named_or_placed_at_their_making_get_their_worktree_at_once() {
    let h = harness("esc156-exceptions");
    let (p, r) = lazy_project(&h).await;
    let made = |o: AgentOptions| {
        let (core, pid) = (h.core.clone(), p.id.clone());
        async move { core.create_agent_with(&pid, o).await.unwrap().meta }
    };
    // Named at its making (by the window or `create_agent` with a name).
    let named = made(AgentOptions {
        name: Some("revue".into()),
        ..Default::default()
    })
    .await;
    assert!(!named.worktree_pending);
    assert_eq!(named.worktree.unwrap().branch, "escouade/revue");
    // On a new branch, and on one that exists.
    let typed = h
        .core
        .create_agent_on_new_branch(&p.id, "feat/saisie", None)
        .await
        .unwrap()
        .meta;
    assert_eq!(typed.worktree.unwrap().branch, "feat/saisie");
    git(&r, &["branch", "feat/la"]);
    let on = h
        .core
        .create_agent_on_branch(&p.id, "feat/la", None)
        .await
        .unwrap()
        .meta;
    assert_eq!(on.worktree.unwrap().branch, "feat/la");
    // A ticket's (its branch given).
    let ticket = made(AgentOptions {
        worktree: Some(("ticket/ESC-1".into(), "main".into())),
        ..Default::default()
    })
    .await;
    assert_eq!(ticket.worktree.unwrap().branch, "ticket/ESC-1");
    // Without a worktree.
    let plain = made(AgentOptions {
        isolated: Some(false),
        ..Default::default()
    })
    .await;
    assert!(plain.worktree.is_none() && !plain.worktree_pending);
    // A copy of an agent with a worktree.
    let original = made(AgentOptions {
        name: Some("source".into()),
        ..Default::default()
    })
    .await;
    let copy = h.core.duplicate_agent(&original.id).await.unwrap().meta;
    assert!(!copy.worktree_pending);
    assert!(copy.worktree.is_some());
    // In the Claude Code view: its interface runs, and is typed in, from the start.
    h.core.settings.write().terminal_view = true;
    let viewed = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert!(viewed.terminal);
    assert!(!viewed.worktree_pending);
    assert!(viewed.worktree.is_some());
    h.core.stop_for_good();
}

#[tokio::test]
async fn the_setting_off_gives_the_worktree_at_once_as_before() {
    let h = harness("esc156-off");
    let (p, _) = lazy_project(&h).await;
    h.core
        .update_project(Project {
            worktree_at_first_message: false,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    assert!(!h.core.project(&p.id).unwrap().worktree_at_first_message);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta;
    assert!(!a.worktree_pending);
    let wt = a.worktree.unwrap();
    assert_eq!(PathBuf::from(&wt.path), p.worktrees_root().join("agent-1"));
    h.turn(&a.id, "Ajoute un bouton").await;
    h.wait("named", |h| h.agent(&a.id).named).await;
    // The branch renamed after it, the folder kept, as before.
    let wt = h.agent(&a.id).worktree.unwrap();
    assert_eq!(wt.branch, "escouade/ajoute-un-fake");
    assert_eq!(PathBuf::from(&wt.path), p.worktrees_root().join("agent-1"));
    h.core.stop_for_good();
}

#[test]
fn the_projects_saved_before_have_it_on_and_the_agents_have_no_pending_worktree() {
    let project: Project = serde_json::from_value(serde_json::json!({
        "id": "p1", "name": "demo", "path": "C:/demo", "color": "", "worktreePerAgent": true,
    }))
    .unwrap();
    assert!(project.worktree_at_first_message);
    // Turned off, it stays off.
    let off: Project = serde_json::from_value(serde_json::json!({
        "id": "p1", "name": "demo", "path": "C:/demo", "color": "",
        "worktreeAtFirstMessage": false,
    }))
    .unwrap();
    assert!(!off.worktree_at_first_message);
    let agent: AgentMeta = serde_json::from_value(serde_json::json!({ "id": "a1" })).unwrap();
    assert!(!agent.worktree_pending);
    // Written only while it waits.
    let v = serde_json::to_value(&agent).unwrap();
    assert!(v.get("worktreePending").is_none());
}

#[tokio::test]
async fn a_pending_worktree_waits_again_after_a_restart_of_the_app() {
    let h = harness("esc156-saved");
    let (p, _) = lazy_project(&h).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.save_state().unwrap();
    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(h.core.data.state_file()).unwrap()).unwrap();
    let agent = state["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id.as_str())
        .unwrap();
    assert_eq!(agent["worktreePending"], true);
    assert!(agent.get("worktree").is_none_or(|w| w.is_null()));
    assert_eq!(state["projects"][0]["worktreeAtFirstMessage"], true);
    h.core.stop_for_good();
}

/// ESC-160: the branch chosen at an agent's making is kept until its first message, its worktree
/// made from it then; the others start from the project's.
#[tokio::test]
async fn the_branch_chosen_at_its_making_is_where_its_worktree_starts_at_its_first_message() {
    let h = harness("esc160-first");
    let (p, r) = lazy_project(&h).await;
    for (branch, file) in [("release", "release.txt"), ("hotfix", "hotfix.txt")] {
        git(&r, &["switch", "-qc", branch, "main"]);
        std::fs::write(r.join(file), "x\n").unwrap();
        git(&r, &["add", "-A"]);
        git(&r, &["commit", "-qm", file]);
        git(&r, &["switch", "-q", "main"]);
    }
    let mut q = h.core.project(&p.id).unwrap();
    q.worktree_base = WorktreeBase::Branch("release".into());
    h.core.update_project(q).unwrap();
    let a = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                base: Some("hotfix".into()),
                select: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let id = a.meta.id.clone();
    assert!(a.meta.worktree_pending);
    assert_eq!(a.meta.worktree_from.as_deref(), Some("hotfix"));
    // Saved with the agent: kept through a restart of the app.
    let saved = serde_json::to_value(&a.meta).unwrap();
    assert_eq!(saved["worktreeFrom"], "hotfix");
    h.turn(&id, "Corrige le bug").await;
    let m = h.agent(&id);
    assert!(!m.worktree_pending);
    assert_eq!(m.worktree_from, None);
    let wt = m.worktree.clone().unwrap();
    assert_eq!(wt.base_branch, "hotfix");
    assert_eq!(
        git(Path::new(&wt.path), &["rev-parse", "HEAD"]),
        git(&r, &["rev-parse", "hotfix"])
    );
    // Another, with nothing chosen: from the project's.
    let b = h.core.create_agent(&p.id, None).await.unwrap();
    assert_eq!(b.meta.worktree_from, None);
    h.turn(&b.meta.id, "Ajoute un test").await;
    let wt = h.agent(&b.meta.id).worktree.unwrap();
    assert_eq!(wt.base_branch, "release");
    assert!(Path::new(&wt.path).join("release.txt").exists());
}
