//! The element `turnModels` (protocol 0.1.7, ESC-662): what the turn about to end spent by model, its subagents' calls
//! on the model each ran on, which the core writes as one row of stats by model. This plugin builds on the protocol's
//! published crate, which does not have its type yet: the translation leaves a marked `said` in its place (`item`), and
//! what the plugin answers or pushes goes through `unmark`, which writes it as the core reads it. A core that does not
//! know it leaves it aside.

use escouade_plugin_protocol::engine::Item;
use serde_json::{json, Value};

/// What a marked `said` starts with: no line of a harness does.
const MARK: &str = "\u{1}escouade.turnModels\u{1}";

/// What a model spent in the turn.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spent {
    pub model: String,
    pub input: u64,
    pub output: u64,
    pub cache: u64,
    pub cost: f64,
}

/// The element for `models`, marked; none when there are none.
pub fn item(models: &[Spent]) -> Option<Item> {
    if models.is_empty() {
        return None;
    }
    let models: Vec<Value> = models
        .iter()
        .map(|m| {
            json!({ "model": m.model, "input": m.input, "output": m.output, "cache": m.cache, "cost": m.cost })
        })
        .collect();
    Some(Item::Said {
        line: format!("{MARK}{}", Value::Array(models)),
    })
}

/// `v` with each marked `said` written as the element `turnModels`.
pub fn unmark(v: &mut Value) {
    match v {
        Value::Array(a) => a.iter_mut().for_each(unmark),
        Value::Object(o) => {
            let marked = (o.get("op").and_then(Value::as_str) == Some("said"))
                .then(|| o.get("line").and_then(Value::as_str))
                .flatten()
                .and_then(|l| l.strip_prefix(MARK))
                .and_then(|m| serde_json::from_str::<Value>(m).ok());
            match marked {
                Some(models) => *v = json!({ "op": "turnModels", "models": models }),
                None => o.values_mut().for_each(unmark),
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marked_element_is_written_as_the_core_reads_it_and_nothing_else_changes() {
        let spent = Spent {
            model: "mai-code-1.1-flash".into(),
            input: 4,
            output: 2,
            cache: 1,
            cost: 0.5,
        };
        let items = vec![
            Item::Said {
                line: "une ligne".into(),
            },
            item(&[spent]).unwrap(),
        ];
        let mut v = json!({ "items": items });
        unmark(&mut v);
        assert_eq!(v["items"][0], json!({ "op": "said", "line": "une ligne" }));
        assert_eq!(
            v["items"][1],
            json!({ "op": "turnModels", "models": [
                { "model": "mai-code-1.1-flash", "input": 4, "output": 2, "cache": 1, "cost": 0.5 },
            ] })
        );
        assert!(item(&[]).is_none());
    }
}
