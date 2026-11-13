//! The facts handed to the plugins (PLG-5, ESC-177; docs/PLUGINS-DESIGN.md §1.6, §3.3): the sink of the
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

// What a manifest says of its facts (webhooks, commands, templates) and `Place`, in `escouade-plugins`; the sink is here.
use super::journal::{Journal, Kind};
use super::manifest::Manifest;
use super::runtime::{matches, Runtime};
use super::{Plugins, State};
use crate::facts::{Fact, Sink};
use crate::i18n::Lang;
use crate::integrations::secrets::SecretStore;
pub use escouade_plugins::events::*;
use parking_lot::{Mutex, RwLock};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Weak};

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
    use serde_json::Map;

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
