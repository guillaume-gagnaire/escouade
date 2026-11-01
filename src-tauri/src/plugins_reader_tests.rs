//! Tests of the reader of workflows a plugin brings (ESC-200, choice 158; `docs/PLUGINS.md` §4.5)
//! against the fake plugin: the files it is given (its globs, nothing outside the project), the
//! graph it answers as the core follows it, what does not hold refused whole, and an agent that
//! follows the workflow of a reader, its `workflow.*` facts as for any other.

use super::plugins_runtime_tests::{fake, quick};
use crate::facts::Body;
use crate::plugins::adapters::is_down;
use crate::plugins::reader::{files_of, id_of, with_read, ProcessWorkflowReader};
use crate::workflow::{Scan, Workflow};
use crate::workflow_track::{AgentTrack, Fact};
use serde_json::json;

const ID: &str = super::plugins_runtime_tests::ID;

fn project() -> std::path::PathBuf {
    let dir = crate::paths::test_dir("reader-project");
    std::fs::create_dir_all(dir.join("_bmad/stories")).unwrap();
    std::fs::write(dir.join("_bmad/brief.md"), "# Brief\n").unwrap();
    std::fs::write(dir.join("_bmad/stories/one.md"), "# One\n").unwrap();
    std::fs::write(dir.join("README.md"), "# Not asked\n").unwrap();
    std::fs::write(dir.join("_bmad/blob.bin"), [0xff, 0xfe, 0x00, 0x9f]).unwrap();
    dir
}

fn globs(l: &[&str]) -> Vec<String> {
    l.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_reader_is_given_the_files_it_declared_and_nothing_else() {
    let root = project();
    let files = files_of(
        &root,
        &[],
        &globs(&["_bmad/**/*", "../*.md", "/etc/*", "C:/x/*"]),
    );
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    // Text only, in the order of their paths, with `/`; a binary file, a README that was not asked
    // for, and every pattern that leaves the project, find nothing.
    assert_eq!(paths, ["_bmad/brief.md", "_bmad/stories/one.md"]);
    assert_eq!(files[0].text, "# Brief\n");

    // A project of several repositories: the same patterns, from each repository's folder.
    let multi = crate::paths::test_dir("reader-multi");
    std::fs::create_dir_all(multi.join("api/_bmad")).unwrap();
    std::fs::write(multi.join("api/_bmad/plan.md"), "plan").unwrap();
    let files = files_of(&multi, &globs(&["api"]), &globs(&["_bmad/*.md"]));
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "api/_bmad/plan.md");
}

#[test]
fn a_reader_is_never_given_git_s_files_nor_what_a_link_leads_to() {
    let root = crate::paths::test_dir("reader-links");
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(
        root.join(".git/config"),
        "[remote]\nurl = https://me:token@host/x\n",
    )
    .unwrap();
    std::fs::write(root.join("plan.md"), "plan").unwrap();
    let outside = crate::paths::test_dir("reader-outside");
    std::fs::write(outside.join("id_ed25519"), "secret").unwrap();
    // A link to a file and to a folder outside the project. Making one may need rights the
    // machine does not give (Windows without the developer mode): then only `.git` is tried.
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(outside.join("id_ed25519"), root.join("key.md"))
        .is_ok()
        && std::os::unix::fs::symlink(&outside, root.join("away")).is_ok();
    #[cfg(windows)]
    let linked =
        std::os::windows::fs::symlink_file(outside.join("id_ed25519"), root.join("key.md")).is_ok()
            && std::os::windows::fs::symlink_dir(&outside, root.join("away")).is_ok();
    let files = files_of(&root, &[], &globs(&["**/*", "*", "away/*"]));
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["plan.md"], "linked: {linked}");
    assert!(files.iter().all(|f| f.text != "secret"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_graph_a_reader_answers_is_a_workflow_the_core_follows() {
    let f = fake("reader", &[], quick());
    let root = project();
    let reader = ProcessWorkflowReader::new(f.rt.clone(), ID, "bmad", &globs(&["_bmad/*.md"]));
    let found = reader.read(&root, &[]).await.unwrap();
    assert_eq!(found.len(), 1);
    let w: &Workflow = &found[0];
    assert_eq!(w.id, id_of(ID, "bmad"));
    assert_eq!(w.name, "BMAD");
    let steps: Vec<(&str, usize)> = w.steps.iter().map(|s| (s.name.as_str(), s.order)).collect();
    assert_eq!(steps, [("brief", 0), ("build", 1), ("review", 2)]);
    assert_eq!(
        w.steps[0].description, "_bmad/brief.md",
        "it was given its files"
    );
    assert_eq!(w.steps[1].agents[0].name, "dev");
    assert_eq!(w.steps[0].writes, ["docs/brief.md"]);
    let back = w.transitions.iter().find(|t| t.back).unwrap();
    assert_eq!(
        (back.from.as_str(), back.to.as_str(), back.max),
        ("review", "build", Some(3))
    );
    assert_eq!(w.exit.as_deref(), Some("the review is clean"));

    // It joins a reading without taking the place of what the app read, and is replaced by its own
    // next reading rather than added twice.
    let mut scan = Scan::default();
    with_read(&mut scan, found.clone());
    with_read(&mut scan, found);
    assert_eq!(scan.workflows.len(), 1);

    // The call: the files and the project are said, nothing else of the machine.
    let calls = f.rt.call(ID, "test/calls", json!({})).await.unwrap();
    let last = calls.as_array().unwrap().last().unwrap();
    assert_eq!(last["method"], "workflow/read");
    assert_eq!(last["params"]["reader"], "bmad");
    assert_eq!(last["params"]["files"].as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_graph_that_does_not_hold_is_refused_whole_and_with_why() {
    let f = fake("reader-bad", &[], quick());
    let root = project();
    let e = ProcessWorkflowReader::new(f.rt.clone(), ID, "broken", &[])
        .read(&root, &[])
        .await
        .unwrap_err();
    let said = format!("{e:#}");
    assert!(said.contains("bad") && said.contains("nowhere"), "{said}");
    let e = ProcessWorkflowReader::new(f.rt.clone(), ID, "many", &[])
        .read(&root, &[])
        .await
        .unwrap_err();
    assert!(format!("{e:#}").contains("50"), "{e:#}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reader_that_crashes_is_down_not_a_graph() {
    let limits = crate::plugins::runtime::Limits {
        restarts: 0,
        ..quick()
    };
    let f = fake("reader-crash", &[], limits);
    let root = project();
    // `crash` makes the fake plugin end on the call.
    let e = ProcessWorkflowReader::new(f.rt.clone(), ID, "crash", &[])
        .read(&root, &[])
        .await
        .unwrap_err();
    assert!(is_down(&e), "{e:#}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_in_the_workflow_of_a_reader_says_its_way_as_facts() {
    let f = fake("reader-track", &[], quick());
    let root = project();
    let found = ProcessWorkflowReader::new(f.rt.clone(), ID, "bmad", &[])
        .read(&root, &[])
        .await
        .unwrap();
    let mut scan = Scan::default();
    with_read(&mut scan, found);
    let mut track = AgentTrack::new("a1", crate::model::EngineId::CLAUDE, false);
    let mut at = 1_000;
    let mut say = |track: &mut AgentTrack, text: &str| {
        at += 1000;
        track.apply(&scan, "", &Fact::Message { text: text.into() }, at)
    };
    let mark = track.mark();
    assert!(say(&mut track, "/bmad:brief"));
    say(&mut track, "/bmad:build");
    say(&mut track, "/bmad:review");
    track.apply(
        &scan,
        "",
        &Fact::TurnEnd {
            end: crate::workflow_track::End::Finished,
            text: "3 gaps".into(),
        },
        2_000_000,
    );
    say(&mut track, "/bmad:build");
    let said = track.facts_since(mark);
    let names: Vec<String> = said.iter().map(Body::name).collect();
    assert_eq!(
        names,
        [
            "workflow.step_entered",
            "workflow.step_left",
            "workflow.step_entered",
            "workflow.step_left",
            "workflow.step_entered",
            "workflow.step_left",
            "workflow.looped_back",
            "workflow.step_entered",
        ]
    );
    match &said[6] {
        Body::WorkflowLoopedBack {
            workflow,
            from,
            to,
            reason,
        } => {
            assert_eq!(workflow, &id_of(ID, "bmad"));
            assert_eq!((from.as_str(), to.as_str()), ("review", "build"));
            assert_eq!(reason.as_deref(), Some("3 gaps"));
        }
        other => panic!("{other:?}"),
    }
}
