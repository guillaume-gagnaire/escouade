//! Claude Code's experimental features (ESC-553): « Fonctionnalités expérimentales » of this plugin's settings, passed to
//! the launch of its interface in a terminal alone (`terminal/launch`), never to its stream, a ticket's agent nor a
//! discussion.
//!
//! - **Code Mode** (`CLAUDE_CODE_EXPERIMENTAL_CODE_MODE`, Claude Code 2.1.323, 2.1.326 with the agent teams): Claude writes
//!   JavaScript that makes its tool calls (`REPL`). An interactive session in a terminal only: `-p` ignores the variable,
//!   and a session started in it is resumed in the terminal alone, the variable set (« Failed to resume session: This
//!   session was started in Code Mode… » in stream-json, 2.1.329). Not passed, and the conversation says why, with Remote
//!   Control, an account routed through a gateway or a cloud provider, a version too old or a model that does not take it.
//! - **Agent Teams** (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS`, 2.1.32): a session leads teammates; an interactive one only.
//! - **Others**: names of `CLAUDE_CODE_EXPERIMENTAL_*` alone, each set to `1`: never another variable, never a value.
//!
//! The variables go in the `env` of the interface's private settings (`--settings <file>`), which Claude Code reads as it
//! reads its user's settings, a `0` of the user's own still winning: no variable of the process's (`permissions.env`).
//! Each set for every project (`experimental.*`), then by project (`experimental.project.*`), and never by the project's
//! `.escouade.yml` (`repo: false`): a repository cloned does not turn them on, as Claude Code ignores Code Mode in a
//! project's settings.

use crate::words::said;
use escouade_plugin_protocol::engine::TerminalParams;
use serde::Deserialize;
use serde_json::{Map, Value};

/// The start of the name of each experimental variable of Claude Code.
pub const PREFIX: &str = "CLAUDE_CODE_EXPERIMENTAL_";
pub const CODE_MODE: &str = "CLAUDE_CODE_EXPERIMENTAL_CODE_MODE";
pub const AGENT_TEAMS: &str = "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS";
/// The oldest Claude Code with Code Mode, and with Code Mode and the agent teams together.
pub const CODE_MODE_MIN: &str = "2.1.323";
pub const CODE_MODE_WITH_TEAMS_MIN: &str = "2.1.326";
/// The oldest Claude Code with the agent teams.
pub const AGENT_TEAMS_MIN: &str = "2.1.32";

/// The keys of its settings: for every project, then for one (`select`: `inherit`, `on`, `off`; a text, empty to inherit).
pub mod key {
    pub const CODE_MODE: &str = "experimental.codeMode";
    pub const AGENT_TEAMS: &str = "experimental.agentTeams";
    pub const OTHERS: &str = "experimental.others";
    pub const HERE_CODE_MODE: &str = "experimental.project.codeMode";
    pub const HERE_AGENT_TEAMS: &str = "experimental.project.agentTeams";
    pub const HERE_OTHERS: &str = "experimental.project.others";
}

/// What a project's « Autres drapeaux » says for none, instead of those of every project.
const NONE: [&str; 3] = ["-", "aucun", "none"];

/// What the core tells `terminal/launch` beyond what the published protocol 0.1.2 has (ESC-553): read from the same
/// parameters, so that the plugin builds against it. A core of before gives none of them: nothing is passed.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    #[serde(default)]
    pub settings: Map<String, Value>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub gateway: Option<String>,
    /// The agent's session started in Code Mode: it resumes in its terminal alone, the variable set.
    #[serde(default)]
    pub terminal_only: bool,
}

/// What the settings ask for the agent's project.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Wanted {
    pub code_mode: bool,
    pub agent_teams: bool,
    /// The other names, valid, in their order, each once.
    pub others: Vec<String>,
    /// What was written among the others and is not taken: not a `CLAUDE_CODE_EXPERIMENTAL_*`, or a flag that has its
    /// own switch.
    pub refused: Vec<String>,
}

/// A switch, by its project's value (`on`, `off`; else every project's).
fn switch(s: &Map<String, Value>, app: &str, here: &str) -> bool {
    match s.get(here).and_then(Value::as_str) {
        Some("on") => true,
        Some("off") => false,
        _ => s.get(app).and_then(Value::as_bool).unwrap_or(false),
    }
}

/// Whether `name` is the name of an experimental variable of Claude Code: `CLAUDE_CODE_EXPERIMENTAL_` then capitals,
/// digits and `_`.
pub fn is_experimental(name: &str) -> bool {
    name.strip_prefix(PREFIX).is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    })
}

/// What the settings `s` (as `settings/get` gives them for the project) ask.
pub fn wanted(s: &Map<String, Value>) -> Wanted {
    let mut w = Wanted {
        code_mode: switch(s, key::CODE_MODE, key::HERE_CODE_MODE),
        agent_teams: switch(s, key::AGENT_TEAMS, key::HERE_AGENT_TEAMS),
        ..Wanted::default()
    };
    let text = |k: &str| {
        s.get(k)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty())
    };
    let others = match text(key::HERE_OTHERS) {
        Some(t) if NONE.contains(&t.to_lowercase().as_str()) => "",
        Some(t) => t,
        None => text(key::OTHERS).unwrap_or(""),
    };
    for name in others
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|n| !n.is_empty())
    {
        let taken = is_experimental(name) && name != CODE_MODE && name != AGENT_TEAMS;
        let list = if taken { &mut w.others } else { &mut w.refused };
        if !list.iter().any(|n| n == name) {
            list.push(name.to_string());
        }
    }
    w
}

/// A version's numbers (`2.1.329`, `2.1.329 (Claude Code)`).
fn numbers(v: &str) -> Option<Vec<u64>> {
    let word = v.split_whitespace().next()?;
    word.split('.')
        .map(|n| n.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()
        .filter(|n| !n.is_empty())
}

/// Whether the version `v` is older than `min`. An unknown version is not.
pub fn older(v: &str, min: &str) -> bool {
    match (numbers(v), numbers(min)) {
        (Some(a), Some(b)) => a < b,
        _ => false,
    }
}

/// Whether Code Mode takes the model `model` (Opus 5.5, Sonnet 5.5 and later, Haiku 5.5, Fable 5.2): an alias follows the
/// newest of its family; a full id is compared; a name not known is left to Claude Code.
pub fn takes_model(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    let m = m.split('[').next().unwrap_or("").trim();
    let Some(rest) = m.strip_prefix("claude-") else {
        return true;
    };
    let mut parts = rest.split(['-', '.']);
    let family = parts.next().unwrap_or("");
    let digits: Vec<u64> = parts
        .map_while(|p| p.parse::<u64>().ok().filter(|n| *n < 100))
        .collect();
    let min = match family {
        "opus" | "sonnet" | "haiku" => [5, 5],
        "fable" => [5, 2],
        _ => return true,
    };
    match digits.as_slice() {
        [major] => [*major, 0] >= min,
        [major, minor, ..] => [*major, *minor] >= min,
        [] => true,
    }
}

/// Why Code Mode is not passed to this launch, in the app's language; None: it is.
fn code_mode_refusal(p: &TerminalParams, c: &Context, teams: bool) -> Option<String> {
    let lang = p.lang.as_str();
    if p.read_only {
        return Some(
            said(
                lang,
                "une discussion lit sans rien lancer, et le Code Mode lance du code",
                "a discussion reads and runs nothing, and Code Mode runs code",
            )
            .to_string(),
        );
    }
    if p.remote.is_some() {
        return Some(said(lang, "le remote control est demandé pour cet agent, et Claude Code ne prend pas les deux", "Remote Control is asked for this agent, and Claude Code does not take both").to_string());
    }
    if let Some(g) = &c.gateway {
        return Some(match lang {
            "en" => format!("the account goes through {g} (a gateway or a cloud provider), and Code Mode works with Anthropic’s API alone"),
            _ => format!("le compte passe par {g} (une passerelle ou un fournisseur cloud), et le Code Mode ne marche qu’avec l’API d’Anthropic"),
        });
    }
    let min = if teams {
        CODE_MODE_WITH_TEAMS_MIN
    } else {
        CODE_MODE_MIN
    };
    if let Some(v) = c.version.as_deref().filter(|v| older(v, min)) {
        return Some(match (lang, teams) {
            ("en", true) => format!("Claude Code {v} is too old (it takes {min} or later with the Agent Teams): update it"),
            ("en", false) => format!("Claude Code {v} is too old (it takes {min} or later): update it"),
            (_, true) => format!("Claude Code {v} est trop ancien (il faut la {min} au moins avec les Agent Teams) : mets-le à jour"),
            (_, false) => format!("Claude Code {v} est trop ancien (il faut la {min} au moins) : mets-le à jour"),
        });
    }
    if !takes_model(&p.model) {
        let m = &p.model;
        return Some(match lang {
            "en" => format!(
                "the model {m} does not take it (Opus 5.5, Sonnet 5.5, Haiku 5.5 or Fable 5.2)"
            ),
            _ => format!(
                "le modèle {m} ne le prend pas (Opus 5.5, Sonnet 5.5, Haiku 5.5 ou Fable 5.2)"
            ),
        });
    }
    None
}

/// What the launch of the interface `p` gets: its variables, each `1`, and what the conversation says of what was not
/// passed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Judged {
    pub env: Vec<String>,
    pub warns: Vec<String>,
}

/// The experimental variables of the interface `p`, as its settings and its context `c` say.
pub fn judge(p: &TerminalParams, c: &Context) -> Judged {
    let w = wanted(&c.settings);
    let lang = p.lang.as_str();
    let mut j = Judged::default();
    if c.terminal_only {
        // Its session resumes only so, whatever the settings say now.
        j.env.push(CODE_MODE.into());
    } else if w.code_mode {
        match code_mode_refusal(p, c, w.agent_teams) {
            Some(why) => j.warns.push(match lang {
                "en" => format!("Code Mode not passed to the interface: {why}."),
                _ => format!("Code Mode non passé à l’interface : {why}."),
            }),
            None => j.env.push(CODE_MODE.into()),
        }
    }
    if w.agent_teams {
        match c.version.as_deref().filter(|v| older(v, AGENT_TEAMS_MIN)) {
            Some(v) => j.warns.push(match lang {
                "en" => format!("Agent Teams not passed to the interface: Claude Code {v} is too old (it takes {AGENT_TEAMS_MIN} or later)."),
                _ => format!("Agent Teams non passées à l’interface : Claude Code {v} est trop ancien (il faut la {AGENT_TEAMS_MIN} au moins)."),
            }),
            None => j.env.push(AGENT_TEAMS.into()),
        }
    }
    j.env.extend(w.others.iter().cloned());
    if !w.refused.is_empty() {
        let names = w.refused.join(", ");
        j.warns.push(match lang {
            "en" => format!("Not passed to the interface: {names} (only names of CLAUDE_CODE_EXPERIMENTAL_* are, and Code Mode and the Agent Teams have their own switch)."),
            _ => format!("Pas passés à l’interface : {names} (seuls des noms de CLAUDE_CODE_EXPERIMENTAL_* le sont, et le Code Mode et les Agent Teams ont leur interrupteur)."),
        });
    }
    j
}

/// The `env` of the interface's private settings: each variable at `1`. None when there is none.
pub fn env_block(env: &[String]) -> Option<Value> {
    (!env.is_empty()).then(|| {
        Value::Object(
            env.iter()
                .map(|n| (n.clone(), Value::String("1".into())))
                .collect(),
        )
    })
}

#[cfg(test)]
#[path = "experimental_tests.rs"]
mod tests;
