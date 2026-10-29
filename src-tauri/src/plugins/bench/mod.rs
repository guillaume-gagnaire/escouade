//! The conformance bench of the plugins (PLG-3, ESC-175; docs/PLUGINS.md §4.5): scenarios played
//! against any plugin's folder through the fake Escouade (`fake`), each judged on what the app
//! would make of what the plugin did, and said with what is missing when it fails. The same bench
//! runs in the tests and in `escouade-plugin test <folder>` for the authors (`cli`).
//!
//! A scenario that the manifest says does not concern the plugin is not played: no fact without
//! `permissions.events`, no decision without `permissions.decide`, nothing of a program for a
//! declarative plugin.

pub mod cli;
pub mod fake;

use super::manifest::{self, Manifest};
use super::runtime::{program_path, timeout_of, METHOD_NOT_FOUND, PROTOCOL};
use crate::i18n::Lang;
use fake::{Reply, Seen, Session};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The method the bench calls to see that a plugin answers what it does not serve.
pub const UNKNOWN: &str = "bench/unknown";
/// The version of the protocol offered to see a plugin refuse it: one no Escouade speaks.
pub const FUTURE: u64 = 999;
/// The facts of the events (PLG-5): a pattern of `permissions.events` is played on the first of
/// these it takes in.
pub const FACTS: [&str; 25] = [
    "agent.created",
    "agent.status",
    "agent.waiting",
    "agent.stopped",
    "agent.handover",
    "turn.ended",
    "request.permission",
    "request.question",
    "request.answered",
    "ticket.created",
    "ticket.column",
    "ticket.report",
    "ticket.approved",
    "ticket.rejected",
    "ticket.merged",
    "ticket.conflict",
    "project.added",
    "project.closed",
    "worktree.created",
    "worktree.removed",
    "launch.started",
    "launch.ready",
    "launch.stopped",
    "tests.result",
    "app.started",
];

/// The scenarios, in the order of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Scenario {
    /// Its manifest holds, for this Escouade.
    Manifest,
    /// It answers `initialize` in time, in the version of the protocol.
    Initialize,
    /// Offered a version it does not speak, it answers its own, or refuses.
    Version,
    /// Every call answered within its delay; what it does not serve, refused at once.
    Delay,
    /// A fact it asked for, taken without an answer and without a crash.
    Fact,
    /// A decision answered `allow`, `deny` or `ask`.
    Decision,
    /// Only its own secrets asked for, never written out.
    Secret,
    /// Killed in the middle of a call, it starts again.
    Crash,
    /// It answers `shutdown`, then ends.
    Shutdown,
}

impl Scenario {
    pub const ALL: [Scenario; 9] = [
        Scenario::Manifest,
        Scenario::Initialize,
        Scenario::Version,
        Scenario::Delay,
        Scenario::Fact,
        Scenario::Decision,
        Scenario::Secret,
        Scenario::Crash,
        Scenario::Shutdown,
    ];

    pub fn label(self, lang: Lang) -> &'static str {
        let (fr, en) = match self {
            Scenario::Manifest => ("Manifeste", "Manifest"),
            Scenario::Initialize => ("Initialisation", "Initialization"),
            Scenario::Version => ("Version refusée", "Version refused"),
            Scenario::Delay => ("Délais", "Delays"),
            Scenario::Fact => ("Un fait reçu", "A fact received"),
            Scenario::Decision => ("Une décision", "A decision"),
            Scenario::Secret => ("Un secret", "A secret"),
            Scenario::Crash => ("Plantage", "Crash"),
            Scenario::Shutdown => ("Fermeture", "Shutdown"),
        };
        match lang {
            Lang::Fr => fr,
            Lang::En => en,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Pass,
    Fail,
    /// Not played: it does not concern the plugin, or what it needs failed first.
    Skip,
}

/// What a scenario made of the plugin.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub scenario: Scenario,
    pub verdict: Verdict,
    /// What happened, in words.
    pub said: String,
    /// What the plugin lacks, when it failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<String>,
}

/// A call of the bench, and how long its answer took (none: no answer).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    pub delay_ms: u64,
}

impl Timing {
    /// Answered, but past its delay: Escouade had stopped waiting. A call never answered is
    /// judged by its own scenario.
    fn late(&self) -> bool {
        self.elapsed_ms.is_some_and(|e| e > self.delay_ms)
    }
}

/// Round trips measured (`--measure`): a call it does not serve, answered.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub calls: usize,
    pub median_us: u64,
    pub p99_us: u64,
}

/// What the bench made of a plugin.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Its id and version, or its folder when its manifest does not hold.
    pub plugin: String,
    pub outcomes: Vec<Outcome>,
    pub timings: Vec<Timing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measure: Option<Measure>,
}

impl Report {
    /// No scenario failed.
    pub fn passed(&self) -> bool {
        self.outcomes.iter().all(|o| o.verdict != Verdict::Fail)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn outcome(&self, s: Scenario) -> &Outcome {
        self.outcomes
            .iter()
            .find(|o| o.scenario == s)
            .expect("every scenario has its outcome")
    }

    /// The report in words, a line per scenario.
    pub fn render(&self, lang: Lang) -> String {
        let mut out = tr_in!(
            lang,
            "Banc de conformité : {}\n\n",
            "Conformance bench: {}\n\n",
            self.plugin
        );
        let width = Scenario::ALL
            .iter()
            .map(|s| s.label(lang).chars().count())
            .max()
            .unwrap_or(0);
        for o in &self.outcomes {
            let mark = match o.verdict {
                Verdict::Pass => "✓",
                Verdict::Fail => "✗",
                Verdict::Skip => "–",
            };
            let label = o.scenario.label(lang);
            let pad = width - label.chars().count();
            out.push_str(&format!("{mark} {label}{}  {}\n", " ".repeat(pad), o.said));
            if let Some(m) = &o.missing {
                let head = tr_in!(lang, "Ce qui manque :", "What is missing:");
                out.push_str(&format!("  {} {head} {m}\n", " ".repeat(width)));
            }
        }
        if let Some(m) = &self.measure {
            out.push_str(&tr_in!(
                lang,
                "\nAllers-retours : {} appels, médiane {} µs, 99e centile {} µs\n",
                "\nRound trips: {} calls, median {} µs, 99th percentile {} µs\n",
                m.calls,
                m.median_us,
                m.p99_us
            ));
        }
        let count = |v| self.outcomes.iter().filter(|o| o.verdict == v).count();
        out.push_str(&tr_in!(
            lang,
            "\n{} réussis, {} échoués, {} sans objet\n",
            "\n{} passed, {} failed, {} not played\n",
            count(Verdict::Pass),
            count(Verdict::Fail),
            count(Verdict::Skip)
        ));
        out
    }
}

/// How the bench plays.
#[derive(Debug, Clone)]
pub struct Options {
    pub lang: Lang,
    /// The version of Escouade the plugin is judged for.
    pub app_version: String,
    /// The folder of the app's programs (`{escouade}/…` of a plugin of Escouade's own).
    pub escouade_dir: PathBuf,
    /// A plugin of Escouade's own, which may take its ids and programs.
    pub builtin: bool,
    /// The delay of `initialize`.
    pub initialize: Duration,
    /// How long past its delay an answer is still waited for, to say how late it came.
    pub grace: Duration,
    /// Round trips to measure; none when 0.
    pub measure: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            lang: crate::i18n::system(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            escouade_dir: std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf))
                .unwrap_or_default(),
            builtin: false,
            initialize: timeout_of("initialize"),
            grace: Duration::from_secs(2),
            measure: 0,
        }
    }
}

/// The project, the ticket and the agent of the facts and the decision the bench sends.
fn subjects() -> Value {
    json!({
        "project": { "id": "bench-project", "name": "Banc", "private": false },
        "ticket": { "id": "bench-ticket", "key": "BEN-1", "title": "Un ticket du banc" },
        "agent": { "id": "bench-agent", "name": "Agent du banc" },
    })
}

/// The fact `name`, as the bench sends it (`events/fact`).
pub fn fact(name: &str) -> Value {
    let mut f = subjects();
    f["name"] = json!(name);
    f["version"] = json!(1);
    f["at"] = json!(chrono::Utc::now().to_rfc3339());
    if name == "ticket.column" {
        f["ticket"]["from"] = json!("doing");
        f["ticket"]["column"] = json!("test");
    }
    f
}

/// The request of a decision, as the bench sends it (`permissions/decide`).
pub fn decision_request() -> Value {
    let mut d = subjects();
    d["request"] = json!({
        "id": "bench-request",
        "kind": "command",
        "tool": "Bash",
        "input": { "command": "npm test" },
    });
    d
}

/// The fact a pattern of `permissions.events` is played on.
fn fact_of(patterns: &[String]) -> Option<String> {
    patterns.iter().find_map(|p| {
        if p.ends_with('*') {
            FACTS
                .iter()
                .find(|f| super::runtime::matches(p, f))
                .map(|f| f.to_string())
        } else {
            Some(p.clone())
        }
    })
}

fn ms(d: Duration) -> u64 {
    d.as_millis() as u64
}

/// The last lines of its standard error, after what they explain.
fn tail_of(tail: &[String]) -> String {
    if tail.is_empty() {
        String::new()
    } else {
        format!(" — {}", tail.join(" | "))
    }
}

struct Bench<'a> {
    o: &'a Options,
    folder: PathBuf,
    outcomes: HashMap<Scenario, Outcome>,
    timings: Vec<Timing>,
    /// What every session saw, the sessions gone included.
    seen: Vec<Seen>,
    /// The part of every secret the bench gives, to find one written out.
    nonce: String,
}

impl Bench<'_> {
    fn set(&mut self, s: Scenario, verdict: Verdict, said: String, missing: Option<String>) {
        self.outcomes.insert(
            s,
            Outcome {
                scenario: s,
                verdict,
                said,
                missing,
            },
        );
    }

    fn pass(&mut self, s: Scenario, said: String) {
        self.set(s, Verdict::Pass, said, None);
    }

    fn fail(&mut self, s: Scenario, said: String, missing: String) {
        self.set(s, Verdict::Fail, said, Some(missing));
    }

    fn skip_rest(&mut self, said: &str) {
        for s in Scenario::ALL {
            if !self.outcomes.contains_key(&s) {
                self.set(s, Verdict::Skip, said.to_string(), None);
            }
        }
    }

    fn secrets(&self, m: &Manifest) -> HashMap<String, String> {
        // A value for each key it may ask; a pattern's keys are made as they are asked.
        m.permissions
            .secrets
            .iter()
            .filter(|k| !k.ends_with('*'))
            .map(|k| (k.clone(), format!("bench-secret-{}-{k}", self.nonce)))
            .collect()
    }

    fn start(&self, m: &Manifest, program: &Path) -> std::io::Result<Session> {
        Session::start(
            &self.folder,
            m,
            program,
            &self.o.app_version,
            self.secrets(m),
            Some(format!("bench-secret-{}", self.nonce)),
        )
    }

    /// Calls `method` within its delay and the grace, and notes how long it took.
    async fn timed(&mut self, s: &Session, method: &str, params: &Value, delay: Duration) -> Reply {
        let reply = s.call(method, params, delay + self.o.grace).await;
        self.timings.push(Timing {
            method: method.to_string(),
            elapsed_ms: match &reply {
                Reply::Answered { elapsed, .. } => Some(ms(*elapsed)),
                _ => None,
            },
            delay_ms: ms(delay),
        });
        reply
    }

    /// Starts a process and has it initialized: its session, or what went wrong and what is
    /// missing.
    async fn initialized(
        &mut self,
        m: &Manifest,
        program: &Path,
    ) -> Result<(Session, String), (String, String)> {
        let lang = self.o.lang;
        let s = self.start(m, program).map_err(|e| {
            (
                tr_in!(
                    lang,
                    "{} ne démarre pas : {e}",
                    "{} does not start: {e}",
                    program.display()
                ),
                tr_in!(
                    lang,
                    "un programme exécutable à l'endroit que dit `run` pour ce système.",
                    "an executable program where `run` says for this system."
                ),
            )
        })?;
        let hello = Session::hello(m, &self.o.app_version, PROTOCOL);
        let delay = self.o.initialize;
        let secs = delay.as_secs_f32();
        let answer_it = tr_in!(
            lang,
            "réponds à `initialize` par {{\"protocolVersion\": {PROTOCOL}, \"capabilities\": {{}}}}, avant toute autre chose.",
            "answer `initialize` with {{\"protocolVersion\": {PROTOCOL}, \"capabilities\": {{}}}}, before anything else."
        );
        match self.timed(&s, "initialize", &hello, delay).await {
            Reply::Silent => Err((
                tr_in!(
                    lang,
                    "aucune réponse à `initialize` en {secs} s : Escouade arrête le plugin",
                    "no answer to initialize within {secs} s: Escouade stops the plugin"
                ),
                answer_it,
            )),
            Reply::Ended { code, tail } => Err((
                tr_in!(
                    lang,
                    "le programme s'est terminé avant de répondre ({}){}",
                    "the program ended before it answered ({}){}",
                    code.map_or("no code".into(), |c| format!("code {c}")),
                    tail_of(&tail)
                ),
                answer_it,
            )),
            Reply::Answered { result: Err(e), .. } => Err((
                tr_in!(
                    lang,
                    "`initialize` refusé : {} ({})",
                    "initialize refused: {} ({})",
                    e.message,
                    e.code
                ),
                answer_it,
            )),
            Reply::Answered {
                result: Ok(v),
                elapsed,
            } => {
                let version = v.get("protocolVersion").and_then(Value::as_u64);
                if elapsed > delay {
                    return Err((
                        tr_in!(
                            lang,
                            "réponse à `initialize` en {} ms, au-delà de son délai de {secs} s : Escouade arrête le plugin",
                            "answered initialize in {} ms, past its delay of {secs} s: Escouade stops the plugin",
                            ms(elapsed)
                        ),
                        tr_in!(
                            lang,
                            "répondre à `initialize` tout de suite, et faire le reste ensuite.",
                            "answer initialize at once, and do the rest after."
                        ),
                    ));
                }
                if version != Some(PROTOCOL) {
                    let said = version.map_or("no version".to_string(), |n| format!("version {n}"));
                    return Err((
                        tr_in!(
                            lang,
                            "il parle {said} du protocole ; cet Escouade parle la version {PROTOCOL}",
                            "it speaks {said} of the protocol; this Escouade speaks version {PROTOCOL}"
                        ),
                        answer_it,
                    ));
                }
                if v.get("capabilities").is_some_and(|c| !c.is_object()) {
                    return Err((
                        tr_in!(
                            lang,
                            "ses `capabilities` ne sont pas un objet",
                            "its capabilities are not an object"
                        ),
                        answer_it,
                    ));
                }
                Ok((
                    s,
                    tr_in!(
                        lang,
                        "répond en {} ms, protocole {PROTOCOL}",
                        "answers in {} ms, protocol {PROTOCOL}",
                        ms(elapsed)
                    ),
                ))
            }
        }
    }

    fn keep(&mut self, s: &Session) {
        self.seen.extend(s.seen());
    }
}

/// Plays every scenario against the plugin in `folder`.
pub async fn run(folder: &Path, o: &Options) -> Report {
    let lang = o.lang;
    let mut b = Bench {
        o,
        folder: folder.to_path_buf(),
        outcomes: HashMap::new(),
        timings: Vec::new(),
        seen: Vec::new(),
        nonce: uuid::Uuid::new_v4().simple().to_string()[..12].to_string(),
    };
    let mut plugin = folder.display().to_string();
    let mut measure = None;
    if let Some(m) = manifest_of(&mut b) {
        plugin = format!("{} {}", m.id, m.version);
        if m.declarative() {
            b.skip_rest(&tr_in!(
                lang,
                "plugin déclaratif : aucun programme à jouer",
                "a declarative plugin: no program to play"
            ));
        } else {
            measure = program(&mut b, &m).await;
        }
    }
    b.skip_rest(&tr_in!(
        lang,
        "le manifeste doit passer d'abord",
        "the manifest must pass first"
    ));
    let mut outcomes: Vec<Outcome> = Vec::new();
    for s in Scenario::ALL {
        outcomes.push(
            b.outcomes
                .remove(&s)
                .expect("every scenario has its outcome"),
        );
    }
    Report {
        plugin,
        outcomes,
        timings: b.timings,
        measure,
    }
}

/// The manifest scenario: the manifest, when it holds for this Escouade.
fn manifest_of(b: &mut Bench) -> Option<Manifest> {
    let lang = b.o.lang;
    let path = b.folder.join(manifest::FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            b.fail(
                Scenario::Manifest,
                tr_in!(
                    lang,
                    "{} illisible : {e}",
                    "{} cannot be read: {e}",
                    path.display()
                ),
                tr_in!(
                    lang,
                    "un {} à la racine du dossier du plugin.",
                    "an {} at the root of the plugin's folder.",
                    manifest::FILE
                ),
            );
            return None;
        }
    };
    let m = match manifest::read(&text, b.o.builtin).manifest {
        Ok(m) => m,
        Err(problems) => {
            let said: Vec<String> = problems.iter().map(|p| p.message(lang)).collect();
            b.fail(
                Scenario::Manifest,
                said.join(" ; "),
                tr_in!(
                    lang,
                    "les champs ci-dessus, comme les décrit le schéma du manifeste.",
                    "the fields above, as the manifest's schema describes them."
                ),
            );
            return None;
        }
    };
    if let Some(why) = m.refusal(&b.o.app_version, lang) {
        b.fail(
            Scenario::Manifest,
            why,
            tr_in!(
                lang,
                "une plage `escouade` qui prend cette version d'Escouade.",
                "an `escouade` range that takes this version of Escouade."
            ),
        );
        return None;
    }
    let said = if m.declarative() {
        tr_in!(lang, "valide, déclaratif", "holds, declarative")
    } else {
        tr_in!(lang, "valide, avec un programme", "holds, with a program")
    };
    b.pass(Scenario::Manifest, said);
    Some(m)
}

/// The scenarios of a program; the round trips measured when asked.
async fn program(b: &mut Bench<'_>, m: &Manifest) -> Option<Measure> {
    let lang = b.o.lang;
    let run = m.run.as_ref().expect("a program");
    let program = program_path(&b.folder, run, &b.o.escouade_dir);

    // Initialization.
    let first = match b.initialized(m, &program).await {
        Ok((s, said)) => {
            b.pass(Scenario::Initialize, said);
            s
        }
        Err((said, missing)) => {
            b.fail(Scenario::Initialize, said, missing);
            b.skip_rest(&tr_in!(
                lang,
                "l'initialisation doit passer d'abord",
                "the initialization must pass first"
            ));
            return None;
        }
    };

    // A fact received.
    let fact_name = fact_of(&m.permissions.events);
    match &fact_name {
        None => b.set(
            Scenario::Fact,
            Verdict::Skip,
            tr_in!(
                lang,
                "il ne demande aucun fait (`permissions.events`)",
                "it asks for no fact (`permissions.events`)"
            ),
            None,
        ),
        Some(name) => {
            let strays = first
                .seen()
                .iter()
                .filter(|s| matches!(s, Seen::Stray { .. }))
                .count();
            first.notify("events/fact", &fact(name));
            // What it does of the fact, then whether it still answers.
            tokio::time::sleep(Duration::from_millis(300)).await;
            let probe = first
                .call(UNKNOWN, &json!({}), timeout_of(UNKNOWN) + b.o.grace)
                .await;
            let answered_it = first
                .seen()
                .iter()
                .filter(|s| matches!(s, Seen::Stray { .. }))
                .count()
                > strays;
            match probe {
                Reply::Ended { code, tail } => b.fail(
                    Scenario::Fact,
                    tr_in!(
                        lang,
                        "le programme s'est terminé après le fait {name} ({}){}",
                        "the program ended after the fact {name} ({}){}",
                        code.map_or("no code".into(), |c| format!("code {c}")),
                        tail_of(&tail)
                    ),
                    tr_in!(
                        lang,
                        "recevoir `events/fact` sans planter, même d'un fait aux champs inconnus.",
                        "take `events/fact` without crashing, even a fact with unknown fields."
                    ),
                ),
                Reply::Silent => b.fail(
                    Scenario::Fact,
                    tr_in!(
                        lang,
                        "après le fait {name}, il ne répond plus",
                        "after the fact {name}, it no longer answers"
                    ),
                    tr_in!(
                        lang,
                        "traiter un fait sans bloquer la lecture des messages suivants.",
                        "handle a fact without blocking the reading of the next messages."
                    ),
                ),
                Reply::Answered { .. } if answered_it => b.fail(
                    Scenario::Fact,
                    tr_in!(
                        lang,
                        "il a répondu à la notification du fait {name}",
                        "it answered the notification of the fact {name}"
                    ),
                    tr_in!(
                        lang,
                        "ne pas répondre à une notification (un message sans `id`).",
                        "no answer to a notification (a message without an `id`)."
                    ),
                ),
                Reply::Answered { .. } => b.pass(
                    Scenario::Fact,
                    tr_in!(
                        lang,
                        "{name} reçu, il répond encore",
                        "{name} received, it still answers"
                    ),
                ),
            }
        }
    }

    // A decision.
    if !m.permissions.decide {
        b.set(
            Scenario::Decision,
            Verdict::Skip,
            tr_in!(
                lang,
                "il ne décide d'aucune autorisation (`permissions.decide`)",
                "it decides no permission (`permissions.decide`)"
            ),
            None,
        );
    } else {
        let method = "permissions/decide";
        let shape = tr_in!(
            lang,
            "répondre {{\"decision\": \"allow\" | \"deny\" | \"ask\"}}, avec un `message` pour un refus.",
            "answer {{\"decision\": \"allow\" | \"deny\" | \"ask\"}}, with a `message` for a denial."
        );
        match b
            .timed(&first, method, &decision_request(), timeout_of(method))
            .await
        {
            Reply::Answered {
                result: Ok(v),
                elapsed,
            } => match v.get("decision").and_then(Value::as_str) {
                Some(d @ ("allow" | "deny" | "ask")) => b.pass(
                    Scenario::Decision,
                    tr_in!(lang, "« {d} » en {} ms", "\"{d}\" in {} ms", ms(elapsed)),
                ),
                _ => b.fail(
                    Scenario::Decision,
                    tr_in!(
                        lang,
                        "sa réponse n'est pas une décision : {v}",
                        "its answer is no decision: {v}"
                    ),
                    shape,
                ),
            },
            Reply::Answered { result: Err(e), .. } => b.fail(
                Scenario::Decision,
                tr_in!(
                    lang,
                    "erreur : {} ({}) ; l'utilisateur est questionné à sa place",
                    "error: {} ({}); the user is asked instead",
                    e.message,
                    e.code
                ),
                shape,
            ),
            Reply::Silent => b.fail(
                Scenario::Decision,
                tr_in!(
                    lang,
                    "aucune réponse : l'utilisateur est questionné à sa place",
                    "no answer: the user is asked instead"
                ),
                shape,
            ),
            Reply::Ended { code, tail } => b.fail(
                Scenario::Decision,
                tr_in!(
                    lang,
                    "le programme s'est terminé pendant la décision ({}){}",
                    "the program ended during the decision ({}){}",
                    code.map_or("no code".into(), |c| format!("code {c}")),
                    tail_of(&tail)
                ),
                shape,
            ),
        }
    }

    // What it does not serve, refused at once.
    let unknown = b
        .timed(&first, UNKNOWN, &json!({}), timeout_of(UNKNOWN))
        .await;
    let refuses_unknown = matches!(
        &unknown,
        Reply::Answered { result: Err(e), .. } if e.code == METHOD_NOT_FOUND
    );

    // A crash in the middle of a call: killed, it starts again.
    let _waits = first.send(UNKNOWN, &json!({ "inFlight": true }));
    if let Some(name) = &fact_name {
        first.notify("events/fact", &fact(name));
    }
    first.kill();
    first.wait_end(Duration::from_secs(5)).await;
    b.keep(&first);
    drop(first);
    let second = match b.initialized(m, &program).await {
        Ok((s, said)) => {
            b.pass(
                Scenario::Crash,
                tr_in!(
                    lang,
                    "tué au milieu d'un appel, il redémarre : {said}",
                    "killed in the middle of a call, it starts again: {said}"
                ),
            );
            Some(s)
        }
        Err((said, missing)) => {
            b.fail(
                Scenario::Crash,
                tr_in!(
                    lang,
                    "tué au milieu d'un appel, il ne redémarre pas : {said}",
                    "killed in the middle of a call, it does not start again: {said}"
                ),
                tr_in!(
                    lang,
                    "redémarrer sans rien attendre du process d'avant (fichier verrou, port, état) ; {missing}",
                    "start again without counting on the process before (a lock file, a port, a state); {missing}"
                ),
            );
            None
        }
    };

    // Round trips, then shutdown.
    let mut measure = None;
    if let Some(s) = &second {
        if b.o.measure > 0 {
            let mut took = Vec::with_capacity(b.o.measure);
            for _ in 0..b.o.measure {
                if let Reply::Answered { elapsed, .. } =
                    s.call(UNKNOWN, &json!({}), timeout_of(UNKNOWN)).await
                {
                    took.push(elapsed.as_micros() as u64);
                }
            }
            took.sort_unstable();
            if !took.is_empty() {
                let at = |q: f64| took[((took.len() - 1) as f64 * q).round() as usize];
                measure = Some(Measure {
                    calls: took.len(),
                    median_us: at(0.5),
                    p99_us: at(0.99),
                });
            }
        }
        let delay = timeout_of("shutdown");
        let missing = tr_in!(
            lang,
            "répondre à `shutdown`, puis se terminer dans la seconde.",
            "answer `shutdown`, then end within the second."
        );
        match b.timed(s, "shutdown", &Value::Null, delay).await {
            Reply::Answered { elapsed, .. } => {
                let end = s.wait_end(delay + b.o.grace).await;
                if end.is_some() {
                    b.pass(
                        Scenario::Shutdown,
                        tr_in!(
                            lang,
                            "répond en {} ms, puis se termine",
                            "answers in {} ms, then ends",
                            ms(elapsed)
                        ),
                    );
                } else {
                    b.fail(
                        Scenario::Shutdown,
                        tr_in!(
                            lang,
                            "répond à `shutdown` mais tourne encore : Escouade met fin à son Job",
                            "answers shutdown but still runs: Escouade ends its job"
                        ),
                        missing,
                    );
                }
            }
            Reply::Ended { .. } => b.pass(
                Scenario::Shutdown,
                tr_in!(lang, "se termine sans répondre", "ends without an answer"),
            ),
            Reply::Silent => b.fail(
                Scenario::Shutdown,
                tr_in!(
                    lang,
                    "aucune réponse à `shutdown` en {} ms : Escouade met fin à son Job",
                    "no answer to shutdown within {} ms: Escouade ends its job",
                    ms(delay)
                ),
                missing,
            ),
        }
        s.kill();
        s.wait_end(Duration::from_secs(5)).await;
        b.keep(s);
    } else {
        b.set(
            Scenario::Shutdown,
            Verdict::Skip,
            tr_in!(
                lang,
                "il n'a pas redémarré après le plantage",
                "it did not start again after the crash"
            ),
            None,
        );
    }
    drop(second);

    // Version refused: a version no Escouade speaks, offered to a process of its own.
    version(b, m, &program).await;

    // Delays, over every call of the bench.
    let late: Vec<String> = b
        .timings
        .iter()
        .filter(|t| t.late())
        .map(|t| {
            tr_in!(
                lang,
                "{} : répondu en {} ms, au-delà de son délai de {} ms",
                "{}: answered in {} ms, past its delay of {} ms",
                t.method,
                t.elapsed_ms.unwrap_or_default(),
                t.delay_ms
            )
        })
        .collect();
    if !late.is_empty() {
        b.fail(
            Scenario::Delay,
            late.join(" ; "),
            tr_in!(
                lang,
                "répondre dans le délai de chaque méthode (500 ms pour une décision, 5 s en général) ; passé ce délai, Escouade n'attend plus et sa réponse par défaut s'applique.",
                "answer within each method's delay (500 ms for a decision, 5 s in general); past it, Escouade no longer waits and its default answer applies."
            ),
        );
    } else if !refuses_unknown {
        let what = match &unknown {
            Reply::Answered { result: Ok(v), .. } => format!("{v}"),
            Reply::Answered { result: Err(e), .. } => format!("{} ({})", e.message, e.code),
            Reply::Silent => tr_in!(lang, "rien", "nothing"),
            Reply::Ended { .. } => tr_in!(lang, "rien : il s'est terminé", "nothing: it ended"),
        };
        b.fail(
            Scenario::Delay,
            tr_in!(
                lang,
                "à {UNKNOWN}, qu'il ne sert pas, il répond {what}",
                "to {UNKNOWN}, which it does not serve, it answers {what}"
            ),
            tr_in!(
                lang,
                "répondre l'erreur {METHOD_NOT_FOUND} à une méthode inconnue, tout de suite : sans réponse, Escouade attendrait tout son délai.",
                "answer the error {METHOD_NOT_FOUND} to an unknown method, at once: without an answer, Escouade would wait its whole delay."
            ),
        );
    } else {
        let worst = b
            .timings
            .iter()
            .filter(|t| t.method != "initialize")
            .filter_map(|t| t.elapsed_ms)
            .max()
            .unwrap_or(0);
        b.pass(
            Scenario::Delay,
            tr_in!(
                lang,
                "{} appels dans leur délai (le plus long : {worst} ms) ; une méthode inconnue refusée",
                "{} calls within their delay (the longest: {worst} ms); an unknown method refused",
                b.timings.len()
            ),
        );
    }

    secret(b, m);
    measure
}

/// Offered a version it does not speak, it must say its own or refuse: never take it.
async fn version(b: &mut Bench<'_>, m: &Manifest, program: &Path) {
    let lang = b.o.lang;
    let missing = tr_in!(
        lang,
        "répondre la version du protocole qu'il parle ({PROTOCOL}), et non celle qu'on lui offre ; Escouade l'arrête si ce n'est pas la sienne.",
        "answer the version of the protocol it speaks ({PROTOCOL}), not the one it is offered; Escouade stops it if it is not its own."
    );
    let s = match b.start(m, program) {
        Ok(s) => s,
        Err(e) => {
            b.fail(
                Scenario::Version,
                format!("{}: {e}", program.display()),
                missing,
            );
            return;
        }
    };
    let hello = Session::hello(m, &b.o.app_version, FUTURE);
    let reply = s
        .call("initialize", &hello, b.o.initialize + b.o.grace)
        .await;
    match reply {
        Reply::Answered { result: Ok(v), .. } => {
            match v.get("protocolVersion").and_then(Value::as_u64) {
                Some(FUTURE) => b.fail(
                    Scenario::Version,
                    tr_in!(
                        lang,
                        "offerte la version {FUTURE}, qui n'existe pas, il dit la parler",
                        "offered version {FUTURE}, which does not exist, it says it speaks it"
                    ),
                    missing,
                ),
                Some(n) => b.pass(
                    Scenario::Version,
                    tr_in!(
                        lang,
                        "offerte la version {FUTURE}, il répond la sienne ({n}) : Escouade l'arrête",
                        "offered version {FUTURE}, it answers its own ({n}): Escouade stops it"
                    ),
                ),
                None => b.fail(
                    Scenario::Version,
                    tr_in!(
                        lang,
                        "offerte la version {FUTURE}, il répond sans version",
                        "offered version {FUTURE}, it answers without a version"
                    ),
                    missing,
                ),
            }
        }
        Reply::Answered { result: Err(e), .. } => b.pass(
            Scenario::Version,
            tr_in!(
                lang,
                "offerte la version {FUTURE}, il refuse : {}",
                "offered version {FUTURE}, it refuses: {}",
                e.message
            ),
        ),
        Reply::Ended { .. } => b.pass(
            Scenario::Version,
            tr_in!(
                lang,
                "offerte la version {FUTURE}, il se termine",
                "offered version {FUTURE}, it ends"
            ),
        ),
        Reply::Silent => b.fail(
            Scenario::Version,
            tr_in!(
                lang,
                "offerte la version {FUTURE}, il ne répond pas : Escouade attend tout le délai de `initialize`",
                "offered version {FUTURE}, it does not answer: Escouade waits the whole delay of initialize"
            ),
            missing,
        ),
    }
    s.kill();
    s.wait_end(Duration::from_secs(5)).await;
    b.keep(&s);
}

/// What it asked of its secrets, and whether it wrote one out, over every session.
fn secret(b: &mut Bench, m: &Manifest) {
    let lang = b.o.lang;
    let mark = format!("bench-secret-{}", b.nonce);
    let mut asked = Vec::new();
    let mut theirs = Vec::new();
    let mut leaked = false;
    for s in &b.seen {
        match s {
            Seen::Secret { key, own: true } => asked.push(key.clone()),
            Seen::Secret { key, own: false } => theirs.push(key.clone()),
            Seen::Stderr { line } | Seen::Stray { line } => leaked |= line.contains(&mark),
            Seen::Log { message, .. } => leaked |= message.contains(&mark),
            _ => {}
        }
    }
    if let Some(key) = theirs.first() {
        b.fail(
            Scenario::Secret,
            tr_in!(
                lang,
                "il a demandé le secret {key:?}, qui n'est pas l'un des siens : Escouade le refuse (-32001)",
                "it asked for the secret {key:?}, which is not one of its own: Escouade refuses it (-32001)"
            ),
            tr_in!(
                lang,
                "ne demander que les clés de ses `permissions.secrets`.",
                "ask only for the keys of its `permissions.secrets`."
            ),
        );
    } else if leaked {
        b.fail(
            Scenario::Secret,
            tr_in!(
                lang,
                "il a écrit la valeur d'un secret sur sa sortie d'erreur ou dans un `log`",
                "it wrote the value of a secret on its standard error or in a `log`"
            ),
            tr_in!(
                lang,
                "ne jamais écrire un secret : Escouade le masque dans le journal, mais pas ailleurs.",
                "never write a secret out: Escouade masks it in the journal, but nowhere else."
            ),
        );
    } else if !asked.is_empty() {
        asked.dedup();
        b.pass(
            Scenario::Secret,
            tr_in!(
                lang,
                "a demandé {} : les siens, jamais écrits",
                "asked for {}: its own, never written out",
                asked.join(", ")
            ),
        );
    } else if m.permissions.secrets.is_empty() {
        b.set(
            Scenario::Secret,
            Verdict::Skip,
            tr_in!(
                lang,
                "il ne demande aucun secret (`permissions.secrets`)",
                "it asks for no secret (`permissions.secrets`)"
            ),
            None,
        );
    } else {
        b.pass(
            Scenario::Secret,
            tr_in!(
                lang,
                "aucun secret demandé pendant le banc",
                "no secret asked for during the bench"
            ),
        );
    }
}
