//! Types persisted on disk and exchanged with the frontend.

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
    /// Send "continue" by itself to an agent stopped by the usage limit, once the quota resets.
    pub auto_resume: bool,
}

impl Settings {
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
            idle_stop_minutes: 30,
            pwsh_path: String::new(),
            bash_path: String::new(),
            wsl_distro: String::new(),
            proxy_url: String::new(),
            no_proxy: "localhost,127.0.0.1".into(),
            proxy_terminals: false,
            auto_resume: true,
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

/// What a new worktree gets from the project when nothing was set: its `.env` files.
pub fn default_worktree_copy() -> Vec<String> {
    vec![".env*".into()]
}

/// Where a ticket stands on the board.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
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
    /// 3, 5 or 8.
    pub max_loops: u32,
    pub column: Column,
    /// Order in "À faire": the lowest first.
    pub rank: i64,
    pub agent_id: Option<String>,
    /// n of "Boucle n/max" (0 before the start).
    pub iteration: u32,
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
    /// What its agent cost, once done.
    pub cost: f64,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub review_at: Option<i64>,
    pub done_at: Option<i64>,
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
}

/// Agent as shown by the UI: persisted metadata plus live runtime fields.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    #[serde(flatten)]
    pub meta: AgentMeta,
    pub active_since: Option<i64>,
    pub alive: bool,
    /// Ids of the question/permission items awaiting an answer.
    pub pending: Vec<String>,
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

#[derive(Debug, Clone, Copy, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateWindow {
    /// 0-100.
    pub pct: f64,
    /// Epoch milliseconds.
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
    pub today_cost: f64,
    pub updated_at: i64,
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
