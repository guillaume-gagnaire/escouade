//! What Escouade knows of Copilot CLI beside its conversation: the arguments of its server, its modes, its models and
//! their prices, the month's credits, the answers its requests take, and the lines said in the app's language. All
//! pure, from what its real runs showed (Copilot CLI 1.0.98 on a Copilot Free account, docs/PROTOCOL.md, « Copilot
//! SDK »). The core's engine `Copilot` reads the same ones for what stays in it: an account's quota and models.

use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// The version of the protocol Escouade speaks (`connect`).
pub const PROTOCOL: u64 = 3;

/// The variables a token of GitHub reaches Copilot in, taken before its own sign-in.
pub const INHERITED_TOKENS: [&str; 3] = ["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"];

/// The variables whose values Copilot strips from its commands and MCP servers (`--secret-env-vars`): the tokens it
/// may be given. Not `ESCOUADE_MCP_TOKEN`: listed, Copilot no longer puts it in the header of Escouade's entry
/// (`${ESCOUADE_MCP_TOKEN}`, which then reaches the server without it: tried on 1.0.98). It is the agent's own token,
/// for the server it is given, valid while its process lives.
pub const SECRETS: &str = "GH_TOKEN,GITHUB_TOKEN,COPILOT_GITHUB_TOKEN";

/// The variable the token of Escouade's server reaches Copilot in: its entry's header names it
/// (`${ESCOUADE_MCP_TOKEN}`, taken from the environment, tried on 1.0.98), so that neither its file nor its command
/// line holds the token.
pub const TOKEN_VAR: &str = "ESCOUADE_MCP_TOKEN";

/// The name of Escouade's MCP server.
pub const SERVER: &str = escouade_acp::SERVER;

/// The models of its menu until a process of the account has listed them: those its real run on a Free account ran
/// (`gpt-6-luna` chosen by `auto`, `gpt-5-mini` taken by `session.model.switchTo`). Never `auto`, which picks a model
/// itself.
pub const MODELS: [(&str, &str); 2] = [("gpt-6-luna", "GPT-6 Luna"), ("gpt-5-mini", "GPT-5 mini")];

/// `fr` or `en`, as the app speaks.
pub(crate) fn said(lang: &str, fr: String, en: String) -> String {
    if lang == "fr" {
        fr
    } else {
        en
    }
}

/// The arguments of its server: no update of its own, no built-in MCP server (GitHub's, with the account), no
/// temporary folder open without a question, and the tokens kept from its commands.
pub fn server_args() -> Vec<String> {
    [
        "--server",
        "--stdio",
        "--no-auto-update",
        "--disable-builtin-mcps",
        "--disallow-temp-dir",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain(std::iter::once(format!("--secret-env-vars={SECRETS}")))
    .collect()
}

/// Whether `model` takes a reasoning effort: never « auto », which picks the model itself (Copilot CLI 1.0.98 refuses
/// the session: « Reasoning effort is not supported when using the "auto" model », seen at ESC-52 with the account of
/// `copilot login`, which lists it first).
pub fn takes_effort(model: &str) -> bool {
    !model.eq_ignore_ascii_case("auto")
}

/// The permission mode of Copilot for Escouade's `mode` (`session.permissions.setMode`): `allow-all` without
/// permissions, else `manual`, Escouade's rules answering the safe requests themselves (`translate`). Never `assisted`:
/// experimental, and its judge failed on the real run.
pub fn permission_mode(mode: &str) -> &'static str {
    if mode == "bypassPermissions" {
        "allow-all"
    } else {
        "manual"
    }
}

/// The mode of Copilot's agent for Escouade's `mode` (`session.mode.set`): `plan` to plan, `autopilot` for « Auto » (it
/// goes on by itself up to the end of its task), else `interactive`.
pub fn agent_mode(mode: &str) -> &'static str {
    match mode {
        "plan" => "plan",
        "auto" => "autopilot",
        _ => "interactive",
    }
}

/// Whether `auth.getStatus` says Copilot is signed in as the account: an account of its own never on the GitHub CLI's
/// (`gh-cli`), which Copilot takes by itself without a sign-in of its own and is the same for every account.
pub fn signed_in(auth: &Value, own_home: bool) -> bool {
    auth["isAuthenticated"] == true && !(own_home && auth["authType"] == "gh-cli")
}

/// The models of the account as the menu takes them (the core's `ToldModel`): those of `models.list` (`listed`), else of
/// `session.model.list` (`of_session.list`), each with its efforts, whether it reads images and its multiplier; on a
/// Free account both are empty (tried), and the menu keeps its declared models, each with its tier of price
/// (`modelPriceCategories`). A model its policy disables is left out.
pub fn catalog(listed: Option<&Value>, of_session: Option<&Value>) -> Vec<Value> {
    let tiers: Vec<(String, String)> = of_session
        .and_then(|s| s["modelPriceCategories"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|c| {
            Some((
                c["id"].as_str()?.to_string(),
                c["priceCategory"].as_str()?.to_string(),
            ))
        })
        .collect();
    let tier = |id: &str| tiers.iter().find(|(m, _)| m == id).map(|(_, t)| t.clone());
    let list = listed
        .and_then(|l| l["models"].as_array())
        .filter(|a| !a.is_empty())
        .or_else(|| of_session.and_then(|s| s["list"].as_array()))
        .cloned()
        .unwrap_or_default();
    let mut models: Vec<Value> = list
        .iter()
        .filter(|m| m["policy"]["state"] != "disabled")
        .filter_map(|m| {
            let id = m["id"].as_str()?;
            let multiplier = match &m["billing"]["multiplier"] {
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            };
            let efforts: Vec<String> = m["supportedReasoningEfforts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|e| e.as_str().map(str::to_string))
                .collect();
            let price = m["modelPickerPriceCategory"]
                .as_str()
                .map(str::to_string)
                .or_else(|| tier(id));
            Some(json!({
                "id": id,
                "label": m["name"].as_str().unwrap_or(id),
                "efforts": efforts,
                "defaultEffort": m["defaultReasoningEffort"].as_str(),
                "images": m["capabilities"]["supports"]["vision"] == true,
                "default": false,
                "price": price,
                "multiplier": multiplier,
            }))
        })
        .collect();
    if models.is_empty() {
        models = MODELS
            .iter()
            .map(|&(id, label)| {
                json!({
                    "id": id, "label": label, "efforts": [], "defaultEffort": null, "images": false,
                    "default": false, "price": tier(id),
                })
            })
            .collect();
    }
    if let Some(first) = models.first_mut() {
        first["default"] = json!(true);
    }
    models
}

/// « Projet privé » (ESC-140): whether the permission `r` Copilot asks (its `permissionRequest`) names a path of `fence`
/// (as Copilot names them): the path read or written, those its command may touch, or its command's text. Copilot asks
/// for any path out of the agent's folder but in « Tout accepter » (`allow-all`), where nothing is asked, and a command
/// can reach a path it does not name: a guard of Escouade's, not a barrier.
pub fn fenced(r: &Value, fence: &[String]) -> bool {
    if fence.is_empty() {
        return false;
    }
    let mut texts: Vec<&str> = ["path", "resolvedPath", "fileName", "fullCommandText"]
        .iter()
        .filter_map(|k| r[*k].as_str())
        .collect();
    for k in ["possiblePaths", "resolvedPaths"] {
        texts.extend(
            r[k].as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str),
        );
    }
    texts
        .iter()
        .any(|t| fence.iter().any(|p| escouade_acp::names(t, p)))
}

/// What the agent is told of a permission `fenced` refused.
pub fn fence_refusal(lang: &str) -> String {
    said(
        lang,
        "Refusé par Escouade : ce chemin est celui d’un autre projet, et l’un des deux est privé."
            .into(),
        "Refused by Escouade: this path is another project’s, and one of the two is private."
            .into(),
    )
}

/// The answer « Pour cette session » to the permission `data` (`permission.requested`): what it is remembered for, by
/// its kind. Never `approve-permanently` nor `approve-for-location`, which write a rule of their own in Copilot's
/// config (decided by the user on 2026-10-18).
pub fn session_approval(data: &Value) -> Value {
    let r = &data["permissionRequest"];
    let approval = match r["kind"].as_str().unwrap_or_default() {
        "shell" => {
            let ids: Vec<Value> = r["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c["identifier"].as_str().map(|s| json!(s)))
                .collect();
            json!({ "kind": "commands", "commandIdentifiers": ids })
        }
        "read" => json!({ "kind": "read" }),
        "write" => json!({ "kind": "write" }),
        "mcp" => json!({ "kind": "mcp", "serverName": r["serverName"], "toolName": r["toolName"] }),
        "url" => {
            let host = r["url"].as_str().and_then(host_of);
            return match host {
                Some(domain) => json!({ "kind": "approve-for-session", "domain": domain }),
                None => json!({ "kind": "approve-once" }),
            };
        }
        _ => return json!({ "kind": "approve-once" }),
    };
    json!({ "kind": "approve-for-session", "approval": approval })
}

/// The host of an address, in lower case. An address it cannot tell, a name beyond ASCII among them, has none: its
/// permission goes for once. By hand: the crate `url` would bring macros out of the closed list (`rust_native.json`).
pub(crate) fn host_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let scheme_ok = scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    let end = rest.find(['/', '?', '#', '\\']).unwrap_or(rest.len());
    let authority = rest[..end].rsplit('@').next()?;
    let host = match authority.strip_prefix('[') {
        Some(v6) => format!("[{}]", v6.split_once(']')?.0),
        None => authority.split(':').next()?.to_string(),
    };
    let host_ok = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '[' | ']' | ':'));
    (scheme_ok && host_ok).then(|| host.to_ascii_lowercase())
}

/// The values of a form (`elicitation.requested`) from its card: each field's answer (by the question the card made of
/// it, `input.questions[].id` its field), as its schema types it.
pub fn form_content(schema: &Value, input: &Value) -> Value {
    let props = &schema["properties"];
    let mut out = Map::new();
    for q in input["questions"].as_array().into_iter().flatten() {
        let (Some(field), Some(text)) = (q["id"].as_str(), q["question"].as_str()) else {
            continue;
        };
        let Some(answer) = input["answers"][text].as_str().map(str::trim) else {
            continue;
        };
        if answer.is_empty() {
            continue;
        }
        let value = match props[field]["type"].as_str() {
            Some("number") => answer
                .replace(',', ".")
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number),
            Some("integer") => answer.parse::<i64>().ok().map(Value::from),
            Some("boolean") => Some(json!(is_yes(answer))),
            Some("array") => Some(json!(answer
                .split(", ")
                .map(str::to_string)
                .collect::<Vec<_>>())),
            _ => None,
        }
        .unwrap_or_else(|| json!(answer));
        out.insert(field.to_string(), value);
    }
    Value::Object(out)
}

/// Whether the answer to a field yes or no says yes, in either language.
pub fn is_yes(answer: &str) -> bool {
    ["fr", "en"].iter().any(|l| answer == yes(l)) || answer.eq_ignore_ascii_case("true")
}

pub fn yes(lang: &str) -> String {
    said(lang, "Oui".into(), "Yes".into())
}

pub fn no(lang: &str) -> String {
    said(lang, "Non".into(), "No".into())
}

/// Refused: the protocol of its server is not the one Escouade speaks.
pub fn protocol_refused(lang: &str, version: &Value) -> String {
    let ours = PROTOCOL;
    said(
        lang,
        format!("Copilot CLI parle le protocole {version}, Escouade le protocole {ours} : mets Copilot CLI à jour (npm install -g @github/copilot@latest)."),
        format!("Copilot CLI speaks protocol {version}, Escouade protocol {ours}: update Copilot CLI (npm install -g @github/copilot@latest)."),
    )
}

/// Refused: no account is signed in to Copilot (an account of its own on the GitHub CLI's is not).
pub fn not_signed_in(lang: &str) -> String {
    said(
        lang,
        "Copilot CLI n’est connecté à aucun compte GitHub sur ce compte : connecte-le dans Réglages › Comptes (« Se connecter… », copilot login).".into(),
        "Copilot CLI isn’t signed in to any GitHub account on this account: sign it in in Settings › Accounts (“Sign in…”, copilot login).".into(),
    )
}

/// No session opened for the agent (`session.create` refused): what Copilot CLI said.
pub fn session_refused(lang: &str, e: &str) -> String {
    said(
        lang,
        format!("Copilot CLI n’a pas ouvert la session de l’agent : {e}"),
        format!("Copilot CLI didn’t open the agent’s session: {e}"),
    )
}

/// The month's credits, read: how much is used (0 to 100, one decimal) and when they are given again (epoch ms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Credits {
    pub pct: f64,
    pub resets_at: i64,
}

impl Credits {
    /// As a meter of `usage::parse_windows` (`QuotaMeter`).
    pub fn meter(self) -> Value {
        json!({ "minutes": null, "pct": self.pct, "resetsAt": self.resets_at })
    }
}

/// The month's credits as Copilot's quota says them (`quotaSnapshots` of each `assistant.usage` and of
/// `account.getQuota`, `snapshots` of `session.quota.get`), read at `now` (epoch ms): those of `chat` (the AI credits,
/// with a billing by tokens), else of `premium_interactions`; none when the plan has no limit or none is told.
pub fn credits(snapshots: &Value, now: i64) -> Option<Credits> {
    let told = |k: &str| {
        let s = snapshots.get(k).filter(|s| s.is_object())?;
        (s["remainingPercentage"].is_number() && s["entitlementRequests"].as_f64() != Some(0.0))
            .then_some(s)
    };
    let s = told("chat").or_else(|| told("premium_interactions"))?;
    if s["isUnlimitedEntitlement"] == true {
        return None;
    }
    let remaining = s["remainingPercentage"].as_f64()?.clamp(0.0, 100.0);
    let told_reset = s["resetDateEpochMs"].as_i64().or_else(|| {
        s["resetDate"]
            .as_str()
            .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
            .map(|d| d.timestamp_millis())
    });
    // `account.getQuota` says the time of its reading (tried): the credits are given again on the first of the month
    // (UTC), as the turns' snapshots say.
    let resets_at = told_reset
        .filter(|&t| t > now + 60_000)
        .unwrap_or_else(|| next_month(now));
    Some(Credits {
        pct: ((100.0 - remaining) * 10.0).round() / 10.0,
        resets_at,
    })
}

/// The first of the month after `now` (epoch ms), at midnight UTC.
pub fn next_month(now: i64) -> i64 {
    use chrono::{Datelike, TimeZone};
    let d = chrono::Utc
        .timestamp_millis_opt(now)
        .single()
        .unwrap_or_else(chrono::Utc::now);
    let (y, m) = if d.month() == 12 {
        (d.year() + 1, 1)
    } else {
        (d.year(), d.month() + 1)
    };
    chrono::Utc
        .with_ymd_and_hms(y, m, 1, 0, 0, 0)
        .single()
        .map_or(now, |t| t.timestamp_millis())
}

/// The question of the card of a session's ceiling of credits reached (`session_limits_exhausted.requested`:
/// `usedAiCredits` of `maxAiCredits`).
pub fn limit_question(lang: &str, data: &Value) -> String {
    let n = |k: &str| {
        let v = data[k].as_f64().unwrap_or(0.0);
        let s = format!("{v:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let (used, max) = (n("usedAiCredits"), n("maxAiCredits"));
    said(
        lang,
        format!("Le plafond de crédits d’IA de la session est atteint ({used} sur {max}). Que faire de l’appel en attente ?"),
        format!("The session’s ceiling of AI credits is reached ({used} of {max}). What about the call waiting?"),
    )
}

/// The choices of that card, each with the answer it gives Copilot (`session.ui.handlePendingSessionLimitsExhausted`):
/// the ceiling raised by as much again, lifted, or the call given up.
pub fn limit_choices(lang: &str, data: &Value) -> Vec<(String, Value)> {
    let max = data["maxAiCredits"].as_f64().unwrap_or(0.0);
    let mut out = Vec::new();
    if max > 0.0 {
        out.push((
            said(
                lang,
                format!("Ajouter {max} crédits"),
                format!("Add {max} credits"),
            ),
            json!({ "action": "add", "additionalAiCredits": max }),
        ));
    }
    out.push((
        said(lang, "Lever le plafond".into(), "Lift the ceiling".into()),
        json!({ "action": "unset" }),
    ));
    out.push((
        said(lang, "Arrêter là".into(), "Stop here".into()),
        json!({ "action": "cancel" }),
    ));
    out
}

/// The answer to that card: the choice `answer` names, in either language; none known, the call is given up.
pub fn limit_answer(data: &Value, answer: &str) -> Value {
    ["fr", "en"]
        .iter()
        .flat_map(|l| limit_choices(l, data))
        .find(|(label, _)| label == answer)
        .map_or_else(|| json!({ "action": "cancel" }), |(_, r)| r)
}

/// The prices of the account's models, by their id (`billing.tokenPrices` of `models.list`: AI credits per batch of
/// tokens), for the calls whose cost Copilot does not tell.
pub fn prices(listed: &Value) -> HashMap<String, Value> {
    listed["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let p = m["billing"]["tokenPrices"].clone();
            p.is_object()
                .then(|| Some((m["id"].as_str()?.to_string(), p)))
                .flatten()
        })
        .collect()
}

/// What one call of the model cost, in nano-AIU (`assistant.usage`): Copilot's (`copilotUsage.totalNanoAiu`), else the
/// sum of its details (`tokenDetails`: tokens by batch), else its tokens at its model's prices (`prices`). A credit is
/// 10⁹ nano-AIU, checked at point 0.
pub fn call_nano(usage: &Value, prices: &HashMap<String, Value>) -> u64 {
    let told = &usage["copilotUsage"];
    if let Some(n) = told["totalNanoAiu"].as_u64() {
        return n;
    }
    let by_batch = |count: f64, size: f64, per_batch: f64| {
        if size > 0.0 {
            count / size * per_batch
        } else {
            0.0
        }
    };
    if let Some(details) = told["tokenDetails"].as_array().filter(|d| !d.is_empty()) {
        let f = |d: &Value, k: &str| d[k].as_f64().unwrap_or(0.0);
        return details
            .iter()
            .map(|d| by_batch(f(d, "tokenCount"), f(d, "batchSize"), f(d, "costPerBatch")))
            .sum::<f64>()
            .round() as u64;
    }
    let Some(p) = usage["model"].as_str().and_then(|m| prices.get(m)) else {
        return 0;
    };
    let n = |k: &str| usage[k].as_f64().unwrap_or(0.0);
    let price = |k: &str| p[k].as_f64().unwrap_or(0.0);
    let size = p["batchSize"].as_f64().unwrap_or(1_000_000.0);
    let (read, write) = (n("cacheReadTokens"), n("cacheWriteTokens"));
    let fresh = (n("inputTokens") - read - write).max(0.0);
    let credits = by_batch(fresh, size, price("inputPrice"))
        + by_batch(read, size, price("cacheReadPrice"))
        + by_batch(write, size, price("cacheWritePrice"))
        + by_batch(n("outputTokens"), size, price("outputPrice"));
    (credits * 1e9).round() as u64
}
