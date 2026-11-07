//! Tests of the files of the workflows (ESC-195): their name, their text written the same way
//! every time and read back, nothing of one machine in them, what a reading does to the folder
//! (adds, updates, never removes, never makes again a workflow set apart), a file that cannot be
//! read, and in the app: the files of a teammate followed without the sources, a file changed by
//! hand that reaches the tracking, « Où garder » that moves them, and no commit ever.

use crate::paths::test_dir;
use crate::workflow::{read as read_workflows, Roots, Workflow};
use crate::workflowfiles as wf;
use crate::workflowfix::{Fix, FixStep};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/workflows")
        .join(name)
}

fn gsd() -> Workflow {
    let home = fixture("gsd").join("home");
    let roots = Roots {
        claude: Some(home.join(".claude")),
        codex: Some(home.join(".codex")),
        home,
    };
    read_workflows(&fixture("gsd/project"), &[], &roots, &[])
        .workflows
        .remove(0)
}

/// The repository's `/dev` (tests/fixtures/workflows/dev).
fn dev() -> Workflow {
    read_workflows(&fixture("dev/project"), &[], &Roots::default(), &[])
        .workflows
        .remove(0)
}

fn renamed(w: &Workflow, step: usize, name: &str) -> Fix {
    Fix {
        workflow: w.id.clone(),
        steps: w
            .steps
            .iter()
            .enumerate()
            .map(|(i, s)| FixStep {
                name: if i == step {
                    name.into()
                } else {
                    s.name.clone()
                },
                from: vec![s.name.clone()],
                signals: Vec::new(),
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn a_file_name_is_its_id_with_nothing_a_system_refuses() {
    assert_eq!(wf::safe_name("|claude|gsd"), "claude-gsd");
    assert_eq!(wf::safe_name("api|claude|gsd"), "api-claude-gsd");
    assert_eq!(
        wf::safe_name("web/app|any|prompt-md"),
        "web-app-any-prompt-md"
    );
    assert_eq!(wf::safe_name("C:\\x:<y>*?\"|"), "c-x-y");
    assert_eq!(wf::safe_name("|||"), "workflow");
    assert_eq!(wf::safe_name("con"), "con-workflow");
    for c in ['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '] {
        assert!(!wf::safe_name("a|b:c d/e").contains(c));
    }
}

#[test]
fn a_file_is_written_the_same_way_every_time_and_read_back_with_its_fixes() {
    let w = gsd();
    let doc = wf::Doc {
        workflow: w.clone(),
        fixes: renamed(&w, 1, "Planifier"),
    };
    let text = wf::write(&doc);
    assert_eq!(text, wf::write(&doc.clone()), "deterministic");
    assert!(text.contains("commit it with your work, Escouade never does"));
    assert!(text.contains("\nworkflow:\n  id: "), "{text}");
    assert!(text.contains("\nfixes:\n"), "{text}");
    let back = wf::read(&text).unwrap();
    assert_eq!(wf::portable(&back.workflow), wf::portable(&w), "{text}");
    assert_eq!(back.fixes, doc.fixes);
    // Without corrections, no `fixes`.
    let plain = wf::write(&wf::Doc {
        workflow: w.clone(),
        fixes: Fix::default(),
    });
    assert!(!plain.contains("fixes:"));
}

#[test]
fn nothing_of_one_machine_is_in_a_file() {
    let mut w = gsd();
    // As this machine reads it: a state file that says where the work is, a path out of every
    // folder it knows.
    w.steps[0].path = "C:\\Users\\me\\dev\\x\\discuss.md".into();
    w.steps[0].writes.push("/home/me/x/CONTEXT.md".into());
    let text = wf::write(&wf::Doc {
        workflow: w,
        fixes: Fix::default(),
    });
    for bad in [
        "C:\\", "C:/", "/home/", "readAt", "exists", "says", "modified", "2026-",
    ] {
        assert!(!text.contains(bad), "{bad} in\n{text}");
    }
    assert!(text.contains("path: discuss.md"), "{text}");
    assert!(
        text.contains("~/.claude/commands/gsd/plan-phase.md"),
        "{text}"
    );
}

#[test]
fn a_reading_adds_and_updates_files_but_never_removes_nor_makes_again_one_set_apart() {
    let dir = test_dir("wf-files-merge").join(".escouade/workflows");
    let w = gsd();
    // Added.
    let changed = wf::merge_reading(&dir, &wf::load(&dir), std::slice::from_ref(&w));
    assert_eq!(changed, [(w.id.clone(), String::new())]);
    let file = dir.join("claude-gsd.yml");
    let first = std::fs::read_to_string(&file).unwrap();
    // The same reading again touches nothing.
    let before = std::fs::metadata(&file).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    assert!(wf::merge_reading(&dir, &wf::load(&dir), std::slice::from_ref(&w)).is_empty());
    assert_eq!(
        std::fs::metadata(&file).unwrap().modified().unwrap(),
        before
    );
    // Corrected, then read again changed: updated, its corrections kept.
    wf::set_fixes(&wf::load(&dir), &[renamed(&w, 0, "Discuter")]).unwrap();
    let mut changed_w = w.clone();
    changed_w.steps.pop();
    let changed = wf::merge_reading(&dir, &wf::load(&dir), std::slice::from_ref(&changed_w));
    assert_eq!(changed.len(), 1);
    assert!(changed[0].1.contains("gsd:verify-work"), "{changed:?}");
    let folder = wf::load(&dir);
    assert_eq!(folder.docs.len(), 1);
    assert_eq!(folder.docs[0].1.workflow.steps.len(), 3);
    assert_eq!(folder.docs[0].1.fixes.steps[0].name, "Discuter");
    assert_ne!(std::fs::read_to_string(&file).unwrap(), first);
    // A reading that does not find it: the file stays.
    assert!(wf::merge_reading(&dir, &folder, &[]).is_empty());
    assert!(file.is_file());
    // Set apart: kept marked, never made again by a reading.
    let mut apart = folder.fixes();
    apart[0].dismissed = true;
    wf::set_fixes(&folder, &apart).unwrap();
    wf::merge_reading(&dir, &wf::load(&dir), std::slice::from_ref(&w));
    let folder = wf::load(&dir);
    assert_eq!(folder.docs.len(), 1);
    assert!(folder.docs[0].1.fixes.dismissed);
}

#[test]
fn a_file_that_cannot_be_read_is_told_and_left_as_it_is() {
    let dir = test_dir("wf-files-bad").join(".escouade/workflows");
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("claude-gsd.yml");
    std::fs::write(&bad, "version: 1\nworkflow: [oops\n").unwrap();
    std::fs::write(dir.join("other.yml"), "version: 9\nworkflow: {}\n").unwrap();
    std::fs::write(dir.join("notes.txt"), "not a workflow").unwrap();
    let folder = wf::load(&dir);
    assert!(folder.docs.is_empty());
    assert_eq!(folder.unreadable.len(), 2, "{:?}", folder.unreadable);
    assert!(folder
        .unreadable
        .iter()
        .any(|(p, why)| p.ends_with("other.yml") && why.contains("9")));
    // A reading that finds its workflow leaves the broken file for the user to mend.
    assert!(wf::merge_reading(&dir, &folder, &[gsd()]).is_empty());
    assert_eq!(
        std::fs::read_to_string(&bad).unwrap(),
        "version: 1\nworkflow: [oops\n"
    );
}

#[test]
fn moved_from_one_place_to_the_other_the_files_are_neither_lost_nor_doubled() {
    let root = test_dir("wf-files-move");
    let (a, b) = (root.join("repo"), root.join("data"));
    let w = dev();
    wf::merge_reading(&a, &wf::load(&a), std::slice::from_ref(&w));
    wf::set_fixes(&wf::load(&a), &[renamed(&w, 0, "Discuter")]).unwrap();
    wf::move_all(&a, &b).unwrap();
    assert!(wf::load(&a).docs.is_empty());
    let moved = wf::load(&b);
    assert_eq!(moved.docs.len(), 1);
    assert_eq!(moved.fixes()[0].steps[0].name, "Discuter");
    // Back, over a copy already there: one file still.
    wf::merge_reading(&a, &wf::load(&a), std::slice::from_ref(&w));
    wf::move_all(&b, &a).unwrap();
    let back = wf::load(&a);
    assert_eq!(back.docs.len(), 1);
    assert_eq!(back.fixes()[0].steps[0].name, "Discuter");
}

/// The schema the repository publishes (`schemas/escouade-workflow.schema.json`, served by the
/// site) is the one of the files, and a file written follows it: `ESCOUADE_WRITE_SCHEMA=1`
/// writes it again.
#[test]
fn the_published_schema_is_up_to_date() {
    let schema = wf::schema();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let made = serde_json::to_string_pretty(&schema).unwrap() + "\n";
    let files = [
        root.join("schemas/escouade-workflow.schema.json"),
        root.join("website/public/schemas/escouade-workflow.schema.json"),
    ];
    if std::env::var_os("ESCOUADE_WRITE_SCHEMA").is_some() {
        for f in &files {
            std::fs::write(f, &made).unwrap();
        }
    }
    for f in &files {
        let published: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(f).unwrap_or_default())
                .unwrap_or_default();
        assert_eq!(published, schema, "{} is not up to date", f.display());
    }
    // The keys a file has are those the schema knows.
    let text = wf::write(&wf::Doc {
        workflow: gsd(),
        fixes: Fix::default(),
    });
    let v = crate::projectfile::tree(&text).unwrap();
    let crate::projectfile::N::Map(top) = &v.v else {
        panic!()
    };
    for (k, _, _) in top {
        assert!(schema["properties"][k].is_object(), "{k}");
    }
}

// ---------- in the app ----------

fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A project whose own commands make a workflow of two steps.
async fn two_steps(h: &crate::core_tests::Harness) -> (crate::model::Project, PathBuf) {
    let (p, dir) = h.project(false).await;
    let cmds = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmds).unwrap();
    std::fs::write(
        cmds.join("one.md"),
        "Do one with subagent_type=\"worker\", then /two.\n",
    )
    .unwrap();
    std::fs::write(cmds.join("two.md"), "Do two. Back to /one when red.\n").unwrap();
    (p, dir)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_reading_writes_the_files_in_the_project_and_never_commits_them() {
    let h = crate::core_tests::harness("wf-files-core");
    let (p, dir) = two_steps(&h).await;
    let commits = git(&dir, &["rev-list", "--count", "HEAD"]);
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(scan.workflows.len(), 1);
    let file = dir.join(".escouade/workflows/claude-one.yml");
    assert!(file.is_file(), "{:?}", scan.files);
    assert_eq!(scan.files[0].path, ".escouade/workflows/claude-one.yml");
    assert!(scan.files[0].local);
    // Read again unchanged: the file is not touched.
    let text = std::fs::read_to_string(&file).unwrap();
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
    // Corrected in the settings: into the file.
    let w = scan.workflows[0].clone();
    h.core
        .update_project(crate::model::Project {
            workflow_fixes: vec![renamed(&w, 0, "Premier")],
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    assert!(std::fs::read_to_string(&file)
        .unwrap()
        .contains("name: Premier"));
    // And read again, the corrections stay.
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(std::fs::read_to_string(&file)
        .unwrap()
        .contains("name: Premier"));
    assert_eq!(h.core.project(&p.id).unwrap().workflow_fixes.len(), 1);
    // Escouade committed nothing: the files wait for the user.
    assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]), commits);
    assert!(
        git(&dir, &["status", "--porcelain", "--untracked-files=all"])
            .contains(".escouade/workflows/claude-one.yml")
    );

    // « Sur cette machine »: the files go to the data folder, and come back.
    h.core
        .update_project(crate::model::Project {
            workflows_in_repo: false,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    assert!(!file.exists());
    let here = h
        .core
        .data
        .project_files(&p.id)
        .join("workflows/claude-one.yml");
    assert!(here.is_file());
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(scan.workflows.len(), 1);
    assert!(!dir
        .join(".escouade/workflows")
        .join("claude-one.yml")
        .exists());
    h.core
        .update_project(crate::model::Project {
            workflows_in_repo: true,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    assert!(file.is_file() && !here.exists());
    assert!(std::fs::read_to_string(&file)
        .unwrap()
        .contains("name: Premier"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_teammate_s_file_is_followed_without_the_sources_and_a_change_by_hand_reaches_the_tracking(
) {
    let h = crate::core_tests::harness("wf-files-mate");
    // The teammate's machine reads the workflow and writes its file.
    let (mate, mate_dir) = two_steps(&h).await;
    h.core.refresh_workflows(&mate.id, true).await.unwrap();
    let text =
        std::fs::read_to_string(mate_dir.join(".escouade/workflows/claude-one.yml")).unwrap();
    // This machine has none of the commands, only the file (a pull).
    let h = crate::core_tests::harness("wf-files-mine");
    let (p, dir) = h.project(false).await;
    std::fs::create_dir_all(dir.join(".escouade/workflows")).unwrap();
    std::fs::write(dir.join(".escouade/workflows/claude-one.yml"), &text).unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(scan.workflows.len(), 1);
    assert!(!scan.files[0].local, "the team's alone");
    let agent = crate::core_tests::agent_on(&h, &p, "principal").await;
    let ops = |text: &str| {
        vec![crate::model::ConvOp::Append {
            item: serde_json::json!({"kind": "user", "id": format!("u{text}"), "text": text}),
        }]
    };
    h.core
        .track_workflow(&agent.id, &p.id, &ops("/one"), 0.0, None);
    assert_eq!(
        h.core.workflow_tracks(&p.id)[0].track.step.as_deref(),
        Some("one")
    );
    // Read again, this machine never removes it.
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(dir.join(".escouade/workflows/claude-one.yml").is_file());

    // Changed by hand (or by a pull): read when the folder changes, the tracking follows at once.
    let w = scan.workflows[0].clone();
    let doc = wf::Doc {
        workflow: w.clone(),
        fixes: renamed(&w, 1, "Deux"),
    };
    std::fs::write(
        dir.join(".escouade/workflows/claude-one.yml"),
        wf::write(&doc),
    )
    .unwrap();
    h.core.sync_workflow_files(&p.id);
    assert_eq!(
        h.core.project(&p.id).unwrap().workflow_fixes[0].steps[1].name,
        "Deux"
    );
    h.core
        .track_workflow(&agent.id, &p.id, &ops("/two"), 0.0, None);
    assert_eq!(
        h.core.workflow_tracks(&p.id)[0].track.step.as_deref(),
        Some("Deux")
    );

    // A file broken by hand: told, the corrections of this machine kept.
    std::fs::write(dir.join(".escouade/workflows/broken.yml"), "version: [\n").unwrap();
    h.core.sync_workflow_files(&p.id);
    let scan = h.core.refresh_workflows(&p.id, false).await.unwrap();
    assert_eq!(scan.unreadable.len(), 1);
    assert_eq!(scan.unreadable[0].path, ".escouade/workflows/broken.yml");
    assert_eq!(h.core.project(&p.id).unwrap().workflow_fixes.len(), 1);

    // « Retirer du dépôt »: its file goes.
    let scan = h.core.remove_workflow_file(&p.id, &w.id).await.unwrap();
    assert!(scan.workflows.is_empty());
    assert!(!dir.join(".escouade/workflows/claude-one.yml").exists());
}

#[test]
fn only_the_workflows_of_the_repository_go_in_it_those_of_the_user_s_folders_stay_on_this_machine()
{
    let root = test_dir("wf-files-places");
    let places = wf::Places {
        repo: root.join("repo/.escouade/workflows"),
        local: root.join("data/workflows"),
    };
    let home = gsd();
    assert!(!wf::in_repo(&home), "GSD is of ~/.claude");
    let dev = read_workflows(&fixture("dev/project"), &[], &Roots::default(), &[])
        .workflows
        .remove(0);
    assert!(wf::in_repo(&dev));
    let written = wf::merge_reading_all(
        &places,
        &wf::load_all(&places),
        &[home.clone(), dev.clone()],
        true,
        &mut wf::Removals::default(),
    );
    assert_eq!(written.len(), 2);
    let repo = wf::load(&places.repo);
    let local = wf::load(&places.local);
    assert_eq!(repo.docs.len(), 1);
    assert_eq!(repo.docs[0].1.workflow.id, dev.id);
    assert_eq!(local.docs.len(), 1);
    assert_eq!(local.docs[0].1.workflow.id, home.id);
    // Both read as one; the repository's moved to this machine and back, the user's staying.
    assert_eq!(wf::load_all(&places).docs.len(), 2);
    wf::move_all(&places.repo, &places.local).unwrap();
    assert_eq!(wf::load(&places.local).docs.len(), 2);
    wf::move_all(&places.local, &places.repo).unwrap();
    assert_eq!(wf::load(&places.repo).docs.len(), 1);
    assert_eq!(wf::load(&places.local).docs.len(), 1);
    // Without the repository's folder, nothing is written there.
    let empty = wf::Places {
        repo: root.join("none/.escouade/workflows"),
        local: root.join("data2/workflows"),
    };
    wf::merge_reading_all(
        &empty,
        &wf::Folder::default(),
        &[dev],
        false,
        &mut wf::Removals::default(),
    );
    assert!(!empty.repo.exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_dev_skill_of_the_repository_goes_in_its_escouade_folder() {
    let h = crate::core_tests::harness("wf-files-dev");
    let (p, dir) = h.project(false).await;
    copy_dir(&fixture("dev/project/.claude"), &dir.join(".claude"));
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(scan.workflows.len(), 1);
    let names: Vec<&str> = scan.workflows[0]
        .steps
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, ["plan", "implement", "review"]);
    assert_eq!(scan.files[0].path, ".escouade/workflows/claude-plan.yml");
    assert!(scan.files[0].in_repo);
    let text = std::fs::read_to_string(dir.join(".escouade/workflows/claude-plan.yml")).unwrap();
    assert!(
        text.contains("subagent") || text.contains("planner"),
        "{text}"
    );
    assert!(
        !text.contains(&dir.to_string_lossy().replace('\\', "/")),
        "{text}"
    );
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let dest = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            std::fs::copy(e.path(), dest).unwrap();
        }
    }
}

// ---------- `.escouade/` of a cloned repository is not to be trusted (ESC-199) ----------

#[test]
fn every_device_name_of_windows_is_kept_away_even_with_an_extension() {
    for n in 0..=9 {
        assert_eq!(
            wf::safe_name(&format!("com{n}")),
            format!("com{n}-workflow")
        );
        assert_eq!(
            wf::safe_name(&format!("LPT{n}")),
            format!("lpt{n}-workflow")
        );
    }
    for name in ["con", "prn", "aux", "nul", "conin", "conout"] {
        assert_eq!(wf::safe_name(name), format!("{name}-workflow"));
    }
    // `con.v2.yml` is the device too, for Windows: the stem before the first dot counts.
    assert_eq!(wf::safe_name("con.v2"), "con-workflow.v2");
    // What only looks like one stays.
    assert_eq!(wf::safe_name("com10"), "com10");
    assert_eq!(wf::safe_name("console"), "console");
}

/// A link to `target` at `link`, when the system allows one (Windows may not without a privilege).
fn link(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        if target.is_dir() {
            // A symbolic link needs a privilege; a junction (what `mklink /J` makes) none, and it leaves
            // the folder all the same.
            std::os::windows::fs::symlink_dir(target, link).is_ok()
                || std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(link.to_string_lossy().replace('/', "\\"))
                    .arg(target.to_string_lossy().replace('/', "\\"))
                    .output()
                    .is_ok_and(|o| o.status.success())
        } else {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        }
    }
}

#[test]
fn a_file_that_is_a_link_is_refused_not_read() {
    let root = test_dir("wf-files-link-file");
    let dir = root.join(".escouade/workflows");
    std::fs::create_dir_all(&dir).unwrap();
    let w = gsd();
    wf::merge_reading(&dir, &wf::load(&dir), std::slice::from_ref(&w));
    let real = wf::load(&dir);
    assert_eq!(real.docs.len(), 1);
    let (path, _) = &real.docs[0];
    let secret = root.join("elsewhere.yml");
    std::fs::rename(path, &secret).unwrap();
    if !link(&secret, path) {
        eprintln!("no symbolic link allowed here: skipped");
        return;
    }
    let folder = wf::load(&dir);
    assert!(folder.docs.is_empty());
    assert_eq!(folder.unreadable.len(), 1);
    assert!(
        folder.unreadable[0].1.contains("lien symbolique")
            || folder.unreadable[0].1.contains("symbolic link"),
        "{}",
        folder.unreadable[0].1
    );
}

#[test]
fn nothing_is_written_when_the_folder_is_a_link_to_elsewhere() {
    let root = test_dir("wf-files-link-dir");
    let outside = root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let project = root.join("project");
    std::fs::create_dir_all(project.join(".escouade")).unwrap();
    let dir = project.join(".escouade/workflows");
    if !link(&outside, &dir) {
        eprintln!("no symbolic link allowed here: skipped");
        return;
    }
    let changed = wf::merge_reading(&dir, &wf::load(&dir), &[gsd()]);
    assert!(changed.is_empty());
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    let folder = wf::load(&dir);
    assert!(folder.docs.is_empty());
    assert_eq!(folder.unreadable.len(), 1, "the section says why");
}

#[test]
fn nothing_is_written_when_escouade_itself_is_a_link() {
    let root = test_dir("wf-files-link-escouade");
    let outside = root.join("outside");
    std::fs::create_dir_all(outside.join("workflows")).unwrap();
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    if !link(&outside, &project.join(".escouade")) {
        eprintln!("no symbolic link allowed here: skipped");
        return;
    }
    let dir = project.join(".escouade/workflows");
    assert!(wf::merge_reading(&dir, &wf::load(&dir), &[gsd()]).is_empty());
    assert_eq!(
        std::fs::read_dir(outside.join("workflows"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn a_plain_folder_is_not_in_the_way() {
    let root = test_dir("wf-files-plain");
    let dir = root.join(".escouade/workflows");
    assert!(wf::unsafe_dir(&dir).is_none(), "not there yet");
    assert!(!wf::merge_reading(&dir, &wf::load(&dir), &[gsd()]).is_empty());
    assert!(wf::unsafe_dir(&dir).is_none());
}

/// ESC-309: a workflow removed (its file deleted by hand, or « Supprimer » in the window) never
/// comes back by itself, neither at a reading nor at a restart; it is proposed again when what
/// found it changes, without being written.
#[tokio::test(flavor = "multi_thread")]
async fn a_workflow_removed_is_never_written_again_by_a_reading() {
    let h = crate::core_tests::harness("wf-files-removed");
    let (p, dir) = two_steps(&h).await;
    let file = dir.join(".escouade/workflows/claude-one.yml");
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(file.is_file());

    // Deleted by hand: out of Escouade, and not written again by the readings that follow.
    std::fs::remove_file(&file).unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(!file.exists(), "the reading wrote the file again");
    assert!(scan.workflows.is_empty());
    assert_eq!(scan.removed.len(), 1);
    assert_eq!(scan.removed[0].id, "|claude|one");
    assert!(!scan.removed[0].changed);
    h.core.refresh_workflows(&p.id, false).await.unwrap();
    assert!(!file.exists());
    // Never read (the reading of this machine forgotten, as after a restart): not written either.
    std::fs::remove_file(h.core.data.workflows_file(&p.id)).unwrap();
    let scan = h.core.refresh_workflows(&p.id, false).await.unwrap();
    assert!(!file.exists() && scan.workflows.is_empty());

    // What found it changed: proposed, not written.
    std::fs::write(
        dir.join(".claude/commands/two.md"),
        "Do two, then /three. Back to /one when red.\n",
    )
    .unwrap();
    std::fs::write(dir.join(".claude/commands/three.md"), "Do three.\n").unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(!file.exists());
    assert!(scan.removed[0].changed, "{:?}", scan.removed);
    // « Laisser supprimé »: no longer proposed.
    let scan = h
        .core
        .set_workflow_removed(&p.id, "|claude|one", true)
        .await
        .unwrap();
    assert!(!scan.removed[0].changed);
    assert!(!file.exists());

    // « Rétablir »: written again, followed again.
    let scan = h
        .core
        .set_workflow_removed(&p.id, "|claude|one", false)
        .await
        .unwrap();
    assert!(file.is_file());
    assert_eq!(scan.workflows.len(), 1);
    assert!(scan.removed.is_empty());

    // « Supprimer » in the window: its file goes, and stays gone.
    let scan = h
        .core
        .set_workflow_removed(&p.id, "|claude|one", true)
        .await
        .unwrap();
    assert!(!file.exists() && scan.workflows.is_empty());
    assert_eq!(scan.removed.len(), 1);
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(!file.exists());

    // Its file back (a pull, another branch): it is the project's again.
    let w = wf::Doc {
        workflow: h.core.cached_reading(&p.id).workflows[0].clone(),
        fixes: Default::default(),
    };
    std::fs::write(&file, wf::write(&w)).unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(scan.workflows.len(), 1);
    assert!(scan.removed.is_empty());
}

/// ESC-309: « Pas de workflow pour ce projet »: nothing read, nothing written, nothing shown nor
/// followed, the files there left as they are.
#[tokio::test(flavor = "multi_thread")]
async fn no_workflow_for_a_project_reads_and_writes_none() {
    let h = crate::core_tests::harness("wf-files-off");
    let (p, dir) = two_steps(&h).await;
    h.core
        .update_project(crate::model::Project {
            workflows_enabled: false,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(scan.workflows.is_empty() && scan.sources.is_empty());
    assert!(scan.off);
    assert!(!dir.join(".escouade/workflows").exists());
    assert!(!h.core.data.workflows_file(&p.id).exists(), "read anyway");
    assert!(h.core.workflows_of(&p.id).workflows.is_empty());
    // Undone: read and written again.
    h.core
        .update_project(crate::model::Project {
            workflows_enabled: true,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let scan = h.core.refresh_workflows(&p.id, false).await.unwrap();
    assert_eq!(scan.workflows.len(), 1);
    assert!(dir.join(".escouade/workflows/claude-one.yml").is_file());
    // Set again: the files stay, but nothing is shown nor followed.
    h.core
        .update_project(crate::model::Project {
            workflows_enabled: false,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(scan.workflows.is_empty());
    assert!(h.core.workflows_of(&p.id).workflows.is_empty());
    assert!(dir.join(".escouade/workflows/claude-one.yml").is_file());
}

/// ESC-309: `merge_reading_all` with a workflow removed writes nothing of it; a file deleted by
/// hand is seen as a removal, one that cannot be read is not.
#[test]
fn a_reading_writes_nothing_of_a_workflow_removed_and_a_file_deleted_is_one() {
    let root = test_dir("wf-files-removals");
    let places = wf::Places {
        repo: root.join("repo/.escouade/workflows"),
        local: root.join("data/workflows"),
    };
    let (home, dev) = (gsd(), dev());
    let mut removals = wf::Removals::default();
    let read = [home.clone(), dev.clone()];
    wf::merge_reading_all(&places, &wf::load_all(&places), &read, true, &mut removals);
    assert_eq!(removals.known.len(), 2);
    assert!(removals.removed.is_empty());
    // Both deleted by hand: neither written again.
    for (path, _) in wf::load_all(&places).docs {
        std::fs::remove_file(path).unwrap();
    }
    let written =
        wf::merge_reading_all(&places, &wf::load_all(&places), &read, true, &mut removals);
    assert!(written.is_empty(), "{written:?}");
    assert!(wf::load_all(&places).docs.is_empty());
    assert_eq!(removals.removed.len(), 2);
    assert!(removals.known.is_empty());
    let listed = removals.listed(&read);
    assert!(listed.iter().all(|r| !r.changed), "{listed:?}");
    assert!(listed.iter().any(|r| r.id == dev.id && r.name == dev.name));
    // What found one changed: proposed, still not written.
    let mut more = dev.clone();
    more.steps.pop();
    let read = [home.clone(), more];
    assert!(
        wf::merge_reading_all(&places, &wf::load_all(&places), &read, true, &mut removals)
            .is_empty()
    );
    let listed = removals.listed(&read);
    assert!(listed.iter().find(|r| r.id == dev.id).unwrap().changed);
    assert!(!listed.iter().find(|r| r.id == home.id).unwrap().changed);

    // A file that cannot be read is not a removal.
    let mut removals = wf::Removals::default();
    let other = test_dir("wf-files-removals-2");
    let places = wf::Places {
        repo: other.join("repo/.escouade/workflows"),
        local: other.join("data/workflows"),
    };
    wf::merge_reading_all(
        &places,
        &wf::Folder::default(),
        std::slice::from_ref(&dev),
        true,
        &mut removals,
    );
    let file = places.repo.join(format!("{}.yml", wf::safe_name(&dev.id)));
    std::fs::write(
        &file,
        "version: [
",
    )
    .unwrap();
    wf::merge_reading_all(
        &places,
        &wf::load_all(&places),
        std::slice::from_ref(&dev),
        true,
        &mut removals,
    );
    assert!(removals.removed.is_empty());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "version: [
"
    );
}
