//! Tests of the reading of a project's workflows (ESC-192): the parsers on text, then the reading
//! on the frozen copies of GSD, Superpowers and Ralph, a project with no workflow and a project of
//! several repositories (`tests/fixtures/workflows/`). The user's own folders are the fixtures'
//! `home`, never the machine's.

use crate::model::EngineId;
use crate::paths::test_dir;
use crate::workflow::*;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/workflows")
        .join(name)
}

/// The user's folders of a fixture (`<fixture>/home`), or a folder with nothing in it.
fn roots(name: &str) -> Roots {
    let home = fixture(name).join("home");
    Roots {
        claude: Some(home.join(".claude")),
        codex: Some(home.join(".codex")),
        home,
    }
}

fn names(w: &Workflow) -> Vec<&str> {
    w.steps.iter().map(|s| s.name.as_str()).collect()
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

// ---------- the parsers ----------

#[test]
fn a_frontmatter_gives_its_keys_and_what_follows() {
    let (front, body) =
        frontmatter("---\nname: x\ndescription: \"A thing\"\ntools: Read, Bash\n---\nBody\n");
    assert_eq!(front["name"], "x");
    assert_eq!(front["description"], "A thing");
    assert_eq!(front["tools"], "Read, Bash");
    assert_eq!(body, "Body\n");
    let (none, text) = frontmatter("No front\n");
    assert!(none.is_empty());
    assert_eq!(text, "No front\n");
    let (folded, _) = frontmatter("---\ndescription: >\n  A long\n  one\n---\n");
    assert_eq!(folded["description"], "A long");
}

#[test]
fn includes_are_the_paths_after_an_at_sign_not_names() {
    let got = includes_of(
        "@~/.claude/get-shit-done/workflows/x.md\nSee @docs/a.md, and @gsd-planner, mail me@example.com, @{var}/x.md\n(@./local.md)",
    );
    assert_eq!(
        got,
        [
            "~/.claude/get-shit-done/workflows/x.md",
            "docs/a.md",
            "./local.md"
        ]
    );
}

#[test]
fn the_agents_a_text_calls_are_its_subagent_types() {
    let got = agent_calls(
        "Task(subagent_type=\"gsd-planner\")\nTask(subagent_type: 'general-purpose')\nsubagent_type=`x:y`\nsubagent_type=\"gsd-planner\"",
    );
    assert_eq!(got, ["gsd-planner", "general-purpose", "x:y"]);
}

#[test]
fn a_door_is_named_as_a_command_a_code_or_qualified_never_inside_a_longer_name() {
    assert!(mentions("run /gsd:plan-phase now", "gsd:plan-phase"));
    assert!(mentions("Use superpowers:writing-plans", "writing-plans"));
    assert!(mentions("the `brainstorming` skill", "brainstorming"));
    assert!(mentions("the plan-phase command", "plan-phase"));
    assert!(!mentions("do some brainstorming", "brainstorming"));
    assert!(!mentions("/gsd:plan-phase-extra", "gsd:plan-phase"));
    assert!(!mentions("/gsd:plan", "gsd:plan-phase"));
    assert!(!mentions("gsd:plan-phase", "plan"));
    // A file named like a step is no mention of it (ESC-195).
    assert!(!mentions("Read docs/plan.md first", "plan"));
    assert!(!mentions("Write plan.md", "plan"));
    assert!(!mentions("see ./review/notes", "review"));
    assert!(mentions("then run /plan, then /review.", "plan"));
    assert!(mentions("(/review)", "review"));
}

#[test]
fn a_maximum_is_read_where_a_text_speaks_of_doing_again() {
    assert_eq!(loop_max("Max 3 iterations."), Some(3));
    assert_eq!(loop_max("revise (iteration_count < 3)"), Some(3));
    assert_eq!(loop_max("Redo it up to 5 times"), Some(5));
    assert_eq!(loop_max("jusqu'à 4 fois"), Some(4));
    assert_eq!(loop_max("or after 20 iterations."), Some(20));
    assert_eq!(loop_max("Max 3 files"), None);
    assert_eq!(loop_max("There are 3 phases"), None);
}

#[test]
fn state_files_are_those_under_planning_known_by_name_or_written() {
    let refs = state_refs(
        "Read `.planning/STATE.md`.\nWrite `notes/out.md`.\nSee `docs/guide.md`.\n@~/.claude/get-shit-done/workflows/x.md\nUpdate CLAUDE.md and SKILL.md",
    );
    let got: Vec<(&str, bool, bool)> = refs.iter().map(|(p, w, r)| (p.as_str(), *w, *r)).collect();
    // Written, or only said: the workflow decides which of those are state (what a step writes).
    assert_eq!(
        got,
        [
            (".planning/STATE.md", false, true),
            ("notes/out.md", true, false),
            ("docs/guide.md", false, true),
        ]
    );
    assert!(is_state(".planning/STATE.md") && is_state("x/STATE.md") && is_state("fix_plan.md"));
    assert!(!is_state("docs/guide.md"));
}

#[test]
fn a_state_file_says_its_phase_its_boxes_and_its_status() {
    let says =
        says_of("# State\nStatus: executing\nCurrent phase: 3 of 6\n- [x] a\n- [ ] b\n- [X] c\n");
    assert_eq!(says.phase, Some(Progress { done: 3, total: 6 }));
    assert_eq!(says.checked, Some(Progress { done: 2, total: 3 }));
    assert_eq!(says.status.as_deref(), Some("executing"));
    assert_eq!(
        says_of("Phase 2/9").phase,
        Some(Progress { done: 2, total: 9 })
    );
    assert_eq!(
        says_of("phase 4 sur 5").phase,
        Some(Progress { done: 4, total: 5 })
    );
    assert_eq!(says_of("nothing").phase, None);
}

#[test]
fn a_numbered_list_gives_the_steps_of_a_turn() {
    let steps = numbered("Intro\n1. Pick it\n2) Do it\n  3. **Check** it\n10 things\nnot 4. this");
    let got: Vec<(u32, &str)> = steps.iter().map(|s| (s.n, s.text.as_str())).collect();
    assert_eq!(got, [(1, "Pick it"), (2, "Do it"), (3, "Check** it")]);
}

// ---------- GSD ----------

#[test]
fn gsd_is_one_workflow_of_four_steps_from_the_commands_of_the_users_folder() {
    let scan = read(&fixture("gsd/project"), &[], &roots("gsd"), &[]);
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let w = &scan.workflows[0];
    assert_eq!(w.name, "GSD");
    assert_eq!(w.kind, Kind::Cycle);
    assert_eq!(w.harness, Some(EngineId::CLAUDE));
    assert_eq!(
        names(w),
        [
            "gsd:discuss-phase",
            "gsd:plan-phase",
            "gsd:execute-phase",
            "gsd:verify-work"
        ]
    );
    assert_eq!(
        w.steps.iter().map(|s| s.order).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    // The `@` of a command brings in the text of the real workflow: its steps are not empty.
    assert!(w.steps[1].description.contains("verification loop"));
}

#[test]
fn gsd_steps_call_their_subagents_found_in_the_users_folder() {
    let scan = read(&fixture("gsd/project"), &[], &roots("gsd"), &[]);
    let w = &scan.workflows[0];
    let agents = |step: usize| -> Vec<&str> {
        w.steps[step]
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect()
    };
    assert_eq!(agents(0), Vec::<&str>::new());
    assert_eq!(
        agents(1),
        ["gsd-phase-researcher", "gsd-plan-checker", "gsd-planner"]
    );
    assert_eq!(agents(2), ["gsd-executor"]);
    assert_eq!(agents(3), ["gsd-verifier"]);
    let planner = w.steps[1]
        .agents
        .iter()
        .find(|a| a.name == "gsd-planner")
        .unwrap();
    assert_eq!(
        planner.path.as_deref(),
        Some("~/.claude/agents/gsd-planner.md")
    );
    assert_eq!(planner.description, "Creates executable phase plans");
}

#[test]
fn gsd_goes_back_to_planning_on_gaps_and_the_planner_is_revised_three_times_at_most() {
    let scan = read(&fixture("gsd/project"), &[], &roots("gsd"), &[]);
    let w = &scan.workflows[0];
    let back: Vec<&Transition> = w.transitions.iter().filter(|t| t.back).collect();
    let gaps = back
        .iter()
        .find(|t| t.from == "gsd:verify-work" && t.to == "gsd:plan-phase")
        .expect("verify goes back to plan");
    assert!(gaps.condition.as_deref().unwrap().contains("--gaps"));
    let revise = back
        .iter()
        .find(|t| t.from == "gsd:plan-phase" && t.to == "gsd:plan-phase")
        .expect("plan goes back to itself");
    assert_eq!(revise.max, Some(3));
    assert!(revise.condition.as_deref().unwrap().contains("iteration"));
    let forward: Vec<(&str, &str)> = w
        .transitions
        .iter()
        .filter(|t| !t.back)
        .map(|t| (t.from.as_str(), t.to.as_str()))
        .collect();
    assert_eq!(
        forward,
        [
            ("gsd:discuss-phase", "gsd:plan-phase"),
            ("gsd:plan-phase", "gsd:execute-phase"),
            ("gsd:execute-phase", "gsd:verify-work"),
        ]
    );
}

#[test]
fn gsd_state_files_say_where_the_work_is() {
    let scan = read(&fixture("gsd/project"), &[], &roots("gsd"), &[]);
    let w = &scan.workflows[0];
    let state = w
        .state_files
        .iter()
        .find(|s| s.path == ".planning/STATE.md")
        .unwrap();
    assert!(state.exists);
    assert_eq!(
        state.says.as_ref().unwrap().phase,
        Some(Progress { done: 3, total: 6 })
    );
    assert_eq!(
        state.says.as_ref().unwrap().status.as_deref(),
        Some("executing")
    );
    assert!(state.written_by.contains(&"gsd:execute-phase".to_string()));
    assert!(state.read_by.contains(&"gsd:plan-phase".to_string()));
    // Said by a folder with a placeholder: kept, as a file that cannot be looked at.
    let plan = w
        .state_files
        .iter()
        .find(|s| s.path.ends_with("PLAN.md"))
        .unwrap();
    assert!(!plan.exists);
    assert_eq!(plan.written_by, ["gsd:plan-phase"]);
    assert!(w.exit.as_deref().unwrap().contains("complete"));
}

#[test]
fn gsd_doors_are_its_steps_the_commands_that_run_several_and_the_rest_of_the_family() {
    let scan = read(&fixture("gsd/project"), &[], &roots("gsd"), &[]);
    let w = &scan.workflows[0];
    let role = |n: &str| w.doors.iter().find(|d| d.name == n).unwrap().role;
    assert_eq!(role("gsd:discuss-phase"), Role::Entry);
    assert_eq!(role("gsd:plan-phase"), Role::Step);
    assert_eq!(role("gsd:autonomous"), Role::Entry);
    assert_eq!(role("gsd:progress"), Role::Entry);
    // A list of every command is no door, nor a step.
    assert_eq!(role("gsd:help"), Role::Other);
    assert_eq!(role("gsd:quick"), Role::Other);
    let auto = w.doors.iter().find(|d| d.name == "gsd:autonomous").unwrap();
    assert_eq!(
        auto.enters,
        ["gsd:discuss-phase", "gsd:execute-phase", "gsd:plan-phase"]
    );
    assert_eq!(auto.command, "/gsd:autonomous");
    assert_eq!(auto.scope, Scope::User);
    assert_eq!(auto.path, "~/.claude/commands/gsd/autonomous.md");
    // No step is a « command alone » of the list.
    assert!(scan.alone.is_empty());
}

#[test]
fn the_users_workflow_counts_for_a_project_that_has_its_trace_only() {
    let bare = read(&fixture("gsd/project-bare"), &[], &roots("gsd"), &[]);
    assert!(bare.workflows.is_empty());
    assert_eq!(bare.untraced.len(), 1);
    assert_eq!(bare.untraced[0].name, "GSD");
    assert_eq!(bare.untraced[0].steps, 4);
    assert!(
        bare.alone.is_empty(),
        "the user's commands are not the project's"
    );
    // An agent of the project launched one of its doors: that is a trace.
    let launched = read(
        &fixture("gsd/project-bare"),
        &[],
        &roots("gsd"),
        &["/gsd:plan-phase".to_string()],
    );
    assert_eq!(launched.workflows.len(), 1);
    assert!(launched.untraced.is_empty());
}

// ---------- Superpowers ----------

#[test]
fn superpowers_is_a_chain_of_skills_that_name_each_other() {
    let scan = read(&fixture("superpowers/project"), &[], &roots("none"), &[]);
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let w = &scan.workflows[0];
    assert_eq!(w.name, "Superpowers");
    assert_eq!(
        names(w),
        [
            "brainstorming",
            "writing-plans",
            "executing-plans",
            "subagent-driven-development",
            "finishing-a-development-branch",
            "requesting-code-review",
        ]
    );
    assert_eq!(
        w.steps.iter().map(|s| s.order).collect::<Vec<_>>(),
        [0, 1, 2, 2, 3, 3]
    );
    assert!(w.transitions.iter().all(|t| !t.back));
    assert!(w
        .transitions
        .iter()
        .any(|t| t.from == "writing-plans" && t.to == "executing-plans"));
    assert!(w
        .transitions
        .iter()
        .any(|t| t.from == "writing-plans" && t.to == "subagent-driven-development"));
    assert_eq!(w.doors.iter().filter(|d| d.role == Role::Entry).count(), 1);
    assert!(w
        .doors
        .iter()
        .all(|d| d.scope == Scope::Project && d.kind == DocKind::Skill));
}

#[test]
fn superpowers_subagents_without_a_file_are_kept_and_the_plan_is_its_state_file() {
    let scan = read(&fixture("superpowers/project"), &[], &roots("none"), &[]);
    let w = &scan.workflows[0];
    let step = w
        .steps
        .iter()
        .find(|s| s.name == "subagent-driven-development")
        .unwrap();
    assert_eq!(step.agents.len(), 1);
    assert_eq!(step.agents[0].name, "general-purpose");
    assert_eq!(step.agents[0].path, None);
    let plan = w
        .state_files
        .iter()
        .find(|s| s.path == "docs/superpowers/plans/2026-01-01-feature.md")
        .unwrap();
    assert_eq!(plan.written_by, ["writing-plans"]);
    assert_eq!(
        plan.read_by,
        ["executing-plans", "subagent-driven-development"]
    );
}

// ---------- Ralph ----------

#[test]
fn ralph_is_a_prompt_run_over_and_over_followed_by_its_turns_and_its_list() {
    let scan = read(&fixture("ralph/project"), &[], &roots("none"), &[]);
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let w = &scan.workflows[0];
    assert_eq!(w.name, "Ralph");
    assert_eq!(w.kind, Kind::Prompt);
    assert_eq!(w.harness, None);
    let l = w.prompt_loop.as_ref().unwrap();
    assert_eq!(l.via, "file");
    assert_eq!(l.prompt, "PROMPT.md");
    assert_eq!(l.steps.len(), 4);
    assert_eq!(l.steps[1].text, "Implement it, and only it.");
    assert_eq!(l.max_turns, Some(20));
    let list = l.list.as_ref().unwrap();
    assert_eq!(list.path, "fix_plan.md");
    assert_eq!(
        list.says.as_ref().unwrap().checked,
        Some(Progress { done: 3, total: 6 })
    );
    // Red tests: back to the step that does it, three attempts.
    assert_eq!(l.returns.len(), 1);
    assert_eq!(
        (l.returns[0].from.as_str(), l.returns[0].to.as_str()),
        ("3", "2")
    );
    assert_eq!(l.returns[0].max, Some(3));
    assert!(w.exit.as_deref().unwrap().contains("no unchecked items"));
}

#[test]
fn a_command_that_is_a_loop_is_a_loop_of_one_prompt() {
    let dir = test_dir("workflow-loop-command");
    let cmd = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmd).unwrap();
    std::fs::write(
        cmd.join("babysit.md"),
        "---\ndescription: Watch the PRs\n---\n/loop 5m check the open pull requests\n1. List them\n2. Fix the red ones\n",
    )
    .unwrap();
    let scan = read(&dir, &[], &roots("none"), &[]);
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let w = &scan.workflows[0];
    assert_eq!(w.kind, Kind::Prompt);
    assert_eq!(w.harness, Some(EngineId::CLAUDE));
    assert_eq!(w.prompt_loop.as_ref().unwrap().via, "loop");
    assert_eq!(w.doors[0].command, "/babysit");
    assert!(scan.alone.is_empty());
}

// ---------- no workflow ----------

#[test]
fn a_project_with_no_workflow_has_none_and_says_where_it_looked() {
    let scan = read(&fixture("none/project"), &[], &roots("none"), &[]);
    assert!(scan.workflows.is_empty());
    assert!(scan.untraced.is_empty());
    // A command alone is not a workflow.
    assert_eq!(scan.alone.len(), 1);
    assert_eq!(scan.alone[0].name, "commit");
    assert_eq!(scan.alone[0].command, "/commit");
    assert_eq!(scan.alone[0].harness, EngineId::CLAUDE);
    assert_eq!(scan.alone[0].path, ".claude/commands/commit.md");
    assert!(scan.searched.contains(&".claude/commands".to_string()));
    assert!(scan.searched.contains(&"PROMPT.md".to_string()));
    assert!(scan
        .sources
        .iter()
        .any(|s| s.kind == "instruction" && s.path.ends_with("CLAUDE.md")));
}

#[test]
fn a_folder_that_does_not_exist_gives_an_empty_reading() {
    let dir = test_dir("workflow-empty");
    let scan = read(&dir.join("gone"), &[], &roots("none"), &[]);
    assert!(scan.workflows.is_empty() && scan.alone.is_empty() && scan.sources.is_empty());
}

// ---------- several repositories ----------

#[test]
fn each_repository_of_a_project_has_its_own_workflows() {
    let repos = vec![String::new(), "api".to_string(), "web".to_string()];
    let scan = read(&fixture("multi/project"), &repos, &roots("none"), &[]);
    assert_eq!(scan.workflows.len(), 2, "{scan:#?}");
    let api = scan.workflows.iter().find(|w| w.repo == "api").unwrap();
    assert_eq!(api.name, "Superpowers");
    assert_eq!(
        names(api),
        [
            "brainstorming",
            "writing-plans",
            "finishing-a-development-branch"
        ]
    );
    assert_eq!(
        api.doors[0].path,
        "api/.claude/skills/brainstorming/SKILL.md"
    );
    let web = scan.workflows.iter().find(|w| w.repo == "web").unwrap();
    assert_eq!(web.kind, Kind::Prompt);
    let list = web.prompt_loop.as_ref().unwrap().list.as_ref().unwrap();
    assert_eq!(list.path, "web/fix_plan.md");
    assert_eq!(
        list.says.as_ref().unwrap().checked,
        Some(Progress { done: 1, total: 2 })
    );
    assert_ne!(api.id, web.id);
    // The root's command alone, said from the root.
    assert_eq!(scan.alone.len(), 1);
    assert_eq!(scan.alone[0].repo, "");
}

#[test]
fn a_repository_missing_from_the_disk_is_skipped() {
    let repos = vec![String::new(), "gone".to_string(), "api".to_string()];
    let scan = read(&fixture("multi/project"), &repos, &roots("none"), &[]);
    assert_eq!(scan.workflows.len(), 1);
    assert_eq!(scan.workflows[0].repo, "api");
}

// ---------- reading again ----------

#[test]
fn a_reading_is_stale_when_a_file_it_read_or_a_folder_it_looked_in_changed() {
    let dir = test_dir("workflow-stale");
    copy_dir(&fixture("gsd/project"), &dir.join("project"));
    copy_dir(&fixture("gsd/home"), &dir.join("home"));
    let (project, roots) = (dir.join("project"), self::roots_in(&dir.join("home")));
    let scan = read(&project, &[], &roots, &[]);
    assert_eq!(scan.workflows.len(), 1);
    assert!(!is_stale(&scan, &project, &[], &roots));

    // A state file written: stale; read again: fresh.
    std::fs::write(
        project.join(".planning/STATE.md"),
        "Status: done\nPhase 6 of 6\n",
    )
    .unwrap();
    assert!(is_stale(&scan, &project, &[], &roots));
    let scan = read(&project, &[], &roots, &[]);
    let state = scan.workflows[0]
        .state_files
        .iter()
        .find(|s| s.path == ".planning/STATE.md")
        .unwrap();
    assert_eq!(
        state.says.as_ref().unwrap().phase,
        Some(Progress { done: 6, total: 6 })
    );
    assert!(!is_stale(&scan, &project, &[], &roots));

    // A command of the user's folder edited, then one added.
    let verify = dir.join("home/.claude/commands/gsd/verify-work.md");
    let text = std::fs::read_to_string(&verify).unwrap();
    std::fs::write(&verify, format!("{text}\nMore.\n")).unwrap();
    assert!(is_stale(&scan, &project, &[], &roots));
    let scan = read(&project, &[], &roots, &[]);
    assert!(!is_stale(&scan, &project, &[], &roots));
    std::fs::write(dir.join("home/.claude/commands/gsd/extra.md"), "Extra.\n").unwrap();
    assert!(is_stale(&scan, &project, &[], &roots));
    let scan = read(&project, &[], &roots, &[]);
    assert!(scan.workflows[0]
        .doors
        .iter()
        .any(|d| d.name == "gsd:extra"));

    // A file gone.
    std::fs::remove_file(project.join(".planning/ROADMAP.md")).ok();
    std::fs::remove_file(project.join(".planning/STATE.md")).unwrap();
    assert!(is_stale(&scan, &project, &[], &roots));
}

fn roots_in(home: &Path) -> Roots {
    Roots {
        claude: Some(home.join(".claude")),
        codex: None,
        home: home.to_path_buf(),
    }
}

#[test]
fn a_change_to_a_step_changes_the_reading_of_the_workflow() {
    let dir = test_dir("workflow-change");
    copy_dir(&fixture("superpowers/project"), &dir);
    let before = read(&dir, &[], &roots("none"), &[]);
    assert_eq!(before.workflows[0].steps.len(), 6);
    // The review no longer follows the subagent step: it falls out of the workflow.
    let path = dir.join(".claude/skills/subagent-driven-development/SKILL.md");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("superpowers:requesting-code-review", "nothing");
    std::fs::write(&path, text).unwrap();
    let after = read(&dir, &[], &roots("none"), &[]);
    assert_eq!(after.workflows[0].steps.len(), 5);
    assert!(after
        .alone
        .iter()
        .any(|a| a.name == "requesting-code-review"));
}

// ---------- the user's folders ----------

#[test]
fn the_tests_never_read_the_real_claude_folder() {
    let roots = Roots::of_user();
    let temp = std::env::temp_dir();
    assert!(roots.home.starts_with(&temp), "{:?}", roots.home);
    assert!(roots.claude.unwrap().starts_with(&temp));
    assert!(roots.codex.unwrap().starts_with(&temp));
}

#[test]
fn codex_prompts_and_skills_are_read_from_its_folder() {
    let dir = test_dir("workflow-codex");
    let codex = dir.join("home/.codex");
    std::fs::create_dir_all(codex.join("prompts")).unwrap();
    std::fs::write(
        codex.join("prompts/spec.md"),
        "Write the spec, then run /build.\n",
    )
    .unwrap();
    std::fs::write(
        codex.join("prompts/build.md"),
        "Build it from `.planning/STATE.md`. Write `.planning/STATE.md`.\n",
    )
    .unwrap();
    let project = dir.join("project");
    std::fs::create_dir_all(project.join(".planning")).unwrap();
    std::fs::write(project.join(".planning/STATE.md"), "Phase 1 of 2\n").unwrap();
    let roots = Roots {
        home: dir.join("home"),
        claude: None,
        codex: Some(codex),
    };
    let scan = read(&project, &[], &roots, &[]);
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let w = &scan.workflows[0];
    assert_eq!(w.harness, Some(EngineId::CODEX));
    assert_eq!(names(w), ["spec", "build"]);
}

#[test]
fn other_harnesses_are_read_from_their_folders() {
    let dir = test_dir("workflow-harnesses");
    let files = [
        (".opencode/command/plan.md", "Plan it, then run /build.\n"),
        (".opencode/command/build.md", "Build it. Then /ship.\n"),
        (".opencode/command/ship.md", "Ship it.\n"),
        (".github/prompts/a.prompt.md", "Do a, then `b`.\n"),
        (".github/prompts/b.prompt.md", "Do b.\n"),
        (".cursor/commands/x.md", "Do x, then /y.\n"),
        (".cursor/commands/y.md", "Do y.\n"),
    ];
    for (p, t) in files {
        let f = dir.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, t).unwrap();
    }
    let scan = read(&dir, &[], &roots("none"), &[]);
    let by = |h: EngineId| scan.workflows.iter().find(|w| w.harness == Some(h));
    assert_eq!(
        names(by(EngineId::OPENCODE).unwrap()),
        ["plan", "build", "ship"]
    );
    assert_eq!(names(by(EngineId::COPILOT).unwrap()), ["a", "b"]);
    assert_eq!(names(by(EngineId::CURSOR).unwrap()), ["x", "y"]);
    assert!(by(EngineId::CLAUDE).is_none());
}

// ---------- kept in the data folder ----------

#[tokio::test]
async fn the_reading_is_kept_in_the_data_folder_and_read_again_when_a_file_changed() {
    let h = crate::core_tests::harness("workflow-core");
    let (p, dir) = h.project(false).await;
    let cmds = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmds).unwrap();
    std::fs::write(cmds.join("one.md"), "Do one, then /two.\n").unwrap();
    std::fs::write(cmds.join("two.md"), "Do two.\n").unwrap();
    let file = h.core.data.workflows_file(&p.id);
    assert!(!file.exists());

    let first = h.core.refresh_workflows(&p.id, false).await.unwrap();
    assert_eq!(first.workflows.len(), 1, "{first:#?}");
    assert!(file.is_file());
    // Never in the project: it is the data folder's, which no repository holds.
    assert!(file.starts_with(h.core.data.root()));
    assert!(!file.starts_with(&dir));
    let kept: Scan = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(kept.read_at, first.read_at);

    // Nothing changed: the reading kept is the one given back.
    let again = h.core.refresh_workflows(&p.id, false).await.unwrap();
    assert_eq!(again.read_at, first.read_at);

    // A file it read changed: read again, and kept.
    std::fs::write(cmds.join("two.md"), "Do two, then /three.\n").unwrap();
    std::fs::write(cmds.join("three.md"), "Do three.\n").unwrap();
    let changed = h.core.refresh_workflows(&p.id, false).await.unwrap();
    assert_eq!(changed.workflows[0].steps.len(), 3);
    let kept: Scan = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(kept.workflows[0].steps.len(), 3);

    // « Relire » reads again whatever.
    std::thread::sleep(std::time::Duration::from_millis(5));
    let forced = h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert!(forced.read_at > changed.read_at);
    assert!(h.core.refresh_workflows("nope", false).await.is_err());
}

#[test]
fn with_claude_code_s_folder_elsewhere_its_includes_are_read_there_and_said_as_its() {
    // CLAUDE_CONFIG_DIR away from the home folder (ESC-195): `@~/.claude/…` is that folder, never the
    // home's own `.claude`.
    let dir = test_dir("workflow-config-dir");
    let claude = dir.join("config");
    copy_dir(&fixture("gsd").join("home").join(".claude"), &claude);
    let roots = Roots {
        home: dir.join("home"),
        claude: Some(claude),
        codex: None,
    };
    let scan = read(&fixture("gsd/project"), &[], &roots, &[]);
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    assert!(scan.workflows[0].steps[1]
        .description
        .contains("verification loop"));
    assert!(scan
        .sources
        .iter()
        .any(|s| s.kind == "include" && s.path.contains("/config/get-shit-done/")));
    assert_eq!(
        scan.workflows[0].steps[1].path,
        "~/.claude/commands/gsd/plan-phase.md"
    );
    assert!(scan.claude.ends_with("/config"), "{}", scan.claude);
    assert!(scan.root.ends_with("gsd/project"), "{}", scan.root);
}

// ---------- what the reading may open ----------

/// A project whose two commands name, one after the other, every file the reading must not open.
fn hostile(name: &str, extra: &str) -> (PathBuf, Roots, String) {
    let dir = test_dir(name);
    let project = dir.join("project");
    std::fs::create_dir_all(project.join(".claude/commands")).unwrap();
    std::fs::create_dir_all(dir.join("outside")).unwrap();
    let outside = dir.join("outside/secret.md");
    std::fs::write(&outside, "TOPSECRET outside\n").unwrap();
    let home = dir.join("home");
    std::fs::create_dir_all(home.join(".claude/projects")).unwrap();
    std::fs::write(home.join(".claude/.credentials.json"), "TOPSECRET token\n").unwrap();
    std::fs::write(home.join(".claude/projects/chat.md"), "TOPSECRET chat\n").unwrap();
    std::fs::create_dir_all(project.join("secrets")).unwrap();
    std::fs::write(project.join("secrets/.env"), "TOPSECRET env\n").unwrap();
    std::fs::write(project.join("data.bin"), "TOPSECRET bin\n").unwrap();
    let abs = outside
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    std::fs::write(
        project.join(".claude/commands/one.md"),
        format!(
            "Run /two, then read\n@{abs}\n@../outside/secret.md\n@../../outside/secret.md\n@~/.claude/.credentials.json\n@~/.claude/projects/chat.md\n@secrets/.env\n@data.bin\n{extra}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        project.join(".claude/commands/two.md"),
        "Done. Run /one again.\n",
    )
    .unwrap();
    let roots = Roots {
        claude: Some(home.join(".claude")),
        codex: Some(home.join(".codex")),
        home,
    };
    (project, roots, abs)
}

fn assert_not_read(scan: &Scan) {
    let json = serde_json::to_string(scan).unwrap();
    assert!(!json.contains("TOPSECRET"), "{json}");
    for s in &scan.sources {
        assert!(!s.path.contains("secret.md"), "{}", s.path);
        assert!(!s.path.contains(".credentials"), "{}", s.path);
        assert!(!s.path.contains(".env"), "{}", s.path);
        assert!(!s.path.contains("chat.md"), "{}", s.path);
        assert!(!s.path.contains("data.bin"), "{}", s.path);
    }
}

#[test]
fn an_include_that_names_a_file_outside_is_never_read_and_is_said_ignored() {
    let (project, roots, abs) = hostile("workflow-hostile", "");
    let scan = read(&project, &[], &roots, &[]);
    assert_not_read(&scan);
    let ignored: Vec<&String> = scan
        .searched
        .iter()
        .filter(|s| s.starts_with(IGNORED))
        .collect();
    let has = |why: &str, what: &str| {
        ignored
            .iter()
            .any(|s| s.starts_with(&format!("{IGNORED}{why}:")) && s.ends_with(what))
    };
    assert!(has("outside", &abs), "{ignored:?}");
    assert!(has("outside", "../outside/secret.md"), "{ignored:?}");
    assert!(has("secret", "~/.claude/.credentials.json"), "{ignored:?}");
    assert!(
        has("transcript", "~/.claude/projects/chat.md"),
        "{ignored:?}"
    );
    assert!(has("secret", "secrets/.env"), "{ignored:?}");
    assert!(has("extension", "data.bin"), "{ignored:?}");
}

#[test]
fn a_state_file_outside_the_repository_or_a_secret_is_not_read() {
    let (project, roots, _) = hostile(
        "workflow-hostile-state",
        "Write `../outside/secret.md` and `.env`; update `.planning/STATE.md`.",
    );
    std::fs::create_dir_all(project.join(".planning")).unwrap();
    std::fs::write(project.join(".planning/STATE.md"), "Phase 1 of 2\n").unwrap();
    let scan = read(&project, &[], &roots, &[]);
    assert_not_read(&scan);
    // The state file of the repository is still read.
    assert!(scan
        .workflows
        .iter()
        .flat_map(|w| &w.state_files)
        .any(|f| f.path.ends_with("STATE.md") && f.exists));
}

#[test]
fn a_link_that_leaves_the_repository_is_not_followed() {
    let (project, roots, _) = hostile("workflow-hostile-link", "");
    let dir = project.parent().unwrap();
    let linked = project.join("linked.md");
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(dir.join("outside/secret.md"), &linked).is_ok();
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(dir.join("outside/secret.md"), &linked).is_ok();
    if !made {
        eprintln!("no symbolic link here: not tried");
        return;
    }
    let one = project.join(".claude/commands/one.md");
    let text = std::fs::read_to_string(&one).unwrap();
    std::fs::write(&one, format!("{text}\n@linked.md\n")).unwrap();
    // A command that is itself a link to a file outside.
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink(
        dir.join("outside/secret.md"),
        project.join(".claude/commands/three.md"),
    );
    #[cfg(windows)]
    let _ = std::os::windows::fs::symlink_file(
        dir.join("outside/secret.md"),
        project.join(".claude/commands/three.md"),
    );
    let scan = read(&project, &[], &roots, &[]);
    assert_not_read(&scan);
    assert!(scan
        .searched
        .iter()
        .any(|s| s.starts_with(IGNORED) && s.ends_with("linked.md")));
}

#[test]
fn a_skill_of_the_repository_that_runs_the_plan_the_implementation_and_the_review_is_their_entry() {
    // As at HelloAsso: `/dev` analyses the need, then runs the plan, the implementation and the
    // review, each a skill of the repository with its sub-agent of `.claude/agents`.
    let scan = read(&fixture("dev/project"), &[], &roots("none"), &[]);
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let w = &scan.workflows[0];
    assert_eq!(names(w), ["plan", "implement", "review"]);
    let agents: Vec<&str> = w.steps.iter().map(|s| s.agents[0].name.as_str()).collect();
    assert_eq!(agents, ["planner", "implementer", "reviewer"]);
    assert_eq!(
        w.steps[0].agents[0].path.as_deref(),
        Some(".claude/agents/planner.md")
    );
    let dev = w.doors.iter().find(|d| d.name == "dev").unwrap();
    assert_eq!(dev.role, Role::Entry);
    assert_eq!(dev.scope, Scope::Project);
    let mut enters = dev.enters.clone();
    enters.sort();
    assert_eq!(enters, ["implement", "plan", "review"]);
    assert!(w
        .transitions
        .iter()
        .any(|t| t.from == "review" && t.to == "implement" && t.back && t.max == Some(3)));
    assert!(crate::workflowfiles::in_repo(w));
}

#[test]
fn the_agents_folder_is_read_whole_its_skills_agents_commands_and_prompts() {
    let scan = read(&fixture("agents/project"), &[], &roots("none"), &[]);
    assert_eq!(scan.workflows.len(), 1, "{scan:#?}");
    let w = &scan.workflows[0];
    assert_eq!(names(w), ["triage", "fix", "ship"]);
    assert_eq!(w.steps[0].path, ".agents/skills/triage/SKILL.md");
    assert_eq!(w.steps[1].path, ".agents/commands/fix.md");
    assert_eq!(w.steps[2].path, ".agents/prompts/ship.md");
    assert_eq!(
        w.steps[1].agents[0].path.as_deref(),
        Some(".agents/agents/fixer.md")
    );
    for f in [
        ".agents/skills",
        ".agents/agents",
        ".agents/commands",
        ".agents/prompts",
    ] {
        assert!(scan.searched.iter().any(|s| s == f), "{f}");
    }
}
