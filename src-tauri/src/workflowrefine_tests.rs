//! « Affiner… » (ESC-196): the answer read and checked by the rules, and the whole cycle on the real
//! core against the fake `claude` that answers valid, invalid, or making things up.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::workflow::{Kind, Step, StepAgent, Transition, Workflow};
use crate::workflowrefine::*;

fn step(name: &str, order: usize, path: &str) -> Step {
    Step {
        name: name.into(),
        command: format!("/{name}"),
        order,
        description: String::new(),
        path: path.into(),
        agents: vec![StepAgent {
            name: "worker".into(),
            path: None,
            description: String::new(),
        }],
        reads: Vec::new(),
        writes: Vec::new(),
        guessed: false,
        also: Vec::new(),
    }
}

fn workflow() -> Workflow {
    Workflow {
        id: "|claude|one".into(),
        name: "One".into(),
        kind: Kind::Cycle,
        harness: Some(crate::model::EngineId::CLAUDE),
        repo: String::new(),
        doors: Vec::new(),
        steps: vec![
            step("one", 0, ".claude/commands/one.md"),
            step("two", 1, ".claude/commands/two.md"),
        ],
        transitions: vec![Transition {
            from: "one".into(),
            to: "two".into(),
            back: false,
            condition: None,
            max: None,
        }],
        state_files: Vec::new(),
        exit: None,
        prompt_loop: None,
        refined: None,
    }
}

fn known() -> BTreeSet<String> {
    [
        ".claude/commands/one.md",
        ".claude/commands/two.md",
        ".claude/commands/three.md",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn answer(json: &str) -> Answer {
    parse(json).unwrap()
}

// ---------- the answer in the schema ----------

#[test]
fn an_answer_is_read_plain_or_in_a_fence_or_among_words() {
    let json = r#"{"workflows":[{"id":"a","steps":[{"name":"x"}]}]}"#;
    assert_eq!(answer(json).workflows.len(), 1);
    assert_eq!(answer(&format!("```json\n{json}\n```")).workflows.len(), 1);
    assert_eq!(
        answer(&format!("Voici : {json}\nVoilà.")).workflows.len(),
        1
    );
}

#[test]
fn an_answer_that_is_not_the_schema_is_refused() {
    for bad in [
        "Tout va bien.",
        "{}",
        r#"{"workflows":"non"}"#,
        r#"{"workflows":[{"steps":[]}]}"#,
        r#"{"workflows":[{"id":"a","steps":[{"description":"sans nom"}]}]}"#,
        r#"{"workflows":[{"id":"a","steps":"non"}]}"#,
    ] {
        assert!(parse(bad).is_err(), "{bad}");
    }
}

// ---------- what the rules let in ----------

#[test]
fn a_step_described_better_and_a_way_back_are_taken() {
    let w = workflow();
    let a = answer(
        r#"{"workflows":[{"id":"|claude|one","steps":[
            {"name":"one","description":"Fait le premier travail."},
            {"name":"two","description":"Vérifie."}],
          "transitions":[{"from":"two","to":"one","condition":"si rouge","max":3}]}]}"#,
    );
    let c = check(&w, &a.workflows[0], &known());
    assert!(c.changed() && c.rejected.is_empty(), "{:?}", c.rejected);
    assert_eq!(c.workflow.steps[0].description, "Fait le premier travail.");
    assert!(!c.workflow.steps[0].guessed);
    // What the rules found stays as found: its command, its sub-agent.
    assert_eq!(c.workflow.steps[1].command, "/two");
    assert_eq!(c.workflow.steps[1].agents[0].name, "worker");
    let back = c.workflow.transitions.last().unwrap();
    assert_eq!((back.from.as_str(), back.to.as_str()), ("two", "one"));
    assert!(back.back);
    assert_eq!(
        (back.condition.as_deref(), back.max),
        (Some("si rouge"), Some(3))
    );
    assert_eq!(c.refined.described, 2);
    assert_eq!(c.refined.transitions, 1);
    // Said from the reading it was made from.
    assert_eq!(
        c.workflow.refined,
        Some(crate::workflowfiles::fingerprint_of(&w))
    );
}

#[test]
fn a_step_that_stands_on_a_known_file_enters_guessed() {
    let w = workflow();
    let a = answer(
        r#"{"workflows":[{"id":"|claude|one","steps":[
            {"name":"three","command":"/three","description":"Une troisième.","path":".claude\\commands\\three.md","agents":["worker","inventé"]}]}]}"#,
    );
    let c = check(&w, &a.workflows[0], &known());
    assert_eq!(c.refined.added, vec!["three"]);
    // In the order the agent gave, those it left out after.
    let names: Vec<&str> = c.workflow.steps.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["three", "one", "two"]);
    assert_eq!(c.workflow.steps[0].order, 0);
    assert_eq!(c.workflow.steps[2].order, 2);
    let three = &c.workflow.steps[0];
    assert!(three.guessed, "marked guessed");
    assert_eq!(three.path, ".claude/commands/three.md");
    // Only a sub-agent the workflow already has.
    let agents: Vec<&str> = three.agents.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(agents, ["worker"]);
}

#[test]
fn what_is_made_up_is_refused_and_told() {
    let w = workflow();
    let a = answer(
        r#"{"workflows":[{"id":"|claude|one","steps":[
            {"name":"ghost","command":"/ghost","description":"Sans fichier.","path":""},
            {"name":"far","description":"Un fichier qui n'existe pas.","path":"nowhere.md"}],
          "transitions":[{"from":"one","to":"ghost"},{"from":"nobody","to":"two"}]}]}"#,
    );
    let c = check(&w, &a.workflows[0], &known());
    assert!(!c.changed(), "{:?}", c.refined);
    assert_eq!(c.workflow, {
        let mut same = w.clone();
        same.refined = None;
        same
    });
    assert_eq!(c.rejected.len(), 4, "{:?}", c.rejected);
    assert!(c.rejected[0].contains("ghost") && c.rejected[1].contains("far"));
}

#[test]
fn a_workflow_the_project_does_not_have_is_refused_and_one_answered_twice_is_taken_once() {
    let w = workflow();
    let a = answer(
        r#"{"workflows":[
            {"id":"inconnu","steps":[]},
            {"id":"|claude|one","steps":[{"name":"one","description":"A."}]},
            {"id":"|claude|one","steps":[{"name":"one","description":"B."}]}]}"#,
    );
    let (checked, rejected) = check_all(&[w], &a, &known());
    assert_eq!(checked.len(), 1);
    assert_eq!(checked[0].workflow.steps[0].description, "A.");
    assert_eq!(rejected.len(), 1);
    assert!(rejected[0].contains("inconnu"));
}

#[test]
fn the_question_names_the_workflows_and_the_files_and_the_cost_is_estimated() {
    let w = workflow();
    let files = vec![(
        ".claude/commands/one.md".to_string(),
        "Fais un.".to_string(),
    )];
    for lang in [crate::i18n::Lang::Fr, crate::i18n::Lang::En] {
        let p = prompt(lang, std::slice::from_ref(&w), &files);
        assert!(p.contains("<workflows-json>") && p.contains("|claude|one"));
        assert!(p.contains("=== .claude/commands/one.md\nFais un."));
        assert!(system(lang).contains("JSON"));
    }
    let (i, o, cost) = estimate("sys", &"x".repeat(30_000), 2, "haiku");
    assert!(i >= 10_000 && o > 0);
    assert!(cost.unwrap() > 0.0 && cost.unwrap() < 0.1);
    assert_eq!(estimate("s", "p", 1, "un-modele-inconnu").2, None);
}

// ---------- the whole cycle, on the real core, against the fake claude ----------

/// A project whose own commands make a workflow of two steps; `note` is added to the first.
async fn project(h: &crate::core_tests::Harness, note: &str) -> (crate::model::Project, PathBuf) {
    let (p, dir) = h.project(false).await;
    let cmds = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmds).unwrap();
    std::fs::write(
        cmds.join("one.md"),
        format!("Do one with subagent_type=\"worker\", then /two. {note}\n"),
    )
    .unwrap();
    std::fs::write(cmds.join("two.md"), "Do two. Back to /one when red.\n").unwrap();
    (p, dir)
}

fn file_of(dir: &Path) -> PathBuf {
    dir.join(".escouade/workflows/claude-one.yml")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_plan_says_the_model_the_account_and_the_cost_before() {
    let h = crate::core_tests::harness("wf-refine-plan");
    let (p, _) = project(&h, "").await;
    let plan = h.core.workflow_refine_plan(&p.id).await.unwrap();
    assert_eq!(plan.workflows, 1);
    assert!(plan.files >= 2, "{plan:?}");
    assert_eq!(plan.harness, "claude");
    assert_eq!(plan.model, "Claude Haiku");
    assert_eq!(plan.account_id, crate::accounts::PRINCIPAL);
    assert!(plan.input_tokens > 0 && plan.output_tokens > 0);
    assert!(plan.cost.is_some_and(|c| c > 0.0));
    assert!(!plan.private);
    // Said only: nothing was asked, nothing written.
    assert!(!h.dir.join("data").join("workflows-asked").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_valid_answer_is_written_into_the_file_and_nothing_is_committed() {
    let h = crate::core_tests::harness("wf-refine-valid");
    let (p, dir) = project(&h, "").await;
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    let before = std::fs::read_to_string(file_of(&dir)).unwrap();
    let commits = git(&dir, &["rev-list", "--count", "HEAD"]);

    let out = h.core.refine_workflows(&p.id).await.unwrap();
    assert!(out.rejected.is_empty(), "{:?}", out.rejected);
    assert_eq!(out.refined.len(), 1);
    assert_eq!(out.refined[0].described, 2);
    // The way back was read with its condition, not its maximum: the agent says the maximum, and
    // leaves the condition.
    assert_eq!(out.refined[0].transitions, 1);
    let text = std::fs::read_to_string(file_of(&dir)).unwrap();
    assert!(text.contains("Précisé par le faux claude : one"), "{text}");
    assert!(text.contains("refined:"), "{text}");
    assert!(
        text.contains("Back to /one when red") && !text.contains("si rouge"),
        "{text}"
    );
    assert_ne!(text, before);
    let w = &out.scan.workflows[0];
    assert_eq!(w.steps[0].description, "Précisé par le faux claude : one");
    assert_eq!(
        out.scan.files[0]
            .updated
            .as_deref()
            .map(|u| u.contains("agent")),
        Some(true)
    );
    // Escouade commits nothing.
    assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]), commits);

    // Read again: the reading is the same, what the agent made stands.
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    assert_eq!(std::fs::read_to_string(file_of(&dir)).unwrap(), text);

    // Refined again: it says the same, so nothing is written.
    let again = h.core.refine_workflows(&p.id).await.unwrap();
    assert!(again.refined.is_empty(), "{:?}", again.refined);
    assert_eq!(std::fs::read_to_string(file_of(&dir)).unwrap(), text);

    // A reading that finds the workflow changed writes its own.
    std::fs::write(
        dir.join(".claude/commands/two.md"),
        "Do two differently. Back to /one when red.\n",
    )
    .unwrap();
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    let text = std::fs::read_to_string(file_of(&dir)).unwrap();
    assert!(!text.contains("refined:"), "{text}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_users_corrections_stay_when_the_workflow_is_refined() {
    let h = crate::core_tests::harness("wf-refine-fixes");
    let (p, dir) = project(&h, "").await;
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    let w = scan.workflows[0].clone();
    h.core
        .update_project(crate::model::Project {
            workflow_fixes: vec![crate::workflowfix::Fix {
                workflow: w.id.clone(),
                steps: vec![crate::workflowfix::FixStep {
                    name: "Premier".into(),
                    from: vec![w.steps[0].name.clone()],
                    signals: Vec::new(),
                }],
                ..Default::default()
            }],
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    h.core.refine_workflows(&p.id).await.unwrap();
    let text = std::fs::read_to_string(file_of(&dir)).unwrap();
    assert!(text.contains("name: Premier") && text.contains("Précisé par le faux claude"));
    assert_eq!(h.core.project(&p.id).unwrap().workflow_fixes.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_out_of_the_schema_changes_nothing() {
    let h = crate::core_tests::harness("wf-refine-invalid");
    let (p, dir) = project(&h, "[invalide]").await;
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    let before = std::fs::read_to_string(file_of(&dir)).unwrap();
    let e = h.core.refine_workflows(&p.id).await.unwrap_err();
    assert!(format!("{e:#}").contains("rien n'a changé"), "{e:#}");
    assert_eq!(std::fs::read_to_string(file_of(&dir)).unwrap(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_model_that_fails_changes_nothing() {
    let h = crate::core_tests::harness("wf-refine-mute");
    let (p, dir) = project(&h, "[muet]").await;
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    let before = std::fs::read_to_string(file_of(&dir)).unwrap();
    assert!(h.core.refine_workflows(&p.id).await.is_err());
    assert_eq!(std::fs::read_to_string(file_of(&dir)).unwrap(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn what_the_agent_makes_up_does_not_get_in_unmarked() {
    let h = crate::core_tests::harness("wf-refine-made-up");
    let (p, dir) = project(&h, "[invente]").await;
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    let out = h.core.refine_workflows(&p.id).await.unwrap();
    let w = &out.scan.workflows[0];
    let names: Vec<&str> = w.steps.iter().map(|s| s.name.as_str()).collect();
    // « Devinée » stands on a file the reading saw, and is guessed; « Fantôme » has none: refused.
    assert!(
        names.contains(&"Devinée") && !names.contains(&"Fantôme"),
        "{names:?}"
    );
    let guessed = w.steps.iter().find(|s| s.name == "Devinée").unwrap();
    assert!(guessed.guessed);
    assert!(w.steps.iter().filter(|s| !s.guessed).count() >= 2);
    assert!(
        guessed.agents.is_empty(),
        "an agent no file defines is dropped"
    );
    let said = out.rejected.join("\n");
    assert!(said.contains("Fantôme"), "{said}");
    assert!(said.contains("Absent"), "{said}");
    assert!(said.contains("inconnu|x"), "{said}");
    let text = std::fs::read_to_string(file_of(&dir)).unwrap();
    assert!(text.contains("guessed: true"), "{text}");
    assert!(!text.contains("Fantôme"), "{text}");
}

#[tokio::test(flavor = "multi_thread")]
async fn in_a_private_project_the_agent_is_asked_of_an_account_reserved_to_it() {
    let h = crate::core_tests::harness("wf-refine-private");
    let (p, _) = project(&h, "").await;
    let principal = h.core.settings.read().accounts[0].clone();
    // Principal kept to another project: no account serves this one, nothing is asked.
    h.core
        .update_claude_account(crate::model::Account {
            projects: vec!["autre".into()],
            ..principal.clone()
        })
        .unwrap();
    let e = h.core.workflow_refine_plan(&p.id).await.unwrap_err();
    assert!(!format!("{e:#}").is_empty());
    assert!(h.core.refine_workflows(&p.id).await.is_err());
    // Kept to this project: it is the account that answers, and says so before.
    h.core
        .update_claude_account(crate::model::Account {
            projects: vec![p.id.clone()],
            ..principal
        })
        .unwrap();
    let plan = h.core.workflow_refine_plan(&p.id).await.unwrap();
    assert_eq!(plan.account_id, crate::accounts::PRINCIPAL);
    assert!(h.core.refine_workflows(&p.id).await.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_without_a_workflow_has_nothing_to_refine() {
    let h = crate::core_tests::harness("wf-refine-none");
    let (p, _) = h.project(false).await;
    assert!(h.core.workflow_refine_plan(&p.id).await.is_err());
    assert!(h.core.refine_workflows(&p.id).await.is_err());
}

// ---------- what is sent to the model ----------

fn secure_dirs(name: &str) -> (PathBuf, crate::workflow::Roots) {
    let dir = crate::paths::test_dir(name);
    let home = dir.join("home");
    std::fs::create_dir_all(home.join(".claude")).unwrap();
    std::fs::create_dir_all(dir.join("project/.claude/commands")).unwrap();
    std::fs::write(
        home.join(".claude/settings.json"),
        "{\"env\":{\"KEY\":\"secret\"}}",
    )
    .unwrap();
    std::fs::write(
        home.join(".claude/autre.md"),
        "Ce fichier n'est dans aucune lecture.\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("project/.claude/commands/one.md"),
        "Do one, then /two.\n",
    )
    .unwrap();
    std::fs::write(dir.join("project/.claude/commands/two.md"), "Do two.\n").unwrap();
    let roots = crate::workflow::Roots {
        claude: Some(home.join(".claude")),
        codex: None,
        home,
    };
    (dir.join("project"), roots)
}

#[test]
fn a_path_only_a_shared_file_names_is_never_read_nor_sent() {
    let (project, roots) = secure_dirs("wf-refine-sources");
    let scan = crate::workflow::read(&project, &[], &roots, &[]);
    let mut w = scan.workflows[0].clone();
    // A shared workflow file (from a cloned repository) names files the reading never found.
    for path in ["~/.claude/settings.json", "~/.claude/autre.md"] {
        w.doors[0].path = path.into();
        w.steps[1].agents = vec![StepAgent {
            name: "x".into(),
            path: Some(path.into()),
            description: String::new(),
        }];
        let files = crate::workflow::files_in_cause(&project, &[w.clone()], &scan.sources, &roots);
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(!paths.contains(&path), "{path}: {paths:?}");
        assert!(files
            .iter()
            .all(|(_, t)| !t.contains("secret") && !t.contains("aucune lecture")));
        // What the reading found is still given.
        assert!(paths.contains(&".claude/commands/one.md"), "{paths:?}");
    }
    // Even named as a step's own path, with no source behind it, it is not read.
    w.steps[0].path = "~/.claude/autre.md".into();
    let files = crate::workflow::files_in_cause(&project, &[w.clone()], &[], &roots);
    assert!(files.is_empty(), "{files:?}");
}

#[test]
fn a_step_added_on_a_file_that_was_not_sent_is_refused() {
    let w = workflow();
    // `known` is what was sent: the path of the settings is not among it.
    let a = answer(
        r#"{"workflows":[{"id":"|claude|one","steps":[
            {"name":"leak","description":"copié","path":"~/.claude/settings.json"}]}]}"#,
    );
    let c = check(&w, &a.workflows[0], &known());
    assert!(!c.changed());
    assert!(c.rejected[0].contains("leak"), "{:?}", c.rejected);
}

#[test]
fn the_settings_of_a_harness_are_never_read_even_when_a_command_includes_them() {
    let (project, roots) = secure_dirs("wf-refine-config");
    std::fs::write(
        project.join(".claude/settings.local.json"),
        "{\"env\":{\"KEY\":\"local\"}}",
    )
    .unwrap();
    std::fs::write(project.join(".claude/settings.json"), "{}").unwrap();
    std::fs::write(
        project.join(".claude/commands/one.md"),
        "Do one with @.claude/settings.local.json and @.claude/settings.json and @~/.claude/settings.json, then /two.\n",
    )
    .unwrap();
    let scan = crate::workflow::read(&project, &[], &roots, &[]);
    for s in &scan.sources {
        assert!(!s.path.contains("settings"), "{}", s.path);
    }
    let said = scan.searched.join("\n");
    assert!(said.contains("ignored:config:"), "{said}");
    assert!(!said.contains("KEY"));
}
