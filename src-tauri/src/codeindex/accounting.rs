//! Ported from tokensave `src/mcp/tools/accounting.rs` and the accounting of `McpServer`'s
//! `tools/call` (`src/mcp/server.rs`, lines 3267-3376) (github.com/aovestdipaperino/tokensave,
//! commit 580936b, MIT, © 2025 Enzo Lombardi; see THIRD-PARTY-NOTICES.md): what an answer of the
//! index saved, counted as tokensave counts it.
//!
//! - `before`: the weight of every file the answer touched (its bytes divided by 4), capped at
//!   4 times the answer (`REF_CAP_K`), at the answer itself for a literal search;
//! - `after`: what the model paid for the call: the answer, the request (the tool's name, its
//!   arguments and the JSON-RPC framing), once per session the schemas of the tools, and the
//!   metrics line itself;
//! - a call whose `after` exceeds its `before` credits nothing: the shortfall is a debt that the
//!   next calls of the session pay down before anything more is credited.
//!
//! What differs, for our architecture: the session is the caller's (an agent, or the clients from
//! outside), not an MCP connection, since Escouade's server makes one handler per request; the
//! schemas charged are those of the index's tools (all of them are listed up front: none is
//! deferred by `anthropic/alwaysLoad`); the metrics line is always sent (tokensave turns it off by
//! default, #561), in the user's language.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// How a tool's `before` (baseline) token count should be computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselinePolicy {
    /// The tool returns full (or near-full) file content — charge the full weight of every
    /// touched file.
    FullFile,
    /// The tool returns references or summaries, not file content — cap the baseline relative to
    /// what was actually returned (see [`cap_baseline`]).
    Reference,
    /// Literal search: the realistic alternative is `grep -n`, which returns roughly the matching
    /// lines, so the baseline is capped at the content.
    Literal,
}

/// Reference-tool baselines are capped at `content_tokens * REF_CAP_K`: a rough upper bound on
/// how much source an agent would plausibly have read by hand to find what the tool just
/// returned, scaled to the size of the answer rather than the number of files merely referenced.
pub const REF_CAP_K: u64 = 4;

/// Classifies a tool by name for the token-savings baseline calculation: every tool is
/// [`BaselinePolicy::Reference`], but a literal search.
pub fn baseline_policy(tool_name: &str, args: &Value) -> BaselinePolicy {
    if tool_name == "index_search"
        && args
            .get("literal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        BaselinePolicy::Literal
    } else {
        BaselinePolicy::Reference
    }
}

/// Applies `policy` to a raw full-file baseline (`full_file_tokens`, the sum of touched files'
/// token weight) given the tokens actually present in the response content (`content_tokens`,
/// not the overhead-inclusive `after`).
pub fn cap_baseline(policy: BaselinePolicy, full_file_tokens: u64, content_tokens: u64) -> u64 {
    match policy {
        BaselinePolicy::FullFile => full_file_tokens,
        BaselinePolicy::Reference => full_file_tokens.min(content_tokens.saturating_mul(REF_CAP_K)),
        BaselinePolicy::Literal => full_file_tokens.min(content_tokens),
    }
}

/// Approximate token overhead of the JSON-RPC envelope wrapping a tool call, beyond the tool name
/// and arguments themselves.
pub const FRAMING_CONST: u64 = 8;

/// tokensave `estimate_tokens` (`src/context/read_modes.rs`): a token per four characters,
/// rounded up.
pub fn estimate_tokens(s: &str) -> u64 {
    (s.chars().count() as u64).div_ceil(4)
}

/// Approximates the token cost of *sending* a tool call: the tool name, its serialized arguments,
/// and [`FRAMING_CONST`] for JSON-RPC framing. Never zero.
pub fn request_overhead_tokens(tool_name: &str, arguments: &Value) -> u64 {
    estimate_tokens(tool_name) + estimate_tokens(&arguments.to_string()) + FRAMING_CONST
}

/// Approximates the one-time token cost of the tool schemas the client loads into context at the
/// start of a session (`definitions`: the tools' listing, as JSON).
pub fn schema_overhead_tokens(definitions: &Value) -> u64 {
    estimate_tokens(&definitions.to_string())
}

/// Settles one call's raw signed savings (`before as i64 - after as i64`) against `debt`, the
/// carried-forward shortfall from earlier calls whose `after` exceeded `before`. Returns
/// `(new_debt, credited)`.
pub fn settle_session_debt(debt: i64, raw_delta: i64) -> (i64, u64) {
    let available = raw_delta - debt;
    if available >= 0 {
        (0, available as u64)
    } else {
        (-available, 0)
    }
}

/// The tokens of a response's text, as tokensave measures `content`: its bytes divided by 4.
pub fn response_tokens(text: &str) -> u64 {
    (text.len() / 4) as u64
}

/// The weight of the touched files (tokensave `get_file_token_map`: each file's size / 4).
pub fn file_tokens(size: u64) -> u64 {
    size / 4
}

/// What one call cost and saved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Metrics {
    /// The baseline, capped.
    pub before: u64,
    /// Everything the model paid for the call.
    pub after: u64,
    /// The tool's own answer (`result`).
    pub result: u64,
    /// `before - after`, never below nothing: what the line says.
    pub saved: u64,
    /// What the session's counter gains, its debt paid first.
    pub credited: u64,
}

/// The ledger of a session: whether the schemas were charged, and its debt.
#[derive(Debug, Default)]
pub struct Session {
    schema_charged: bool,
    debt: i64,
}

/// The sessions of the callers, by key.
#[derive(Debug, Default)]
pub struct Ledger {
    sessions: Mutex<HashMap<String, Session>>,
}

/// A call to account for.
pub struct Call<'a> {
    pub tool: &'a str,
    pub arguments: &'a Value,
    /// The answer's text, as sent (without the metrics line).
    pub response: &'a str,
    /// The touched files' raw weight (`file_tokens` of each).
    pub full_file_tokens: u64,
    /// The schemas' weight, charged on the session's first call.
    pub schema_tokens: u64,
}

impl Ledger {
    /// Accounts for `call` in the session `key`; `line` renders the metrics line for the numbers
    /// (before, after, result, saved), so its own cost is counted (two passes, as tokensave).
    pub fn account(&self, key: &str, call: &Call, line: impl Fn(&Metrics) -> String) -> Metrics {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = sessions.entry(key.to_string()).or_default();
        let schema_overhead = if session.schema_charged {
            0
        } else {
            session.schema_charged = true;
            call.schema_tokens
        };
        let tool_response_tokens = response_tokens(call.response);
        let request_overhead = request_overhead_tokens(call.tool, call.arguments);
        let after_pre_metrics = tool_response_tokens + request_overhead + schema_overhead;
        let policy = baseline_policy(call.tool, call.arguments);
        let before = cap_baseline(policy, call.full_file_tokens, tool_response_tokens);
        let emit_metrics_line = before > 0;
        let metrics_line_tokens = if emit_metrics_line {
            let provisional = Metrics {
                before,
                after: after_pre_metrics,
                result: tool_response_tokens,
                saved: before.saturating_sub(after_pre_metrics),
                credited: 0,
            };
            (line(&provisional).len() / 4) as u64
        } else {
            0
        };
        let after = after_pre_metrics + metrics_line_tokens;
        let raw_delta = before as i64 - after as i64;
        let (debt, credited) = settle_session_debt(session.debt, raw_delta);
        session.debt = debt;
        Metrics {
            before,
            after,
            result: tool_response_tokens,
            saved: before.saturating_sub(after),
            credited,
        }
    }

    /// Forgets a session (an agent gone).
    pub fn forget(&self, key: &str) {
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Ported from tokensave `src/mcp/tools/accounting.rs`'s tests.

    #[test]
    fn variable_delivery_tools_default_to_reference() {
        for tool in [
            "index_body",
            "index_context",
            "index_callers",
            "index_impact",
        ] {
            assert_eq!(baseline_policy(tool, &json!({})), BaselinePolicy::Reference);
        }
    }

    #[test]
    fn unknown_and_reference_tools_default_to_reference() {
        assert_eq!(
            baseline_policy("index_search", &json!({})),
            BaselinePolicy::Reference
        );
        assert_eq!(
            baseline_policy("index_some_future_tool", &json!({})),
            BaselinePolicy::Reference
        );
    }

    #[test]
    fn reference_cap_is_a_no_op_when_content_matches_or_exceeds_the_file() {
        let full_file_tokens = 10_000;
        let content_tokens = 10_500;
        assert_eq!(
            cap_baseline(BaselinePolicy::Reference, full_file_tokens, content_tokens),
            full_file_tokens
        );
    }

    #[test]
    fn full_file_baseline_is_never_capped() {
        assert_eq!(cap_baseline(BaselinePolicy::FullFile, 100_000, 10), 100_000);
    }

    #[test]
    fn reference_baseline_is_capped_at_k_times_content() {
        assert_eq!(cap_baseline(BaselinePolicy::Reference, 100_000, 10), 40);
    }

    #[test]
    fn reference_baseline_passes_through_when_under_the_cap() {
        assert_eq!(cap_baseline(BaselinePolicy::Reference, 30, 100), 30);
    }

    #[test]
    fn literal_search_baseline_is_capped_at_content() {
        assert_eq!(
            baseline_policy("index_search", &json!({ "literal": true })),
            BaselinePolicy::Literal
        );
        assert_eq!(cap_baseline(BaselinePolicy::Literal, 100_000, 10), 10);
    }

    #[test]
    fn reference_baseline_with_zero_content_tokens_caps_to_zero() {
        assert_eq!(cap_baseline(BaselinePolicy::Reference, 100_000, 0), 0);
    }

    #[test]
    fn request_overhead_is_never_zero() {
        assert!(request_overhead_tokens("index_search", &json!({})) >= FRAMING_CONST);
    }

    #[test]
    fn request_overhead_grows_with_argument_size() {
        let small = request_overhead_tokens("index_context", &json!({"task": "x"}));
        let large = request_overhead_tokens("index_context", &json!({"task": "x".repeat(1000)}));
        assert!(large > small);
    }

    #[test]
    fn settle_session_debt_defers_a_shortfall_instead_of_discarding_it() {
        assert_eq!(settle_session_debt(0, -9_800), (9_800, 0));
    }

    #[test]
    fn settle_session_debt_pays_down_before_crediting_anything() {
        assert_eq!(settle_session_debt(9_800, 900), (8_900, 0));
    }

    #[test]
    fn settle_session_debt_credits_the_remainder_once_debt_is_cleared() {
        assert_eq!(settle_session_debt(100, 150), (0, 50));
    }

    #[test]
    fn settle_session_debt_over_a_session_matches_the_signed_total() {
        let mut debt = 0i64;
        let mut total_credited = 0u64;
        for raw_delta in [-1_700, 900, 900] {
            let (new_debt, credited) = settle_session_debt(debt, raw_delta);
            debt = new_debt;
            total_credited += credited;
        }
        assert_eq!(debt, 0);
        assert_eq!(total_credited, 100);
    }

    #[test]
    fn schema_overhead_scales_with_definition_count() {
        let def =
            |name: &str| json!({"name": name, "description": "d".repeat(100), "inputSchema": {}});
        let few = schema_overhead_tokens(&json!([def("a")]));
        let many =
            schema_overhead_tokens(&json!([def("a"), def("b"), def("c"), def("d"), def("e")]));
        assert!(many > few);
        assert!(few > 0);
    }

    // Ours: the server's accounting (`server.rs`), session by session.

    fn line(m: &Metrics) -> String {
        format!(
            "\nindex_metrics: before={} after={} result={} saved={}",
            m.before, m.after, m.result, m.saved
        )
    }

    #[test]
    fn the_schemas_are_charged_once_per_session_and_the_shortfall_is_carried() {
        let ledger = Ledger::default();
        let args = json!({"query": "x"});
        let response = "r".repeat(400); // 100 tokens
        let call = Call {
            tool: "index_search",
            arguments: &args,
            response: &response,
            full_file_tokens: 10_000,
            schema_tokens: 1_000,
        };
        let first = ledger.account("a", &call, line);
        // before = min(10 000, 4 × 100); after = 100 + request + 1 000 + the line.
        assert_eq!(first.before, 400);
        assert_eq!(first.result, 100);
        let request = request_overhead_tokens("index_search", &args);
        let line_tokens = (line(&Metrics {
            before: 400,
            after: 100 + request + 1_000,
            result: 100,
            saved: 0,
            credited: 0,
        })
        .len()
            / 4) as u64;
        assert_eq!(first.after, 100 + request + 1_000 + line_tokens);
        assert_eq!((first.saved, first.credited), (0, 0));
        // The second call pays no schema, but first pays the debt down.
        let second = ledger.account("a", &call, line);
        assert_eq!(second.after, 100 + request + line_tokens);
        assert_eq!(second.saved, 400 - second.after);
        let debt = first.after as i64 - 400;
        assert_eq!(second.credited as i64, (second.saved as i64 - debt).max(0));
        // Another session pays the schemas on its own first call.
        assert_eq!(ledger.account("b", &call, line).after, first.after);
    }

    #[test]
    fn an_answer_that_touched_no_file_sends_no_line_and_charges_none() {
        let ledger = Ledger::default();
        let args = json!({});
        let call = Call {
            tool: "index_search",
            arguments: &args,
            response: "count: 0\n",
            full_file_tokens: 0,
            schema_tokens: 0,
        };
        let m = ledger.account("a", &call, line);
        assert_eq!(m.before, 0);
        assert_eq!(m.after, 2 + request_overhead_tokens("index_search", &args));
    }
}
