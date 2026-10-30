//! The permission rules of the plugins (PLG-8, ESC-180; docs/PLUGINS.md §1.7): before a permission
//! an agent asks for waits for the user (`AgentRt::add_pending`), the plugins of
//! `permissions.decide` turned on for its project are asked (`permissions/decide`), all at once,
//! each within its delay (500 ms). A refusal wins; else an allowance; else (`ask`, no answer, an
//! error, a crash) the request goes to the user as it would without them. Every answer is written
//! in the plugin's journal.
//!
//! A red permission (Réglages › Plugins), never given by a project: `.escouade.yml` turns no plugin
//! on.

use super::journal::Kind;
use super::runtime::{timeout_of, CallError, PhaseName, Runtime};
use super::{granted, Plugins, State};
use crate::i18n::Lang;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub const METHOD: &str = "permissions/decide";
/// How long a plugin whose process does not run yet has to start and answer: once, after which
/// it runs (and `warm` starts them with the app).
const COLD: Duration = Duration::from_secs(3);

/// A permission an agent just asked for, as its card in the conversation has it.
#[derive(Debug, Clone)]
pub struct Asked {
    /// The request's id (its card's).
    pub request_id: String,
    /// The tool, as Claude Code names it whatever the harness (Bash, Edit, WebFetch, mcp__…).
    pub tool: String,
    /// What the tool was given, its long texts cut.
    pub input: Value,
    /// What it acts on, as its card sums it up (the command, the file): for the journal.
    pub arg: String,
}

/// What the plugins decided of a request.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Allowed by the plugin named.
    Allow { by: String },
    /// Refused by the plugin named, with its reason.
    Deny { by: String, message: Option<String> },
    /// Left to the user.
    Ask,
}

/// What a request is, for a rule: `command`, `edit`, `read`, `fetch`, `mcp` or `tool`.
pub fn kind_of(tool: &str) -> &'static str {
    match tool {
        "Bash" | "BashOutput" | "KillShell" | "PowerShell" => "command",
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => "edit",
        "Read" | "Grep" | "Glob" | "LS" => "read",
        "WebFetch" | "WebSearch" => "fetch",
        t if t.starts_with("mcp__") => "mcp",
        _ => "tool",
    }
}

/// The plugins that decide in the project `project` (its privacy `private`): loaded, with a
/// program, `permissions.decide` not taken back, on for it. Their ids and names, in the order of
/// the list.
pub fn deciders(plugins: &Plugins, project: &str, private: bool) -> Vec<(String, String)> {
    plugins
        .list(crate::i18n::ui())
        .into_iter()
        .filter(|v| {
            v.state == State::Ready
                && v.manifest.as_ref().is_some_and(|m| {
                    m.run.is_some() && granted(m, &v.choices.revoked).permissions.decide
                })
                && plugins.reaches(&v.id, Some(project), private)
        })
        .map(|v| (v.id, v.name))
        .collect()
}

/// Starts the plugins that decide (with the app, a plugin turned on): their first decision is
/// not spent starting them.
pub fn warm(plugins: &Plugins, runtime: &Runtime) {
    for v in plugins.list(Lang::En) {
        let decides = v.state == State::Ready
            && v.choices.enabled
            && v.manifest.as_ref().is_some_and(|m| {
                m.run.is_some() && granted(m, &v.choices.revoked).permissions.decide
            });
        if decides {
            if let Err(e) = runtime.start(&v.id) {
                log::debug!("plugin {}: not started: {e}", v.id);
            }
        }
    }
}

/// The parameters of `permissions/decide` for `asked`; `subjects` holds its project, its ticket
/// and its agent.
pub fn params(asked: &Asked, mut subjects: Value) -> Value {
    subjects["request"] = json!({
        "id": asked.request_id,
        "kind": kind_of(&asked.tool),
        "tool": asked.tool,
        "input": asked.input,
    });
    subjects
}

/// What one plugin answered.
#[derive(Debug, Clone, PartialEq)]
enum Answer {
    Allow,
    Deny(Option<String>),
    Ask,
    /// No decision: no answer in time, an error, a crash, an answer that is none.
    None(String),
}

/// What a plugin's answer says, or why it says nothing.
fn read(answer: Result<Value, CallError>) -> Answer {
    match answer {
        Ok(v) => match v.get("decision").and_then(Value::as_str) {
            Some("allow") => Answer::Allow,
            Some("deny") => Answer::Deny(
                v.get("message")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|m| !m.is_empty())
                    .map(|m| m.chars().take(1000).collect()),
            ),
            Some("ask") => Answer::Ask,
            _ => Answer::None(format!("its answer is no decision: {v}")),
        },
        Err(CallError::Timeout) => Answer::None("no answer in time".into()),
        Err(e) => Answer::None(e.to_string()),
    }
}

/// Asks every plugin of `deciders` about `params` at once; writes each answer in its journal;
/// gives what they decided: a refusal wins, then an allowance, else the user is asked. `what`
/// says the request in the journal (the tool and its argument).
pub async fn decide(
    runtime: &Runtime,
    deciders: &[(String, String)],
    params: &Value,
    what: &str,
) -> Verdict {
    let asks = deciders.iter().map(|(id, _)| {
        let params = params.clone();
        async move {
            let cold = runtime.status(id).phase != PhaseName::Running;
            let within = if cold { COLD } else { timeout_of(METHOD) };
            let started = Instant::now();
            let answer = read(runtime.call_within(id, METHOD, params, within).await);
            let ms = started.elapsed().as_millis();
            let said = match &answer {
                Answer::Allow => format!("allowed {what} ({ms} ms)"),
                Answer::Deny(Some(m)) => format!("refused {what} ({ms} ms): {m}"),
                Answer::Deny(None) => format!("refused {what} ({ms} ms)"),
                Answer::Ask => format!("left {what} to the user ({ms} ms)"),
                Answer::None(why) => format!("did not decide {what}: {why}; the user is asked"),
            };
            runtime.journal_of(id).push(Kind::Decision, said);
            answer
        }
    });
    let answers = futures_util::future::join_all(asks).await;
    let mut allowed = None;
    for ((_, name), answer) in deciders.iter().zip(answers) {
        match answer {
            Answer::Deny(message) => {
                return Verdict::Deny {
                    by: name.clone(),
                    message,
                }
            }
            Answer::Allow if allowed.is_none() => allowed = Some(name.clone()),
            _ => {}
        }
    }
    match allowed {
        Some(by) => Verdict::Allow { by },
        None => Verdict::Ask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_says_its_kind_and_an_answer_is_read_or_left_to_the_user() {
        assert_eq!(kind_of("Bash"), "command");
        assert_eq!(kind_of("Write"), "edit");
        assert_eq!(kind_of("WebFetch"), "fetch");
        assert_eq!(kind_of("mcp__escouade__create_ticket"), "mcp");
        assert_eq!(kind_of("Permissions"), "tool");
        assert_eq!(read(Ok(json!({ "decision": "allow" }))), Answer::Allow);
        assert_eq!(
            read(Ok(json!({ "decision": "deny", "message": "  pas ici " }))),
            Answer::Deny(Some("pas ici".into()))
        );
        assert_eq!(
            read(Ok(json!({ "decision": "deny", "message": "" }))),
            Answer::Deny(None)
        );
        assert_eq!(read(Ok(json!({ "decision": "ask" }))), Answer::Ask);
        assert!(matches!(
            read(Ok(json!({ "decision": "yes" }))),
            Answer::None(_)
        ));
        assert!(matches!(read(Err(CallError::Timeout)), Answer::None(_)));
        assert!(matches!(read(Err(CallError::Crashed)), Answer::None(_)));
    }
}
