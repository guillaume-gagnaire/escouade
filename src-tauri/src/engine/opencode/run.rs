//! The texts written by OpenCode (« Modèle des textes rédigés »): one question to `opencode run`,
//! on the model Escouade gives it (`--model`, and `model` and `small_model` in its config: without
//! them OpenCode would pick OpenCode Zen without an account, seen at the real run of ESC-25), its
//! answer the last message of its `--format json` stream. Tried on OpenCode 1.19.3 with a local
//! model (ESC-45, docs/PROTOCOL.md, « OpenCode serve »):
//! - the question is read from its input, appended to its message (none on the command line);
//! - an agent of Escouade's own, declared by `OPENCODE_CONFIG_CONTENT`, gives the instructions as
//!   its prompt and the tools it may use: none for a short text (178 tokens in all, instead of
//!   about 10 700 with OpenCode's prompt and tools), only those that read for a text written after
//!   reading its folder;
//! - each run keeps a session in OpenCode's history: deleted once read (`opencode session delete`);
//! - a model OpenCode does not know ends on an event `error`.

use super::split_model;
use crate::engine::{mcp, OneShot, Program, Weight};
use crate::i18n;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The agent of OpenCode the texts are written by (`--agent`).
const AGENT: &str = "escouade-writer";

/// How long the deletion of its session may take (1,2 s on 1.19.3, tried).
const DELETE_LIMIT: Duration = Duration::from_secs(20);

/// The tools a text written after reading its folder may use: those that only read.
const READING: [&str; 4] = ["read", "glob", "grep", "list"];

/// What `opencode run` is given on top of the user's config: the model, the writer's agent (its
/// instructions, its tools), nothing of Escouade's server, no share nor update of its own.
pub(crate) fn config(model: &str, weight: Weight, system: &str) -> Value {
    let mut tools = json!({ "*": false });
    let mut permission = json!({ "*": "deny", "external_directory": "deny" });
    if weight == Weight::Reading {
        for t in READING {
            tools[t] = json!(true);
            permission[t] = json!("allow");
        }
    }
    json!({
        "autoupdate": false,
        "share": "disabled",
        "model": model,
        "small_model": model,
        "mcp": { mcp::SERVER: { "enabled": false } },
        "agent": { AGENT: {
            "mode": "primary",
            "description": "Escouade's texts",
            "prompt": system,
            "tools": tools,
            "permission": permission,
        } },
    })
}

/// The arguments of `opencode run` on `model`, its question read from its input.
pub(crate) fn args(model: &str) -> Vec<String> {
    [
        "run", "--format", "json", "--model", model, "--agent", AGENT, "--title", "Escouade",
    ]
    .map(String::from)
    .to_vec()
}

/// The answer in the `--format json` stream `out`: the text of its last message, and the session
/// it kept; refused with OpenCode's reason when it failed.
pub(crate) fn answer(out: &[u8]) -> (Option<String>, Result<String>) {
    let mut session = None;
    let mut texts: Vec<(String, String)> = Vec::new();
    let mut error = None;
    for line in String::from_utf8_lossy(out).lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if session.is_none() {
            session = v["sessionID"].as_str().map(str::to_string);
        }
        match v["type"].as_str() {
            Some("text") => {
                let p = &v["part"];
                if let Some(text) = p["text"].as_str() {
                    let msg = p["messageID"].as_str().unwrap_or_default().to_string();
                    texts.push((msg, text.to_string()));
                }
            }
            Some("error") => {
                let e = &v["error"];
                error = Some(
                    e["data"]["message"]
                        .as_str()
                        .or(e["message"].as_str())
                        .or(e["name"].as_str())
                        .unwrap_or("error")
                        .to_string(),
                );
            }
            _ => {}
        }
    }
    let last = texts.last().map(|(m, _)| m.clone());
    let text: String = texts
        .iter()
        .filter(|(m, _)| Some(m) == last.as_ref())
        .map(|(_, t)| t.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let result = match (error, text.trim()) {
        (Some(e), _) => Err(anyhow::anyhow!(tr!("OpenCode : {e}", "OpenCode: {e}"))),
        (None, "") => Err(anyhow::anyhow!(tr!(
            "OpenCode n’a rien répondu",
            "OpenCode gave no answer"
        ))),
        (None, t) => Ok(t.to_string()),
    };
    (session, result)
}

/// The folder a short text is asked in: one of its own, empty (OpenCode looks at its folder).
fn light_dir() -> PathBuf {
    let dir = std::env::temp_dir()
        .join("escouade-opencode")
        .join("writer");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// One question to OpenCode (`opencode run`), as `ask` says: its answer (its limit is
/// `oneshot::ask`'s). Without a model of a provider, none is asked.
pub(crate) async fn one_shot(ask: OneShot<'_>) -> Result<String> {
    let model = ask.model.as_str();
    if split_model(model).is_none() {
        bail!(escouade_plugin_opencode::no_model(i18n::ui().tag()));
    }
    let cwd: PathBuf = match ask.weight {
        Weight::Light => light_dir(),
        Weight::Reading => ask.cwd.to_path_buf(),
    };
    let mut env = ask.env.clone();
    env.push((
        "OPENCODE_CONFIG_CONTENT".into(),
        config(model, ask.weight, ask.system).to_string(),
    ));
    env.push(("OPENCODE_DISABLE_AUTOUPDATE".into(), "1".into()));
    let out = crate::engine::oneshot::run(
        &ask.program,
        &cwd,
        &args(model),
        &env,
        ask.prompt,
        "OpenCode",
    )
    .await?;
    let (session, answer) = answer(&out);
    if let Some(s) = session {
        delete(&ask.program, &cwd, &env, &s).await;
    }
    answer
}

/// The session a question kept in OpenCode's history, deleted.
async fn delete(program: &Program, cwd: &Path, env: &[(String, String)], session: &str) {
    let args: Vec<String> = ["session", "delete", session].map(String::from).to_vec();
    let Ok(launch) = program.launch(&cwd.to_string_lossy(), &args, env) else {
        return;
    };
    let mut cmd = tokio::process::Command::new(&launch.program);
    cmd.args(&launch.args)
        .envs(launch.env)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    crate::proc::no_inherited_fds(&mut cmd);
    match crate::proc::output_within(cmd, DELETE_LIMIT).await {
        Ok(out) if out.status.success() => {}
        Ok(out) => log::warn!(
            "opencode: session {session} of a text not deleted: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(e) => log::warn!("opencode: session {session} of a text not deleted: {e:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_text_goes_without_tools_and_a_reading_one_only_reads() {
        let light = config("ollama/q", Weight::Light, "Nomme.");
        assert_eq!(light["model"], "ollama/q");
        assert_eq!(light["small_model"], "ollama/q");
        assert_eq!(light["mcp"]["escouade"]["enabled"], false);
        let agent = &light["agent"][AGENT];
        assert_eq!(agent["prompt"], "Nomme.");
        assert_eq!(agent["tools"], json!({ "*": false }));
        assert_eq!(agent["permission"]["*"], "deny");
        let reading = config("ollama/q", Weight::Reading, "Lis.");
        let agent = &reading["agent"][AGENT];
        for t in READING {
            assert_eq!(agent["tools"][t], true, "{t}");
            assert_eq!(agent["permission"][t], "allow", "{t}");
        }
        assert_eq!(agent["tools"]["bash"], Value::Null);
        assert_eq!(agent["permission"]["external_directory"], "deny");
        let a = args("ollama/q");
        assert!(a.windows(2).any(|w| w == ["--model", "ollama/q"]));
        assert!(a.windows(2).any(|w| w == ["--agent", AGENT]));
        assert!(a.windows(2).any(|w| w == ["--format", "json"]));
    }

    #[test]
    fn the_answer_is_the_last_messages_text_and_an_error_says_why() {
        // As OpenCode 1.19.3 wrote them (ESC-45): a message with a tool, then the answer.
        let out = br#"{"type":"step_start","sessionID":"ses_1","part":{"type":"step-start"}}
{"type":"text","sessionID":"ses_1","part":{"messageID":"msg_a","type":"text","text":"Je regarde."}}
{"type":"tool_use","sessionID":"ses_1","part":{"messageID":"msg_a","type":"tool","tool":"read"}}
{"type":"text","sessionID":"ses_1","part":{"messageID":"msg_b","type":"text","text":" corrige-bouton-enregistrer\n"}}
{"type":"step_finish","sessionID":"ses_1","part":{"type":"step-finish","cost":0}}
"#;
        let (session, text) = answer(out);
        assert_eq!(session.as_deref(), Some("ses_1"));
        assert_eq!(text.unwrap(), "corrige-bouton-enregistrer");
        let failed = br#"{"type":"error","sessionID":"ses_2","error":{"name":"UnknownError","data":{"message":"Unexpected server error. Check server logs for details."}}}"#;
        let (session, text) = answer(failed);
        assert_eq!(session.as_deref(), Some("ses_2"));
        assert!(format!("{:#}", text.unwrap_err()).contains("Unexpected server error"));
        assert!(answer(b"").1.is_err());
    }
}
