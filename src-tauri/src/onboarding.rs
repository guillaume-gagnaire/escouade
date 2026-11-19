//! The first launch's journey (ESC-504, ON-1, `docs/ONBOARDING.md` § 5): its stations in their order, where it stands on
//! this installation (`model::Onboarding`, kept in `state.json`), and what each station's status is.
//!
//! The journey reads the real state of the app, it keeps no truth of its own: a station is done when what it sets is set
//! (an account added, the mode of the new agents chosen, a project, an answer to the statistics), wherever it was set
//! (Réglages too), and it stays done once it was, even if that changes later. What the journey keeps is the user's own
//! gestures: a station passed (« Plus tard », « Tout passer »), and where it ended.
//!
//! It opens by itself on a new installation alone (`Core::fresh`, no project): never on a device already set up, unless
//! the user opens it again (« Refaire le premier lancement »). Opened and left half-way, it opens again at the next start,
//! at the first station neither done nor passed.

use crate::core::Core;
use crate::model::{AgentStatus, Onboarding, OnboardingStep, StepOutcome};
use escouade_policy::list::{DEFAULT_MODE, TELEMETRY};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The version of the journey: a later one that changes a station raises it.
pub const VERSION: u32 = 1;

/// The stations, in their order (§ 4).
pub const STEPS: [OnboardingStep; 6] = [
    OnboardingStep::Harness,
    OnboardingStep::Account,
    OnboardingStep::Defaults,
    OnboardingStep::Project,
    OnboardingStep::Agent,
    OnboardingStep::Telemetry,
];

/// What the journey reads of the app (`Core::onboarding_facts`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// A new installation (`Core::fresh`).
    pub fresh: bool,
    /// The accounts of the harnesses.
    pub accounts: usize,
    /// The mode of the new agents is chosen (`Settings::agent_mode`).
    pub mode_chosen: bool,
    pub projects: usize,
    /// An agent has ended a turn well (ESC-505): its first message sent by the user, its answer come.
    pub first_reply: bool,
    /// The statistics' question is answered (`Settings::telemetry`).
    pub telemetry_answered: bool,
    /// What a policy imposes: the mode of the new agents, the statistics.
    pub mode_imposed: bool,
    pub telemetry_imposed: bool,
}

/// Where a station stands.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StepStatus {
    Todo,
    Done,
    Skipped,
    /// Imposed by a policy: done, locked, shown to say who imposes it.
    Policy,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StepView {
    pub id: OnboardingStep,
    pub status: StepStatus,
}

/// Why the journey is open.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Trigger {
    /// A new installation, opened at its first start.
    First,
    /// Left half-way, opened again at a later start.
    Resume,
    /// Opened again from Aide or Réglages on a device already set up.
    Redo,
}

/// The journey as the window shows it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingView {
    /// To show now; and why.
    pub open: bool,
    pub trigger: Option<Trigger>,
    pub steps: Vec<StepView>,
    /// The station it goes on at: the first neither done nor passed (the first of all when it is done again).
    pub resume_at: Option<OnboardingStep>,
    /// Opened once at least on this installation.
    pub started: bool,
    pub finished: bool,
    /// The statistics' question passed in the journey, to ask once more at this start.
    pub ask_telemetry_again: bool,
    /// Where the demonstration project lives (`demo::dir`), made or not: the window knows it by its folder (its band).
    pub demo: String,
}

/// A gesture of the window on the journey.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Act {
    /// A station done (« Continuer ») or passed (« Plus tard »).
    Step {
        step: OnboardingStep,
        outcome: StepOutcome,
    },
    /// « Tout passer »: every station not done is passed, and the journey ends.
    SkipAll,
    /// The last station passed: the journey ends.
    Finish,
    /// « Refaire le premier lancement »: opened again, every station shown from the first; nothing set is undone.
    Redo,
    /// The statistics' question passed in the journey asked once more: never again.
    TelemetryAskedAgain,
}

/// Whether what `facts` say makes `step` done, wherever it was set.
pub fn real(step: OnboardingStep, facts: &Facts) -> bool {
    match step {
        // A harness is chosen once an account of it is added: a harness found alone is not chosen yet.
        OnboardingStep::Harness | OnboardingStep::Account => facts.accounts > 0,
        OnboardingStep::Defaults => facts.mode_chosen,
        OnboardingStep::Project => facts.projects > 0,
        OnboardingStep::Agent => facts.first_reply,
        OnboardingStep::Telemetry => facts.telemetry_answered,
    }
}

/// Whether a policy imposes what `step` sets.
fn imposed(step: OnboardingStep, facts: &Facts) -> bool {
    match step {
        OnboardingStep::Defaults => facts.mode_imposed,
        OnboardingStep::Telemetry => facts.telemetry_imposed,
        _ => false,
    }
}

/// Where `step` stands, from what the user did (`saved`) and the real state.
pub fn status(step: OnboardingStep, saved: &Onboarding, facts: &Facts) -> StepStatus {
    if imposed(step, facts) {
        return StepStatus::Policy;
    }
    match saved.steps.get(&step) {
        _ if real(step, facts) => StepStatus::Done,
        Some(StepOutcome::Done) => StepStatus::Done,
        Some(StepOutcome::Skipped) => StepStatus::Skipped,
        None => StepStatus::Todo,
    }
}

/// The stations made done by the real state since, kept as done (a station done stays done, § 5.2). Whether any was.
pub fn settle(saved: &mut Onboarding, facts: &Facts) -> bool {
    if saved.started.is_none() {
        return false;
    }
    let mut changed = false;
    for step in STEPS {
        if real(step, facts) && saved.steps.get(&step) != Some(&StepOutcome::Done) {
            saved.steps.insert(step, StepOutcome::Done);
            changed = true;
        }
    }
    changed
}

/// Whether the journey opens by itself at this start: a new installation with no project that never saw it.
pub fn opens_first(saved: &Onboarding, facts: &Facts) -> bool {
    saved.started.is_none() && facts.fresh && facts.projects == 0
}

/// The journey as the window shows it. `window`: the desktop's window (a client of the remote API never has it shown).
pub fn view(saved: &Onboarding, facts: &Facts, window: bool) -> OnboardingView {
    let steps: Vec<StepView> = STEPS
        .iter()
        .map(|&id| StepView {
            id,
            status: status(id, saved, facts),
        })
        .collect();
    let pending = steps
        .iter()
        .find(|s| s.status == StepStatus::Todo)
        .map(|s| s.id);
    let under_way = saved.started.is_some() && saved.finished.is_none();
    let trigger = if !window {
        None
    } else if saved.redo && under_way {
        Some(Trigger::Redo)
    } else if under_way && pending.is_some() {
        Some(if saved.steps.is_empty() {
            Trigger::First
        } else {
            Trigger::Resume
        })
    } else if opens_first(saved, facts) {
        Some(Trigger::First)
    } else {
        None
    };
    let resume_at = match trigger {
        Some(Trigger::Redo) => Some(STEPS[0]),
        _ => pending,
    };
    let ask_telemetry_again = window
        && trigger.is_none()
        && saved.finished.is_some()
        && saved.steps.get(&OnboardingStep::Telemetry) == Some(&StepOutcome::Skipped)
        && !facts.telemetry_answered
        && !facts.telemetry_imposed
        && !saved.telemetry_asked_again;
    OnboardingView {
        open: trigger.is_some(),
        trigger,
        steps,
        resume_at,
        started: saved.started.is_some(),
        finished: saved.finished.is_some(),
        ask_telemetry_again,
        demo: String::new(),
    }
}

/// `act` done on `saved`, at `now` (ms).
pub fn apply(saved: &mut Onboarding, act: &Act, facts: &Facts, now: i64) {
    saved.version = VERSION;
    saved.started.get_or_insert(now);
    match act {
        Act::Step { step, outcome } => {
            // A station done stays done: « Plus tard » on a station shown again never undoes it.
            if !(*outcome == StepOutcome::Skipped
                && (saved.steps.get(step) == Some(&StepOutcome::Done) || real(*step, facts)))
            {
                saved.steps.insert(*step, *outcome);
            }
        }
        Act::SkipAll => {
            for step in STEPS {
                if status(step, saved, facts) == StepStatus::Todo {
                    saved.steps.insert(step, StepOutcome::Skipped);
                }
            }
            saved.finished = Some(now);
            saved.redo = false;
        }
        Act::Finish => {
            saved.finished = Some(now);
            saved.redo = false;
        }
        Act::Redo => {
            saved.finished = None;
            saved.redo = true;
        }
        Act::TelemetryAskedAgain => saved.telemetry_asked_again = true,
    }
    settle(saved, facts);
}

// === The first project (ESC-505, § 4.4) ===

/// What the station of the first project reads of a folder before it is opened.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FolderCheck {
    pub exists: bool,
    /// Escouade can write in it (a project's git, its worktrees, its agents' changes need it).
    pub writable: bool,
    /// It has a test command Escouade recognizes, that can run here: « Lance les tests et dis-moi ce qui casse » is
    /// proposed then (a `package.json`'s test only when Node is on the machine).
    pub tests: bool,
}

/// Whether Escouade can write in `dir`: a file of its own made and removed at once (the only sure way, the read-only
/// flag of a folder saying nothing under Windows).
pub fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(
        ".escouade-ecriture-{}",
        uuid::Uuid::new_v4().simple()
    ));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(f) => {
            drop(f);
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Whether the program `name` is on the `PATH`.
pub fn on_path(name: &str) -> bool {
    let ends: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path)
            .any(|d| ends.iter().any(|e| d.join(format!("{name}{e}")).is_file()))
    })
}

/// Whether `dir` has a test command Escouade recognizes; a `package.json`'s counts when `node` is there to run it.
pub fn has_tests(dir: &Path, node: bool) -> bool {
    const BUILDS: [&str; 7] = [
        "Cargo.toml",
        "pyproject.toml",
        "setup.py",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
    ];
    if BUILDS.iter().any(|f| dir.join(f).is_file()) {
        return true;
    }
    let Ok(text) = std::fs::read_to_string(dir.join("package.json")) else {
        return false;
    };
    let test = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v["scripts"]["test"].as_str().map(str::to_owned))
        .unwrap_or_default();
    // npm's own placeholder is no test command.
    node && !test.trim().is_empty() && !test.contains("no test specified")
}

/// `dir` read for the station.
pub fn check_folder(dir: &Path, node: bool) -> FolderCheck {
    let exists = dir.is_dir();
    FolderCheck {
        exists,
        writable: exists && writable(dir),
        tests: exists && has_tests(dir, node),
    }
}

/// How a clone of the station went wrong: what the window says, and lets try again.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CloneFailure {
    /// `offline`: the host could not be reached; `access`: it refused; `place`: the folder; `address`: the address;
    /// `error`: anything else git said.
    pub kind: String,
    pub detail: String,
}

/// What the station's « Cloner » gives: the folder of the clone, or why there is none.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Cloned {
    pub path: Option<String>,
    pub failure: Option<CloneFailure>,
}

fn failed(kind: &str, detail: String) -> Cloned {
    Cloned {
        path: None,
        failure: Some(CloneFailure {
            kind: kind.into(),
            detail,
        }),
    }
}

/// git could not reach the host: no network, no name resolution.
pub fn offline_in(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    [
        "could not resolve host",
        "could not resolve hostname",
        "temporary failure in name resolution",
        "name or service not known",
        "failed to connect",
        "network is unreachable",
        "no route to host",
        "connection timed out",
        "couldn't connect to server",
    ]
    .iter()
    .any(|w| s.contains(w))
}

/// The address typed, checked as a team's list checks one (https or ssh, no local path, no credential in it); why not.
pub fn clone_address(url: &str) -> Result<escouade_team::catalog::Address, String> {
    use escouade_team::catalog::{address, BadAddress};
    address(url).map_err(|e| match e {
        BadAddress::Empty => tr!(
            "Donne l’adresse du dépôt : https://… ou git@….",
            "Give the repository’s address: https://… or git@…."
        ),
        BadAddress::Local => tr!(
            "C’est un dossier, pas une adresse : choisis « Un dossier de ta machine ».",
            "That’s a folder, not an address: pick “A folder on your machine”."
        ),
        BadAddress::Credential => tr!(
            "Pas de mot de passe ni de jeton dans l’adresse : git clone avec tes accès habituels.",
            "No password or token in the address: git clones with your usual access."
        ),
        BadAddress::Scheme { scheme } => tr!(
            "Escouade clone en https:// ou en ssh, pas en {scheme}.",
            "Escouade clones over https:// or ssh, not {scheme}."
        ),
        _ => {
            let u = url.trim();
            tr!(
                "« {u} » n’est pas l’adresse d’un dépôt git.",
                "“{u}” isn’t a git repository’s address."
            )
        }
    })
}

/// « Cloner » of the station: the repository at `url` into `parent/folder`, by the user's own git access (nothing of
/// Escouade's); a folder there already that is not empty is left alone and refused.
pub async fn clone(url: &str, parent: &Path, folder: &str) -> Cloned {
    let a = match clone_address(url) {
        Ok(a) => a,
        Err(why) => return failed("address", why),
    };
    let folder = folder.trim();
    if !crate::team_projects::folder_ok(folder) {
        return failed(
            "place",
            tr!(
                "« {folder} » n’est pas un nom de dossier.",
                "“{folder}” isn’t a folder name."
            ),
        );
    }
    if !parent.is_dir() {
        let p = parent.display();
        return failed(
            "place",
            tr!("{p} n’est pas un dossier.", "{p} isn’t a folder."),
        );
    }
    if !writable(parent) {
        let p = parent.display();
        return failed(
            "place",
            tr!(
                "Escouade ne peut pas écrire dans ce dossier ({p}) : choisis-en un autre.",
                "Escouade can’t write to this folder ({p}): pick another one."
            ),
        );
    }
    let dest = parent.join(folder);
    if dest.exists()
        && std::fs::read_dir(&dest)
            .map(|mut d| d.next().is_some())
            .unwrap_or(true)
    {
        let p = dest.display();
        return failed(
            "place",
            tr!(
                "{p} existe déjà : choisis un autre nom de dossier.",
                "{p} already exists: pick another folder name."
            ),
        );
    }
    if dest.exists() {
        let _ = std::fs::remove_dir(&dest);
    }
    match crate::team_projects::clone_one(&a, "", &dest, &[]).await {
        Ok(()) => Cloned {
            path: Some(dest.to_string_lossy().to_string()),
            failure: None,
        },
        Err((_, e)) if offline_in(&e) => failed("offline", e),
        Err((true, e)) => failed("access", e),
        Err((false, e)) => failed("error", e),
    }
}

impl Core {
    /// What the journey reads of the app now.
    pub fn onboarding_facts(&self) -> Facts {
        let (accounts, mode_chosen, telemetry_answered) = {
            let s = self.settings.read();
            (
                s.accounts.len(),
                !s.agent_mode.trim().is_empty(),
                s.telemetry.is_some(),
            )
        };
        let projects = self.projects.read().len();
        // A turn ended well: some prompt sent, the agent at rest, neither stopped by its usage limit nor failed.
        let first_reply = self.agents.read().values().any(|h| {
            let m = &h.lock().meta;
            m.prompts > 0
                && matches!(m.status, AgentStatus::Idle | AgentStatus::Done)
                && m.resume_at.is_none()
        });
        let (mode_imposed, telemetry_imposed) = {
            let locks = self.locks.read();
            (
                locks.value(DEFAULT_MODE).is_some(),
                locks.value(TELEMETRY).is_some(),
            )
        };
        Facts {
            fresh: self.fresh,
            accounts,
            mode_chosen,
            projects,
            first_reply,
            telemetry_answered,
            mode_imposed,
            telemetry_imposed,
        }
    }

    /// The journey as the window shows it. Opened by itself at a new installation's first start, it is kept as started
    /// (left half-way, it opens again); the stations the real state made done since are kept as done.
    pub fn onboarding_view(&self) -> OnboardingView {
        let window = crate::remoteapi::current() == crate::remoteapi::Client::Window;
        let facts = self.onboarding_facts();
        let mut saved = self.onboarding.write();
        let mut changed = settle(&mut saved, &facts);
        let shown = view(&saved, &facts, window);
        if shown.trigger == Some(Trigger::First) && saved.started.is_none() {
            saved.version = VERSION;
            saved.started = Some(chrono::Utc::now().timestamp_millis());
            changed = true;
        }
        drop(saved);
        if changed {
            self.request_save();
        }
        self.with_demo(shown)
    }

    /// `view` with where the demonstration project lives.
    fn with_demo(&self, mut view: OnboardingView) -> OnboardingView {
        view.demo = crate::demo::dir(self.data.root())
            .to_string_lossy()
            .to_string();
        view
    }

    /// `act` on the journey; the journey as it then is.
    pub fn onboarding_act(&self, act: Act) -> OnboardingView {
        let window = crate::remoteapi::current() == crate::remoteapi::Client::Window;
        let facts = self.onboarding_facts();
        let shown = {
            let mut saved = self.onboarding.write();
            apply(
                &mut saved,
                &act,
                &facts,
                chrono::Utc::now().timestamp_millis(),
            );
            view(&saved, &facts, window)
        };
        self.request_save();
        self.with_demo(shown)
    }
}

#[cfg(test)]
#[path = "onboarding_tests.rs"]
mod tests;
