//! What the core keeps of Codex's words, for what stays in it (`terminal/launch` is not served yet): the launch of its
//! interface, an account's quota and models. The agents' own are its plugin's (`plugins/codex/src/words.rs`), which it does
//! not link (docs/PLUGINS.md §8.2).

use serde_json::{json, Value};

#[cfg(test)]
/// What the next turn is started with (Codex takes them at `turn/start`).
#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub model: String,
    pub effort: String,
    /// Escouade's mode (`default`, `plan`…), turned into Codex's by `mode_params`.
    pub mode: String,
    /// The paths its sandbox refuses (« Projet privé »), as Codex names them: a permissions
    /// profile of Escouade's in place of the sandbox of its mode (`fence_profile`).
    pub fence: Vec<String>,
    /// A discussion of the Docs tab (ESC-261): read only whatever its mode (`Settings::sandboxed`).
    pub read_only: bool,
}

#[cfg(test)]
impl Settings {
    #[cfg(test)]
    /// The mode its sandbox is of: Plan's, read only, for a discussion.
    pub fn sandboxed(&self) -> &str {
        sandboxed(&self.mode, self.read_only)
    }
}

/// The mode whose sandbox an agent in `mode` has: Plan's, read only, when it is `read_only` (a
/// discussion, ESC-261), which writes nothing but through Escouade's tools; its own otherwise.
pub fn sandboxed(mode: &str, read_only: bool) -> &str {
    if read_only {
        "plan"
    } else {
        mode
    }
}

/// « Projet privé » (ESC-140): Codex's permissions profiles, its sandbox with the paths of a fence
/// refused (`deny`: neither read nor written by its commands, tried on 0.163.0 under Windows).
/// Its sandbox enforces them: none in « Sans permissions », which has none.
pub const FENCE_PROFILE: &str = "escouade-fence";
/// The same, read only: the mode Plan.
pub const FENCE_PLAN_PROFILE: &str = "escouade-fence-plan";

/// The profile of a fenced agent in `mode`; None in « Sans permissions » (no sandbox).
pub fn fence_profile(mode: &str) -> Option<&'static str> {
    match mode {
        "bypassPermissions" => None,
        "plan" => Some(FENCE_PLAN_PROFILE),
        _ => Some(FENCE_PROFILE),
    }
}

/// The options (`-c`) of a process of Codex that declare both profiles with `paths` refused, and
/// `default` its profile (`default_permissions`, which Codex wants once it has profiles). Declared
/// for the process, not a thread's `config`: Codex reads its config again for a turn that names
/// its profile, and a thread's is not in it (« failed to load workspace requirements », tried on
/// 0.163.0). The plan's holds the paths, read only; the other is it with its folder written, as
/// `workspace-write` is (the paths once: a `.cmd` launcher takes 8,191 characters). A path of
/// Windows with `/`: Codex reads it so, its `\` an escape in a string of its config.
pub fn fence_args(paths: &[String], default: &str) -> Vec<String> {
    let deny: Vec<String> = paths
        .iter()
        .map(|p| {
            let p = if p.starts_with('/') {
                p.clone()
            } else {
                p.replace('\\', "/")
            };
            format!("{}=\"deny\"", toml_string(&p))
        })
        .collect();
    vec![
        "-c".into(),
        format!("default_permissions={}", toml_string(default)),
        "-c".into(),
        format!(
            "permissions.{FENCE_PLAN_PROFILE}={{extends=\":read-only\", filesystem={{{}}}}}",
            deny.join(", ")
        ),
        "-c".into(),
        format!(
            "permissions.{FENCE_PROFILE}={{extends=\"{FENCE_PLAN_PROFILE}\", filesystem={{\":workspace_roots\"=\"write\", \":tmpdir\"=\"write\"}}}}"
        ),
    ]
}

/// `s` as a string of TOML (JSON's escapes are TOML's).
pub fn toml_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// The sandbox of a thread in `mode` (`thread/start`): read only to plan, none without
/// permissions, else the agent's folder.
pub fn sandbox_mode(mode: &str) -> &'static str {
    match mode {
        "plan" => "read-only",
        "bypassPermissions" => "danger-full-access",
        _ => "workspace-write",
    }
}

#[cfg(test)]
/// Escouade's modes in Codex (docs/TODO-1.8.md, « Composer : modèles, effort, modes »): its approval
/// policy, its sandbox, who reviews the approvals, and its collaboration mode (the mode Plan).
pub fn mode_params(s: &Settings, _cwd: &str) -> Value {
    let (approval, sandbox, reviewer) = match s.sandboxed() {
        "plan" => ("on-request", json!({ "type": "readOnly" }), "user"),
        "bypassPermissions" => ("never", json!({ "type": "dangerFullAccess" }), "user"),
        "auto" => ("on-request", workspace_write(), "auto_review"),
        // « Accepter les modifications »: the sandbox lets it write in its folder, and Escouade
        // accepts the git commands of its worktree (`translate`).
        _ => ("on-request", workspace_write(), "user"),
    };
    let profile = (!s.fence.is_empty())
        .then(|| fence_profile(s.sandboxed()))
        .flatten();
    let collaboration = if s.mode == "plan" { "plan" } else { "default" };
    let mut settings = json!({ "model": s.model, "developer_instructions": null });
    if !s.effort.is_empty() {
        settings["reasoning_effort"] = json!(s.effort);
    }
    let mut p = json!({
        "approvalPolicy": approval,
        "approvalsReviewer": reviewer,
        "sandboxPolicy": sandbox,
        "collaborationMode": { "mode": collaboration, "settings": settings },
    });
    // Fenced: its profile, which Codex takes in place of a sandbox policy (not both).
    if let (Some(profile), Some(o)) = (profile, p.as_object_mut()) {
        o.remove("sandboxPolicy");
        o.insert("permissions".into(), json!(profile));
    }
    p
}

#[cfg(test)]
fn workspace_write() -> Value {
    json!({ "type": "workspaceWrite", "writableRoots": [], "networkAccess": false })
}

#[cfg(test)]
/// The mode of Escouade a thread's settings say (`thread/settings/updated`), given the agent's
/// `current` one: « Demander » and « Accepter les modifications » are the same for Codex.
pub fn mode_of(settings: &Value, current: &str) -> Option<&'static str> {
    let collab = settings["collaborationMode"]["mode"].as_str();
    let approval = settings["approvalPolicy"].as_str()?;
    let sandbox = settings["sandboxPolicy"]["type"].as_str();
    Some(if collab == Some("plan") {
        "plan"
    } else if approval == "never" && sandbox == Some("dangerFullAccess") {
        "bypassPermissions"
    } else if settings["approvalsReviewer"] == "auto_review" {
        "auto"
    } else if current == "acceptEdits" {
        "acceptEdits"
    } else {
        "default"
    })
}

#[cfg(test)]
pub fn is_plan_approval(request_id: &str) -> bool {
    request_id.starts_with("plan:")
}

/// The name of Escouade's server, as its agents know it.
const SERVER: &str = escouade_acp::SERVER;

/// The variable of Escouade's token in the process of Codex.
pub const TOKEN_VAR: &str = "ESCOUADE_MCP_TOKEN";

/// Escouade's server given to its process, by `-c` alone (never by the `config` of `thread/start`,
/// which would drop them, tried): its address and the variable of its token, which is in the
/// process's environment.
pub fn granted_args(url: &str) -> Vec<String> {
    vec![
        "-c".into(),
        format!("mcp_servers.{SERVER}.url={url}"),
        "-c".into(),
        format!("mcp_servers.{SERVER}.bearer_token_env_var={TOKEN_VAR}"),
    ]
}

/// Escouade's server refused: an entry of that name turned off, the user's own included.
pub fn denied_args() -> Vec<String> {
    vec!["-c".into(), format!("mcp_servers.{SERVER}.enabled=false")]
}

/// Whether a `config.toml` of Codex declares a server named `escouade` (`[mcp_servers.escouade]`,
/// one of its subtables, or a key of `[mcp_servers]`).
pub fn declares_escouade(text: &str) -> bool {
    let mut in_servers = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            let table: String = line
                .chars()
                .filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
                .collect();
            if table.starts_with(&format!("[mcp_servers.{SERVER}")) {
                return true;
            }
            in_servers = table == "[mcp_servers]";
            continue;
        }
        let key = line
            .split(['=', '.'])
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches(['"', '\'']);
        if (in_servers && key == SERVER) || line.starts_with(&format!("mcp_servers.{SERVER}")) {
            return true;
        }
    }
    false
}

/// The models of `model/list`, as `ToldModel`s (the core's, in JSON): the composer's menu.
pub fn catalog(list: &Value) -> Option<Value> {
    let models: Vec<Value> = list["data"]
        .as_array()?
        .iter()
        .filter(|m| m["hidden"] != true)
        .filter_map(|m| {
            let id = m["model"].as_str().or_else(|| m["id"].as_str())?;
            let efforts: Vec<&str> = m["supportedReasoningEfforts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|e| e["reasoningEffort"].as_str())
                .collect();
            Some(json!({
                "id": id,
                "label": m["displayName"].as_str().unwrap_or(id),
                "efforts": efforts,
                "defaultEffort": m["defaultReasoningEffort"].as_str(),
                "images": m["inputModalities"].as_array().is_some_and(|a| a.iter().any(|x| x == "image")),
                "default": m["isDefault"] == true,
            }))
        })
        .collect();
    (!models.is_empty()).then_some(Value::Array(models))
}

/// The windows of a quota of Codex (`rateLimits` of `account/rateLimits/*`), as the core's `QuotaMeter`s in JSON: each
/// labelled by its length (5 hours, a week, 30 days on the free plan).
pub fn meters(limits: &Value) -> Vec<Value> {
    let mut all: Vec<(u64, Value)> = ["primary", "secondary"]
        .iter()
        .filter_map(|k| {
            let w = limits.get(*k).filter(|w| w.is_object())?;
            let minutes = w["windowDurationMins"].as_u64()?;
            Some((
                minutes,
                json!({
                    "minutes": minutes,
                    "pct": w["usedPercent"].as_f64().unwrap_or(0.0),
                    "resetsAt": w["resetsAt"].as_i64().map(|s| s * 1000),
                }),
            ))
        })
        .collect();
    all.sort_by_key(|(m, _)| *m);
    all.into_iter().map(|(_, v)| v).collect()
}

#[cfg(test)]
/// The answers of a question's card (`input.answers`, by question as Escouade keeps them) in the
/// form `item/tool/requestUserInput` takes: by the question's id, a list of answers.
pub fn answers(input: &Value) -> Value {
    let given = &input["answers"];
    let mut out = serde_json::Map::new();
    for q in input["questions"].as_array().into_iter().flatten() {
        let (Some(id), Some(text)) = (q["id"].as_str(), q["question"].as_str()) else {
            continue;
        };
        let answer = match &given[text] {
            Value::String(s) => vec![s.clone()],
            Value::Array(a) => a
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => continue,
        };
        out.insert(id.to_string(), json!({ "answers": answer }));
    }
    Value::Object(out)
}
