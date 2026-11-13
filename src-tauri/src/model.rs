//! Types persisted on disk and exchanged with the frontend. The data is in `escouade-model` (docs/CRATES.md), re-exported
//! here; what names the app's own types stays: the events told to the window (`UiEvent`) and the quotas' snapshot,
//! which `usage` keeps up to date.

pub use escouade_model::model::*;

use crate::resources::Resources;
use serde::Serialize;
use std::collections::BTreeMap;

/// The voice a conversation reads in, from the voices shipped with the app (`voice::speech::catalog`).
pub trait SpeechVoice {
    /// The voice of `lang`: the one chosen, else the default one.
    fn voice(&self, lang: &str) -> String;
}

impl SpeechVoice for SpeechSettings {
    fn voice(&self, lang: &str) -> String {
        self.voices
            .get(lang)
            .filter(|id| crate::voice::speech::catalog::find(id).is_some())
            .cloned()
            .unwrap_or_else(|| crate::voice::speech::catalog::default_voice(lang).to_string())
    }
}

/// The quotas of every account, of every harness, as the window is told them (`usage::settle`).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    /// What the turns cost today, on every account (also those the settings no longer list).
    pub today_cost: f64,
    /// Every account, in the settings' order.
    pub accounts: Vec<AccountUsage>,
    /// The account new agents of each harness would go to (`accounts::pick`): the one whose
    /// meters the status bar shows for that harness. Every harness that has an account.
    pub current: BTreeMap<EngineId, String>,
}

/// One account's quota, as last read.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsage {
    pub id: String,
    /// Its harness.
    pub engine: EngineId,
    /// Its meters as last read (`QuotaMeter`), the shortest window first; none for a harness
    /// without a quota, or not read yet.
    pub meters: Vec<QuotaMeter>,
    /// Signed in, as the last reading tells (an account not read yet is taken as signed in).
    pub connected: bool,
    /// Why its quota is not read (not signed in, sign-in expired), in the interface's language.
    pub reason: Option<String>,
    /// What its turns cost today (`UsageSnapshot::set_today`).
    pub today_cost: f64,
    /// Its « Budget par jour ($) » (`Account::daily_budget`), when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_budget: Option<f64>,
    /// The start of the day `today_cost` is of (local midnight, ms): a budget holds nothing back
    /// once that day is over, before the cost is read again.
    #[serde(skip)]
    pub today_start: i64,
    /// When its windows were last read, ms since epoch (0: not in this run).
    pub updated_at: i64,
    /// Its agents go on answering over their window, as extra usage billed apart (the last
    /// `rate_limit_event` of one of them says so, `isUsingOverage`).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub overage: bool,
    /// What `connected` and `reason` tell, kept to write them again in another language.
    #[serde(skip)]
    pub problem: Option<SignInProblem>,
}

#[cfg(test)]
impl UsageSnapshot {
    /// The account new agents of Claude Code go to.
    pub fn claude_current(&self) -> &str {
        self.current
            .get(&EngineId::CLAUDE)
            .map_or("", String::as_str)
    }

    /// Its 5-hour window, as the status bar shows it for Claude Code.
    pub fn five_hour(&self) -> Option<RateWindow> {
        let id = self.claude_current();
        self.accounts.iter().find(|a| a.id == id)?.five_hour()
    }

    pub fn seven_day(&self) -> Option<RateWindow> {
        let id = self.claude_current();
        self.accounts.iter().find(|a| a.id == id)?.seven_day()
    }

    /// When the current account of Claude Code was last read.
    pub fn updated_at(&self) -> i64 {
        let id = self.claude_current();
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .map_or(0, |a| a.updated_at)
    }
}

#[cfg(test)]
impl AccountUsage {
    pub fn five_hour(&self) -> Option<RateWindow> {
        self.window(FIVE_HOURS)
    }

    pub fn seven_day(&self) -> Option<RateWindow> {
        self.window(WEEK)
    }
}

impl AccountUsage {
    /// Its window of `minutes` of every model (not a model's own, ESC-312), as last read.
    pub fn window(&self, minutes: u32) -> Option<RateWindow> {
        self.meters
            .iter()
            .find(|m| m.minutes == Some(minutes) && m.model.is_none())
            .map(QuotaMeter::rate)
    }
}

/// Why an account's quota cannot be read from the usage endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInProblem {
    /// No sign-in to claude.ai in its folder (an API key, or never signed in).
    NotSignedIn,
    /// Its sign-in is out of date: Claude Code renews it when it runs.
    Expired,
}

// Serialized and sent right away: the size gap between variants does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UiEvent {
    /// Where an agent is in its project's workflow, when what shows of it changed (ESC-193).
    WorkflowTrack {
        track: crate::workflow_track::TrackView,
    },
    Agent {
        agent: AgentView,
    },
    #[serde(rename_all = "camelCase")]
    AgentRemoved {
        id: String,
        project_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Conv {
        agent_id: String,
        ops: Vec<ConvOp>,
    },
    /// A project's shared memory changed (ESC-255): written or forgotten through the server or the window.
    #[serde(rename_all = "camelCase")]
    Memory {
        project_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Git {
        project_id: String,
        git: GitInfo,
    },
    Usage {
        usage: UsageSnapshot,
    },
    #[serde(rename_all = "camelCase")]
    Focus {
        project_id: String,
        agent_id: Option<String>,
    },
    /// "Quitter" while the editor holds unsaved files: the window confirms first.
    QuitRequested {
        unsaved: usize,
    },
    /// The automatic restart for the update downloaded: when it comes (the user is warned before
    /// it), or none once it is called off.
    UpdateRestart {
        at: Option<i64>,
    },
    /// The update downloaded did not install, the app still running: it does not restart by
    /// itself for it any more (« Réessayer »).
    UpdateFailed {
        version: String,
    },
    #[serde(rename_all = "camelCase")]
    TerminalExit {
        id: String,
        code: Option<u32>,
    },
    Resources {
        resources: Resources,
    },
    Models {
        models: Vec<ModelInfo>,
    },
    /// The accounts as the backend changed them by itself (a harness's Principal made as it first
    /// serves, `Core::accounts_of`).
    Accounts {
        accounts: Vec<Account>,
        engine_order: Vec<EngineId>,
    },
    /// What the `claude` of an account told of its models (`PersistedState::account_models`).
    AccountModels {
        account: String,
        models: AccountModels,
    },
    /// The models a harness listed for its account (`PersistedState::engine_models`).
    EngineModels {
        engine: EngineId,
        models: Vec<crate::engine::registry::ToldModel>,
    },
    Ticket {
        ticket: Ticket,
    },
    #[serde(rename_all = "camelCase")]
    TicketRemoved {
        id: String,
        project_id: String,
    },
    /// A project changed on the backend's side (its board's prefix, target or settings).
    Project {
        project: Project,
    },
    /// A ticket's notification was clicked: the project's board is shown.
    #[serde(rename_all = "camelCase")]
    FocusBoard {
        project_id: String,
    },
    /// The notification of a review asked of the user was clicked: the project's pull requests
    /// are shown (ESC-68).
    #[serde(rename_all = "camelCase")]
    FocusPrs {
        project_id: String,
    },
    /// The project's list of pull requests was read, whoever read it (the view, the « PR et CI »
    /// tab, the overview, the tools): the numbers of its open ones, for the PR tab's badge of the new
    /// ones (ESC-403). Told by a reading already made, never by one of its own.
    #[serde(rename_all = "camelCase")]
    PrsRead {
        project_id: String,
        numbers: Vec<u64>,
    },
    /// Why no ticket of the project's board starts (its target branch has no commit yet, or is
    /// gone), or None once they may start again.
    /// A review of a pull request written by an agent, changed (ESC-66).
    PrReview {
        review: crate::prreview::PrReview,
    },
    /// That review, dropped.
    #[serde(rename_all = "camelCase")]
    PrReviewGone {
        project_id: String,
        number: u64,
    },
    /// The code hosts may have changed (a host's plugin switched on or off, installed, removed; an
    /// account connected or gone): the window reads its projects' code host again (ESC-390).
    CodeHosts,
    /// The environments of a project's host, as last read (ESC-67).
    Environments {
        environments: crate::deploys::ProjectEnvironments,
    },
    #[serde(rename_all = "camelCase")]
    BoardIssue {
        project_id: String,
        issue: Option<String>,
    },
    /// The CI of a project's target branch as last read (ESC-63), or None when it has none to show
    /// (no remote on a host Escouade reads, no account there).
    #[serde(rename_all = "camelCase")]
    TargetCi {
        project_id: String,
        ci: Option<crate::branchci::BranchCi>,
    },
    /// Why no ticket of a board starts for now (a quota, a usage limit), or None once they may
    /// start again: `pause` for the projects that go to any account, `projects` for the paused
    /// ones that prefer an account (each pauses on its own).
    AutopilotPause {
        pause: Option<AutopilotPause>,
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        projects: BTreeMap<String, AutopilotPause>,
    },
    /// What happened to a launch's page (ESC-311): a load, a link opened outside, its devtools.
    DevPage {
        page: crate::devpage::PageEvent,
    },
    /// An address to open in the default browser (a pull request to finish on GitHub).
    OpenUrl {
        url: String,
    },
    /// A plugin of the seed installed for a gesture that needs it (ESC-373): the window says it in a line, and reads
    /// the plugins again.
    PluginSeeded {
        id: String,
        text: String,
    },
    /// Something done in the background the window says in passing (tickets imported by
    /// themselves).
    Toast {
        text: String,
    },
    /// The settings changed the language of the interface or of the texts Claude writes: the
    /// window switches to it at once.
    Language {
        lang: crate::i18n::LangInfo,
    },
    /// Lines the step `step` (from 0) of the setup of an agent's worktree wrote since the window
    /// was last sent some (50 ms apart at most), its `total` lines so far counting them. A step
    /// starts with none: the window forgets the lines of the step before.
    #[serde(rename_all = "camelCase")]
    SetupOutput {
        agent_id: String,
        step: usize,
        total: usize,
        lines: Vec<String>,
    },
    /// The MCP server started, stopped, could not start, or listens on another port now.
    McpStatus {
        status: crate::mcp::McpStatus,
    },
    /// What the MCP server was asked, or refused, as its activity log keeps it.
    McpActivity {
        entry: crate::mcp::activity::ActivityEntry,
    },
    /// Where the server stands in each active account's Claude Code (declared, or why not).
    McpDeclared {
        declared: Vec<crate::mcp::install::Declaration>,
    },
    /// The dictation: the level while it listens, a listening that ended by itself, a model's
    /// download.
    Voice {
        voice: crate::voice::VoiceEvent,
    },
    /// A link `escouade://` came (STORE-11): the window takes it (`deep_links_take`).
    DeepLink {},
    /// The download of the vectors' model (ESC-254): bytes so far, of `total`.
    VectorsDownload {
        done: u64,
        total: u64,
    },
    /// What a project's docs have to share (ESC-257): the changes not shared yet, and the conflict
    /// a « Partager » stopped on, if any.
    #[serde(rename_all = "camelCase")]
    DocsShare {
        project_id: String,
        pending: u32,
        conflict: Option<crate::docsshare::Conflict>,
    },
    /// The pages of a project being compared to the code (ESC-263), told when one starts or ends.
    #[serde(rename_all = "camelCase")]
    DocsCompared {
        project_id: String,
        comparing: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_first_dictation_takes_the_default_model_of_the_voice_on_the_machine() {
        // `escouade-model` names it without the voice's crate: the two agree.
        assert_eq!(DEFAULT_WHISPER_MODEL, crate::voice::models::DEFAULT_MODEL);
        assert_eq!(
            VoiceSettings::default().model,
            crate::voice::models::DEFAULT_MODEL
        );
    }

    #[test]
    fn the_quotas_of_every_account_travel_in_camel_case() {
        let w = RateWindow {
            pct: 12.0,
            resets_at: Some(5),
        };
        let usage = UsageSnapshot {
            today_cost: 1.5,
            accounts: vec![AccountUsage {
                id: "pro".into(),
                meters: vec![
                    QuotaMeter::window(FIVE_HOURS, w),
                    QuotaMeter::credits(RateWindow {
                        pct: 40.0,
                        resets_at: None,
                    }),
                ],
                connected: false,
                reason: Some("Pas connecté".into()),
                updated_at: 3,
                problem: Some(SignInProblem::NotSignedIn),
                ..Default::default()
            }],
            current: BTreeMap::from([(EngineId::CLAUDE, "pro".to_string())]),
        };
        assert_eq!(
            serde_json::to_value(&usage).unwrap(),
            json!({ "todayCost": 1.5, "current": { "claude": "pro" },
                    "accounts": [{ "id": "pro", "engine": "claude",
                                   "meters": [{ "minutes": 300, "pct": 12.0, "resetsAt": 5 },
                                              { "minutes": null, "pct": 40.0, "resetsAt": null }],
                                   "connected": false,
                                   "reason": "Pas connecté", "todayCost": 0.0,
                                   "updatedAt": 3 }] })
        );
    }

    #[test]
    fn the_autopilots_pause_travels_in_camel_case() {
        let e = UiEvent::AutopilotPause {
            pause: Some(AutopilotPause {
                reason: PauseReason::Window,
                minutes: Some(FIVE_HOURS),
                pct: Some(100.0),
                until: 5,
                accounts: vec![],
            }),
            projects: BTreeMap::new(),
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "autopilotPause",
                    "pause": { "reason": "window", "minutes": 300, "pct": 100.0, "until": 5 } })
        );
        let over = UiEvent::AutopilotPause {
            pause: None,
            projects: BTreeMap::new(),
        };
        assert_eq!(
            serde_json::to_value(&over).unwrap(),
            json!({ "type": "autopilotPause", "pause": null })
        );
    }

    #[test]
    fn a_setup_step_keeps_its_last_500_lines_and_counts_them_all() {
        let mut out = SetupOutput::new(1);
        assert_eq!((out.step, out.total, out.lines.len()), (1, 0, 0));
        let lines: Vec<String> = (1..=600).map(|i| format!("ligne {i}")).collect();
        out.push(&lines[..550]);
        out.push(&lines[550..]);
        assert_eq!(SETUP_LINES, 500);
        assert_eq!(
            (out.step, out.total, out.lines.len()),
            (1, 600, SETUP_LINES)
        );
        assert_eq!(out.lines.front().map(String::as_str), Some("ligne 101"));
        assert_eq!(out.lines.back().map(String::as_str), Some("ligne 600"));
        // As the window reads it, and as the lines just written are sent to it.
        let sent = serde_json::to_value(&out).unwrap();
        assert_eq!(
            (sent["step"].clone(), sent["total"].clone()),
            (json!(1), json!(600))
        );
        assert_eq!(sent["lines"][0], "ligne 101");
        let e = UiEvent::SetupOutput {
            agent_id: "a1".into(),
            step: 1,
            total: 600,
            lines: vec!["ligne 600".into()],
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "setupOutput", "agentId": "a1", "step": 1, "total": 600, "lines": ["ligne 600"] })
        );
    }

    #[test]
    fn a_click_on_a_tickets_notification_asks_for_the_projects_board() {
        let e = UiEvent::FocusBoard {
            project_id: "p1".into(),
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "focusBoard", "projectId": "p1" })
        );
    }

    #[test]
    fn the_pause_of_several_accounts_names_them_and_the_projects_that_wait_for_their_own() {
        let pause = |accounts: &[&str]| AutopilotPause {
            reason: PauseReason::Window,
            minutes: Some(WEEK),
            pct: Some(97.0),
            until: 5,
            accounts: accounts.iter().map(|a| a.to_string()).collect(),
        };
        let e = UiEvent::AutopilotPause {
            pause: Some(pause(&["principal", "pro"])),
            projects: BTreeMap::from([("p1".to_string(), pause(&["pro"]))]),
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "autopilotPause",
                    "pause": { "reason": "window", "minutes": 10080, "pct": 97.0, "until": 5,
                               "accounts": ["principal", "pro"] },
                    "projects": { "p1": { "reason": "window", "minutes": 10080, "pct": 97.0, "until": 5,
                                          "accounts": ["pro"] } } })
        );
    }
}
