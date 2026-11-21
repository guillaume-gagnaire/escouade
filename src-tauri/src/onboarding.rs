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
use crate::model::{
    AgentStatus, Hookup, Onboarding, OnboardingChoice, OnboardingProfile, OnboardingStep,
    StepOutcome,
};
use crate::telemetry::{self, Event};
use escouade_policy::list::{DEFAULT_MODE, DICTATION, TELEMETRY, VOICE_ONLINE};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The version of the journey: a later one that changes a station raises it.
pub const VERSION: u32 = 1;

/// The stations, in their order (§ 4). « Ce que tu veux brancher » (ESC-512) comes after the starting settings and before
/// the project: a code host connected there gives the project's station its repositories to clone.
pub const STEPS: [OnboardingStep; 7] = [
    OnboardingStep::Harness,
    OnboardingStep::Account,
    OnboardingStep::Defaults,
    OnboardingStep::Hookups,
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
    /// That reply came in the demonstration project (ESC-507: the funnel says so).
    pub first_reply_demo: bool,
    /// The statistics' question is answered (`Settings::telemetry`).
    pub telemetry_answered: bool,
    /// What a policy imposes: the mode of the new agents, the statistics.
    pub mode_imposed: bool,
    pub telemetry_imposed: bool,
    /// What is plugged in already (ESC-512), wherever it was: an account of a source of tickets, of a code host, a voice
    /// service on line with its key.
    pub sources_connected: bool,
    pub forge_connected: bool,
    pub voice_ready: bool,
    /// The dictation turned off by a policy (`voice.dictation`): the voice's card is shown locked.
    pub dictation_off: bool,
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

/// The cards of « Ce que tu veux brancher » (ESC-512), in their order. A second harness and the MCP server for another
/// client stay in the discovery (`docs/ONBOARDING.md` § 7, `second-agent` and `mcp`): the first is the harness station's
/// and Réglages › Comptes et harness', the second opens a door of the machine, which a first launch does not.
pub const HOOKUPS: [Hookup; 3] = [Hookup::Sources, Hookup::Voice, Hookup::Forge];

/// Where a card stands: plugged in (wherever it was), passed (by its « Plus tard », or the station's), imposed by a policy,
/// or still to do. A card passed waits in the discovery (ON-3) and in its page of Réglages.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HookupView {
    pub id: Hookup,
    pub status: StepStatus,
}

/// A thing a card of « Ce que tu veux brancher » offers (ESC-512): a source of tickets, a voice service on line, a code
/// host; its plugin, whether it is installed, whether the seed holds it (it installs then with no network), and why a
/// policy denies it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub kind: Hookup,
    pub id: String,
    pub name: String,
    pub plugin: Option<String>,
    pub installed: bool,
    pub seed: bool,
    pub denied: Option<String>,
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
    /// The cards of « Ce que tu veux brancher » (ESC-512).
    pub hookups: Vec<HookupView>,
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
        /// At a station that recommends a choice: whether the user kept it (ESC-507, the funnel).
        #[serde(default)]
        choice: Option<OnboardingChoice>,
    },
    /// What Escouade found of the harnesses at the journey's first station (ESC-507): kept once per run.
    Profile { profile: OnboardingProfile },
    /// « Tout passer »: every station not done is passed, and the journey ends.
    SkipAll,
    /// The last station passed: the journey ends.
    Finish,
    /// « Refaire le premier lancement »: opened again, every station shown from the first; nothing set is undone.
    Redo,
    /// The statistics' question passed in the journey asked once more: never again.
    TelemetryAskedAgain,
    /// A card of « Ce que tu veux brancher » plugged in, or passed (its « Plus tard », or left unticked when the station
    /// goes on).
    Hookup {
        hookup: Hookup,
        outcome: StepOutcome,
    },
}

/// Whether what `facts` say makes the card `h` plugged in, wherever it was.
pub fn hookup_real(h: Hookup, facts: &Facts) -> bool {
    match h {
        Hookup::Sources => facts.sources_connected,
        Hookup::Voice => facts.voice_ready,
        Hookup::Forge => facts.forge_connected,
    }
}

/// Where the card `h` stands.
pub fn hookup_status(h: Hookup, saved: &Onboarding, facts: &Facts) -> StepStatus {
    if h == Hookup::Voice && facts.dictation_off {
        return StepStatus::Policy;
    }
    if hookup_real(h, facts) {
        return StepStatus::Done;
    }
    match saved.hookups.get(&h) {
        Some(StepOutcome::Done) => StepStatus::Done,
        Some(StepOutcome::Skipped) => StepStatus::Skipped,
        // The station passed whole (its « Plus tard », « Tout passer »): each card not plugged in is passed with it.
        None if saved.steps.get(&OnboardingStep::Hookups) == Some(&StepOutcome::Skipped) => {
            StepStatus::Skipped
        }
        None => StepStatus::Todo,
    }
}

/// Whether what `facts` say makes `step` done, wherever it was set.
pub fn real(step: OnboardingStep, facts: &Facts) -> bool {
    match step {
        // A harness is chosen once an account of it is added: a harness found alone is not chosen yet.
        OnboardingStep::Harness | OnboardingStep::Account => facts.accounts > 0,
        OnboardingStep::Defaults => facts.mode_chosen,
        // Something plugged in already (a migration, Réglages): the station has done its work.
        OnboardingStep::Hookups => HOOKUPS.iter().any(|&h| hookup_real(h, facts)),
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
        hookups: HOOKUPS
            .iter()
            .map(|&id| HookupView {
                id,
                status: hookup_status(id, saved, facts),
            })
            .collect(),
        demo: String::new(),
    }
}

/// `act` done on `saved`, at `now` (ms).
pub fn apply(saved: &mut Onboarding, act: &Act, facts: &Facts, now: i64) {
    saved.version = VERSION;
    saved.started.get_or_insert(now);
    saved.run_started.get_or_insert(now);
    match act {
        Act::Step {
            step,
            outcome,
            choice,
        } => {
            if let (StepOutcome::Done, Some(c)) = (outcome, choice) {
                saved.choices.insert(*step, *c);
            }
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
            // A new run, a funnel of its own.
            saved.run_started = Some(now);
            saved.profile = None;
            saved.choices.clear();
            saved.told = false;
        }
        Act::Profile { profile } => {
            saved.profile.get_or_insert(*profile);
        }
        Act::TelemetryAskedAgain => saved.telemetry_asked_again = true,
        Act::Hookup { hookup, outcome } => {
            // A card plugged in stays so: « Plus tard » on it, shown again, never undoes it.
            if !(*outcome == StepOutcome::Skipped
                && (saved.hookups.get(hookup) == Some(&StepOutcome::Done)
                    || hookup_real(*hookup, facts)))
            {
                saved.hookups.insert(*hookup, *outcome);
            }
        }
    }
    settle(saved, facts);
}

// === The funnel (ESC-507, § 9) ===

/// The first agent's first turn ended during the journey, kept with when (`now`, ms): the funnel's `first_reply`. Whether
/// it was just seen.
pub fn note_reply(saved: &mut Onboarding, facts: &Facts, now: i64) -> bool {
    if saved.started.is_none()
        || saved.finished.is_some()
        || !facts.first_reply
        || saved.first_reply_at.is_some()
    {
        return false;
    }
    saved.first_reply_at = Some(now);
    saved.first_reply_demo = facts.first_reply_demo;
    true
}

/// How a station's id is said in the statistics.
pub fn step_name(step: OnboardingStep) -> &'static str {
    match step {
        OnboardingStep::Harness => "harness",
        OnboardingStep::Account => "account",
        OnboardingStep::Defaults => "defaults",
        OnboardingStep::Hookups => "hookups",
        OnboardingStep::Project => "project",
        OnboardingStep::Agent => "agent",
        OnboardingStep::Telemetry => "telemetry",
    }
}

/// What the usage statistics tell of a run of the journey once it ended (`docs/ONBOARDING.md` § 9): it began (why, and
/// what Escouade found), each station left (how, and whether its recommended choice was kept), it ended (how many
/// stations passed, how long it took), and the first agent's first turn when it came during the run. Closed lists and
/// brackets only: never a name, a path, an address or what was chosen. Nothing while the run is under way.
pub fn funnel(saved: &Onboarding, facts: &Facts) -> Vec<Event> {
    let (Some(began), Some(ended)) = (saved.run_started.or(saved.started), saved.finished) else {
        return Vec::new();
    };
    let profile = match saved.profile {
        Some(OnboardingProfile::Ready) => "ready",
        Some(OnboardingProfile::Installed) => "installed",
        Some(OnboardingProfile::Managed) => "managed",
        Some(OnboardingProfile::None) | None => "none",
    };
    // A run that began after the first one is the journey done again.
    let redo = saved.run_started.is_some_and(|r| Some(r) != saved.started);
    let mut out = vec![Event::OnboardingStarted {
        trigger: if redo { "redo" } else { "first" },
        profile,
    }];
    let mut skipped = 0;
    for step in STEPS {
        let outcome = match status(step, saved, facts) {
            StepStatus::Todo => continue,
            StepStatus::Done => "done",
            StepStatus::Skipped => {
                skipped += 1;
                "skipped"
            }
            StepStatus::Policy => "policy",
        };
        let choice = match saved.choices.get(&step) {
            Some(OnboardingChoice::Recommended) if outcome == "done" => Some("recommended"),
            Some(OnboardingChoice::Other) if outcome == "done" => Some("other"),
            _ => None,
        };
        out.push(Event::OnboardingStep {
            step: step_name(step),
            outcome,
            choice,
        });
    }
    out.push(Event::OnboardingFinished {
        skipped,
        seconds: telemetry::seconds(ended - began),
    });
    if let Some(at) = saved.first_reply_at.filter(|&at| at >= began) {
        out.push(Event::FirstReply {
            seconds: telemetry::seconds(at - began),
            demo: saved.first_reply_demo,
        });
    }
    out
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
        let replied: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let m = &h.lock().meta;
                (m.prompts > 0
                    && matches!(m.status, AgentStatus::Idle | AgentStatus::Done)
                    && m.resume_at.is_none())
                .then(|| m.project_id.clone())
            })
            .collect();
        let first_reply = !replied.is_empty();
        let first_reply_demo = self.projects.read().iter().any(|p| {
            replied.contains(&p.id) && crate::demo::is_demo(self.data.root(), Path::new(&p.path))
        });
        let (mode_imposed, telemetry_imposed, dictation_off) = {
            let locks = self.locks.read();
            (
                locks.value(DEFAULT_MODE).is_some(),
                locks.value(TELEMETRY).is_some(),
                locks.value(DICTATION) == Some(&serde_json::Value::Bool(false)),
            )
        };
        let (sources_connected, forge_connected) = {
            let a = self.accounts.read();
            let own = [&a.jira, &a.trello, &a.github, &a.linear, &a.clickup]
                .iter()
                .any(|x| x.is_some());
            // GitLab Issues and Azure Boards are served by their code host's account.
            let hosted = a.forges.iter().any(|f| {
                crate::integrations::source::registry()
                    .services()
                    .iter()
                    .any(|s| s.as_str() == f.kind)
            });
            (own || hosted, a.github.is_some() || !a.forges.is_empty())
        };
        let voice_ready = {
            let s = self.settings.read();
            s.voice.transcription == "online"
                && crate::voice::online::usable_keys(&s, &*self.secrets)
                    .contains(&s.voice.online_service)
        };
        Facts {
            fresh: self.fresh,
            accounts,
            mode_chosen,
            projects,
            first_reply,
            first_reply_demo,
            telemetry_answered,
            mode_imposed,
            telemetry_imposed,
            sources_connected,
            forge_connected,
            voice_ready,
            dictation_off,
        }
    }

    /// What « Ce que tu veux brancher » offers (ESC-512), read in the registry of the sources, the tables of the voice
    /// services and of the code hosts, the seed and the installed plugins: never written down in the window. Nothing is
    /// read on the network.
    pub fn onboarding_offers(&self) -> Vec<Offer> {
        use crate::plugins::{builtin, sort};
        let lang = crate::i18n::ui();
        let locks = self.locks.read();
        let online_off = locks.value(VOICE_ONLINE) == Some(&serde_json::Value::Bool(false));
        let offer = |kind: Hookup, id: &str, name: String, plugin: Option<String>| {
            let denied = match &plugin {
                _ if kind == Hookup::Voice && online_off => Some(locks.imposed(VOICE_ONLINE)),
                Some(p) => locks
                    .check_plugin(p, "store-verified")
                    .err()
                    .map(|e| e.to_string()),
                None => None,
            };
            Offer {
                kind,
                id: id.to_string(),
                name,
                installed: plugin
                    .as_deref()
                    .is_some_and(|p| self.plugins.is_installed(p)),
                seed: plugin.as_deref().is_some_and(|p| self.plugins.in_seed(p)),
                plugin,
                denied,
            }
        };
        let mut out = Vec::new();
        for k in crate::integrations::source::registry().all() {
            out.push(offer(
                Hookup::Sources,
                k.id(),
                k.label().to_string(),
                k.plugin().map(str::to_string),
            ));
        }
        for id in sort::VOICES {
            let plugin = sort::of_voice(id);
            let name = plugin
                .as_deref()
                .and_then(|p| builtin::name(p, lang))
                .unwrap_or_else(|| id.to_string());
            out.push(offer(Hookup::Voice, id, name, plugin));
        }
        for (kind, plugin) in sort::FORGES {
            let name = builtin::name(plugin, lang).unwrap_or_else(|| kind.to_string());
            out.push(offer(Hookup::Forge, kind, name, Some(plugin.to_string())));
        }
        out
    }

    /// The plugin of what was chosen on a card of « Ce que tu veux brancher » (ESC-512), from the seed with no network,
    /// and nothing else: the gesture is the user's consent (ESC-373). None when it is installed already, or the seed does
    /// not hold it (it is then the store's, installed from Plugins, its permissions read); refused when a policy denies
    /// it or the card offers no such thing.
    pub fn onboarding_need(
        &self,
        kind: Hookup,
        id: &str,
    ) -> Result<Option<crate::plugins::PluginView>, crate::plugins::Refusal> {
        match to_install(&self.onboarding_offers(), kind, id) {
            Err(why) => Err(crate::plugins::Refusal::other(why)),
            Ok(Some(p)) => self.install_seed_plugin(&p),
            Ok(None) => Ok(None),
        }
    }
}

/// The plugin to install from the seed for `id` chosen on the card `kind`, among `offers`: its own, and only when it is
/// not installed and the seed holds it; refused when a policy denies it or the card does not offer it.
pub fn to_install(offers: &[Offer], kind: Hookup, id: &str) -> Result<Option<String>, String> {
    let Some(o) = offers.iter().find(|o| o.kind == kind && o.id == id) else {
        return Err(tr!(
            "« {id} » n'est pas offert ici.",
            "“{id}” is not offered here."
        ));
    };
    if let Some(why) = &o.denied {
        return Err(why.clone());
    }
    Ok(o.plugin.clone().filter(|_| !o.installed && o.seed))
}

impl Core {
    /// The journey as the window shows it. Opened by itself at a new installation's first start, it is kept as started
    /// (left half-way, it opens again); the stations the real state made done since are kept as done.
    pub fn onboarding_view(&self) -> OnboardingView {
        let window = crate::remoteapi::current() == crate::remoteapi::Client::Window;
        let facts = self.onboarding_facts();
        let mut saved = self.onboarding.write();
        let now = chrono::Utc::now().timestamp_millis();
        let mut changed = settle(&mut saved, &facts);
        changed |= note_reply(&mut saved, &facts, now);
        let shown = view(&saved, &facts, window);
        if shown.trigger == Some(Trigger::First) && saved.started.is_none() {
            saved.version = VERSION;
            saved.started = Some(now);
            saved.run_started = Some(now);
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
            let now = chrono::Utc::now().timestamp_millis();
            apply(&mut saved, &act, &facts, now);
            note_reply(&mut saved, &facts, now);
            view(&saved, &facts, window)
        };
        self.onboarding_tell();
        self.request_save();
        self.with_demo(shown)
    }

    /// The funnel of the run that ended told to the usage statistics, once (ESC-507): sent with the user's yes, dropped
    /// with a no, kept while there is no answer yet (asked once more at the next start: a yes then sends it).
    pub fn onboarding_tell(&self) {
        let facts = self.onboarding_facts();
        let events = {
            let mut saved = self.onboarding.write();
            if saved.finished.is_none() || saved.told {
                return;
            }
            if !self.telemetry.enabled() {
                if self.settings.read().telemetry == Some(false) {
                    saved.told = true;
                    drop(saved);
                    self.request_save();
                }
                return;
            }
            saved.told = true;
            funnel(&saved, &facts)
        };
        for e in events {
            self.telemetry.track(e);
        }
        self.request_save();
    }
}

#[cfg(test)]
#[path = "onboarding_tests.rs"]
mod tests;
