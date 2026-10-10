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
//! ones answer is built by `read`.
//!
//! None accepts a permission, answers for the user, runs a command or the tests, merges or
//! deletes: `tests::EXPOSED` lists exactly what the router holds, so a tool is added knowingly.

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

// The tools: those that read (M2); M4 adds those that act.
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
        description = "Read the Claude quota: for each Claude account, whether new agents go to it now (current), and how much of its 5-hour and weekly windows is used (pct, 0 to 100) with when each resets (resetsAt, epoch milliseconds), null when not read; and why the autopilot holds the tickets back, when it does (reason: fiveHour, week or limit), until when (epoch milliseconds).",
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
