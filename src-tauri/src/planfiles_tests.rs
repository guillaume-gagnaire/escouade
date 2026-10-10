//! Tests of what `planfiles` reads of an agent's folder: the parsers on text, then the disk on
//! throwaway folders built the way the superpowers scripts build theirs (`sdd-workspace`,
//! `task-brief`, `task-done`).

use crate::paths::{make_dir_link, make_file_link, test_dir};
use crate::plan::TaskStatus;
use crate::planfiles::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

fn ids(plan: &ParsedPlan) -> Vec<&str> {
    plan.tasks.iter().map(|t| t.id.as_str()).collect()
}

fn set(ids: &[&str]) -> std::collections::BTreeSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

// ---------- parse_plan ----------

/// The shapes of this repository's own plan (docs/superpowers/plans/2026-10-10-escouade-1-7.md),
/// shortened: the H1 with its suffix, ids like `L1` and `K4`, a heading for several tasks at once,
/// the files list, steps checked or not.
const REPO_PLAN: &str = "# Escouade 1.7 — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development.

**Goal:** Livrer la 1.7.

## Chantier L — Langues

### Task L1: Mécanique de la fenêtre (catalogues typés, `t`, pluriels, `Rich`, formats, garde-fous)

**Files:**
- Create: `src/lib/i18n/index.ts`

- [x] Tests rouges : `t()` rend la clé
- [x] `index.ts` : le dictionnaire
- [ ] Vérifications globales

### Tasks L3 à L9: Extraction des textes de la fenêtre

- [ ] Une case qui n'est à aucune tâche

## Chantier K — Plusieurs comptes Claude

### Task K4: Choix du compte, pause quand tous ont passé le seuil, toast de bascule

- [ ] **Step 1: tests**
- [ ] **Step 2: code**
";

#[test]
fn the_plan_of_this_repository_gives_its_title_tasks_and_steps() {
    let plan = parse_plan(REPO_PLAN);
    assert_eq!(plan.title.as_deref(), Some("Escouade 1.7"));
    assert_eq!(ids(&plan), ["L1", "K4"]);
    assert_eq!(
        plan.tasks[0].title,
        "Mécanique de la fenêtre (catalogues typés, `t`, pluriels, `Rich`, formats, garde-fous)"
    );
    // The steps of a task are those of its section: the boxes under « Tasks L3 à L9 » are its
    // neighbour's, they end L1's.
    assert_eq!(plan.tasks[0].steps, Some((2, 3)));
    assert_eq!(plan.tasks[1].steps, Some((0, 2)));
    assert!(plan
        .tasks
        .iter()
        .all(|t| t.status == TaskStatus::Pending && t.blocked_by.is_empty() && t.active.is_none()));
}

#[test]
fn a_plan_with_numbered_tasks_as_writing_plans_models_it() {
    let plan = parse_plan(
        "# Démo Implementation Plan\n\n**Goal:** one hook\n\n### Task 1: Hook installer\n\n**Files:**\n- Create: `a.py`\n\n- [ ] **Step 1: Write the failing test**\n\n```python\ndef test_install():\n    assert install()\n```\n\n- [ ] **Step 2: Run it to verify it fails**\n\nRun: `pytest -v`\n\n### Task 2: Recovery\n\n- [ ] **Step 1: Write the test**\n",
    );
    assert_eq!(plan.title.as_deref(), Some("Démo"));
    assert_eq!(ids(&plan), ["1", "2"]);
    assert_eq!(plan.tasks[0].title, "Hook installer");
    assert_eq!(plan.tasks[0].steps, Some((0, 2)));
    assert_eq!(plan.tasks[1].steps, Some((0, 1)));
}

#[test]
fn a_task_without_boxes_has_no_steps() {
    let plan = parse_plan("# P\n\n### Task 1: Seule\n\nUn texte.\n");
    assert_eq!(plan.tasks[0].steps, None);
}

#[test]
fn headings_in_code_blocks_are_not_read() {
    let plan = parse_plan(
        "```bash\n# a comment, not the title\n### Task 9: not a task\n```\n\n# Le vrai titre\n\n````markdown\nAn example plan:\n```\n### Task 8: still inside\n```\n- [x] not a step\n````\n\n~~~\n### Task 7: tilde fence\n~~~\n\n### Task 1: Réelle\n\n- [ ] one\n",
    );
    assert_eq!(plan.title.as_deref(), Some("Le vrai titre"));
    assert_eq!(ids(&plan), ["1"]);
    assert_eq!(plan.tasks[0].steps, Some((0, 1)));
}

#[test]
fn a_box_in_a_code_block_is_not_a_step() {
    let plan = parse_plan(
        "# P\n\n### Task 1: A\n\n- [ ] real\n\n```\n- [ ] in code\n- [x] in code\n```\n",
    );
    assert_eq!(plan.tasks[0].steps, Some((0, 1)));
}

#[test]
fn checked_boxes_are_counted_in_all_the_ways_markdown_writes_them() {
    let plan = parse_plan(
        "# P\n\n### Task 1: A\n\n- [x] a\n- [X] b\n* [x] c\n+ [ ] d\n  - [x] nested\n- [ ] e\n-[x] no space, not a box\n- [x]no space after, not a box\n- [y] not a box\n- [] not a box\n",
    );
    assert_eq!(plan.tasks[0].steps, Some((4, 6)));
}

#[test]
fn an_empty_text_and_a_text_without_tasks_give_no_list() {
    assert_eq!(parse_plan(""), ParsedPlan::default());
    let titled = parse_plan("# Seulement un titre\n\nDu texte.\n\n## Une section\n");
    assert_eq!(titled.title.as_deref(), Some("Seulement un titre"));
    assert!(titled.tasks.is_empty());
    let bare = parse_plan("no heading at all\n- [ ] a box\n");
    assert_eq!(bare, ParsedPlan::default());
}

#[test]
fn the_title_is_the_first_h1_and_only_a_h1() {
    let plan = parse_plan("#hashtag\n\n## Section\n\n#  Premier \n\n# Second\n\n### Task 1: A\n");
    assert_eq!(plan.title.as_deref(), Some("Premier"));
}

#[test]
fn the_implementation_plan_suffix_goes_from_the_title() {
    for (h1, want) in [
        ("Démo Implementation Plan", "Démo"),
        ("Démo implementation plan", "Démo"),
        ("Démo — plan d'implémentation", "Démo"),
        ("Démo – Plan d’implémentation", "Démo"),
        ("Démo: Implementation Plan", "Démo"),
        (
            "Implementation Plan for caching",
            "Implementation Plan for caching",
        ),
        // Nothing left once it is gone: the title stays as written.
        ("Implementation Plan", "Implementation Plan"),
        ("Hooks et plans", "Hooks et plans"),
    ] {
        assert_eq!(
            parse_plan(&format!("# {h1}\n")).title.as_deref(),
            Some(want),
            "{h1}"
        );
    }
}

#[test]
fn a_title_is_one_line_cut_at_160_characters() {
    let long = "é".repeat(300);
    let plan = parse_plan(&format!("# {long}\n\n### Task 1: {long}\n"));
    let title = plan.title.unwrap();
    assert_eq!(title.chars().count(), 160);
    assert!(title.ends_with('…'));
    assert_eq!(plan.tasks[0].title.chars().count(), 160);
    // Characters that are not drawn do not get in.
    let tricky = parse_plan("# Safe\u{202e}evil\u{200b}\t title\n");
    assert_eq!(tricky.title.as_deref(), Some("Safeevil title"));
}

#[test]
fn a_task_heading_is_one_line_the_lines_after_it_do_not_join_its_title() {
    let plan = parse_plan(
        "# P\n\n### Task 1: Un titre qui\ncontinue sur la ligne suivante\n\n- [ ] a\n\n### Task 2: Suivante ###\n",
    );
    assert_eq!(plan.tasks[0].title, "Un titre qui");
    // Closing hashes of an ATX heading are not part of its title.
    assert_eq!(plan.tasks[1].title, "Suivante");
    assert_eq!(plan.tasks[0].steps, Some((0, 1)));
}

#[test]
fn the_ids_and_separators_the_task_heading_allows() {
    let plan = parse_plan(
        "## Task 1: deux dièses\n### Task L2 — tiret long\n#### Task K3 - tiret court\n### Task 12. point\n### Task 3b: lettre\n### Task P1 : espace avant\n",
    );
    assert_eq!(ids(&plan), ["1", "L2", "K3", "12", "3b", "P1"]);
    assert_eq!(plan.tasks[1].title, "tiret long");
    assert_eq!(plan.tasks[2].title, "tiret court");
    assert_eq!(plan.tasks[3].title, "point");
    assert_eq!(plan.tasks[5].title, "espace avant");
}

#[test]
fn what_is_not_a_task_heading_is_not_one() {
    let plan = parse_plan(
        "# Task 1: un titre\n##### Task 2: trop profond\n### task 3: minuscule\n### Task: sans numéro\n### Tasks 4 à 6: plusieurs\n### Task 7\n### Task A: sans chiffre\n  ### Task 8: indenté\nTask 9: pas un titre\n### Task 10:\n",
    );
    assert!(plan.tasks.is_empty(), "{:?}", ids(&plan));
}

#[test]
fn a_task_id_met_twice_is_the_first() {
    let plan = parse_plan(
        "### Task 1: Première\n- [x] a\n### Task 1: Doublon\n- [ ] b\n- [ ] c\n### Task 2: B\n",
    );
    assert_eq!(ids(&plan), ["1", "2"]);
    assert_eq!(plan.tasks[0].title, "Première");
    // The boxes of the repeated heading are not the first's.
    assert_eq!(plan.tasks[0].steps, Some((1, 1)));
}

#[test]
fn a_huge_plan_gives_a_hundred_tasks_at_most() {
    let mut text = String::from("# Énorme\n");
    // Ten thousand lines: three hundred tasks of thirty lines.
    for n in 1..=333 {
        text.push_str(&format!("\n### Task {n}: Tâche {n}\n\n"));
        for step in 0..28 {
            text.push_str(&format!("- [ ] étape {step}\n"));
        }
    }
    assert!(text.lines().count() > 10_000);
    let plan = parse_plan(&text);
    assert_eq!(plan.tasks.len(), 100);
    assert_eq!(plan.tasks[99].id, "100");
    assert_eq!(plan.tasks[99].steps, Some((0, 28)));
}

#[test]
fn a_byte_order_mark_and_windows_line_endings_change_nothing() {
    let crlf = format!("\u{feff}{}", REPO_PLAN.replace('\n', "\r\n"));
    assert_eq!(parse_plan(&crlf), parse_plan(REPO_PLAN));
    let plan = parse_plan(&crlf);
    assert_eq!(plan.title.as_deref(), Some("Escouade 1.7"));
    assert_eq!(
        plan.tasks[1].title,
        "Choix du compte, pause quand tous ont passé le seuil, toast de bascule"
    );
}

// ---------- parse_ledger ----------

/// Lines the real scripts and skills write (`task-done`, SDD's fix rounds, parked findings).
const LEDGER: &str = "# SDD ledger — plan: docs/superpowers/plans/demo.md
Task 1: complete (commits a1b2c3d..d4e5f6a, tests: npm test -- hooks → 1/1 pass)
Task 2: Ruling: install_hook → installHook — matches Task 1 Produces — cost if wrong: one rename
Task 2: fix round 1/5 (2 addressed, 0 open; commits d4e5f6a..b7c8d9e)
Task 3: parked — blocked by the upstream change
Task 4: minor (deferred): rename the helper
Task 5: complete (commits a..b, tests: pytest → 8 passed, 2 parked)
Final: minor (deferred): hardcoded interval
";

#[test]
fn the_lines_of_the_real_scripts_say_what_is_done_and_what_is_under_way() {
    let ledger = parse_ledger(LEDGER);
    assert_eq!(
        ledger.plan.as_deref(),
        Some("docs/superpowers/plans/demo.md")
    );
    assert_eq!(ledger.done, set(&["1", "5"]));
    assert_eq!(ledger.active, set(&["2"]));
}

#[test]
fn the_ledger_of_this_repository_with_its_free_lines() {
    let ledger = parse_ledger(
        "# SDD ledger — plan: docs/superpowers/plans/2026-10-10-escouade-1-7.md

Spec: docs/TODO-1.7.md (reachable). Briefs: `bash brief.sh <ID>`.

| Tasks | Shared file / interface | Finding |
|---|---|---|
| L1 → L2..L12 | `t()`, `Rich` | OK |

Task L1: dispatched (opus) in v17-socle, BASE 8b43aa0
Task S1: dispatched (sonnet) in v17-site, BASE 8b43aa0
Task S1: implementer DONE_WITH_CONCERNS (commits d66b961..fe50524); review dispatched (sonnet)
Task S1: complete (commits 8b43aa0..fe50524, review clean — Approved, minors only)
Task S1: minor (deferred → carried into S2): fr.ts:112
Task L1: implementer DONE_WITH_CONCERNS (commits 794f7c5..a84a2e0); review dispatched (opus).
Task S2: dispatched (sonnet) in v17-site, BASE fe50524, with S1 minors carried in
Task L1: review Needs fixes (1 Important). Fix round 1 dispatched (resumed implementer), FIX_BASE a84a2e0.
Task L1: fix round 1/5 (9 addressed, 0 open; commits a84a2e0..ac5adac)
Task L1: complete (commits 8b43aa0..ac5adac, review clean)
Task L1: minor (deferred → carried into L2): hardcoded-allow.ts
Task S2: polish round 1 (5 addressed, 0 open; commits e44c3d4..1fa7ae9) — re-review APPROVED
Task L2: dispatched (opus) in v17-socle, BASE ac5adac
Task L6: implementer DONE_WITH_CONCERNS (commits ec72333..947d04a, 147 keys); review dispatched (sonnet).
Task L6: complete (commits ac5adac..947d04a, review clean — Approved, minors only)
Task L2: implementer DONE (commits 115abff..2f973e8); review dispatched (opus).
",
    );
    assert_eq!(ledger.done, set(&["L1", "L6", "S1"]));
    // S2 has no line of work in the words the skills use for the last one (« polish round »): it
    // was dispatched, and nothing says it is over.
    assert_eq!(ledger.active, set(&["L2", "S2"]));
}

#[test]
fn a_line_of_work_after_complete_reopens_the_task_and_complete_after_it_closes_it() {
    let reopened = parse_ledger(
        "# SDD ledger — plan: p.md\nTask 1: complete (x)\nTask 1: fix round 2/5 (final review)\n",
    );
    assert!(reopened.done.is_empty());
    assert_eq!(reopened.active, set(&["1"]));
    let closed = parse_ledger(
        "# SDD ledger — plan: p.md\nTask 1: dispatched\nTask 1: complete (x)\nTask 1: minor (deferred): y\n",
    );
    assert_eq!(closed.done, set(&["1"]));
    assert!(closed.active.is_empty());
}

#[test]
fn only_the_words_of_the_skills_count_as_work_and_as_done() {
    let ledger = parse_ledger(
        "# SDD ledger — plan: p.md\nTask 1: re-review dispatched\nTask 2: reviewer found nothing\nTask 3: completed (by hand)\nTask 4: completely redone\nTask 5: fix this later\nTask 6: Fix round 1/5\nTask 7: fix-round 1\nTask 8: implementer\nTask 9: dispatched\n",
    );
    // The capital letter changes nothing; « fix-round » is not what the skills write (« fix
    // round »), and a line in free words says nothing.
    assert_eq!(ledger.done, set(&["3"]));
    assert_eq!(ledger.active, set(&["1", "2", "6", "8", "9"]));
}

#[test]
fn ledger_lines_that_are_not_task_lines_have_no_effect() {
    let ledger = parse_ledger(
        "# SDD ledger — plan: p.md\n- Task 1: complete\n Task 2: complete\nTask 3:complete\nTask  4: complete\ntask 5: complete\nTask : complete\nTask 6 complete\nTask 7:\nNote: Task 8: complete\n",
    );
    assert!(ledger.done.is_empty(), "{:?}", ledger.done);
    assert!(ledger.active.is_empty());
}

#[test]
fn ids_are_compared_as_they_are_written() {
    let ledger = parse_ledger("# SDD ledger — plan: p.md\nTask 1: complete\nTask 10: dispatched\nTask L1: complete\nTask l1: dispatched\n");
    assert_eq!(ledger.done, set(&["1", "L1"]));
    assert_eq!(ledger.active, set(&["10", "l1"]));
}

#[test]
fn a_ledger_without_its_first_line_names_no_plan() {
    for text in [
        "",
        "\n",
        "Task 1: complete\n",
        "# Some other notes\nTask 1: complete\n",
        "# SDD ledger\nTask 1: complete\n",
        "# SDD ledger — plan:\nTask 1: complete\n",
        "# SDD ledger — plan:   \nTask 1: complete\n",
    ] {
        let ledger = parse_ledger(text);
        assert_eq!(ledger.plan, None, "{text:?}");
    }
    // Its lines are read all the same: the caller decides what a ledger without a plan is.
    assert_eq!(parse_ledger("Task 1: complete\n").done, set(&["1"]));
}

#[test]
fn the_ledger_of_a_windows_machine_reads_the_same() {
    let text = LEDGER.replace('\n', "\r\n");
    let ledger = parse_ledger(&format!("\u{feff}{text}"));
    assert_eq!(
        ledger.plan.as_deref(),
        Some("docs/superpowers/plans/demo.md")
    );
    assert_eq!(ledger.done, set(&["1", "5"]));
    assert_eq!(ledger.active, set(&["2"]));
}

#[test]
fn the_plan_a_ledger_names_is_cleaned() {
    for (line, want) in [
        ("# SDD ledger — plan: a/b.md", "a/b.md"),
        ("# SDD ledger - plan: a/b.md", "a/b.md"),
        ("#   SDD ledger — plan:   a/b.md  ", "a/b.md"),
        ("# SDD ledger — plan: `a/b.md`", "a/b.md"),
        ("# SDD ledger — plan: \"a b/c.md\"", "a b/c.md"),
        (
            "# SDD ledger — plan: C:\\repo\\docs\\p.md",
            "C:\\repo\\docs\\p.md",
        ),
    ] {
        assert_eq!(parse_ledger(line).plan.as_deref(), Some(want), "{line}");
    }
}

// ---------- the disk ----------

/// A folder with the plan of `name` and its workspace the way `sdd-workspace` makes it: the plan's
/// path in `plan-path`, the ledger, a self-ignoring `.gitignore`.
struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new(name: &str) -> Self {
        let root = test_dir(&format!("p2-{name}")).join("repo");
        std::fs::create_dir_all(&root).unwrap();
        Repo { root }
    }

    fn put(&self, rel: &str, text: &str) -> PathBuf {
        let path = self.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn plan(&self, rel: &str) {
        self.put(
            rel,
            "# Démo Implementation Plan\n\n### Task 1: Un\n\n- [x] a\n- [ ] b\n\n### Task 2: Deux\n\n- [ ] c\n\n### Task 3: Trois\n\n- [ ] d\n",
        );
    }

    /// The workspace of the plan at `plan_rel`, with its ledger saying `lines` after the first.
    fn workspace(&self, slug: &str, plan_rel: &str, lines: &[&str]) -> PathBuf {
        self.put(".superpowers/sdd/.gitignore", "*\n");
        self.put(
            &format!(".superpowers/sdd/{slug}/plan-path"),
            &format!("{plan_rel}\n"),
        );
        let mut ledger = format!("# SDD ledger — plan: {plan_rel}\n");
        for line in lines {
            ledger.push_str(line);
            ledger.push('\n');
        }
        self.put(&format!(".superpowers/sdd/{slug}/progress.md"), &ledger)
    }

    fn brief(&self, slug: &str, id: &str) {
        self.put(
            &format!(".superpowers/sdd/{slug}/task-{id}-brief.md"),
            "### Task\n",
        );
    }

    fn load(&self) -> Option<FileList> {
        load(&self.root, &[])
    }
}

fn statuses(list: &FileList) -> Vec<(&str, TaskStatus)> {
    list.tasks
        .iter()
        .map(|t| (t.id.as_str(), t.status))
        .collect()
}

fn age(path: &Path, seconds: u64) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(seconds))
        .unwrap();
}

#[test]
fn a_workspace_gives_done_under_way_and_to_do() {
    let r = Repo::new("basic");
    r.plan("docs/superpowers/plans/demo.md");
    r.workspace(
        "demo",
        "docs/superpowers/plans/demo.md",
        &["Task 1: complete (commits a..b, tests: npm test → ok)"],
    );
    r.brief("demo", "1");
    r.brief("demo", "2");
    let list = r.load().expect("the plan");
    assert_eq!(list.plan_file, "docs/superpowers/plans/demo.md");
    assert_eq!(list.title.as_deref(), Some("Démo"));
    // Task 1 is done (its brief is still there), 2 has a brief and no complete, 3 has none.
    assert_eq!(
        statuses(&list),
        [
            ("1", TaskStatus::Done),
            ("2", TaskStatus::InProgress),
            ("3", TaskStatus::Pending)
        ]
    );
    assert_eq!(list.tasks[0].steps, Some((1, 2)));
    assert_eq!(list.tasks[1].steps, Some((0, 1)));
}

#[test]
fn a_brief_alone_is_work_under_way_and_a_complete_line_ends_it() {
    let r = Repo::new("brief-rule");
    r.plan("docs/superpowers/plans/demo.md");
    r.workspace("demo", "docs/superpowers/plans/demo.md", &[]);
    assert_eq!(
        statuses(&r.load().unwrap()),
        [
            ("1", TaskStatus::Pending),
            ("2", TaskStatus::Pending),
            ("3", TaskStatus::Pending)
        ]
    );
    r.brief("demo", "3");
    assert_eq!(r.load().unwrap().tasks[2].status, TaskStatus::InProgress);
    r.workspace(
        "demo",
        "docs/superpowers/plans/demo.md",
        &["Task 3: complete (x)"],
    );
    assert_eq!(r.load().unwrap().tasks[2].status, TaskStatus::Done);
    // The briefs of tasks the plan does not have, and files that are no brief, say nothing.
    r.brief("demo", "9");
    r.put(".superpowers/sdd/demo/task-1-report.md", "done\n");
    r.put(".superpowers/sdd/demo/task-2-brief.md.bak", "x\n");
    r.put(".superpowers/sdd/demo/brief-2.md", "x\n");
    let list = r.load().unwrap();
    assert_eq!(list.tasks[0].status, TaskStatus::Pending);
    assert_eq!(list.tasks[1].status, TaskStatus::Pending);
}

#[test]
fn tasks_of_the_l1_style_ids_are_followed_the_same_way() {
    let r = Repo::new("l-ids");
    r.put(
        "docs/superpowers/plans/big.md",
        "# Big — plan d'implémentation\n\n### Task L1: Langues\n\n### Task K4: Comptes\n\n### Task K5: Barre\n",
    );
    r.workspace(
        "big",
        "docs/superpowers/plans/big.md",
        &[
            "Task L1: dispatched (opus) in v17-socle, BASE 8b43aa0",
            "Task L1: complete (commits 1..2, review clean)",
            "Task K4: implementer DONE (commits 3..4); review dispatched",
        ],
    );
    let list = r.load().unwrap();
    assert_eq!(list.title.as_deref(), Some("Big"));
    assert_eq!(
        statuses(&list),
        [
            ("L1", TaskStatus::Done),
            ("K4", TaskStatus::InProgress),
            ("K5", TaskStatus::Pending)
        ]
    );
}

#[test]
fn the_plan_may_be_named_absolutely_with_either_slash() {
    let r = Repo::new("absolute");
    let plan = r.put("docs/superpowers/plans/demo.md", "# P\n\n### Task 1: Un\n");
    for named in [
        plan.to_string_lossy().into_owned(),
        plan.to_string_lossy().replace('\\', "/"),
        "./docs/superpowers/plans/demo.md".into(),
        "docs\\superpowers\\plans\\demo.md".into(),
    ] {
        r.workspace("demo", &named, &[]);
        let list = r.load().unwrap_or_else(|| panic!("not found: {named}"));
        assert_eq!(list.plan_file, "docs/superpowers/plans/demo.md", "{named}");
    }
}

#[test]
fn a_plan_outside_the_repository_is_refused() {
    let r = Repo::new("outside");
    let outside = r.root.parent().unwrap().join("elsewhere.md");
    std::fs::write(&outside, "# Hors dépôt\n\n### Task 1: Secret\n").unwrap();
    for named in [
        outside.to_string_lossy().into_owned(),
        "../elsewhere.md".into(),
        "docs/../../elsewhere.md".into(),
        "/etc/passwd".into(),
        "C:\\Windows\\win.ini".into(),
        // Another machine's: refused without asking the network.
        "\\\\p2-no-such-server\\share\\plan.md".into(),
        "//p2-no-such-server/share/plan.md".into(),
    ] {
        // The marker names it too: the two are refused.
        r.put(".superpowers/sdd/x/plan-path", &format!("{named}\n"));
        r.put(
            ".superpowers/sdd/x/progress.md",
            &format!("# SDD ledger — plan: {named}\nTask 1: complete\n"),
        );
        assert!(r.load().is_none(), "{named}");
        assert!(discover(&r.root).is_none(), "{named}");
    }
}

#[test]
fn only_a_markdown_file_is_a_plan() {
    let r = Repo::new("not-md");
    r.put("src/secret.env", "# KEY\n\n### Task 1: leak\n");
    r.workspace("x", "src/secret.env", &[]);
    assert!(r.load().is_none());
}

#[test]
fn a_plan_that_is_a_link_out_of_the_repository_is_refused() {
    let r = Repo::new("link-plan");
    let outside = r.root.parent().unwrap().join("outside.md");
    std::fs::write(&outside, "# Hors dépôt\n\n### Task 1: Secret\n").unwrap();
    std::fs::create_dir_all(r.root.join("docs/superpowers/plans")).unwrap();
    if !make_file_link(
        &outside,
        &r.root
            .join("docs")
            .join("superpowers")
            .join("plans")
            .join("link.md"),
    ) {
        eprintln!("no symbolic link here: skipped");
        return;
    }
    r.workspace("link", "docs/superpowers/plans/link.md", &[]);
    assert!(r.load().is_none());
    assert!(fallback(&r.root, &["docs/superpowers/plans/link.md".into()]).is_none());
}

#[test]
fn a_ledger_that_is_a_link_out_of_the_repository_is_not_read() {
    let r = Repo::new("link-ledger");
    r.plan("docs/superpowers/plans/demo.md");
    let outside = r.root.parent().unwrap().join("ledger-out.md");
    std::fs::write(
        &outside,
        "# SDD ledger — plan: docs/superpowers/plans/demo.md\nTask 1: complete\n",
    )
    .unwrap();
    std::fs::create_dir_all(r.root.join(".superpowers/sdd/demo")).unwrap();
    if !make_file_link(
        &outside,
        &r.root
            .join(".superpowers")
            .join("sdd")
            .join("demo")
            .join("progress.md"),
    ) {
        eprintln!("no symbolic link here: skipped");
        return;
    }
    assert!(discover(&r.root).is_none());
}

#[test]
fn a_workspace_folder_or_a_superpowers_folder_that_is_a_link_is_not_followed() {
    let r = Repo::new("link-dir");
    r.plan("docs/superpowers/plans/demo.md");
    // A whole workspace elsewhere.
    let elsewhere = r.root.parent().unwrap().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(
        elsewhere.join("progress.md"),
        "# SDD ledger — plan: docs/superpowers/plans/demo.md\nTask 1: complete\n",
    )
    .unwrap();
    std::fs::create_dir_all(r.root.join(".superpowers/sdd")).unwrap();
    if !make_dir_link(
        &elsewhere,
        &r.root.join(".superpowers").join("sdd").join("demo"),
    ) {
        eprintln!("no link to a folder here: skipped");
        return;
    }
    assert!(discover(&r.root).is_none());
    // The `.superpowers` folder itself.
    std::fs::remove_dir_all(r.root.join(".superpowers")).unwrap();
    let sdd = r.root.parent().unwrap().join("sdd-out");
    std::fs::create_dir_all(sdd.join("sdd/demo")).unwrap();
    std::fs::write(
        sdd.join("sdd/demo/progress.md"),
        "# SDD ledger — plan: docs/superpowers/plans/demo.md\nTask 1: complete\n",
    )
    .unwrap();
    if !make_dir_link(&sdd, &r.root.join(".superpowers")) {
        eprintln!("no link to a folder here: skipped");
        return;
    }
    assert!(discover(&r.root).is_none());
}

#[test]
fn of_two_workspaces_the_most_recent_ledger_is_the_one() {
    let r = Repo::new("two");
    r.plan("docs/superpowers/plans/old.md");
    r.put(
        "docs/superpowers/plans/new.md",
        "# Nouveau\n\n### Task 1: N1\n\n### Task 2: N2\n",
    );
    let old = r.workspace(
        "old",
        "docs/superpowers/plans/old.md",
        &["Task 1: complete"],
    );
    let new = r.workspace("new", "docs/superpowers/plans/new.md", &[]);
    age(&old, 3600);
    age(&new, 60);
    assert_eq!(r.load().unwrap().plan_file, "docs/superpowers/plans/new.md");
    // The other way round.
    age(&old, 5);
    age(&new, 3600);
    assert_eq!(r.load().unwrap().plan_file, "docs/superpowers/plans/old.md");
}

#[test]
fn a_newer_workspace_that_cannot_be_used_leaves_the_place_to_the_next() {
    let r = Repo::new("skip");
    r.plan("docs/superpowers/plans/good.md");
    let good = r.workspace(
        "good",
        "docs/superpowers/plans/good.md",
        &["Task 1: complete"],
    );
    age(&good, 600);
    // Newer, but its ledger has no first line…
    let headless = r.put(
        ".superpowers/sdd/headless/progress.md",
        "Task 1: complete\n",
    );
    age(&headless, 300);
    // …names a plan that is gone and has no marker…
    let gone = r.workspace("gone", "docs/superpowers/plans/gone.md", &[]);
    std::fs::remove_file(r.root.join(".superpowers/sdd/gone/plan-path")).unwrap();
    age(&gone, 120);
    // …or has no ledger at all.
    r.put(
        ".superpowers/sdd/bare/plan-path",
        "docs/superpowers/plans/good.md\n",
    );
    r.brief("bare", "2");
    let list = r.load().expect("the good one");
    assert_eq!(list.plan_file, "docs/superpowers/plans/good.md");
    assert_eq!(list.tasks[0].status, TaskStatus::Done);
}

#[test]
fn a_ledger_for_another_plan_than_its_marker_follows_the_plan_it_names() {
    let r = Repo::new("other");
    r.plan("docs/superpowers/plans/a.md");
    r.put("docs/superpowers/plans/b.md", "# B\n\n### Task 1: Seule\n");
    r.workspace("a", "docs/superpowers/plans/a.md", &[]);
    // The ledger's first line is the plan it follows; the marker is the fallback when that path
    // does not lead to a plan of the repository.
    r.put(
        ".superpowers/sdd/a/progress.md",
        "# SDD ledger — plan: docs/superpowers/plans/b.md\nTask 1: complete\n",
    );
    let list = r.load().unwrap();
    assert_eq!(list.plan_file, "docs/superpowers/plans/b.md");
    assert_eq!(list.tasks.len(), 1);
    assert_eq!(list.tasks[0].status, TaskStatus::Done);
    // A path it wrote relative to somewhere else: the marker (repo-relative) finds the plan.
    r.put(
        ".superpowers/sdd/a/progress.md",
        "# SDD ledger — plan: ../docs/superpowers/plans/a.md\nTask 2: complete\n",
    );
    let list = r.load().unwrap();
    assert_eq!(list.plan_file, "docs/superpowers/plans/a.md");
    assert_eq!(list.tasks[1].status, TaskStatus::Done);
}

#[test]
fn no_workspace_no_superpowers_folder_a_plan_without_tasks() {
    let r = Repo::new("none");
    assert!(r.load().is_none());
    assert!(discover(&r.root).is_none());
    // A folder that is no folder of this repository.
    assert!(load(&r.root.join("nowhere"), &[]).is_none());
    // `.superpowers/sdd` is there and empty; a file is where a workspace should be.
    r.put(".superpowers/sdd/.gitignore", "*\n");
    r.put(".superpowers/sdd/stray.md", "x\n");
    assert!(r.load().is_none());
    // A plan without task headings is a list of none, found all the same.
    r.put("docs/superpowers/plans/empty.md", "# Rien\n\nDu texte.\n");
    r.workspace(
        "empty",
        "docs/superpowers/plans/empty.md",
        &["Task 1: complete"],
    );
    let list = r.load().unwrap();
    assert!(list.tasks.is_empty());
    assert_eq!(list.title.as_deref(), Some("Rien"));
}

#[test]
fn a_ledger_without_a_plan_or_with_a_plan_deleted_gives_nothing() {
    let r = Repo::new("deleted");
    r.plan("docs/superpowers/plans/demo.md");
    r.workspace(
        "demo",
        "docs/superpowers/plans/demo.md",
        &["Task 1: complete"],
    );
    assert!(r.load().is_some());
    std::fs::remove_file(r.root.join("docs/superpowers/plans/demo.md")).unwrap();
    assert!(r.load().is_none());
    assert!(discover(&r.root).is_none());
}

#[test]
fn a_file_of_a_megabyte_is_cut_never_refused() {
    let r = Repo::new("huge");
    // A plan: its tasks, then a megabyte of text, then a heading nobody reads.
    let mut plan = String::from("# Gros\n\n### Task 1: Avant\n\n- [ ] a\n\n");
    plan.push_str(&"du texte de remplissage qui ne finit pas\n".repeat(30_000));
    plan.push_str("\n### Task 2: Après la borne\n");
    assert!(plan.len() > 1024 * 1024);
    r.put("docs/superpowers/plans/gros.md", &plan);
    // A ledger: its first line, a megabyte of notes, then the line that closes Task 1.
    let mut ledger = String::from("# SDD ledger — plan: docs/superpowers/plans/gros.md\n");
    ledger.push_str("Task 1: dispatched (sonnet)\n");
    ledger.push_str(
        &"Note libre qui remplit le registre, sans effet sur les tâches\n".repeat(20_000),
    );
    ledger.push_str("Task 1: complete (the line at the end)\n");
    assert!(ledger.len() > 1024 * 1024);
    r.put(".superpowers/sdd/gros/progress.md", &ledger);
    let list = r.load().expect("cut, not refused");
    assert_eq!(
        list.tasks.len(),
        1,
        "the heading past the bound is not read"
    );
    assert_eq!(list.tasks[0].title, "Avant");
    // The end of a ledger is what tells how far the plan is: it is read.
    assert_eq!(list.tasks[0].status, TaskStatus::Done);
}

#[test]
fn a_ledger_cut_in_the_middle_keeps_its_first_line_and_its_last_lines() {
    let r = Repo::new("cut");
    r.plan("docs/superpowers/plans/demo.md");
    let mut ledger = String::from("# SDD ledger — plan: docs/superpowers/plans/demo.md\n");
    ledger.push_str("Task 1: complete (early)\n");
    ledger.push_str(&"x".repeat(MAX_READ));
    ledger.push_str("\nTask 3: complete (in the middle)\n");
    ledger.push_str(&"x".repeat(MAX_READ));
    ledger.push_str("\nTask 2: complete (late)\nTask 9: dispat");
    r.put(".superpowers/sdd/demo/progress.md", &ledger);
    let list = r.load().unwrap();
    assert_eq!(list.tasks[0].status, TaskStatus::Done, "the head is read");
    assert_eq!(list.tasks[1].status, TaskStatus::Done, "the tail is read");
    // There is no room for the middle: it is not read.
    assert_eq!(list.tasks[2].status, TaskStatus::Pending);
}

#[test]
fn bytes_that_are_not_text_do_not_fail_the_read() {
    let r = Repo::new("binary");
    r.plan("docs/superpowers/plans/demo.md");
    let mut ledger = b"# SDD ledger \xe2\x80\x94 plan: docs/superpowers/plans/demo.md\n".to_vec();
    ledger.extend_from_slice(b"\xff\xfe\x00 garbage\nTask 2: complete\n");
    let path = r.root.join(".superpowers/sdd/demo/progress.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, ledger).unwrap();
    let list = r.load().unwrap();
    assert_eq!(list.tasks[1].status, TaskStatus::Done);
}

// ---------- a plan the agent wrote ----------

#[test]
fn a_plan_the_agent_wrote_is_followed_without_a_ledger() {
    let r = Repo::new("written");
    r.plan("docs/superpowers/plans/2026-10-10-demo.md");
    let list = load(
        &r.root,
        &["docs/superpowers/plans/2026-10-10-demo.md".into()],
    )
    .unwrap();
    assert_eq!(list.plan_file, "docs/superpowers/plans/2026-10-10-demo.md");
    assert_eq!(list.title.as_deref(), Some("Démo"));
    assert!(list.tasks.iter().all(|t| t.status == TaskStatus::Pending));
    assert_eq!(list.tasks[0].steps, Some((1, 2)));
    // As the tool wrote it: absolute, or relative.
    let abs = r.root.join("docs/superpowers/plans/2026-10-10-demo.md");
    for named in [
        abs.to_string_lossy().into_owned(),
        abs.to_string_lossy().replace('\\', "/"),
    ] {
        assert!(
            load(&r.root, std::slice::from_ref(&named)).is_some(),
            "{named}"
        );
    }
    // Nothing written, nothing found.
    assert!(load(&r.root, &[]).is_none());
}

#[test]
fn the_ledger_comes_before_a_plan_the_agent_wrote() {
    let r = Repo::new("ledger-first");
    r.plan("docs/superpowers/plans/followed.md");
    r.plan("docs/superpowers/plans/written.md");
    r.workspace(
        "followed",
        "docs/superpowers/plans/followed.md",
        &["Task 1: complete"],
    );
    let list = load(&r.root, &["docs/superpowers/plans/written.md".into()]).unwrap();
    assert_eq!(list.plan_file, "docs/superpowers/plans/followed.md");
}

#[test]
fn only_a_plan_of_the_plans_folder_is_read_without_a_ledger() {
    let r = Repo::new("written-only-plans");
    r.put("README.md", "# Lisez-moi\n\n### Task 1: Pas un plan\n");
    r.put("docs/notes.md", "# Notes\n\n### Task 1: Pas un plan\n");
    r.put(
        "docs/superpowers/specs/spec.md",
        "# Spec\n\n### Task 1: Pas un plan\n",
    );
    r.put(
        "docs/superpowers/plans/notes.txt",
        "# Texte\n\n### Task 1: Pas un plan\n",
    );
    r.put("src/secret.env", "# Clé\n\n### Task 1: Pas un plan\n");
    let outside = r
        .root
        .parent()
        .unwrap()
        .join("docs/superpowers/plans/out.md");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(&outside, "# Dehors\n\n### Task 1: Pas un plan\n").unwrap();
    for named in [
        "README.md",
        "docs/notes.md",
        "docs/superpowers/specs/spec.md",
        "docs/superpowers/plans/notes.txt",
        "src/secret.env",
        "docs/superpowers/plans/missing.md",
        "docs/superpowers/plans/../../../README.md",
        "../docs/superpowers/plans/out.md",
        "",
        "docs/superpowers/plans/",
    ] {
        assert!(
            fallback(&r.root, &[named.to_string()]).is_none(),
            "{named:?}"
        );
    }
    let abs_outside = outside.to_string_lossy().into_owned();
    assert!(fallback(&r.root, &[abs_outside]).is_none());
}

#[test]
fn of_several_plans_written_the_latest_that_exists_is_followed() {
    let r = Repo::new("written-many");
    r.plan("docs/superpowers/plans/a.md");
    r.plan("docs/superpowers/plans/b.md");
    let written: Vec<String> = [
        "docs/superpowers/plans/a.md",
        "docs/superpowers/plans/b.md",
        "docs/superpowers/plans/c.md",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    // c.md was written and is gone: b.md is the latest that is there.
    assert_eq!(
        load(&r.root, &written).unwrap().plan_file,
        "docs/superpowers/plans/b.md"
    );
}

// ---------- scan ----------

fn read(scan: Scan) -> (Stamp, FileList) {
    match scan {
        Scan::Read(stamp, list) => (stamp, list),
        other => panic!("not read: {other:?}"),
    }
}

#[test]
fn a_scan_reads_once_and_then_only_what_moved() {
    let r = Repo::new("scan");
    r.plan("docs/superpowers/plans/demo.md");
    let ledger = r.workspace("demo", "docs/superpowers/plans/demo.md", &[]);
    let (stamp, list) = read(scan(&r.root, &[], 0, None));
    assert_eq!(list.tasks[0].status, TaskStatus::Pending);
    // Nothing moved.
    assert_eq!(scan(&r.root, &[], 0, Some(&stamp)), Scan::Unchanged);
    // The ledger grew.
    let mut text = std::fs::read_to_string(&ledger).unwrap();
    text.push_str("Task 1: complete\n");
    std::fs::write(&ledger, text).unwrap();
    let (stamp, list) = read(scan(&r.root, &[], 0, Some(&stamp)));
    assert_eq!(list.tasks[0].status, TaskStatus::Done);
    assert_eq!(scan(&r.root, &[], 0, Some(&stamp)), Scan::Unchanged);
    // A brief came.
    r.brief("demo", "2");
    let (stamp, list) = read(scan(&r.root, &[], 0, Some(&stamp)));
    assert_eq!(list.tasks[1].status, TaskStatus::InProgress);
    // The plan was edited (a box ticked).
    let plan = r.root.join("docs/superpowers/plans/demo.md");
    // (Longer, so that a clock that ticks slowly sees it too.)
    let text = std::fs::read_to_string(&plan)
        .unwrap()
        .replace("- [ ] c", "- [x] c, done");
    std::fs::write(&plan, text).unwrap();
    let (stamp, list) = read(scan(&r.root, &[], 0, Some(&stamp)));
    assert_eq!(list.tasks[1].steps, Some((1, 1)));
    assert_eq!(scan(&r.root, &[], 0, Some(&stamp)), Scan::Unchanged);
}

#[test]
fn a_scan_with_nothing_to_read_says_so() {
    let r = Repo::new("scan-none");
    assert_eq!(scan(&r.root, &[], 0, None), Scan::Nothing);
    let (stamp, _) = {
        r.plan("docs/superpowers/plans/demo.md");
        r.workspace("demo", "docs/superpowers/plans/demo.md", &[]);
        read(scan(&r.root, &[], 0, None))
    };
    // The workspace goes: what was read is not found any more.
    std::fs::remove_dir_all(r.root.join(".superpowers")).unwrap();
    assert_eq!(scan(&r.root, &[], 0, Some(&stamp)), Scan::Nothing);
}

#[test]
fn a_workspace_the_agent_has_not_worked_on_since_its_conversation_began_is_not_its_own() {
    let r = Repo::new("since");
    r.plan("docs/superpowers/plans/demo.md");
    let ledger = r.workspace(
        "demo",
        "docs/superpowers/plans/demo.md",
        &["Task 1: complete"],
    );
    age(&ledger, 3600);
    let now = || {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    };
    // A ledger of an hour ago, a conversation of a minute ago: someone else's, or an old one's.
    assert_eq!(scan(&r.root, &[], now() - 60_000, None), Scan::Nothing);
    // The conversation began before: it is read.
    assert!(matches!(
        scan(&r.root, &[], now() - 7_200_000, None),
        Scan::Read(..)
    ));
    // A brief written since is work on it, whatever date the ledger has.
    r.brief("demo", "2");
    let (_, list) = read(scan(&r.root, &[], now() - 60_000, None));
    assert_eq!(list.tasks[1].status, TaskStatus::InProgress);
    assert_eq!(list.tasks[0].status, TaskStatus::Done);
}

#[test]
fn a_stale_ledger_does_not_hide_a_plan_written_since() {
    let r = Repo::new("since-written");
    r.plan("docs/superpowers/plans/old.md");
    let ledger = r.workspace(
        "old",
        "docs/superpowers/plans/old.md",
        &["Task 1: complete"],
    );
    age(&ledger, 3600);
    r.plan("docs/superpowers/plans/new.md");
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let written = ["docs/superpowers/plans/new.md".to_string()];
    let (_, list) = read(scan(&r.root, &written, now - 60_000, None));
    assert_eq!(list.plan_file, "docs/superpowers/plans/new.md");
}

// ---------- fix round 1 ----------

#[test]
fn a_path_on_another_machine_or_a_device_is_remote_whatever_the_separators() {
    let remote = [
        r"\\host\share\x.md",
        "//host/share/x.md",
        r"\/host/share/x.md",
        r"/\host\share\x.md",
        r"\\?\UNC\host\share\x.md",
        r"\\.\pipe\x",
        r"\/./pipe/x",
        r"/\./pipe/x",
        r"\/?/C:/x.md",
        r"/\?\C:\x.md",
        r"\\?\C:\repo\x.md",
        r"\??\UNC\host\share\x.md",
        "/??/UNC/host/share/x.md",
        "///host/x",
        r"\\\host\x",
    ];
    for root in [r"C:\repo", "C:/repo", "/home/me/repo"] {
        for path in remote {
            assert!(is_remote_path(path, root), "{path} (root {root})");
        }
        for local in [
            r"C:\repo\x.md",
            "C:/repo/x.md",
            "/home/me/repo/x.md",
            "docs/x.md",
            "./docs/x.md",
            r"docs\x.md",
            "x.md",
            "..",
            "",
        ] {
            assert!(!is_remote_path(local, root), "{local} (root {root})");
        }
    }
    // A repository that is itself on the network: its own files are not remote.
    for root in [r"\\srv\share\repo", "//srv/share/repo", r"\/srv/share/repo"] {
        for own in [r"\\srv\share\repo\docs\x.md", "//srv/share/repo/docs/x.md"] {
            assert!(!is_remote_path(own, root), "{own} (root {root})");
        }
    }
}

#[test]
fn a_remote_or_device_path_is_refused_before_it_is_compared_to_the_root() {
    let r = Repo::new("remote-rel");
    let root = r.root.to_string_lossy().into_owned();
    // The root's own files, spelled with a verbatim prefix: a device path all the same.
    let verbatim = format!(r"\\?\{root}\docs\superpowers\plans\demo.md");
    assert_eq!(relative_to(&r.root, &verbatim), None);
    // The same, spelled normally.
    assert_eq!(
        relative_to(&r.root, &format!("{root}/docs/superpowers/plans/demo.md")).as_deref(),
        Some("docs/superpowers/plans/demo.md")
    );
    for raw in [
        r"\\host\share\x.md",
        r"\/host/share/x.md",
        r"/\host\share\x.md",
        r"\/./pipe/x.md",
        r"\/?/C:/x.md",
        r"\??\UNC\host\share\x.md",
    ] {
        assert_eq!(relative_to(&r.root, raw), None, "{raw}");
    }
}

#[test]
fn a_path_not_lexically_under_the_root_is_refused_without_asking_the_disk() {
    let r = Repo::new("lexical");
    r.plan("docs/superpowers/plans/demo.md");
    // A second name for the repository (a link): the disk would say it is the same folder; the
    // spelling is not the root's, so it is not looked at.
    let alias = r.root.parent().unwrap().join("alias");
    if !make_dir_link(&r.root, &alias) {
        eprintln!("no link to a folder here: skipped");
        return;
    }
    let through = alias
        .join("docs")
        .join("superpowers")
        .join("plans")
        .join("demo.md");
    let through = through.to_string_lossy().into_owned();
    assert_eq!(relative_to(&r.root, &through), None);
    assert!(fallback(&r.root, std::slice::from_ref(&through)).is_none());
    // Rooted where the repository is, it is the plan.
    let rooted = reroot(&r.root, &alias, std::slice::from_ref(&through));
    assert!(fallback(&r.root, &rooted).is_some(), "{rooted:?}");
    // A folder that is not the repository's second name changes nothing.
    let other = r.root.join("docs");
    let same = reroot(&r.root, &other, std::slice::from_ref(&through));
    assert_eq!(same, std::slice::from_ref(&through));
    // Nor does a path outside the folder it is rooted from.
    let strange = vec!["/etc/passwd".to_string(), "docs/x.md".to_string()];
    assert_eq!(reroot(&r.root, &alias, &strange), strange);
}

#[test]
fn a_stream_or_a_device_name_is_not_a_plan() {
    let r = Repo::new("device");
    r.put("docs/superpowers/plans/demo.md", "# P\n\n### Task 1: Un\n");
    for named in [
        "docs/superpowers/plans/demo.md:stream.md",
        "docs/superpowers/plans/demo.md::$DATA",
        "docs/superpowers/plans/CON.md",
        "docs/superpowers/plans/nul.md",
        "docs/superpowers/plans/Com1.md",
        "docs/superpowers/plans/lpt9.sub.md",
        "docs/superpowers/aux/demo.md",
        "C:demo.md",
    ] {
        assert_eq!(relative_to(&r.root, named), None, "{named}");
    }
    // Names that only look like them.
    for named in [
        "docs/superpowers/plans/console.md",
        "docs/superpowers/plans/com0.md",
        "docs/superpowers/plans/com10.md",
        "docs/superpowers/plans/nullable.md",
    ] {
        assert!(relative_to(&r.root, named).is_some(), "{named}");
    }
}

#[test]
fn every_statement_of_a_ledger_line_counts() {
    // Lines of this repository's ledger, as the controller writes them.
    let ledger = parse_ledger(
        "# SDD ledger — plan: p.md
Task M4: complete (commits 9871993..10666e6); Task M3: complete (commits ba31a2f..9871993, paired review approved)
Task G2: complete (commits 47a577b..da35091); Task G3: complete (commits da35091..94b8f93) — paired review Approved (minors)
Task K3: fix round (commits db57e97..19616e8: sign-in stamp baseline, re-login) — to be covered by the K5+K6 paired review. Task K3: complete pending that check; Task K4: complete (commits 4e9eac1..3efd2b7).
Task K5: complete; Task K6: complete (paired review Approved; K3 fix round approved in the same review → Task K3: complete)
Task K7: hardening (commits 751d8ba..cde19fb) — accepted without re-review; Task K7: complete (commits 9812b79..cde19fb). Chantier K complete.
Task L2: dispatched (opus); Task L3: dispatched (sonnet)
",
    );
    assert_eq!(
        ledger.done,
        set(&["M3", "M4", "G2", "G3", "K3", "K4", "K5", "K6", "K7"])
    );
    assert_eq!(ledger.active, set(&["L2", "L3"]));
    // The last statement about a task wins, within a line as between lines.
    let later = parse_ledger(
        "# SDD ledger — plan: p.md\nTask 1: complete; Task 1: fix round 2/5 (final review)\nTask 2: dispatched; Task 2: complete\n",
    );
    assert_eq!(later.done, set(&["2"]));
    assert_eq!(later.active, set(&["1"]));
}

#[test]
fn a_mention_of_a_task_is_not_a_statement() {
    let ledger = parse_ledger(
        "# SDD ledger — plan: p.md
Task 2: Ruling: install_hook → installHook — matches Task 1 Produces — cost if wrong: one rename
Task 3: review Needs fixes (see the note on Task 4)
Note: Task 5: complete
See Task 6: complete
Task 7: minor (deferred): the Task 8: complete wording
",
    );
    // A task named in a sentence is not a statement; one after a full stop, a semicolon, an
    // arrow or a dash is. A line that does not start with a task is a note.
    assert!(ledger.done.is_empty(), "{:?}", ledger.done);
    assert_eq!(ledger.active, set(&["3"]));
}

#[test]
fn the_newest_workspaces_are_kept_not_the_first_the_disk_lists() {
    let r = Repo::new("many");
    r.plan("docs/superpowers/plans/demo.md");
    // Three hundred workspaces whose plan is gone, older; the one that works is the newest and
    // sorts last by name, where a listing in name order stops before it.
    for n in 0..300u64 {
        let ledger = r.workspace(
            &format!("a{n:03}"),
            "docs/superpowers/plans/gone.md",
            &["Task 1: complete"],
        );
        std::fs::remove_file(ledger.parent().unwrap().join("plan-path")).unwrap();
        age(&ledger, 3600 + n);
    }
    let good = r.workspace(
        "zzz",
        "docs/superpowers/plans/demo.md",
        &["Task 1: complete"],
    );
    age(&good, 5);
    let found = discover(&r.root).expect("the newest, past the 256th");
    assert_eq!(found.plan_rel, "docs/superpowers/plans/demo.md");
}
