//! What a project runs in its agents' worktrees: its setup once one is made (dependencies,
//! generated code…), its teardown before one is removed, and the commands Claude suggests for both
//! from what it reads of the project.

use crate::model::{Worktree, WorktreeStep};
use crate::paths;
use crate::pty::ShellInfo;
use crate::testlaunch;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

/// How long a setup command may run: an install from scratch takes its time.
pub const SETUP_LIMIT: Duration = Duration::from_secs(20 * 60);
/// How long a teardown command may run: the removal waits for it.
pub const TEARDOWN_LIMIT: Duration = Duration::from_secs(5 * 60);
/// How long Claude may read the project to suggest the commands.
pub const SUGGEST_LIMIT: Duration = Duration::from_secs(4 * 60);
/// At most this many steps of each kind are taken from a suggestion.
const MAX_SUGGESTED: usize = 8;

/// The variables a worktree's commands get: where the project and the worktree are, its branch,
/// and its reserved ports when it has some.
pub fn step_env(project: &str, wt: &Worktree, port_base: Option<u16>) -> Vec<(String, String)> {
    let mut env = vec![
        ("ESCOUADE_PROJECT_DIR".to_string(), project.to_string()),
        ("ESCOUADE_WORKTREE_DIR".to_string(), wt.path.clone()),
        ("ESCOUADE_BRANCH".to_string(), wt.branch.clone()),
    ];
    env.extend(testlaunch::port_env(port_base));
    env
}

/// The steps that have something to run.
pub fn runnable(steps: &[WorktreeStep]) -> Vec<WorktreeStep> {
    steps
        .iter()
        .filter(|s| !s.command.trim().is_empty())
        .cloned()
        .collect()
}

/// A step as it is shown: its command's first line, with its folder.
pub fn label(step: &WorktreeStep) -> String {
    let line = step
        .command
        .trim()
        .lines()
        .next()
        .unwrap_or_default()
        .trim();
    let line = if line.chars().count() > 80 {
        format!("{}…", line.chars().take(79).collect::<String>())
    } else {
        line.to_string()
    };
    match step.cwd.trim() {
        "" | "." => line,
        dir => format!("{line} ({dir})"),
    }
}

/// A step that failed: which one, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct StepFailure {
    pub command: String,
    /// Its exit code, or why it did not run or end.
    pub reason: String,
    /// Its last lines.
    pub tail: String,
}

impl StepFailure {
    /// In a sentence: `what` ("le démontage du worktree") failed on which command, and how.
    pub fn summary(&self, what: &str) -> String {
        format!("{what} a échoué sur `{}` ({}).", self.command, self.reason)
    }

    /// As the conversation and the agent are told: `what` ("La préparation du worktree") failed,
    /// with the last lines it wrote.
    pub fn describe(&self, what: &str) -> String {
        let head = self.summary(what);
        let tail = testlaunch::tail_lines(self.tail.trim(), 30);
        if tail.trim().is_empty() {
            head
        } else {
            format!("{head}\n\n```\n{tail}\n```")
        }
    }
}

/// Runs `step` in the worktree at `root` (in its folder there), with the shell it names or else
/// the system's first one, `env` added, `limit` at most.
pub async fn run_step(
    step: &WorktreeStep,
    shells: &[ShellInfo],
    root: &str,
    env: &[(String, String)],
    limit: Duration,
) -> Result<(), StepFailure> {
    let command = step.command.trim();
    let fail = |reason: String, tail: String| StepFailure {
        command: label(step),
        reason,
        tail,
    };
    let shell = shells
        .iter()
        .find(|s| s.id == step.shell)
        .or_else(|| shells.first())
        .ok_or_else(|| fail("aucun shell détecté".into(), String::new()))?;
    let dir = step.cwd.trim();
    let cwd = if dir.is_empty() || dir == "." {
        root.to_string()
    } else {
        match paths::contained(Path::new(root), dir) {
            Ok(p) if p.is_dir() => p.to_string_lossy().into_owned(),
            Ok(_) => {
                return Err(fail(
                    format!("le dossier « {dir} » n'existe pas dans le worktree"),
                    String::new(),
                ))
            }
            Err(e) => return Err(fail(format!("{e:#}"), String::new())),
        }
    };
    match testlaunch::run_command(shell, &cwd, command, env, limit).await {
        Ok(Some(run)) if run.passed => Ok(()),
        Ok(Some(run)) => Err(fail(
            run.code
                .map(|c| format!("code {c}"))
                .unwrap_or_else(|| "arrêtée".into()),
            run.tail,
        )),
        Ok(None) => Err(fail(
            format!("pas finie en {} min", (limit.as_secs() / 60).max(1)),
            String::new(),
        )),
        Err(e) => Err(fail(format!("ne démarre pas : {e:#}"), String::new())),
    }
}

// ---------- suggested by Claude ----------

/// The setup and teardown Claude suggests for a project's worktrees.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct WorktreeSuggestion {
    pub setup: Vec<WorktreeStep>,
    pub teardown: Vec<WorktreeStep>,
}

/// Claude's role when it suggests a project's worktree commands.
pub const SUGGEST_SYSTEM: &str = "Tu lis un projet pour préparer les commandes qu'Escouade lance dans ses worktrees git. Tu ne modifies rien et n'exécutes rien : tu lis les fichiers, puis tu réponds uniquement par le bloc JSON demandé.";

/// What Claude is asked: the setup and teardown commands of the project's worktrees, for `shell`
/// (as named to the user), knowing what is copied into them and whether isola runs them.
pub fn suggest_prompt(shell: &str, copied: &[String], isola: bool) -> String {
    let copied = copied
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let copied = if copied.is_empty() {
        "aucun fichier ignoré par git".to_string()
    } else {
        format!("seulement les fichiers ignorés par git qui correspondent à {copied}, copiés depuis le projet")
    };
    let isola = if isola {
        " Le projet a un .isola.toml : isola lance lui-même ses services et ses étapes « setup » (isola up), ne les répète pas."
    } else {
        ""
    };
    format!(
        "<worktrees>\nChaque agent d'Escouade travaille dans un nouveau worktree git de ce projet : un checkout neuf de la branche, \
         sans dépendances installées ni rien de généré, avec {copied}.{isola}\n\n\
         Lis le projet (manifestes et lockfiles, README, CONTRIBUTING, Makefile, scripts, docker-compose… à la racine et dans les sous-dossiers) \
         et donne :\n\
         - « preparation » : les commandes qui rendent un worktree neuf prêt à développer et tester, dans l'ordre : installer les dépendances \
         avec le gestionnaire du lockfile (npm ci, pnpm install --frozen-lockfile, yarn install --immutable, bundle install, uv sync, poetry install, \
         composer install, go mod download…) dans chaque dossier qui en a, puis ce que le build ou les tests exigent d'avance (code généré, \
         client Prisma…). Jamais de serveur ni de commande qui ne se termine pas, ni de tests, ni de build qui n'est pas nécessaire.\n\
         - « demontage » : seulement ce qu'il faut défaire hors du worktree avant de le supprimer (base de données ou conteneurs créés \
         pour lui) ; le plus souvent rien.\n\n\
         Les commandes tournent dans {shell}, chacune dans le dossier « dossier » (relatif à la racine du worktree, vide pour la racine). \
         Variables disponibles : ESCOUADE_PROJECT_DIR (dossier du projet principal), ESCOUADE_WORKTREE_DIR, ESCOUADE_BRANCH.\n\n\
         Réponds uniquement par :\n```json\n{{\"preparation\": [{{\"commande\": \"npm ci\", \"dossier\": \"\"}}], \"demontage\": []}}\n```\n</worktrees>"
    )
}

/// The blocks of `text` that may hold the answer: its fenced blocks, last first, then the text
/// itself from its first `{` to its last `}`.
fn candidates(text: &str) -> Vec<&str> {
    let mut blocks: Vec<&str> = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("```") {
        let after = &rest[start + 3..];
        // The fence's language, up to the end of its line.
        let body_start = after.find('\n').map(|i| i + 1).unwrap_or(after.len());
        let body = &after[body_start..];
        match body.find("```") {
            Some(end) => {
                blocks.push(&body[..end]);
                rest = &body[end + 3..];
            }
            None => break,
        }
    }
    blocks.reverse();
    if let (Some(a), Some(b)) = (text.find('{'), text.rfind('}')) {
        if a < b {
            blocks.push(&text[a..=b]);
        }
    }
    blocks
}

/// The steps of a list of the answer, each run by `shell`: those with a command whose folder
/// stays inside `root` (the project's folder: the worktree's is the same tree).
fn steps_of(list: Option<&Value>, root: &Path, shell: &str) -> Vec<WorktreeStep> {
    list.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let text = |keys: &[&str]| {
                keys.iter()
                    .find_map(|k| s.get(*k).and_then(Value::as_str))
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            };
            let command = match s {
                Value::String(c) => c.trim().to_string(),
                _ => text(&["commande", "command"]),
            };
            if command.is_empty() {
                return None;
            }
            let dir = text(&["dossier", "dir", "cwd"]);
            let dir = dir.trim_start_matches("./").trim_end_matches(['/', '\\']);
            let cwd = match dir {
                "" | "." => String::new(),
                d if paths::contained(root, d).is_ok() => d.to_string(),
                _ => return None,
            };
            Some(WorktreeStep {
                id: uuid::Uuid::new_v4().to_string(),
                command,
                shell: shell.to_string(),
                cwd,
            })
        })
        .take(MAX_SUGGESTED)
        .collect()
}

/// The setup and teardown in Claude's `answer` (its JSON object with "preparation" and
/// "demontage"), each step run by `shell`; None when no block of it reads as such.
pub fn parse_suggestion(
    answer: &str,
    root: &Path,
    shell: &str,
) -> Option<(Vec<WorktreeStep>, Vec<WorktreeStep>)> {
    candidates(answer).into_iter().find_map(|block| {
        let v: Value = serde_json::from_str(block.trim()).ok()?;
        let setup = v.get("preparation").or_else(|| v.get("setup"));
        let teardown = v.get("demontage").or_else(|| v.get("teardown"));
        if setup.is_none() && teardown.is_none() {
            return None;
        }
        Some((
            steps_of(setup, root, shell),
            steps_of(teardown, root, shell),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    fn step(command: &str, cwd: &str) -> WorktreeStep {
        WorktreeStep {
            id: "s".into(),
            command: command.into(),
            shell: String::new(),
            cwd: cwd.into(),
        }
    }

    /// The system's first shell, as the app finds it.
    fn shells() -> Vec<ShellInfo> {
        crate::pty::detect_shells(&crate::model::Settings::default())
    }

    #[test]
    fn a_step_shows_its_commands_first_line_and_its_folder() {
        assert_eq!(label(&step("  npm ci  ", "")), "npm ci");
        assert_eq!(label(&step("npm ci\nnpm run gen", "web")), "npm ci (web)");
        let long = "x".repeat(100);
        assert_eq!(label(&step(&long, ".")).chars().count(), 80);
        assert!(runnable(&[step(" ", ""), step("npm ci", "")]).len() == 1);
    }

    #[test]
    fn a_worktrees_commands_know_the_project_the_worktree_its_branch_and_its_ports() {
        let wt = Worktree {
            path: "/p/.claude/worktrees/dem-1".into(),
            branch: "ticket/dem-1".into(),
            base_branch: "main".into(),
        };
        let env = step_env("/p", &wt, Some(4110));
        let get = |k: &str| env.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("ESCOUADE_PROJECT_DIR"), Some("/p"));
        assert_eq!(
            get("ESCOUADE_WORKTREE_DIR"),
            Some("/p/.claude/worktrees/dem-1")
        );
        assert_eq!(get("ESCOUADE_BRANCH"), Some("ticket/dem-1"));
        assert_eq!(get("ESCOUADE_PORT_BASE"), Some("4110"));
        assert!(step_env("/p", &wt, None)
            .iter()
            .all(|(k, _)| k != "ESCOUADE_PORT_BASE"));
    }

    #[tokio::test]
    async fn a_step_runs_in_its_folder_with_the_variables_and_its_failure_says_why() {
        let root = test_dir("wt-step");
        std::fs::create_dir_all(root.join("web")).unwrap();
        let r = root.to_string_lossy().to_string();
        let env = vec![("ESCOUADE_BRANCH".to_string(), "feat".to_string())];
        let limit = Duration::from_secs(60);
        // Written by node, the same on every system.
        let write = step(
            "node -e \"require('fs').writeFileSync('out.txt', process.env.ESCOUADE_BRANCH)\"",
            "web",
        );
        run_step(&write, &shells(), &r, &env, limit).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("web").join("out.txt")).unwrap(),
            "feat"
        );
        let fails = step(
            "node -e \"console.log('ligne utile'); process.exit(3)\"",
            "",
        );
        let e = run_step(&fails, &shells(), &r, &env, limit)
            .await
            .unwrap_err();
        assert_eq!(e.reason, "code 3");
        assert!(e.tail.contains("ligne utile"), "{e:?}");
        let described = e.describe("La préparation du worktree");
        assert!(
            described.starts_with("La préparation du worktree a échoué sur `node -e"),
            "{described}"
        );
        assert!(described.contains("(code 3).\n\n```\n"), "{described}");
        let missing = run_step(&step("npm ci", "absent"), &shells(), &r, &env, limit)
            .await
            .unwrap_err();
        assert_eq!(
            missing.reason,
            "le dossier « absent » n'existe pas dans le worktree"
        );
        let outside = run_step(&step("npm ci", "../ailleurs"), &shells(), &r, &env, limit)
            .await
            .unwrap_err();
        assert!(outside.reason.contains("hors du dossier"), "{outside:?}");
    }

    #[tokio::test]
    async fn a_step_that_does_not_end_in_time_is_stopped() {
        let root = test_dir("wt-step-slow");
        let r = root.to_string_lossy().to_string();
        let slow = step("node -e \"setTimeout(() => {}, 30000)\"", "");
        let started = std::time::Instant::now();
        let e = run_step(&slow, &shells(), &r, &[], Duration::from_secs(2))
            .await
            .unwrap_err();
        assert_eq!(e.reason, "pas finie en 1 min");
        assert!(started.elapsed() < Duration::from_secs(20));
    }

    #[test]
    fn the_suggestion_is_read_from_the_answers_json_and_its_folders_stay_inside() {
        let root = test_dir("wt-suggest");
        std::fs::create_dir_all(root.join("web")).unwrap();
        let answer = "J'ai lu le projet.\n\n```json\n{\"preparation\": [\
            {\"commande\": \"npm ci\", \"dossier\": \"\"},\
            {\"commande\": \"npm run gen\", \"dossier\": \"./web/\"},\
            {\"commande\": \"rm -rf /\", \"dossier\": \"../dehors\"},\
            {\"commande\": \"  \", \"dossier\": \"web\"},\
            \"cargo fetch\"],\
            \"demontage\": [{\"commande\": \"docker compose down\", \"dossier\": null}]}\n```";
        let (setup, teardown) = parse_suggestion(answer, &root, "bash").unwrap();
        let got: Vec<(&str, &str, &str)> = setup
            .iter()
            .map(|s| (s.command.as_str(), s.cwd.as_str(), s.shell.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                ("npm ci", "", "bash"),
                ("npm run gen", "web", "bash"),
                ("cargo fetch", "", "bash"),
            ]
        );
        assert!(setup.iter().all(|s| !s.id.is_empty()) && setup[0].id != setup[1].id);
        assert_eq!(teardown.len(), 1);
        assert_eq!(teardown[0].command, "docker compose down");
        // Bare JSON, an empty teardown, English keys.
        let (s, t) = parse_suggestion(
            "{\"setup\": [{\"command\": \"uv sync\"}], \"teardown\": []}",
            &root,
            "pwsh",
        )
        .unwrap();
        assert_eq!((s[0].command.as_str(), t.len()), ("uv sync", 0));
        // The last block that reads as an answer wins.
        let two = "```json\n{\"preparation\": [\"a\"]}\n```\npuis\n```json\n{\"preparation\": [\"b\"]}\n```";
        assert_eq!(
            parse_suggestion(two, &root, "bash").unwrap().0[0].command,
            "b"
        );
        assert!(parse_suggestion("Je ne sais pas.", &root, "bash").is_none());
        assert!(parse_suggestion("```json\n{\"autre\": 1}\n```", &root, "bash").is_none());
    }

    #[test]
    fn the_question_names_the_shell_the_copied_files_and_isola() {
        let p = suggest_prompt("PowerShell 7", &[".env*".into(), " ".into()], false);
        assert!(p.contains("PowerShell 7") && p.contains(".env*"), "{p}");
        assert!(!p.contains("isola"));
        assert!(suggest_prompt("bash", &[], true).contains(".isola.toml"));
        assert!(suggest_prompt("bash", &[], false).contains("aucun fichier ignoré par git"));
    }
}
