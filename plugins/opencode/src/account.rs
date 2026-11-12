//! What an account of OpenCode tells by a process of its own (`account/read`): its models, as `opencode models --verbose`
//! lists them (the providers connected to it, its default one first). Its terminal view is no launch of its own: its
//! interface joins the agent's shared server (`opencode attach`), as `agent/open` answers it (`joins`).

use crate::rules::listed;
use escouade_plugin_protocol::engine::{AccountLines, AccountRead, AccountStep, Launch};
use serde_json::{json, Value};

/// The first step of a read of its account: its models.
pub fn read(r: &AccountRead) -> AccountStep {
    match r.read.as_str() {
        "models" => AccountStep::start(
            Launch {
                args: vec!["models".into(), "--verbose".into()],
                ..Launch::default()
            },
            Vec::new(),
            json!({ "listed": "" }),
        ),
        other => AccountStep::failed(format!("OpenCode reads no {other}")),
    }
}

/// What `opencode models --verbose` wrote, gathered until it ends: its models, none when it lists none.
pub fn lines(l: &AccountLines) -> AccountStep {
    let mut text = l.state["listed"].as_str().unwrap_or_default().to_string();
    for line in &l.lines {
        text.push_str(line);
        text.push('\n');
    }
    if !l.ended {
        return AccountStep::more(Vec::new(), json!({ "listed": text }));
    }
    let models = listed(&text).models;
    AccountStep::done(match models.is_empty() {
        true => Value::Null,
        false => serde_json::to_value(models).unwrap_or(Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_models_are_what_opencode_models_lists_once_it_ends() {
        let step = read(&AccountRead {
            read: "models".into(),
            ..AccountRead::default()
        });
        assert_eq!(step.launch.as_ref().unwrap().args, ["models", "--verbose"]);
        // It writes nothing to it: its input is closed.
        assert!(step.writes.is_empty());
        let first = lines(&AccountLines {
            read: "models".into(),
            state: step.state,
            lines: vec!["ollama/qwen3".into(), "{\"name\":\"Qwen 3\"}".into()],
            ..AccountLines::default()
        });
        assert!(first.done.is_none() && first.error.is_none());
        let last = lines(&AccountLines {
            read: "models".into(),
            state: first.state,
            ended: true,
            ..AccountLines::default()
        });
        assert_eq!(last.done.unwrap()[0]["id"], "ollama/qwen3");
        let none = lines(&AccountLines {
            read: "models".into(),
            ended: true,
            ..AccountLines::default()
        });
        assert_eq!(none.done, Some(Value::Null));
    }
}
