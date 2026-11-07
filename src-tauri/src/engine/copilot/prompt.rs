//! The texts written by Copilot CLI (« Modèle des textes rédigés »): one question to `copilot -p`
//! read from its input, its answer the last message of its `--output-format json` stream. Tried on
//! Copilot CLI 1.0.98 (ESC-40, a Free account):
//! - its input is the question (« piped stdin »), so that it is never on the command line;
//! - `--available-tools=` (empty) keeps every tool, but a list naming no tool of its own leaves it
//!   none: a short text then costs a sixth of what it costs with them (0.04 credit, not 0.23);
//! - a Free account takes no `--model` but `auto` (« Model "gpt-5-mini" from --model flag is not
//!   available », no output at all): the question is then asked again on `auto`;
//! - each run is a session of the account's folder (`session-state/<id>`) and writes a log there:
//!   its session is named (`--session-id`) and removed once answered, its log kept apart and
//!   removed.

use crate::engine::{OneShot, Weight};
use anyhow::{bail, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The model of the texts: a light one of Copilot's, given always (`auto` picks one itself).
pub(crate) const MODEL: &str = "gpt-5-mini";

/// What the setting « Modèle des textes rédigés » names.
pub(crate) const WRITER: &str = "Copilot (GPT-5 mini)";

/// The only tools a short text may use: none of Copilot's (a list of none of its tools leaves it
/// none, tried).
const NO_TOOL: &str = "escouade-aucun-outil";

/// The tools a text written after reading its folder uses: reading ones only, allowed without a
/// question (`-p` has nobody to ask).
const READING_TOOLS: &str = "view,glob,grep";

/// The arguments of `copilot -p` for a question of `weight` on `model`, in the session `session`,
/// its log in `logs`; its question read from its input.
pub(crate) fn args(weight: Weight, model: &str, session: &str, logs: &Path) -> Vec<String> {
    let effort = match weight {
        Weight::Light => "low",
        Weight::Reading | Weight::Given => "medium",
    };
    let mut a: Vec<String> = [
        "--output-format",
        "json",
        "--stream",
        "off",
        "--no-auto-update",
        "--disable-builtin-mcps",
        "--no-custom-instructions",
        "--disallow-temp-dir",
        "--model",
        model,
    ]
    .map(String::from)
    .to_vec();
    if model != "auto" {
        a.extend(["--reasoning-effort".to_string(), effort.to_string()]);
    }
    match weight {
        Weight::Light | Weight::Given => a.push(format!("--available-tools={NO_TOOL}")),
        Weight::Reading => {
            a.push(format!("--available-tools={READING_TOOLS}"));
            a.push(format!("--allow-tool={READING_TOOLS}"));
        }
    }
    a.push(format!("--session-id={session}"));
    a.push("--log-dir".into());
    a.push(logs.to_string_lossy().into_owned());
    a
}

/// The question as Copilot reads it: `-p` takes no system prompt of its own.
pub(crate) fn prompt(system: &str, prompt: &str) -> String {
    format!("<instructions>\n{system}\n</instructions>\n\n{prompt}")
}

/// The answer in the `--output-format json` stream `out`: its last message; refused with
/// Copilot's reason when the session failed. None: Copilot wrote nothing (it refused to start: a
/// model it does not offer, say).
pub(crate) fn answer(out: &[u8]) -> Option<Result<String>> {
    let mut last = None;
    let mut error = None;
    let mut any = false;
    for line in String::from_utf8_lossy(out).lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        any = true;
        match v["type"].as_str() {
            Some("assistant.message") if v["agentId"].is_null() => {
                let text = v["data"]["content"].as_str().unwrap_or_default();
                if !text.trim().is_empty() {
                    last = Some(text.trim().to_string());
                }
            }
            Some("session.error") => {
                error = v["data"]["message"].as_str().map(str::to_string);
            }
            _ => {}
        }
    }
    if !any {
        return None;
    }
    Some(match (last, error) {
        (Some(text), _) => Ok(text),
        (None, Some(e)) => Err(anyhow::anyhow!(e)),
        (None, None) => Err(anyhow::anyhow!(tr!(
            "Copilot n’a rien répondu",
            "Copilot gave no answer"
        ))),
    })
}

/// The folder of the account of a question given `env`: its `COPILOT_HOME`, else `~/.copilot`.
fn home(env: &[(String, String)]) -> Option<PathBuf> {
    env.iter()
        .find(|(k, _)| k == "COPILOT_HOME")
        .map(|(_, v)| PathBuf::from(v))
        .or_else(|| {
            // Never the machine user's in the tests.
            (!cfg!(test))
                .then(dirs::home_dir)
                .flatten()
                .map(|h| h.join(".copilot"))
        })
}

/// One question to Copilot (`copilot -p`), as `ask` says: its answer (its limit is
/// `oneshot::ask`'s). Refused its model (a Free account offers none), it is asked again on `auto`.
pub(crate) async fn one_shot(ask: OneShot<'_>) -> Result<String> {
    let question = prompt(ask.system, ask.prompt);
    let home = home(&ask.env);
    // An account of its own on its own sign-in, never on a token of the app's environment.
    let mut env = ask.env.clone();
    if super::process::own_home(&env) {
        for var in super::process::INHERITED_TOKENS {
            env.push((var.to_string(), String::new()));
        }
    }
    for model in [MODEL, "auto"] {
        let session = uuid::Uuid::new_v4().to_string();
        let logs = std::env::temp_dir().join(format!("escouade-copilot-{session}"));
        let args = args(ask.weight, model, &session, &logs);
        let out =
            crate::engine::oneshot::run(&ask.program, ask.cwd, &args, &env, &question, WRITER)
                .await;
        // Its session and its log go, answered or not.
        if let Some(h) = &home {
            let _ = std::fs::remove_dir_all(h.join("session-state").join(&session));
        }
        let _ = std::fs::remove_dir_all(&logs);
        match answer(&out?) {
            Some(answered) => return answered,
            None => log::info!("copilot -p: nothing on {model}, asked again on auto"),
        }
    }
    bail!(tr!(
        "Copilot n’a pas répondu : est-il connecté (copilot login) ?",
        "Copilot didn’t answer: is it signed in (copilot login)?"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_text_goes_without_tools_and_a_reading_one_only_reads() {
        let logs = Path::new("C:/tmp/logs");
        let light = args(Weight::Light, MODEL, "s1", logs);
        for a in [
            "--output-format",
            "json",
            "--disable-builtin-mcps",
            "--no-custom-instructions",
            "gpt-5-mini",
            "low",
            "--available-tools=escouade-aucun-outil",
            "--session-id=s1",
            "C:/tmp/logs",
        ] {
            assert!(light.iter().any(|x| x == a), "{a}: {light:?}");
        }
        // The question is never on the command line: it is read from the input.
        assert!(!light.iter().any(|x| x == "-p" || x == "--prompt"));
        let reading = args(Weight::Reading, MODEL, "s2", logs);
        assert!(reading.contains(&"--available-tools=view,glob,grep".to_string()));
        assert!(reading.contains(&"--allow-tool=view,glob,grep".to_string()));
        assert!(reading.contains(&"medium".to_string()));
        // `auto` takes no effort.
        let auto = args(Weight::Light, "auto", "s3", logs);
        assert!(!auto.iter().any(|x| x == "--reasoning-effort"));
    }

    #[test]
    fn a_text_given_has_the_effort_of_a_reading_and_no_tool() {
        let given = args(Weight::Given, MODEL, "s4", Path::new("C:/tmp/logs"));
        assert!(given.contains(&"medium".to_string()));
        assert!(given.contains(&"--available-tools=escouade-aucun-outil".to_string()));
        assert!(!given.iter().any(|x| x.contains("--allow-tool")));
        assert!(!given.iter().any(|x| x.contains("view")));
    }

    #[test]
    fn the_answer_is_the_last_message_and_an_empty_stream_says_nothing() {
        // As Copilot CLI 1.0.98 wrote it (ESC-40), shortened.
        let out = br#"{"type":"session.auto_mode_resolved","data":{"chosenModel":"mai-code-1.1-flash"}}
{"type":"user.message","data":{"content":"Nomme."}}
{"type":"assistant.message","data":{"messageId":"m1","content":"Je regarde.","toolRequests":[]}}
{"type":"assistant.message","data":{"messageId":"m2","content":" ajouter-bouton-connexion\n","phase":"final_answer"}}
{"type":"assistant.message","agentId":"a1","data":{"messageId":"m3","content":"sous-agent"}}
{"type":"session.usage_checkpoint","data":{"totalNanoAiu":37767500}}
{"type":"result","sessionId":"s","exitCode":0}
"#;
        assert_eq!(answer(out).unwrap().unwrap(), "ajouter-bouton-connexion");
        let failed = br#"{"type":"session.error","data":{"errorType":"quota","message":"You have no credits left."}}
{"type":"result","sessionId":"s","exitCode":1}
"#;
        assert_eq!(
            format!("{:#}", answer(failed).unwrap().unwrap_err()),
            "You have no credits left."
        );
        assert!(answer(b"").is_none());
        assert!(answer(b"Error: Model \"x\" from --model flag is not available.\n").is_none());
    }

    #[test]
    fn the_question_carries_its_instructions_first() {
        assert_eq!(
            prompt("Nomme.", "la tâche"),
            "<instructions>\nNomme.\n</instructions>\n\nla tâche"
        );
    }
}
