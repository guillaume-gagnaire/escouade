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
//! A call the router refuses before its tool (unknown, arguments that do not fit the schema) is
//! logged by `call_tool`. What a tool's arguments name is found by `resolve`; what the reading
//! ones answer is built by `read`, and what the ones that act do is in `act`.
//!
//! None accepts a permission, answers for the user, runs a command or the tests, merges or
//! deletes: `tests::EXPOSED` lists exactly what the router holds, so a tool is added knowingly.

use super::act;
use super::activity::{self, Outcome};
use super::read;
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
        let (outcome, answer) = match result {
            Ok(text) => (
                Outcome::Ok,
                CallToolResult::success(vec![ContentBlock::text(text)]),
            ),
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

    /// Every tool the server offers (and, in tests, the test's own).
    pub fn router() -> ToolRouter<Self> {
        let router = Self::tool_router();
        #[cfg(test)]
        let router = router + Self::test_router();
        router
    }
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
pub struct GetAgentSummaryArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
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
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateTicketArgs {
    #[schemars(description = "The ticket: its key (like ATL-12, in any case) or its id.")]
    pub ticket: String,
    #[schemars(
        description = "The new title, on one line (200 characters at most). Leave it out to keep the current one."
    )]
    pub title: Option<String>,
    #[schemars(
        description = "The new description (10,000 characters at most). Leave it out to keep the current one."
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
        description = "The model, as Claude Code names it (sonnet, opus, haiku…). Leave it out for the default model."
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
        description = "The message. The agent reads it, and the conversation shows it, headed by your name (Message from <you>: …), as coming from you and not from the user."
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
        description = "List Escouade's projects, in the app's order: for each one its id, name, folder, current git branch (null when none), number of agents (archived ones left out) and number of tickets in each column of its Kanban board (todo, doing, review, done).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_projects(&self, ctx: ToolCtx<R>) -> CallToolResult {
        let result = read::projects(&ctx.core).await;
        ctx.reply("list_projects", "", result)
    }

    #[tool(
        description = "List Escouade's Claude Code agents, oldest first (archived ones left out), of every project or of one: their id, name, project (its name) and projectId, status (idle, running, waiting, done or error), whether a question or a permission waits for the user's answer, the key of their ticket (null when none), their model, their Claude account and what they cost so far in US dollars.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_agents(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ListAgentsArgs>,
    ) -> CallToolResult {
        let result = read::agents(&ctx.core, args.project.as_deref());
        let asked = summary(&[("project", args.project.as_deref())]);
        ctx.reply("list_agents", &asked, result)
    }

    #[tool(
        description = "List the tickets of a project's Kanban board in the board's order (todo by priority, doing and review by arrival, done newest first), of every column or of one: their id, key, title, column, the keys of the tickets they come after, their agent's name, and why they are blocked when they are.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_tickets(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<ListTicketsArgs>,
    ) -> CallToolResult {
        let result = read::tickets(&ctx.core, &args.project, args.column.map(Column::from));
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("column", args.column.map(ColumnArg::name)),
        ]);
        ctx.reply("list_tickets", &asked, result)
    }

    #[tool(
        description = "Read a ticket whole: its title, description, acceptance criteria (each with whether it is met and its agent's note), loops (the current one and the most it may run), column, the keys of the tickets it comes after, the features in place so far as its agent listed them, and the external ticket it was imported from (Jira, Trello or GitHub; null when none). An agent of Escouade may leave out the ticket to read its own.",
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
        description = "Read the Claude quota: for each Claude account, whether new agents go to it now (current: the first active account whose windows are under the autopilot's threshold, else the first active one), and how much of its own 5-hour and weekly windows is used (pct, 0 to 100) with when each resets (resetsAt, epoch milliseconds), null when not read; and why the autopilot holds the tickets back, when it does (reason: fiveHour, week or limit), until when (epoch milliseconds).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_usage(&self, ctx: ToolCtx<R>) -> CallToolResult {
        let result = read::usage(&ctx.core);
        ctx.reply("get_usage", "", result)
    }

    #[tool(
        description = "Sum up an agent without reading its conversation: its id, name, status, the key of its ticket (null when none), its last message (null before its first; 2,000 characters at most, a longer one given by its start and its end, where it says what it concluded or asks, joined by \" … \") and the files it edited (the latest 100 at most, relative to its folder).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_agent_summary(
        &self,
        ctx: ToolCtx<R>,
        Parameters(args): Parameters<GetAgentSummaryArgs>,
    ) -> CallToolResult {
        let result = read::agent_summary(&ctx.core, &args.agent, args.project.as_deref());
        let asked = summary(&[
            ("agent", Some(&args.agent)),
            ("project", args.project.as_deref()),
        ]);
        ctx.reply("get_agent_summary", &asked, result)
    }

    #[tool(
        description = "Create a ticket in the To do column of a project's Kanban board, at the end of the column. When the project's autopilot is on, it starts by itself as soon as a place is free and the tickets it comes after are done. Answers the ticket as get_ticket does.",
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
        let asked = summary(&[
            ("project", Some(&args.project)),
            ("title", Some(&args.title)),
            ("after", after.as_deref()),
        ]);
        let result = act::create_ticket(&ctx.core, &ctx.caller, args).await;
        ctx.reply("create_ticket", &asked, result)
    }

    #[tool(
        description = "Edit a ticket of the To do column: the fields you leave out stay as they are. A ticket that has started cannot be edited, and a dependency that would make two tickets wait for each other is refused. Answers the ticket as get_ticket does.",
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
        let asked = summary(&[
            ("ticket", Some(&args.ticket)),
            ("title", args.title.as_deref()),
            ("after", after.as_deref()),
        ]);
        let result = act::update_ticket(&ctx.core, &ctx.caller, args);
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
        description = "Launch a To do ticket now, as the Launch button does: its agent starts as soon as a place is free. Refused while the autopilot is paused (quota or usage limit: the answer says until when), when the project already has its most tickets in progress in parallel, when the ticket comes after tickets that are not done, and when the board cannot start anything. Answers once it is queued; follow it with get_ticket. It uses the quota.",
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
        let asked = summary(&[("ticket", Some(&args.ticket))]);
        let result = act::start_ticket(&ctx.core, &ctx.caller, args);
        ctx.reply("start_ticket", &asked, result)
    }

    #[tool(
        description = "Start a new Claude Code agent in a project and send it its first message, which it reads headed by your name (Message from <you>: …), as coming from you and not from the user. It works on its own from then on: follow it with get_agent_summary. Refused while the autopilot is paused (the answer says until when) and when the project already has its most agents working at once (its Kanban's In parallel setting). It uses the quota. Answers the agent's id and name.",
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
        description = "Send a message to an agent of Escouade. The agent reads it, and the conversation shows it, headed by your name (Message from <you>: …), as coming from you and not from the user; an agent at work reads it after its current turn. Refused while the autopilot is paused (the answer says until when), for an archived agent, and for yourself when you are an agent. It uses the quota. Answers the agent's id and name, and whether the message waits for the end of a turn.",
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
        description = "Only for Escouade's own agents (refused to any other client, and to an agent without a ticket). Split the work that remains of your ticket into new To do tickets (10 at most per call), made after it: with chain true (the default) each comes after the one before it, the first after yours; with chain false all come after yours alone and may run in parallel. They start once the tickets they come after are done. Answers the keys of the tickets made.",
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
        let asked = summary(&[("tickets", Some(&titles))]);
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
    instructions = "Escouade runs Claude Code agents on a Kanban board of tickets, project by project. Its tools read and act on its projects, agents and tickets. Name a project by its id or its name, an agent by its id or its name, a ticket by its key (like ATL-12) or its id."
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
        let why = match &answer {
            Err(e) => e.message.to_string(),
            // Arguments that do not fit: rmcp answers an error result the model reads.
            Ok(CallToolResponse::Complete(r)) => serde_json::to_value(r)
                .ok()
                .and_then(|v| v["content"][0]["text"].as_str().map(str::to_string))
                .unwrap_or_default(),
            Ok(_) => String::new(),
        };
        let message = tr!(
            "Appel refusé par le serveur : {why}",
            "Call refused by the server: {why}"
        );
        activity::record(
            &ctx.core,
            Some(&ctx.caller),
            &tool,
            "",
            Outcome::Error(message),
        );
        answer
    }
}
