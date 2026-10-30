//! The tools the MCP server offers (`Tools`, rmcp's router), and what each one works with: the
//! app's core and who calls (`ToolCtx`, from the request's token).
//!
//! A tool is a method of the `#[tool_router]` block below. It takes a `ToolCtx<R>` (any position
//! among its parameters), its arguments as `Parameters<T>` (`T: Deserialize + JsonSchema`, the
//! schema the model reads, each field's `#[schemars(description)]` its argument's), and answers
//! through `ToolCtx::reply`, which logs the call (`activity::record`) and turns a refusal or a
//! failure into an error result the model reads:
//!
//! ```text
//! #[tool(description = "List the projects…")]
//! async fn list_projects(&self, ctx: ToolCtx<R>) -> CallToolResult {
//!     let projects = …;
//!     ctx.reply("list_projects", "", Ok(serde_json::to_string(&projects)?))
//! }
//! ```
//!
//! `get_ticket_image` answers more than a text (an image block, then a text): `ToolCtx::reply_blocks`.
//! `get_ticket_file` reads a ticket's other files part by part (`files`).
//!
//! A call the router refuses before its tool (unknown, arguments that do not fit the schema) is
//! logged by `call_tool`. What a tool's arguments name is found by `resolve`; what the reading
//! ones answer is built by `read` (`wait_ticket`, which waits for a change, by `wait`), and what
//! the ones that act do is in `act`.
//!
//! None runs a command of its own. The ones of the board's flow do what the window's buttons do,
//! through the same functions of the core: `approve_ticket` runs the tests (the command the user
//! gave) and merges (or pushes), and `delete_ticket` deletes, each refused where the window refuses.
//! Two answer an agent's request as the user does in the window (`answer_permission`,
//! `answer_question`), and never for good: a lasting permission rule (« Toujours autoriser ») stays
//! the user's to give there. `set_board` changes what the window's Kanban settings do, but not what
//! runs without a question: the mode that asks for no permission at all (`bypassPermissions`) and
//! the test command stay the user's to give there too. The subagents and background tasks of an
//! agent (`background`) are listed by anyone, read where the caller acts (`act::may_act_on`), stopped
//! (`stop_background_work`) and written to (`message_subagent`) as the window's « Arrêter » and
//! message box do, through the same functions of the core and refused where it refuses; a
//! message's words never reach the log.
//! `tests::EXPOSED` lists exactly what the router holds, so a tool is added knowingly.

use super::act;
use super::activity::{self, Outcome};
use super::background;
use super::docs;
use super::files;
use super::images;
use super::index;
use super::memory;
use super::read;
use super::wait;
use super::workflow;
use super::Caller;
use crate::core::Core;
use crate::model::Column;
use rmcp::handler::server::common::{AsRequestContext, FromContextPart};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock};
use rmcp::service::RequestContext;
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler};
use serde::Deserialize;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Runtime;

/// What a tool works with: the app's core, and who calls it (from the request's token, set by
/// `http::Gate` in the request's extensions).
pub struct ToolCtx<R: Runtime> {
    pub core: Arc<Core<R>>,
    pub caller: Caller,
    /// The call was logged (`reply`): shared by the copies rmcp hands out for one request.
    logged: Arc<AtomicBool>,
}

impl<R: Runtime> Clone for ToolCtx<R> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            caller: self.caller.clone(),
            logged: self.logged.clone(),
        }
    }
}

/// Why a tool did not do what it was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// A guard said no (the autopilot's pause, a tool reserved to agents…).
    Refused(String),
    /// It failed (a name that fits nothing, or several things…).
    Failed(String),
}

impl<R: Runtime> ToolCtx<R> {
    /// What a request admitted from `caller` works with.
    pub fn new(core: Arc<Core<R>>, caller: Caller) -> Self {
        Self {
            core,
            caller,
            logged: Arc::default(),
        }
    }

    /// The call's: from the request's HTTP parts, which rmcp keeps in the call's extensions.
    fn of(context: &RequestContext<RoleServer>) -> Option<Self> {
        context
            .extensions
            .get::<hyper::http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<ToolCtx<R>>())
            .cloned()
    }

    /// The answer to a call of `tool` (asked `summary`, never a token), logged: `Ok` is the text
    /// the model reads (JSON), a refusal or a failure an error result that says why.
    pub fn reply(
        &self,
        tool: &str,
        summary: &str,
        result: Result<String, ToolError>,
    ) -> CallToolResult {
        self.answer(
            tool,
            summary,
            result.map(|text| (vec![ContentBlock::text(text)], Outcome::Ok)),
        )
    }

    /// `reply`, for a tool whose answer is more than a text (`get_ticket_image`: an image, then a
    /// text). The log has the call as for the others, summary and outcome: never the blocks.
    pub fn reply_blocks(
        &self,
        tool: &str,
        summary: &str,
        result: Result<Vec<ContentBlock>, ToolError>,
    ) -> CallToolResult {
        self.answer(tool, summary, result.map(|blocks| (blocks, Outcome::Ok)))
    }

    /// `reply`, for a tool that also says what it changed: `Ok` is the text the model reads and,
    /// for the log, what the call changed (its message, which a refusal or a failure keeps as it
    /// says why).
    pub fn reply_changed(
        &self,
        tool: &str,
        summary: &str,
        result: Result<(String, String), ToolError>,
    ) -> CallToolResult {
        self.answer(
            tool,
            summary,
            result
                .map(|(text, changed)| (vec![ContentBlock::text(text)], Outcome::Changed(changed))),
        )
    }

    /// The answer, and the log's entry: `Ok` is the blocks and how the log says it ended.
    fn answer(
        &self,
        tool: &str,
        summary: &str,
        result: Result<(Vec<ContentBlock>, Outcome), ToolError>,
    ) -> CallToolResult {
        let (outcome, answer) = match result {
            Ok((blocks, outcome)) => (outcome, CallToolResult::success(blocks)),
            Err(ToolError::Refused(m)) => (
                Outcome::Refused(m.clone()),
                CallToolResult::error(vec![ContentBlock::text(m)]),
            ),
            Err(ToolError::Failed(m)) => (
                Outcome::Error(m.clone()),
                CallToolResult::error(vec![ContentBlock::text(m)]),
            ),
        };
        activity::record(&self.core, Some(&self.caller), tool, summary, outcome);
        self.logged.store(true, Ordering::Release);
        answer
    }
}

/// A tool's `ToolCtx` parameter.
impl<C: AsRequestContext, R: Runtime> FromContextPart<C> for ToolCtx<R> {
    fn from_context_part(context: &mut C) -> Result<Self, ErrorData> {
        Self::of(context.as_request_context())
            .ok_or_else(|| ErrorData::internal_error("no caller for this request", None))
    }
}

/// The blocks of an answer of the index: its text, then its metrics line when there is one.
fn index_blocks((text, line): index::Reply) -> Vec<ContentBlock> {
    let mut blocks = vec![ContentBlock::text(text)];
    if !line.is_empty() {
        blocks.push(ContentBlock::text(line));
    }
    blocks
}

/// What the activity log says a call was asked: its arguments given, « name: value · … ».
fn summary(args: &[(&str, Option<&str>)]) -> String {
    args.iter()
        .filter_map(|(name, value)| value.map(|v| format!("{name}: {v}")))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The MCP server's handler: rmcp makes one per request (`http::Gate`), from a router built once.
pub struct Tools<R: Runtime> {
    router: ToolRouter<Self>,
    _runtime: PhantomData<fn() -> R>,
}

impl<R: Runtime> Tools<R> {
    pub fn new(router: ToolRouter<Self>) -> Self {
        Self {
            router,
            _runtime: PhantomData,
        }
    }

    /// The schemas of the index's tools, as the client lists them, in tokens: what a session pays
    /// once for them (tokensave `schema_overhead_tokens`). The same from one call to the next.
    fn index_schemas(&self) -> u64 {
        static TOKENS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
        *TOKENS.get_or_init(|| {
            crate::codeindex::accounting::schema_overhead_tokens(&index_tools(&self.router))
        })
    }

    /// Every tool the server offers (and, in tests, the test's own).
    pub fn router() -> ToolRouter<Self> {
        let router = Self::tool_router();
        #[cfg(test)]
        let router = router + Self::test_router();
        router
    }
}

/// The index's tools of `router`, as a client lists them (also served by the bench of the real
/// agents, `codeindex::probes::agents`).
pub(crate) fn index_tools<R: Runtime>(router: &ToolRouter<Tools<R>>) -> serde_json::Value {
    let tools: Vec<_> = router
        .list_all()
        .into_iter()
        .filter(|t| t.name.starts_with("index_"))
        .collect();
    serde_json::to_value(tools).unwrap_or_default()
}

// The arguments' descriptions are the model's (English, one line each: a doc comment's line
// breaks would stay in them).

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListAgentsArgs {
    #[schemars(
        description = "Only the agents of this project: its id, or its name in any case. Leave it out for every project's."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTicketsArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
    #[schemars(
        description = "Only the tickets of this column of the board: todo (to do), doing (in progress), review (to test) or done. Leave it out for every column's."
    )]
    pub column: Option<ColumnArg>,
}

/// A column of the board, as a tool's argument names it.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum ColumnArg {
    Todo,
    Doing,
    Review,
    Done,
}

impl From<ColumnArg> for Column {
    fn from(c: ColumnArg) -> Self {
        match c {
            ColumnArg::Todo => Column::Todo,
            ColumnArg::Doing => Column::Doing,
            ColumnArg::Review => Column::Review,
            ColumnArg::Done => Column::Done,
        }
    }
}

impl PositionArg {
    pub(super) fn name(self) -> &'static str {
        match self {
            PositionArg::Top => "top",
            PositionArg::Bottom => "bottom",
            PositionArg::Before => "before",
        }
    }
}

impl ColumnArg {
    fn name(self) -> &'static str {
        match self {
            ColumnArg::Todo => "todo",
            ColumnArg::Doing => "doing",
            ColumnArg::Review => "review",
            ColumnArg::Done => "done",
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTicketArgs {
    #[schemars(
        description = "The ticket: its key (like ATL-12, in any case) or its id. An agent of Escouade may leave it out to read its own ticket."
    )]
    pub ticket: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTicketImageArgs {
    #[schemars(
        description = "The ticket: its key (like ATL-12, in any case) or its id. An agent of Escouade may leave it out to read an image of its own ticket."
    )]
    pub ticket: Option<String>,
    #[schemars(
        description = "The image's file, as get_ticket gives it. Give this or index, not both."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "The image's rank in the list get_ticket gives, from 0. Give this or file, not both."
    )]
    pub index: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTicketFileArgs {
    #[schemars(
        description = "The ticket: its key (like ATL-12, in any case) or its id. An agent of Escouade may leave it out to read a file of its own ticket."
    )]
    pub ticket: Option<String>,
    #[schemars(
        description = "The file's file, as get_ticket gives it (files). Give this or index, not both."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "The file's rank in the list get_ticket gives (files), from 0. Give this or file, not both."
    )]
    pub index: Option<u32>,
    #[schemars(
        description = "Where to start reading, in bytes from the start of the file (0 for its start). Leave it out to read its end."
    )]
    pub offset: Option<u64>,
    #[schemars(
        description = "The most bytes to read: 8192 by default, 1 at least and 65536 at most."
    )]
    pub bytes: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetAgentSummaryArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListBackgroundWorkArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBackgroundOutputArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The subagent or task to read: its id, as list_background_work gives it (id). A subagent may also be named by its taskId."
    )]
    pub id: String,
    #[schemars(
        description = "The most bytes of the output to give, its end: 8192 by default, 1 at least and 65536 at most (a task that still runs gives 8192 at most). Not for a subagent's report, which is given whole."
    )]
    pub bytes: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StopBackgroundWorkArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The subagent or task to stop: its id, as list_background_work gives it (id). A subagent may also be named by its taskId."
    )]
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MessageSubagentArgs {
    #[schemars(description = "The agent whose subagent it is: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The subagent to write to: its id, as list_background_work gives it (id). It may also be named by its taskId."
    )]
    pub subagent: String,
    #[schemars(
        description = "The message, 32,000 characters at most. The agent is asked to pass it on to the subagent, which reads it headed by your name (Message from <you>: …), as coming from you and not from the user."
    )]
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListRequestsArgs {
    #[schemars(
        description = "Only this agent's requests: its id, or its name in any case. Leave it out for every agent's."
    )]
    pub agent: Option<String>,
    #[schemars(
        description = "Only the requests of this project's agents (and the project to look for the agent's name in: two projects may each have an agent of that name): its id, or its name in any case. Leave it out for every project's."
    )]
    pub project: Option<String>,
}

/// How `answer_permission` answers. `Always` is no choice the schema offers (nor the error of a
/// decision that is none lists it): a lasting permission rule is the user's to give, in the window,
/// so it is read to be refused with the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, schemars::JsonSchema)]
#[schemars(rename_all = "lowercase", inline)]
pub enum DecisionArg {
    Allow,
    Deny,
    #[schemars(skip)]
    Always,
}

impl<'de> Deserialize<'de> for DecisionArg {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let said = String::deserialize(deserializer)?;
        match said.as_str() {
            "allow" => Ok(DecisionArg::Allow),
            "deny" => Ok(DecisionArg::Deny),
            "always" => Ok(DecisionArg::Always),
            other => Err(serde::de::Error::custom(format!(
                "unknown decision `{other}`, expected `allow` or `deny`"
            ))),
        }
    }
}

impl DecisionArg {
    pub(super) fn name(self) -> &'static str {
        match self {
            DecisionArg::Allow => "allow",
            DecisionArg::Deny => "deny",
            DecisionArg::Always => "always",
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnswerPermissionArgs {
    #[schemars(description = "The agent that asks: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for the agent's name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The request to answer: its id, as list_requests gives it (request)."
    )]
    pub request: String,
    #[schemars(
        description = "allow: the tool runs (or the plan is approved); deny: it does not (or the agent keeps planning). Always allow is not offered: a lasting permission rule is the user's to give, in the window."
    )]
    pub decision: DecisionArg,
    #[schemars(
        description = "Only with deny: what the agent should know of the refusal, or do instead. Leave it out to refuse without a word."
    )]
    pub message: Option<String>,
}

/// One question's answer, for `answer_question`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct QuestionAnswer {
    #[schemars(
        description = "The question this answers: its text, as list_requests gives it. Leave it out when the request asks a single question."
    )]
    pub question: Option<String>,
    #[schemars(
        description = "The options picked, by their labels as list_requests gives them: one, or several when the question allows it (multiSelect). Give this or text, not both."
    )]
    pub choices: Option<Vec<String>>,
    #[schemars(
        description = "A free answer, when no option fits. Give this or choices, not both."
    )]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnswerQuestionArgs {
    #[schemars(description = "The agent that asks: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for the agent's name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The request to answer: its id, as list_requests gives it (request)."
    )]
    pub request: String,
    #[schemars(
        description = "One answer for each of the request's questions: the options picked (choices) or a free text."
    )]
    pub answers: Vec<QuestionAnswer>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WaitTicketArgs {
    #[schemars(
        description = "The tickets to watch (20 at most): their keys (like ATL-12, in any case) or their ids."
    )]
    pub tickets: Vec<String>,
    #[schemars(
        description = "The most seconds to wait: 120 by default, 240 at most (a longer wait is cut to it), 1 at least."
    )]
    pub timeout: Option<u64>,
    #[schemars(
        description = "The since the previous wait_ticket answered, or that an action on the ticket gave (approve_ticket, reject_ticket, resume_ticket, resolve_conflict, dismiss_block: the ticket as it was before), as it was given, even across other tool calls: a ticket whose state differs from the one it holds is answered at once, a change made between the two calls included. Leave it out and each ticket's state at the call is the one compared."
    )]
    pub since: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTicketArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
    #[schemars(description = "The ticket's title, on one line (200 characters at most).")]
    pub title: String,
    #[schemars(
        description = "What to do, in a few sentences or paragraphs (10,000 characters at most). Leave it out for none."
    )]
    pub description: Option<String>,
    #[schemars(
        description = "The acceptance criteria, one per item (30 at most, 500 characters each): the ticket's agent checks each one in its report. Leave them out for two default ones."
    )]
    pub criteria: Option<Vec<String>>,
    #[schemars(
        description = "The tickets of the same project this one comes after, by key (like ATL-12, in any case) or id: it starts once all are done. Leave it out for none."
    )]
    pub after: Option<Vec<String>>,
    #[schemars(
        description = "Images to attach (screenshots, mock-ups; 20 at most, 5 MB each and 18 MB in all: PNG, JPEG, GIF or WebP, read from their content), which the ticket's agent gets with its first message. Leave them out for none."
    )]
    pub images: Option<Vec<ImageArg>>,
    #[schemars(
        description = "Other files to attach, of any format (a spreadsheet, an archive, a log, a PDF…; 20 at most, each no bigger than the size the settings allow, Settings › Harness › Attachments), whose paths the ticket's agent gets with its first message. Leave them out for none."
    )]
    pub files: Option<Vec<FileArg>>,
    #[schemars(
        description = "The account its agents start on, every time: its name (in any case) or its id, or auto (the default), the current account of the ticket's harness. Set on an account, its agents stay there: at its usage limit or past the threshold to change accounts, they wait for its reset. An account its project may not use is refused, with why."
    )]
    pub account: Option<String>,
    #[schemars(
        description = "The model of its agents, as the window's model menu offers it for the harness of the ticket, else of its account, else of its project (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives the project's models), or auto (the default): the Kanban's, else its account's."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The effort of its agents (low, medium, high, xhigh or max: those of the harness and of the model, see list_projects), or auto (the default): the Kanban's, else its account's (a model that takes none, Haiku, goes without)."
    )]
    pub effort: Option<String>,
    #[schemars(
        description = "Share it with the team (true): Escouade writes its file .escouade/tickets/<id>.yml in the project's folder and keeps its status up to date; the user commits it, Escouade never does. Left out: as the project's « Partager chaque nouveau ticket » says. Refused while the user has given no name."
    )]
    pub shared: Option<bool>,
}

/// A file to attach to a ticket, as `create_ticket` and `update_ticket` take it.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct FileArg {
    #[schemars(
        description = "Its name (only what follows the last slash is kept, made safe as the window makes it). Required with data; with path it is the file's name by default."
    )]
    pub name: Option<String>,
    #[schemars(
        description = "Its content in base64, for a small file (a request weighs 32 MiB at most: give a bigger file by its path). Give this or path, not both."
    )]
    pub data: Option<String>,
    #[schemars(
        description = "The full path of a file on this computer, which is copied. An agent of Escouade can only give a file of its working folder (a safeguard of the server, not a security boundary). Give this or data, not both."
    )]
    pub path: Option<String>,
}

/// A file of a ticket, as the tools that name one take it.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct FileRef {
    #[schemars(
        description = "The file's file, as get_ticket gives it (files). Give this or index, not both."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "The file's rank in the list get_ticket gives (files), from 0. Give this or file, not both."
    )]
    pub index: Option<u32>,
}

/// An image to attach to a ticket, as `create_ticket`, `update_ticket` and the like take it.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(inline)]
pub struct ImageArg {
    #[schemars(
        description = "Its name (100 characters at most; only what follows the last slash is kept). Required with data; with path it is the file's name by default."
    )]
    pub name: Option<String>,
    #[schemars(
        description = "Its media type, as an indication only: the image's type is the one its content has (PNG, JPEG, GIF or WebP)."
    )]
    pub media_type: Option<String>,
    #[schemars(description = "Its content in base64. Give this or path, not both.")]
    pub data: Option<String>,
    #[schemars(
        description = "The full path of an image file on this computer (5 MB at most). An agent of Escouade can only give a file of its working folder (a safeguard of the server, not a security boundary). Give this or data, not both."
    )]
    pub path: Option<String>,
}

/// An image of a ticket, as the tools that name one take it.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct ImageRef {
    #[schemars(
        description = "The image's file, as get_ticket gives it. Give this or index, not both."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "The image's rank in the list get_ticket gives, from 0. Give this or file, not both."
    )]
    pub index: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTicketArgs {
    #[schemars(description = "The ticket: its key (like ATL-12, in any case) or its id.")]
    pub ticket: String,
    #[schemars(
        description = "The new title, on one line (200 characters at most). Leave it out to keep the current one."
    )]
    pub title: Option<String>,
    #[schemars(
        description = "The new description (10,000 characters at most, not counting the line saying who edited it, which is added after it). Leave it out to keep the current one."
    )]
    pub description: Option<String>,
    #[schemars(
        description = "The new acceptance criteria, one per item, replacing all the current ones (an empty list gives the two default ones). Leave it out to keep the current ones."
    )]
    pub criteria: Option<Vec<String>>,
    #[schemars(
        description = "The tickets of the same project it now comes after, by key or id, replacing all the current ones (an empty list takes them all away). Leave it out to keep the current ones."
    )]
    pub after: Option<Vec<String>>,
    #[schemars(
        description = "Images to attach, added after the ones the ticket has (same form and limits as create_ticket's images: 20 images in all at most). Leave it out to add none."
    )]
    pub add_images: Option<Vec<ImageArg>>,
    #[schemars(
        description = "Images of the ticket to take away, each named by its file or its rank in the list get_ticket gives. The images you do not name stay. Leave it out to take none away."
    )]
    pub remove_images: Option<Vec<ImageRef>>,
    #[schemars(
        description = "Other files to attach, added after the ones the ticket has (same form and limits as create_ticket's files: 20 files in all at most). Leave it out to add none."
    )]
    pub add_files: Option<Vec<FileArg>>,
    #[schemars(
        description = "Files of the ticket to take away, each named by its file or its rank in the list get_ticket gives (files). The files you do not name stay. Leave it out to take none away."
    )]
    pub remove_files: Option<Vec<FileRef>>,
    #[schemars(
        description = "The account its agents start on, every time: its name (in any case) or its id, or auto, the current account of the ticket's harness. Set on an account, its agents stay there: at its usage limit or past the threshold to change accounts, they wait for its reset. An account its project may not use is refused, with why. Leave it out to keep it."
    )]
    pub account: Option<String>,
    #[schemars(
        description = "The model of its agents, as the window's model menu offers it for the harness of the ticket, else of its account, else of its project (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives the project's models), or auto: the Kanban's, else its account's. Leave it out to keep it."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The effort of its agents (low, medium, high, xhigh or max: those of the harness and of the model, see list_projects), or auto: the Kanban's, else its account's (a model that takes none, Haiku, goes without). Leave it out to keep it."
    )]
    pub effort: Option<String>,
    #[schemars(
        description = "true shares it with the team (its file .escouade/tickets/<id>.yml, committed by the user), false stops sharing it (its file is deleted, to be committed). Any column. Leave it out to keep it."
    )]
    pub shared: Option<bool>,
}

/// Where `move_ticket` puts a ticket.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum PositionArg {
    Top,
    Bottom,
    Before,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveTicketArgs {
    #[schemars(description = "The ticket to move: its key (like ATL-12, in any case) or its id.")]
    pub ticket: String,
    #[schemars(
        description = "Where it goes in the To do column: top (first), bottom (last) or before (just before the ticket named by before)."
    )]
    pub position: PositionArg,
    #[schemars(
        description = "Only with position before: the To do ticket of the same project to put this one just before, by key or id."
    )]
    pub before: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartTicketArgs {
    #[schemars(
        description = "The ticket to launch: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
    #[schemars(
        description = "The account this ticket's agent starts on: its name (in any case) or its id, or auto for the current account of its harness. It is the ticket's from then on, as its model and effort are, and takes precedence over the one it had. Leave it out to leave it as it is."
    )]
    pub account: Option<String>,
    #[schemars(
        description = "The model of this ticket's agent, as its harness names it (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives the project's models), for this ticket alone: it takes precedence over the Kanban's, e.g. keep opus on an architecture ticket while the Kanban is on sonnet. An empty string, or auto, goes back to the Kanban's. Leave it out to leave it as it is."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The effort of this ticket's agent (low, medium, high, xhigh or max: those of its harness and model, see list_projects), for this ticket alone: it takes precedence over the Kanban's (a model that takes none, Haiku, goes without). An empty string, or auto, goes back to the Kanban's. Leave it out to leave it as it is."
    )]
    pub effort: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetWorkflowArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
}

const MEMORY_PROJECT: &str = "The project: its id, or its name in any case. An agent of Escouade may leave it out for its own.";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchMemoryArgs {
    #[schemars(description = MEMORY_PROJECT)]
    pub project: Option<String>,
    #[schemars(
        description = "What to look for, in a few words (any language). Empty: the memories by name, up to limit."
    )]
    pub query: String,
    #[schemars(description = "The most memories answered: 10 when left out, 50 at most.")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadMemoryArgs {
    #[schemars(description = MEMORY_PROJECT)]
    pub project: Option<String>,
    #[schemars(description = "The memory's name, as search_memory gives it.")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteMemoryArgs {
    #[schemars(description = MEMORY_PROJECT)]
    pub project: Option<String>,
    #[schemars(
        description = "Its name, short, in kebab-case (made so: lower case letters, digits and dashes). A memory of that name is replaced: read it first to keep what it says."
    )]
    pub name: String,
    #[schemars(
        description = "One line that says what it is about, so that a search and the index tell when it matters (300 characters at most)."
    )]
    pub description: String,
    #[serde(rename = "type")]
    #[schemars(
        description = "Its kind: feedback (a way to work, a trap to avoid; the default), project (a constraint of the work under way), reference (where to find something) or user (what the user prefers)."
    )]
    pub kind: Option<crate::memory::Kind>,
    #[schemars(
        description = "What to remember, in Markdown: the rule, then why, then how to apply it (20,000 characters at most)."
    )]
    pub body: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ForgetMemoryArgs {
    #[schemars(description = MEMORY_PROJECT)]
    pub project: Option<String>,
    #[schemars(description = "The memory's name.")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBoardArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
}

/// What validating a ticket does, as `set_board` names it.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum ActionArg {
    Merge,
    Pr,
    Push,
    Keep,
}

impl ActionArg {
    pub(super) fn name(self) -> &'static str {
        match self {
            ActionArg::Merge => "merge",
            ActionArg::Pr => "pr",
            ActionArg::Push => "push",
            ActionArg::Keep => "keep",
        }
    }
}

/// How a merge goes in, as `set_board` names it.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum StrategyArg {
    Merge,
    Squash,
    Rebase,
}

impl StrategyArg {
    pub(super) fn name(self) -> &'static str {
        match self {
            StrategyArg::Merge => "merge",
            StrategyArg::Squash => "squash",
            StrategyArg::Rebase => "rebase",
        }
    }
}

/// What a merge conflict does, as `set_board` names it.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum ConflictArg {
    Ask,
    Agent,
    Abort,
}

impl ConflictArg {
    pub(super) fn name(self) -> &'static str {
        match self {
            ConflictArg::Ask => "ask",
            ConflictArg::Agent => "agent",
            ConflictArg::Abort => "abort",
        }
    }
}

// Named as `get_board` answers them (camelCase), so that its answer can be given back as it is.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetBoardArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
    #[schemars(
        description = "What approving a ticket does: merge (into the target branch), pr (push its branch and open a pull request), push (push its branch only) or keep (nothing: the changes stay in its worktree). Leave it out to keep the current one."
    )]
    pub action: Option<ActionArg>,
    #[schemars(
        description = "The target branch: where tickets merge, the base of their pull request and of their branch. One of the project's local branches, unless it is the current one (the Kanban's own, or the branch get_board shows while the Kanban has none): it is then left as it is. Leave it out to keep the current one."
    )]
    pub target: Option<String>,
    #[schemars(
        description = "How a merge goes in: merge (a merge commit), squash or rebase. Leave it out to keep the current one."
    )]
    pub strategy: Option<StrategyArg>,
    #[schemars(
        description = "true: the pull request opens as a draft. Leave it out to keep the current one."
    )]
    pub draft: Option<bool>,
    #[schemars(
        description = "true: the ticket's worktree is deleted once it is approved. Leave it out to keep the current one."
    )]
    pub cleanup: Option<bool>,
    #[schemars(
        description = "true: the commit messages are written by Haiku in the Conventional Commits form. Leave it out to keep the current one."
    )]
    pub conventional: Option<bool>,
    #[schemars(
        description = "What a merge conflict does: ask (the user decides), agent (the ticket's agent resolves it) or abort (cancel the merge). Leave it out to keep the current one."
    )]
    pub conflict: Option<ConflictArg>,
    #[schemars(
        description = "The most tickets in progress at once, 1 to 6. Leave it out to keep the current one."
    )]
    pub max_parallel: Option<u32>,
    #[schemars(
        description = "true: the autopilot starts the To do tickets by itself as soon as a place is free; false: they wait to be launched by hand. Leave it out to keep the current one."
    )]
    pub autopilot: Option<bool>,
    #[schemars(
        description = "The model of the tickets' agents, as the project's harness names it (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives its models). An empty string is the app's default model. Leave it out to keep the current one."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The effort of the tickets' agents: low, medium, high, xhigh or max, those of the project's harness and model (list_projects). An empty string is the app's default effort. Leave it out to keep the current one."
    )]
    pub effort: Option<String>,
    #[schemars(
        description = "The permission mode of the tickets' agents: auto, default, plan or acceptEdits. An empty string is the app's default mode. bypassPermissions (no permission request at all) is the user's to give, in the window: it is refused here unless the Kanban is already on it. auto does not apply to a model that has no auto mode (Haiku): its agents go in the default mode. Leave it out to keep the current one."
    )]
    pub mode: Option<String>,
    #[schemars(
        description = "true: the tests run again before approving a ticket, and a failing test sends it back to its agent. Refused to turn it on when the Kanban has no test command: it is set in the window, the server runs no command (it is off with none, as get_board says). Leave it out to keep the current one."
    )]
    pub tests_first: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApproveTicketArgs {
    #[schemars(
        description = "The ticket to approve: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RejectTicketArgs {
    #[schemars(
        description = "The ticket to send back: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
    #[schemars(
        description = "What is wrong, or what to do: the agent reads it as test feedback, followed by who sent the ticket back."
    )]
    pub comment: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResumeTicketArgs {
    #[schemars(
        description = "The blocked In progress ticket to take up: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResolveConflictArgs {
    #[schemars(
        description = "The To review ticket stopped on a merge conflict: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DismissBlockArgs {
    #[schemars(
        description = "The blocked To review ticket: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteTicketArgs {
    #[schemars(
        description = "The ticket to delete, in any column: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateAgentArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
    #[schemars(
        description = "The agent's first message: what it should do. The agent reads it headed by your name (Message from <you>: …), as coming from you and not from the user."
    )]
    pub message: String,
    #[schemars(
        description = "true: the agent works in a worktree of its own; false: in the project's folder. Leave it out to do as the project does by default."
    )]
    pub worktree: Option<bool>,
    #[schemars(
        description = "The model, as the project's harness names it (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives its models). Leave it out for the default model."
    )]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SendMessageArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The message, 32,000 characters at most. The agent reads it, and the conversation shows it, headed by your name (Message from <you>: …), as coming from you and not from the user."
    )]
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StopAgentArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReportProgressArgs {
    #[schemars(
        description = "What you are doing, in one line of 120 characters at most. An empty line takes the previous one off."
    )]
    pub line: String,
}

/// The most tickets one `split_ticket` makes.
pub const SPLIT_MAX: usize = 10;

/// One of the tickets `split_ticket` makes.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct SubTicket {
    #[schemars(description = "The ticket's title, on one line (200 characters at most).")]
    pub title: String,
    #[schemars(description = "What to do in this ticket (10,000 characters at most).")]
    pub description: Option<String>,
    #[schemars(
        description = "Its acceptance criteria, one per item. Leave them out for two default ones."
    )]
    pub criteria: Option<Vec<String>>,
    #[schemars(
        description = "The images of your ticket this one gets a copy of, each named by its file or its rank in the list get_ticket gives. Leave it out for all of them, give an empty list for none."
    )]
    pub images: Option<Vec<ImageRef>>,
    #[schemars(
        description = "The other files of your ticket this one gets a copy of, each named by its file or its rank in the list get_ticket gives (files). Leave it out for all of them, give an empty list for none."
    )]
    pub files: Option<Vec<FileRef>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SplitTicketArgs {
    #[schemars(description = "The tickets to make (10 at most), each with a title.")]
    pub tickets: Vec<SubTicket>,
    #[schemars(
        description = "true (the default): each ticket comes after the one before it, the first after yours; false: all come after yours alone and may run in parallel."
    )]
    pub chain: Option<bool>,
}

// The tools: those that read, those that act, those for the agents of Escouade alone.
#[tool_router(vis = "pub(crate)")]
impl<R: Runtime> Tools<R> {
    #[tool(
        description = "List Escouade's projects, in the app's order (for an agent of Escouade, those it sees: a private project is listed to its own agents alone, and they see no other project): for each one its id, name, folder, current git branch (null when none), harness (engine) with the models and efforts its tools take (models, efforts: what model and effort accept for this project), number of agents (archived ones left out), number of tickets in each column of its Kanban board (todo, doing, review, done), and its Remote Control settings: remoteControl (the project's own, which makes its agents reachable from claude.ai) and remoteControlAll (the global one, the same on every row, which covers every agent of every project). Remote Control is only read here: no tool turns Remote Control on or off, since it exposes a session on the internet and stays the user's choice in the window.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_projects(&self, ctx: ToolCtx<R>) -> CallToolResult {
        let result = read::projects(&ctx.core, &ctx.caller).await;
        ctx.reply("list_projects", "", result)
    }

    #[tool(
        description = "List Escouade's Claude Code agents, oldest first (archived ones left out), of every project or of one: their id, name, project (its name) and projectId, status (idle, running, waiting, done or error), whether a question or a permission waits for the user's answer and how many (requests; list_requests gives them), the key of their ticket (null when none), their harness (engine: claude for Claude Code, codex, copilot, cursor or opencode), their model, their Claude account, what they cost so far in US dollars, how many subagents and background tasks they have running (subagentsRunning: the subagents at work, in the foreground or the background; backgroundTasksRunning: the commands, monitors, remote agents, teammates and workflows Claude Code runs for them, never the subagents; an agent that finished its turn may still have some going on, and list_background_work lists them), and their Remote Control: remoteControl (in force now, so the agent is reachable from claude.ai), remoteSource (what turned it on: hand, project or global; off when the user set the agent apart with « Désactiver le remote control », so no setting gives it back; null otherwise), remoteState (the link's state Claude Code reports: ready, connected, unavailable; null without a link) and interfaceRemote (in the Claude Code view: true when its interface runs and was started with Remote Control, so it stays linked to claude.ai until its next launch even after Remote Control was turned off; false when it runs without; null when none runs), and workflow (where it is in its project's workflow: name, step, turn, since in epoch milliseconds, and outside when it does something else; null when it follows none; get_workflow gives the workflow). Remote Control is only read here: no tool turns Remote Control on or off, since it exposes a session on the internet and stays the user's choice in the window.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_agents(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ListAgentsArgs>,
    ) -> CallToolResult {
        let result = read::agents(&ctx.core, &ctx.caller, args.project.as_deref());
        let asked = summary(&[("project", args.project.as_deref())]);
        ctx.reply("list_agents", &asked, result)
    }

    #[tool(
        description = "List the tickets of a project's Kanban board in the board's order (todo by priority, doing and review by arrival, done newest first), of every column or of one: their id, key, title, column, the keys of the tickets they come after, their agent's name, why they are blocked when they are, and the account (its name), the model and the effort their agents start with when they are set to one (left out when auto).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_tickets(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ListTicketsArgs>,
    ) -> CallToolResult {
        let result = read::tickets(
            &ctx.core,
            &ctx.caller,
            &args.project,
            args.column.map(Column::from),
        );
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("column", args.column.map(ColumnArg::name)),
        ]);
        ctx.reply("list_tickets", &asked, result)
    }

    #[tool(
        description = "Read a ticket whole: its title, description, acceptance criteria (each with whether it is met and its agent's note), loops (the current one and the most it may run), column, the keys of the tickets it comes after, the features in place so far as its agent listed them, its images (images: for each its index, from 0, its file, name, mediaType and size in bytes, null when its file is gone; get_ticket_image reads one), its other files, of any format (files: for each its index, from 0, its file, name and size in bytes, null when its file is gone; its agent is given their paths, and get_ticket_file reads one), the account, the model and the effort its agents start with (account: the account's name, with accountId, or auto for the current account of its harness; model and effort: auto for the Kanban's, else the account's; accountLost: why it went back to auto, its account removed, switched off or signed out, left out otherwise), the external ticket it was imported from (Jira, Trello, GitHub, Linear, ClickUp, GitLab or Azure Boards; null when none), and where it stands in the board's flow: its agent (id, name, status, and how many requests wait for an answer; null before it starts), its branch and its worktree, those of its agent (null before it starts; kept once the ticket is done, when the worktree may be gone), why it is blocked (blocked) and whether that is a merge conflict (conflict), the validation step running (step: Validation… or Approving…, Tests…, Commit…, Merge…, Push…) and, once it is approved, its outcome (text, the pull request's url, and the commit it made on the target branch). An agent of Escouade may leave out the ticket to read its own.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<GetTicketArgs>,
    ) -> CallToolResult {
        let result = read::ticket(&ctx.core, &ctx.caller, args.ticket.as_deref());
        let asked = summary(&[("ticket", args.ticket.as_deref())]);
        ctx.reply("get_ticket", &asked, result)
    }

    #[tool(
        description = "Read an image of a ticket: answers the image itself (an image block), then a text with the ticket's key and the image's index, file, name, mediaType and size in bytes. Name the image by its file or by its index, from 0, in the list get_ticket gives (images): one of the two. An agent of Escouade may leave out the ticket to read an image of its own, in its own project or another it sees (a private project is seen by its own agents alone). Fails when the image's file is gone.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_ticket_image(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<GetTicketImageArgs>,
    ) -> CallToolResult {
        let index = args.index.map(|i| i.to_string());
        let asked = summary(&[
            ("ticket", args.ticket.as_deref()),
            ("file", args.file.as_deref()),
            ("index", index.as_deref()),
        ]);
        let result = images::fetch(&ctx.core, &ctx.caller, args).await;
        ctx.reply_blocks("get_ticket_image", &asked, result)
    }

    #[tool(
        description = "Read a file of a ticket other than an image (any format: get_ticket lists them, files), part of it at a time: its end by default, or bytes from offset. Answers the ticket's key and the file's index, file, name and size in bytes, then the part read: offset (where it starts in the file), length (how many bytes of the file it holds), encoding (text when it is UTF-8 text, cut at characters, so offset and length may differ by a few bytes from what was asked; base64 otherwise) and content. bytes: 8192 by default, 65536 at most (Claude Code refuses an answer of more than about 25,000 tokens): read a bigger file in several parts. For Claude outside Escouade, the answer also gives path, where the file is on this computer, to read it whole by its own means; an agent of Escouade is not given it (the ticket's agent has the paths of its files in its first message). Name the file by its file or by its index, from 0, in the list get_ticket gives: one of the two. An agent of Escouade may leave out the ticket to read a file of its own, in its own project or another it sees (a private project is seen by its own agents alone). Fails when the file is gone from the ticket's folder (the purge of the settings, « Conserver les fichiers »).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_ticket_file(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<GetTicketFileArgs>,
    ) -> CallToolResult {
        let (index, offset, bytes) = (
            args.index.map(|i| i.to_string()),
            args.offset.map(|o| o.to_string()),
            args.bytes.map(|b| b.to_string()),
        );
        let asked = summary(&[
            ("ticket", args.ticket.as_deref()),
            ("file", args.file.as_deref()),
            ("index", index.as_deref()),
            ("offset", offset.as_deref()),
            ("bytes", bytes.as_deref()),
        ]);
        let result = files::fetch(&ctx.core, &ctx.caller, args).await;
        ctx.reply("get_ticket_file", &asked, result)
    }

    #[tool(
        description = "Read a project's Kanban settings: the project (its name, and projectId), what approving a ticket does (action: merge, pr, push or keep; target: the branch tickets merge into, the base of their pull request and of their branch, the project's current branch while the Kanban has none, empty on a detached HEAD; strategy: merge, squash or rebase; draft; cleanup; conventional commit messages; conflict: ask, agent or abort), maxParallel (the most tickets in progress at once), autopilot, the model, effort and mode of the tickets' agents (an empty string is the app's default, given by defaults), testsFirst (true only when tests first is on and testCommand is not empty, as the window shows it), testCommand and the tickets' key prefix. set_board changes them, and takes this answer back as it is.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_board(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<GetBoardArgs>,
    ) -> CallToolResult {
        let result = read::board(&ctx.core, &ctx.caller, &args.project).await;
        let asked = summary(&[("project", Some(&args.project))]);
        ctx.reply("get_board", &asked, result)
    }

    #[tool(
        description = "Read a project's workflows, as the window's Workflow tab shows them (read again if a file they read changed), as the user corrected them in Settings › Projects › Workflow (steps renamed, merged, removed, reordered or added; a workflow dismissed as not one is left out): for each workflow its id, name, kind (cycle: steps that name each other; prompt: a prompt run over and over), harness (engine; null for a PROMPT.md), repo (of a project of several repositories), doors (the commands or skills that go in: name, command as typed, kind, scope, path, description, role entry, step or other, enters), steps (name, command, order, description, path, agents: the sub-agents it launches, reads, writes, and agentsHere: the agents of the project on it now, with id, name, since in epoch milliseconds and pass, the nth stay in that step), transitions (from, to, back: a way back to a step already passed, condition: the line that says when, max), stateFiles, exit and promptLoop. Then alone (commands of the harness' folders that belong to no workflow), and agentsElsewhere (agents that follow a workflow but are on no step: between two, or outside it, doing something else). An agent is placed by what it does, compared to the steps; nothing is asked of it. Open to every client, for the projects it sees (a private project is seen by its own agents alone). list_agents and get_agent_summary give each agent's workflow too.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_workflow(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<GetWorkflowArgs>,
    ) -> CallToolResult {
        let result = workflow::get(&ctx.core, &ctx.caller, &args.project).await;
        let asked = summary(&[("project", Some(&args.project))]);
        ctx.reply("get_workflow", &asked, result)
    }

    #[tool(
        description = "Search for symbols (functions, structs, traits, etc.) in the code index by name or keyword, the best first: one line per symbol, file:line-end_line: name (kind) — signature. Write the words of the code (its identifiers, in English here), not a paraphrase; when a word finds nothing, ask index_context with keywords. ids: true adds the id that index_body, index_callers, index_callees and index_impact take. literal: true finds an exact text in the source (an error message, a string) and returns file:line locations. path_include and path_exclude narrow by path; persistent noise (vendored trees, docs) can be suppressed project-wide via gitignore-style patterns in .escouade/queryignore. Keep grep for a regex. Each answer ends with index_metrics: before (the files touched, read whole, capped at 4 times the answer), after (the answer, the call and, once, the tools' schemas) and saved. The project is the agent's own when left out; the code index is off until the user turns it on in the project's settings (the tool then says so). Open to every client, for the projects it sees (a private project is seen by its own agents alone, and its index holds only what its privacy allows).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn index_search(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<index::SearchArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("query", Some(&args.query)),
        ]);
        let arguments = index::sent(&args);
        let result = index::search(
            &ctx.core,
            &ctx.caller,
            self.index_schemas(),
            args,
            arguments,
        )
        .await;
        ctx.reply_blocks("index_search", &asked, result.map(index_blocks))
    }

    #[tool(
        description = "Build an AI-ready context for a task description, in Markdown: the entry points, the related symbols by file, and the code of the entry points (5 blocks at most, of 1,500 bytes at most; a truncated one ends with the index_body call that returns the rest). Tell the task with the words of the code (its identifiers, in English here) where you know them. The output includes a `### Retrieval` section: when it lists zero-hit terms, re-query once with `keywords` synonyms for them (or use index_search); a `match: fts-only` tier means weak lexical matches, so verify entry points before building on them. Pass seen_node_ids from each answer to the next call's exclude_node_ids; path, path_include and path_exclude narrow the entry points. Each answer ends with index_metrics: before (the files touched, read whole, capped at 4 times the answer), after (the answer, the call and, once, the tools' schemas) and saved. The project is the agent's own when left out; the code index is off until the user turns it on in the project's settings (the tool then says so). Open to every client, for the projects it sees (a private project is seen by its own agents alone, and its index holds only what its privacy allows).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn index_context(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<index::ContextArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("task", Some(&args.task)),
        ]);
        let arguments = index::sent(&args);
        let result = index::context(
            &ctx.core,
            &ctx.caller,
            self.index_schemas(),
            args,
            arguments,
        )
        .await;
        ctx.reply_blocks("index_context", &asked, result.map(index_blocks))
    }

    #[tool(
        description = "Return the full source body of a symbol by name (function, struct, const, etc.) from the code index. Collapses search + node lookup + file read into a single call. Returns a body only when exactly one definition matches; when the name is ambiguous (e.g. several methods named `refresh`), returns no body but a candidate list (qualified name, kind, file, line range, node id) — re-call with a qualified name such as `Type::Member` / `Type.Member`, or with `node_id`. A function or type definition outranks a same-named field or import. A truncated snippet of index_context names its node_id. Each answer ends with index_metrics, as the other index tools. The project is the agent's own when left out; the code index is off until the user turns it on in the project's settings (the tool then says so). Open to every client, for the projects it sees (a private project is seen by its own agents alone, and its index holds only what its privacy allows).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn index_body(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<index::BodyArgs>,
    ) -> CallToolResult {
        let id = args.node_id.map(|i| i.to_string());
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("node_id", id.as_deref()),
            ("symbol", args.symbol.as_deref()),
        ]);
        let arguments = index::sent(&args);
        let result = index::body(
            &ctx.core,
            &ctx.caller,
            self.index_schemas(),
            args,
            arguments,
        )
        .await;
        ctx.reply_blocks("index_body", &asked, result.map(index_blocks))
    }

    #[tool(
        description = "Who calls a function or method, from the code index, as JSON: by its id, or by a name answered definition by definition (matches: each definition, its callers resolved to it (file, line, the symbol that calls, its id), and apart the calls that may mean it among others, ambiguous; more: the other definitions of the name, to ask again by id). The calls are resolved to symbols when the files are read, as tokensave does; a tie is never guessed. Each answer ends with index_metrics, as the other index tools. The project is the agent's own when left out; the code index is off until the user turns it on in the project's settings (the tool then says so). Open to every client, for the projects it sees (a private project is seen by its own agents alone, and its index holds only what its privacy allows).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn index_callers(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<index::CallArgs>,
    ) -> CallToolResult {
        let id = args.id.map(|i| i.to_string());
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("id", id.as_deref()),
            ("name", args.name.as_deref()),
        ]);
        let arguments = index::sent(&args);
        let result = index::callers(
            &ctx.core,
            &ctx.caller,
            self.index_schemas(),
            args,
            arguments,
        )
        .await;
        ctx.reply_blocks("index_callers", &asked, result.map(index_blocks))
    }

    #[tool(
        description = "What a function or method calls, from the code index, as JSON: by its id, or by a name answered definition by definition (matches: the definitions it calls, resolved; apart the calls left to a tie, ambiguous; the names left, the language's and libraries', external; more: the other definitions of the name). Each answer ends with index_metrics, as the other index tools. The project is the agent's own when left out; the code index is off until the user turns it on in the project's settings (the tool then says so). Open to every client, for the projects it sees (a private project is seen by its own agents alone, and its index holds only what its privacy allows).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn index_callees(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<index::CallArgs>,
    ) -> CallToolResult {
        let id = args.id.map(|i| i.to_string());
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("id", id.as_deref()),
            ("name", args.name.as_deref()),
        ]);
        let arguments = index::sent(&args);
        let result = index::callees(
            &ctx.core,
            &ctx.caller,
            self.index_schemas(),
            args,
            arguments,
        )
        .await;
        ctx.reply_blocks("index_callees", &asked, result.map(index_blocks))
    }

    #[tool(
        description = "What a change would touch, from the code index, as JSON: a symbol by its id, a name answered definition by definition, or a file by its path; for each, its callers followed by the resolved links up to depth levels, the types that extend or implement it, the files that import it, every file touched, and apart the calls that may mean it among others (ambiguous). Each answer ends with index_metrics, as the other index tools. The project is the agent's own when left out; the code index is off until the user turns it on in the project's settings (the tool then says so). Open to every client, for the projects it sees (a private project is seen by its own agents alone, and its index holds only what its privacy allows).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn index_impact(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<index::ImpactArgs>,
    ) -> CallToolResult {
        let id = args.id.map(|i| i.to_string());
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("id", id.as_deref()),
            ("target", args.target.as_deref()),
        ]);
        let arguments = index::sent(&args);
        let result = index::impact(
            &ctx.core,
            &ctx.caller,
            self.index_schemas(),
            args,
            arguments,
        )
        .await;
        ctx.reply_blocks("index_impact", &asked, result.map(index_blocks))
    }

    #[tool(
        description = "Search the shared memory of a project: the work notebook of its agents, shared by every harness (Claude Code, Codex, Copilot CLI, Cursor CLI, OpenCode) and by the team, of how to work in the project and the traps to avoid (not the product: the spec says what it does). Look in it at the start of a task. By the words (name, description, text; accents and case ignored), and by the meaning too when Escouade's vectors' model is downloaded (by: meaning, else words). Answers the memories found, best first: name, description, type and the start of their text (excerpt); read_memory reads one whole. A private project's memory is seen by its own agents alone.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn search_memory(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<SearchMemoryArgs>,
    ) -> CallToolResult {
        let result = memory::search(
            &ctx.core,
            &ctx.caller,
            args.project.as_deref(),
            &args.query,
            args.limit,
        )
        .await;
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("query", Some(args.query.trim())),
        ]);
        ctx.reply("search_memory", &asked, result)
    }

    #[tool(
        description = "Read a memory of a project's shared memory whole: its name, description, type and text (body, Markdown).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn read_memory(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ReadMemoryArgs>,
    ) -> CallToolResult {
        let result = memory::read(&ctx.core, &ctx.caller, args.project.as_deref(), &args.name);
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("name", Some(&args.name)),
        ]);
        ctx.reply("read_memory", &asked, result)
    }

    #[tool(
        description = "Note in a project's shared memory what you learned of how to work in it: a way of doing that works, a trap to avoid, a command to run first, a test that is flaky and why. One memory per file of .escouade/memory/ in the project's folder (Markdown with a header), its index MEMORY.md written again; every agent of the project finds it, whatever its harness or its branch. Not the product (what it does belongs to the spec), never a secret (a token, a password, a key). A memory of the same name is replaced: search and read first, and complete one rather than repeat it. Escouade never commits it; the team shares it by git. Answers the memory as written and whether it is new.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn write_memory(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<WriteMemoryArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("name", Some(&args.name)),
        ]);
        let draft = crate::memory::Draft {
            name: args.name,
            description: args.description,
            kind: memory::kind_of(args.kind),
            body: args.body,
        };
        let result = memory::write(&ctx.core, &ctx.caller, args.project.as_deref(), draft);
        ctx.reply_changed("write_memory", &asked, result)
    }

    #[tool(
        description = "Forget a memory of a project's shared memory that turned out wrong or no longer holds: its file is removed and the index written again. Answers the name forgotten.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn forget_memory(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ForgetMemoryArgs>,
    ) -> CallToolResult {
        let result = memory::forget(&ctx.core, &ctx.caller, args.project.as_deref(), &args.name);
        let asked = summary(&[
            ("project", args.project.as_deref()),
            ("name", Some(&args.name)),
        ]);
        ctx.reply_changed("forget_memory", &asked, result)
    }

    #[tool(
        description = "Read a project's specification, the tree of its pages: the subjects (folders) with, for each page, its path, title, when it was last compared to the code (compared, empty when never), the tickets and the decisions it is linked to. The spec lives under .escouade/spec of the project's docs branch (branch: current, or the name of the fixed docs branch). When the spec has no page yet, existingSpec lists what the project already has that looks like one (SPEC.md, a docs folder…: path, kind file or folder, files, bytes) so it can be offered for import; nothing is imported by this tool. Open to every client, for the projects it sees (a private project is seen by its own agents alone).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_spec_tree(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::DocsProjectArgs>,
    ) -> CallToolResult {
        let result = docs::tree(&ctx.core, &ctx.caller, &args.project).await;
        let asked = summary(&[("project", Some(&args.project))]);
        ctx.reply("get_spec_tree", &asked, result)
    }

    #[tool(
        description = "Read a page of the specification: its path, title, subject, sources, tickets, decisions, compared (when it was last compared to the code, empty when never) and body (Markdown). Open to every client, for the projects it sees.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn read_spec_page(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::SpecPageArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("project", Some(&args.project)), ("path", Some(&args.path))]);
        let result = docs::page(&ctx.core, &ctx.caller, args).await;
        ctx.reply("read_spec_page", &asked, result)
    }

    #[tool(
        description = "Create a page of the specification or change one, in the project's docs branch (the branch you work on, or the project's fixed docs branch): the fields you leave out stay as they are, body replaces the text (or goes at its end with append). Every write of the spec goes through this tool, so the spec stays where the team reads it. Nothing is committed nor pushed: the user shares the changes. Answers the page's path, title, subject, created, and the lines added and removed (which propose_decision takes as the changes of a decision).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn write_spec_page(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::WriteSpecArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("project", Some(&args.project)), ("path", Some(&args.path))]);
        let result = docs::write(&ctx.core, &ctx.caller, args).await;
        ctx.reply("write_spec_page", &asked, result)
    }

    #[tool(
        description = "Search the specification: the pages whose title or text hold all the words (any case), the best first, each with its path, title and the first lines that hold a word. Open to every client, for the projects it sees.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn search_spec(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::SearchSpecArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("query", Some(&args.query)),
        ]);
        let result = docs::search(&ctx.core, &ctx.caller, args).await;
        ctx.reply("search_spec", &asked, result)
    }

    #[tool(
        description = "List a project's decisions, the register of the product's choices, accepted ones by number then the proposals: number (none for a proposal: it gets one when accepted), id (how a proposal is cited), title, date, state (proposed, accepted or superseded), origin, what it supersedes and what superseded it, replacementProposed (the id of a proposal that would replace an accepted one), and what it changed in the spec (changes: page, lines added and removed). A decision is accepted by a human, in the window: no tool does it. Open to every client, for the projects it sees.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_decisions(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::ListDecisionsArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("state", args.state.as_deref()),
        ]);
        let result = docs::decisions(&ctx.core, &ctx.caller, args).await;
        ctx.reply("list_decisions", &asked, result)
    }

    #[tool(
        description = "Read a decision whole, by its number or its id (a proposal has only an id): number, id, title, date, state, origin, links (pages, tickets, documents), what it supersedes, what superseded it, what it changed in the spec, when and by whom it was accepted, and its context, decision and consequences. Open to every client, for the projects it sees.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn read_decision(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::ReadDecisionArgs>,
    ) -> CallToolResult {
        let number = args.number.map(|n| n.to_string());
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("number", number.as_deref()),
            ("id", args.id.as_deref()),
        ]);
        let result = docs::decision(&ctx.core, &ctx.caller, args).await;
        ctx.reply("read_decision", &asked, result)
    }

    #[tool(
        description = "Propose a decision about the product: a new proposal (state proposed, with an id and no number: cite it by its id in the spec pages and tickets; it gets its number when it is accepted, after the register of the branch it goes to, and what cites it takes that number), or the revision of a proposal still open (id). It is not decided until a human accepts it in the window (the agent of a ticket: when its ticket is validated; its decisions say they come from it), and an accepted decision is never rewritten: replace_decision replaces it. Give it a title, its context, what is decided and what follows, where it comes from and the spec pages it changed (changes, from write_spec_page). Nothing is committed nor pushed.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn propose_decision(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::ProposeDecisionArgs>,
    ) -> CallToolResult {
        let number = args.number.map(|n| n.to_string());
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("id", args.id.as_deref()),
            ("number", number.as_deref()),
            ("title", Some(&args.fields.title)),
        ]);
        let result = docs::propose(&ctx.core, &ctx.caller, args).await;
        ctx.reply("propose_decision", &asked, result)
    }

    #[tool(
        description = "Propose a decision that replaces an accepted one (number): a new proposal that lists it in supersedes. Once a human accepts the new one, the old one becomes superseded and says by which; until then the old one stands. Only an accepted decision is replaced. Same fields as propose_decision.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn replace_decision(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::ReplaceDecisionArgs>,
    ) -> CallToolResult {
        let number = args.number.to_string();
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("number", Some(&number)),
            ("title", Some(&args.fields.title)),
        ]);
        let result = docs::replace(&ctx.core, &ctx.caller, args).await;
        ctx.reply("replace_decision", &asked, result)
    }

    #[tool(
        description = "List the documents of a project's drive (the files the team dropped under .escouade/documents of the docs branch): its folders (path, name, files), and each document's path, name, folder, size, kind (text, pdf, word, image or other), readable (whether read_document gives their text), modified (epoch milliseconds), author, reads (times the agents read it) and cited (pages of the spec and decisions that cite it). To find the documents about something, search_documents. Open to every client, for the projects it sees.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_documents(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::ListDocumentsArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("folder", args.folder.as_deref()),
        ]);
        let result = docs::documents(&ctx.core, &ctx.caller, args).await;
        ctx.reply("list_documents", &asked, result)
    }

    #[tool(
        description = "Read a document's text, extracted on this machine from text, Markdown, PDF, Word (.docx) and OpenDocument (.odt) files and kept in Escouade's folder, not in the repository: text (null for a format whose text Escouade does not read, as an image or a PDF of scans; an image gives its format and size in meta), total (characters of the whole text), from and truncated (more follows: ask again with from). To cite a document, name its path (documents/<path>) in a page's sources (write_spec_page) or a decision's documents: the window counts the citations. Open to every client, for the projects it sees.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn read_document(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::ReadDocumentArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("project", Some(&args.project)), ("path", Some(&args.path))]);
        let result = docs::read_document(&ctx.core, &ctx.caller, args).await;
        ctx.reply("read_document", &asked, result)
    }

    #[tool(
        description = "Search a project's documents (the drive of .escouade/documents: text, Markdown, PDF, Word, OpenDocument, and images by their name): by the meaning and the words together when the vectors' model is downloaded, by the words otherwise (by says which). Answers each document's path, name, folder, kind and the passage that answers (excerpt); read_document gives the whole text. Everything runs on this machine. Open to every client, for the projects it sees.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn search_documents(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::SearchDocumentsArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("query", Some(&args.query)),
        ]);
        let result = docs::search_documents(&ctx.core, &ctx.caller, args).await;
        ctx.reply("search_documents", &asked, result)
    }

    #[tool(
        description = "The changes of a project's docs (spec, decisions, documents) not shared with the team yet: pending.files, pending.count, and pending.conflict when a sharing stopped on a conflict between two versions of a page, which the writer agent resolves from the window. Open to every client, for the projects it sees.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_docs_to_share(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::DocsProjectArgs>,
    ) -> CallToolResult {
        let result = docs::to_share(&ctx.core, &ctx.caller, &args.project).await;
        let asked = summary(&[("project", Some(&args.project))]);
        ctx.reply("get_docs_to_share", &asked, result)
    }

    #[tool(
        description = "Share a project's docs, as the window's « Partager » does: the remote branch is fetched first, the changes of the docs (and only them) are committed with a message, others' work is merged in and the branch is pushed. A conflict between two versions of a page is undone and left for the writer agent (the window offers « L'agent résout » / « Annuler »); it is never shown as lines of git. Sharing is the user's gesture: an agent of Escouade is refused unless the user told it to (userAskedForIt). Refused, in a sentence, while the branch has commits of code not pushed yet, unless withCode is true. Answers kind nothing, done (committed, message, pulled) or conflict.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = true
        )
    )]
    async fn share_docs(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::ShareDocsArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("project", Some(&args.project))]);
        let result = docs::share(&ctx.core, &ctx.caller, args).await;
        ctx.reply("share_docs", &asked, result)
    }

    #[tool(
        description = "Add a text file to a project's drive of documents, in the docs branch: under the folder given (made if needed), or, without one, filed by Escouade in the folder that fits it (by its name, its sort and its words, or a new folder): placement says where and why (by). Nothing already there is overwritten: a taken name gets a number. Nothing is committed nor pushed.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn add_document(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<docs::AddDocumentArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("folder", args.folder.as_deref()),
            ("name", Some(&args.name)),
        ]);
        let result = docs::add_document(&ctx.core, &ctx.caller, args).await;
        ctx.reply("add_document", &asked, result)
    }

    #[tool(
        description = "Read the quota of every account, of every harness: for each account its harness (engine), whether new agents of that harness go to it now (current: the first active account of the harness whose meters are under the autopilot's threshold, else the first active one), and its meters as last read (meters: a window of a length in minutes, 300 for 5 hours and 10080 for a week, or the month's credits with minutes null; each with how much is used, pct 0 to 100, and when it resets, resetsAt in epoch milliseconds; none for a harness without a quota or not read yet), Claude Code's 5-hour and weekly windows also as fiveHour and sevenDay (null when not read); and why the autopilot holds the tickets of the default harness back, when it does (reason: window, with its length in minutes, credits or limit), until when (epoch milliseconds).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_usage(&self, ctx: ToolCtx<R>) -> CallToolResult {
        let result = read::usage(&ctx.core);
        ctx.reply("get_usage", "", result)
    }

    #[tool(
        description = "Sum up an agent without reading its conversation: its id, name, status, its harness (engine, as list_agents gives it), the key of its ticket (null when none), its last message (null before its first; 2,000 characters at most, a longer one given by its start and its end, where it says what it concluded or asks, joined by \" … \"), the files it edited (the latest 100 at most, relative to its folder), and how many requests wait for the user's answer (requests) and of which type, in the order they came (waitingFor: permission, question or plan), how many subagents and background tasks it has running (subagentsRunning and backgroundTasksRunning, as list_agents gives them; list_background_work lists them), its Remote Control as list_agents gives it (remoteControl, remoteSource, remoteState, interfaceRemote), and its workflow as list_agents gives it (workflow: name, step, turn, since, outside), with the steps it went through (history, the latest 100, oldest first: step, door, version, pass, at, ended, ms, cost, did, outcome running, done, back, failed, stopped or left, reason and backTo for a way back; ms and cost for its whole stay in the workflow). Remote Control is only read here: no tool turns Remote Control on or off, since it exposes a session on the internet and stays the user's choice in the window.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_agent_summary(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<GetAgentSummaryArgs>,
    ) -> CallToolResult {
        let result =
            read::agent_summary(&ctx.core, &ctx.caller, &args.agent, args.project.as_deref());
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
        ]);
        ctx.reply("get_agent_summary", &asked, result)
    }

    #[tool(
        description = "List what an agent has going on besides its turn, as the window shows it under its card: its subagents and the tasks Claude Code runs for it in the background. For the agent: id, name, status, subagentsRunning and backgroundTasksRunning (how many of each run: list_agents and get_agent_summary give them too). subagents, those running first in the plan's order then the latest ended: id (the call that launched it, which get_background_output, stop_background_work and message_subagent name it by), type (subagent), description, subagentType (Explore…), model, status (running, done, failed, stopped or interrupted), background (it goes on after the agent's turn), startedAt and endedAt (epoch milliseconds), tools used, tokens, doing (what it does now), taskId (Claude Code's id of its task, once a frame told it) and parent (the call of the subagent that launched it); a field that is not known is left out. tasks, those running first from the oldest launched then the latest ended: id (Claude Code's id of the task, which the other tools name it by), type (command, monitor, remote, teammate or workflow), description (what the tool said it was for), command, status, startedAt, endedAt, summary (Claude Code's last words on it), parent (the taskId of the subagent that owns it) and, for a workflow, name, now (its phase) and tools and tokens used. Open to every client, for the agents of every project it sees (an agent of Escouade sees its own project and every project that is not private; a private project is seen by its own agents alone). It never gives the file a task's output is written to: get_background_output reads the output.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_background_work(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ListBackgroundWorkArgs>,
    ) -> CallToolResult {
        let result = background::list(&ctx.core, &ctx.caller, &args.agent, args.project.as_deref());
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
        ]);
        ctx.reply("list_background_work", &asked, result)
    }

    #[tool(
        description = "Read the output of a command or a monitor Claude Code runs in the background for an agent, or the report of one of its subagents, as the window shows them (list_background_work gives the ids). agent (id, name), id, type (command, monitor, remote, teammate, workflow or subagent), status, source (process: asked to the running process; file: the end of the file Claude Code writes the output to, the task's own file and no other, never one of another machine; report: what a subagent handed back; none: nothing to read, as for a remote agent, a teammate or a workflow, a task whose file is gone, or a subagent that has not handed anything back yet), output, totalBytes (the size of the whole output) and truncated (the start of the output is not in output). The output is its end, bytes of it at most: 8192 by default, 65536 at most, and a task that still runs gives 8192 at most, as the window does; it is cut at a character. The window itself shows up to 256 KiB of a task that is over: a client cannot ask for that much, because Claude Code refuses an answer of more than about 25,000 tokens. A subagent's report is given whole (16,000 bytes at most) and bytes does not apply to it. Open to every client, in the projects it sees (an agent of Escouade sees its own project and every project that is not private; a private project is seen by its own agents alone).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_background_output(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<GetBackgroundOutputArgs>,
    ) -> CallToolResult {
        let bytes = args.bytes.map(|b| b.to_string());
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
            ("id", Some(&args.id)),
            ("bytes", bytes.as_deref()),
        ]);
        let result = background::output(&ctx.core, &ctx.caller, args).await;
        ctx.reply("get_background_output", &asked, result)
    }

    #[tool(
        description = "List the questions and the permissions that Escouade's Claude Code agents wait for an answer to, in the order they came, of every agent, of every agent of one project, or of one agent: for each, the agent (id, name, project, projectId, the key of its ticket, null when none), request (its id, which answer_permission and answer_question name), type (permission, question, or plan for a plan the agent asks to have approved), since (epoch milliseconds), tool, arg (what the tool acts on: the command, the file, the whole plan; a command or a plan up to 20,000 characters), input (for a permission, all it asks as Claude Code sent it: the content to write, the strings of an edit, the flags of a command; each text up to 20,000 characters), description, reason (the permission rule Claude Code met), defaultNo (Claude Code would refuse it by default), and for a question its questions with their options (label and description) and whether several may be picked (multiSelect). cut is true when a text was cut: the request in the conversation has it whole. An agent in the Claude Code view has none here: its requests are in its terminal.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_requests(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ListRequestsArgs>,
    ) -> CallToolResult {
        let result = read::requests(
            &ctx.core,
            &ctx.caller,
            args.agent.as_deref(),
            args.project.as_deref(),
        );
        let asked = summary(&[
            ("agent", args.agent.as_deref()),
            ("project", args.project.as_deref()),
        ]);
        ctx.reply("list_requests", &asked, result)
    }

    #[tool(
        description = "Wait until one of the tickets named changes, then say what changed: its column, its block (blocked, conflict), its agent starting or ceasing to wait for an answer (waiting: list_requests gives the requests), its agent coming to rest in To review (resting: the ticket can be approved), or the ticket deleted (gone). Answers at the first change, or after timeout seconds (120 by default, 240 at most) with changed false. For each change, what changed (what) and how the ticket was (before); every ticket named is given as it is now (key, id, column, blocked, conflict, step, waiting, resting, gone). Each answer gives since: pass it to the next wait_ticket (even across other tool calls) and a ticket whose state differs from the one it holds is answered at once, a change made between the two calls included; without since, each ticket's state at the call is the one compared.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn wait_ticket(
        &self,
        ctx: ToolCtx<R>,
        context: RequestContext<RoleServer>,
        Parameters(args): Parameters<WaitTicketArgs>,
    ) -> CallToolResult {
        let tickets = args.tickets.join(", ");
        let timeout = args.timeout.map(|t| t.to_string());
        let asked = summary(&[("tickets", Some(&tickets)), ("timeout", timeout.as_deref())]);
        // Cancelled when the client leaves: its connection closes (a stateless request has no
        // other way to be cancelled).
        let result = wait::wait(&ctx.core, &ctx.caller, args, context.ct.cancelled()).await;
        ctx.reply("wait_ticket", &asked, result)
    }

    #[tool(
        description = "Create a ticket in the To do column of a project's Kanban board, at the end of the column. When the project's autopilot is on, it starts by itself as soon as a place is free and the tickets it comes after are done. The description ends with a line saying who created the ticket. You may attach images (screenshots, mock-ups) with images, each given by its content in base64 (data, with a name) or by the full path of a file on this computer (path; an agent of Escouade can only give a file of its working folder): PNG, JPEG, GIF or WebP, read from their content, 5 MB each, 18 MB in all, 20 at most. The ticket's agent gets them with its first message. You may also attach other files of any format (a spreadsheet, an archive, a log, a PDF…) with files, given the same two ways (by content in base64 for a small one, a request weighing 32 MiB at most; by path otherwise, the file then copied): 20 at most, each no bigger than the size the settings allow (Settings › Harness › Attachments, as in the window); the ticket's agent gets their paths with its first message. An image or a file that is refused refuses the whole call, and nothing is created. You may set the account, the model and the effort of its agents (account, model, effort; auto, the default, is the current account of its harness and the Kanban's model and effort, else the account's): a ticket set on an account starts there every time and stays there, waiting for its reset at its usage limit rather than going on another account or harness; an account its project may not use is refused, with why (reserved to other projects, or shared while the private project uses its own). Answers the ticket as get_ticket does.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<CreateTicketArgs>,
    ) -> CallToolResult {
        let after = args.after.as_ref().map(|a| a.join(", "));
        // How many, never what: no content, no path.
        let images = args.images.as_ref().map(|i| i.len().to_string());
        let files = args.files.as_ref().map(|f| f.len().to_string());
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("title", Some(&args.title)),
            ("after", after.as_deref()),
            ("images", images.as_deref()),
            ("files", files.as_deref()),
            ("account", args.account.as_deref()),
            ("model", args.model.as_deref()),
            ("effort", args.effort.as_deref()),
        ]);
        let result = act::create_ticket(&ctx.core, &ctx.caller, args).await;
        ctx.reply("create_ticket", &asked, result)
    }

    #[tool(
        description = "Edit a ticket of the To do column: the fields you leave out stay as they are. A ticket that has started cannot be edited, and a dependency that would make two tickets wait for each other is refused. When you change the title, the description or the criteria, the description ends with a line saying who edited the ticket, in the place of the one that said who created it. You can attach images with addImages (same form and limits as create_ticket's images, counting the ones the ticket keeps) and take some away with removeImages (each named by its file or its index, from 0, as get_ticket lists them): the images you do not name stay, and images alone leave the title, the description and the criteria as they are (no line saying who edited the ticket). The other files go the same way, with addFiles (same form and limits as create_ticket's files, counting the ones the ticket keeps) and removeFiles (each named by its file or its index, from 0, as get_ticket lists them in files): the files you do not name stay. Nothing changes when an image or a file is refused. account, model and effort are create_ticket's (auto goes back to the default); a ticket moved to another harness goes back to auto for its account. Answers the ticket as get_ticket does.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn update_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<UpdateTicketArgs>,
    ) -> CallToolResult {
        let after = args.after.as_ref().map(|a| a.join(", "));
        let added = args.add_images.as_ref().map(|i| i.len().to_string());
        let removed = args.remove_images.as_ref().map(|i| i.len().to_string());
        let added_files = args.add_files.as_ref().map(|f| f.len().to_string());
        let removed_files = args.remove_files.as_ref().map(|f| f.len().to_string());
        let asked = summary(&[
            ("ticket", Some(&args.ticket)),
            ("title", args.title.as_deref()),
            ("after", after.as_deref()),
            ("addImages", added.as_deref()),
            ("removeImages", removed.as_deref()),
            ("addFiles", added_files.as_deref()),
            ("removeFiles", removed_files.as_deref()),
            ("account", args.account.as_deref()),
            ("model", args.model.as_deref()),
            ("effort", args.effort.as_deref()),
        ]);
        let result = act::update_ticket(&ctx.core, &ctx.caller, args).await;
        ctx.reply("update_ticket", &asked, result)
    }

    #[tool(
        description = "Change the place of a ticket in the To do column of its project, which is the order the autopilot starts them in: to the top, to the bottom, or just before another To do ticket. A ticket that has started does not move. Answers the keys of the To do column in their new order.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn move_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<MoveTicketArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("ticket", Some(&args.ticket)),
            ("position", Some(args.position.name())),
            ("before", args.before.as_deref()),
        ]);
        let result = act::move_ticket(&ctx.core, &ctx.caller, args);
        ctx.reply("move_ticket", &asked, result)
    }

    #[tool(
        description = "Launch a To do ticket now, as the Launch button does: its agent starts as soon as a place is free. Refused while every Claude account the project may use is at its usage limit (the answer says until when); asked for so, it is not held by “Pause above quota” nor by a day's budget, which hold back the autopilot alone, when the project already has its most tickets in progress in parallel, when the ticket comes after tickets that are not done, and when the board cannot start anything. You may give the ticket's agent its own account, model and effort (they take precedence over the ticket's and the Kanban's, which get_board gives, and are the ticket's from then on; auto, or an empty string, goes back to the default): they are checked before anything starts. A model that has no auto mode (Haiku) starts its agent in the default mode where the Kanban's is auto. Answers once it is queued, with the model and the effort its agent starts with (the ticket's, else the Kanban's, else the app's default; no effort for a model that takes none, Haiku); follow it with get_ticket. It uses the quota.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn start_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<StartTicketArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("ticket", Some(&args.ticket)),
            ("account", args.account.as_deref()),
            ("model", args.model.as_deref()),
            ("effort", args.effort.as_deref()),
        ]);
        let result = act::start_ticket(&ctx.core, &ctx.caller, args).await;
        ctx.reply("start_ticket", &asked, result)
    }

    #[tool(
        description = "Approve a To review ticket, as the Approve… button does: the tests first when the project's board runs them, then the commit, then what its board settings say (merge into the target branch, push the branch and open a pull request, push the branch, or keep it); a ticket whose branch brings nothing more than its target (merged already) is closed without merging again. Answers once the approval has started, not at its end: follow it with wait_ticket or get_ticket (step, blocked, column, outcome). Failing tests send the ticket back In progress to its agent with their output, as the window does; any other step that fails leaves the ticket in To review, blocked with the reason. Refused for a ticket that is not in To review, while its agent works, while its approval already runs, and for your own ticket when you are an agent (it would merge your work without a review). Answers the ticket as get_ticket does, and since, the cursor of the ticket as it was before: pass it to wait_ticket to be told at once of what follows, a fast approval included.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = true
        )
    )]
    async fn approve_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ApproveTicketArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("ticket", Some(&args.ticket))]);
        let result = act::approve_ticket(&ctx.core, &ctx.caller, args);
        ctx.reply("approve_ticket", &asked, result)
    }

    #[tool(
        description = "Send a To review ticket back for rework, as the Send back button does: it goes back In progress with the same agent and worktree, on loop 1 again, and the comment is sent to its agent, which reads it as test feedback followed by who sent the ticket back (sent back by <you> through Escouade), then fixes it, checks every criterion again and ends with its report. Refused for a blank comment, for a ticket that is not in To review or has no agent, while its agent works (wait for the end of its turn), while its approval runs, while the agent's own Claude account is at its usage limit (or has an agent waiting for its quota: the answer says until when; “Pause above quota” holds back the autopilot alone), and for your own ticket when you are an agent. It uses the quota. Answers the ticket as get_ticket does, and since, the cursor of the ticket as it was before: pass it to wait_ticket to be told at once of what follows.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn reject_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<RejectTicketArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("ticket", Some(&args.ticket)),
            ("comment", Some(args.comment.trim())),
        ]);
        let result = act::reject_ticket(&ctx.core, &ctx.caller, args).await;
        ctx.reply("reject_ticket", &asked, result)
    }

    #[tool(
        description = "Take up a blocked In progress ticket, as the Resume button does: its agent goes on where it was (an agent that never got the ticket gets it, with its description and criteria), or, when its agent is gone, the ticket starts again with a new agent even if the autopilot is off. Refused for a ticket that is not In progress or not blocked, while its agent waits for its quota (the answer says until when), while the Claude account that would take it is at its usage limit (the answer says until when; “Pause above quota” holds back the autopilot alone), and for your own ticket when you are an agent. It uses the quota. Answers the ticket as get_ticket does, and since, the cursor of the ticket as it was before: pass it to wait_ticket to be told at once of what follows.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn resume_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ResumeTicketArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("ticket", Some(&args.ticket))]);
        let result = act::resume_ticket(&ctx.core, &ctx.caller, args).await;
        ctx.reply("resume_ticket", &asked, result)
    }

    #[tool(
        description = "Hand a merge conflict to the ticket's agent, as the \"Let the agent resolve\" button does on a To review ticket whose approval stopped on a conflict: the target branch is merged into the ticket's branch with the conflicts left in its worktree (with the rebase strategy, the agent is asked to rebase instead), the ticket goes back In progress on loop 1, and the agent is asked to resolve the conflicts and to end with its report. Refused for a ticket that is not in To review or has no conflict, while its approval runs, while its agent works, while the agent's own Claude account is at its usage limit (or has an agent waiting for its quota: the answer says until when; “Pause above quota” holds back the autopilot alone), and for your own ticket when you are an agent. It uses the quota. Answers the ticket as get_ticket does, and since, the cursor of the ticket as it was before: pass it to wait_ticket to be told at once of what follows.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn resolve_conflict(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ResolveConflictArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("ticket", Some(&args.ticket))]);
        let result = act::resolve_conflict(&ctx.core, &ctx.caller, args).await;
        ctx.reply("resolve_conflict", &asked, result)
    }

    #[tool(
        description = "Dismiss the block of a To review ticket (a merge conflict, a step that failed), as the Cancel button does: the block goes and nothing else changes, the ticket stays in To review to be approved again, sent back, or its conflict handed to its agent. Refused for a ticket that is not in To review or not blocked, while its approval runs, and for your own ticket when you are an agent. Answers the ticket as get_ticket does, and since, the cursor of the ticket as it was before: pass it to wait_ticket to be told at once of what follows.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn dismiss_block(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<DismissBlockArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("ticket", Some(&args.ticket))]);
        let result = act::dismiss_block(&ctx.core, &ctx.caller, args);
        ctx.reply("dismiss_block", &asked, result)
    }

    #[tool(
        description = "Delete a ticket of any column, as the window's Delete does: the card is gone for good and this cannot be undone; a ticket In progress or To review has its agent archived, and with the board's worktree cleanup (on by default) its agents' worktrees are deleted unless they hold uncommitted changes or commits neither merged nor pushed; the tickets that came after it no longer wait for it. Refused while the ticket's approval runs (wait for its end), and for your own ticket when you are an agent. Answers the ticket's key, deleted (true) and agentArchived (whether its agent was archived with it).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn delete_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<DeleteTicketArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("ticket", Some(&args.ticket))]);
        let result = act::delete_ticket(&ctx.core, &ctx.caller, args).await;
        ctx.reply("delete_ticket", &asked, result)
    }

    #[tool(
        description = "Change a project's Kanban settings, the same ones as the window's Settings > Kanban: what approving a ticket does (action, target, strategy, draft, cleanup, conventional, conflict), how many tickets run at once (maxParallel, 1 to 6), the autopilot, and the model, effort and mode of the tickets' agents (an empty string is the app's default). The fields you leave out stay as they are; read them first with get_board, whose answer can be given back as it is (nothing changes then). Checked as the window does, and nothing changes when one is refused: the model must be one the app offers, the effort one of low, medium, high, xhigh or max, the mode one of auto, default, plan or acceptEdits, the target a local branch of the project (one that is the Kanban's own, or the branch get_board shows while it has none, is left as it is), maxParallel between 1 and 6. Refused: the mode bypassPermissions (agents that never ask for a permission are the user's to allow, in the window) unless the Kanban is already on it, and turning testsFirst on when the Kanban has no test command (the server runs no command, and does not set one). An agent of Escouade sets the Kanban of its own project and of every project that is not private; a private project is set by its own agents alone. It changes the tickets that start from now on, not the agents already at work. Answers the settings as get_board does, after the change.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn set_board(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<SetBoardArgs>,
    ) -> CallToolResult {
        let given = |v: Option<bool>| v.map(|v| v.to_string());
        let (draft, cleanup, conventional) = (
            given(args.draft),
            given(args.cleanup),
            given(args.conventional),
        );
        let (autopilot, tests_first) = (given(args.autopilot), given(args.tests_first));
        let max_parallel = args.max_parallel.map(|n| n.to_string());
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("action", args.action.map(ActionArg::name)),
            ("target", args.target.as_deref()),
            ("strategy", args.strategy.map(StrategyArg::name)),
            ("draft", draft.as_deref()),
            ("cleanup", cleanup.as_deref()),
            ("conventional", conventional.as_deref()),
            ("conflict", args.conflict.map(ConflictArg::name)),
            ("maxParallel", max_parallel.as_deref()),
            ("autopilot", autopilot.as_deref()),
            ("model", args.model.as_deref()),
            ("effort", args.effort.as_deref()),
            ("mode", args.mode.as_deref()),
            ("testsFirst", tests_first.as_deref()),
        ]);
        let result = act::set_board(&ctx.core, &ctx.caller, args).await;
        ctx.reply_changed("set_board", &asked, result)
    }

    #[tool(
        description = "Start a new Claude Code agent in a project and send it its first message, which it reads headed by your name (Message from <you>: …), as coming from you and not from the user. It works on its own from then on: follow it with get_agent_summary. Refused while every Claude account the project may use is at its usage limit (the answer says until when); asked for so, it is not held by “Pause above quota” nor by a day's budget, which hold back the autopilot alone and when the project already has its most agents working at once (its Kanban's In parallel setting). It uses the quota. Answers the agent's id and name.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn create_agent(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<CreateAgentArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("message", Some(&args.message)),
        ]);
        let result = act::create_agent(&ctx.core, &ctx.caller, args).await;
        ctx.reply("create_agent", &asked, result)
    }

    #[tool(
        description = "Send a message to an agent of Escouade (32,000 characters at most). The agent reads it, and the conversation shows it, headed by your name (Message from <you>: …), as coming from you and not from the user; an agent at work reads it after its current turn. Refused while the agent's own Claude account is at its usage limit (or has an agent waiting for its quota: the answer says until when; “Pause above quota” holds back the autopilot alone), for an archived agent, and for yourself when you are an agent. It uses the quota. Answers the agent's id and name, and whether the message waits for the end of a turn.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn send_message(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<SendMessageArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
            ("text", Some(&args.text)),
        ]);
        let result = act::send_message(&ctx.core, &ctx.caller, args).await;
        ctx.reply("send_message", &asked, result)
    }

    #[tool(
        description = "Write to a subagent that works in the background, as the message box of its view in the window does (list_background_work gives the ids; a subagent may also be named by its taskId). The message is 32,000 characters at most. Claude Code lets no host write to a subagent: the agent it belongs to is asked to pass the message on with its SendMessage tool, and does so in a turn of its own, reading it and the subagent reading it headed by your name (Message from <you>: …), as coming from you and not from the user. Refused, as the window refuses, for a subagent that no longer runs, for a subagent in the foreground (the agent waits for it and would only pass the message on once it ended), for a subagent whose task Claude Code has not told yet, for an agent in the Claude Code view (the message is typed in its interface), for an archived agent, and for a subagent of yourself when you are an agent; and, once none of those applies, while the agent's own Claude account is at its usage limit (or has an agent waiting for its quota: the answer says until when; “Pause above quota” holds back the autopilot alone). A refusal that waiting would not change is therefore told before the one that says to try again later. It uses the quota. The activity log keeps the agent and the subagent, never the message. Answers the agent (id, name), the subagent (the call that launched it) and whether the message waits for the end of the agent's turn (queued).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn message_subagent(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<MessageSubagentArgs>,
    ) -> CallToolResult {
        // Never the words: the log keeps who was written to.
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
            ("subagent", Some(&args.subagent)),
        ]);
        let result = background::message(&ctx.core, &ctx.caller, args).await;
        ctx.reply("message_subagent", &asked, result)
    }

    #[tool(
        description = "Interrupt an agent's current turn, as the stop button of its conversation does: the agent stays, and can be sent a message after. Refused for yourself when you are an agent. Answers the agent's id and name, and whether a turn was running.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn stop_agent(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<StopAgentArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
        ]);
        let result = act::stop_agent(&ctx.core, &ctx.caller, args).await;
        ctx.reply("stop_agent", &asked, result)
    }

    #[tool(
        description = "Stop a subagent that works in the background, or a task Claude Code runs in the background for an agent (a command, a monitor, a remote agent, a teammate or a workflow), as the Stop button of its row in the window does: Claude Code is asked to stop its task (list_background_work gives the ids; a subagent may also be named by its taskId). The row ends with the frames Claude Code sends next, not before this answers: list_background_work shows it. The agent and its turn go on. Refused, as the window refuses, for something that no longer runs, for a subagent in the foreground (the agent waits for it: stop_agent interrupts the agent's turn), for a subagent whose task Claude Code has not told yet, for an agent whose process is not running, for an agent in the Claude Code view (its interface stops them), for an archived agent, and for yourself when you are an agent. Answers the agent (id, name), id (the subagent's call, or the task's), type (as list_background_work gives it) and stopped (true).",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn stop_background_work(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<StopBackgroundWorkArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
            ("id", Some(&args.id)),
        ]);
        let result = background::stop(&ctx.core, &ctx.caller, args).await;
        ctx.reply("stop_background_work", &asked, result)
    }

    #[tool(
        description = "Answer a permission an agent of Escouade asks for, or approve or refuse its plan, as the Allow and Deny buttons of its conversation do: allow lets the tool run (or approves the plan), deny refuses it, and the message tells the agent why, or what to do instead. Always allow is not offered: a lasting permission rule is the user's to give, in the window. Refused for a request of your own when you are an agent, for a request that no longer waits (the user or another client answered it, or its agent no longer runs), and for an agent in the Claude Code view (its requests are in its terminal). The agent's conversation shows who answered (Allowed by <you> through Escouade) and a refusal tells the agent who refused. Answers the agent's id and name, the request and the decision.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = false
        )
    )]
    async fn answer_permission(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<AnswerPermissionArgs>,
    ) -> CallToolResult {
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
            ("request", Some(&args.request)),
            ("decision", Some(args.decision.name())),
            ("message", args.message.as_deref()),
        ]);
        let result = act::answer_permission(&ctx.core, &ctx.caller, args);
        ctx.reply("answer_permission", &asked, result)
    }

    #[tool(
        description = "Answer a question an agent of Escouade asks (AskUserQuestion), as the question card of its conversation does: for each of the request's questions, the options picked by their labels (several only when the question allows it) or a free text. Every question needs an answer. Refused for a request of your own when you are an agent, for a request that no longer waits (the user or another client answered it, or its agent no longer runs), and for an agent in the Claude Code view (its requests are in its terminal). The agent's conversation shows who answered (Answered by <you> through Escouade), and the agent reads each answer followed by who gave it (SQLite (answer from <you> through Escouade)). Answers the agent's id and name, the request and the answers as given.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn answer_question(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<AnswerQuestionArgs>,
    ) -> CallToolResult {
        let answers = act::answers_summary(&args.answers);
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
            ("request", Some(&args.request)),
            ("answers", Some(&answers)),
        ]);
        let result = act::answer_question(&ctx.core, &ctx.caller, args);
        ctx.reply("answer_question", &asked, result)
    }

    #[tool(
        description = "Only for Escouade's own agents (refused to any other client). Report in one short line what you are doing: it shows on your card and on your ticket's card until your next report (120 characters at most; an empty line takes it off). Answers the line as kept.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn report_progress(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ReportProgressArgs>,
    ) -> CallToolResult {
        let asked = summary(&[("line", Some(args.line.trim()))]);
        let result = act::report_progress(&ctx.core, &ctx.caller, args);
        ctx.reply("report_progress", &asked, result)
    }

    #[tool(
        description = "Only for Escouade's own agents (refused to any other client, and to an agent without a ticket). Split the work that remains of your ticket into new To do tickets (10 at most per call), made after it: with chain true (the default) each comes after the one before it, the first after yours; with chain false all come after yours alone and may run in parallel. They start once the tickets they come after are done. Each ends with a line saying who made it. Each also gets its own copies of the images of your ticket: all of them by default, or those you name in its images (by file or index, from 0, as get_ticket lists them), none with an empty list; a reference that fits no image of your ticket, or that names one whose file is gone, refuses the whole call. When the images are left out (images not given), one whose file is gone is skipped, not copied. The other files of your ticket go the same way, with files: all of them by default (one whose file is gone, or bigger than the settings now allow, is skipped), those you name, or none with an empty list; one named that cannot be copied refuses the whole call before any ticket is made. Answers the keys of the tickets made; when an image or a file was skipped, an object with those keys (tickets) and one sentence for each one skipped (skipped).",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            open_world_hint = false
        )
    )]
    async fn split_ticket(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<SplitTicketArgs>,
    ) -> CallToolResult {
        let titles = args
            .tickets
            .iter()
            .map(|t| t.title.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        // How many images were named in all; nothing when no ticket said anything of them.
        let images = args.tickets.iter().any(|t| t.images.is_some()).then(|| {
            let named: usize = args
                .tickets
                .iter()
                .map(|t| t.images.as_ref().map_or(0, Vec::len))
                .sum();
            named.to_string()
        });
        let files = args.tickets.iter().any(|t| t.files.is_some()).then(|| {
            let named: usize = args
                .tickets
                .iter()
                .map(|t| t.files.as_ref().map_or(0, Vec::len))
                .sum();
            named.to_string()
        });
        let asked = summary(&[
            ("tickets", Some(&titles)),
            ("images", images.as_deref()),
            ("files", files.as_deref()),
        ]);
        let result = act::split_ticket(&ctx.core, &ctx.caller, args).await;
        ctx.reply("split_ticket", &asked, result)
    }
}

/// A tool of the tests only: who the server takes the caller for.
#[cfg(test)]
#[tool_router(router = test_router)]
impl<R: Runtime> Tools<R> {
    #[rmcp::tool(description = "Tests only: who the server takes the caller for.")]
    async fn whoami(&self, ctx: ToolCtx<R>) -> CallToolResult {
        let who = match &ctx.caller {
            Caller::External => serde_json::json!({ "caller": "external" }),
            Caller::Agent(id) => serde_json::json!({ "caller": "agent", "agent": id }),
        };
        ctx.reply("whoami", "", Ok(who.to_string()))
    }
}

#[tool_handler(
    router = self.router,
    name = "escouade",
    instructions = "Escouade runs Claude Code agents on a Kanban board of tickets, project by project. Its tools read and act on its projects, agents, tickets and Kanban settings. Name a project by its id or its name, an agent by its id or its name, a ticket by its key (like ATL-12) or its id. An agent of Escouade acts on its own project and on every project that is not private; a private project is known to its own agents alone, who know and act on no other project."
)]
impl<R: Runtime> ServerHandler for Tools<R> {
    /// A call goes to its tool, which logs it (`ToolCtx::reply`); one the router answers itself
    /// (an unknown tool, arguments that do not fit its schema) is logged here.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tool = request.name.to_string();
        let ctx = ToolCtx::<R>::of(&context);
        let answer = self
            .router
            .call(ToolCallContext::new(self, request, context))
            .await;
        let Some(ctx) = ctx.filter(|c| !c.logged.load(Ordering::Acquire)) else {
            return answer;
        };
        // What the router says repeats the client’s own values (serde quotes them): neither the log
        // nor the reply keeps one. The tool’s name is the client’s too, and is cut short.
        let (answer, why) = match answer {
            Err(e) => {
                let said = activity::clip(before_any_value(&e.message));
                (Err(ErrorData::new(e.code, said.clone(), None)), said)
            }
            // Arguments that do not fit: rmcp answers an error result the model reads.
            Ok(CallToolResponse::Complete(r)) => {
                let text = serde_json::to_value(&r)
                    .ok()
                    .and_then(|v| v["content"][0]["text"].as_str().map(str::to_string))
                    .unwrap_or_default();
                // The reason is cut ahead of the value, a fixed sentence says what to do.
                let reason: String = before_any_value(&text).chars().take(200).collect();
                let reply = tr!(
                    "{reason}. Relis le schéma d’entrée de l’outil, puis refais l’appel.",
                    "{reason}. Read the tool’s input schema again, then call it again."
                );
                let blocks = vec![ContentBlock::text(activity::clip(&reply))];
                let why = tr!(
                    "les arguments ne correspondent pas au schéma de l’outil",
                    "the arguments do not match the tool’s schema"
                );
                (Ok(CallToolResult::error(blocks).into()), why)
            }
            other => (other, String::new()),
        };
        let message = tr!(
            "Appel refusé par le serveur : {why}",
            "Call refused by the server: {why}"
        );
        activity::record(
            &ctx.core,
            Some(&ctx.caller),
            &activity::clip(&tool),
            "",
            Outcome::Error(message),
        );
        answer
    }
}

/// `message` up to the first value of the client’s that it quotes: serde puts them in double
/// quotes or backticks (`invalid type: string "…"`, `unknown variant `x``), and one may be a path, or
/// the 32 MB of an image’s content.
fn before_any_value(message: &str) -> &str {
    let end = message.find(['"', '`']).unwrap_or(message.len());
    message[..end].trim_end_matches([' ', ':'])
}
