//! The MCP server on HTTP: the transport and its guards are `escouade-mcp`'s (`http`); the app's
//! core is the server they stand before (`Host`): it knows the tokens, logs a refusal, and gives
//! the tools what they work with (`ToolCtx`).

use super::activity::{self, Outcome};
use super::tools::ToolCtx;
use super::Caller;
use crate::core::Core;
use std::sync::Arc;
use tauri::Runtime;

pub use escouade_mcp::http::*;

impl<R: Runtime> Host for Core<R> {
    type Ctx = ToolCtx<R>;

    fn caller(&self, token: &str) -> Option<Caller> {
        self.mcp.caller(token)
    }

    fn refused(&self, caller: Option<&Caller>, asked: &str, message: String) {
        activity::record(self, caller, "", asked, Outcome::Refused(message));
    }

    fn ctx(self: Arc<Self>, caller: Caller) -> ToolCtx<R> {
        ToolCtx::new(self, caller)
    }
}
