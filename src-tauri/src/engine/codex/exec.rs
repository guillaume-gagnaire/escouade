//! The texts written by Codex (« Modèle des textes rédigés »): one question to `codex exec`, with
//! no session kept (`--ephemeral`), none of the user's settings nor MCP servers
//! (`--ignore-user-config`), in a read-only sandbox, its answer the last message of its `--json`
//! stream. Tried on Codex 0.163.0 (ESC-38): without its shell, a short text comes at once; with it,
//! a model asked for a name looks around its folder first.

use crate::engine::{OneShot, Weight};
use anyhow::{bail, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The model of the texts: Codex's lightest, the one its free plan has.
pub(crate) const MODEL: &str = "gpt-6-luna";

/// What the setting « Modèle des textes rédigés » names.
pub(crate) const WRITER: &str = "Codex (GPT-6-Luna)";

/// The tools a short text goes without: it needs none, and each one weighs on the question.
const LIGHT_OFF: [&str; 5] = [
    "shell_tool",
    "apps",
    "plugins",
    "browser_use",
    "computer_use",
];

/// The arguments of `codex exec` for a question of `weight`, its account's Windows sandbox
/// `sandbox` given back (`--ignore-user-config` drops it, and without one every command is
/// refused: tried), its question read from its input.
pub(crate) fn args(weight: Weight, sandbox: Option<&str>) -> Vec<String> {
    let effort = match weight {
        Weight::Light => "low",
        Weight::Reading | Weight::Given => "medium",
    };
    let mut a: Vec<String> = [
        "exec",
        "--ephemeral",
        "--skip-git-repo-check",
        "--ignore-user-config",
        "--ignore-rules",
        "--sandbox",
        "read-only",
        "--json",
        "--color",
        "never",
        "-m",
        MODEL,
        "-c",
    ]
    .map(String::from)
    .to_vec();
    a.push(format!("model_reasoning_effort=\"{effort}\""));
    match weight {
        Weight::Light | Weight::Given => {
            for f in LIGHT_OFF {
                a.extend(["--disable".to_string(), f.to_string()]);
            }
        }
        Weight::Reading => {
            if let Some(s) = sandbox {
                a.extend(["-c".to_string(), format!("windows.sandbox=\"{s}\"")]);
            }
        }
    }
    a.push("-".into());
    a
}

/// The question as Codex reads it: `codex exec` takes no system prompt of its own.
pub(crate) fn prompt(system: &str, prompt: &str) -> String {
    format!("<instructions>\n{system}\n</instructions>\n\n{prompt}")
}

/// The Windows sandbox the `config.toml` `text` sets (`[windows] sandbox`), when it is one Codex
/// knows.
pub(crate) fn windows_sandbox(text: &str) -> Option<String> {
    let mut in_windows = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_windows = line == "[windows]";
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let ours = (in_windows && key == "sandbox") || (!in_windows && key == "windows.sandbox");
        if ours {
            let value = value
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"');
            return ["elevated", "unelevated"]
                .contains(&value)
                .then(|| value.to_string());
        }
    }
    None
}

/// The folder of the account of a question given `env`: its `CODEX_HOME`, else `~/.codex`.
fn home(env: &[(String, String)]) -> Option<PathBuf> {
    env.iter()
        .find(|(k, _)| k == "CODEX_HOME")
        .map(|(_, v)| PathBuf::from(v))
        .or_else(|| {
            // Never the machine user's in the tests.
            (!cfg!(test))
                .then(dirs::home_dir)
                .flatten()
                .map(|h| h.join(".codex"))
        })
}

/// The answer in the `--json` stream `out`: its last message; refused with Codex's reason when
/// its turn failed.
pub(crate) fn answer(out: &[u8]) -> Result<String> {
    let mut last = None;
    for line in String::from_utf8_lossy(out).lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match v["type"].as_str() {
            Some("item.completed") if v["item"]["type"] == "agent_message" => {
                last = v["item"]["text"].as_str().map(str::to_string);
            }
            Some("turn.failed") => bail!(reason(&v["error"]["message"])),
            _ => {}
        }
    }
    match last {
        Some(text) if !text.trim().is_empty() => Ok(text.trim().to_string()),
        _ => bail!(tr!("Codex n’a rien répondu", "Codex gave no answer")),
    }
}

/// What a failed turn says: the API's message within it, when it holds one.
fn reason(message: &Value) -> String {
    let text = message.as_str().unwrap_or_default();
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| text.to_string())
}

/// One question to Codex (`codex exec`), as `ask` says: its answer (its limit is
/// `oneshot::ask`'s).
pub(crate) async fn one_shot(ask: OneShot<'_>) -> Result<String> {
    let sandbox = home(&ask.env)
        .and_then(|h| std::fs::read_to_string(h.join("config.toml")).ok())
        .and_then(|text| windows_sandbox(&text));
    let args = args(ask.weight, sandbox.as_deref());
    let cwd: &Path = ask.cwd;
    let out = crate::engine::oneshot::run(
        &ask.program,
        cwd,
        &args,
        &ask.env,
        &prompt(ask.system, ask.prompt),
        WRITER,
    )
    .await?;
    answer(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_text_goes_without_tools_and_a_reading_one_keeps_the_accounts_sandbox() {
        let light = args(Weight::Light, Some("elevated"));
        for a in [
            "exec",
            "--ephemeral",
            "--ignore-user-config",
            "read-only",
            "--json",
            "gpt-6-luna",
            "shell_tool",
            "model_reasoning_effort=\"low\"",
        ] {
            assert!(light.iter().any(|x| x == a), "{a}");
        }
        assert!(!light.iter().any(|x| x.contains("windows.sandbox")));
        assert_eq!(light.last().map(String::as_str), Some("-"));
        let reading = args(Weight::Reading, Some("elevated"));
        assert!(reading.contains(&"windows.sandbox=\"elevated\"".to_string()));
        assert!(!reading.contains(&"shell_tool".to_string()));
        assert!(!args(Weight::Reading, None)
            .iter()
            .any(|x| x.contains("windows")));
    }

    /// `--sandbox read-only` reads the whole disk: a text from a third party goes without shell.
    #[test]
    fn a_text_given_has_the_effort_of_a_reading_and_no_tool() {
        let given = args(Weight::Given, Some("elevated"));
        assert!(given.contains(&"model_reasoning_effort=\"medium\"".to_string()));
        for f in LIGHT_OFF {
            assert!(
                given.windows(2).any(|w| w == ["--disable", f]),
                "{f}: {given:?}"
            );
        }
        assert!(!given.iter().any(|x| x.contains("windows.sandbox")));
    }

    #[test]
    fn the_windows_sandbox_is_read_from_its_table_or_its_dotted_key() {
        assert_eq!(
            windows_sandbox("model = \"x\"\n\n[windows]\nsandbox = \"elevated\" # admin\n"),
            Some("elevated".into())
        );
        assert_eq!(
            windows_sandbox("windows.sandbox = \"unelevated\"\n"),
            Some("unelevated".into())
        );
        // Another table's `sandbox`, or a value Codex does not know: none.
        assert_eq!(
            windows_sandbox("[profiles.x]\nsandbox = \"elevated\"\n"),
            None
        );
        assert_eq!(windows_sandbox("[windows]\nsandbox = \"rien\"\n"), None);
        assert_eq!(windows_sandbox(""), None);
    }

    #[test]
    fn the_answer_is_the_last_message_and_a_failed_turn_says_why() {
        // As Codex 0.163.0 wrote them (ESC-38): a message before the answer, then the answer.
        let out = br#"{"type":"thread.started","thread_id":"t"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Je regarde."}}
{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":" ajouter-bouton-connexion\n"}}
{"type":"turn.completed","usage":{"input_tokens":12094,"output_tokens":11}}
"#;
        assert_eq!(answer(out).unwrap(), "ajouter-bouton-connexion");
        let failed = br#"{"type":"item.completed","item":{"id":"item_0","type":"error","message":"Model metadata not found."}}
{"type":"turn.failed","error":{"message":"{\"type\":\"error\",\"status\":400,\"error\":{\"type\":\"invalid_request_error\",\"message\":\"The 'x' model is not supported when using Codex with a ChatGPT account.\"}}"}}
"#;
        assert_eq!(
            format!("{:#}", answer(failed).unwrap_err()),
            "The 'x' model is not supported when using Codex with a ChatGPT account."
        );
        assert!(answer(b"").is_err());
    }

    #[test]
    fn the_question_carries_its_instructions_first() {
        assert_eq!(
            prompt("Nomme.", "la tâche"),
            "<instructions>\nNomme.\n</instructions>\n\nla tâche"
        );
    }
}
