//! Types persisted on disk and exchanged with the frontend.

use crate::agent::NotifyKind;
use crate::resources::Resources;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Empty = auto-detect `claude` on PATH.
    pub claude_path: String,
    pub default_model: String,
    pub default_effort: String,
    pub default_mode: String,
    pub sound: bool,
    pub os_notifications: bool,
    /// "Me prévenir pour": what the chime and the system notifications are for.
    pub notify_for: NotifyFor,
    /// Stop idle Claude processes after N minutes (0 = never). Sessions stay resumable.
    pub idle_stop_minutes: u32,
    pub pwsh_path: String,
    pub bash_path: String,
    pub wsl_distro: String,
    /// HTTP(S) proxy URL, e.g. `http://user:pass@proxy:3128`. Empty = direct connection.
    pub proxy_url: String,
    /// Comma-separated hosts that bypass the proxy (NO_PROXY).
    pub no_proxy: String,
    /// Also export the proxy variables in integrated terminals.
    pub proxy_terminals: bool,
    /// TLS certificates are not checked (a proxy that decrypts the traffic with a certificate of
    /// its own): the app's own requests, its updates and Claude Code's.
    pub insecure_tls: bool,
    /// Send "continue" by itself to an agent stopped by the usage limit, once the quota resets.
    pub auto_resume: bool,
    /// "Pause au-delà du quota", in percent (80, 90, 95 or 100): no ticket of any board starts
    /// while the 5-hour or the weekly window is used this much or more (`board::autopilot_pause`).
    pub quota_pause: u32,
    /// "Installer les mises à jour automatiquement": a downloaded update restarts the app by
    /// itself once it is at rest (`updates::busy`). Off, the user restarts it, or the update
    /// installs when the app is closed.
    pub auto_update: bool,
    /// What the external ticket systems' links do (their accounts are kept apart, with their
    /// secrets: `integrations::Accounts`).
    pub integrations: IntegrationSettings,
    /// "Langue de l'interface": "system" (French when the system is, English otherwise), "fr" or
    /// "en" (`i18n::resolve`).
    pub language: String,
    /// "Langue des textes rédigés par Claude": "ui" (the interface's), "fr" or "en". Many teams
    /// commit in English with an interface in French.
    pub claude_language: String,
    /// « Claude peut piloter Escouade »: the MCP server runs (`mcp`), declared in Claude Code.
    pub mcp_enabled: bool,
    /// The MCP server's port on 127.0.0.1: 0 until its first start chooses a free one (47000 to
    /// 47999), kept from then on. The backend's own: a save from the window leaves it as it is.
    pub mcp_port: u16,
    /// The Claude accounts, in the order new agents try them; Principal always among them
    /// (`accounts::normalize`, at every load and save).
    pub accounts: Vec<Account>,
}

/// A Claude account: Claude Code with a configuration folder of its own (`accounts`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Account {
    /// `accounts::PRINCIPAL` for the user's own.
    pub id: String,
    pub name: String,
    /// Its `CLAUDE_CONFIG_DIR`; empty for Principal, launched without one.
    pub config_dir: String,
    /// Its own `claude`; empty: the settings' one.
    pub claude_path: String,
    /// New agents may go to it.
    pub active: bool,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            config_dir: String::new(),
            claude_path: String::new(),
            active: true,
        }
    }
}

/// "Me prévenir pour": which kinds of event chime and show a system notification. A kind switched
/// off does neither; the other signals stay: the agent's tab and card blink, and the taskbar
/// flashes (the Dock bounces).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct NotifyFor {
    /// "Questions et autorisations".
    pub questions: bool,
    /// "Tâches terminées".
    pub done: bool,
    /// "Erreurs".
    pub errors: bool,
    /// "Tickets (prêt à tester, bloqué)".
    pub tickets: bool,
}

impl Default for NotifyFor {
    fn default() -> Self {
        Self {
            questions: true,
            done: true,
            errors: true,
            tickets: true,
        }
    }
}

impl NotifyFor {
    /// The user wants to be told of this kind of event.
    pub fn allows(&self, kind: NotifyKind) -> bool {
        match kind {
            NotifyKind::Question => self.questions,
            NotifyKind::Done => self.done,
            NotifyKind::Error => self.errors,
            NotifyKind::Ticket => self.tickets,
        }
    }
}

/// The sync with the external ticket systems and their automatic import, for every project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct IntegrationSettings {
    /// "Mettre à jour le statut externe": a ticket that changes column gives its external one the
    /// state its project's link says.
    pub sync_states: bool,
    /// "Publier un résumé à chaque boucle".
    pub loop_comments: bool,
    /// "Extraire les critères d'acceptation" of an imported ticket's description or checklists.
    pub extract_criteria: bool,
    /// "Importer les tickets étiquetés": the open tickets of the linked sources that carry
    /// `import_label` come in by themselves.
    pub auto_import: bool,
    pub import_label: String,
    /// Minutes between two automatic imports.
    pub import_every: u32,
}

impl Default for IntegrationSettings {
    fn default() -> Self {
        Self {
            sync_states: true,
            loop_comments: false,
            extract_criteria: true,
            auto_import: false,
            import_label: "claude-ready".into(),
            import_every: 15,
        }
    }
}

impl Settings {
    /// The network settings of a Claude Code process: its proxy, and its TLS verification off
    /// when asked.
    pub fn claude_env(&self) -> Vec<(String, String)> {
        let mut env = self.proxy_env();
        if self.insecure_tls {
            env.push(("NODE_TLS_REJECT_UNAUTHORIZED".into(), "0".into()));
        }
        env
    }

    /// Environment variables to inject into child processes for the configured proxy.
    pub fn proxy_env(&self) -> Vec<(String, String)> {
        let url = self.proxy_url.trim();
        if url.is_empty() {
            return Vec::new();
        }
        let mut env = Vec::new();
        for k in ["HTTPS_PROXY", "HTTP_PROXY", "https_proxy", "http_proxy"] {
            env.push((k.to_string(), url.to_string()));
        }
        let no = self.no_proxy.trim();
        if !no.is_empty() {
            env.push(("NO_PROXY".into(), no.to_string()));
            env.push(("no_proxy".into(), no.to_string()));
        }
        env
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            claude_path: String::new(),
            default_model: "sonnet".into(),
            default_effort: "medium".into(),
            default_mode: "auto".into(),
            sound: true,
            os_notifications: true,
            notify_for: NotifyFor::default(),
            idle_stop_minutes: 30,
            pwsh_path: String::new(),
            bash_path: String::new(),
            wsl_distro: String::new(),
            proxy_url: String::new(),
            no_proxy: "localhost,127.0.0.1".into(),
            proxy_terminals: false,
            insecure_tls: false,
            auto_resume: true,
            quota_pause: 100,
            auto_update: true,
            integrations: IntegrationSettings::default(),
            language: "system".into(),
            claude_language: "ui".into(),
            mcp_enabled: false,
            mcp_port: 0,
            accounts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color: String,
    #[serde(default)]
    pub worktree_per_agent: bool,
    #[serde(default)]
    pub created_at: i64,
    /// Commands that launch the project (dev servers, watchers…), each in its own read-only terminal.
    #[serde(default)]
    pub run_commands: Vec<RunCommand>,
    /// The board: what validating a ticket does, its agents, its tickets' key.
    #[serde(default)]
    pub board: BoardSettings,
    /// Untracked files of the project copied into every new worktree (glob patterns).
    #[serde(default = "default_worktree_copy")]
    pub worktree_copy: Vec<String>,
    /// Run in every new worktree, one after the other, before its agent's first message
    /// (dependencies, generated code…).
    #[serde(default)]
    pub worktree_setup: Vec<WorktreeStep>,
    /// Run in a worktree before the app removes it (what its setup made outside of it).
    #[serde(default)]
    pub worktree_teardown: Vec<WorktreeStep>,
    /// The external ticket systems its tickets come from, and what moving them does there.
    #[serde(default)]
    pub integrations: ProjectIntegrations,
    /// "Commit": who writes the commits of the files panel's « Commit… » and « Commit tout… ».
    #[serde(default)]
    pub commit_mode: CommitMode,
    /// « Les agents peuvent utiliser Escouade »: its agents get the MCP server, each with a token
    /// of its own. The server runs while a project lets them (`Core::mcp_wanted`).
    #[serde(default)]
    pub agents_use_escouade: bool,
}

/// Who writes a project's commits asked from the files panel.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CommitMode {
    /// "Rédigé par l'agent": the agent is asked to commit its changes.
    #[default]
    Agent,
    /// "Direct, avec un message proposé": the app commits, with the message the user read (Haiku
    /// proposes one).
    Direct,
}

/// What a direct commit takes: the files it commits, and the files copied into the worktrees
/// (`.env`…) among the changes, which it never commits.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitScope {
    pub files: Vec<FileChange>,
    pub left_out: Vec<String>,
}

/// An external ticket system.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Service {
    #[default]
    Jira,
    Trello,
    Github,
}

impl Service {
    pub const ALL: [Service; 3] = [Service::Jira, Service::Trello, Service::Github];

    /// Its name, as the window and the comments say it.
    pub fn label(self) -> &'static str {
        match self {
            Service::Jira => "Jira",
            Service::Trello => "Trello",
            Service::Github => "GitHub",
        }
    }
}

/// A state of an external ticket: a Jira status, a Trello list, or for GitHub "open", "closed"
/// or "label:<name>".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalState {
    pub id: String,
    pub name: String,
}

/// A project's source of tickets in an external system: a Jira project, a Trello board, a GitHub
/// repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceLink {
    pub service: Service,
    /// The Jira project's key, the Trello board's id, `owner/repo`.
    pub container: String,
    /// As the window shows it.
    pub name: String,
    /// The state an imported ticket's external one is given when it comes into each column
    /// (none: left as it is).
    pub states: BTreeMap<Column, ExternalState>,
}

/// What a project's tickets have to do with external ticket systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectIntegrations {
    /// One per service at most.
    pub links: Vec<SourceLink>,
    /// The columns whose arrival writes a comment on the external ticket.
    pub comments: Vec<Column>,
    /// Every external ticket imported into the project (`<service>|<id>`), even those deleted
    /// since: the automatic import never brings one back.
    pub imported: Vec<String>,
}

impl Default for ProjectIntegrations {
    fn default() -> Self {
        Self {
            links: Vec::new(),
            comments: vec![Column::Review, Column::Done],
            imported: Vec::new(),
        }
    }
}

impl ProjectIntegrations {
    pub fn link(&self, service: Service) -> Option<&SourceLink> {
        self.links.iter().find(|l| l.service == service)
    }
}

/// Where an imported ticket comes from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExternalRef {
    pub service: Service,
    /// What its API knows it by: the Jira issue's key, the Trello card's id, the GitHub issue's
    /// number.
    pub id: String,
    /// As it is shown: ATL-1287, #142, #42.
    pub key: String,
    /// Its Jira project, Trello board or GitHub repository (`SourceLink::container`).
    pub container: String,
    pub url: String,
    /// Why its last sync failed, until one succeeds.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RunCommand {
    pub id: String,
    pub name: String,
    pub command: String,
    /// Shell id ("pwsh", "powershell", "bash", "wsl").
    pub shell: String,
    /// Folder relative to the project's, empty for the project itself.
    pub cwd: String,
}

/// A command run in a worktree once it is made (its setup) or before it is removed (its
/// teardown).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WorktreeStep {
    pub id: String,
    pub command: String,
    /// Shell id ("pwsh", "powershell", "bash", "wsl"…), the system's default one when not found.
    pub shell: String,
    /// Folder relative to the worktree's, empty for the worktree itself.
    pub cwd: String,
}

/// What a new worktree gets from the project when nothing was set: its `.env` files.
pub fn default_worktree_copy() -> Vec<String> {
    vec![".env*".into()]
}

/// Where a ticket stands on the board.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Column {
    /// « À faire »
    #[default]
    Todo,
    /// « En cours »
    Doing,
    /// « À tester »
    Review,
    /// « Terminé »
    Done,
}

/// An acceptance criterion, with what the agent last said of it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Criterion {
    pub text: String,
    pub ok: bool,
    pub note: String,
}

/// A ticket of a project's board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Ticket {
    pub id: String,
    pub project_id: String,
    /// `<PRÉFIXE>-<n>`, e.g. ATL-42.
    pub key: String,
    pub title: String,
    pub description: String,
    pub criteria: Vec<Criterion>,
    /// The features in place so far, as its agent last listed them (3 to 8 short lines); the
    /// agent writes it `avancement`.
    #[serde(alias = "avancement")]
    pub progress: Vec<String>,
    /// 3, 5 or 8.
    pub max_loops: u32,
    pub column: Column,
    /// Order in "À faire": the lowest first.
    pub rank: i64,
    /// The tickets of its project it comes after ("Après"), by id: the autopilot starts it once
    /// they are all "Terminé". Empty for a ticket saved before.
    pub after: Vec<String>,
    pub agent_id: Option<String>,
    /// n of "Boucle n/max" (0 before the start).
    pub iteration: u32,
    /// Every loop its agent began, sent back or not ("Renvoyer", failed tests, a conflict handed
    /// over start again at 1): what "n boucles" says once it is done. 0 for a ticket saved before
    /// they were counted.
    pub loops: u32,
    /// Sent to "À tester" without all its criteria (loop limit).
    pub partial: bool,
    /// Why it no longer moves on by itself.
    pub blocked: Option<String>,
    /// The block is a merge conflict: the card offers "L'agent résout" and "Annuler".
    pub conflict: bool,
    /// The validation step running ("Tests…", "Commit…", "Merge…", "Push…").
    pub step: Option<String>,
    /// What its validation did ("⤵ Mergé dans main · squash"…), and its link.
    pub outcome: Option<String>,
    pub outcome_url: Option<String>,
    /// "Lancer" was clicked while the autopilot is off.
    pub forced: bool,
    /// Its agent was reminded once to end with its report.
    pub reminded: bool,
    /// What its agents cost together (archived ones included), once done.
    pub cost: f64,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub review_at: Option<i64>,
    pub done_at: Option<i64>,
    /// Imported from an external ticket system: the ticket there, kept in step.
    pub external: Option<ExternalRef>,
}

/// A project's board: what validating a ticket does, and its agents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct BoardSettings {
    /// "merge", "pr", "push" or "keep".
    pub action: String,
    /// Where tickets merge, the base of their PR and of their branch. Empty until the first
    /// ticket, which sets the project's current branch.
    pub target: String,
    /// "merge" (merge commit), "squash" or "rebase".
    pub strategy: String,
    pub draft: bool,
    pub tests_first: bool,
    pub test_command: String,
    pub cleanup: bool,
    /// Commit messages written by Haiku in the Conventional Commits form.
    pub conventional: bool,
    /// "ask", "agent" or "abort".
    pub conflict: String,
    /// Tickets "En cours" at once, 1 to 6.
    pub max_parallel: u32,
    /// The ticket agents' model, effort and mode; empty: the app's defaults.
    pub model: String,
    pub effort: String,
    pub mode: String,
    pub autopilot: bool,
    /// The tickets' key prefix, fixed by the first ticket.
    pub prefix: String,
    /// The next ticket's number (never reused).
    pub next_number: u32,
}

impl Default for BoardSettings {
    fn default() -> Self {
        Self {
            action: "merge".into(),
            target: String::new(),
            strategy: "squash".into(),
            draft: false,
            tests_first: false,
            test_command: String::new(),
            cleanup: true,
            conventional: true,
            conflict: "ask".into(),
            max_parallel: 2,
            model: String::new(),
            effort: String::new(),
            mode: String::new(),
            autopilot: true,
            prefix: String::new(),
            next_number: 1,
        }
    }
}

/// A step of a test launch's preparation (`npm install`…), in a folder of the worktree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RecipeStep {
    #[serde(alias = "commande", deserialize_with = "null_default")]
    pub command: String,
    #[serde(alias = "dossier", deserialize_with = "null_default")]
    pub dir: String,
}

/// A process of a test launch (a dev server…) and the address that answers once it is ready.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RecipeProcess {
    #[serde(alias = "nom", deserialize_with = "null_default")]
    pub name: String,
    #[serde(alias = "commande", deserialize_with = "null_default")]
    pub command: String,
    #[serde(alias = "dossier", deserialize_with = "null_default")]
    pub dir: String,
    #[serde(deserialize_with = "env_strings")]
    pub env: BTreeMap<String, String>,
    #[serde(deserialize_with = "null_default")]
    pub url: String,
}

/// How to launch a worktree for a test, as its agent wrote it (key `lancement` of its report).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TestRecipe {
    #[serde(alias = "preparation", deserialize_with = "null_default")]
    pub prepare: Vec<RecipeStep>,
    #[serde(alias = "processus", deserialize_with = "null_default")]
    pub processes: Vec<RecipeProcess>,
    /// The address that shows the feature itself; else the first process's url.
    #[serde(alias = "ouvrir", deserialize_with = "null_default")]
    pub open: String,
}

/// A `null` where the agent had nothing to say is the field's default (empty), not an unreadable
/// recipe.
fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

/// Environment values as strings, whatever the agent wrote (`"PORT": 4110`); a null `env` is an
/// empty one and a null value is skipped.
fn env_strings<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    let raw: Option<BTreeMap<String, Value>> = Option::deserialize(d)?;
    Ok(raw
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| match v {
            Value::Null => None,
            Value::String(s) => Some((k, s)),
            other => Some((k, other.to_string())),
        })
        .collect())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    #[default]
    Idle,
    Running,
    Waiting,
    Done,
    Error,
}

impl AgentStatus {
    pub fn is_active(self) -> bool {
        matches!(self, AgentStatus::Running | AgentStatus::Waiting)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: String,
    pub branch: String,
    pub base_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentMeta {
    pub id: String,
    pub project_id: String,
    pub name: String,
    /// True once the name was generated or chosen by the user.
    pub named: bool,
    pub model: String,
    pub effort: String,
    pub mode: String,
    pub session_id: Option<String>,
    /// The latest entry of its session's main chain (the uuid of its latest assistant message),
    /// saved with the end of each turn: where a copy made now forks its session.
    pub last_entry: Option<String>,
    /// A copy of another agent (« Dupliquer la conversation »): the original's session, which
    /// its starts fork (`--resume <it> --fork-session`) until a turn gives it one of its own.
    pub fork_of: Option<String>,
    /// The original's latest entry when it was copied: the copy's starts fork its session there
    /// (`--resume-session-at`), the turns it ran since left out. None: the whole session.
    pub fork_at: Option<String>,
    pub cwd: String,
    pub worktree: Option<Worktree>,
    pub created_at: i64,
    pub archived: bool,
    pub status: AgentStatus,
    pub tokens: u64,
    pub cost: f64,
    pub active_ms: u64,
    /// Files edited by the agent, relative to its cwd with forward slashes.
    pub touched_files: Vec<String>,
    pub last_activity: i64,
    pub prompts: u32,
    /// Remote Control on: the session is also reachable from claude.ai / the Claude app, so the
    /// agent's process stays up (started with the app, never idle-stopped).
    pub remote_control: bool,
    /// Remote session to reattach to when the process restarts, and its claude.ai link.
    pub remote_session: Option<String>,
    pub remote_url: Option<String>,
    /// Stopped by the usage limit: when the agent is sent "continue" by itself (quota reset).
    pub resume_at: Option<i64>,
    /// The board's ticket this agent works on.
    pub ticket_id: Option<String>,
    /// Appended to Claude Code's system prompt at every start (the ticket's protocol), on one line.
    pub append_prompt: Option<String>,
    /// First of the 10 ports reserved for its test launches.
    pub port_base: Option<u16>,
    /// How to launch its worktree for a test, as it last wrote it.
    pub recipe: Option<TestRecipe>,
    /// The recipe the user read and let run (« Lancer » in the test modal), kept whole rather than
    /// hashed: a recipe that differs in anything, the agent's answer to a later request included,
    /// is asked again, with no collision to forge.
    pub approved_recipe: Option<TestRecipe>,
    /// What the user read and let run for an agent whose services isola runs (`isola up` runs the
    /// commands of its `.isola.toml`, which the agent can write): see `IsolaApproval`.
    pub approved_isola: Option<IsolaApproval>,
    /// The Claude account it runs on (an `Account`'s id), where its session is kept: Principal for
    /// an agent saved before there were accounts.
    #[serde(default = "principal_id")]
    pub account: String,
}

fn principal_id() -> String {
    crate::accounts::PRINCIPAL.into()
}

/// What the user approved of an agent's isola launch: the content of the worktree's `.isola.toml`
/// (the services and setup commands `isola up` runs in their shell) and the address the agent gave to
/// open. Kept whole, like `approved_recipe`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct IsolaApproval {
    pub config: String,
    pub open: String,
}

/// A request waiting for the user's answer, summed up as « Vue d'ensemble » shows it: each text
/// capped (`MAX_PENDING_FIELD` characters), the card in the conversation has it whole.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingView {
    /// The item's id in the conversation, which the answer names.
    pub id: String,
    /// "permission" or "question".
    pub kind: String,
    /// The tool asking for permission (AskUserQuestion for a question).
    pub tool: String,
    /// What the tool is asked to act on, as the conversation's card sums it up (the command,
    /// the file from the agent's folder…), its line breaks kept.
    pub arg: String,
    pub description: Option<String>,
    /// Why Claude Code asks (the permission rule it met).
    pub reason: Option<String>,
    /// A question's questions, with their options' labels.
    pub questions: Vec<PendingQuestion>,
    /// A text, a question or an option was cut: what is shown is not the whole request.
    pub cut: bool,
    /// Claude Code would refuse it by default (`default_to_no`): it is not allowed in one click.
    pub default_no: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingQuestion {
    pub question: String,
    pub options: Vec<String>,
}

/// The most characters a field of a `PendingView` carries.
pub const MAX_PENDING_FIELD: usize = 2000;
/// The most questions, and options of a question, a `PendingView` carries.
pub const MAX_PENDING_ITEMS: usize = 10;

/// Agent as shown by the UI: persisted metadata plus live runtime fields.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    #[serde(flatten)]
    pub meta: AgentMeta,
    pub active_since: Option<i64>,
    pub alive: bool,
    /// Ids of the question/permission items awaiting an answer, in the order they were asked.
    pub pending: Vec<String>,
    /// The same requests summed up, in the same order: what « Vue d'ensemble » shows of them
    /// without loading the conversation.
    pub requests: Vec<PendingView>,
    pub context_tokens: u64,
    /// Size of the context window of the conversation's model (0 until a turn told it).
    pub context_window: u64,
    /// Tokens of the running turn so far (not yet in `tokens`, which the turn's end updates).
    pub live_tokens: u64,
    /// Estimated cost of the running turn so far, from list prices (not yet in `cost`).
    pub live_cost: f64,
    /// Remote Control link state reported by Claude Code ("ready", "connected"…).
    pub remote_state: Option<String>,
    /// What it does right now ("Lit src/db.ts", "Lance npm test"…), during a turn.
    pub activity: Option<String>,
    /// The setup of its new worktree under way: the command running ("npm ci (1/2)").
    pub setup: Option<String>,
    /// isola runs its worktree's services (installed, and an `.isola.toml` in the worktree): its
    /// test launches go through it, with no recipe nor reserved ports.
    pub isola: bool,
}

/// How many of the last lines of the step running a worktree's setup keeps for the window.
pub const SETUP_LINES: usize = 500;

/// What the step of a worktree's setup under way wrote, as the window shows it (its whole output
/// goes to the agent's setup log).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct SetupOutput {
    /// The step's rank, from 0.
    pub step: usize,
    /// How many lines it wrote so far: the window takes the lines it is sent once.
    pub total: usize,
    /// Its last lines, `SETUP_LINES` at most.
    pub lines: std::collections::VecDeque<String>,
}

impl SetupOutput {
    /// Step `step`, before it wrote anything.
    pub fn new(step: usize) -> Self {
        Self {
            step,
            ..Self::default()
        }
    }

    /// `lines`, just written: the oldest beyond `SETUP_LINES` go.
    pub fn push(&mut self, lines: &[String]) {
        self.total += lines.len();
        let kept = &lines[lines.len().saturating_sub(SETUP_LINES)..];
        self.lines.extend(kept.iter().cloned());
        let excess = self.lines.len().saturating_sub(SETUP_LINES);
        self.lines.drain(..excess);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct UiState {
    pub active_project: Option<String>,
    pub view: String,
    pub selected_agent: HashMap<String, String>,
    /// "split" (conversation | files) or "" (classic).
    pub layout: String,
}

/// A choice of Claude Code's model picker (`initialize`): an alias or a full id, and the model
/// it runs (an alias follows the version of Claude Code).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelInfo {
    pub value: String,
    pub resolved_model: String,
}

impl ModelInfo {
    /// The models of an `initialize` response, skipping the entries that say no model.
    pub fn list(reported: &Value) -> Vec<Self> {
        reported
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| serde_json::from_value::<Self>(m.clone()).ok())
            .filter(|m| !m.value.is_empty() && !m.resolved_model.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PersistedState {
    pub projects: Vec<Project>,
    pub agents: Vec<AgentMeta>,
    pub ui: UiState,
    /// Claude Code's models as it last reported them, to label the aliases from the start.
    pub models: Vec<ModelInfo>,
    /// Every project's tickets.
    pub tickets: Vec<Ticket>,
    /// The autopilot's pause as the app stopped: the next start holds the tickets back the same.
    pub pause: SavedPause,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub is_repo: bool,
    /// "(detached)" for a detached HEAD.
    pub branch: String,
    /// The remote branch it tracks ("origin/main"), if any.
    pub upstream: Option<String>,
    /// The upstream no longer exists on the remote (deleted, e.g. once merged).
    pub upstream_gone: bool,
    /// Commits to push / to pull, against the upstream as last fetched.
    pub ahead: u32,
    pub behind: u32,
    pub has_remote: bool,
    /// When the repository was last fetched, ms since epoch.
    pub last_fetch: Option<i64>,
    pub modified: u32,
    pub added: u32,
    pub deleted: u32,
    pub total: u32,
    /// Dirty file count attributed to each agent of the project.
    pub agents: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub status: String,
    pub add: u32,
    pub del: u32,
    pub agent_id: Option<String>,
    /// Listed from `agent_id`'s worktree rather than the project's repository.
    pub in_worktree: bool,
}

/// The unified diff of the files of the files panel that one agent owns in one checkout (the
/// project's own, or the agent's worktree); `agent_id` is `None` for files no agent is credited with.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedDiff {
    pub agent_id: Option<String>,
    /// Read from `agent_id`'s worktree rather than the project's repository.
    pub in_worktree: bool,
    pub diff: String,
}

/// One commit of the repository graph.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    /// Author date, Unix seconds.
    pub time: i64,
    /// Branches and tags pointing at it ("HEAD", "main", "origin/main", "tag: v1.0").
    pub refs: Vec<String>,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLog {
    pub commits: Vec<Commit>,
    /// The branch the agent works on (its worktree's, else the project's current branch).
    pub head: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RateWindow {
    /// 0-100.
    pub pct: f64,
    /// Epoch milliseconds.
    pub resets_at: Option<i64>,
}

/// What holds the autopilot back: a quota window over "Pause au-delà du quota", or the pause
/// after a usage limit with no resume planned.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PauseReason {
    FiveHour,
    Week,
    Limit,
}

/// What holds the autopilot back besides the quota windows read, or no longer does
/// (`board::autopilot_pause`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Hold {
    /// After a usage limit with no resume planned: until then.
    pub limit_until: Option<i64>,
    /// "Reprendre maintenant": the 5-hour and the weekly window hold nothing back until the end
    /// they had then (the next window holds again).
    pub lifted: [Option<i64>; 2],
}

/// The autopilot's pause, kept across a restart: the quota windows last read on each account (a
/// reading holds until its window's end, and with an API key none comes again) and what else
/// holds it.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedPause {
    /// Principal's windows, where the versions before the accounts kept the only ones: a state
    /// they saved gives them to Principal, and a version rolled back still reads them.
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
    pub hold: Hold,
    /// The other accounts' windows, by account.
    pub accounts: BTreeMap<String, SavedWindows>,
}

/// An account's quota windows, as last read.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedWindows {
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
}

/// Why no ticket of any board starts, and until when (`board::autopilot_pause`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutopilotPause {
    pub reason: PauseReason,
    /// The window's use, 0-100 (none after a limit).
    pub pct: Option<f64>,
    /// When the tickets start again: the window's end, or about then after a limit.
    pub until: i64,
}

/// The quotas of every Claude account, as the window is told them (`usage::settle`).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    /// The current account's windows: what the status bar shows and the autopilot goes by.
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
    /// What the turns cost today, on every account.
    pub today_cost: f64,
    /// When the current account's windows were last read, ms since epoch.
    pub updated_at: i64,
    /// Every account, in the settings' order.
    pub accounts: Vec<AccountUsage>,
    /// The account new agents would go to (`accounts::current`).
    pub current: String,
}

/// One Claude account's quota, as last read.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsage {
    pub id: String,
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
    /// Signed in, as the last reading tells (an account not read yet is taken as signed in).
    pub connected: bool,
    /// Why its quota is not read (not signed in, sign-in expired), in the interface's language.
    pub reason: Option<String>,
    /// What its turns cost today (0 until the statistics are kept by account).
    pub today_cost: f64,
    /// When its windows were last read, ms since epoch (0: not in this run).
    pub updated_at: i64,
    /// What `connected` and `reason` tell, kept to write them again in another language.
    #[serde(skip)]
    pub problem: Option<SignInProblem>,
}

/// Why an account's quota cannot be read from the usage endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInProblem {
    /// No sign-in to claude.ai in its folder (an API key, or never signed in).
    NotSignedIn,
    /// Its sign-in is out of date: Claude Code renews it when it runs.
    Expired,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum ConvOp {
    Append {
        item: Value,
    },
    Patch {
        id: String,
        patch: Value,
    },
    /// Text appended to the `text` field of an item (streaming).
    Delta {
        id: String,
        text: String,
    },
}

// Serialized and sent right away: the size gap between variants does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UiEvent {
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
    /// Why no ticket of the project's board starts (its target branch has no commit yet, or is
    /// gone), or None once they may start again.
    #[serde(rename_all = "camelCase")]
    BoardIssue {
        project_id: String,
        issue: Option<String>,
    },
    /// Why no ticket of any board starts for now (a quota, a usage limit), or None once they may
    /// start again.
    AutopilotPause {
        pause: Option<AutopilotPause>,
    },
    /// An address to open in the default browser (a pull request to finish on GitHub).
    OpenUrl {
        url: String,
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
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

/// Merges consecutive text deltas of the same item, keeping every other op in order.
pub fn coalesce_ops(ops: Vec<ConvOp>) -> Vec<ConvOp> {
    let mut out: Vec<ConvOp> = Vec::with_capacity(ops.len());
    for op in ops {
        if let (
            ConvOp::Delta { id, text },
            Some(ConvOp::Delta {
                id: last_id,
                text: last,
            }),
        ) = (&op, out.last_mut())
        {
            if id == last_id {
                last.push_str(text);
                continue;
            }
        }
        out.push(op);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn delta(id: &str, t: &str) -> ConvOp {
        ConvOp::Delta {
            id: id.into(),
            text: t.into(),
        }
    }

    #[test]
    fn settings_saved_with_an_external_editor_still_load() {
        let s: Settings =
            serde_json::from_str(r#"{"editorCommand":"code","sound":false}"#).unwrap();
        assert!(!s.sound);
        // The field is gone: saved again, it is not written back.
        assert!(!serde_json::to_string(&s).unwrap().contains("editorCommand"));
        // Saved before the TLS setting: certificates are checked.
        assert!(!s.insecure_tls);
    }

    #[test]
    fn settings_saved_before_the_languages_follow_the_system_and_the_interface() {
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert_eq!(
            (s.language.as_str(), s.claude_language.as_str()),
            ("system", "ui")
        );
        let d = Settings::default();
        assert_eq!(
            (d.language.as_str(), d.claude_language.as_str()),
            ("system", "ui")
        );
        let v = serde_json::to_value(Settings {
            language: "en".into(),
            claude_language: "fr".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            (&v["language"], &v["claudeLanguage"]),
            (&json!("en"), &json!("fr"))
        );
    }

    #[test]
    fn what_was_saved_before_the_notification_choices_notifies_for_everything() {
        let s: Settings =
            serde_json::from_str(r#"{"sound":false,"osNotifications":true}"#).unwrap();
        assert_eq!(s.notify_for, NotifyFor::default());
        for kind in [
            NotifyKind::Question,
            NotifyKind::Done,
            NotifyKind::Error,
            NotifyKind::Ticket,
        ] {
            assert!(s.notify_for.allows(kind), "{kind:?}");
        }
        // A choice saved alone leaves the others on.
        let s: Settings = serde_json::from_str(r#"{"notifyFor":{"done":false}}"#).unwrap();
        assert_eq!(
            s.notify_for,
            NotifyFor {
                done: false,
                ..NotifyFor::default()
            }
        );
        assert_eq!(
            serde_json::to_value(&s).unwrap()["notifyFor"],
            json!({ "questions": true, "done": false, "errors": true, "tickets": true })
        );
    }

    #[test]
    fn a_notification_kind_is_let_through_by_its_own_choice_only() {
        let all_but = |off: NotifyKind| {
            let mut n = NotifyFor::default();
            match off {
                NotifyKind::Question => n.questions = false,
                NotifyKind::Done => n.done = false,
                NotifyKind::Error => n.errors = false,
                NotifyKind::Ticket => n.tickets = false,
            }
            n
        };
        let kinds = [
            NotifyKind::Question,
            NotifyKind::Done,
            NotifyKind::Error,
            NotifyKind::Ticket,
        ];
        for off in kinds {
            let n = all_but(off);
            for kind in kinds {
                assert_eq!(n.allows(kind), kind != off, "{kind:?} with {off:?} off");
            }
        }
    }

    #[test]
    fn an_agent_saved_before_the_accounts_runs_on_principal() {
        let m: AgentMeta =
            serde_json::from_value(json!({ "id": "a1", "sessionId": "s1" })).unwrap();
        assert_eq!(m.account, "principal");
        let v = serde_json::to_value(AgentMeta {
            account: "pro".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["account"], json!("pro"));
        // Settings saved before them have none: the load puts Principal in (`accounts::normalize`).
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert!(s.accounts.is_empty());
    }

    #[test]
    fn settings_saved_before_the_quota_pause_hold_tickets_back_at_100_percent_only() {
        let s: Settings =
            serde_json::from_value(json!({ "sound": false, "autoResume": false })).unwrap();
        assert_eq!(s.quota_pause, 100);
        assert_eq!(Settings::default().quota_pause, 100);
        let v = serde_json::to_value(Settings {
            quota_pause: 90,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["quotaPause"], json!(90));
    }

    #[test]
    fn a_state_saved_before_the_autopilots_pause_still_loads_without_one() {
        let s: PersistedState = serde_json::from_value(json!({ "projects": [] })).unwrap();
        assert_eq!(s.pause, SavedPause::default());
        let saved = SavedPause {
            five_hour: Some(RateWindow {
                pct: 100.0,
                resets_at: Some(9),
            }),
            seven_day: None,
            hold: Hold {
                limit_until: Some(5),
                lifted: [None, Some(7)],
            },
            ..Default::default()
        };
        let v = serde_json::to_value(PersistedState {
            pause: saved.clone(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["pause"]["fiveHour"]["resetsAt"], json!(9));
        assert_eq!(v["pause"]["hold"]["limitUntil"], json!(5));
        let back: PersistedState = serde_json::from_value(v).unwrap();
        assert_eq!(back.pause, saved);
        // Saved with only part of it.
        let part: SavedPause = serde_json::from_value(json!({ "hold": {} })).unwrap();
        assert_eq!(part, SavedPause::default());
    }

    #[test]
    fn the_quota_windows_are_saved_by_account_and_an_older_state_keeps_principals() {
        let full = RateWindow {
            pct: 100.0,
            resets_at: Some(9),
        };
        // Saved before the accounts: the windows of the only account there was.
        let old: SavedPause =
            serde_json::from_value(json!({ "fiveHour": { "pct": 100.0, "resetsAt": 9 },
                                           "hold": {} }))
            .unwrap();
        assert_eq!(old.five_hour, Some(full));
        assert!(old.accounts.is_empty());
        // The other accounts' beside them, by id.
        let saved = SavedPause {
            seven_day: Some(full),
            accounts: BTreeMap::from([(
                "pro".to_string(),
                SavedWindows {
                    five_hour: Some(full),
                    seven_day: None,
                },
            )]),
            ..Default::default()
        };
        let v = serde_json::to_value(&saved).unwrap();
        assert_eq!(v["sevenDay"]["resetsAt"], json!(9));
        assert_eq!(
            v["accounts"],
            json!({ "pro": { "fiveHour": { "pct": 100.0, "resetsAt": 9 }, "sevenDay": null } })
        );
        assert_eq!(serde_json::from_value::<SavedPause>(v).unwrap(), saved);
    }

    #[test]
    fn the_quotas_of_every_account_travel_in_camel_case() {
        let w = RateWindow {
            pct: 12.0,
            resets_at: Some(5),
        };
        let usage = UsageSnapshot {
            five_hour: Some(w),
            today_cost: 1.5,
            updated_at: 3,
            accounts: vec![AccountUsage {
                id: "pro".into(),
                five_hour: Some(w),
                connected: false,
                reason: Some("Pas connecté".into()),
                updated_at: 3,
                problem: Some(SignInProblem::NotSignedIn),
                ..Default::default()
            }],
            current: "pro".into(),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(&usage).unwrap(),
            json!({ "fiveHour": { "pct": 12.0, "resetsAt": 5 }, "sevenDay": null,
                    "todayCost": 1.5, "updatedAt": 3, "current": "pro",
                    "accounts": [{ "id": "pro", "fiveHour": { "pct": 12.0, "resetsAt": 5 },
                                   "sevenDay": null, "connected": false,
                                   "reason": "Pas connecté", "todayCost": 0.0,
                                   "updatedAt": 3 }] })
        );
    }

    #[test]
    fn the_autopilots_pause_travels_in_camel_case() {
        let e = UiEvent::AutopilotPause {
            pause: Some(AutopilotPause {
                reason: PauseReason::FiveHour,
                pct: Some(100.0),
                until: 5,
            }),
        };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "type": "autopilotPause",
                    "pause": { "reason": "fiveHour", "pct": 100.0, "until": 5 } })
        );
        let over = UiEvent::AutopilotPause { pause: None };
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
    fn settings_saved_before_automatic_updates_install_them() {
        let s: Settings = serde_json::from_str(r#"{"sound":false}"#).unwrap();
        assert!(s.auto_update);
        assert_eq!(serde_json::to_value(&s).unwrap()["autoUpdate"], json!(true));
        let off: Settings = serde_json::from_str(r#"{"autoUpdate":false}"#).unwrap();
        assert!(!off.auto_update);
    }

    #[test]
    fn claude_code_gets_the_proxy_and_skips_tls_verification_only_when_asked() {
        let mut s = Settings::default();
        assert!(s.claude_env().is_empty());
        s.insecure_tls = true;
        assert_eq!(
            s.claude_env(),
            [("NODE_TLS_REJECT_UNAUTHORIZED".to_string(), "0".to_string())]
        );
        s.proxy_url = "http://proxy:3128".into();
        let env = s.claude_env();
        assert!(env.contains(&("HTTPS_PROXY".to_string(), "http://proxy:3128".to_string())));
        assert!(env.contains(&("NODE_TLS_REJECT_UNAUTHORIZED".to_string(), "0".to_string())));
        // The terminals' proxy stays the proxy alone.
        assert!(s
            .proxy_env()
            .iter()
            .all(|(k, _)| k != "NODE_TLS_REJECT_UNAUTHORIZED"));
        assert_eq!(
            serde_json::to_value(&s).unwrap()["insecureTls"],
            json!(true)
        );
    }

    #[test]
    fn consecutive_deltas_of_one_item_are_merged_in_order() {
        let ops = vec![
            delta("a", "Bon"),
            delta("a", "jour"),
            delta("b", "x"),
            delta("a", " !"),
            ConvOp::Patch {
                id: "a".into(),
                patch: json!({ "streaming": false }),
            },
            delta("a", "?"),
        ];
        let out = serde_json::to_value(coalesce_ops(ops)).unwrap();
        assert_eq!(
            out,
            json!([
                { "op": "delta", "id": "a", "text": "Bonjour" },
                { "op": "delta", "id": "b", "text": "x" },
                { "op": "delta", "id": "a", "text": " !" },
                { "op": "patch", "id": "a", "patch": { "streaming": false } },
                { "op": "delta", "id": "a", "text": "?" },
            ])
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
    fn the_models_claude_code_reports_are_read_one_by_one() {
        let reported = json!([
            { "value": "sonnet", "resolvedModel": "claude-sonnet-5-5", "displayName": "Sonnet 5.5" },
            { "value": "odd", "resolvedModel": null },
            "garbage",
            { "value": "bare" },
            { "value": "haiku", "resolvedModel": "claude-haiku-4-5-20251001" },
        ]);
        let values: Vec<String> = ModelInfo::list(&reported)
            .into_iter()
            .map(|m| m.value)
            .collect();
        assert_eq!(values, ["sonnet", "haiku"]);
        assert!(ModelInfo::list(&Value::Null).is_empty());
    }

    #[test]
    fn ui_layout_is_kept_and_defaults_to_classic_for_older_state_files() {
        let ui: UiState =
            serde_json::from_value(json!({ "view": "project", "layout": "split" })).unwrap();
        assert_eq!(serde_json::to_value(&ui).unwrap()["layout"], "split");
        let old: UiState = serde_json::from_value(json!({ "view": "project" })).unwrap();
        assert_eq!(serde_json::to_value(&old).unwrap()["layout"], "");
    }

    #[test]
    fn projects_agents_and_states_saved_before_the_board_still_load() {
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        assert_eq!(p.worktree_copy, vec![".env*".to_string()]);
        assert!(p.worktree_setup.is_empty() && p.worktree_teardown.is_empty());
        assert_eq!(p.board, BoardSettings::default());
        assert_eq!(
            (
                p.board.action.as_str(),
                p.board.strategy.as_str(),
                p.board.conflict.as_str()
            ),
            ("merge", "squash", "ask")
        );
        assert_eq!((p.board.max_parallel, p.board.next_number), (2, 1));
        assert!(p.board.autopilot && p.board.cleanup && p.board.conventional);
        assert!(!p.board.tests_first && !p.board.draft);
        let a: AgentMeta = serde_json::from_value(json!({ "id": "a1", "name": "x" })).unwrap();
        assert_eq!((a.ticket_id, a.port_base, a.recipe), (None, None, None));
        let s: PersistedState = serde_json::from_value(json!({ "projects": [] })).unwrap();
        assert!(s.tickets.is_empty());
    }

    #[test]
    fn a_project_saved_before_the_commit_choice_has_its_agent_write_the_commits() {
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        assert_eq!(p.commit_mode, CommitMode::Agent);
        let direct = Project {
            commit_mode: CommitMode::Direct,
            ..p
        };
        let v = serde_json::to_value(&direct).unwrap();
        assert_eq!(v["commitMode"], json!("direct"));
        let back: Project = serde_json::from_value(v).unwrap();
        assert_eq!(back.commit_mode, CommitMode::Direct);
    }

    #[test]
    fn an_agent_saved_before_recipes_were_approved_loads_with_nothing_approved() {
        // A state.json of 1.5: the agent has a recipe, and no approval was ever asked.
        let s: PersistedState = serde_json::from_value(json!({
            "agents": [{
                "id": "a1",
                "name": "x",
                "recipe": { "prepare": [], "processes": [{ "name": "web", "command": "node web.js" }], "open": "" }
            }]
        }))
        .unwrap();
        assert!(s.agents[0].recipe.is_some());
        assert_eq!(s.agents[0].approved_recipe, None);
        assert_eq!(s.agents[0].approved_isola, None);
    }

    #[test]
    fn an_agent_saved_before_copies_has_no_session_to_fork() {
        // A state.json of 1.5: an agent with a session of its own, and no copy ever made.
        let s: PersistedState = serde_json::from_value(json!({
            "agents": [{ "id": "a1", "name": "x", "sessionId": "s1" }]
        }))
        .unwrap();
        let a = &s.agents[0];
        assert_eq!(a.session_id.as_deref(), Some("s1"));
        // No entry recorded yet: a copy made before its next turn forks its whole session.
        assert_eq!(
            (&a.fork_of, &a.fork_at, &a.last_entry),
            (&None, &None, &None)
        );
        // A copy's are saved with it, in camel case.
        let copy = AgentMeta {
            fork_of: Some("s1".into()),
            fork_at: Some("e1".into()),
            last_entry: Some("e2".into()),
            ..Default::default()
        };
        let v = serde_json::to_value(&copy).unwrap();
        assert_eq!(
            (&v["forkOf"], &v["forkAt"], &v["lastEntry"]),
            (&json!("s1"), &json!("e1"), &json!("e2"))
        );
        let back: AgentMeta = serde_json::from_value(v).unwrap();
        assert_eq!(
            (
                back.fork_of.as_deref(),
                back.fork_at.as_deref(),
                back.last_entry.as_deref()
            ),
            (Some("s1"), Some("e1"), Some("e2"))
        );
    }

    #[test]
    fn the_approved_recipe_is_saved_with_the_agent_and_travels_in_camel_case() {
        let recipe = TestRecipe {
            prepare: vec![RecipeStep {
                command: "npm install".into(),
                dir: "web".into(),
            }],
            open: "http://localhost:4111".into(),
            ..Default::default()
        };
        let state = PersistedState {
            agents: vec![AgentMeta {
                id: "a1".into(),
                recipe: Some(recipe.clone()),
                approved_recipe: Some(recipe.clone()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let saved = serde_json::to_string_pretty(&state).unwrap();
        let back: PersistedState = serde_json::from_str(&saved).unwrap();
        assert_eq!(back.agents[0].approved_recipe, Some(recipe));
        let sent = serde_json::to_value(&state.agents[0]).unwrap();
        assert_eq!(
            sent["approvedRecipe"]["prepare"][0]["command"],
            "npm install"
        );
    }

    #[test]
    fn a_projects_worktree_steps_travel_in_camel_case() {
        let p: Project = serde_json::from_value(json!({
            "id": "p1", "name": "demo", "path": "C:/demo", "color": "red",
            "worktreeSetup": [{ "id": "s1", "command": "npm ci", "shell": "pwsh", "cwd": "web" }],
            "worktreeTeardown": [{ "command": "docker compose down" }],
        }))
        .unwrap();
        assert_eq!(
            p.worktree_setup,
            [WorktreeStep {
                id: "s1".into(),
                command: "npm ci".into(),
                shell: "pwsh".into(),
                cwd: "web".into(),
            }]
        );
        assert_eq!(p.worktree_teardown[0].command, "docker compose down");
        let sent = serde_json::to_value(&p).unwrap();
        assert_eq!(sent["worktreeSetup"][0]["cwd"], "web");
        assert_eq!(sent["worktreeTeardown"][0]["shell"], "");
    }

    #[test]
    fn a_ticket_and_a_recipe_travel_in_camel_case() {
        let t = Ticket {
            id: "t1".into(),
            key: "ATL-42".into(),
            max_loops: 5,
            column: Column::Review,
            agent_id: Some("a1".into()),
            ..Default::default()
        };
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(
            (
                v["maxLoops"].clone(),
                v["column"].clone(),
                v["agentId"].clone()
            ),
            (json!(5), json!("review"), json!("a1"))
        );
        // As the agent writes it, in French (a port may come as a number)…
        let r: TestRecipe = serde_json::from_value(json!({
            "preparation": [{ "commande": "npm install", "dossier": "web" }],
            "processus": [{ "nom": "api", "commande": "npm run dev", "env": { "PORT": 4110 },
                            "url": "http://localhost:4110" }],
            "ouvrir": "http://localhost:4111/connexion"
        }))
        .unwrap();
        assert_eq!(r.prepare[0].dir, "web");
        assert_eq!(r.processes[0].env["PORT"], "4110");
        assert_eq!(r.open, "http://localhost:4111/connexion");
        // …sent to the window in English.
        let sent = serde_json::to_value(&r).unwrap();
        assert_eq!(sent["processes"][0]["name"], "api");
        assert_eq!(sent["prepare"][0]["command"], "npm install");
    }

    #[test]
    fn a_ticket_saved_before_the_progress_still_loads_and_keeps_it_in_english() {
        let t: Ticket = serde_json::from_value(json!({ "id": "t1", "key": "ATL-42" })).unwrap();
        assert!(t.progress.is_empty());
        // As the agent names it, in French…
        let t: Ticket =
            serde_json::from_value(json!({ "id": "t1", "avancement": ["a", "b"] })).unwrap();
        assert_eq!(t.progress, ["a", "b"]);
        // …and as it is saved and sent to the window.
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["progress"], json!(["a", "b"]));
        assert!(v.get("avancement").is_none());
        let back: Ticket = serde_json::from_value(v).unwrap();
        assert_eq!(back, t);
        // A new ticket has none to show.
        assert_eq!(
            serde_json::to_value(Ticket::default()).unwrap()["progress"],
            json!([])
        );
    }

    #[test]
    fn what_was_saved_before_the_integrations_still_loads_with_their_defaults() {
        let s: Settings = serde_json::from_value(json!({ "sound": false })).unwrap();
        assert_eq!(s.integrations, IntegrationSettings::default());
        assert!(s.integrations.sync_states && s.integrations.extract_criteria);
        assert!(!s.integrations.auto_import && !s.integrations.loop_comments);
        assert_eq!(
            (
                s.integrations.import_label.as_str(),
                s.integrations.import_every
            ),
            ("claude-ready", 15)
        );
        let p: Project = serde_json::from_value(
            json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red" }),
        )
        .unwrap();
        assert!(p.integrations.links.is_empty());
        assert_eq!(p.integrations.comments, [Column::Review, Column::Done]);
        let t: Ticket = serde_json::from_value(json!({ "id": "t1", "key": "ATL-42" })).unwrap();
        assert_eq!(t.external, None);
    }

    #[test]
    fn a_links_states_travel_by_column_and_an_external_ref_in_camel_case() {
        let mut states = BTreeMap::new();
        states.insert(
            Column::Doing,
            ExternalState {
                id: "3".into(),
                name: "In Progress".into(),
            },
        );
        let link = SourceLink {
            service: Service::Jira,
            container: "ATL".into(),
            name: "Atlas".into(),
            states,
        };
        let v = serde_json::to_value(&link).unwrap();
        assert_eq!(v["service"], "jira");
        assert_eq!(v["states"]["doing"]["name"], "In Progress");
        let back: SourceLink = serde_json::from_value(v).unwrap();
        assert_eq!(back, link);
        let r = ExternalRef {
            service: Service::Github,
            id: "42".into(),
            key: "#42".into(),
            container: "acme/api".into(),
            url: "https://github.com/acme/api/issues/42".into(),
            error: None,
        };
        let t = serde_json::to_value(Ticket {
            external: Some(r),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(t["external"]["service"], "github");
        assert_eq!(t["external"]["container"], "acme/api");
    }

    #[test]
    fn a_ticket_saved_before_its_loops_were_counted_still_loads_with_none() {
        let t: Ticket =
            serde_json::from_value(json!({ "id": "t1", "key": "ATL-42", "iteration": 3 })).unwrap();
        assert_eq!((t.loops, t.iteration), (0, 3));
        let v = serde_json::to_value(Ticket {
            loops: 4,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["loops"], json!(4));
    }

    #[test]
    fn a_state_saved_before_dependencies_still_loads_with_tickets_waiting_for_none() {
        // A state.json of 1.5: its tickets have no `after`.
        let s: PersistedState = serde_json::from_value(json!({
            "projects": [],
            "tickets": [{ "id": "t1", "projectId": "p1", "key": "DEM-1", "column": "todo" }]
        }))
        .unwrap();
        assert!(s.tickets[0].after.is_empty());
        let v = serde_json::to_value(Ticket {
            after: vec!["t2".into(), "t3".into()],
            ..Default::default()
        })
        .unwrap();
        assert_eq!(v["after"], json!(["t2", "t3"]));
        let back: Ticket = serde_json::from_value(v).unwrap();
        assert_eq!(back.after, ["t2", "t3"]);
    }

    #[test]
    fn a_null_env_or_a_null_value_in_it_does_not_spoil_the_recipe() {
        let r: TestRecipe = serde_json::from_value(json!({
            "processus": [
                { "nom": "web", "commande": "npm run dev", "env": null },
                { "nom": "api", "commande": "npm start",
                  "env": { "PORT": 4110, "DEBUG": null, "NAME": "x" } }
            ]
        }))
        .unwrap();
        assert!(r.processes[0].env.is_empty());
        let env = &r.processes[1].env;
        assert_eq!(
            (env.len(), env["PORT"].as_str(), env["NAME"].as_str()),
            (2, "4110", "x")
        );
    }

    #[test]
    fn a_null_text_or_list_in_the_recipe_is_an_absent_one() {
        let r: TestRecipe = serde_json::from_value(json!({
            "preparation": [{ "commande": "npm install", "dossier": null },
                            { "commande": null, "dossier": "web" }],
            "processus": [{ "nom": null, "commande": "npm run dev", "dossier": null,
                            "env": null, "url": null }],
            "ouvrir": null
        }))
        .unwrap();
        assert_eq!(
            r.prepare,
            [
                RecipeStep {
                    command: "npm install".into(),
                    dir: String::new()
                },
                RecipeStep {
                    command: String::new(),
                    dir: "web".into()
                }
            ]
        );
        let p = &r.processes[0];
        assert_eq!(p.command, "npm run dev");
        assert!(p.name.is_empty() && p.dir.is_empty() && p.url.is_empty() && p.env.is_empty());
        assert!(r.open.is_empty());
        // The lists themselves may be null.
        let r: TestRecipe = serde_json::from_value(json!({
            "preparation": null, "processus": null, "ouvrir": null
        }))
        .unwrap();
        assert_eq!(r, TestRecipe::default());
        // The English names the window sends back are as tolerant.
        let r: TestRecipe = serde_json::from_value(json!({
            "prepare": null, "processes": [{ "name": null, "url": null }], "open": null
        }))
        .unwrap();
        assert_eq!(r.processes, [RecipeProcess::default()]);
    }
}

#[cfg(test)]
mod run_command_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn projects_saved_before_launch_commands_still_load() {
        let p: Project = serde_json::from_value(json!({
            "id": "p1", "name": "demo", "path": "C:/demo", "color": "red"
        }))
        .unwrap();
        assert!(p.run_commands.is_empty());
        let with = json!({ "id": "p1", "name": "demo", "path": "C:/demo", "color": "red",
            "runCommands": [{ "id": "c1", "name": "Front", "command": "npm run dev", "shell": "pwsh", "cwd": "web" }] });
        let p: Project = serde_json::from_value(with.clone()).unwrap();
        assert_eq!(p.run_commands[0].cwd, "web");
        assert_eq!(
            serde_json::to_value(&p).unwrap()["runCommands"],
            with["runCommands"]
        );
    }
}
