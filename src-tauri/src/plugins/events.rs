//! The facts handed to the plugins (PLG-5, ESC-177; docs/PLUGINS.md §1.6, §3.3): the sink of the
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

use super::journal::{Journal, Kind};
use super::manifest::Manifest;
use super::runtime::{matches, Runtime};
use super::{Plugins, State};
use crate::facts::{Fact, Sink};
use crate::i18n::Lang;
use crate::integrations::secrets::SecretStore;
use crate::pty::ShellInfo;
use parking_lot::{Mutex, RwLock};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Weak};
use std::time::Duration;

/// How many facts wait for a plugin before the oldest is dropped.
pub const QUEUE: usize = 256;
/// How long a webhook has to answer.
const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a command may run.
const COMMAND_LIMIT: Duration = Duration::from_secs(60);
/// How much of a webhook's answer or a command's output the journal keeps.
const SAID: usize = 300;

/// What the app tells the dispatch: the shell commands run in, a project's folder.
pub trait Place: Send + Sync {
    fn shell(&self) -> Option<ShellInfo>;
    fn project_path(&self, id: &str) -> Option<String>;
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

fn strings(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn object(v: Option<&Value>) -> Map<String, Value> {
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
    format!("{:x}", h.finalize())
}

/// The value at `path` (`ticket.key`, `data.to`) of `v`.
fn at<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(v, |v, k| v.get(k).filter(|x| !x.is_null()))
}

/// Whether a fact (`v`, as the plugins get it) is one of `events` and holds what `when` asks.
fn wanted(v: &Value, name: &str, events: &[String], when: &Map<String, Value>) -> bool {
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
fn text_of(v: Option<&Value>) -> String {
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
fn render_json(template: &Value, fact: &Value) -> Value {
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

/// What waits in a plugin's queue.
enum Job {
    Program(Arc<Fact>),
    Webhook(Webhook, Arc<Fact>),
    Command(Command, Arc<Fact>),
}

impl Job {
    fn fact(&self) -> &Fact {
        match self {
            Job::Program(f) | Job::Webhook(_, f) | Job::Command(_, f) => f,
        }
    }
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    draining: bool,
}

/// The facts' sink of the plugins.
pub struct Dispatch {
    me: Weak<Self>,
    plugins: Arc<Plugins>,
    runtime: Arc<Runtime>,
    secrets: Arc<dyn SecretStore>,
    cap: usize,
    queues: Mutex<HashMap<String, Arc<Mutex<Queue>>>>,
    place: RwLock<Option<Weak<dyn Place>>>,
    http: reqwest::Client,
}

impl Dispatch {
    pub fn new(
        plugins: Arc<Plugins>,
        runtime: Arc<Runtime>,
        secrets: Arc<dyn SecretStore>,
        cap: usize,
    ) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .timeout(WEBHOOK_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default();
        Arc::new_cyclic(|me| Self {
            me: me.clone(),
            plugins,
            runtime,
            secrets,
            cap: cap.max(1),
            queues: Mutex::new(HashMap::new()),
            place: RwLock::new(None),
            http,
        })
    }

    /// From now on, `p` gives the shell and the projects' folders.
    pub fn place_with(&self, p: Weak<dyn Place>) {
        *self.place.write() = Some(p);
    }

    fn place(&self) -> Option<Arc<dyn Place>> {
        self.place.read().as_ref().and_then(Weak::upgrade)
    }

    /// Puts `job` in the plugin `id`'s queue, its oldest dropped when it is full; drains it if
    /// nothing does.
    fn push(&self, id: &str, job: Job) {
        let queue = self
            .queues
            .lock()
            .entry(id.to_string())
            .or_default()
            .clone();
        let start = {
            let mut q = queue.lock();
            if q.jobs.len() >= self.cap {
                if let Some(old) = q.jobs.pop_front() {
                    let f = old.fact();
                    self.runtime.journal_of(id).push(
                        Kind::Dropped,
                        format!(
                            "the queue of facts is full ({}): the oldest, {} #{}, is dropped",
                            self.cap,
                            f.name(),
                            f.seq
                        ),
                    );
                }
            }
            q.jobs.push_back(job);
            !std::mem::replace(&mut q.draining, true)
        };
        if start {
            let (Some(me), id) = (self.me.upgrade(), id.to_string()) else {
                return;
            };
            tauri::async_runtime::spawn(async move { me.drain(&id, queue).await });
        }
    }

    /// Does what waits in a queue, one at a time, in order.
    async fn drain(self: Arc<Self>, id: &str, queue: Arc<Mutex<Queue>>) {
        loop {
            let job = {
                let mut q = queue.lock();
                match q.jobs.pop_front() {
                    Some(j) => j,
                    None => {
                        q.draining = false;
                        return;
                    }
                }
            };
            match job {
                Job::Program(f) => self.tell(id, &f).await,
                Job::Webhook(w, f) => self.call_webhook(id, &w, &f).await,
                Job::Command(c, f) => self.run_command(id, &c, &f).await,
            }
        }
    }

    /// Tells the plugin's program the fact; waits until its input took it.
    async fn tell(&self, id: &str, fact: &Fact) {
        let params = match serde_json::value::to_raw_value(fact) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("plugin {id}: fact not written: {e}");
                return;
            }
        };
        match self.runtime.deliver(id, "events/fact", &params) {
            Ok(written) => {
                let _ = written.await;
            }
            Err(e) => log::debug!("plugin {id}: fact {} not told: {e}", fact.name()),
        }
    }

    fn secret_of(&self, id: &str, m: &Manifest, journal: &Journal, key: &str) -> Option<String> {
        if !m.permissions.secrets.iter().any(|p| matches(p, key)) {
            journal.push(
                Kind::Refused,
                format!("a webhook asked for the secret {key:?}, which is not one of its own"),
            );
            return None;
        }
        let s = self
            .secrets
            .get(&super::runtime::secret_name(id, key))
            .ok()
            .flatten()?;
        journal.mask(&s);
        Some(s)
    }

    async fn call_webhook(&self, id: &str, w: &Webhook, fact: &Fact) {
        let journal = self.runtime.journal_of(id);
        // What it may still do: a host of `permissions.network` taken back is no longer one.
        let Some(m) = self
            .plugins
            .get(id, Lang::En)
            .and_then(|v| v.manifest.map(|m| super::granted(&m, &v.choices.revoked)))
        else {
            return;
        };
        let v = serde_json::to_value(fact).unwrap_or(Value::Null);
        let secret = |k: &str| self.secret_of(id, &m, &journal, k);
        let url = render(&w.url, &v, &secret, false);
        let Ok(parsed) = reqwest::Url::parse(&url) else {
            journal.push(
                Kind::Refused,
                format!("a webhook's address is not one ({})", fact.name()),
            );
            return;
        };
        // The host as the request will reach it (its user, its port and its case set apart),
        // whatever made it: the manifest, a secret, or a fact's value in a template.
        let host = parsed.host_str().unwrap_or("").to_string();
        if !matches!(parsed.scheme(), "http" | "https")
            || !super::manifest::host_covered(&host, &m.permissions.network)
        {
            journal.push(
                Kind::Refused,
                format!(
                    "a webhook to {host} is not sent: not an http(s) address of a host of `permissions.network` ({})",
                    fact.name()
                ),
            );
            return;
        }
        let (body, content_type) = webhook_body(w, &v);
        let mut req = self
            .http
            .post(parsed)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body);
        for (k, t) in &w.headers {
            req = req.header(k.as_str(), render(t, &v, &secret, false));
        }
        let said = match req.send().await {
            Ok(r) => {
                let status = r.status();
                let text: String = r
                    .text()
                    .await
                    .unwrap_or_default()
                    .chars()
                    .take(SAID)
                    .collect();
                if status.is_success() {
                    format!(
                        "webhook to {host}: {status} ({} #{})",
                        fact.name(),
                        fact.seq
                    )
                } else {
                    format!(
                        "webhook to {host}: {status} ({} #{}) {text}",
                        fact.name(),
                        fact.seq
                    )
                }
            }
            Err(e) => format!(
                "webhook to {host}: no answer ({} #{}): {}",
                fact.name(),
                fact.seq,
                if e.is_timeout() {
                    "timed out".to_string()
                } else {
                    e.without_url().to_string()
                }
            ),
        };
        journal.push(Kind::Webhook, said);
    }

    async fn run_command(&self, id: &str, c: &Command, fact: &Fact) {
        let journal = self.runtime.journal_of(id);
        let Some(view) = self.plugins.get(id, Lang::En) else {
            return;
        };
        let Some(m) = view.manifest else { return };
        if commands_fingerprint(&m) != self.plugins.commands_approved(id) {
            journal.push(
                Kind::Refused,
                format!(
                    "its commands were not agreed to as they are (Réglages › Plugins): {:?} not run on {}",
                    c.run,
                    fact.name()
                ),
            );
            return;
        }
        let place = self.place();
        let shell = place.as_ref().and_then(|p| p.shell()).or_else(|| {
            crate::pty::detect_shells(&crate::model::Settings::default())
                .into_iter()
                .next()
        });
        let Some(shell) = shell else {
            journal.push(Kind::Refused, "no shell to run its command");
            return;
        };
        let cwd = fact
            .project
            .as_ref()
            .and_then(|p| place.as_ref().and_then(|x| x.project_path(&p.id)))
            .unwrap_or(view.folder);
        let named = |n: &Option<crate::facts::Named>| {
            n.as_ref().map(|n| n.name.clone()).unwrap_or_default()
        };
        // The fact goes in variables, never in the command line: a ticket's title runs nothing.
        let env = vec![
            (
                "ESCOUADE_FACT".to_string(),
                serde_json::to_string(fact).unwrap_or_default(),
            ),
            ("ESCOUADE_FACT_NAME".to_string(), fact.name()),
            (
                "ESCOUADE_PROJECT".to_string(),
                fact.project
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_default(),
            ),
            (
                "ESCOUADE_TICKET".to_string(),
                fact.ticket
                    .as_ref()
                    .map(|t| t.key.clone())
                    .unwrap_or_default(),
            ),
            ("ESCOUADE_AGENT".to_string(), named(&fact.agent)),
        ];
        let said =
            match crate::testlaunch::run_command(&shell, &cwd, &c.run, &env, COMMAND_LIMIT).await {
                Ok(Some(run)) => {
                    let tail: String = run
                        .tail
                        .chars()
                        .rev()
                        .take(SAID)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    format!(
                        "command {:?} on {} #{}: {}{}",
                        c.run,
                        fact.name(),
                        fact.seq,
                        match run.code {
                            Some(code) => format!("exit {code}"),
                            None => "ended".into(),
                        },
                        if run.passed || tail.trim().is_empty() {
                            String::new()
                        } else {
                            format!(" · {}", tail.trim())
                        }
                    )
                }
                Ok(None) => format!(
                    "command {:?} on {} #{}: stopped after {} s",
                    c.run,
                    fact.name(),
                    fact.seq,
                    COMMAND_LIMIT.as_secs()
                ),
                Err(e) => format!("command {:?} on {}: {e:#}", c.run, fact.name()),
            };
        journal.push(Kind::Command, said);
    }
}

impl Sink for Dispatch {
    fn take(&self, fact: &Arc<Fact>) {
        let name = fact.name();
        let project = fact.project.as_ref().map(|p| p.id.as_str());
        let mut value = None;
        for view in self.plugins.list(Lang::En) {
            let Some(m) = &view.manifest else { continue };
            // One rule with Réglages › Plugins (PLG-4): its switches, and the events it may still
            // receive (`events:<e>` taken back, nor its webhooks nor its commands get it).
            let m = &super::granted(m, &view.choices.revoked);
            if view.state != State::Ready
                || !m.permissions.events.iter().any(|p| matches(p, &name))
                || !self.plugins.reaches(&view.id, project, fact.private)
            {
                continue;
            }
            if m.run.is_some() {
                self.push(&view.id, Job::Program(fact.clone()));
            }
            let (hooks, cmds) = (webhooks(m), commands(m));
            if hooks.is_empty() && cmds.is_empty() {
                continue;
            }
            let v = value.get_or_insert_with(|| serde_json::to_value(&**fact).unwrap_or_default());
            for w in hooks {
                if wanted(v, &name, &w.events, &w.when) {
                    self.push(&view.id, Job::Webhook(w, fact.clone()));
                }
            }
            for c in cmds {
                if wanted(v, &name, &c.events, &c.when) {
                    self.push(&view.id, Job::Command(c, fact.clone()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fact() -> Value {
        json!({
            "name": "ticket.column",
            "data": { "from": "doing", "to": "review" },
            "ticket": { "key": "ESC-1", "title": "Un \"titre\"" },
            "project": { "name": "Escouade" },
        })
    }

    #[test]
    fn a_template_takes_the_facts_values_and_escapes_them_in_json() {
        let f = fact();
        let none = |_: &str| None;
        assert_eq!(
            render("{{ticket.key}} → {{data.to}}{{missing}}", &f, &none, false),
            "ESC-1 → review"
        );
        assert_eq!(
            render(r#"{"t":"{{ticket.title}}"}"#, &f, &none, true),
            r#"{"t":"Un \"titre\""}"#
        );
        let secret = |k: &str| (k == "token").then(|| "s3cret".to_string());
        assert_eq!(
            render("https://x/{{secrets.token}}", &f, &secret, false),
            "https://x/s3cret"
        );
        assert_eq!(
            render_json(
                &json!({ "content": "{{ticket.key}} à tester", "data": "{{data}}" }),
                &f
            ),
            json!({ "content": "ESC-1 à tester", "data": { "from": "doing", "to": "review" } })
        );
    }

    #[test]
    fn a_condition_picks_the_facts() {
        let f = fact();
        let when = |w: Value| w.as_object().cloned().unwrap();
        let ev = vec!["ticket.*".to_string()];
        assert!(wanted(
            &f,
            "ticket.column",
            &ev,
            &when(json!({ "data.to": "review" }))
        ));
        assert!(wanted(
            &f,
            "ticket.column",
            &ev,
            &when(json!({ "data.to": ["done", "review"] }))
        ));
        assert!(!wanted(
            &f,
            "ticket.column",
            &ev,
            &when(json!({ "data.to": "done" }))
        ));
        assert!(!wanted(&f, "agent.status", &ev, &Map::new()));
    }
}
