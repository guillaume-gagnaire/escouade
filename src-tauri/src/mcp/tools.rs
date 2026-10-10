//! The tools the MCP server offers (`Tools`, rmcp's router), and what each one works with: the
//! app's core and who calls (`ToolCtx`, from the request's token).
//!
//! A tool is a method of the `#[tool_router]` block below. It takes a `ToolCtx<R>` (any position
//! among its parameters), its arguments as `Parameters<T>` (`T: Deserialize + JsonSchema`, the
//! schema the model reads), and answers through `ToolCtx::reply`, which logs the call
//! (`activity::record`) and turns a refusal or a failure into an error result the model reads:
//!
//! ```text
//! #[tool(description = "List the projects…")]
//! async fn list_projects(&self, ctx: ToolCtx<R>) -> CallToolResult {
//!     let projects = …;
//!     ctx.reply("list_projects", "", Ok(serde_json::to_string(&projects)?))
//! }
//! ```
//!
//! None accepts a permission, answers for the user, runs a command or the tests, merges or
//! deletes: `tests::EXPOSED` lists exactly what the router holds, so a tool is added knowingly.

use super::activity::{self, Outcome};
use super::Caller;
use crate::core::Core;
use rmcp::handler::server::common::{AsRequestContext, FromContextPart};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{tool_handler, tool_router, ErrorData, ServerHandler};
use std::marker::PhantomData;
use std::sync::Arc;
use tauri::Runtime;

/// What a tool works with: the app's core, and who calls it (from the request's token, set by
/// `http::Gate` in the request's extensions).
pub struct ToolCtx<R: Runtime> {
    pub core: Arc<Core<R>>,
    pub caller: Caller,
}

impl<R: Runtime> Clone for ToolCtx<R> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            caller: self.caller.clone(),
        }
    }
}

/// Why a tool did not do what it was asked.
// Allowed unused until the tools of M2 and M4 answer with it (then drop the allow).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// A guard said no (the autopilot's pause, a tool reserved to agents…).
    Refused(String),
    /// It failed.
    Failed(String),
}

// Allowed unused until the tools of M2 and M4 answer through it (then drop the allow).
#[allow(dead_code)]
impl<R: Runtime> ToolCtx<R> {
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
        answer
    }
}

/// A tool's `ToolCtx` parameter: from the request's HTTP parts, which rmcp keeps in the call's
/// extensions.
impl<C: AsRequestContext, R: Runtime> FromContextPart<C> for ToolCtx<R> {
    fn from_context_part(context: &mut C) -> Result<Self, ErrorData> {
        context
            .as_request_context()
            .extensions
            .get::<hyper::http::request::Parts>()
            .and_then(|parts| parts.extensions.get::<ToolCtx<R>>())
            .cloned()
            .ok_or_else(|| ErrorData::internal_error("no caller for this request", None))
    }
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

// The tools: none yet (M2 adds the reading ones, M4 those that act).
#[tool_router(allow_empty, vis = "pub(crate)")]
impl<R: Runtime> Tools<R> {}

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
    instructions = "Escouade runs Claude Code agents on a Kanban board of tickets, project by project. Its tools read and act on its projects, agents and tickets."
)]
impl<R: Runtime> ServerHandler for Tools<R> {}
