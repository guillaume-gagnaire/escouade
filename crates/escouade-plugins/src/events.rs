//! What a manifest says of the facts it is handed (its webhooks, its commands, their templates) and what the app tells the
//! host of its place (`Place`); the app's `plugins::events` hands the facts. The facts handed to the plugins (PLG-5, ESC-177; docs/PLUGINS-DESIGN.md §1.6, §3.3): the sink of the
//! facts' bus (`crate::facts`) that gives each plugin the facts it asked for
//! (`permissions.events`), only of the projects it takes part in (`Plugins::reaches`, on the `Choices` of PLG-4: a
//! private project's go only to a plugin turned on for it), through a bounded queue of its own,
//! never waiting for it.
//!
//! What a plugin does of a fact:
//! - its program (`run`) is told it (`events/fact`, a notification); the next fact goes once its
//!   input took the last one, so that a plugin that does not read keeps its facts in its queue;
//! - its webhooks (`contributes.webhooks`): a URL, the facts, a template of the body; called
//!   without code;
//! - its commands (`contributes.commands`): a command line run on a fact, once the user agreed to
//!   them as they are (`commands_fingerprint`), as `.escouade.yml`'s (ESC-146).
//!
//! A queue full drops its oldest fact, and says it in the plugin's journal.

use super::manifest::Manifest;
use super::runtime::matches;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;

/// A shell of the machine (the app's `pty` lists them): what a plugin's command runs in.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellInfo {
    pub id: String,
    pub label: String,
    pub path: String,
}

/// How many facts wait for a plugin before the oldest is dropped.
pub const QUEUE: usize = 256;
/// How long a webhook has to answer.
pub const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a command may run.
pub const COMMAND_LIMIT: Duration = Duration::from_secs(60);
/// How much of a webhook's answer or a command's output the journal keeps.
pub const SAID: usize = 300;

/// What the app tells the dispatch: the shell commands run in, a project's folder.
pub trait Place: Send + Sync {
    fn shell(&self) -> Option<ShellInfo>;
    fn project_path(&self, id: &str) -> Option<String>;
    /// Whether the project `id` is private, and the settings its `.escouade.yml` gives the plugin
    /// `plugin` (`plugins.<plugin>`, PLG-14; empty when none); None for a project that is not.
    fn project_plugin(&self, id: &str, plugin: &str) -> Option<(bool, Map<String, Value>)>;
}

/// A webhook of a plugin (`contributes.webhooks[]`).
#[derive(Debug, Clone)]
pub struct Webhook {
    /// Its address: a text, which may take a secret of the plugin (`{{secrets.webhook}}`).
    pub url: String,
    pub events: Vec<String>,
    /// What the fact must hold, by path (`data.to`: `review`, or a list of values).
    pub when: Map<String, Value>,
    /// Its body: a template, text or JSON; none sends the fact.
    pub body: Option<Value>,
    pub content_type: Option<String>,
    pub headers: Vec<(String, String)>,
}

/// A command of a plugin run on facts (`contributes.commands[]` with `run` and `events`).
#[derive(Debug, Clone)]
pub struct Command {
    pub run: String,
    pub events: Vec<String>,
    pub when: Map<String, Value>,
}

pub fn strings(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn object(v: Option<&Value>) -> Map<String, Value> {
    v.and_then(Value::as_object).cloned().unwrap_or_default()
}

/// The webhooks a manifest declares; one without an address or a fact is left out.
pub fn webhooks(m: &Manifest) -> Vec<Webhook> {
    let Some(list) = m.contributes.get("webhooks").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|w| {
            let url = w.get("url")?.as_str()?.trim().to_string();
            let events = strings(w.get("events"));
            (!url.is_empty() && !events.is_empty()).then(|| Webhook {
                url,
                events,
                when: object(w.get("when")),
                body: w.get("body").cloned(),
                content_type: w
                    .get("contentType")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                headers: object(w.get("headers"))
                    .into_iter()
                    .filter_map(|(k, v)| v.as_str().map(|v| (k, v.to_string())))
                    .collect(),
            })
        })
        .collect()
}

/// The commands a manifest runs on facts.
pub fn commands(m: &Manifest) -> Vec<Command> {
    let Some(list) = m.contributes.get("commands").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|c| {
            let run = c.get("run")?.as_str()?.trim().to_string();
            let events = strings(c.get("events"));
            (!run.is_empty() && !events.is_empty()).then(|| Command {
                run,
                events,
                when: object(c.get("when")),
            })
        })
        .collect()
}

/// The fingerprint of the commands a manifest runs on facts, as written: another command, another
/// fact, another condition, and it must be agreed to again. Empty without any.
pub fn commands_fingerprint(m: &Manifest) -> String {
    let list = commands(m);
    if list.is_empty() {
        return String::new();
    }
    let mut h = Sha256::new();
    for c in &list {
        h.update(c.run.as_bytes());
        h.update([0]);
        h.update(c.events.join("\n").as_bytes());
        h.update([0]);
        h.update(Value::Object(c.when.clone()).to_string().as_bytes());
        h.update([1]);
    }
    h.finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

/// The value at `path` (`ticket.key`, `data.to`) of `v`.
fn at<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(v, |v, k| v.get(k).filter(|x| !x.is_null()))
}

/// Whether a fact (`v`, as the plugins get it) is one of `events` and holds what `when` asks.
pub fn wanted(v: &Value, name: &str, events: &[String], when: &Map<String, Value>) -> bool {
    events.iter().any(|p| matches(p, name))
        && when.iter().all(|(path, want)| {
            let got = at(v, path);
            match want {
                Value::Array(any) => any.iter().any(|w| got == Some(w)),
                w => got == Some(w),
            }
        })
}

/// The text of a value in a template: a text as it is, anything else as JSON, nothing for none.
pub fn text_of(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// A text with `{{path}}` replaced by the fact's values; `{{secrets.<key>}}` by the plugin's
/// secret when `secret` gives one. `escape`: each value JSON-escaped (inside a JSON text).
pub fn render(
    template: &str,
    fact: &Value,
    secret: &dyn Fn(&str) -> Option<String>,
    escape: bool,
) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let path = after[..end].trim();
        let value = match path.strip_prefix("secrets.") {
            Some(key) => secret(key).unwrap_or_default(),
            None => text_of(at(fact, path)),
        };
        if escape {
            let quoted = Value::String(value).to_string();
            out.push_str(&quoted[1..quoted.len() - 1]);
        } else {
            out.push_str(&value);
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// A JSON template with the fact's values: a text that is one `{{path}}` alone takes the value
/// itself (a list stays a list), any other text is rendered.
pub fn render_json(template: &Value, fact: &Value) -> Value {
    match template {
        Value::String(s) => {
            let t = s.trim();
            if let Some(path) = t
                .strip_prefix("{{")
                .and_then(|x| x.strip_suffix("}}"))
                .filter(|p| !p.contains("{{") && !p.trim().starts_with("secrets."))
            {
                return at(fact, path.trim()).cloned().unwrap_or(Value::Null);
            }
            Value::String(render(s, fact, &|_| None, false))
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| render_json(x, fact)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), render_json(v, fact)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The body and its type a webhook sends for a fact.
pub fn webhook_body(w: &Webhook, fact: &Value) -> (String, String) {
    match &w.body {
        None => (
            fact.to_string(),
            w.content_type
                .clone()
                .unwrap_or_else(|| "application/json".into()),
        ),
        Some(Value::String(t)) => {
            let ct = w
                .content_type
                .clone()
                .unwrap_or_else(|| "text/plain; charset=utf-8".into());
            // Secrets stay out of a body: it may be shown, kept, forwarded.
            let text = render(t, fact, &|_| None, ct.contains("json"));
            (text, ct)
        }
        Some(t) => (
            render_json(t, fact).to_string(),
            w.content_type
                .clone()
                .unwrap_or_else(|| "application/json".into()),
        ),
    }
}
