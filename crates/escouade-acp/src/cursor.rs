//! What is Cursor CLI's own beside ACP (its extensions `cursor/*`, read on 2026.10.22: ESC-24,
//! `tests/fixtures/engines/cursor-2026.10.22-27dcf5f.acp.json`), for a harness whose manifest names the extension
//! `cursor`: `cursor/ask_question` is a question's card, `cursor/create_plan` the card « Plan à approuver » (its tasks in
//! the plan too), `cursor/update_todos` the task list; a subagent (`task`, `subagent_spawned`) speaks under its own
//! session, shown under its `Task`; a tool of an MCP server is `rawInput.providerIdentifier` and `toolName`; Auto is
//! `default[]`; a turn Cursor refuses ends on its words (« Upgrade your plan to continue »).

use crate::texts::tr;
use crate::tools;
use serde_json::{json, Value};

/// Auto, the model Cursor picks itself, by the id Escouade keeps.
pub const AUTO: &str = "default";

/// Where the user sees the usage of a Cursor account.
pub const DASHBOARD: &str = "https://cursor.com/dashboard?tab=usage";

/// What Cursor takes for `model`, not listed: Auto's own id, else the id as it is.
pub fn full_id(model: &str) -> String {
    if model == AUTO {
        "default[]".into()
    } else {
        model.to_string()
    }
}

/// The option that sets the effort of `model` (its base id, `default` for Auto) among the models Cursor lists
/// (`cursor/list_available_models`: its `thought_level` option), if it has efforts.
pub fn effort_key(listed: &Value, model: &str) -> Option<String> {
    let base = model.split('[').next().unwrap_or(model);
    let m = listed["models"]
        .as_array()?
        .iter()
        .find(|m| m["value"] == base)?;
    let thought = m["configOptions"]
        .as_array()?
        .iter()
        .find(|c| c["category"] == "thought_level")?;
    let efforts = thought["options"].as_array().is_some_and(|o| !o.is_empty());
    efforts
        .then(|| thought["id"].as_str().map(str::to_string))
        .flatten()
}

/// The task list as Cursor last gave it (merged by id), and as Claude Code's `TodoWrite` gives it.
#[derive(Default)]
pub struct Todos {
    list: Vec<Value>,
}

impl Todos {
    /// The list after `todos` (`merge`: the tasks it names changed, the others kept), as `TodoWrite`'s.
    pub fn set(&mut self, todos: &Value, merge: bool) -> Value {
        if !merge {
            self.list.clear();
        }
        for t in todos.as_array().into_iter().flatten() {
            match self
                .list
                .iter_mut()
                .find(|k| k["id"] == t["id"] && !t["id"].is_null())
            {
                Some(k) => {
                    for (key, v) in t.as_object().into_iter().flatten() {
                        k[key] = v.clone();
                    }
                }
                None => self.list.push(t.clone()),
            }
        }
        Value::Array(
            self.list
                .iter()
                .filter(|t| t["status"] != "cancelled")
                .map(|t| {
                    let status = match t["status"].as_str() {
                        Some("in_progress") => "in_progress",
                        Some("completed") => "completed",
                        _ => "pending",
                    };
                    json!({ "content": t["content"], "status": status, "activeForm": t["content"] })
                })
                .collect(),
        )
    }
}

/// The calls that are Cursor's own cards or lists, not shown as calls.
pub fn own_tool(u: &Value) -> bool {
    matches!(
        u["rawInput"]["_toolName"].as_str(),
        Some("updateTodos" | "createPlan" | "askQuestion")
    )
}

/// A subagent started: its session, and the call it runs under.
pub fn spawned(u: &Value) -> (Option<String>, Option<String>) {
    (
        u["subagentSessionId"].as_str().map(str::to_string),
        u["_meta"]["cursor"]["toolCallId"]
            .as_str()
            .map(tools::call_id),
    )
}

/// Cursor's calls that ACP's kinds do not say: a tool of an MCP server, a subagent's `task`.
pub fn tool_of(u: &Value, cwd: &str) -> Option<(String, Value)> {
    let raw = &u["rawInput"];
    if let (Some(server), Some(tool)) =
        (raw["providerIdentifier"].as_str(), raw["toolName"].as_str())
    {
        return Some((format!("mcp__{server}__{tool}"), raw["args"].clone()));
    }
    let title = u["title"].as_str().unwrap_or_default();
    match raw["_toolName"].as_str() {
        Some("task") => {
            let mut input = json!({
                "description": raw["description"].as_str().unwrap_or(title.trim_start_matches("Task: ")),
                "prompt": raw["prompt"], "subagent_type": raw["subagentType"],
            });
            if let Some(o) = input.as_object_mut() {
                o.retain(|_, v| !v.is_null());
            }
            Some(("Task".into(), input))
        }
        Some(own)
            if !matches!(
                u["kind"].as_str(),
                Some("execute" | "edit" | "delete" | "read" | "search" | "fetch")
            ) =>
        {
            let mut input = raw.clone();
            if let Some(o) = input.as_object_mut() {
                o.remove("_toolName");
            }
            let _ = cwd;
            Some((own.to_string(), input))
        }
        _ => None,
    }
}

/// The MCP server and tool of a call, from its name (`mcp__<server>__<tool>`) or its input.
pub fn mcp_of(tc: &Value, name: &str) -> Option<(String, String)> {
    if let Some(rest) = name.strip_prefix("mcp__") {
        let (server, tool) = rest.split_once("__")?;
        return Some((server.to_string(), tool.to_string()));
    }
    let raw = &tc["rawInput"];
    Some((
        raw["providerIdentifier"].as_str()?.to_string(),
        raw["toolName"].as_str()?.to_string(),
    ))
}

/// Escouade's mode after Cursor's `currentModeId`: `plan` in its mode Plan; out of it, « Demander » if Escouade's
/// was Plan; else unchanged.
pub fn mode_of(acp: &str, mode: &str) -> Option<String> {
    match acp {
        "plan" => Some("plan".into()),
        _ if mode == "plan" => Some("default".into()),
        _ => None,
    }
}

/// The questions of `cursor/ask_question`, as the card shows them.
pub fn questions(p: &Value) -> Value {
    let header = p["title"].as_str().unwrap_or_default();
    Value::Array(
        p["questions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|q| {
                let options: Vec<Value> = q["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|o| json!({ "label": o["label"], "description": "" }))
                    .collect();
                json!({
                    "question": q["prompt"], "header": header,
                    "multiSelect": q["allowMultiple"] == true, "options": options,
                })
            })
            .collect(),
    )
}

/// The answer to `cursor/ask_question` (its `params`) from a card's `answers` (by question, its choices' labels joined
/// by « , »): each by the ids of its choices; an answer that names none of them (written by the user) skips the
/// questions, with what was written as the reason the model reads.
pub fn question_outcome(params: &Value, answers: &Value) -> Value {
    let mut picked = Vec::new();
    let mut written = Vec::new();
    for q in params["questions"].as_array().into_iter().flatten() {
        let text = q["prompt"].as_str().unwrap_or_default();
        let answer = answers[text].as_str().unwrap_or_default().trim();
        let options = q["options"].as_array().cloned().unwrap_or_default();
        let id_of = |label: &str| {
            options
                .iter()
                .find(|o| o["label"].as_str() == Some(label))
                .and_then(|o| o["id"].as_str())
                .map(str::to_string)
        };
        let ids: Option<Vec<String>> = if answer.is_empty() {
            Some(Vec::new())
        } else {
            id_of(answer).map(|id| vec![id]).or_else(|| {
                answer
                    .split(", ")
                    .map(id_of)
                    .collect::<Option<Vec<String>>>()
            })
        };
        match ids {
            Some(ids) => picked.push(json!({ "questionId": q["id"], "selectedOptionIds": ids })),
            None => written.push(format!("{text} {answer}")),
        }
    }
    if written.is_empty() {
        json!({ "outcome": "answered", "answers": picked })
    } else {
        json!({ "outcome": "skipped", "reason": written.join("\n") })
    }
}

/// The text of a plan (`cursor/create_plan`): its name, overview and plan.
pub fn plan_text(p: &Value) -> String {
    [
        p["name"].as_str().map(|n| format!("# {n}")),
        p["overview"].as_str().map(str::to_string),
        p["plan"].as_str().map(str::to_string),
    ]
    .into_iter()
    .flatten()
    .filter(|t| !t.trim().is_empty() && t != "# ")
    .collect::<Vec<_>>()
    .join("\n\n")
}

/// The answer to a plan, and the mode its work starts in: approved, in « Demander » (« Pour cet agent »: « Accepter
/// les modifications »); sent back, the planning goes on.
pub fn plan_answer(
    allow: bool,
    always: bool,
    message: Option<&str>,
) -> (Value, Option<&'static str>) {
    match (allow, always) {
        (true, true) => (json!({ "outcome": "accepted" }), Some("acceptEdits")),
        (true, false) => (json!({ "outcome": "accepted" }), Some("default")),
        _ => (json!({ "outcome": "rejected", "reason": message }), None),
    }
}

/// What Cursor answers in place of a turn it does not run: a text as its last words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Upgrade,
    Payment,
    SignIn,
    Unauthenticated,
}

/// The refusal a turn's text `text` ends with, if any.
pub fn refusal(text: &str) -> Option<Refusal> {
    let last = text.trim().rsplit("\n\n").next().unwrap_or_default().trim();
    match last {
        "Upgrade your plan to continue" => Some(Refusal::Upgrade),
        "Add a payment method to continue" => Some(Refusal::Payment),
        "Please sign in to continue" => Some(Refusal::SignIn),
        t if t.starts_with("Error: [unauthenticated]") => Some(Refusal::Unauthenticated),
        _ => None,
    }
}

/// Whether the refusal is the account's usage limit: its turn ends « limité ».
pub fn at_limit(refusal: Refusal) -> bool {
    matches!(refusal, Refusal::Upgrade | Refusal::Payment)
}

/// What the card says of a turn Cursor refused, the agent on `model`, its account by its key or not (`by_key`).
pub fn refusal_error(lang: &str, refusal: Refusal, model: &str, by_key: bool) -> String {
    let url = DASHBOARD;
    match refusal {
        Refusal::Upgrade if model == AUTO || model.is_empty() => tr(
            lang,
            format!("Limite d’usage de ce compte Cursor atteinte (« Upgrade your plan to continue ») : l’usage inclus du mois est épuisé jusqu’à la remise à zéro de son offre. Son usage : {url}"),
            format!("This Cursor account’s usage limit is reached (“Upgrade your plan to continue”): the month’s included usage is spent until its plan resets. Its usage: {url}"),
        ),
        Refusal::Upgrade => tr(
            lang,
            format!("Cursor refuse le modèle {model} sur ce compte (« Upgrade your plan to continue ») : il n’est pas dans l’offre de ce compte Cursor, ou son usage inclus du mois est épuisé. Choisis Auto dans le menu Modèle ; son usage : {url}"),
            format!("Cursor refuses the model {model} on this account (“Upgrade your plan to continue”): it isn’t in this Cursor account’s plan, or its included usage of the month is spent. Choose Auto in the Model menu; its usage: {url}"),
        ),
        Refusal::Payment => tr(
            lang,
            format!("Limite d’usage de ce compte Cursor atteinte : la suite passe par l’usage à la demande, qui demande un moyen de paiement (« Add a payment method to continue »). Son usage : {url}"),
            format!("This Cursor account’s usage limit is reached: what comes next is on-demand usage, which needs a payment method (“Add a payment method to continue”). Its usage: {url}"),
        ),
        Refusal::SignIn | Refusal::Unauthenticated if by_key => tr(
            lang,
            "Cursor a refusé la clé d’API de ce compte : donne-la de nouveau dans Réglages › Comptes, « Changer la clé… ».".into(),
            "Cursor refused this account’s API key: give it again in Settings › Accounts, “Change the key…”.".into(),
        ),
        Refusal::SignIn | Refusal::Unauthenticated => tr(
            lang,
            "Cursor CLI n’est pas connecté sur cette machine (« Please sign in to continue ») : lance agent login dans un terminal.".into(),
            "Cursor CLI isn’t signed in on this machine (“Please sign in to continue”): run agent login in a terminal.".into(),
        ),
    }
}

/// What a write of Cursor that changed nothing says (its call ends `completed` with no diff).
pub fn write_not_done(lang: &str) -> String {
    tr(
        lang,
        "Cursor n’a rien écrit : l’écriture a été refusée, par un hook (ceux de Claude Code tournent aussi sous Cursor) ou par Cursor lui-même, qui n’en donne pas la raison.".into(),
        "Cursor wrote nothing: the write was refused, by a hook (those of Claude Code run under Cursor too) or by Cursor itself, which doesn’t give the reason.".into(),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn the_effort_of_a_model_goes_by_the_option_its_list_gives_it() {
        let listed = json!({ "models": [
            { "value": "claude-opus-5-5", "configOptions": [
                { "id": "effort", "category": "thought_level", "options": [{ "value": "low" }, { "value": "high" }] },
            ] },
            { "value": "default", "configOptions": [] },
        ] });
        assert_eq!(
            super::effort_key(&listed, "claude-opus-5-5").as_deref(),
            Some("effort")
        );
        assert_eq!(
            super::effort_key(&listed, "claude-opus-5-5[context=300k]").as_deref(),
            Some("effort")
        );
        assert_eq!(super::effort_key(&listed, "default"), None);
        assert_eq!(
            super::effort_key(&serde_json::Value::Null, "claude-opus-5-5"),
            None
        );
    }
}
